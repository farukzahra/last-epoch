"""Middle-mouse rebind, with a genuine cursor move before the press."""

import ctypes
import ctypes.wintypes as wintypes
import sys
import time

user32 = ctypes.WinDLL("user32", use_last_error=True)
MOUSEEVENTF_MOVE = 0x0001
MOUSEEVENTF_ABSOLUTE = 0x8000


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", ctypes.c_long),
        ("top", ctypes.c_long),
        ("right", ctypes.c_long),
        ("bottom", ctypes.c_long),
    ]


user32.GetWindowRect.argtypes = (wintypes.HWND, ctypes.POINTER(RECT))


def title_of(hwnd):
    n = user32.GetWindowTextLengthW(hwnd)
    b = ctypes.create_unicode_buffer(n + 1)
    user32.GetWindowTextW(hwnd, b, n + 1)
    return b.value


def find(title):
    found = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def cb(h, l):
        if user32.IsWindowVisible(h) and title_of(h) == title:
            found.append(h)
        return True

    user32.EnumWindows(cb, 0)
    return found[0] if found else None


def move(x, y):
    """Absolute mouse_event move: Chromium needs a move to update its target."""
    sw = user32.GetSystemMetrics(0)
    sh = user32.GetSystemMetrics(1)
    user32.mouse_event(
        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
        int(x * 65535 / (sw - 1)),
        int(y * 65535 / (sh - 1)),
        0,
        0,
    )
    time.sleep(0.3)


def click(x, y, button="left"):
    move(x, y)
    down, up = {"left": (0x0002, 0x0004), "middle": (0x0020, 0x0040)}[button]
    user32.mouse_event(down, 0, 0, 0, 0)
    time.sleep(0.1)
    user32.mouse_event(up, 0, 0, 0, 0)
    time.sleep(0.8)


app = find("LE Turbo")
if not app:
    print("app not running")
    sys.exit(1)

user32.ShowWindow(app, 9)
user32.SetForegroundWindow(app)
time.sleep(1.0)

r = RECT()
user32.GetWindowRect(app, ctypes.byref(r))
print(f"app rect ({r.left},{r.top})")

print("1) Change on the arm slot")
click(r.left + 975, r.top + 165)

print("2) middle mouse press over the panel")
click(r.left + 500, r.top + 400, button="middle")

print("3) confirm")
click(r.left + 907, r.top + 165)

print("flow finished")
