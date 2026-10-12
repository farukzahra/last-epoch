/// Does the foreground window belong to the expected game process?
///
/// Full-name equality only: a substring match would open the gate for
/// `Last Epoch Launcher.exe` or a browser tab titled "Last Epoch".
pub fn focus_matches(expected_process: &str, foreground_process: &str) -> bool {
    let expected = expected_process.trim();
    let actual = foreground_process.trim();
    if expected.is_empty() || actual.is_empty() {
        return false;
    }
    expected.eq_ignore_ascii_case(actual)
}

/// `rect` is `(left, top, right, bottom)` with the right and bottom edges
/// exclusive, matching Win32 `GetWindowRect`.
pub fn point_in_rect(x: i32, y: i32, rect: (i32, i32, i32, i32)) -> bool {
    let (left, top, right, bottom) = rect;
    x >= left && x < right && y >= top && y < bottom
}

pub trait FocusProbe {
    fn foreground_process(&self) -> Option<String>;
}

pub trait CursorProbe {
    fn cursor_pos(&self) -> (i32, i32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_matches_exact_process_name_case_insensitively() {
        assert!(focus_matches("Last Epoch.exe", "Last Epoch.exe"));
        assert!(focus_matches("Last Epoch.exe", "last epoch.exe"));
        assert!(focus_matches("Last Epoch.exe", "  LAST EPOCH.EXE  "));
    }

    #[test]
    fn focus_does_not_match_a_merely_similar_window() {
        assert!(!focus_matches("Last Epoch.exe", "Last Epoch Launcher.exe"));
        assert!(!focus_matches("Last Epoch.exe", "chrome.exe"));
        assert!(!focus_matches("Last Epoch.exe", ""));
    }

    #[test]
    fn point_in_rect_excludes_the_far_edges() {
        let r = (0, 0, 20, 20);
        assert!(point_in_rect(0, 0, r));
        assert!(point_in_rect(19, 19, r));
        assert!(!point_in_rect(20, 20, r));
        assert!(!point_in_rect(-1, 10, r));
    }

    #[test]
    fn point_in_rect_handles_a_negative_origin_monitor() {
        let r = (-1920, 0, 0, 1080);
        assert!(point_in_rect(-100, 500, r));
        assert!(!point_in_rect(10, 500, r));
    }
}
