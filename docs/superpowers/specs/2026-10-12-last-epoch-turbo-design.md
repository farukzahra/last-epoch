# LE Turbo — design

> Written 2026-10-12. Status: awaiting the user's approval before any product code.
> Inputs: `docs/reverse-engineering/d4turbo-analysis.md` (how the reference tool
> works) and `spike/README.md` (what the throwaway prototype proved).

## Intent and why

Last Epoch gives a character five skill slots. The user has remapped theirs to
`1`, `2`, `3`, `4` and the **right mouse button**, and plays build that wants
those skills re-pressed on short, repeating cycles. Pressing them by hand for a
whole session is wrist abuse, and it competes for attention with actually
playing.

There is an existing tool that solves this for Diablo IV (`D4Turbo`, studied in
the reverse-engineering document). What it does **not** do — and cannot, because
it has no way to observe the game — is refresh a buff when the buff actually
drops. It just fires on a blind clock and hopes. That is the gap this project
exists to close.

So: build the same style of tool for Last Epoch, then make it smarter than the
reference.

## Goals

| # | Goal |
|---|---|
| G1 | **v1 — the clicker.** One row per input. Each row has a key (or mouse button), an interval in milliseconds, and an **action**: `Press` (tap once per interval, repeating) or `Hold` (press down and stay down while armed). |
| G1b | **Movement.** Holding `Mouse Left` is how the character runs toward the cursor. So `Hold` on `Mouse Left` gives automatic movement, and low-interval `Press` on `Mouse Left` gives auto-click movement. Both must work. |
| G2 | Profiles: save / load / reset to `JSON`. Global hotkeys for start, stop and quit, working regardless of which window has focus. |
| G3 | Never interfere. No injection unless Last Epoch is the foreground window. The user must stay able to type in other applications. |
| G4 | **v2 — smart buffs.** Refresh a buff when its icon actually leaves the screen, instead of on a blind timer. |
| G5 | Ship as one small, self-contained `.exe` that starts instantly, matching the reference tool's footprint (~13 MB, no runtime to install). |

## Non-goals

- No reading or writing the game's process memory. No code injection, no DLL
  injection, no remote thread, no kernel driver.
- No modification of game files, saves, or network traffic.
- No chat spam, no trading automation, no multi-boxing.
- Not a general-purpose macro recorder. It presses keys on a clock; that is all.
- v1 ships **no** vision and no condition evaluation. Every row fires on its clock.

## Anti-goals

These are ways the project fails even if it technically works:

- Getting flagged by antivirus. This is why there is no driver and no injection:
  `SendInput` from a plain user-mode process is the least suspicious mechanism
  available, and the reference tool survives on the same basis.
- Injecting into the wrong window. A clicker that types `1` into a Discord chat
  while the user is writing is worse than no clicker at all.
- Being fragile across game patches. Reading memory would break on every patch;
  pixel regions and key presses do not.
- Silently doing something the user did not arm. Rows default to OFF.

## Constraints

- Windows x64. User mode. Must not require administrator rights at runtime.
- **Stack: Rust + Tauri 2** (user's decision). Input is `SendInput` — the same
  mechanism as the reference tool — called directly through the **`windows`
  crate** rather than through `enigo`. The struct layout was already measured and
  validated in the spike (`sizeof(INPUT) == 40` on x64, `KEYBDINPUT == 24`), so
  calling Win32 directly removes a dependency whose API drifts between versions
  while changing nothing about the mechanism the spec requires.
- The game runs fullscreen 1920x1080 on the primary monitor. Our window belongs
  on the **secondary monitor** and must stay above the game.
- Display is 100% DPI, so screen coordinates and pixels are 1:1. The app will
  still be written DPI-aware rather than relying on that.
- The user accepted the risk of using this in online play. The risk is recorded
  in this document and in the repo; the design does not try to evade detection
  beyond using the least invasive input mechanism.

## Decided how (settled with the user, or forced by measurement)

These are not open questions. `spike/README.md` holds the measurements behind
the ones marked **measured**.

1. **Input = `SendInput`.** Measured: pressing `1` into a foreground Last Epoch
   changed 13.6% of the skill-bar pixels (VK mode) and 16.9% (scancode mode),
   against 0.00% with no input. Both modes work; the scancode variant stays
   available as a per-profile fallback switch.
2. **Hotkeys = polling `GetAsyncKeyState`, not `RegisterHotKey`.** Measured:
   Windows does not raise a registered hotkey for injected input, and Tk's
   message pump swallowed `WM_HOTKEY`. Polling was verified PASS. Note for the
   Rust build: Tauri's `global-shortcut` plugin is the equivalent of
   `RegisterHotKey` and will work for *physical* presses; it must be re-verified
   for synthetic ones.
3. **The window must be forced topmost at creation.** Measured: without it, a
   hit test inside our own window returned the game's window handle. This is the
   actual reason the prototype's row toggles appeared unclickable.
4. **Window placement:** the secondary monitor (the one not holding the game),
   offset slightly from the work-area corner, topmost by default, with a switch
   to disable topmost.
5. **Row model:** `ON/OFF | KEY | ACTION | MILLISECONDS`, plus add-row and
   remove-row. Defaults are the user's remap, in this order: `1` `2` `3` `4` as
   `Press`, then **`Mouse Left` as `Hold` for movement**, then **`Mouse Right` as
   `Press`** for the fifth skill bind — all at 1000 ms, all **OFF**.
6. **Safety gate:** a default-on option "only inject while Last Epoch is
   focused". Key detail: the check is on the foreground window's title/process.
7. **Cosmetics do not affect behaviour.** Row icons are decoration only.
8. **UI language: English (v1).** Decided by the user. Profile data stays
   language-free, and every user-facing string lives in one strings module so a
   second language is a mechanical addition later, not a refactor.
9. **Theme:** dark, low-contrast, aged-gold accents — closer to Last Epoch's own
   panels than to a stock Windows app.
10. **Mouse-button rows require the cursor inside the game window.**
    *Measured:* Last Epoch does **not** clip the cursor — while the game was
    foreground, `GetClipCursor` returned the whole virtual desktop
    (-1920,0)-(1920,1080) and the cursor was moved to the other monitor
    successfully. So a `Mouse Right` row firing while the pointer sits on the
    other monitor would click **whatever is under the pointer** (our own window,
    the taskbar, a desktop icon) — a high-impact failure. `SendInput` mouse events
    go to the window under the cursor, not the focused window. Therefore mouse
    rows skip their press unless the cursor is inside the game window. This
    containment check is **always on**, independent of the focus gate. Warping the
    cursor into the game first was rejected as too invasive.
11. **The focus gate matches the process, not the title.**
    *Measured:* the game process is `Last Epoch.exe` and its window class is
    `UnityWndClass`, title `Last Epoch`. The spike matched a title substring,
    which would also match a browser tab, a Discord channel or a stream title —
    exactly the anti-goal. The gate matches the foreground window's **process
    image name** (`Last Epoch.exe`), with the expected name stored in **app
    config** (not the profile) so it can be corrected without a rebuild.
12. **Hotkeys are fixed F2/F3/F9 in v1, stored in app config, not the profile.**
    Rebindable shortcuts are a v1.1 item; the reference tool supports rebinding
    and that is worth copying later, but it adds a field, a capture UI and
    another versioning surface for no v1 benefit.
13. **The profile reserves the v2 shape now.** Each row carries an action
    discriminator (`press` for v1) and optional condition fields (region of
    interest + reference image), so shipping v2 does not invalidate v1 files.
    The format carries an explicit version integer.
14. **v1 carries a display-mode sanity check.** Because v2's pixel detection
    needs the game to be captured, v1 already verifies it can grab a non-blank
    frame of the game window and warns the user if it cannot (exclusive
    fullscreen would come back black). This is cheap insurance and it fails
    loudly instead of silently.
15. **Row action is `Press` or `Hold`.** Added by the user for automatic
    movement.
    - `Press` — down+up once per interval, repeating. This is the whole v1 model
      as originally designed, and a `Mouse Left` row at a low interval already
      gives auto-click movement.
    - `Hold` — key/button down once when the row is armed, and up when it is
      disarmed. The interval field is unused for `Hold` rows in v1 (a re-assert
      period can come later if a game drops synthetic held state).
    - `Hold` on `Mouse Left` is the auto-run feature: the character keeps moving
      toward the cursor.
16. **Invariant: never leave an input held.** This is the highest-impact failure
    mode the project now has — if the app exits, is disarmed, or the gate closes
    while `Mouse Left` is held, Windows is left with a stuck button and the user's
    machine drags/selects everything until they click manually.
    - The engine tracks the set of currently-held inputs. `release_all()` is
      idempotent and runs on **every** exit path: explicit disarm (F3 / Stop
      button), quit (F9 / window close), the focus gate transitioning from open to
      blocked, an armed→disarmed profile change, and unwinding on panic.
    - A watchdog also releases when the game loses foreground, even if the gate
      option is off, because a held button with no game to receive it is always a
      bug.
    - Tests assert the sink's held set is empty after each of those transitions.

## v1 scope (the deliverable)

A single window with:

- a table of rows: per-row ON/OFF toggle, key selector (keyboard keys, function
  keys, and mouse left/right/middle), an **action selector (`Press` / `Hold`)**,
  and a millisecond interval
- automatic movement: `Hold` on `Mouse Left`, or `Press` on `Mouse Left` at a low
  interval for auto-click
- add row, remove row, and a "fire once" test button per the prototype
- Save / Load / Reset profile to a JSON file next to the executable
- global hotkeys: start, stop, quit (fixed F2/F3/F9 in v1, stored in app config)
- a status area showing whether Last Epoch is focused, whether injection is
  armed, and how many presses have been sent
- options: gate on focus, scancode mode, always on top, open on the other monitor
- a loud warning if the game's frame cannot be captured (exclusive fullscreen)
- **release-all on disarm, quit, focus loss, and panic** — a held button must
  never outlive the app

## Builder check — questions raised and how they were resolved

A fresh reviewer read this document and asked six things. Five were settled here
with a measurement or a call; one belongs to the user.

| # | Question | Resolution |
|---|---|---|
| 1 | Do mouse-button rows mis-fire when the cursor is not over the game? | **Real, measured, fixed.** The game does not clip the cursor → mouse rows now require cursor containment (decided item 10). |
| 2 | What exactly identifies "Last Epoch is focused"? | **Settled.** Match the process image name, stored in app config (decided item 11). A title substring would match unrelated windows. |
| 3 | Which language(s) must v1 ship in? | **Settled by the user: English.** All user-facing strings in one module; profile stays language-free. |
| 4 | Are start/stop/quit fixed or rebindable, and where are they stored? | **Settled as a builder call.** Fixed F2/F3/F9 for v1, in app config, rebinding deferred to v1.1 (decided item 12). |
| 5 | Is Last Epoch available for in-game verification before "done"? | **Yes — answered by the environment.** The game is installed (`C:\Program Files (x86)\Steam\steamapps\common\Last Epoch`) and running, and in-game input and pixel measurements have already been performed successfully. In-game end-to-end is therefore the acceptance step, not a mock. |
| 6 | Stop at v1, or land v2-shaped seams now? And does v1 need display-mode awareness? | **Settled as builder calls.** Reserve the v2 row shape now (decided item 13) and add the capture sanity check to v1 (decided item 14). |

## v2 scope (the differentiator) — one open question

A row may optionally be *conditioned* on something on the screen instead of a
clock. The intended condition for v2: **the buff icon is absent**, meaning
"refresh it now".

Mechanics: capture a small rectangle of the screen, compare it against a
reference of the buff icon, and treat "icon missing" as the trigger, with a
minimum re-press interval to avoid machine-gunning.

**Open question, and it blocks the v2 design:** the icons at the top-right of the
screen that were measured during the prototype are the **season mechanic's
icons** and the user says to ignore them. Where do the *player's own* buffs
(e.g. a skill-granted buff the user wants kept up) appear on screen, and which
buff is the first target? Without that, there is no rectangle to watch and no
reference image to capture. Until it is answered, v2 stays unplanned — this is
the one thing only the user can supply.

### Rejected alternatives for v2

- **Timer only** (what the reference tool does). Simplest, but re-presses early
  or late and wastes the skill.
- **Reading process memory** for real buff timers. Precise, but breaks every
  patch, requires injection, and is the most hostile option to the game's terms.
- **Parsing game log files.** Cheap if the data exists, but Last Epoch is not
  known to emit buff apply/expire events. Would need verification before it can
  be promised; deliberately not the primary plan.

## What is left to the builder

- Project layout, crate choices beyond Rust + Tauri + `enigo`, and the frontend
  framework inside Tauri (or no framework at all).
- How the input loop is scheduled: one thread per row, one scheduler loop, or a
  timer wheel. Requirement: a 10 ms floor on the interval, and no drift pile-up
  when a press is slow.
- How profiles are versioned so a future field does not break old files.
- Where the executable and its profile live; installer or portable zip.
- Whether the "fire once" test button remains in the shipped build.
- Icon/artwork for the app.
- How the v2 rectangle and reference image are captured *from the running game*
  and stored in the profile (screenshot-and-crop picker vs manual coordinates).

## Risks

| Risk | Severity | Note |
|---|---|---|
| Account action from the game's terms | user-accepted | Mitigated only by using plain `SendInput`; not hidden |
| Antivirus false positive | medium | Avoided by staying user-mode; a signed binary would help but is out of scope |
| Game updates change key handling | low | `SendInput` is stable; the scancode fallback exists |
| v2 vision breaks on UI scale/resolution changes | medium | The rectangle must be stored per-profile, and the resolution it was captured at should be recorded |
| Injecting while the wrong window is focused | high impact, low probability | The focus gate plus rows defaulting to OFF |

## Evidence

- `docs/reverse-engineering/d4turbo-analysis.md` — how the reference tool works,
  with the import table, the crate list, the recovered settings schema, and the
  validated Last Epoch results.
- `spike/README.md` — the throwaway prototype, its test scripts, and the measured
  facts quoted above.
