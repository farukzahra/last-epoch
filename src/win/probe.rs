use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsWindowVisible,
};

use crate::engine::{Engine, GameProbe};
use crate::win::sender::WinRawSender;

/// The production input sink: `SendInput` plus held-key tracking.
pub type AppSink = crate::input::SendInputSink<WinRawSender>;

const FOREGROUND_TTL: Duration = Duration::from_millis(50);
const WINDOW_TTL: Duration = Duration::from_secs(1);
const TICK: Duration = Duration::from_millis(5);

/// `C:\Games\Last Epoch\Last Epoch.exe` -> `Last Epoch.exe`
pub fn process_name_from_path(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or("").to_string()
}

fn process_name_of_window(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        ok.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        Some(process_name_from_path(&full))
    }
}

pub fn foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }
        process_name_of_window(hwnd)
    }
}

struct EnumCtx {
    want: String,
    found: Option<HWND>,
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumCtx);
    if IsWindowVisible(hwnd).as_bool() {
        if let Some(name) = process_name_of_window(hwnd) {
            if name.eq_ignore_ascii_case(&ctx.want) {
                ctx.found = Some(hwnd);
                return BOOL(0); // stop enumerating
            }
        }
    }
    BOOL(1)
}

pub fn find_window_by_process(name: &str) -> Option<isize> {
    let mut ctx = EnumCtx {
        want: name.to_string(),
        found: None,
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(&mut ctx as *mut EnumCtx as isize),
        );
    }
    ctx.found.map(|h| h.0 as isize)
}

pub fn window_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(HWND(hwnd as *mut _), &mut rect).ok()? };
    Some((rect.left, rect.top, rect.right, rect.bottom))
}

pub fn cursor_position() -> (i32, i32) {
    let mut point = windows::Win32::Foundation::POINT::default();
    unsafe {
        if GetCursorPos(&mut point).is_err() {
            return (0, 0);
        }
    }
    (point.x, point.y)
}

#[derive(Default)]
struct ForegroundCache {
    name: Option<String>,
    at: Option<Instant>,
}

#[derive(Default)]
struct WindowCache {
    hwnd: Option<isize>,
    rect: Option<(i32, i32, i32, i32)>,
    at: Option<Instant>,
}

/// Caching Win32 view of the world. The engine ticks every 5 ms, so the
/// expensive lookups (process name, window search) are rate-limited.
pub struct WinProbe {
    expected_process: String,
    foreground: Mutex<ForegroundCache>,
    window: Mutex<WindowCache>,
}

impl WinProbe {
    pub fn new(expected_process: String) -> Self {
        WinProbe {
            expected_process,
            foreground: Mutex::new(ForegroundCache::default()),
            window: Mutex::new(WindowCache::default()),
        }
    }

    pub fn expected_process(&self) -> &str {
        &self.expected_process
    }

    /// The probe the UI uses for its status line.
    pub fn game_focused_by_name(name: &str) -> bool {
        foreground_process_name()
            .map(|fg| crate::safety::focus_matches(name, &fg))
            .unwrap_or(false)
    }
}

impl GameProbe for WinProbe {
    fn foreground_process(&self) -> Option<String> {
        let mut cache = match self.foreground.lock() {
            Ok(c) => c,
            Err(p) => p.into_inner(),
        };
        let fresh = cache
            .at
            .map(|at| at.elapsed() < FOREGROUND_TTL)
            .unwrap_or(false);
        if fresh {
            return cache.name.clone();
        }
        let name = foreground_process_name();
        cache.name = name.clone();
        cache.at = Some(Instant::now());
        name
    }

    fn cursor_pos(&self) -> (i32, i32) {
        cursor_position()
    }

    fn game_window_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let mut cache = match self.window.lock() {
            Ok(c) => c,
            Err(p) => p.into_inner(),
        };
        let fresh = cache
            .at
            .map(|at| at.elapsed() < WINDOW_TTL)
            .unwrap_or(false);
        if fresh {
            return cache.rect;
        }
        let hwnd = find_window_by_process(&self.expected_process);
        let rect = hwnd.and_then(window_rect);
        cache.hwnd = hwnd;
        cache.rect = rect;
        cache.at = Some(Instant::now());
        rect
    }
}

/// Drives `Engine::tick` on its own thread. The final `release_all` on the way
/// out is deliberate: whatever happens, no input stays held.
pub fn spawn_send_loop(
    engine: Arc<Mutex<Engine<AppSink>>>,
    probe: Arc<WinProbe>,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let start = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            let now_ms = start.elapsed().as_millis() as u64;
            {
                let mut guard = match engine.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                guard.tick(probe.as_ref(), now_ms);
            }
            std::thread::sleep(TICK);
        }
        let mut guard = match engine.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let _ = guard.disarm();
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_file_name_from_an_image_path() {
        assert_eq!(
            process_name_from_path(r"C:\Games\Last Epoch\Last Epoch.exe"),
            "Last Epoch.exe"
        );
        assert_eq!(process_name_from_path("Last Epoch.exe"), "Last Epoch.exe");
        assert_eq!(process_name_from_path(""), "");
    }
}
