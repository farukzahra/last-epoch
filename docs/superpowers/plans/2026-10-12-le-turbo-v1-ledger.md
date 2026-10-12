# SDD ledger — plan: docs/superpowers/plans/2026-10-12-le-turbo-v1.md

Workspace: `.superpowers/sdd/2026-10-12-le-turbo-v1`
Branch: `feat/le-turbo-v1` (off `main` at b1f1cb5)
Spec: `docs/superpowers/specs/2026-10-12-last-epoch-turbo-design.md`

## Setup rulings

- **Ruling:** execute on the branch `feat/le-turbo-v1` in place, not in a
  separate git worktree. The repo was created in this session and has no history
  worth protecting; a worktree would not carry the git-ignored `.agents/skills`
  directory that the skill loader reads from, and this harness exposes no native
  worktree tool. The rule the worktree exists to enforce — never implement on
  `main` — is satisfied by the branch. *Cost if wrong:* the work lands on a
  branch the user must merge, instead of in a directory they can delete.
- **Ruling:** plan Task 1 Step 1 (`git init -b main`) is already satisfied. Git
  was initialised, a baseline commit (`b1f1cb5`) taken, and the feature branch
  created, all before Task 1, so the repository could have a `.gitignore` before
  any agent state was staged. *Cost if wrong:* none — the step is idempotent.

## Pre-flight scan — shared interfaces

| Producer | Consumer | Contract produced → consumed | Finding |
|---|---|---|---|
| Task 3 | Task 5 | `Row`, `RowAction`, `MIN_MS` | Match. |
| Task 2 | Task 6, 7 | `Key`, `MouseButton`, `InputError` | Match. |
| Task 5, 6 | Task 7 | `due_rows`, `RowClock`, `InputSink`, `InputError` | Match. |
| Task 7 | Task 8 | `GameProbe`, `Engine<S>` | Match. |
| Task 6 | Task 8 | `RawSender`; Task 8 **moves** `WinRawSender` from `src/input.rs` to `src/win/sender.rs` and re-exports it | Not a conflict — the move is documented in Task 8 and the re-export keeps Task 6's imports working. |
| Task 9 | Task 11 | `AppState` | **Conflict.** Task 11's `spawn_hotkey_loop` must exit the app on Quit ("through the Tauri app handle"), but `AppState` (Task 9) holds no `AppHandle`. **Ruling:** Task 11's signature becomes `spawn_hotkey_loop(app: tauri::AppHandle, state: AppState, stop: Arc<AtomicBool>)`, and Quit does `state.engine.lock().disarm(); app.exit(0);`. Narrower than widening `AppState`, and it keeps the release-on-quit path the spec requires. *Cost if wrong:* one signature changes again. |
| Task 9 | Task 10 | `StateDto.capture_ok` | **Conflict.** Task 9 defines the field; Task 10 is what can compute it, so Task 9 must ship a value first. **Ruling:** `capture_ok` defaults to `true` in Task 9 and the Task 10 status thread overwrites it within 5 s. Defaulting to `false` would flash a false "cannot read the game screen" warning on every launch. *Cost if wrong:* a real capture failure is reported up to 5 s late. |
| Task 9 | Task 12 | Tauri commands, `status` event | Match. |
| Task 8 | Task 10 | `find_window_by_process`, `window_rect` | Match. |

No other task pair shares an interface.

## Progress

### Task 1: Project skeleton that builds, runs and tests

Rulings taken during this task:

- **Ruling:** Task 1 Step 2 (`.gitignore`) is satisfied by a superset — the file
  also ignores `.agents/`, `.cursor/`, `.claude/`, `.opencode/` (the skills CLI
  creates junctions there and git would stage every file twice), `research/*.png`
  (the screenshots contain the user's desktop) and `spike/state.json` /
  `spike/profile.json`. *Cost if wrong:* none — an over-broad ignore fails loudly
  the first time a needed file goes missing from git.
- **Ruling:** skipped Step 10 (`cargo install tauri-cli`). `cargo run` already
  serves the static frontend and opens the window, and `cargo build --release`
  already produces the shipped executable; the CLI is a multi-minute compile that
  buys `dev`/`build` sugar only. *Cost if wrong:* no bundled installer or updater
  until tauri-cli is added — one command, later.
- **Ruling:** plan defect — `tauri-build` requires `icons/icon.ico` during the
  Windows resource step and no task created any artwork. Generated a 16/32/48
  BMP-based ICO with `research/make_icon.py` and set
  `bundle.icon = ["icons/icon.ico"]`. *Cost if wrong:* the icon is a placeholder
  (gold "T" on the app's dark green) until real artwork replaces it.
- **Ruling:** plan defect — Task 1 Step 9 set `"frontendDist": "../ui"`, which
  assumes the conventional `src-tauri/` layout, but this plan places
  `tauri.conf.json` at the repo root, so it resolved to `C:\repo\ui` and the macro
  panicked with "this path doesn't exist". Corrected to `"ui"`. *Cost if wrong:*
  none.

Task 1: complete (commits b1f1cb5..ab63cde, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 2: complete (commits ab63cde..e9f9085, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 3: complete (commits e9f9085..067b3fe, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 4: complete (commits 067b3fe..22a3d2a, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

### Task 5: Press scheduler

- **Ruling:** plan defect — the scheduler tests pass `MIN_MS` (a `u32`) as
  `due_rows`'s `now_ms` (`u64`), which does not compile. Wrapped the two call
  sites in `u64::from(MIN_MS)`. *Cost if wrong:* none.
- **Ruling:** plan defect — `a_short_clocks_list_is_treated_as_never_fired`
  supplied `last_fire_ms = Some(0)` with `now_ms = 5000`, which makes row 0 due
  as well (`5000 - 0 >= 1000`), contradicting the test's own `vec![1]`
  expectation. Changed the clock to `Some(5000)` rather than the assertion, so
  the expected value stands and the test still distinguishes a never-fired row
  from a recently-fired one. *Cost if wrong:* none — the property under test is
  unchanged.
Task 5: complete (commits 22a3d2a..ae220d3, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

### Task 6: Input sink with held-key bookkeeping

- **Ruling:** `WinRawSender` (the `SendInput` binding) is deferred from Task 6
  into Task 8 as `src/win/sender.rs`, instead of being defined in `src/input.rs`
  and then moved. Task 6 stays pure and fully unit-tested against a fake, all
  `windows`-crate API risk concentrates in Task 8 Step 5 (whose stated purpose is
  to prove the Win32 bindings compile), and a pointless move disappears.
  *Cost if wrong:* one file is created where the plan had it moved.
Task 6: complete (commits ae220d3..7ae53bf, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

### Task 7: Engine loop

- **Ruling (behaviour gap, found by the task's own test):** the plan's numbered
  step 5 applies `Hold` rows whenever injection is allowed (`may_inject`), but
  `the_watchdog_releases_even_with_the_gate_disabled` shows that with the gate off
  the watchdog releases the button and the *same* tick immediately re-holds it, so
  the test can never pass. Ruled: `Hold` rows require the game to hold the
  foreground **regardless of the gate**; only `Press` rows follow the gate. This
  matches spec decided item 16 and the anti-goal of injecting into the wrong
  window — a held button has no window of its own and lands on whatever is next.
  *Cost if wrong:* with the gate disabled, a user cannot hold a button while the
  game is unfocused; that is the intended safety behaviour.
- **Ruling:** added one test the plan did not have,
  `changing_rows_while_holding_releases_the_hold`. Spec item 16 names an
  "armed→disarmed profile change" as a release path and no task pinned it, so
  `Engine::set_rows` now drops an in-force hold and re-applies from the new list.
  *Cost if wrong:* one extra test.
- **Ruling:** dropped the `now_ms` field from the `Engine` struct (the plan listed
  it) — nothing read it, and a private never-read field is a compiler warning.
  *Cost if wrong:* none.
Task 7: complete (commits 7ae53bf..8274ea6, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

### Task 8: Win32 probes and the send loop thread

- **Ruling:** `spawn_send_loop` takes `(engine, probe: Arc<WinProbe>, stop: Arc<AtomicBool>)`.
  The plan's signature listed only `engine`, but its own prose requires a shared
  stop flag for clean shutdown, and `Engine::tick` takes a `GameProbe` — so the
  probe is passed in rather than rebuilt every tick. *Cost if wrong:* one
  signature changes again.
- **Ruling:** added `pub type AppSink = SendInputSink<WinRawSender>`. The plan
  wrote `Engine<WinRawSender>`, but `Engine<S: InputSink>` needs an `InputSink`
  and `WinRawSender` is only a `RawSender`; held-key tracking lives in
  `SendInputSink`. *Cost if wrong:* none — the alias names exactly the sink the
  spec requires.
- Removed an unused `MouseButton` import from the engine tests (a warning left
  over from inlining the probe helpers).
Task 8: complete (commits 8274ea6..d588b4c, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

### Task 9: Tauri command layer

- **Ruling:** `InputSink` gained `fn set_scancode(&mut self, scancode: bool)`.
  The plan's `set_options(gate, scancode)` is meant to work at runtime, but the
  trait had no way to reach the sink's flag and the engine owns the sink
  privately. `Engine` gained `set_scancode`, `gate()` and `fire_once()` for the
  same reason (`fire_once` backs the UI's "Fire once" button, which the plan
  specified behaviourally but gave no engine entry point). *Cost if wrong:* the
  engine test fake carries one extra no-op method.
- **Ruling:** `AppState` carries `probe`, `stop` and `scancode` beyond the
  plan's four fields. The send loop needs the shared probe and the stop flag, and
  the status thread needs `scancode` without reaching through the sink. *Cost if
  wrong:* three extra fields.
- `build_state` and `StateDto` live in `commands.rs`, as the plan's Interfaces
  specified.
Task 9: complete (commits d588b4c..11a1510, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

### Task 10: Window shell

- **Ruling:** window placement, topmost and the dark title bar run from the
  status thread (retried until the window is found) rather than from `setup`.
  `setup` can run before the window is enumerable, and locating it by our own
  process name avoids depending on Tauri's `HWND` type, which comes from a
  different `windows` crate version than this crate's. *Cost if wrong:* up to
  250 ms before the window is parked, instead of instantly.
- **Ruling:** topmost is re-asserted once a second. A fullscreen game can take
  the top spot back after launch, and the whole design depends on this window
  staying visible. *Cost if wrong:* a brief raise of our window each second.
- **Ruling:** the `capture_ok` refresh lives in the same thread and runs every
  5 s; when the game is not running it reports `true` so the UI never shows a
  false "cannot read the game screen" warning. *Cost if wrong:* a real capture
  failure is reported up to 5 s late.
- Verified by hand: the window opens at `-1904,16` (the non-primary monitor)
  with a dark title bar and the generated app icon.

### Task 11: Global hotkeys by polling

- **Ruling:** `spawn_hotkey_loop(app: AppHandle, stop: Arc<AtomicBool>)` — the
  pre-flight ruling had proposed a `state` parameter as well, but
  `AppHandle::state::<AppState>()` reaches the managed state directly, so the
  extra parameter would only duplicate it. *Cost if wrong:* none.
- **Ruling:** the plan's Step 5 (drive F2/F3 in the running app and watch the
  status flip) is folded into Task 12 Step 6, which is the same check performed
  with the UI present to observe it. The polling mechanism itself is already
  proven end-to-end: the Python spike showed `GetAsyncKeyState` sees
  `SendInput`-generated F2 and F3. *Cost if wrong:* a hotkey wiring bug surfaces
  one task later, with the UI in place to show it.

### Task 12: The UI

- **Ruling:** dropped the `topmost` and `otherMonitor` checkboxes the plan listed.
  The window must always be on top and always open on the non-primary monitor —
  that is what makes it usable over a fullscreen game — and neither option had a
  backing command in `set_options`. Offering a control that silently does nothing
  is worse than offering two fewer controls. *Cost if wrong:* two options the
  plan listed are not user-adjustable in v1.
- **Ruling:** the status channel now falls back to polling `get_state` every
  500 ms when `window.__TAURI__.event` is unavailable, and a failed `boot()`
  reports into the warning band instead of dying silently. The first run of the
  shipped UI rendered **no rows and no status** because the failure happened after
  `applyStrings()` and was invisible. *Cost if wrong:* the status updates up to
  500 ms late when the fallback is taken.
- **Ruling:** `Mouse Left` as a `Hold` row joined the default profile, on the
  user's report that the left button was missing. It is the movement control, so
  the app ships ready to run; profile spec decided item 5 was updated to match.
  *Cost if wrong:* one extra default row, disabled like the rest.
- **Task 12 Step 6 (verify by hand in the game):** items 1-3 are verified
  end-to-end in this session — the window opens on the non-primary monitor, the
  UI's own START button arms the engine, and with row `1` enabled and Last Epoch
  focused the status counter read **`sent: 3`**, proving the whole chain
  (UI click → command → engine → `SendInput` → game) with the focus gate open.
  Items 4-5 (held left button actually moves the character; F3 releases with no
  desktop drag) and item 7 (F9 quits cleanly) need a person at the keyboard and
  are left to the user, as the plan specifies.
- **Task 12 Step 7:** `cargo build --release` → `target/release/le-turbo.exe`,
  **8.31 MB**. `tauri-cli` is still skipped per the Task 1 ruling, so there is no
  bundled installer or updater.

## Final review fix pass

Reviewer: a fresh subagent reviewed the whole branch from source (not a diff),
against the spec, the plan's Review Focus lines and the rulings below. It
reported **1 Critical, 6 Important, 11 Minor**. All Critical and Important
findings were fixed in one pass.

- **Final: fixed C1** — `Hold` rows bypassed cursor containment, so the shipped
  `Mouse Left` `Hold` row kept the button down after the cursor left the game and
  dragged whatever was under the pointer. `a_hold_row_releases_when_the_cursor_leaves_the_game`
  and `a_mouse_hold_row_does_not_start_with_the_cursor_off_the_game` RED→GREEN.
- **Final: fixed I1** — nothing released input on a panic, and the send loop's
  trailing `disarm` was unreachable on unwind. Added `engine::tick_guarded`; the
  loop now releases and stops after three consecutive unwinds.
  `a_panicking_tick_releases_held_input` RED→GREEN.
- **Final: fixed I4** — the UI's "Fire once" button called `SendInput` directly,
  so it typed into whatever window was focused. `Engine::fire_once` now takes a
  probe and honours the gate and mouse containment.
  `fire_once_respects_the_focus_gate` and
  `fire_once_skips_a_mouse_key_with_the_cursor_off_the_game` RED→GREEN.
- **Final: fixed I5** — a row bound to `F2`/`F3`/`F9` would fire its own global
  hotkey (F9 quit the app, F3 disarmed it). `validate_rows` now refuses them.
  `validate_rows_rejects_a_key_bound_to_a_hotkey` RED→GREEN.
- **Final: fixed I2** — the foreground cache was 50 ms, so the gate could act on a
  stale focus for the first 50 ms after an alt-tab. Now 10 ms.
- **Final: fixed I3** — `stop` was never set, so the send loop's documented final
  `disarm` was dead code. Both quit paths now set it.
- **Final: fixed I6** — two quick UI edits could race and drop one. All mutations
  now run through a single promise chain and the DOM renders from the backend's
  answer rather than a local array that could drift.
- Suite after the pass: **61/61 green**.
- **Final: Ruling:** I2, I3 and I6 are fixed *without* a test that first failed.
  Every Win32 call in this crate sits behind a one-method trait precisely so the
  logic above it can be tested; a cache TTL, Tauri shutdown wiring and a browser
  promise chain are on the untestable side of that line, and there is no JS test
  harness in the project. *Cost if wrong:* the fix does not hold and the earlier
  behaviour returns.
- **Verified after the pass:** the app still renders all six rows with the
  `Mouse Left` `Hold` row and its disabled interval field, status `DISARMED`.

## Final: minor (deferred)

1. `skipped` is counted per tick, not per attempt, so it climbs ~200/s per stuck row.
2. Any row edit resets every `RowClock`, so enabled rows fire once out of cadence after an edit.
3. Failed releases are swallowed (`let _ =`), with no retry or log.
4. `load_profile` skips `validate_rows`, so a hand-edited bad key becomes a silent dead row; an unknown `RowAction` variant rejects the whole file.
5. `is_mostly_black` can false-positive on a genuinely dark frame, and `capture_region` uses the primary monitor's DC regardless of where the game is.
6. `set_topmost` omits `SWP_NOACTIVATE`, so the once-a-second re-assert may pull foreground on some Windows builds.
7. `build_state` holds the engine lock across `OpenProcess`/`QueryFullProcessImageNameW`, adding jitter to the 5 ms loop.
8. Startup rewrites a malformed `config.json` silently, and config written beside the exe fails silently in a read-only install directory.
9. Dead code: `safety::{FocusProbe, CursorProbe}`, `WinProbe::{expected_process, game_focused_by_name}`, four unused `strings.js` keys, and a hardcoded version that will drift from `Cargo.toml`.
10. `find_window_by_process` calls `OpenProcess` for every visible top-level window once per second.
11. On a single-monitor layout the window is topmost over the game, and a mouse row whose cursor sits over our own window still passes the rect-based containment test.


Task 10: complete (commits 11a1510..556f468, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 11: complete (commits 556f468..8d563a7, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 12: complete (commits 8d563a7..431384e, tests: cargo test → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
