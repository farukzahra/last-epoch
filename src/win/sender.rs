use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};

use crate::input::{InputError, RawSender};
use crate::keys::MouseButton;

/// The only place in the crate that talks to `SendInput`.
pub struct WinRawSender;

impl WinRawSender {
    pub fn new() -> Self {
        WinRawSender
    }

    fn submit(input: INPUT) -> Result<(), InputError> {
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if sent == 1 {
            Ok(())
        } else {
            Err(InputError::Rejected)
        }
    }
}

impl Default for WinRawSender {
    fn default() -> Self {
        Self::new()
    }
}

impl RawSender for WinRawSender {
    fn key(&mut self, vk: u16, up: bool, scancode: bool) -> Result<(), InputError> {
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        let (wvk, wscan) = if scancode {
            flags |= KEYEVENTF_SCANCODE;
            let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
            (VIRTUAL_KEY(0), scan)
        } else {
            (VIRTUAL_KEY(vk), 0)
        };

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: wvk,
                    wScan: wscan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        Self::submit(input)
    }

    fn mouse(&mut self, button: MouseButton, up: bool) -> Result<(), InputError> {
        let flag: MOUSE_EVENT_FLAGS = match (button, up) {
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEUP,
        };

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        Self::submit(input)
    }
}
