"""
THROWAWAY SPIKE - Last Epoch auto-clicker prototype.

NOT production code. No tests. Planned replacement: Rust + Tauri + enigo.

Fixes in this revision:
  * global hotkeys now use a hidden message-only window + WndProc, so Tk's own
    message pump can no longer swallow WM_HOTKEY (F2/F3/F9 work regardless of focus)
  * registration failures are reported in the UI
  * themed UI (Last Epoch style: dark teal/green, aged gold, ornate panels)

Run:  python spike/le_turbo_spike.py
Keys: F2 start, F3 stop, F9 quit
"""

import ctypes
import ctypes.wintypes as wintypes
import json
import os
import sys
import threading
import time
import tkinter as tk
from tkinter import ttk, messagebox

user32 = ctypes.WinDLL("user32", use_last_error=True)


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", ctypes.c_long),
        ("top", ctypes.c_long),
        ("right", ctypes.c_long),
        ("bottom", ctypes.c_long),
    ]


class MONITORINFO(ctypes.Structure):
    _fields_ = [
        ("cbSize", wintypes.DWORD),
        ("rcMonitor", RECT),
        ("rcWork", RECT),
        ("dwFlags", wintypes.DWORD),
    ]


MONITORENUMPROC = ctypes.WINFUNCTYPE(
    wintypes.BOOL,
    wintypes.HMONITOR,
    wintypes.HDC,
    ctypes.POINTER(RECT),
    wintypes.LPARAM,
)
user32.GetMonitorInfoW.argtypes = (wintypes.HMONITOR, ctypes.POINTER(MONITORINFO))
user32.EnumDisplayMonitors.argtypes = (
    wintypes.HDC,
    ctypes.POINTER(RECT),
    MONITORENUMPROC,
    wintypes.LPARAM,
)
user32.GetParent.argtypes = (wintypes.HWND,)
user32.GetParent.restype = wintypes.HWND
user32.SetWindowPos.argtypes = (
    wintypes.HWND,
    wintypes.HWND,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    wintypes.UINT,
)

HWND_TOPMOST = wintypes.HWND(-1)
SWP_NOSIZE = 0x0001
SWP_SHOWWINDOW = 0x0040
MONITORINFOF_PRIMARY = 1


def list_monitors():
    """All monitors with their work areas, primary first."""
    found = []

    def _cb(hmon, hdc, lprect, data):
        mi = MONITORINFO()
        mi.cbSize = ctypes.sizeof(MONITORINFO)
        if user32.GetMonitorInfoW(hmon, ctypes.byref(mi)):
            found.append(mi)
        return True

    user32.EnumDisplayMonitors(None, None, MONITORENUMPROC(_cb), 0)
    found.sort(key=lambda m: 0 if (m.dwFlags & MONITORINFOF_PRIMARY) else 1)
    return found


def app_hwnd(root):
    """Real toplevel HWND behind a Tk window."""
    root.update_idletasks()
    child = wintypes.HWND(root.winfo_id())
    return user32.GetParent(child) or child


# ---------------------------------------------------------------- SendInput ---
INPUT_MOUSE = 0
INPUT_KEYBOARD = 1
KEYEVENTF_KEYUP = 0x0002
KEYEVENTF_SCANCODE = 0x0008

MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004
MOUSEEVENTF_RIGHTDOWN = 0x0008
MOUSEEVENTF_RIGHTUP = 0x0010
MOUSEEVENTF_MIDDLEDOWN = 0x0020
MOUSEEVENTF_MIDDLEUP = 0x0040


class MOUSEINPUT(ctypes.Structure):
    _fields_ = [
        ("dx", wintypes.LONG),
        ("dy", wintypes.LONG),
        ("mouseData", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.POINTER(ctypes.c_ulong)),
    ]


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [
        ("wVk", wintypes.WORD),
        ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.POINTER(ctypes.c_ulong)),
    ]


class HARDWAREINPUT(ctypes.Structure):
    _fields_ = [
        ("uMsg", wintypes.DWORD),
        ("wParamL", wintypes.WORD),
        ("wParamH", wintypes.WORD),
    ]


class _INPUTUNION(ctypes.Union):
    _fields_ = [("mi", MOUSEINPUT), ("ki", KEYBDINPUT), ("hi", HARDWAREINPUT)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _INPUTUNION)]


user32.SendInput.argtypes = (wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int)
user32.SendInput.restype = wintypes.UINT
user32.MapVirtualKeyW.argtypes = (wintypes.UINT, wintypes.UINT)

if ctypes.sizeof(INPUT) != (40 if sys.maxsize > 2**32 else 28):
    raise SystemExit("FATAL: unexpected sizeof(INPUT)")

# ------------------------------------------------------------- global hotkeys ---
# RegisterHotKey turned out to be the wrong tool for a spike: Windows will not
# fire a registered hotkey for SendInput-generated keys, and Tk's own message
# pump swallows WM_HOTKEY before a handler window can see it. Polling
# GetAsyncKeyState is focus-independent and is exercised by spike/test_hotkeys.py.
VK_HOTKEYS = [(1, "F2", 0x71), (2, "F3", 0x72), (3, "F9", 0x78)]

user32.GetAsyncKeyState.argtypes = (ctypes.c_int,)
user32.GetAsyncKeyState.restype = ctypes.c_short


class KeyPoller:
    """Edge-detecting global key poller (F2/F3/F9 by default)."""

    def __init__(self):
        self._down = {}

    def poll(self):
        fired = []
        for hid, _name, vk in VK_HOTKEYS:
            state = user32.GetAsyncKeyState(vk)
            now_down = bool(state & 0x8000)
            tapped = bool(state & 0x0001)  # set if pressed since our previous poll
            if (now_down and not self._down.get(hid)) or tapped:
                fired.append(hid)
            self._down[hid] = now_down
        return fired


# ------------------------------------------------------------------ key map ---
MOUSE_BUTTONS = {
    "Mouse Left": (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
    "Mouse Right": (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
    "Mouse Middle": (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
}

NAMED_KEYS = {
    "Space": 0x20,
    "Enter": 0x0D,
    "Tab": 0x09,
    "Esc": 0x1B,
    "Shift": 0x10,
    "Ctrl": 0x11,
    "Alt": 0x12,
}
for _i in range(1, 25):
    NAMED_KEYS[f"F{_i}"] = 0x6F + _i

KEY_CHOICES = (
    [
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "Q",
        "W",
        "E",
        "R",
        "T",
        "A",
        "S",
        "D",
        "F",
        "G",
        "Z",
        "X",
        "C",
        "V",
        "B",
        "Space",
        "Esc",
        "Tab",
        "Enter",
        "Shift",
        "Ctrl",
        "Alt",
    ]
    + [f"F{i}" for i in range(1, 13)]
    + list(MOUSE_BUTTONS)
)


def vk_for(key):
    if key in MOUSE_BUTTONS:
        return None
    if key in NAMED_KEYS:
        return NAMED_KEYS[key]
    if len(key) == 1:
        ch = key.upper()
        if "0" <= ch <= "9":
            return 0x30 + int(ch)
        if "A" <= ch <= "Z":
            return ord(ch)
    return None


def _send(inputs):
    arr = (INPUT * len(inputs))(*inputs)
    return user32.SendInput(len(inputs), arr, ctypes.sizeof(INPUT)) == len(inputs)


def press(key, use_scancode=False):
    """One press+release of a key or mouse button, at the current cursor position."""
    if key in MOUSE_BUTTONS:
        down, up = MOUSE_BUTTONS[key]
        mk = lambda f: INPUT(
            type=INPUT_MOUSE, u=_INPUTUNION(mi=MOUSEINPUT(0, 0, 0, f, 0, None))
        )
        return _send([mk(down), mk(up)])

    vk = vk_for(key)
    if vk is None:
        return False
    flags = KEYEVENTF_SCANCODE if use_scancode else 0

    def kb(f):
        return INPUT(
            type=INPUT_KEYBOARD,
            u=_INPUTUNION(
                ki=KEYBDINPUT(
                    0 if use_scancode else vk,
                    user32.MapVirtualKeyW(vk, 0) if use_scancode else 0,
                    f | flags,
                    0,
                    None,
                )
            ),
        )

    return _send([kb(0), kb(KEYEVENTF_KEYUP)])


# ------------------------------------------------------------ focus tracking ---
def foreground_title():
    hwnd = user32.GetForegroundWindow()
    if not hwnd:
        return ""
    n = user32.GetWindowTextLengthW(hwnd)
    buf = ctypes.create_unicode_buffer(n + 1)
    user32.GetWindowTextW(hwnd, buf, n + 1)
    return buf.value


def last_epoch_focused():
    return "last epoch" in foreground_title().lower()


# ------------------------------------------------------------------- engine ---
class Engine(threading.Thread):
    def __init__(self):
        super().__init__(daemon=True)
        self.rows = []
        self.lock = threading.Lock()
        self.running = False
        self.gate = True
        self.scancode = False
        self.stop_evt = threading.Event()
        self.last_fire = {}
        self.fired = 0

    def snapshot(self):
        with self.lock:
            return list(self.rows)

    def run(self):
        while not self.stop_evt.is_set():
            rows = self.snapshot()
            if not self.running or (self.gate and not last_epoch_focused()):
                time.sleep(0.03)
                continue
            now = time.perf_counter()
            for i, row in enumerate(rows):
                if not row.get("enabled") or not row.get("key"):
                    continue
                ms = max(10, int(row.get("ms") or 1000))
                if now - self.last_fire.get(i, 0.0) >= ms / 1000.0:
                    if press(row["key"], self.scancode):
                        self.fired += 1
                    self.last_fire[i] = now
            time.sleep(0.002)


# ------------------------------------------------------------------- theme ----
BG = "#0c1210"
PANEL = "#121b17"
PANEL_HI = "#16211c"
ROW_ALT = "#101815"
BORDER = "#2f3f36"
GOLD = "#c9a94f"
GOLD_DIM = "#8b7530"
TEAL = "#3fb9a6"
TEXT = "#ded4b2"
MUTED = "#7f8d7c"
DANGER = "#c0512f"

FONT_TITLE = ("Georgia", 21, "bold")
FONT_H = ("Segoe UI", 8, "bold")
FONT_B = ("Segoe UI", 9)
FONT_MONO = ("Consolas", 10, "bold")

DEFAULT_ROWS = [
    {"key": "1", "ms": 1000, "enabled": False},
    {"key": "2", "ms": 1000, "enabled": False},
    {"key": "3", "ms": 1000, "enabled": False},
    {"key": "4", "ms": 1000, "enabled": False},
    {"key": "Mouse Right", "ms": 1000, "enabled": False},
]

PROFILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "profile.json")
STATE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "state.json")

HOTKEY_SPEC = [
    (1, "F2", 0x71, "iniciar o Auto"),
    (2, "F3", 0x72, "parar o Auto"),
    (3, "F9", 0x78, "fechar o app"),
]


def build_style(root):
    st = ttk.Style(root)
    try:
        st.theme_use("clam")
    except tk.TclError:
        pass

    st.configure(".", background=BG, foreground=TEXT, font=FONT_B)
    st.configure("TFrame", background=BG)
    st.configure("Panel.TFrame", background=PANEL, relief="flat")
    st.configure("Row.TFrame", background=ROW_ALT)
    st.configure("RowA.TFrame", background=PANEL_HI)
    st.configure("TLabel", background=BG, foreground=TEXT)
    st.configure("Title.TLabel", background=BG, foreground=GOLD, font=FONT_TITLE)
    st.configure(
        "Sub.TLabel", background=BG, foreground=MUTED, font=("Segoe UI", 7, "bold")
    )
    st.configure("PanelH.TLabel", background=PANEL, foreground=GOLD, font=FONT_H)
    st.configure("Row.TLabel", background=PANEL_HI, foreground=TEXT, font=FONT_B)
    st.configure("RowGrey.TLabel", background=ROW_ALT, foreground=TEXT, font=FONT_B)
    st.configure("Muted.TLabel", background=BG, foreground=MUTED, font=("Segoe UI", 8))
    st.configure(
        "Status.TLabel", background=BG, foreground=TEAL, font=("Segoe UI", 9, "bold")
    )
    st.configure(
        "Badge.TLabel",
        background=GOLD_DIM,
        foreground="#12180f",
        font=("Segoe UI", 8, "bold"),
    )

    # sunken framed panel used for the settings boxes
    st.configure("Box.TFrame", background=PANEL_HI, relief="solid", borderwidth=1)

    st.configure(
        "TButton",
        background=PANEL_HI,
        foreground=GOLD,
        borderwidth=1,
        focusthickness=1,
        focuscolor=GOLD,
        font=("Segoe UI", 9, "bold"),
        padding=(10, 5),
    )
    st.map(
        "TButton",
        background=[("active", "#22302a"), ("pressed", "#2b3d34")],
        foreground=[("disabled", "#5b6b60")],
    )
    st.configure("Go.TButton", background="#1d3a2f", foreground=TEAL)
    st.map("Go.TButton", background=[("active", "#27503f")])
    st.configure("Stop.TButton", background="#3a1f1a", foreground=DANGER)
    st.map("Stop.TButton", background=[("active", "#52281f")])

    st.configure("TCheckbutton", background=PANEL_HI, foreground=TEXT, font=FONT_B)
    st.map(
        "TCheckbutton", background=[("active", PANEL_HI)], foreground=[("active", GOLD)]
    )
    st.configure("Bg.TCheckbutton", background=BG, foreground=TEXT, font=FONT_B)
    st.map(
        "Bg.TCheckbutton", background=[("active", BG)], foreground=[("active", GOLD)]
    )

    st.configure(
        "TEntry",
        fieldbackground="#0a0f0d",
        foreground=GOLD,
        insertcolor=GOLD,
        bordercolor=BORDER,
        lightcolor=BORDER,
        darkcolor=BORDER,
        padding=3,
    )
    st.configure(
        "TCombobox",
        fieldbackground="#0a0f0d",
        background=PANEL_HI,
        foreground=GOLD,
        arrowcolor=GOLD,
        bordercolor=BORDER,
        padding=3,
    )
    st.map("TCombobox", fieldbackground=[("readonly", "#0a0f0d")])
    root.option_add("*TCombobox*Listbox.background", "#0a0f0d")
    root.option_add("*TCombobox*Listbox.foreground", GOLD)
    root.option_add("*TCombobox*Listbox.selectBackground", GOLD_DIM)
    root.option_add("*TCombobox*Listbox.selectForeground", "#12180f")


USER32 = user32


def enable_dark_titlebar(root):
    """Windows 10 2004+ immersive dark title bar."""
    try:
        root.update_idletasks()
        hwnd = user32.GetParent(wintypes.HWND(root.winfo_id())) or wintypes.HWND(
            root.winfo_id()
        )
        dwm = ctypes.WinDLL("dwmapi")
        one = ctypes.c_int(1)
        for attr in (20, 19):
            if (
                dwm.DwmSetWindowAttribute(
                    hwnd, attr, ctypes.byref(one), ctypes.sizeof(one)
                )
                == 0
            ):
                break
    except Exception:
        pass


class App:
    def __init__(self, root, engine):
        self.root = root
        self.engine = engine
        self.row_widgets = []
        engine.rows = [dict(r) for r in DEFAULT_ROWS]
        self.keypoller = KeyPoller()

        root.title("LE Turbo - spike")
        root.configure(bg=BG)
        root.geometry("1010x700")
        root.minsize(980, 660)
        build_style(root)
        enable_dark_titlebar(root)

        self._header()
        body = ttk.Frame(root, padding=(14, 6, 14, 6))
        body.pack(fill="both", expand=True)
        body.columnconfigure(0, weight=1)
        body.columnconfigure(1, weight=0)
        body.rowconfigure(0, weight=1)
        self._table(body)
        self._sidebar(body)
        self._footer()

        self.register_hotkey_labels()
        self.apply_opts()
        self.apply_topmost()
        self.place_window()
        self.tick()

    # ------------------------------------------------------------ header ----
    def _header(self):
        h = tk.Frame(self.root, bg=BG, padx=16, pady=12)
        h.pack(fill="x")
        left = tk.Frame(h, bg=BG)
        left.pack(side="left")
        tk.Label(
            left, text="LAST EPOCH", bg=BG, fg=MUTED, font=("Segoe UI", 7, "bold")
        ).pack(anchor="w")
        tk.Label(left, text="TURBO", bg=BG, fg=GOLD, font=FONT_TITLE).pack(anchor="w")
        tk.Label(
            h,
            text=" v0.1 spike ",
            bg=GOLD_DIM,
            fg="#12180f",
            font=("Segoe UI", 8, "bold"),
        ).place(x=232, y=34)

        btns = tk.Frame(h, bg=BG)
        btns.pack(side="right")
        ttk.Button(btns, text="Salvar", command=self.on_save).pack(side="left", padx=3)
        ttk.Button(btns, text="Carregar", command=self.on_load).pack(
            side="left", padx=3
        )
        ttk.Button(btns, text="Reset", command=self.on_reset).pack(side="left", padx=3)

    # ------------------------------------------------------------- table ----
    def _table(self, parent):
        wrap = ttk.Frame(parent, style="Panel.TFrame", padding=1)
        wrap.grid(row=0, column=0, sticky="nsew", padx=(0, 12))
        outer = tk.Frame(wrap, bg=PANEL, padx=14, pady=12)
        outer.pack(fill="both", expand=True)

        tk.Label(
            outer,
            text="SKILLS & CLICKS",
            bg=PANEL,
            fg=GOLD,
            font=("Segoe UI", 8, "bold"),
        ).pack(anchor="w")
        tk.Frame(outer, bg=BORDER, height=1).pack(fill="x", pady=(6, 8))

        head = tk.Frame(outer, bg=PANEL)
        head.pack(fill="x", pady=(0, 4))
        head.grid_columnconfigure(0, minsize=68)
        head.grid_columnconfigure(1, minsize=150)
        for c, t in enumerate(("ON", "KEY", "MILLISECONDS")):
            tk.Label(
                head,
                text=t,
                bg=PANEL,
                fg=MUTED,
                anchor="w",
                font=("Segoe UI", 7, "bold"),
            ).grid(row=0, column=c, sticky="w")

        self.rows_holder = tk.Frame(outer, bg=PANEL)
        self.rows_holder.pack(fill="x", pady=(0, 10))
        for r in engine_rows(engine=self.engine):
            self.add_row_widget(r)

        btns = tk.Frame(outer, bg=PANEL)
        btns.pack(fill="x")
        ttk.Button(btns, text="+ Adicionar linha", command=self.on_add).pack(
            side="left"
        )
        ttk.Button(btns, text="Testar 1x", command=self.on_test).pack(
            side="left", padx=8
        )

    # ----------------------------------------------------------- sidebar ----
    def _sidebar(self, parent):
        side = tk.Frame(parent, bg=BG, width=310)
        side.grid(row=0, column=1, sticky="ns")
        side.grid_propagate(False)

        tk.Label(
            side,
            text="HOTKEYS & SETTINGS",
            bg=BG,
            fg=GOLD,
            font=("Segoe UI", 8, "bold"),
        ).pack(anchor="w", pady=(0, 8))

        tk.Label(
            side, text="HOTKEYS", bg=BG, fg=MUTED, font=("Segoe UI", 7, "bold")
        ).pack(anchor="w", pady=(6, 4))
        self.hotkey_labels = {}
        for hid, key, vk, desc in HOTKEY_SPEC:
            box = tk.Frame(
                side,
                bg=PANEL_HI,
                highlightbackground=BORDER,
                highlightthickness=1,
                padx=10,
                pady=7,
            )
            box.pack(fill="x", pady=2)
            badge = tk.Label(
                box,
                text=key,
                bg=PANEL_HI,
                fg=GOLD,
                font=FONT_MONO,
                width=5,
                relief="flat",
            )
            badge.pack(side="left")
            lbl = tk.Label(box, text=desc, bg=PANEL_HI, fg=TEXT, font=FONT_B)
            lbl.pack(side="left")
            self.hotkey_labels[hid] = lbl

        tk.Label(
            side, text="STATUS", bg=BG, fg=MUTED, font=("Segoe UI", 7, "bold")
        ).pack(anchor="w", pady=(12, 4))
        self.status_box = tk.Frame(
            side,
            bg=PANEL_HI,
            highlightbackground=BORDER,
            highlightthickness=1,
            padx=10,
            pady=8,
        )
        self.status_box.pack(fill="x")
        self.lbl_auto = tk.Label(
            self.status_box,
            text="AUTO: OFF",
            bg=PANEL_HI,
            fg=DANGER,
            font=("Segoe UI", 10, "bold"),
        )
        self.lbl_auto.pack(anchor="w")
        self.lbl_focus = tk.Label(
            self.status_box,
            text="",
            bg=PANEL_HI,
            fg=MUTED,
            font=("Segoe UI", 8),
            justify="left",
            anchor="w",
            wraplength=265,
        )
        self.lbl_focus.pack(anchor="w", pady=(4, 0))
        self.lbl_sent = tk.Label(
            self.status_box,
            text="envios: 0",
            bg=PANEL_HI,
            fg=TEAL,
            font=("Segoe UI", 8, "bold"),
        )
        self.lbl_sent.pack(anchor="w", pady=(2, 0))

        tk.Label(
            side, text="OPCOES", bg=BG, fg=MUTED, font=("Segoe UI", 7, "bold")
        ).pack(anchor="w", pady=(12, 4))
        self.gate_var = tk.BooleanVar(value=True)
        self.scan_var = tk.BooleanVar(value=False)
        self.top_var = tk.BooleanVar(value=True)
        self.mon2_var = tk.BooleanVar(value=True)
        for text, var, cmd in (
            ("So injetar com Last Epoch em foco", self.gate_var, self.apply_opts),
            ("Modo scancode (se VK falhar)", self.scan_var, self.apply_opts),
            ("Janela sempre no topo", self.top_var, self.apply_topmost),
            (
                "Abrir no outro monitor (nao o do jogo)",
                self.mon2_var,
                self.place_window,
            ),
        ):
            ttk.Checkbutton(
                side, text=text, variable=var, command=cmd, style="Bg.TCheckbutton"
            ).pack(anchor="w", pady=1)

        hint = tk.Frame(
            side,
            bg=PANEL_HI,
            highlightbackground=BORDER,
            highlightthickness=1,
            padx=10,
            pady=8,
        )
        hint.pack(fill="x", pady=(12, 0))
        tk.Label(
            hint,
            text="Clique no campo KEY pra trocar a tecla. Marque ON nas linhas "
            "que devem disparar. Aperte F2 pra ligar.",
            bg=PANEL_HI,
            fg=MUTED,
            font=("Segoe UI", 8),
            wraplength=260,
            justify="left",
        ).pack(anchor="w")

    def _footer(self):
        bar = tk.Frame(self.root, bg=BG, padx=16, pady=8)
        bar.pack(fill="x")
        ttk.Button(
            bar, text="START  (F2)", style="Go.TButton", command=self.start
        ).pack(side="left")
        ttk.Button(
            bar, text="STOP  (F3)", style="Stop.TButton", command=self.stop
        ).pack(side="left", padx=8)
        self.lbl_hint = tk.Label(bar, text="", bg=BG, fg=MUTED, font=("Segoe UI", 8))
        self.lbl_hint.pack(side="right")

    # ------------------------------------------------------------- rows -----
    def add_row_widget(self, row):
        i = len(self.row_widgets)
        bgc = PANEL_HI if i % 2 == 0 else ROW_ALT
        f = tk.Frame(
            self.rows_holder, bg=bgc, highlightbackground=BORDER, highlightthickness=1
        )
        f.pack(fill="x", pady=2)
        f.grid_columnconfigure(0, minsize=68)
        f.grid_columnconfigure(1, minsize=150)

        en = tk.BooleanVar(value=row["enabled"])
        tog = tk.Button(
            f,
            width=5,
            relief="flat",
            bd=0,
            highlightthickness=0,
            cursor="hand2",
            font=("Segoe UI", 8, "bold"),
            command=lambda: self.toggle_row(i),
        )
        tog.grid(row=0, column=0, padx=(6, 8), pady=5, sticky="w")
        self.paint_toggle(tog, en)

        kv = tk.StringVar(value=row["key"])
        cbx = ttk.Combobox(
            f,
            values=KEY_CHOICES,
            textvariable=kv,
            width=15,
            state="readonly",
            font=FONT_MONO,
        )
        cbx.grid(row=0, column=1, sticky="w", pady=5)
        cbx.bind("<<ComboboxSelected>>", lambda e: self.sync())

        mv = tk.StringVar(value=str(row["ms"]))
        ent = ttk.Entry(f, textvariable=mv, width=12, font=FONT_MONO)
        ent.grid(row=0, column=2, sticky="w", padx=(10, 0), pady=5)
        ent.bind("<KeyRelease>", lambda ev: self.sync())

        tk.Button(
            f,
            text="\u00d7",
            bg=bgc,
            fg=DANGER,
            activebackground=bgc,
            activeforeground=GOLD,
            bd=0,
            highlightthickness=0,
            font=("Segoe UI", 11, "bold"),
            cursor="hand2",
            command=lambda: self.remove_row(i),
        ).grid(row=0, column=3, padx=10)

        f.columnconfigure(2, weight=1)
        self.row_widgets.append(
            {"frame": f, "enabled": en, "key": kv, "ms": mv, "toggle": tog}
        )

    @staticmethod
    def paint_toggle(btn, var):
        if var.get():
            btn.config(
                text="ON",
                bg=TEAL,
                fg="#0d1310",
                activebackground=TEAL,
                activeforeground="#0d1310",
            )
        else:
            btn.config(
                text="OFF",
                bg="#1a241f",
                fg=MUTED,
                activebackground="#243029",
                activeforeground=GOLD,
            )

    def toggle_row(self, idx):
        if not (0 <= idx < len(self.row_widgets)):
            return
        w = self.row_widgets[idx]
        w["enabled"].set(not w["enabled"].get())
        self.paint_toggle(w["toggle"], w["enabled"])
        self.sync()

    def _reindex_rows(self):
        rows = self.engine.snapshot()
        for w in self.row_widgets:
            w["frame"].destroy()
        self.row_widgets.clear()
        for r in rows:
            self.add_row_widget(r)

    def on_add(self):
        with self.engine.lock:
            self.engine.rows.append({"key": "1", "ms": 1000, "enabled": False})
        self._reindex_rows()
        self.sync()

    def remove_row(self, idx):
        if len(self.row_widgets) <= 1:
            return
        with self.engine.lock:
            rows = self.engine.rows
            if 0 <= idx < len(rows):
                rows.pop(idx)
        self._reindex_rows()
        self.sync()

    def sync(self):
        rows = []
        for w in self.row_widgets:
            try:
                ms = int(w["ms"].get() or 0)
            except ValueError:
                ms = 1000
            rows.append(
                {
                    "key": w["key"].get().strip(),
                    "ms": max(10, ms),
                    "enabled": bool(w["enabled"].get()),
                }
            )
        with self.engine.lock:
            self.engine.rows = rows

    # ---------------------------------------------------------- hotkeys -----
    def register_hotkey_labels(self):
        """Polling is always available; just mark the labels as active."""
        for hid, key, vk, desc in HOTKEY_SPEC:
            self.hotkey_labels[hid].config(text=desc + "  \u2713", fg=TEXT)

    def poll_hotkeys(self):
        for hid in self.keypoller.poll():
            if hid == 1:
                self.start()
            elif hid == 2:
                self.stop()
            elif hid == 3:
                self.stop()
                self.root.destroy()
                return

    # ----------------------------------------------------------- actions ----
    def apply_opts(self):
        self.engine.gate = self.gate_var.get()
        self.engine.scancode = self.scan_var.get()

    def apply_topmost(self):
        self.root.attributes("-topmost", bool(self.top_var.get()))

    def place_window(self):
        """Park the window on the non-primary monitor (the one without the game)."""
        mons = list_monitors()
        if not mons:
            return
        if self.mon2_var.get() and len(mons) > 1:
            target = next(
                (m for m in mons if not (m.dwFlags & MONITORINFOF_PRIMARY)), mons[0]
            )
        else:
            target = mons[0]
        w = max(self.root.winfo_width(), 1010)
        h = max(self.root.winfo_height(), 700)
        x = target.rcWork.left + 16
        y = target.rcWork.top + 16
        user32.SetWindowPos(
            app_hwnd(self.root), HWND_TOPMOST, x, y, w, h, SWP_NOSIZE | SWP_SHOWWINDOW
        )
        self.lbl_hint.config(
            text=f"janela no monitor em ({x},{y})  |  topo: {bool(self.top_var.get())}",
            fg=MUTED,
        )

    def start(self):
        self.sync()
        self.engine.fired = 0
        self.engine.last_fire.clear()
        self.engine.running = True
        self.lbl_auto.config(text="AUTO: ON", fg=TEAL)

    def stop(self):
        self.engine.running = False
        self.lbl_auto.config(text="AUTO: OFF", fg=DANGER)

    def on_test(self):
        self.sync()
        for w in self.row_widgets:
            if w["enabled"].get():
                press(w["key"].get().strip(), self.scan_var.get())
                return
        if self.row_widgets:
            press(self.row_widgets[0]["key"].get().strip(), self.scan_var.get())

    def on_save(self):
        self.sync()
        with open(PROFILE, "w", encoding="utf-8") as fh:
            json.dump({"rows": self.engine.snapshot()}, fh, indent=1)
        self.lbl_hint.config(text=f"salvo em {PROFILE}", fg=TEAL)

    def on_load(self):
        if not os.path.exists(PROFILE):
            messagebox.showwarning("LE Turbo", "Nenhum profile.json ainda.")
            return
        with open(PROFILE, encoding="utf-8") as fh:
            rows = json.load(fh).get("rows", [])
        with self.engine.lock:
            self.engine.rows = [
                {
                    "key": r.get("key", "1"),
                    "ms": r.get("ms", 1000),
                    "enabled": bool(r.get("enabled")),
                }
                for r in rows
            ]
        self._reindex_rows()
        self.sync()
        self.lbl_hint.config(text="profile carregado", fg=TEAL)

    def on_reset(self):
        with self.engine.lock:
            self.engine.rows = [dict(r) for r in DEFAULT_ROWS]
        self._reindex_rows()
        self.sync()

    # -------------------------------------------------------------- loop ----
    def tick(self):
        self.poll_hotkeys()
        if not self.root.winfo_exists():
            return
        fg = foreground_title() or "(nenhuma)"
        ok = "sim" if last_epoch_focused() else "nao"
        self.lbl_focus.config(text=f"foco: {fg}\nLast Epoch em foco: {ok}")
        self.lbl_sent.config(text=f"envios: {self.engine.fired}")
        try:
            with open(STATE, "w", encoding="utf-8") as fh:
                json.dump(
                    {
                        "auto": self.engine.running,
                        "sends": self.engine.fired,
                        "foreground": fg,
                        "le_focused": ok == "sim",
                        "hotkeys": {"poll": True, "F2": True, "F3": True, "F9": True},
                        "window": {
                            "x": self.root.winfo_rootx(),
                            "y": self.root.winfo_rooty(),
                            "w": self.root.winfo_width(),
                            "h": self.root.winfo_height(),
                        },
                        "rows_on": sum(
                            1 for w in self.row_widgets if w["enabled"].get()
                        ),
                        "toggles": [
                            {
                                "on": bool(w["enabled"].get()),
                                "x": w["toggle"].winfo_rootx()
                                + w["toggle"].winfo_width() // 2,
                                "y": w["toggle"].winfo_rooty()
                                + w["toggle"].winfo_height() // 2,
                            }
                            for w in self.row_widgets
                        ],
                    },
                    fh,
                )
        except OSError:
            pass
        self.root.after(60, self.tick)


def engine_rows(engine):
    with engine.lock:
        return list(engine.rows)


def main():
    if sys.platform == "win32":
        ctypes.WinDLL("user32").SetProcessDPIAware()
    engine = Engine()
    engine.start()
    root = tk.Tk()
    app = App(root, engine)
    try:
        root.mainloop()
    finally:
        engine.stop_evt.set()


if __name__ == "__main__":
    main()
