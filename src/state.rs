use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::engine::Engine;
use crate::profile::{AppConfig, Profile};
use crate::win::probe::{AppSink, WinProbe};
use crate::win::sender::WinRawSender;

/// Everything the app keeps on disk, beside the executable, so a copy of the
/// folder carries the whole setup.
///
/// Profiles live in their own `profiles` subfolder: that is where every profile
/// dialog opens, and it gives the user one predictable place to keep several.
/// `config.json` stays flat because there is only ever one of it.
pub struct Paths {
    pub config: PathBuf,
    pub profiles_dir: PathBuf,
    /// The profile in use on a fresh start.
    pub profile: PathBuf,
}

impl Paths {
    pub fn next_to_exe() -> Paths {
        let dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        let profiles_dir = dir.join("profiles");
        Paths {
            config: dir.join("config.json"),
            profile: profiles_dir.join("profile.json"),
            profiles_dir,
        }
    }
}

/// Carries a profile from the old flat layout, beside the executable, into the
/// profiles folder. Runs once: it refuses when the target already exists, so a
/// folder copy is never overwritten by a stale flat one.
pub fn adopt_legacy_profile(legacy: &Path, target: &Path) -> bool {
    if target.exists() || !legacy.exists() {
        return false;
    }
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::copy(legacy, target).is_ok()
}

pub struct AppState {
    pub engine: Arc<Mutex<Engine<AppSink>>>,
    pub probe: Arc<WinProbe>,
    pub config: Mutex<AppConfig>,
    pub profile: Mutex<Profile>,
    pub paths: Paths,
    pub stop: Arc<AtomicBool>,
    /// The profile in use. Starts at `paths.profile`, and moves when the user
    /// opens, or saves as, a different file.
    pub current_profile: Mutex<PathBuf>,
    /// Latest capture sanity result: false means we found the game but its
    /// frame came back black, which is what exclusive fullscreen looks like.
    pub capture_ok: AtomicBool,
    /// True while the UI is recording a new hotkey, so the hotkeys stand down.
    pub capturing: AtomicBool,
}

impl AppState {
    pub fn new() -> AppState {
        let paths = Paths::next_to_exe();

        // Make the profiles folder exist, then adopt a profile left over from the
        // flat layout so an existing setup continues where it left off.
        let _ = std::fs::create_dir_all(&paths.profiles_dir);
        if let Some(dir) = paths.profiles_dir.parent() {
            adopt_legacy_profile(&dir.join("profile.json"), &paths.profile);
        }

        let config = AppConfig::load_or_default(&paths.config);
        let profile = Profile::load(&paths.profile).unwrap_or_else(|_| Profile::default_profile());
        let current_profile = paths.profile.clone();

        // Write the config out so the user can see and edit the expected process.
        let _ = config.save(&paths.config);

        let sink = AppSink::new(WinRawSender::new(), false);
        let engine = Engine::new(sink, config.expected_process.clone(), profile.rows.clone());

        AppState {
            engine: Arc::new(Mutex::new(engine)),
            probe: Arc::new(WinProbe::new(config.expected_process.clone())),
            config: Mutex::new(config),
            profile: Mutex::new(profile),
            paths,
            stop: Arc::new(AtomicBool::new(false)),
            current_profile: Mutex::new(current_profile),
            capture_ok: AtomicBool::new(true),
            capturing: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("le_turbo_state_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_profiles_folder_sits_beside_the_executable() {
        let paths = Paths::next_to_exe();
        assert_eq!(paths.profiles_dir.file_name().unwrap(), "profiles");
        assert_eq!(
            paths.profile.parent().unwrap(),
            paths.profiles_dir,
            "the default profile belongs inside the folder"
        );
        assert_eq!(paths.config.parent(), paths.profiles_dir.parent());
    }

    #[test]
    fn a_flat_profile_is_carried_into_the_folder_once() {
        let dir = scratch("adopt");
        let legacy = dir.join("profile.json");
        let target = dir.join("profiles").join("profile.json");
        std::fs::write(&legacy, "{\"rows\":[]}").unwrap();

        assert!(adopt_legacy_profile(&legacy, &target), "first call adopts");
        assert!(target.exists());

        // A second run must never clobber what the user has since saved.
        std::fs::write(&target, "{\"kept\":true}").unwrap();
        assert!(!adopt_legacy_profile(&legacy, &target));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "{\"kept\":true}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adopting_does_nothing_when_there_is_no_legacy_profile() {
        let dir = scratch("no_legacy");
        let target = dir.join("profiles").join("profile.json");
        assert!(!adopt_legacy_profile(&dir.join("profile.json"), &target));
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
