use std::path::Path;

use serde::{Deserialize, Serialize};

pub const PROFILE_VERSION: u32 = 1;
pub const MIN_MS: u32 = 10;

/// How a row delivers its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RowAction {
    /// Down and up once per interval, repeating.
    Press,
    /// Down once when armed, up when disarmed.
    Hold,
}

/// Reserved for v2 (buff detection). v1 never constructs one, but the field
/// exists in the schema so shipping v2 does not invalidate v1 files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Condition {
    IconAbsent {
        rect: [i32; 4],
        reference_png: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    #[serde(default)]
    pub id: u32,
    pub key: String,
    pub action: RowAction,
    pub ms: u32,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub version: u32,
    pub rows: Vec<Row>,
}

/// A shipped default row: 1000 ms, disabled.
fn default_row(id: u32, key: &str, action: RowAction) -> Row {
    Row {
        id,
        key: key.into(),
        action,
        ms: 1000,
        enabled: false,
        condition: None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hotkeys {
    pub toggle: String,
    pub quit: String,
}

/// The two bindable slots, in a stable order. Names, not indices, are the stable
/// thing: the config file and the UI both speak them.
pub const HOTKEY_SLOTS: [&str; 2] = ["toggle", "quit"];

impl Hotkeys {
    /// `(slot, key name)` for each slot, in `HOTKEY_SLOTS` order.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &String)> + '_ {
        HOTKEY_SLOTS.iter().map(move |slot| {
            let key = match *slot {
                "toggle" => &self.toggle,
                _ => &self.quit,
            };
            (*slot, key)
        })
    }

    pub fn get(&self, slot: &str) -> Option<&String> {
        match slot {
            "toggle" => Some(&self.toggle),
            "quit" => Some(&self.quit),
            _ => None,
        }
    }

    /// Returns false for an unknown slot, so a bad request cannot invent a slot.
    pub fn set(&mut self, slot: &str, key: String) -> bool {
        match slot {
            "toggle" => self.toggle = key,
            "quit" => self.quit = key,
            _ => return false,
        }
        true
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Hotkeys {
            toggle: "F2".into(),
            quit: "F9".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub expected_process: String,
    pub open_on_other_monitor: bool,
    pub hotkeys: Hotkeys,
    /// Most recently used profile paths, newest first, so the UI can offer them
    /// instead of making the user browse the filesystem every time.
    /// A missing field in an older config falls back to the default (empty).
    pub recent_profiles: Vec<String>,
}

/// How many paths the recent list keeps.
pub const MAX_RECENT_PROFILES: usize = 5;

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            expected_process: "Last Epoch.exe".into(),
            open_on_other_monitor: true,
            hotkeys: Hotkeys::default(),
            recent_profiles: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Remembers a profile path: newest first, no duplicates, capped, and blank
    /// paths ignored so a cancelled dialog cannot add an empty entry.
    pub fn remember_profile(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            return;
        }
        self.recent_profiles
            .retain(|known| !known.eq_ignore_ascii_case(path));
        self.recent_profiles.insert(0, path.to_string());
        self.recent_profiles.truncate(MAX_RECENT_PROFILES);
    }
}

#[derive(Debug)]
pub enum ProfileError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl From<std::io::Error> for ProfileError {
    fn from(e: std::io::Error) -> Self {
        ProfileError::Io(e)
    }
}

impl From<serde_json::Error> for ProfileError {
    fn from(e: serde_json::Error) -> Self {
        ProfileError::Json(e)
    }
}

impl Profile {
    /// The user's in-game remap: skills on `1`-`4`, then the **left** button as a
    /// `Hold` row for movement, then the **right** button as the fifth skill bind.
    /// Every row ships disabled.
    pub fn default_profile() -> Profile {
        let rows = vec![
            default_row(0, "1", RowAction::Press),
            default_row(1, "2", RowAction::Press),
            default_row(2, "3", RowAction::Press),
            default_row(3, "4", RowAction::Press),
            default_row(4, "Mouse Left", RowAction::Hold),
            default_row(5, "Mouse Right", RowAction::Press),
            default_row(6, "Space", RowAction::Press),
        ];
        Profile {
            version: PROFILE_VERSION,
            rows,
        }
    }

    /// Assign sequential ids and clamp every interval to the floor. `version`
    /// is deliberately left alone so a file from a newer build keeps its tag.
    pub fn normalize(&mut self) {
        for (i, row) in self.rows.iter_mut().enumerate() {
            row.id = i as u32;
            row.ms = row.ms.max(MIN_MS);
        }
    }

    pub fn load(path: &Path) -> Result<Profile, ProfileError> {
        let text = std::fs::read_to_string(path)?;
        let mut profile: Profile = serde_json::from_str(&text)?;
        profile.normalize();
        Ok(profile)
    }

    pub fn save(&self, path: &Path) -> Result<(), ProfileError> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl AppConfig {
    pub fn default_config() -> AppConfig {
        AppConfig::default()
    }

    /// A missing *or malformed* config must never stop the app from starting.
    pub fn load_or_default(path: &Path) -> AppConfig {
        let Ok(text) = std::fs::read_to_string(path) else {
            return AppConfig::default();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return AppConfig::default();
        };
        let mut config: AppConfig = serde_json::from_value(value.clone()).unwrap_or_default();

        // v0.1 shipped separate `arm` and `disarm` binds. They collapsed into one
        // toggle, so carry the old arm key over rather than resetting the user.
        let hotkeys = value.get("hotkeys");
        let had_toggle = hotkeys.and_then(|h| h.get("toggle")).is_some();
        if !had_toggle {
            if let Some(legacy_arm) = hotkeys.and_then(|h| h.get("arm")).and_then(|v| v.as_str()) {
                config.hotkeys.toggle = legacy_arm.to_string();
            }
        }
        config
    }

    pub fn save(&self, path: &Path) -> Result<(), ProfileError> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("le_turbo_test_{name}.json"));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn default_profile_matches_the_users_remap() {
        let p = Profile::default_profile();
        assert_eq!(p.version, PROFILE_VERSION);
        let keys: Vec<&str> = p.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["1", "2", "3", "4", "Mouse Left", "Mouse Right", "Space"],
            "skills on 1-4, then the left button for movement, then the right button, then Space"
        );
        assert!(p.rows.iter().all(|r| !r.enabled), "rows must ship disabled");
        assert!(p.rows[..4].iter().all(|r| r.action == RowAction::Press));
        assert_eq!(
            p.rows[4].action,
            RowAction::Hold,
            "the left button must be a Hold row so it can drive movement"
        );
        assert_eq!(
            p.rows[5].action,
            RowAction::Press,
            "the right button is the fifth skill bind, so it stays a tap"
        );
        assert!(p.rows.iter().all(|r| r.ms == 1000));
    }

    #[test]
    fn round_trips_through_disk() {
        let path = temp_path("roundtrip");
        let mut p = Profile::default_profile();
        p.rows[0].enabled = true;
        p.rows[1].action = RowAction::Hold;
        p.rows[1].key = "Mouse Left".into();
        p.save(&path).unwrap();
        let loaded = Profile::load(&path).unwrap();
        assert_eq!(loaded, p);
    }

    #[test]
    fn ignores_unknown_fields_from_a_newer_version() {
        let path = temp_path("newer");
        std::fs::write(
            &path,
            r#"{"version":2,"rows":[{"id":0,"key":"1","action":"Press","ms":500,
               "enabled":false,"future_field":true}],"future_top_level":1}"#,
        )
        .unwrap();
        let loaded = Profile::load(&path).unwrap();
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(loaded.rows[0].ms, 500);
    }

    #[test]
    fn a_v1_file_without_condition_loads_and_writes_without_condition() {
        let path = temp_path("nocond");
        std::fs::write(
            &path,
            r#"{"version":1,"rows":[{"id":0,"key":"1","action":"Press","ms":1000,"enabled":false}]}"#,
        )
        .unwrap();
        let loaded = Profile::load(&path).unwrap();
        assert!(loaded.rows[0].condition.is_none());
        let text = serde_json::to_string(&loaded).unwrap();
        assert!(
            !text.contains("condition"),
            "absent condition must not be written"
        );
    }

    #[test]
    fn normalize_clamps_interval_and_fills_ids() {
        let mut p = Profile {
            version: PROFILE_VERSION,
            rows: vec![
                Row {
                    id: 99,
                    key: "1".into(),
                    action: RowAction::Press,
                    ms: 0,
                    enabled: false,
                    condition: None,
                },
                Row {
                    id: 99,
                    key: "2".into(),
                    action: RowAction::Press,
                    ms: 3,
                    enabled: false,
                    condition: None,
                },
            ],
        };
        p.normalize();
        assert_eq!(p.rows[0].id, 0);
        assert_eq!(p.rows[1].id, 1);
        assert_eq!(p.rows[0].ms, MIN_MS);
        assert_eq!(p.rows[1].ms, MIN_MS);
    }

    #[test]
    fn a_missing_config_file_yields_defaults() {
        let path = temp_path("cfg_missing");
        let cfg = AppConfig::load_or_default(&path);
        assert_eq!(cfg.expected_process, "Last Epoch.exe");
        assert!(cfg.open_on_other_monitor);
    }

    #[test]
    fn config_round_trips_through_disk() {
        let path = temp_path("cfg_roundtrip");
        let mut cfg = AppConfig::default_config();
        cfg.expected_process = "OtherGame.exe".into();
        cfg.save(&path).unwrap();
        assert_eq!(AppConfig::load_or_default(&path), cfg);
    }

    #[test]
    fn default_config_pins_the_documented_hotkeys() {
        let cfg = AppConfig::default_config();
        assert_eq!(cfg.hotkeys.toggle, "F2");
        assert_eq!(cfg.hotkeys.quit, "F9");
    }

    #[test]
    fn hotkeys_can_be_read_and_written_by_slot_name() {
        let mut hotkeys = Hotkeys::default();
        assert_eq!(hotkeys.get("toggle").map(String::as_str), Some("F2"));
        assert!(hotkeys.set("toggle", "Mouse Middle".into()));
        assert_eq!(
            hotkeys.get("toggle").map(String::as_str),
            Some("Mouse Middle")
        );
        assert!(
            !hotkeys.set("nope", "F4".into()),
            "an unknown slot must not be written"
        );
        assert_eq!(hotkeys.get("nope"), None);
    }

    #[test]
    fn hotkeys_iter_reports_the_slots_in_a_stable_order() {
        let hotkeys = Hotkeys::default();
        let slots: Vec<&str> = hotkeys.iter().map(|(slot, _)| slot).collect();
        assert_eq!(slots, vec!["toggle", "quit"]);
    }

    #[test]
    fn a_hotkey_edit_survives_a_round_trip_through_disk() {
        let path = temp_path("cfg_hotkeys");
        let mut cfg = AppConfig::default_config();
        cfg.hotkeys.set("toggle", "Mouse Middle".into());
        cfg.save(&path).unwrap();
        let loaded = AppConfig::load_or_default(&path);
        assert_eq!(loaded.hotkeys.toggle, "Mouse Middle");
    }

    #[test]
    fn a_legacy_arm_bind_becomes_the_toggle() {
        let path = temp_path("cfg_legacy");
        std::fs::write(
            &path,
            r#"{"expected_process":"Last Epoch.exe","open_on_other_monitor":true,
                "hotkeys":{"arm":"Mouse Middle","disarm":"F3","quit":"F9"}}"#,
        )
        .unwrap();
        let loaded = AppConfig::load_or_default(&path);
        assert_eq!(
            loaded.hotkeys.toggle, "Mouse Middle",
            "the old arm key must carry over to the new toggle slot"
        );
        assert_eq!(loaded.hotkeys.quit, "F9");
    }

    #[test]
    fn recent_profiles_keep_the_newest_first_without_duplicates() {
        let mut cfg = AppConfig::default_config();
        cfg.remember_profile(r"C:\a.json");
        cfg.remember_profile(r"C:\b.json");
        cfg.remember_profile(r"C:\a.json");
        assert_eq!(cfg.recent_profiles, vec![r"C:\a.json", r"C:\b.json"]);
    }

    #[test]
    fn recent_profiles_are_capped_and_blank_paths_are_ignored() {
        let mut cfg = AppConfig::default_config();
        for i in 0..(MAX_RECENT_PROFILES + 2) {
            cfg.remember_profile(&format!(r"C:\p{i}.json"));
        }
        cfg.remember_profile("   ");
        assert_eq!(cfg.recent_profiles.len(), MAX_RECENT_PROFILES);
        assert_eq!(cfg.recent_profiles[0], r"C:\p6.json");
    }

    #[test]
    fn a_config_from_before_the_recent_list_still_loads() {
        let path = temp_path("cfg_no_recents");
        std::fs::write(
            &path,
            r#"{"expected_process":"Last Epoch.exe","open_on_other_monitor":true,
                "hotkeys":{"toggle":"F2","quit":"F9"}}"#,
        )
        .unwrap();
        let loaded = AppConfig::load_or_default(&path);
        assert!(loaded.recent_profiles.is_empty());
    }
}
