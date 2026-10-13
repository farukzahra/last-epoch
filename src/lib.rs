pub mod commands;
pub mod engine;
pub mod input;
pub mod keys;
pub mod profile;
pub mod safety;
pub mod schedule;
pub mod state;
pub mod win;

use std::time::Duration;

use engine::GameProbe;
use tauri::{Emitter, Manager};

pub fn greeting() -> &'static str {
    "LE Turbo"
}

pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::set_rows,
            commands::set_row_enabled,
            commands::arm,
            commands::disarm,
            commands::set_options,
            commands::set_hotkey,
            commands::set_capture_mode,
            commands::save_profile,
            commands::load_profile,
            commands::reset_profile
        ])
        .setup(|app| {
            {
                let state = app.state::<state::AppState>();
                win::probe::spawn_send_loop(
                    state.engine.clone(),
                    state.probe.clone(),
                    state.stop.clone(),
                );
                win::hotkeys::spawn_hotkey_loop(app.handle().clone(), state.stop.clone());
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let own_process = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
                    .unwrap_or_default();

                let mut hwnd: Option<isize> = None;
                let mut tick: u64 = 0;

                loop {
                    std::thread::sleep(Duration::from_millis(250));
                    tick += 1;

                    // The window exists by now, but not necessarily at setup time.
                    if hwnd.is_none() {
                        if let Some(found) = win::probe::find_window_by_process(&own_process) {
                            win::window::enable_dark_titlebar(found);
                            let open_other = {
                                let state = handle.state::<state::AppState>();
                                let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
                                cfg.open_on_other_monitor
                            };
                            win::window::place_on_monitor(found, open_other);
                            hwnd = Some(found);
                        }
                    } else if tick % 4 == 0 {
                        // A fullscreen game can steal the top spot; re-assert it.
                        win::window::set_topmost(hwnd.unwrap(), true);
                    }

                    let dto = {
                        let state = handle.state::<state::AppState>();
                        if tick % 20 == 0 {
                            let ok = match state.probe.game_window_rect() {
                                Some(rect) => win::capture::capture_region(rect)
                                    .map(|px| !win::capture::is_mostly_black(&px, 95))
                                    .unwrap_or(false),
                                None => true, // game not running: nothing to warn about
                            };
                            state.capture_ok.store(ok, std::sync::atomic::Ordering::Relaxed);
                        }
                        commands::build_state(state.inner())
                    };
                    let _ = handle.emit("status", dto);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Whatever way the window goes away, nothing stays held down.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.app_handle().try_state::<state::AppState>() {
                    state
                        .engine
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .disarm();
                    state.stop.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start LE Turbo");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_is_not_empty() {
        assert_eq!(greeting(), "LE Turbo");
    }
}
