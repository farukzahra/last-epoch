import { S } from "./strings.js";

const { invoke } = window.__TAURI__.core;
const tauriEvent = (window.__TAURI__ && window.__TAURI__.event) || null;

// Mirrors Key::KEY_CHOICES on the Rust side.
const KEY_CHOICES = [
  "1", "2", "3", "4", "5", "6",
  "Q", "W", "E", "R", "T", "A", "S", "D", "F", "G", "Z", "X", "C", "V", "B",
  "Space", "Tab", "Enter", "Esc",
  "Mouse Left", "Mouse Right", "Mouse Middle",
  ...Array.from({ length: 24 }, (_, i) => `F${i + 1}`)
];

const HOTKEY_SLOTS = ["toggle", "quit"];
const HOTKEY_LABELS = { toggle: "hotkeyToggle", quit: "hotkeyQuit" };

const MOUSE_NAMES = { 0: "Mouse Left", 1: "Mouse Middle", 2: "Mouse Right" };

// The middle button is what the user wants for "disarm", so be tolerant of how
// the button number arrives: some event shapes carry only the `buttons` bitmask.
function mouseName(event) {
  if (typeof event.button === "number" && MOUSE_NAMES[event.button]) {
    return MOUSE_NAMES[event.button];
  }
  const mask = event.buttons || 0;
  if (mask & 4) return "Mouse Middle";
  if (mask & 2) return "Mouse Right";
  if (mask & 1) return "Mouse Left";
  return null;
}

let rows = [];
let hotkeys = { arm: "F2", disarm: "F3", quit: "F9" };
let capture = null; // { slot, detected }
let optionsSynced = false;

const $ = (id) => document.getElementById(id);

function applyStrings() {
  for (const el of document.querySelectorAll("[data-i18n]")) {
    const value = S[el.dataset.i18n];
    if (value) el.textContent = value;
  }
}

function flash(message) {
  $("flash").textContent = message;
}

function showFailure(message) {
  const warn = $("warn");
  warn.textContent = `UI error: ${message}`;
  warn.classList.remove("hidden");
}

/* ------------------------------------------------------------- key capture --- */

// One key only, no modifiers. Returns the canonical name the Rust side knows, or
// null when the press cannot be a bind.
function nameFromEvent(event) {
  if (event.type === "mousedown") {
    return mouseName(event);
  }
  const key = event.key;
  if (key === " ") return "Space";
  if (key === "Tab") return "Tab";
  if (key === "Enter") return "Enter";
  if (key.length === 1) {
    const upper = key.toUpperCase();
    return /[0-9A-Z]/.test(upper) ? upper : null;
  }
  return /^F([1-9]|1[0-9]|2[0-4])$/.test(key) ? key : null;
}

function onCaptureInput(event) {
  if (!capture) return;

  // Never swallow a click on one of our own buttons or on the key list. That
  // keeps confirm, cancel and the list usable, and — the bug this replaced —
  // keeps START and STOP alive while a capture is open.
  if (event.target && event.target.closest && event.target.closest("button, select")) {
    return;
  }

  // A modifier on its own is not a bind, and a combo is not wanted.
  if (event.type === "keydown" && /^(Shift|Control|Alt|Meta)$/.test(event.key)) {
    event.preventDefault();
    flash(S.captureNotSupported);
    return;
  }
  if (event.type === "keydown" && event.key === "Escape") {
    event.preventDefault();
    stopCapture(S.captureEscape);
    return;
  }

  // Swallow the press so a middle click cannot scroll and a hotkey cannot fire.
  event.preventDefault();
  event.stopPropagation();

  const name = nameFromEvent(event);
  if (!name) {
    flash(S.captureNotSupported);
    return;
  }
  capture.detected = name;
  renderHotkeys();
}

function startCapture(slot) {
  capture = { slot, detected: null };
  renderHotkeys();
  invoke("set_capture_mode", { active: true }).catch((error) => flash(String(error)));
}

function stopCapture(message) {
  capture = null;
  renderHotkeys();
  if (message) flash(message);
  invoke("set_capture_mode", { active: false }).catch(() => {});
}

async function confirmHotkey(slot, key) {
  if (!key) return;
  try {
    const status = await invoke("set_hotkey", { slot, key });
    hotkeys = status.hotkeys;
    capture = null;
    renderHotkeys();
    applyStatus(status);
    flash(`${key} — ${S[HOTKEY_LABELS[slot]]}`);
  } catch (error) {
    flash(String(error));
    capture = null;
    renderHotkeys();
  }
  invoke("set_capture_mode", { active: false }).catch(() => {});
}

/* ------------------------------------------------------------------- rows --- */

function renderRows() {
  const host = $("rows");
  host.replaceChildren();

  rows.forEach((row, index) => {
    const line = document.createElement("div");
    line.className = "row";

    const toggle = document.createElement("button");
    toggle.className = row.enabled ? "toggle on" : "toggle";
    toggle.textContent = row.enabled ? "ON" : "OFF";
    toggle.addEventListener("click", () => toggleRow(index));

    const key = document.createElement("select");
    const choices = KEY_CHOICES.includes(row.key) ? KEY_CHOICES : [row.key, ...KEY_CHOICES];
    for (const choice of choices) {
      const option = document.createElement("option");
      option.value = choice;
      option.textContent = choice;
      if (choice === row.key) option.selected = true;
      key.append(option);
    }
    key.addEventListener("change", () => {
      rows[index].key = key.value;
      commit();
    });

    const action = document.createElement("select");
    for (const [value, label] of [["Press", S.actionPress], ["Hold", S.actionHold]]) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = label;
      if (value === row.action) option.selected = true;
      action.append(option);
    }
    action.addEventListener("change", () => {
      rows[index].action = action.value;
      commit();
      renderRows();
    });

    const ms = document.createElement("input");
    ms.type = "number";
    ms.min = "10";
    ms.step = "50";
    ms.value = row.ms;
    ms.disabled = row.action === "Hold";
    ms.addEventListener("change", () => {
      rows[index].ms = Number(ms.value) || 10;
      commit();
    });

    line.append(toggle, key, action, ms);
    host.append(line);
  });
}

/* ---------------------------------------------------------------- hotkeys --- */

function renderHotkeys() {
  const host = $("hotkeys");
  host.replaceChildren();

  for (const slot of HOTKEY_SLOTS) {
    const capturing = capture && capture.slot === slot;

    const line = document.createElement("div");
    line.className = capturing ? "box hotkey capturing" : "box hotkey";

    const badge = document.createElement("kbd");
    badge.textContent = capturing ? capture.detected || "…" : hotkeys[slot];
    line.append(badge);

    const desc = document.createElement("span");
    desc.className = "hotkey-desc";
    if (!capturing) {
      desc.textContent = S[HOTKEY_LABELS[slot]];
    } else {
      desc.textContent = capture.detected ? S.captureDetected : S.capturePrompt;
    }
    line.append(desc);

    if (!capturing) {
      const change = document.createElement("button");
      change.className = "tiny";
      change.textContent = S.change;
      change.addEventListener("click", () => startCapture(slot));
      line.append(change);
    } else {
      // Sensing covers a normal press, but a remapping mouse driver can still hide
      // one, so the list is always there as a manual override.
      const pick = document.createElement("select");
      pick.className = "tiny-select";
      const blank = document.createElement("option");
      blank.value = "";
      blank.textContent = "…";
      blank.selected = !capture.detected;
      pick.append(blank);
      for (const choice of KEY_CHOICES) {
        const option = document.createElement("option");
        option.value = choice;
        option.textContent = choice;
        if (choice === capture.detected) option.selected = true;
        pick.append(option);
      }
      pick.addEventListener("change", () => {
        capture.detected = pick.value || null;
        renderHotkeys();
      });

      const use = document.createElement("button");
      use.className = "tiny primary";
      use.textContent = S.useThis;
      use.disabled = !capture.detected;
      use.addEventListener("click", () => confirmHotkey(slot, capture.detected));

      const cancel = document.createElement("button");
      cancel.className = "tiny";
      cancel.textContent = S.cancel;
      cancel.addEventListener("click", () => stopCapture(null));

      line.append(pick, use, cancel);
    }

    host.append(line);
  }
}

/* --------------------------------------------------------------- mutate ----- */

let chain = Promise.resolve();

function queue(work) {
  chain = chain.then(work).catch((error) => flash(String(error)));
  return chain;
}

function adopt(result) {
  if (Array.isArray(result)) {
    rows = result;
    renderRows();
  }
}

function commit() {
  const payload = rows.map((row) => ({ ...row }));
  queue(async () => adopt(await invoke("set_rows", { rows: payload })));
}

function toggleRow(index) {
  const enabled = !rows[index].enabled;
  queue(async () => adopt(await invoke("set_row_enabled", { index, enabled })));
}

/* --------------------------------------------------------------- status ----- */

// Recently used profiles, newest first, so switching back is one click instead
// of a trip through the file dialog.
function renderRecents(paths) {
  const select = $("recents");
  select.replaceChildren();

  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = S.recents;
  select.append(placeholder);

  for (const path of paths || []) {
    const option = document.createElement("option");
    option.value = path;
    const parts = path.split(/[\\/]/);
    option.textContent = parts[parts.length - 1] || path;
    option.title = path;
    select.append(option);
  }
  select.value = "";
}

function applyStatus(status) {
  const version = `v${status.version}`;
  $("version").textContent = version;
  const badge = document.querySelector(".badge");
  if (badge) badge.textContent = version;

  renderRecents(status.recent_profiles);

  const armed = $("armed");
  armed.textContent = status.armed ? S.labelArmed : S.labelDisarmed;
  armed.className = status.armed ? "armed on" : "armed off";

  $("focus").textContent = status.game_focused ? "yes" : "no";
  $("presses").textContent = status.presses;
  $("skipped").textContent = status.skipped;
  $("profile").textContent = status.profile_path;

  const warn = $("warn");
  warn.textContent = S.captureWarning;
  warn.classList.toggle("hidden", status.capture_ok !== false);

  if (status.hotkeys && !capture) {    // Only rebuild when the binds actually changed: replacing the buttons every
    // status tick would destroy the one under the user's cursor mid-click.
    const unchanged = HOTKEY_SLOTS.every((slot) => hotkeys[slot] === status.hotkeys[slot]);
    if (!unchanged) {
      hotkeys = status.hotkeys;
      renderHotkeys();
    }
  }

  if (!optionsSynced) {
    $("gate").checked = status.gate;
    optionsSynced = true;
  }
}

async function pushOptions() {
  try {
    await invoke("set_options", { gate: $("gate").checked });
  } catch (error) {
    flash(String(error));
  }
}

async function readState() {
  const status = await invoke("get_state");
  if (!rows.length) {
    rows = status.rows;
    renderRows();
  }
  applyStatus(status);
  return status;
}

/* ------------------------------------------------------------------ boot ---- */

async function boot() {
  applyStrings();

  $("start").addEventListener("click", () => invoke("arm").catch((e) => flash(String(e))));
  $("stop").addEventListener("click", () => invoke("disarm").catch((e) => flash(String(e))));
  $("gate").addEventListener("change", pushOptions);

  $("save").addEventListener("click", () =>
    queue(async () => {
      flash(`${S.savedTo} ${await invoke("save_profile", { path: null })}`);
    })
  );

  // Save as asks for a path first, then saves into it and starts using it.
  $("saveas").addEventListener("click", () =>
    queue(async () => {
      const chosen = await invoke("pick_profile_file", { kind: "save" });
      if (!chosen) return;
      adopt(await invoke("save_profile", { path: chosen }));
      flash(`${S.savedTo} ${chosen}`);
    })
  );

  // Load asks for a path, so more than one profile is reachable.
  $("load").addEventListener("click", () =>
    queue(async () => {
      const chosen = await invoke("pick_profile_file", { kind: "open" });
      if (!chosen) return;
      adopt(await invoke("load_profile", { path: chosen }));
      flash(S.load);
    })
  );

  $("recents").addEventListener("change", (event) => {
    const chosen = event.target.value;
    event.target.value = "";
    if (!chosen) return;
    queue(async () => {
      adopt(await invoke("load_profile", { path: chosen }));
      flash(S.load);
    });
  });

  $("reset").addEventListener("click", () =>
    queue(async () => {
      adopt(await invoke("reset_profile"));
      flash(S.reset);
    })
  );

  // Capture phase, so the press is taken before anything else reacts to it.
  window.addEventListener("keydown", onCaptureInput, true);
  window.addEventListener("mousedown", onCaptureInput, true);
  window.addEventListener("auxclick", (event) => {
    if (capture) event.preventDefault();
  }, true);

  try {
    if (tauriEvent && typeof tauriEvent.listen === "function") {
      await tauriEvent.listen("status", (event) => applyStatus(event.payload));
      // The authoritative capture path: it reads the same keys the hotkey poller
      // will read, so a remapping mouse driver cannot hide a press from us.
      await tauriEvent.listen("capture-key", (event) => {
        if (capture && event.payload) {
          capture.detected = event.payload;
          renderHotkeys();
        }
      });
    } else {
      setInterval(() => readState().catch(() => {}), 500);
    }
  } catch (error) {
    setInterval(() => readState().catch(() => {}), 500);
  }

  await readState();
  renderHotkeys(); // first paint; applyStatus only rebuilds when a bind changes
  flash(S.hint);
}

boot().catch((error) => showFailure(error && error.message ? error.message : error));
