use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::State;

use crate::keys::Key;
use crate::profile::{Hotkeys, Profile, Row};
use crate::safety::focus_matches;
use crate::state::AppState;
use crate::win::probe::foreground_process_name;

/// Rejects a row list the engine could never act on, naming the first offender
/// so the UI can point at it.
///
/// A row bound to a global hotkey would fire that hotkey every time it pressed —
/// so the hotkeys are checked here, and they are user-configured.
pub fn validate_rows(rows: &[Row], hotkeys: &Hotkeys) -> Result<(), String> {
    for (index, row) in rows.iter().enumerate() {
        let key = Key::parse(&row.key)
            .map_err(|_| format!("row {index}: unsupported key \"{}\"", row.key))?;
        let canonical = key.name();
        if hotkeys
            .iter()
            .any(|(_, bound)| bound.eq_ignore_ascii_case(&canonical))
        {
            return Err(format!("row {index}: \"{canonical}\" is bound to a hotkey"));
        }
    }
    Ok(())
}

/// Validates one hotkey slot change and returns the canonical key name.
///
/// One key, no modifiers: the poller watches a single virtual key, which is also
/// what lets a mouse button be a hotkey.
pub fn validate_hotkey(key: &str, hotkeys: &Hotkeys, slot: &str) -> Result<String, String> {
    if hotkeys.get(slot).is_none() {
        return Err(format!("unknown hotkey slot \"{slot}\""));
    }
    let parsed = Key::parse(key).map_err(|_| format!("unsupported key \"{key}\""))?;
    let canonical = parsed.name();
    if let Some((other, _)) = hotkeys
        .iter()
        .find(|(other, bound)| *other != slot && bound.eq_ignore_ascii_case(&canonical))
    {
        return Err(format!("\"{canonical}\" already {other}s the presser"));
    }
    Ok(canonical)
}

#[derive(Serialize, Clone)]
pub struct StateDto {
    pub version: String,
    pub armed: bool,
    pub presses: u64,
    pub skipped: u64,
    pub rows: Vec<Row>,
    pub hotkeys: Hotkeys,
    pub gate: bool,
    pub game_focused: bool,
    pub expected_process: String,
    pub profile_path: String,
    pub recent_profiles: Vec<String>,
    pub capture_ok: bool,
}

pub fn build_state(state: &AppState) -> StateDto {
    let engine = state.engine.lock().unwrap_or_else(|e| e.into_inner());
    let profile = state.profile.lock().unwrap_or_else(|e| e.into_inner());
    let config = state.config.lock().unwrap_or_else(|e| e.into_inner());

    StateDto {
        // Taken from the crate manifest, so the label cannot drift from the build.
        version: env!("CARGO_PKG_VERSION").to_string(),
        armed: engine.is_armed(),
        presses: engine.presses(),
        skipped: engine.skipped(),
        rows: profile.rows.clone(),
        hotkeys: config.hotkeys.clone(),
        gate: engine.gate(),
        game_focused: focus_matches(
            &config.expected_process,
            &foreground_process_name().unwrap_or_default(),
        ),
        expected_process: config.expected_process.clone(),
        profile_path: current_profile_path(state).display().to_string(),
        recent_profiles: config.recent_profiles.clone(),
        capture_ok: state.capture_ok.load(Ordering::Relaxed),
    }
}

fn lock_engine(
    state: &AppState,
) -> std::sync::MutexGuard<'_, crate::engine::Engine<crate::win::probe::AppSink>> {
    state.engine.lock().unwrap_or_else(|e| e.into_inner())
}

fn lock_profile(state: &AppState) -> std::sync::MutexGuard<'_, Profile> {
    state.profile.lock().unwrap_or_else(|e| e.into_inner())
}

/// Hand rows to the engine. Nothing is written to disk here: saving is the
/// user's explicit action, so an edit they did not save is an edit they can
/// still walk away from.
fn push_rows(state: &AppState, rows: Vec<Row>) -> Vec<Row> {
    lock_engine(state).set_rows(rows.clone());
    rows
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> StateDto {
    build_state(state.inner())
}

#[tauri::command]
pub fn set_rows(state: State<'_, AppState>, rows: Vec<Row>) -> Result<Vec<Row>, String> {
    {
        let config = state.config.lock().unwrap_or_else(|e| e.into_inner());
        validate_rows(&rows, &config.hotkeys)?;
    }
    let rows = {
        let mut profile = lock_profile(&state);
        profile.rows = rows;
        profile.normalize();
        profile.rows.clone()
    };
    Ok(push_rows(&state, rows))
}

#[tauri::command]
pub fn set_row_enabled(
    state: State<'_, AppState>,
    index: usize,
    enabled: bool,
) -> Result<Vec<Row>, String> {
    let rows = {
        let mut profile = lock_profile(&state);
        if index >= profile.rows.len() {
            return Err(format!("row {index} does not exist"));
        }
        profile.rows[index].enabled = enabled;
        profile.rows.clone()
    };
    Ok(push_rows(&state, rows))
}

#[tauri::command]
pub fn arm(state: State<'_, AppState>) {
    lock_engine(&state).arm();
}

#[tauri::command]
pub fn disarm(state: State<'_, AppState>) {
    lock_engine(&state).disarm();
}

#[tauri::command]
pub fn set_options(state: State<'_, AppState>, gate: bool) -> Result<(), String> {
    lock_engine(&state).set_gate(gate);
    Ok(())
}

/// Records a new bind for one hotkey slot. The UI captures the key, shows it and
/// asks the user to confirm before calling this.
#[tauri::command]
pub fn set_hotkey(
    state: State<'_, AppState>,
    slot: String,
    key: String,
) -> Result<StateDto, String> {
    // Take the locks one at a time, never nested, so no lock order can invert.
    let rows = lock_profile(&state).rows.clone();

    let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    let canonical = validate_hotkey(&key, &config.hotkeys, &slot)?;
    let mut candidate = config.hotkeys.clone();
    candidate.set(&slot, canonical);
    validate_rows(&rows, &candidate)?;
    config.hotkeys = candidate;
    config
        .save(&state.paths.config)
        .map_err(|e| format!("could not save config: {e:?}"))?;
    drop(config);

    Ok(build_state(state.inner()))
}

/// While the UI is capturing a key, the hotkeys must not fire: recording a bind
/// for the quit key must not quit the app.
#[tauri::command]
pub fn set_capture_mode(state: State<'_, AppState>, active: bool) {
    state.capturing.store(active, Ordering::Relaxed);
}

/// The profile in use. It starts beside the executable and moves whenever the
/// user opens, or saves as, a different file.
fn current_profile_path(state: &AppState) -> PathBuf {
    state
        .current_profile
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Native file dialog, so a profile can live wherever the user keeps it.
/// `kind` is `"open"` or `"save"`.
#[tauri::command]
pub fn pick_profile_file(
    state: State<'_, AppState>,
    kind: String,
) -> Result<Option<String>, String> {
    let current = current_profile_path(&state);
    let dialog = rfd::FileDialog::new()
        .add_filter("LE Turbo profile", &["json"])
        .set_directory(
            current
                .parent()
                .filter(|dir| dir.exists())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(".")),
        )
        .set_file_name(
            current
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("profile.json"),
        );
    let picked = if kind == "save" {
        dialog.save_file()
    } else {
        dialog.pick_file()
    };
    Ok(picked.map(|path| path.display().to_string()))
}

/// Point the app at a profile and remember it in the recent list.
fn use_profile(state: &AppState, path: &Path) {
    {
        let mut current = state
            .current_profile
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *current = path.to_path_buf();
    }
    if let Ok(mut config) = state.config.lock() {
        config.remember_profile(&path.display().to_string());
        let _ = config.save(&state.paths.config);
    }
}

#[tauri::command]
pub fn save_profile(state: State<'_, AppState>, path: Option<String>) -> Result<String, String> {
    let target = match path.as_deref().map(str::trim) {
        Some(given) if !given.is_empty() => PathBuf::from(given),
        _ => current_profile_path(&state),
    };
    {
        let profile = lock_profile(&state);
        profile
            .save(&target)
            .map_err(|e| format!("could not save {}: {e:?}", target.display()))?;
    }
    use_profile(&state, &target);
    Ok(target.display().to_string())
}

#[tauri::command]
pub fn load_profile(state: State<'_, AppState>, path: Option<String>) -> Result<Vec<Row>, String> {
    let target = match path.as_deref().map(str::trim) {
        Some(given) if !given.is_empty() => PathBuf::from(given),
        _ => current_profile_path(&state),
    };
    if !target.exists() {
        return Err(format!(
            "no profile saved at {}. Use Save as to choose another one.",
            target.display()
        ));
    }
    let loaded = Profile::load(&target)
        .map_err(|e| format!("could not read {}: {e:?}", target.display()))?;
    {
        let config = state.config.lock().unwrap_or_else(|e| e.into_inner());
        validate_rows(&loaded.rows, &config.hotkeys)?;
    }
    let rows = loaded.rows.clone();
    {
        let mut profile = lock_profile(&state);
        *profile = loaded;
    }
    use_profile(&state, &target);
    Ok(push_rows(&state, rows))
}

#[tauri::command]
pub fn reset_profile(state: State<'_, AppState>) -> Result<Vec<Row>, String> {
    let fresh = Profile::default_profile();
    let rows = fresh.rows.clone();
    {
        let mut profile = lock_profile(&state);
        *profile = fresh;
    }
    Ok(push_rows(&state, rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Hotkeys, Row, RowAction};

    fn row(key: &str) -> Row {
        Row {
            id: 0,
            key: key.into(),
            action: RowAction::Press,
            ms: 1000,
            enabled: true,
            condition: None,
        }
    }

    #[test]
    fn validate_rows_accepts_only_parseable_keys() {
        assert!(validate_rows(&[row("1"), row("Mouse Right")], &Hotkeys::default()).is_ok());
    }

    #[test]
    fn validate_rows_names_the_offending_row() {
        let err = validate_rows(&[row("1"), row("Banana")], &Hotkeys::default()).unwrap_err();
        assert!(err.contains("row 1"), "message was: {err}");
        assert!(err.contains("Banana"), "message was: {err}");
    }

    #[test]
    fn validate_rows_rejects_a_key_bound_to_a_hotkey() {
        let hotkeys = Hotkeys::default();
        for hotkey in ["F2", "F9"] {
            let err = validate_rows(&[row(hotkey)], &hotkeys).unwrap_err();
            assert!(
                err.contains(hotkey),
                "a row on {hotkey} must be refused, it would fire the global hotkey: {err}"
            );
        }
    }

    #[test]
    fn validate_rows_frees_a_key_once_the_hotkey_moves_away() {
        let mut hotkeys = Hotkeys::default();
        hotkeys.set("toggle", "Mouse Middle".into());
        assert!(
            validate_rows(&[row("F2")], &hotkeys).is_ok(),
            "F2 is an ordinary key again once nothing is bound to it"
        );
        assert!(
            validate_rows(&[row("Mouse Middle")], &hotkeys).is_err(),
            "and the new bind is protected instead"
        );
    }

    #[test]
    fn validate_hotkey_returns_the_canonical_name() {
        assert_eq!(
            validate_hotkey("mouse middle", &Hotkeys::default(), "toggle").unwrap(),
            "Mouse Middle"
        );
        assert_eq!(
            validate_hotkey(" f4 ", &Hotkeys::default(), "toggle").unwrap(),
            "F4"
        );
    }

    #[test]
    fn validate_hotkey_refuses_a_key_another_slot_already_uses() {
        let err = validate_hotkey("F9", &Hotkeys::default(), "toggle").unwrap_err();
        assert!(err.contains("F9"), "message was: {err}");
        assert!(err.contains("quit"), "message was: {err}");
    }

    #[test]
    fn validate_hotkey_allows_a_slot_to_keep_its_own_key() {
        assert!(validate_hotkey("F2", &Hotkeys::default(), "toggle").is_ok());
    }

    #[test]
    fn validate_hotkey_refuses_an_unsupported_key_and_an_unknown_slot() {
        assert!(validate_hotkey("Banana", &Hotkeys::default(), "toggle").is_err());
        assert!(validate_hotkey("F4", &Hotkeys::default(), "nope").is_err());
    }
}
