"""End-to-end acceptance for the real app: does the UI actually drive SendInput
into Last Epoch?

Clicks the row-1 toggle, clicks START, hands the foreground back to the game,
waits, and leaves it there. The proof is the "sent" counter in the next
screenshot plus the game reacting.
"""

import ctypes
import ctypes.wintypes as wintypes
import sys
import time

sys.path.insert(0, r"C:\repo\last-epoch\spike")

user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.FindWindowExW.restype = wintypes.HWND


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


def click(x, y):
    user32.SetCursorPos(x, y)
    time.sleep(0.15)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP
    time.sleep(0.45)


app = find("LE Turbo")
game = find("Last Epoch")
if not app or not game:
    print(f"missing window: app={app} game={game}")
    sys.exit(1)

r = RECT()
user32.GetWindowRect(app, ctypes.byref(r))
print(f"app rect: ({r.left},{r.top})")

row1_toggle = (r.left + 78, r.top + 189)
start_button = (r.left + 81, r.top + 719)

print(f"clicking row 1 toggle at {row1_toggle}")
click(*row1_toggle)
print(f"clicking START at {start_button}")
click(*start_button)

user32.ShowWindow(game, 9)
user32.SetForegroundWindow(game)
time.sleep(0.5)
print("foreground now:", title_of(user32.GetForegroundWindow()))
print("waiting 4s with the game focused ...")
time.sleep(4.0)
print("done")
