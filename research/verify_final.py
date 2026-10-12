"""Capture open, then: (a) middle press on the hotkey row binds, (b) confirm,
(c) START still works. One run, one screenshot of the whole window."""

import ctypes
import ctypes.wintypes as wintypes
import struct
import sys
import time
import zlib

sys.path.insert(0, r"C:\repo\last-epoch\spike")
import test_sendinput_in_game as t  # noqa: E402

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


def write_png(path, w, h, bgra):
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        row = bgra[y * w * 4 : (y + 1) * w * 4]
        for x in range(w):
            raw += bytes((row[x * 4 + 2], row[x * 4 + 1], row[x * 4], row[x * 4 + 3]))

    def chunk(tag, data):
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    with open(path, "wb") as fh:
        fh.write(
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(bytes(raw), 6))
            + chunk(b"IEND", b"")
        )


sw, sh = user32.GetSystemMetrics(0), user32.GetSystemMetrics(1)


def click(x, y, middle=False, times=1):
    for _ in range(times):
        user32.mouse_event(
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
            int(x * 65535 / (sw - 1)),
            int(y * 65535 / (sh - 1)),
            0,
            0,
        )
        time.sleep(0.3)
        down, up = (0x0020, 0x0040) if middle else (0x0002, 0x0004)
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

# window may not be active yet: click a neutral spot first
click(r.left + 350, r.top + 600)
print("1) row 1 ON")
click(r.left + 78, r.top + 189)
print("2) Change, then middle press over the hotkey row")
click(r.left + 975, r.top + 165)
click(r.left + 760, r.top + 165, middle=True)
print("3) Use this")
click(r.left + 907, r.top + 165)
print("4) START")
click(r.left + 81, r.top + 718)

write_png(
    "C:\\repo\\last-epoch\\research\\final_state.png",
    1036,
    640,
    t.grab(r.left, r.top, 1036, 640),
)
print("done")
