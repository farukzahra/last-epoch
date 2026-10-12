use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::State;

use crate::keys::Key;
use crate::profile::{Profile, Row, RowAction};
use crate::safety::focus_matches;
use crate::state::AppState;
use crate::win::probe::foreground_process_name;

/// Rejects a row list the engine could never act on, naming the first offender
/// so the UI can point at it.
pub fn validate_rows(rows: &[Row]) -> Result<(), String> {
    for (index, row) in rows.iter().enumerate() {
        if Key::parse(&row.key).is_err() {
            return Err(format!("row {index}: unsupported key \"{}\"", row.key));
        }
    }
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct StateDto {
    pub armed: bool,
    pub presses: u64,
    pub skipped: u64,
    pub rows: Vec<Row>,
    pub gate: bool,
    pub scancode: bool,
    pub game_focused: bool,
    pub expected_process: String,
    pub profile_path: String,
    pub capture_ok: bool,
}

pub fn build_state(state: &AppState) -> StateDto {
    let engine = state.engine.lock().unwrap_or_else(|e| e.into_inner());
    let profile = state.profile.lock().unwrap_or_else(|e| e.into_inner());
    let config = state.config.lock().unwrap_or_else(|e| e.into_inner());

    StateDto {
        armed: engine.is_armed(),
        presses: engine.presses(),
        skipped: engine.skipped(),
        rows: profile.rows.clone(),
        gate: engine.gate(),
        scancode: state.scancode.load(Ordering::Relaxed),
        game_focused: focus_matches(
            &config.expected_process,
            &foreground_process_name().unwrap_or_default(),
        ),
        expected_process: config.expected_process.clone(),
        profile_path: state.paths.profile.display().to_string(),
        capture_ok: state.capture_ok.load(Ordering::Relaxed),
    }
}

fn lock_engine(state: &AppState) -> std::sync::MutexGuard<'_, crate::engine::Engine<crate::win::probe::AppSink>> {
    state.engine.lock().unwrap_or_else(|e| e.into_inner())
}

fn lock_profile(state: &AppState) -> std::sync::MutexGuard<'_, Profile> {
    state.profile.lock().unwrap_or_else(|e| e.into_inner())
}

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
    validate_rows(&rows)?;
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
pub fn add_row(state: State<'_, AppState>) -> Result<Vec<Row>, String> {
    let rows = {
        let mut profile = lock_profile(&state);
        let next_id = profile.rows.len() as u32;
        profile.rows.push(Row {
            id: next_id,
            key: "1".into(),
            action: RowAction::Press,
            ms: 1000,
            enabled: false,
            condition: None,
        });
        profile.normalize();
        profile.rows.clone()
    };
    Ok(push_rows(&state, rows))
}

#[tauri::command]
pub fn remove_row(state: State<'_, AppState>, index: usize) -> Result<Vec<Row>, String> {
    let rows = {
        let mut profile = lock_profile(&state);
        if profile.rows.len() <= 1 {
            return Err("a profile must keep at least one row".into());
        }
        if index >= profile.rows.len() {
            return Err(format!("row {index} does not exist"));
        }
        profile.rows.remove(index);
        profile.normalize();
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
pub fn test_fire(state: State<'_, AppState>, key: String) -> Result<(), String> {
    let parsed = Key::parse(&key).map_err(|e| format!("unsupported key \"{key}\": {e:?}"))?;
    lock_engine(&state)
        .fire_once(parsed)
        .map_err(|e| format!("input rejected: {e:?}"))
}

#[tauri::command]
pub fn set_options(state: State<'_, AppState>, gate: bool, scancode: bool) -> Result<(), String> {
    {
        let mut engine = lock_engine(&state);
        engine.set_gate(gate);
        engine.set_scancode(scancode);
    }
    state.scancode.store(scancode, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn save_profile(state: State<'_, AppState>) -> Result<String, String> {
    let profile = lock_profile(&state);
    profile
        .save(&state.paths.profile)
        .map_err(|e| format!("could not save profile: {e:?}"))?;
    Ok(state.paths.profile.display().to_string())
}

#[tauri::command]
pub fn load_profile(state: State<'_, AppState>) -> Result<Vec<Row>, String> {
    let loaded = Profile::load(&state.paths.profile)
        .map_err(|e| format!("could not load profile: {e:?}"))?;
    let rows = loaded.rows.clone();
    {
        let mut profile = lock_profile(&state);
        *profile = loaded;
    }
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
    use crate::profile::{Row, RowAction};

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
        assert!(validate_rows(&[row("1"), row("Mouse Right")]).is_ok());
    }

    #[test]
    fn validate_rows_names_the_offending_row() {
        let err = validate_rows(&[row("1"), row("Banana")]).unwrap_err();
        assert!(err.contains("row 1"), "message was: {err}");
        assert!(err.contains("Banana"), "message was: {err}");
    }
}
