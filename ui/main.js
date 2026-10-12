import { S } from "./strings.js";

const { invoke } = window.__TAURI__.core;
// `event` may be absent depending on how the global API is injected; the
// polling fallback below keeps the status live either way.
const tauriEvent = (window.__TAURI__ && window.__TAURI__.event) || null;

// Mirrors Key::KEY_CHOICES on the Rust side.
const KEY_CHOICES = [
  "1", "2", "3", "4", "5", "6",
  "Q", "W", "E", "R", "T", "A", "S", "D", "F", "G", "Z", "X", "C", "V", "B",
  "Space", "Tab", "Enter", "Esc",
  "Mouse Left", "Mouse Right", "Mouse Middle",
  ...Array.from({ length: 24 }, (_, i) => `F${i + 1}`)
];

let rows = [];
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
      renderRows(); // Hold rows have no interval
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

    const remove = document.createElement("button");
    remove.className = "remove";
    remove.textContent = "\u00d7";
    remove.title = S.removeRow;
    remove.addEventListener("click", () => removeRow(index));

    line.append(toggle, key, action, ms, remove);
    host.append(line);
  });
}

// Every mutation goes through one chain so two quick edits cannot race and
// overwrite each other, and the DOM is always rendered from the backend's answer
// rather than from a local array that may have drifted.
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

function removeRow(index) {
  queue(async () => adopt(await invoke("remove_row", { index })));
}

function addRow() {
  queue(async () => adopt(await invoke("add_row")));
}

async function fireOnce() {
  const target = rows.find((row) => row.enabled) || rows[0];
  if (!target) return;
  try {
    await invoke("test_fire", { key: target.key });
    flash(`${S.testFire}: ${target.key}`);
  } catch (error) {
    flash(String(error));
  }
}

function applyStatus(status) {
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

  // Sync once, so the backend and the checkboxes agree without fighting clicks.
  if (!optionsSynced) {
    $("gate").checked = status.gate;
    $("scancode").checked = status.scancode;
    optionsSynced = true;
  }
}

async function pushOptions() {
  try {
    await invoke("set_options", {
      gate: $("gate").checked,
      scancode: $("scancode").checked
    });
  } catch (error) {
    flash(String(error));
  }
}

function showFailure(message) {
  const warn = $("warn");
  warn.textContent = `UI error: ${message}`;
  warn.classList.remove("hidden");
  flash(String(message));
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

async function boot() {
  applyStrings();

  $("add").addEventListener("click", addRow);
  $("fire").addEventListener("click", fireOnce);
  $("start").addEventListener("click", () => invoke("arm").catch((e) => flash(String(e))));
  $("stop").addEventListener("click", () => invoke("disarm").catch((e) => flash(String(e))));
  $("gate").addEventListener("change", pushOptions);
  $("scancode").addEventListener("change", pushOptions);

  $("save").addEventListener("click", async () => {
    try {
      flash(`${S.savedTo} ${await invoke("save_profile")}`);
    } catch (error) {
      flash(String(error));
    }
  });

  $("load").addEventListener("click", () =>
    queue(async () => {
      adopt(await invoke("load_profile"));
      flash(S.load);
    })
  );

  $("reset").addEventListener("click", () =>
    queue(async () => {
      adopt(await invoke("reset_profile"));
      flash(S.reset);
    })
  );

  try {
    if (tauriEvent && typeof tauriEvent.listen === "function") {
      await tauriEvent.listen("status", (event) => applyStatus(event.payload));
    } else {
      setInterval(() => readState().catch(() => {}), 500);
    }
  } catch (error) {
    setInterval(() => readState().catch(() => {}), 500);
  }

  const first = await readState();
  flash(S.hint);
  return first;
}

boot().catch((error) => showFailure(error && error.message ? error.message : error));
