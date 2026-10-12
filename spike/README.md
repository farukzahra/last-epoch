# LE Turbo — spike (throwaway)

Prototype auto-clicker for Last Epoch, built to answer two questions before any
real work starts:

1. does Win32 `SendInput` actually register inside Last Epoch?
2. does the `(key, interval_ms)` table model feel right?

**This is a spike, not the product.** No tests were skipped for good reasons —
there are no unit tests, no error handling to speak of, and it is an end-of-life
artifact. It gets deleted or archived when the Rust + Tauri version lands
(`docs/superpowers/plans/` once the design is approved).

## Run

```powershell
python spike/le_turbo_spike.py
```

- **F2** start, **F3** stop, **F9** quit
- rows: `ON/OFF` toggle, `KEY` (keyboard key **or** mouse button), `MILLISECONDS`
- defaults match the in-game remap: `1 2 3 4` + `Mouse Right`
- window parks itself on the **non-primary monitor** on launch (the one without the game)
- "So injetar com Last Epoch em foco" gates injection to the foreground game window

## Verify

```powershell
python spike/test_hotkeys.py       # F2/F3 state transitions
python spike/test_toggle_click.py  # real synthetic click on the row toggles
```

Both read `spike/state.json`, which the app rewrites every ~60 ms (auto, sends,
foreground window, window geometry, toggle button screen coords).

## What the spike proved (evidence, not guesses)

| Finding | Evidence |
|---|---|
| **`SendInput` lands in Last Epoch** | `spike/test_sendinput_in_game.py` pressed `1` while the game was foreground: skill-bar region changed 13.6% (VK) / 16.9% (scancode); a control run with no input showed **0.00%** change |
| Both VK and scancode modes work | same test, both produced large deltas |
| `RegisterHotKey` is **not** a viable hotkey path here | it registered fine (all three returned TRUE) but `SendInput(F2)` never fired it — Windows does not raise registered hotkeys for injected input — and Tk's message pump swallowed `WM_HOTKEY` before a message-only window's `WndProc` could see it |
| Polling `GetAsyncKeyState` works | `spike/test_hotkeys.py`: F2 → auto ON, F3 → auto OFF, both PASS. Uses `state & 0x8000` (held) **or** `state & 0x0001` (pressed-since-last-poll) so instant taps are not missed |
| The window must be forced topmost **at startup** | a fullscreen game sits above normal windows; `WindowFromPoint` returned the *game* for every point inside the app until `place_window()`/`apply_topmost()` ran on launch |
| Tk widgets are separate HWNDs | `WindowFromPoint` on a row toggle returns a child window of class `TkChild`, not the toplevel |

## Notes for the real build (Rust + Tauri)

- input: `enigo` (SendInput backend) — the same choice D4Turbo makes
- hotkeys: Tauri `global-shortcut` works for **physical** presses; if synthetic
  input must also trigger hotkeys, poll `GetAsyncKeyState` instead
- window: `alwaysOnTop` at creation, `set_position` to the target monitor
- the reference tool is Rust + Tauri 2.11 (`docs/reverse-engineering/d4turbo-analysis.md`)

## Screen geometry recorded for the vision work (1920x1080, 100% DPI)

| Region | Coordinates | Use |
|---|---|---|
| status/buff column, top-right | x 1545-1595 (centre ~1569), first centre y ≈ 77, vertical pitch ≈ 73 px, icon Ø ≈ 48 px | the season-buff icons (to be ignored) — re-measure for real player buffs |
| skill bar | x ≈ 740-1060, y ≈ 975-1055 | per-skill cooldown wipe can be read here |
| player orb | x 420-520, y 890-1010 | health 647/647 in the sample |
| XP bar | x ≈ 765-1010, y ≈ 1038-1047 | animates; exclude from any diff region |

Capture works with plain `CopyFromScreen` / `BitBlt`, i.e. the game is **not**
exclusive fullscreen. That is the key enabler for pixel-based buff detection.
