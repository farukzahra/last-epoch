"""Does Last Epoch clip the OS cursor to its window while it is foreground?

If it does, mouse-button rows can never fire outside the game and the focus gate
is sufficient. If it does not, mouse rows need a cursor-containment check, because
SendInput mouse clicks go to whatever window is under the cursor, not the focused one.
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


user32.GetClipCursor.argtypes = (ctypes.POINTER(RECT),)
user32.GetWindowRect.argtypes = (wintypes.HWND, ctypes.POINTER(RECT))

found = []


@ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
def cb(h, l):
    if user32.IsWindowVisible(h):
        n = user32.GetWindowTextLengthW(h)
        if n:
            b = ctypes.create_unicode_buffer(n + 1)
            user32.GetWindowTextW(h, b, n + 1)
            if "last epoch" in b.value.lower():
                found.append((h, b.value))
    return True


user32.EnumWindows(cb, 0)
if not found:
    print("Last Epoch window not found - is the game running?")
    sys.exit(1)

hwnd, title = found[0]
wr = RECT()
user32.GetWindowRect(hwnd, ctypes.byref(wr))
print(
    f"game window      : {title!r} rect=({wr.left},{wr.top})-({wr.right},{wr.bottom})"
)

user32.ShowWindow(hwnd, 9)
user32.SetForegroundWindow(hwnd)
time.sleep(1.0)

n = user32.GetWindowTextLengthW(user32.GetForegroundWindow())
fb = ctypes.create_unicode_buffer(n + 1)
user32.GetWindowTextW(user32.GetForegroundWindow(), fb, n + 1)
print(f"foreground now   : {fb.value!r}")

cr = RECT()
ok = user32.GetClipCursor(ctypes.byref(cr))
print(f"GetClipCursor ok : {bool(ok)}")
if ok:
    print(f"clip rect        : ({cr.left},{cr.top})-({cr.right},{cr.bottom})")
    clipped_to_game = (cr.left, cr.top, cr.right, cr.bottom) == (
        wr.left,
        wr.top,
        wr.right,
        wr.bottom,
    )
    full_desktop = (cr.left, cr.top, cr.right, cr.bottom) == (-1920, 0, 1920, 1080)
    print(f"clipped to game  : {clipped_to_game}")
    print(f"clipped to whole virtual desktop: {full_desktop}")

# move the cursor to the far corner of the *other* monitor and see whether it sticks
user32.SetCursorPos(-1900, 20)
time.sleep(0.4)
pt = wintypes.POINT()
user32.GetCursorPos(ctypes.byref(pt))
print(f"cursor after SetCursorPos(-1900,20): ({pt.x},{pt.y})")
inside_game = wr.left <= pt.x <= wr.right and wr.top <= pt.y <= wr.bottom
print(f"cursor ended up inside the game window: {inside_game}")
print()
if inside_game:
    print(
        "VERDICT: the game keeps the cursor inside its window -> focus gate is enough"
    )
else:
    print(
        "VERDICT: cursor can sit outside the game -> mouse rows need a containment check"
    )
