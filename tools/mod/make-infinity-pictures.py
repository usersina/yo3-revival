#!/usr/bin/env python3
"""The duel's "∞" pictures (tools/mod/duel-numbers.h): the connection DLL carries them inside itself, as the C header
infinity-pictures.h, and shows them where the game's own digits have no ∞:

  lp-infinity.png            the LP disc, for ∞ LP (in place of the LP counter's five digits)
  damage-infinity-loss.png   the damage pop-up of an ∞ loss ("-∞", red as the game's loss digits)
  damage-infinity-gain.png   the damage pop-up of an ∞ gain ("∞", cyan as the game's gain digits)

Usage:
  make-infinity-pictures.py           the header from the PNGs in tools/mod/art/ (after changing one)
  make-infinity-pictures.py --check   exit 1 when the header is not made from them

Each picture: a PNG with an alpha channel (colour type 6, 8 bits) on a transparent background, drawn at four times the
screen's size (the DLL shows it at 25%, its middle on the middle of the digits it replaces). Each has its source SVG beside
it (tools/mod/art/README.md: the specs and how to render them)."""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]):
    print(__doc__); raise SystemExit(0)
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ART = os.path.join(HERE, "art")
HEADER = os.path.join(HERE, "infinity-pictures.h")
# (file in art/, the C name of its bytes, of its width and of its height)
PICTURES = [
    ("lp-infinity.png", "yo3_lp_infinity_png", "YO3_LP_INFINITY_W", "YO3_LP_INFINITY_H"),
    ("damage-infinity-loss.png", "yo3_damage_loss_png", "YO3_DAMAGE_LOSS_W", "YO3_DAMAGE_LOSS_H"),
    ("damage-infinity-gain.png", "yo3_damage_gain_png", "YO3_DAMAGE_GAIN_W", "YO3_DAMAGE_GAIN_H"),
]


def size(data):
    """(width, height, colour type) of a PNG, or ValueError."""
    if data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError("not a PNG")
    width, height, depth, kind = struct.unpack(">IIBB", data[16:26])
    return width, height, kind


def header(pictures):
    """The header for `pictures`: [(file name, bytes)] in PICTURES' order."""
    lines = ["/* Made by tools/mod/make-infinity-pictures.py from the PNGs in tools/mod/art/: do not edit. */",
             "#ifndef YO3_INFINITY_PICTURES_H", "#define YO3_INFINITY_PICTURES_H"]
    for (name, data), (_, array, w, h) in zip(pictures, PICTURES):
        width, height, kind = size(data)
        if kind != 6:
            raise ValueError(f"{name} needs an alpha channel (PNG colour type 6, RGBA)")
        lines += [f"/* {name} */", f"#define {w} {width}", f"#define {h} {height}",
                  f"static const unsigned char {array}[{len(data)}] = {{"]
        for i in range(0, len(data), 24):
            lines.append("    " + ",".join("0x%02x" % b for b in data[i:i + 24]) + ",")
        lines.append("};")
    lines += ["#endif", ""]
    return "\n".join(lines)


def read():
    out = []
    for name, *_ in PICTURES:
        with open(os.path.join(ART, name), "rb") as f:
            out.append((name, f.read()))
    return out


def main(args):
    text = header(read())
    if "--check" in args:
        current = open(HEADER).read() if os.path.exists(HEADER) else ""
        if current != text:
            print("infinity-pictures.h is not made from tools/mod/art/: run tools/mod/make-infinity-pictures.py", file=sys.stderr)
            return 1
        return 0
    with open(HEADER, "w") as f:
        f.write(text)
    for name, data in read():
        width, height, _ = size(data)
        print(f"{name}: {width}x{height}, {len(data)} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
