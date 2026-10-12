use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    Arm,
    Disarm,
    Quit,
}

/// Fixed in v1; stored in app config so they can be rebound later without a
/// schema change on the profile.
pub const HOTKEYS: &[(Hotkey, &str, u16)] = &[
    (Hotkey::Arm, "F2", 0x71),
    (Hotkey::Disarm, "F3", 0x72),
    (Hotkey::Quit, "F9", 0x78),
];

/// `RegisterHotKey` is the wrong tool here: Windows never raises a registered
/// hotkey for injected input, and a Tauri/Tk message pump can swallow
/// `WM_HOTKEY` before a handler window sees it. Polling is focus-independent.
pub struct EdgeDetector {
    down: [bool; 3],
}

impl Default for EdgeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeDetector {
    pub fn new() -> Self {
        EdgeDetector { down: [false; 3] }
    }

    pub fn poll(&mut self, states: [u16; 3]) -> Vec<Hotkey> {
        let mut fired = Vec::new();
        for (index, state) in states.iter().enumerate() {
            let down = state & 0x8000 != 0;
            // The low bit is set when the key went down since the last poll,
            // which is what catches a tap shorter than the poll interval.
            let tapped = state & 0x0001 != 0;
            if (down && !self.down[index]) || tapped {
                fired.push(HOTKEYS[index].0);
            }
            self.down[index] = down;
        }
        fired
    }
}

fn read_states() -> [u16; 3] {
    let mut states = [0u16; 3];
    for (index, (_, _, vk)) in HOTKEYS.iter().enumerate() {
        states[index] = unsafe { GetAsyncKeyState(*vk as i32) as u16 };
    }
    states
}

pub fn spawn_hotkey_loop(app: AppHandle, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut detector = EdgeDetector::new();
        while !stop.load(Ordering::Relaxed) {
            for hotkey in detector.poll(read_states()) {
                match hotkey {
                    Hotkey::Arm => {
                        let state = app.state::<AppState>();
                        state
                            .engine
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .arm();
                    }
                    Hotkey::Disarm => {
                        let state = app.state::<AppState>();
                        state
                            .engine
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .disarm();
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
        assert_eq!(d.poll([UP, UP, UP]).len(), 0);
        assert_eq!(d.poll([DOWN, UP, UP]), vec![Hotkey::Arm]);
        assert_eq!(d.poll([DOWN, UP, UP]), vec![], "still held must not refire");
        assert_eq!(d.poll([UP, UP, UP]).len(), 0);
        assert_eq!(d.poll([DOWN, UP, UP]), vec![Hotkey::Arm]);
    }

    #[test]
    fn a_tap_shorter_than_the_poll_interval_is_still_caught() {
        let mut d = EdgeDetector::new();
        assert_eq!(d.poll([TAPPED, UP, UP]), vec![Hotkey::Arm]);
    }

    #[test]
    fn all_three_hotkeys_are_mapped() {
        let mut d = EdgeDetector::new();
        assert_eq!(
            d.poll([DOWN, DOWN, DOWN]),
            vec![Hotkey::Arm, Hotkey::Disarm, Hotkey::Quit]
        );
    }
}
