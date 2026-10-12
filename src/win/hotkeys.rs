use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use crate::keys::Key;
use crate::profile::HOTKEY_SLOTS;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    /// One key for both directions: arms the presser when idle, disarms it when
    /// running. The user asked for a single toggle rather than two binds.
    Toggle,
    Quit,
}

/// What each one does, in the order `HOTKEY_SLOTS` defines.
const ACTIONS: [Hotkey; 2] = [Hotkey::Toggle, Hotkey::Quit];

/// `RegisterHotKey` is the wrong tool here: Windows never raises a registered
/// hotkey for injected input, and a message pump can swallow `WM_HOTKEY` before
/// a handler window sees it. Polling a single virtual key per slot is
/// focus-independent, and it is also what lets a mouse button be a hotkey.
pub struct EdgeDetector {
    down: [bool; 2],
}

impl Default for EdgeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeDetector {
    pub fn new() -> Self {
        EdgeDetector { down: [false; 2] }
    }

    pub fn poll(&mut self, states: [u16; 2]) -> Vec<Hotkey> {
        let mut fired = Vec::new();
        for (index, state) in states.iter().enumerate() {
            let down = state & 0x8000 != 0;
            // The low bit is set when the key went down since the last poll,
            // which is what catches a tap shorter than the poll interval.
            let tapped = state & 0x0001 != 0;
            if (down && !self.down[index]) || tapped {
                fired.push(ACTIONS[index]);
            }
            self.down[index] = down;
        }
        fired
    }
}

/// Reads the currently configured key for each slot. Re-read every poll so a
/// rebind takes effect immediately, without restarting the app.
fn poll_states(app: &AppHandle) -> [u16; 2] {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    let mut states = [0u16; 2];
    for (index, slot) in HOTKEY_SLOTS.iter().enumerate() {
        let Some(name) = config.hotkeys.get(slot) else {
            continue;
        };
        let Ok(key) = Key::parse(name) else {
            continue;
        };
        states[index] = unsafe { GetAsyncKeyState(key.poll_vk()) as u16 };
    }
    states
}

pub fn spawn_hotkey_loop(app: AppHandle, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut detector = EdgeDetector::new();
        while !stop.load(Ordering::Relaxed) {
            // The detector is fed even while the UI is capturing a bind, so the
            // key used to record a bind does not look like a fresh press once
            // capture ends. Only the actions are suspended.
            let fired = detector.poll(poll_states(&app));
            let capturing = {
                let state = app.state::<AppState>();
                state.capturing.load(Ordering::Relaxed)
            };

            if !capturing {
                for hotkey in fired {
                    match hotkey {
                        Hotkey::Toggle => {
                            let state = app.state::<AppState>();
                            let mut engine =
                                state.engine.lock().unwrap_or_else(|e| e.into_inner());
                            if engine.is_armed() {
                                engine.disarm();
                            } else {
                                engine.arm();
                            }
                        }
                        Hotkey::Quit => {
                            // Release first, then let the send loop wind down, so
                            // quitting can never leave a button down.
                            {
                                let state = app.state::<AppState>();
                                state
                                    .engine
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .disarm();
                                state.stop.store(true, Ordering::Relaxed);
                            }
                            app.exit(0);
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(30));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const UP: u16 = 0;
    const DOWN: u16 = 0x8000;
    const TAPPED: u16 = 0x0001;

    #[test]
    fn a_held_key_fires_once_on_the_transition() {
        let mut d = EdgeDetector::new();
        assert_eq!(d.poll([UP, UP]).len(), 0);
        assert_eq!(d.poll([DOWN, UP]), vec![Hotkey::Toggle]);
        assert_eq!(d.poll([DOWN, UP]), vec![], "still held must not refire");
        assert_eq!(d.poll([UP, UP]).len(), 0);
        assert_eq!(d.poll([DOWN, UP]), vec![Hotkey::Toggle]);
    }

    #[test]
    fn a_tap_shorter_than_the_poll_interval_is_still_caught() {
        let mut d = EdgeDetector::new();
        assert_eq!(d.poll([TAPPED, UP]), vec![Hotkey::Toggle]);
    }

    #[test]
    fn the_toggle_and_the_quit_key_are_mapped() {
        let mut d = EdgeDetector::new();
        assert_eq!(d.poll([DOWN, DOWN]), vec![Hotkey::Toggle, Hotkey::Quit]);
    }

    #[test]
    fn the_detector_slots_line_up_with_the_config_slots() {
        assert_eq!(HOTKEY_SLOTS.len(), ACTIONS.len());
    }
}
