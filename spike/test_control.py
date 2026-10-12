"""THROWAWAY SPIKE TEST - control run: does the region change on its own?

Same region and timing as test_sendinput_in_game.py but with NO input sent.
If this baseline is near zero, the earlier 13-17% change was caused by the key.

Run: python spike/test_control.py
"""

import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_sendinput_in_game as t
import le_turbo_spike as s

hwnd = t.find_last_epoch_window()
if not hwnd:
    print("Last Epoch window not found")
    sys.exit(1)

t.user32.ShowWindow(hwnd, 9)
t.user32.SetForegroundWindow(hwnd)
time.sleep(1.2)
print("foreground:", repr(s.foreground_title()))
if "last epoch" not in s.foreground_title().lower():
    print("!! not foreground; abort")
    sys.exit(1)

print("settling 6s so any cooldown from the previous run expires...")
time.sleep(6)

for i in range(3):
    a = t.grab(t.RX, t.RY, t.RW, t.RH)
    time.sleep(0.35)
    b = t.grab(t.RX, t.RY, t.RW, t.RH)
    diff = t.changed_pixels(a, b)
    total = t.RW * t.RH
    print(
        f"baseline #{i + 1}: changed pixels {diff}/{total} ({100.0 * diff / total:.2f}%)"
    )
    time.sleep(0.4)
