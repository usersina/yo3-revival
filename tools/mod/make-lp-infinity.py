#!/usr/bin/env python3
"""The duel's LP "∞" picture (tools/mod/duel-numbers.h): the connection DLL carries it inside itself, as the C header
lp-infinity-png.h, and shows it in place of the LP counter's digits when a player's LP is beyond 99,999.

Usage:
  make-lp-infinity.py                 the header from tools/mod/art/lp-infinity.png (after replacing that picture)
  make-lp-infinity.py --placeholder   draw the generated placeholder into tools/mod/art/lp-infinity.png first
  make-lp-infinity.py --check         exit 1 when the header is not made from the picture

The picture: a PNG with an alpha channel (colour type 6, 8 bits), the "∞" centred on a transparent background, drawn at
four times the screen's size: the DLL shows it at 25%, its middle on the middle of the LP counter's digits (13x14 screen
pixels each). 160x64 shows as 40x16 (tools/mod/art/README.md, docs/client-internals.md "Numbers in the duel")."""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]):
    print(__doc__); raise SystemExit(0)
import math
import os
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
PICTURE = os.path.join(HERE, "art", "lp-infinity.png")
HEADER = os.path.join(HERE, "lp-infinity-png.h")
LP_W, LP_H = 160, 64
# The sign inside: as tall as the LP digits (14 screen pixels, 56 of the picture's), about two and a half digits wide.
SIGN_W, SIGN_H = 116, 56


def png(width, height, rgba_rows):
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    raw = b"".join(b"\0" + bytes(row) for row in rgba_rows)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def size(data):
    """(width, height, colour type) of a PNG, or ValueError."""
    if data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError("not a PNG")
    width, height, depth, kind = struct.unpack(">IIBB", data[16:26])
    return width, height, kind


def placeholder(width=LP_W, height=LP_H):
    """A plain generated "∞" (a lemniscate): white, a dark outline, transparent around. Not the final art."""
    a, cx, cy = (SIGN_W - 12) / 2, width / 2, height / 2
    points = []
    for i in range(720):
        t = i * 2 * math.pi / 720
        d = 1 + math.sin(t) ** 2
        points.append((cx + a * math.cos(t) / d, cy + a * math.sin(t) * math.cos(t) / d * 1.25))
    stroke, outline = SIGN_H * 0.12, SIGN_H * 0.2
    rows = []
    for y in range(height):
        row = []
        for x in range(width):
            dist = min(math.hypot(x + 0.5 - px, y + 0.5 - py) for px, py in points)
            if dist <= stroke:
                row += (255, 255, 255, 255)
            elif dist <= outline:
                row += (24, 24, 24, 255)
            elif dist <= outline + 1:
                row += (24, 24, 24, int(255 * (outline + 1 - dist)))
            else:
                row += (0, 0, 0, 0)
        rows.append(row)
    return png(width, height, rows)


def header(data):
    width, height, kind = size(data)
    if kind != 6:
        raise ValueError("the picture needs an alpha channel (PNG colour type 6, RGBA)")
    lines = ["/* Made by tools/mod/make-lp-infinity.py from tools/mod/art/lp-infinity.png: do not edit. */",
             "#ifndef YO3_LP_INFINITY_PNG_H", "#define YO3_LP_INFINITY_PNG_H",
             "#define YO3_LP_INFINITY_W %d" % width, "#define YO3_LP_INFINITY_H %d" % height,
             "static const unsigned char yo3_lp_infinity_png[%d] = {" % len(data)]
    for i in range(0, len(data), 24):
        lines.append("    " + ",".join("0x%02x" % b for b in data[i:i + 24]) + ",")
    lines += ["};", "#endif", ""]
    return "\n".join(lines)


def main(args):
    if "--placeholder" in args:
        os.makedirs(os.path.dirname(PICTURE), exist_ok=True)
        with open(PICTURE, "wb") as f:
            f.write(placeholder())
    with open(PICTURE, "rb") as f:
        data = f.read()
    text = header(data)
    if "--check" in args:
        current = open(HEADER).read() if os.path.exists(HEADER) else ""
        if current != text:
            print("lp-infinity-png.h is not made from art/lp-infinity.png: run tools/mod/make-lp-infinity.py", file=sys.stderr)
            return 1
        return 0
    with open(HEADER, "w") as f:
        f.write(text)
    width, height, _ = size(data)
    print("lp-infinity-png.h: %dx%d, %d bytes" % (width, height, len(data)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
