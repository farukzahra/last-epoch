use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::engine::Engine;
use crate::profile::{AppConfig, Profile};
use crate::win::probe::{AppSink, WinProbe};
use crate::win::sender::WinRawSender;

/// `profile.json` and `config.json` live beside the executable, so the app is
/// portable: copy the folder, keep the setup.
pub struct Paths {
    pub profile: PathBuf,
    pub config: PathBuf,
}

impl Paths {
    pub fn next_to_exe() -> Paths {
        let dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        Paths {
            profile: dir.join("profile.json"),
            config: dir.join("config.json"),
        }
    }
}

pub struct AppState {
    pub engine: Arc<Mutex<Engine<AppSink>>>,
    pub probe: Arc<WinProbe>,
    pub config: Mutex<AppConfig>,
    pub profile: Mutex<Profile>,
    pub paths: Paths,
    pub stop: Arc<AtomicBool>,
    pub scancode: AtomicBool,
}

impl AppState {
    pub fn new() -> AppState {
        let paths = Paths::next_to_exe();
        let config = AppConfig::load_or_default(&paths.config);
        let profile = Profile::load(&paths.profile).unwrap_or_else(|_| Profile::default_profile());

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
            scancode: AtomicBool::new(false),
        }
    }
}
