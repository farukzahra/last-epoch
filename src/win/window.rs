use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOSIZE, SWP_SHOWWINDOW};

pub struct MonitorInfo {
    pub primary: bool,
    pub work: (i32, i32, i32, i32),
}

unsafe extern "system" fn monitor_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let found = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if GetMonitorInfoW(hmonitor, &mut info).as_bool() {
        found.push(MonitorInfo {
            primary: info.dwFlags & 1 != 0, // MONITORINFOF_PRIMARY
            work: (
                info.rcWork.left,
                info.rcWork.top,
                info.rcWork.right,
                info.rcWork.bottom,
            ),
        });
    }
    BOOL(1)
}

/// Primary monitor first.
pub fn list_monitors() -> Vec<MonitorInfo> {
    let mut found: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_proc),
            LPARAM(&mut found as *mut Vec<MonitorInfo> as isize),
        );
    }
    found.sort_by_key(|m| if m.primary { 0 } else { 1 });
    found
}

/// Park the window on the non-primary monitor when asked and one exists. The
/// window is raised once, not made topmost: it behaves like any other window
/// afterwards and can go behind whatever the user is doing.
pub fn place_on_monitor(hwnd: isize, other_monitor: bool) -> Option<(i32, i32)> {
    let monitors = list_monitors();
    if monitors.is_empty() {
        return None;
    }
    let target = if other_monitor && monitors.len() > 1 {
        monitors.iter().find(|m| !m.primary).unwrap_or(&monitors[0])
    } else {
        &monitors[0]
    };
    let (left, top, _, _) = target.work;
    let at = (left + 16, top + 16);
    unsafe {
        let _ = SetWindowPos(
            HWND(hwnd as *mut _),
            HWND_TOP,
            at.0,
            at.1,
            0,
            0,
            SWP_NOSIZE | SWP_SHOWWINDOW,
        );
    }
    Some(at)
}

/// Best effort: ignore failure on older Windows builds.
pub fn enable_dark_titlebar(hwnd: isize) {
    let enabled: i32 = 1;
    for attribute in [
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMWINDOWATTRIBUTE(19), // the pre-20H1 spelling
    ] {
        unsafe {
            let result = DwmSetWindowAttribute(
                HWND(hwnd as *mut _),
                attribute,
                &enabled as *const i32 as *const _,
                std::mem::size_of::<i32>() as u32,
            );
            if result.is_ok() {
                return;
            }
        }
    }
}
