import sys, math, collections, datetime
import pefile

path = sys.argv[1]
pe = pefile.PE(path, fast_load=False)


def entropy(data):
    if not data:
        return 0.0
    c = collections.Counter(data)
    n = len(data)
    return -sum((v / n) * math.log2(v / n) for v in c.values())


print("=== HEADERS ===")
print("path           :", path)
print(
    "machine        :",
    hex(pe.FILE_HEADER.Machine),
    {0x14C: "i386", 0x8664: "x86-64"}.get(pe.FILE_HEADER.Machine, "?"),
)
print(
    "timestamp      :",
    datetime.datetime.utcfromtimestamp(pe.FILE_HEADER.TimeDateStamp),
    "(epoch",
    pe.FILE_HEADER.TimeDateStamp,
    ")",
)
print(
    "characteristics:",
    hex(pe.FILE_HEADER.Characteristics),
    "DLL" if pe.FILE_HEADER.Characteristics & 0x2000 else "EXE",
)
print(
    "subsystem      :",
    pe.OPTIONAL_HEADER.Subsystem,
    {2: "GUI", 3: "CONSOLE"}.get(pe.OPTIONAL_HEADER.Subsystem, "?"),
)
print("entrypoint RVA :", hex(pe.OPTIONAL_HEADER.AddressOfEntryPoint))
print("image base     :", hex(pe.OPTIONAL_HEADER.ImageBase))

print("\n=== SECTIONS ===")
for s in pe.sections:
    name = s.Name.rstrip(b"\x00").decode("latin1")
    print(
        f"{name:<10} vsize={s.Misc_VirtualSize:<10} rawsize={s.SizeOfRawData:<10} "
        f"rawptr={hex(s.PointerToRawData):<12} entropy={entropy(s.get_data()):.3f} "
        f"flags={hex(s.Characteristics)}"
    )

print("\n=== DATADIR (non-empty) ===")
DIRS = [
    "EXPORT",
    "IMPORT",
    "RESOURCE",
    "EXCEPTION",
    "SECURITY",
    "BASERELOC",
    "DEBUG",
    "ARCHITECTURE",
    "GLOBALPTR",
    "TLS",
    "LOAD_CONFIG",
    "BOUND_IMPORT",
    "IAT",
    "DELAY_IMPORT",
    "CLR",
    "RESERVED",
]
for i, d in enumerate(pe.OPTIONAL_HEADER.DATA_DIRECTORY):
    if d.VirtualAddress or d.Size:
        label = DIRS[i] if i < len(DIRS) else str(i)
        print(f"{label:<14} rva={hex(d.VirtualAddress):<12} size={d.Size}")

print("\n=== IMPORTS ===")
imp = getattr(pe, "DIRECTORY_ENTRY_IMPORT", [])
print("dll count:", len(imp))
for entry in imp:
    dll = entry.dll.decode("latin1")
    names = []
    for f in entry.imports:
        names.append(f.name.decode("latin1") if f.name else f"#{f.ordinal}")
    print(f"\n-- {dll} ({len(names)})")
    for n in sorted(names):
        print("   ", n)

di = getattr(pe, "DIRECTORY_ENTRY_DELAY_IMPORT", [])
if di:
    print("\n=== DELAY IMPORTS ===")
    for entry in di:
        print("--", entry.dll.decode("latin1"))
        for f in entry.imports:
            print("   ", f.name.decode("latin1") if f.name else f"#{f.ordinal}")

dbg = getattr(pe, "DIRECTORY_ENTRY_DEBUG", [])
if dbg:
    print("\n=== DEBUG ===")
    for d in dbg:
        if hasattr(d, "entry"):
            try:
                print("   ", d.entry.PdbFileName.rstrip(b"\x00").decode("latin1"))
            except Exception:
                print("   (debug entry type", d.struct.Type, ")")

print("\n=== OVERLAY ===")
print("size of image (file):", len(open(path, "rb").read()))
last = max((s.PointerToRawData + s.SizeOfRawData) for s in pe.sections)
print("last section end    :", last)
print("overlay bytes       :", len(open(path, "rb").read()) - last)

print("\n=== RESOURCES (types) ===")
try:
    for r in pe.DIRECTORY_ENTRY_RESOURCE.entries:
        print("   ", r.id if r.id is not None else r.name, r.struct.Name)
except Exception as e:
    print("   none/err", e)
