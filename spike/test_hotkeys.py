"""THROWAWAY SPIKE TEST - verify the global hotkeys actually work.

The app polls GetAsyncKeyState (F2/F3/F9), so injected keys are observed exactly
like physical ones. This reads spike/state.json (written every ~60 ms) to check
the state transitions.

Run: python spike/test_hotkeys.py
"""

import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import le_turbo_spike as s  # noqa: E402

STATE = s.STATE


def state():
    for _ in range(25):
        try:
            with open(STATE, encoding="utf-8") as fh:
                return json.load(fh)
        except (OSError, ValueError):
            time.sleep(0.1)
    return {}


def wait_auto(expected, timeout=2.5):
    end = time.time() + timeout
    while time.time() < end:
        st = state()
        if bool(st.get("auto")) == expected:
            return st, True
        time.sleep(0.07)
    return state(), False


def main():
    st = state()
    if not st:
        print("FAIL: no state.json - is the app running?")
        return 1

    print("hotkeys reported:", st.get("hotkeys"))
    print("auto at start   :", st.get("auto"))

    results = []

    print("\n1) SendInput F2  -> expect AUTO ON")
    s.press("F2")
    st, ok = wait_auto(True)
    results.append(("F2 starts auto", ok))
    print("   auto =", st.get("auto"), "OK" if ok else "<-- FAIL")

    print("2) SendInput F3  -> expect AUTO OFF")
    s.press("F3")
    st, ok = wait_auto(False)
    results.append(("F3 stops auto", ok))
    print("   auto =", st.get("auto"), "OK" if ok else "<-- FAIL")

    print("\nsummary:")
    for name, ok in results:
        print(f"   {'PASS' if ok else 'FAIL'}  {name}")
    return 0 if all(ok for _, ok in results) else 1


if __name__ == "__main__":
    sys.exit(main())
