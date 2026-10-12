"""Drive the hotkey rebind flow with a real middle-mouse press.

Clicks Change on the arm slot, presses the middle mouse button (the key the user
wants), and leaves it in the "detected, waiting for confirmation" state.
"""

import ctypes
import ctypes.wintypes as wintypes
import sys
import time

user32 = ctypes.WinDLL("user32", use_last_error=True)


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


def click(x, y, button="left", settle=0.5):
    user32.SetCursorPos(int(x), int(y))
    time.sleep(0.2)
    down, up = {
        "left": (0x0002, 0x0004),
        "middle": (0x0020, 0x0040),
    }[button]
    user32.mouse_event(down, 0, 0, 0, 0)
    time.sleep(0.06)
    user32.mouse_event(up, 0, 0, 0, 0)
    time.sleep(settle)


app = find("LE Turbo")
if not app:
    print("app not running")
    sys.exit(1)

user32.ShowWindow(app, 9)
user32.SetForegroundWindow(app)
time.sleep(0.8)

r = RECT()
user32.GetWindowRect(app, ctypes.byref(r))
print(f"app rect ({r.left},{r.top})")

# "Change" on the arm slot, read off the previous screenshot: offset (975, 165)
print("clicking Change on the arm slot")
click(r.left + 975, r.top + 165)

time.sleep(0.4)
print("pressing the MIDDLE mouse button")
click(r.left + 500, r.top + 400, button="middle", settle=0.8)
print("done - capture should now show the detected key")
