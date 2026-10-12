"""
THROWAWAY SPIKE TEST - does SendInput actually land inside Last Epoch?

Method: bring the Last Epoch window to the foreground, snapshot the skill-bar
region, press '1' via SendInput, snapshot again, and count changed pixels.
A non-trivial change means the game consumed the synthetic input.

Run: python spike/test_sendinput_in_game.py
"""

import ctypes
import ctypes.wintypes as wintypes
import sys
import time
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import le_turbo_spike as s  # noqa: E402

user32 = s.user32
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)

SRCCOPY = 0x00CC0020
DIB_RGB_COLORS = 0

user32.SetProcessDPIAware()

# region of the skill bar (1920x1080 reference)
RX, RY, RW, RH = 740, 975, 320, 80


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wintypes.DWORD),
        ("biWidth", wintypes.LONG),
        ("biHeight", wintypes.LONG),
        ("biPlanes", wintypes.WORD),
        ("biBitCount", wintypes.WORD),
        ("biCompression", wintypes.DWORD),
        ("biSizeImage", wintypes.DWORD),
        ("biXPelsPerMeter", wintypes.LONG),
        ("biYPelsPerMeter", wintypes.LONG),
        ("biClrUsed", wintypes.DWORD),
        ("biClrImportant", wintypes.DWORD),
    ]


class BITMAPINFO(ctypes.Structure):
    _fields_ = [("bmiHeader", BITMAPINFOHEADER), ("bmiColors", wintypes.DWORD * 3)]


for fn, args in (
    (
        gdi32.BitBlt,
        (
            wintypes.HDC,
            ctypes.c_int,
            ctypes.c_int,
            ctypes.c_int,
            ctypes.c_int,
            wintypes.HDC,
            ctypes.c_int,
            ctypes.c_int,
            wintypes.DWORD,
        ),
    ),
    (gdi32.CreateCompatibleDC, (wintypes.HDC,)),
    (gdi32.CreateCompatibleBitmap, (wintypes.HDC, ctypes.c_int, ctypes.c_int)),
    (gdi32.SelectObject, (wintypes.HDC, wintypes.HANDLE)),
    (gdi32.DeleteObject, (wintypes.HANDLE,)),
    (gdi32.DeleteDC, (wintypes.HDC,)),
    (
        gdi32.GetDIBits,
        (
            wintypes.HDC,
            wintypes.HANDLE,
            wintypes.UINT,
            wintypes.UINT,
            ctypes.c_void_p,
            ctypes.POINTER(BITMAPINFO),
            wintypes.UINT,
        ),
    ),
):
    fn.argtypes = args
user32.GetDC.argtypes = (wintypes.HWND,)
user32.GetDC.restype = wintypes.HDC
user32.ReleaseDC.argtypes = (wintypes.HWND, wintypes.HDC)


def grab(x, y, w, h):
    hdc = user32.GetDC(None)
    memdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    old = gdi32.SelectObject(memdc, bmp)
    gdi32.BitBlt(memdc, 0, 0, w, h, hdc, x, y, SRCCOPY)

    bi = BITMAPINFO()
    bi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.bmiHeader.biWidth = w
    bi.bmiHeader.biHeight = -h  # top-down
    bi.bmiHeader.biPlanes = 1
    bi.bmiHeader.biBitCount = 32
    bi.bmiHeader.biCompression = 0

    buf = (ctypes.c_char * (w * h * 4))()
    gdi32.GetDIBits(memdc, bmp, 0, h, buf, ctypes.byref(bi), DIB_RGB_COLORS)

    gdi32.SelectObject(memdc, old)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(memdc)
    user32.ReleaseDC(None, hdc)
    return bytes(buf)


def find_last_epoch_window():
    found = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, _):
        if user32.IsWindowVisible(hwnd):
            n = user32.GetWindowTextLengthW(hwnd)
            if n:
                buf = ctypes.create_unicode_buffer(n + 1)
                user32.GetWindowTextW(hwnd, buf, n + 1)
                if "last epoch" in buf.value.lower():
                    found.append(hwnd)
        return True

    user32.EnumWindows(cb, 0)
    return found[0] if found else None


def changed_pixels(a, b, threshold=12):
    n = 0
    for i in range(0, len(a), 4):
        if (
            abs(a[i] - b[i]) + abs(a[i + 1] - b[i + 1]) + abs(a[i + 2] - b[i + 2])
            > threshold
        ):
            n += 1
    return n


def main():
    hwnd = find_last_epoch_window()
    if not hwnd:
        print("Last Epoch window not found")
        return

    user32.ShowWindow(hwnd, 9)
    user32.SetForegroundWindow(hwnd)
    time.sleep(1.2)
    print("foreground now:", repr(s.foreground_title()))
    if "last epoch" not in s.foreground_title().lower():
        print(
            "!! Last Epoch is not foreground; aborting so we do not type into another app"
        )
        return

    for label, scancode in (("VK", False), ("SCANCODE", True)):
        before = grab(RX, RY, RW, RH)
        s.press("1", scancode)
        time.sleep(0.35)
        after = grab(RX, RY, RW, RH)
        diff = changed_pixels(before, after)
        total = RW * RH
        print(
            f"{label:9} press '1' -> changed pixels: {diff}/{total} ({100.0 * diff / total:.1f}%)"
        )
        time.sleep(0.8)

    print(
        "\nInterpretation: a few hundred+ changed pixels = the game reacted to the synthetic key."
    )


if __name__ == "__main__":
    main()
