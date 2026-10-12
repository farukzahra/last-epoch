"""Discriminating test: do synthetic clicks reach the app's UI at all?

Clicks "Fire once" (a large target whose effect is visible in the status line)
and the row-1 toggle, then leaves the result on screen for a screenshot.
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


def click(x, y, settle=0.6):
    user32.SetCursorPos(int(x), int(y))
    time.sleep(0.2)
    user32.mouse_event(0x0002, 0, 0, 0, 0)
    time.sleep(0.05)
    user32.mouse_event(0x0004, 0, 0, 0, 0)
    time.sleep(settle)


app = find("LE Turbo")
if not app:
    print("app window not found")
    sys.exit(1)

user32.ShowWindow(app, 9)
user32.SetForegroundWindow(app)
time.sleep(0.8)

r = RECT()
user32.GetWindowRect(app, ctypes.byref(r))
print(f"app rect ({r.left},{r.top}) size {r.right - r.left}x{r.bottom - r.top}")

# image-space offsets read off the last screenshot
fire_button = (r.left + 176, r.top + 459)
row1_toggle = (r.left + 70, r.top + 190)

print(f"click Fire once at {fire_button}")
click(*fire_button)
print(f"click row 1 toggle at {row1_toggle}")
click(*row1_toggle)
print("done")
