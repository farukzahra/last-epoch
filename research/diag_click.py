"""Diagnose why the synthetic click missed. Prints DPI, window rects and the
window actually under the computed toggle point."""

import ctypes
import ctypes.wintypes as wintypes
import json
import os
import sys
import time

sys.path.insert(0, r"C:\repo\last-epoch\spike")

user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.WindowFromPoint.argtypes = (wintypes.POINT,)
user32.WindowFromPoint.restype = wintypes.HWND
user32.GetForegroundWindow.restype = wintypes.HWND


class RECT(ctypes.Structure):
    _fields_ = [
        ("L", ctypes.c_long),
        ("T", ctypes.c_long),
        ("R", ctypes.c_long),
        ("B", ctypes.c_long),
    ]


user32.GetWindowRect.argtypes = (wintypes.HWND, ctypes.POINTER(RECT))

STATE = r"C:\repo\last-epoch\spike\state.json"


def title_of(hwnd):
    n = user32.GetWindowTextLengthW(hwnd)
    b = ctypes.create_unicode_buffer(n + 1)
    user32.GetWindowTextW(hwnd, b, n + 1)
    return b.value


def rect_of(hwnd):
    r = RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    return r.L, r.T, r.R, r.B


print(
    "this process DPI aware:",
    bool(user32.SetProcessDPIAware()),
    "| GetDpiForSystem:",
    end=" ",
)
try:
    user32.GetDpiForSystem.restype = wintypes.UINT
    print(user32.GetDpiForSystem())
except Exception as e:
    print("n/a", e)

with open(STATE, encoding="utf-8") as fh:
    st = json.load(fh)
w = st["window"]
print("state.json window :", w)
print("first toggle      :", st["toggles"][0])

# find the Tk window and print its real rect
found = []


@ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
def cb(h, l):
    t = title_of(h)
    if "LE Turbo" in t and user32.IsWindowVisible(h):
        found.append(h)
    return True


user32.EnumWindows(cb, 0)
for h in found:
    print("Tk window rect    :", rect_of(h), "title:", title_of(h))

pt = wintypes.POINT(st["toggles"][0]["x"], st["toggles"][0]["y"])
under = user32.WindowFromPoint(pt)
print("WindowFromPoint at the toggle point ->", hex(under), repr(title_of(under)))
under_rect = rect_of(under)
print("  that window rect:", under_rect)

# also probe a couple of sample points across the app window
if found:
    L, T, R, B = rect_of(found[0])
    print("\nsample probe inside the app window:")
    for frac_x, frac_y, label in (
        (0.5, 0.05, "titlebar"),
        (0.08, 0.30, "ON column"),
        (0.08, 0.55, "below rows"),
    ):
        x = int(L + (R - L) * frac_x)
        y = int(T + (B - T) * frac_y)
        hw = user32.WindowFromPoint(wintypes.POINT(x, y))
        print(f"  ({x},{y}) {label:12} -> {hex(hw)} {title_of(hw)!r}")
