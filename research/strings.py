import re, sys, collections

path = sys.argv[1]
data = open(path, "rb").read()

ascii_re = re.compile(rb"[\x20-\x7e]{5,}")
wide_re = re.compile(rb"(?:[\x20-\x7e]\x00){4,}")

ascii_str = set()
wide_str = set()
for m in ascii_re.finditer(data):
    ascii_str.add(m.group().decode("latin1"))
for m in wide_re.finditer(data):
    wide_str.add(m.group().decode("utf-16-le"))

print(f"ascii strings: {len(ascii_str)}   wide strings: {len(wide_str)}")


def show(title, pat, limit=400):
    rx = re.compile(pat, re.I)
    hits = sorted({s for s in (ascii_str | wide_str) if rx.search(s)})
    print(f"\n### {title} ({len(hits)})")
    for s in hits[:limit]:
        print("   ", s[:200])


show("DLLs / libraries", r"\.(dll|sys|ocx|so|node)$|\.dll\b")
show(
    "Game / window names",
    r"diablo|d4|last epoch|game window|window class|blizzard|battle\.net|steam",
)
show(
    "Input APIs (dynamic)",
    r"sendinput|postmessage|sendmessage|setcursorpos|mouse_event|keybd_event|setwindowshook|rawinput|interception|vigem|vjoy|xinput|dinput|hid|driver|inject",
)
show(
    "Network / URLs",
    r"https?://|\.com|\.net|\.io|api\.|license|token|auth|webhook|discord",
)
show(
    "Features / config",
    r"buff|spam|autocast|auto-?cast|cooldown|cool-?down|macro|hotkey|keybind|skill|rotation|profile|plugin|script|lua|vision|pixel|screenshot|template|ocr",
)
show(
    "Files / paths",
    r"\.json|\.ini|\.cfg|\.toml|\.yaml|\.xml|\.lua|config|settings|profile",
)
show(
    "Runtime / language hints",
    r"rustc|golang|go1\.|rust_|cargo|libcore|panic|dwproc|c\+\+|qt|imgui|mfc|wxwidgets|electron|chrome|cef",
)
