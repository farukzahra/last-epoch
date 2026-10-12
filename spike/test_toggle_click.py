"""THROWAWAY SPIKE TEST - the row toggle is clickable.

Regression guard for the reported bug ("o checkbox nao ta dando pra clicar").
Reads the toggle button centres from spike/state.json, really clicks them with
a synthetic left-click, and checks the enabled state flips in both directions.

Run: python spike/test_toggle_click.py
"""

import ctypes
import ctypes.wintypes as wintypes
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import le_turbo_spike as s  # noqa: E402

user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.WindowFromPoint.argtypes = (wintypes.POINT,)
user32.WindowFromPoint.restype = wintypes.HWND


def state():
    for _ in range(25):
        try:
            with open(s.STATE, encoding="utf-8") as fh:
                return json.load(fh)
        except (OSError, ValueError):
            time.sleep(0.1)
    return {}


def app_window_rect():
    """Root rect of the Tk window owning the toggle buttons."""
    st = state()
    if not st.get("toggles"):
        return None
    return st


def click(x, y):
    user32.SetCursorPos(int(x), int(y))
    time.sleep(0.15)
    s.press("Mouse Left")
    time.sleep(0.35)


def main():
    st = app_window_rect()
    if not st:
        print("FAIL: no state.json / no toggles - is the app running?")
        return 1

    toggles = st["toggles"]
    print(f"rows_on = {st.get('rows_on')} (of {len(toggles)})")
    print(
        f"first toggle centre = ({toggles[0]['x']}, {toggles[0]['y']})  on={toggles[0]['on']}"
    )

    before = st["rows_on"]

    print("\n1) clicking first toggle")
    click(toggles[0]["x"], toggles[0]["y"])
    st = state()
    after_on = st["rows_on"]
    ok_on = after_on == before + 1
    print(f"   rows_on {before} -> {after_on}  {'OK' if ok_on else '<-- FAIL'}")

    print("2) clicking it again (must go back)")
    st = state()
    t0 = st["toggles"][0]
    click(t0["x"], t0["y"])
    st = state()
    after_off = st["rows_on"]
    ok_off = after_off == before
    print(f"   rows_on {after_on} -> {after_off}  {'OK' if ok_off else '<-- FAIL'}")

    print("\nsummary:")
    print(f"   {'PASS' if ok_on else 'FAIL'}  click turns a row ON")
    print(f"   {'PASS' if ok_off else 'FAIL'}  click turns it back OFF")
    return 0 if (ok_on and ok_off) else 1


if __name__ == "__main__":
    sys.exit(main())
