use crate::input::{InputSink};
use crate::keys::Key;
use crate::profile::{Row, RowAction};
use crate::safety::{focus_matches, point_in_rect};
use crate::schedule::{due_rows, RowClock};

/// Everything the engine needs to know about the outside world. Implemented by
/// Win32 in `win::probe` and by a fake in the tests.
pub trait GameProbe {
    fn foreground_process(&self) -> Option<String>;
    fn cursor_pos(&self) -> (i32, i32);
    fn game_window_rect(&self) -> Option<(i32, i32, i32, i32)>;
}

pub struct Engine<S: InputSink> {
    rows: Vec<Row>,
    clocks: Vec<RowClock>,
    sink: S,
    armed: bool,
    gate: bool,
    expected_process: String,
    presses: u64,
    skipped: u64,
    hold_armed: bool,
}

impl<S: InputSink> Engine<S> {
    pub fn new(sink: S, expected_process: String, rows: Vec<Row>) -> Self {
        let clocks = rows.iter().map(|_| RowClock::default()).collect();
        Engine {
            rows,
            clocks,
            sink,
            armed: false,
            // The focus gate is on by default: a fresh install must not inject
            // into whatever window happens to be in front.
            gate: true,
            expected_process,
            presses: 0,
            skipped: 0,
            hold_armed: false,
        }
    }

    /// Replacing the rows while a hold is in force must not leave that input
    /// down, so the hold is dropped and re-applied from the new list.
    pub fn set_rows(&mut self, rows: Vec<Row>) {
        if self.hold_armed {
            let _ = self.sink.release_all();
            self.hold_armed = false;
        }
        self.clocks = rows.iter().map(|_| RowClock::default()).collect();
        self.rows = rows;
    }

    pub fn arm(&mut self) {
        self.armed = true;
    }

    pub fn disarm(&mut self) {
        self.armed = false;
        self.hold_armed = false;
        let _ = self.sink.release_all();
    }

    pub fn set_gate(&mut self, gate: bool) {
        self.gate = gate;
    }

    pub fn presses(&self) -> u64 {
        self.presses
    }

    pub fn skipped(&self) -> u64 {
        self.skipped
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    #[cfg(test)]
    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn tick<P: GameProbe>(&mut self, probe: &P, now_ms: u64) {
        let focused = focus_matches(
            &self.expected_process,
            &probe.foreground_process().unwrap_or_default(),
        );
        // Focus and the gate are tracked separately: the gate governs whether we
        // may inject at all, but a held button must be released the moment the
        // game loses the foreground *even when the gate is off*.
        let may_inject = focused || !self.gate;

        if self.hold_armed && !focused {
            let _ = self.sink.release_all();
            self.hold_armed = false;
        }

        if !self.armed || !may_inject {
            return;
        }

        // Holding an input is only ever safe while the game holds the
        // foreground: unlike a press, a held button has no window of its own and
        // will drag/select whatever it lands on next. So holds require focus
        // even when the gate is off; only press rows follow the gate.
        if !self.hold_armed && focused {
            for row in self.rows.iter() {
                if row.action != RowAction::Hold || !row.enabled {
                    continue;
                }
                match Key::parse(&row.key) {
                    Ok(key) => {
                        if self.sink.hold(key).is_err() {
                            self.skipped += 1;
                        }
                    }
                    Err(_) => self.skipped += 1,
                }
            }
            self.hold_armed = true;
        }

        let rect = probe.game_window_rect();
        let cursor = probe.cursor_pos();

        for i in due_rows(&self.rows, &self.clocks, now_ms) {
            let key = match Key::parse(&self.rows[i].key) {
                Ok(key) => key,
                Err(_) => {
                    self.skipped += 1;
                    continue;
                }
            };

            // Mouse events go to whatever window is under the cursor, not the
            // focused one, so a mouse row only fires with the cursor on the game.
            if key.is_mouse() {
                let inside = match rect {
                    Some(r) => point_in_rect(cursor.0, cursor.1, r),
                    None => false,
                };
                if !inside {
                    self.skipped += 1;
                    continue;
                }
            }

            if self.sink.press(key).is_ok() {
                self.clocks[i].last_fire_ms = Some(now_ms);
                self.presses += 1;
            } else {
                self.skipped += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputError;
    use crate::keys::MouseButton;
    use crate::profile::{Row, RowAction};

    #[derive(Default)]
    struct FakeSink {
        calls: Vec<String>,
        held: Vec<Key>,
    }

    impl InputSink for FakeSink {
        fn press(&mut self, key: Key) -> Result<(), InputError> {
            self.calls.push(format!("press:{key:?}"));
            Ok(())
        }
        fn hold(&mut self, key: Key) -> Result<(), InputError> {
            self.calls.push(format!("hold:{key:?}"));
            if !self.held.contains(&key) {
                self.held.push(key);
            }
            Ok(())
        }
        fn release(&mut self, key: Key) -> Result<(), InputError> {
            self.calls.push(format!("release:{key:?}"));
            self.held.retain(|k| *k != key);
            Ok(())
        }
        fn release_all(&mut self) -> Result<(), InputError> {
            self.calls.push("release_all".into());
            self.held.clear();
            Ok(())
        }
        fn held(&self) -> &[Key] {
            &self.held
        }
    }

    struct FakeProbe {
        fg: Option<String>,
        cursor: (i32, i32),
        rect: Option<(i32, i32, i32, i32)>,
    }

    impl GameProbe for FakeProbe {
        fn foreground_process(&self) -> Option<String> {
            self.fg.clone()
        }
        fn cursor_pos(&self) -> (i32, i32) {
            self.cursor
        }
        fn game_window_rect(&self) -> Option<(i32, i32, i32, i32)> {
            self.rect
        }
    }

    fn game() -> FakeProbe {
        FakeProbe {
            fg: Some("Last Epoch.exe".into()),
            cursor: (960, 540),
            rect: Some((0, 0, 1920, 1080)),
        }
    }

    fn other() -> FakeProbe {
        FakeProbe {
            fg: Some("explorer.exe".into()),
            cursor: (0, 0),
            rect: None,
        }
    }

    fn press_row(key: &str, ms: u32) -> Row {
        Row {
            id: 0,
            key: key.into(),
            action: RowAction::Press,
            ms,
            enabled: true,
            condition: None,
        }
    }

    fn hold_row(key: &str) -> Row {
        Row {
            id: 0,
            key: key.into(),
            action: RowAction::Hold,
            ms: 1000,
            enabled: true,
            condition: None,
        }
    }

    #[test]
    fn armed_press_row_fires_a_press() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.arm();
        e.tick(&game(), 0);
        assert_eq!(e.presses(), 1);
        assert!(e.sink().calls.iter().any(|c| c.starts_with("press:")));
    }

    #[test]
    fn a_disarmed_engine_injects_nothing() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.tick(&game(), 0);
        assert_eq!(e.presses(), 0);
        assert!(e.sink().calls.is_empty());
    }

    #[test]
    fn the_gate_blocks_injection_when_the_game_is_not_foreground() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.arm();
        e.tick(&other(), 0);
        assert_eq!(e.presses(), 0);
        assert!(e.sink().calls.is_empty());
    }

    #[test]
    fn a_window_with_a_similar_name_does_not_open_the_gate() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.arm();
        let launcher = FakeProbe {
            fg: Some("Last Epoch Launcher.exe".into()),
            cursor: (0, 0),
            rect: None,
        };
        e.tick(&launcher, 0);
        assert_eq!(e.presses(), 0);
    }

    #[test]
    fn arming_a_hold_row_presses_down_once_and_disarm_releases_everything() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![hold_row("Mouse Left")],
        );
        e.arm();
        e.tick(&game(), 0);
        e.tick(&game(), 5000);
        assert_eq!(e.sink().held().len(), 1, "hold must not repeat");
        e.disarm();
        assert!(
            e.sink().held().is_empty(),
            "disarm must release the held button"
        );
    }

    #[test]
    fn losing_the_game_while_holding_releases_immediately() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![hold_row("Mouse Left")],
        );
        e.arm();
        e.tick(&game(), 0);
        assert_eq!(e.sink().held().len(), 1);
        e.tick(&other(), 100);
        assert!(
            e.sink().held().is_empty(),
            "watchdog must release on focus loss"
        );
    }

    #[test]
    fn the_watchdog_releases_even_with_the_gate_disabled() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![hold_row("Mouse Left")],
        );
        e.set_gate(false);
        e.arm();
        e.tick(&game(), 0);
        e.tick(&other(), 100);
        assert!(e.sink().held().is_empty());
    }

    #[test]
    fn a_mouse_row_is_skipped_when_the_cursor_is_off_the_game() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("Mouse Left", 1000)],
        );
        e.arm();
        let off = FakeProbe {
            fg: Some("Last Epoch.exe".into()),
            cursor: (-1900, 20),
            rect: Some((0, 0, 1920, 1080)),
        };
        e.tick(&off, 0);
        assert_eq!(e.presses(), 0);
        assert_eq!(e.skipped(), 1);
    }

    #[test]
    fn a_mouse_row_fires_when_the_cursor_is_on_the_game() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("Mouse Right", 1000)],
        );
        e.arm();
        e.tick(&game(), 0);
        assert_eq!(e.presses(), 1);
    }

    #[test]
    fn a_keyboard_row_fires_even_when_the_cursor_is_elsewhere() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.arm();
        let off = FakeProbe {
            fg: Some("Last Epoch.exe".into()),
            cursor: (-1900, 20),
            rect: Some((0, 0, 1920, 1080)),
        };
        e.tick(&off, 0);
        assert_eq!(e.presses(), 1, "containment applies to mouse rows only");
    }

    #[test]
    fn a_mouse_row_is_skipped_when_the_game_window_is_unknown() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("Mouse Left", 1000)],
        );
        e.arm();
        let unknown = FakeProbe {
            fg: Some("Last Epoch.exe".into()),
            cursor: (100, 100),
            rect: None,
        };
        e.tick(&unknown, 0);
        assert_eq!(e.presses(), 0, "never click blind");
        assert_eq!(e.skipped(), 1);
    }

    #[test]
    fn an_unparseable_key_is_counted_as_skipped_and_never_injected() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("Banana", 1000)],
        );
        e.arm();
        e.tick(&game(), 0);
        assert_eq!(e.presses(), 0);
        assert_eq!(e.skipped(), 1);
    }

    #[test]
    fn the_interval_is_respected_across_ticks() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.arm();
        e.tick(&game(), 0);
        e.tick(&game(), 500);
        assert_eq!(e.presses(), 1);
        e.tick(&game(), 1000);
        assert_eq!(e.presses(), 2);
    }

    #[test]
    fn set_rows_replaces_the_row_list_and_its_clocks() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![press_row("1", 1000)],
        );
        e.arm();
        e.tick(&game(), 0);
        e.set_rows(vec![press_row("2", 1000)]);
        e.tick(&game(), 10);
        assert_eq!(e.presses(), 2, "the new row is due immediately");
    }

    #[test]
    fn changing_rows_while_holding_releases_the_hold() {
        let mut e = Engine::new(
            FakeSink::default(),
            "Last Epoch.exe".into(),
            vec![hold_row("Mouse Left")],
        );
        e.arm();
        e.tick(&game(), 0);
        assert_eq!(e.sink().held().len(), 1);
        e.set_rows(vec![hold_row("Mouse Right")]);
        assert!(
            e.sink().held().is_empty(),
            "a profile change must not leave a button down"
        );
        e.tick(&game(), 10);
        assert_eq!(e.sink().held().len(), 1, "the new hold takes effect");
    }
}
