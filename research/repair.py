import io

P = r"C:\repo\last-epoch\spike\le_turbo_spike.py"
src = io.open(P, encoding="utf-8").read().split("\n")

# 1) un-join the comment line that swallowed the next statement
for i, ln in enumerate(src):
    if "key map ---MOUSE_BUTTONS" in ln:
        src[i] = "# " + "-" * 66 + " key map ---"
        src.insert(i + 1, "MOUSE_BUTTONS = {")
        break

# 2) drop the dead WndProc / message-only-window machinery (duplicated)
first = next(i for i, l in enumerate(src) if l.startswith("LRESULT = ctypes.c_ssize_t"))
last = max(i for i, l in enumerate(src) if l.strip() == "return hwnd, delivered")
del src[first : last + 1]
while first < len(src) and src[first].strip() == "":
    del src[first]

io.open(P, "w", encoding="utf-8", newline="\n").write("\n".join(src))
print("lines now:", len(src))
