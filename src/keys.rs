use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    Vk(u16),
    Mouse(MouseButton),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseKeyError {
    Empty,
    Unknown(String),
}

/// The keys offered in the UI, in display order. `Key::name` resolves through
/// this list, so every entry round-trips through `Key::parse`.
pub const KEY_CHOICES: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "Q", "W", "E", "R", "T", "A", "S", "D", "F", "G", "Z", "X", "C",
    "V", "B", "Space", "Tab", "Enter", "Esc", "Mouse Left", "Mouse Right", "Mouse Middle", "F1",
    "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15", "F16",
    "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
];

const NAMED: &[(&str, u16)] = &[
    ("Space", 0x20),
    ("Tab", 0x09),
    ("Enter", 0x0D),
    ("Esc", 0x1B),
];

impl Key {
    pub fn parse(name: &str) -> Result<Key, ParseKeyError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ParseKeyError::Empty);
        }
        let lower = trimmed.to_ascii_lowercase();

        match lower.as_str() {
            "mouse left" => return Ok(Key::Mouse(MouseButton::Left)),
            "mouse right" => return Ok(Key::Mouse(MouseButton::Right)),
            "mouse middle" => return Ok(Key::Mouse(MouseButton::Middle)),
            _ => {}
        }

        for (named, vk) in NAMED {
            if lower == named.to_ascii_lowercase() {
                return Ok(Key::Vk(*vk));
            }
        }

        if let Some(rest) = lower.strip_prefix('f') {
            if let Ok(n) = rest.parse::<u16>() {
                if (1..=24).contains(&n) {
                    return Ok(Key::Vk(0x6F + n));
                }
            }
        }

        if trimmed.len() == 1 {
            let ch = trimmed.chars().next().unwrap().to_ascii_uppercase();
            if ch.is_ascii_digit() {
                return Ok(Key::Vk(0x30 + (ch as u16 - '0' as u16)));
            }
            if ch.is_ascii_uppercase() {
                return Ok(Key::Vk(ch as u16));
            }
        }

        Err(ParseKeyError::Unknown(trimmed.to_string()))
    }

    /// The canonical spelling, taken from `KEY_CHOICES` so it always round-trips.
    pub fn name(&self) -> String {
        for candidate in KEY_CHOICES {
            if let Ok(parsed) = Key::parse(candidate) {
                if parsed == *self {
                    return (*candidate).to_string();
                }
            }
        }
        match self {
            Key::Vk(vk) => format!("VK({vk:#04x})"),
            Key::Mouse(button) => format!("Mouse {button:?}"),
        }
    }

    pub fn is_mouse(&self) -> bool {
        matches!(self, Key::Mouse(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_digits_letters_and_named_keys() {
        assert_eq!(Key::parse("1").unwrap(), Key::Vk(0x31));
        assert_eq!(Key::parse("Q").unwrap(), Key::Vk(0x51));
        assert_eq!(Key::parse("q").unwrap(), Key::Vk(0x51));
        assert_eq!(Key::parse("F9").unwrap(), Key::Vk(0x78));
        assert_eq!(Key::parse("Space").unwrap(), Key::Vk(0x20));
    }

    #[test]
    fn parses_mouse_buttons_case_insensitively() {
        assert_eq!(
            Key::parse("Mouse Right").unwrap(),
            Key::Mouse(MouseButton::Right)
        );
        assert_eq!(
            Key::parse("mouse left").unwrap(),
            Key::Mouse(MouseButton::Left)
        );
        assert_eq!(
            Key::parse("MOUSE MIDDLE").unwrap(),
            Key::Mouse(MouseButton::Middle)
        );
    }

    #[test]
    fn rejects_empty_and_unknown_names() {
        assert_eq!(Key::parse(""), Err(ParseKeyError::Empty));
        assert_eq!(Key::parse("   "), Err(ParseKeyError::Empty));
        assert_eq!(
            Key::parse("Banana"),
            Err(ParseKeyError::Unknown("Banana".into()))
        );
    }

    #[test]
    fn name_round_trips_through_parse() {
        for name in KEY_CHOICES {
            let parsed = Key::parse(name).expect(name);
            assert_eq!(
                parsed.name(),
                *name,
                "name() must return the canonical spelling"
            );
        }
    }

    #[test]
    fn is_mouse_distinguishes_mouse_from_keyboard() {
        assert!(Key::parse("Mouse Left").unwrap().is_mouse());
        assert!(!Key::parse("1").unwrap().is_mouse());
    }
}
