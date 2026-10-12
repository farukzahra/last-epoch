use crate::keys::{Key, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// The OS refused some or all of the submitted input events.
    Rejected,
}

/// The thin, untestable layer. Everything above it is exercised by fakes.
pub trait RawSender {
    fn key(&mut self, vk: u16, up: bool, scancode: bool) -> Result<(), InputError>;
    fn mouse(&mut self, button: MouseButton, up: bool) -> Result<(), InputError>;
}

pub trait InputSink {
    fn press(&mut self, key: Key) -> Result<(), InputError>;
    fn hold(&mut self, key: Key) -> Result<(), InputError>;
    fn release(&mut self, key: Key) -> Result<(), InputError>;
    fn release_all(&mut self) -> Result<(), InputError>;
    fn held(&self) -> &[Key];
}

/// An `InputSink` that tracks which keys it is currently holding, so nothing
/// can be left down: `release_all` is the safety net the spec demands.
pub struct SendInputSink<B: RawSender> {
    backend: B,
    scancode: bool,
    held: Vec<Key>,
}

impl<B: RawSender> SendInputSink<B> {
    pub fn new(backend: B, scancode: bool) -> Self {
        SendInputSink {
            backend,
            scancode,
            held: Vec::new(),
        }
    }

    pub fn set_scancode(&mut self, scancode: bool) {
        self.scancode = scancode;
    }

    #[cfg(test)]
    pub fn backend(&self) -> &B {
        &self.backend
    }

    fn raw(&mut self, key: Key, up: bool) -> Result<(), InputError> {
        match key {
            Key::Vk(vk) => self.backend.key(vk, up, self.scancode),
            Key::Mouse(button) => self.backend.mouse(button, up),
        }
    }
}

impl<B: RawSender> InputSink for SendInputSink<B> {
    fn press(&mut self, key: Key) -> Result<(), InputError> {
        self.raw(key, false)?;
        self.raw(key, true)
    }

    fn hold(&mut self, key: Key) -> Result<(), InputError> {
        if self.held.contains(&key) {
            return Ok(());
        }
        self.raw(key, false)?;
        self.held.push(key);
        Ok(())
    }

    fn release(&mut self, key: Key) -> Result<(), InputError> {
        if let Some(pos) = self.held.iter().position(|k| *k == key) {
            self.held.remove(pos);
        }
        self.raw(key, true)
    }

    fn release_all(&mut self) -> Result<(), InputError> {
        let held = std::mem::take(&mut self.held);
        let mut outcome = Ok(());
        for key in held.iter().rev() {
            if let Err(e) = self.raw(*key, true) {
                outcome = Err(e);
            }
        }
        outcome
    }

    fn held(&self) -> &[Key] {
        &self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Key;

    #[derive(Default)]
    struct FakeRaw {
        calls: Vec<String>,
    }

    impl RawSender for FakeRaw {
        fn key(&mut self, vk: u16, up: bool, scancode: bool) -> Result<(), InputError> {
            self.calls
                .push(format!("key:{vk:#x}:{}:{scancode}", if up { "up" } else { "down" }));
            Ok(())
        }
        fn mouse(&mut self, button: MouseButton, up: bool) -> Result<(), InputError> {
            self.calls
                .push(format!("mouse:{button:?}:{}", if up { "up" } else { "down" }));
            Ok(())
        }
    }

    #[test]
    fn press_sends_down_then_up_and_holds_nothing() {
        let mut s = SendInputSink::new(FakeRaw::default(), false);
        s.press(Key::Vk(0x31)).unwrap();
        assert_eq!(
            s.backend().calls,
            vec!["key:0x31:down:false", "key:0x31:up:false"]
        );
        assert!(s.held().is_empty());
    }

    #[test]
    fn hold_records_the_key_and_release_removes_it() {
        let mut s = SendInputSink::new(FakeRaw::default(), false);
        let left = Key::parse("Mouse Left").unwrap();
        s.hold(left).unwrap();
        assert_eq!(s.held(), &[left]);
        assert_eq!(s.backend().calls, vec!["mouse:Left:down"]);
        s.release(left).unwrap();
        assert!(s.held().is_empty());
        assert_eq!(s.backend().calls[1], "mouse:Left:up");
    }

    #[test]
    fn holding_twice_does_not_duplicate_or_resend() {
        let mut s = SendInputSink::new(FakeRaw::default(), false);
        let one = Key::Vk(0x31);
        s.hold(one).unwrap();
        s.hold(one).unwrap();
        assert_eq!(s.held(), &[one]);
        assert_eq!(s.backend().calls.len(), 1);
    }

    #[test]
    fn release_all_releases_every_held_key_in_reverse_order() {
        let mut s = SendInputSink::new(FakeRaw::default(), false);
        s.hold(Key::Vk(0x31)).unwrap();
        s.hold(Key::parse("Mouse Left").unwrap()).unwrap();
        s.release_all().unwrap();
        assert!(s.held().is_empty());
        assert_eq!(
            s.backend().calls,
            vec![
                "key:0x31:down:false",
                "mouse:Left:down",
                "mouse:Left:up",
                "key:0x31:up:false"
            ]
        );
    }

    #[test]
    fn release_all_with_nothing_held_touches_the_backend_zero_times() {
        let mut s = SendInputSink::new(FakeRaw::default(), false);
        s.release_all().unwrap();
        assert!(s.backend().calls.is_empty());
    }

    #[test]
    fn scancode_mode_is_forwarded_to_the_backend() {
        let mut s = SendInputSink::new(FakeRaw::default(), true);
        s.press(Key::Vk(0x31)).unwrap();
        assert!(s.backend().calls[0].ends_with(":true"));
    }
}
