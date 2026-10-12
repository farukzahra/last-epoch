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
            commands::add_row,
            commands::remove_row,
            commands::arm,
            commands::disarm,
            commands::test_fire,
            commands::set_options,
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
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(250));
                let dto = {
                    let state = handle.state::<state::AppState>();
                    commands::build_state(state.inner())
                };
                let _ = handle.emit("status", dto);
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
