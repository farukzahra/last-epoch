"""Isolate the Change click: click it and nothing else."""

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


app = find("LE Turbo")
if not app:
    print("app not running")
    sys.exit(1)

user32.ShowWindow(app, 9)
user32.SetForegroundWindow(app)
time.sleep(0.8)

r = RECT()
user32.GetWindowRect(app, ctypes.byref(r))
print(f"app rect ({r.left},{r.top}) {r.right - r.left}x{r.bottom - r.top}")

x, y = r.left + 975, r.top + 165
user32.SetCursorPos(x, y)
time.sleep(0.4)
pt = wintypes.POINT()
user32.GetCursorPos(ctypes.byref(pt))
print(f"cursor placed at ({pt.x},{pt.y})")

user32.mouse_event(0x0002, 0, 0, 0, 0)
time.sleep(0.08)
user32.mouse_event(0x0004, 0, 0, 0, 0)
time.sleep(0.9)
print("clicked Change on the arm slot only")
