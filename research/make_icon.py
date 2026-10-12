"""Generate icons/icon.ico for the Windows resource.

tauri-build requires icons/icon.ico during the Windows resource step, and the
plan did not create any artwork. This writes a classic BMP-based multi-size ICO
(16/32/48) so the resource compiler is happy without depending on a PNG-in-ICO
extension. Replace with real artwork later.
"""

import os
import struct

BG = (0x10, 0x12, 0x0C)  # #0c1210 as RGB
GOLD = (0x4F, 0xA9, 0xC9)  # #c9a94f as RGB


def pixels(size):
    """A gold 'T' on the app's dark green, as a list of RGB rows."""
    rows = []
    bar_h = max(2, size // 5)
    stem_w = max(2, size // 5)
    stem_x = (size - stem_w) // 2
    for y in range(size):
        row = []
        for x in range(size):
            in_top_bar = y < bar_h and x >= size // 6 and x < size - size // 6
            in_stem = y >= bar_h - 1 and stem_x <= x < stem_x + stem_w
            row.append(GOLD if (in_top_bar or in_stem) else BG)
        rows.append(row)
    return rows


def ico_image(size):
    """BITMAPINFOHEADER + bottom-up BGRA XOR bitmap + 1bpp AND mask."""
    rows = pixels(size)
    xor = bytearray()
    for y in range(size - 1, -1, -1):
        for r, g, b in rows[y]:
            xor += bytes((b, g, r, 0xFF))

    mask_stride = ((size + 31) // 32) * 4
    mask = bytearray()
    for y in range(size - 1, -1, -1):
        row_bits = bytearray(mask_stride)
        for x in range(size):
            if rows[y][x] == BG:
                row_bits[x // 8] |= 0x80 >> (x % 8)
        mask += row_bits

    header = struct.pack(
        "<IiiHHIIiiII",
        40,  # biSize
        size,  # biWidth
        size * 2,  # biHeight = XOR + AND
        1,  # biPlanes
        32,  # biBitCount
        0,  # biCompression = BI_RGB
        len(xor) + len(mask),
        0,
        0,
        0,
        0,
    )
    return header + bytes(xor) + bytes(mask)


def main():
    sizes = (16, 32, 48)
    images = [ico_image(s) for s in sizes]

    out = bytearray(struct.pack("<HHH", 0, 1, len(images)))
    offset = 6 + 16 * len(images)
    for s, img in zip(sizes, images):
        out += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(img), offset)
        offset += len(img)
    for img in images:
        out += img

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    icons = os.path.join(root, "icons")
    os.makedirs(icons, exist_ok=True)
    path = os.path.join(icons, "icon.ico")
    with open(path, "wb") as fh:
        fh.write(out)
    print(f"wrote {path} ({len(out)} bytes, sizes {sizes})")


if __name__ == "__main__":
    main()
