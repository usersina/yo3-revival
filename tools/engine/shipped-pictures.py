#!/usr/bin/env python3
"""The card pictures the project ships (the owner's own artwork, `crates/yo3-launcher/assets/pics/`): each original,
named by its passcode as EDOPro names pictures, scaled to the game's 200x290 card face. The launcher uses them for the
cards shown as their anime or manga versions and the cards the game never had, unless the player has a picture of their
own (`crates/yo3-launcher/src/art/pics.rs`, docs/anime-cards.md "Artwork").

    tools/engine/shipped-pictures.py SOURCE_DIR      write the faces from the originals in SOURCE_DIR (needs Pillow)
    tools/engine/shipped-pictures.py --check         every listed face is there, a 200x290 PNG, and nothing else is

Only the scaled faces are committed; the originals stay with their author. To ship another picture: add its passcode to
SHIPPED here and to `SHIPPED` in pics.rs, then run this with the originals' folder.
"""
import argparse
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "crates" / "yo3-launcher" / "assets" / "pics"
FACE = (200, 290)
# passcode: card (all by the project's owner)
SHIPPED = {
    513000134: "The Winged Dragon of Ra (manga)",
    513000135: "Obelisk the Tormentor (manga)",
    513000136: "Slifer the Sky Dragon (manga)",
    513000137: "The Wicked Dreadroot (manga)",
    513000138: "The Wicked Avatar (manga)",
    513000139: "The Wicked Eraser (manga)",
    170000170: "Divine Serpent Geh (anime)",
}


def png_size(path):
    head = path.read_bytes()[:24]
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", head[16:24])


def problems(assets=ASSETS):
    out = []
    names = {p.name for p in assets.glob("*")} if assets.is_dir() else set()
    for code in SHIPPED:
        path = assets / f"{code}.png"
        if not path.is_file():
            out.append(f"{path.name}: missing (run this with the originals' folder)")
        elif png_size(path) != FACE:
            out.append(f"{path.name}: not a {FACE[0]}x{FACE[1]} PNG")
    out += [f"{name}: not listed in SHIPPED" for name in sorted(names - {f"{c}.png" for c in SHIPPED})]
    return out


def write(source):
    from PIL import Image
    ASSETS.mkdir(parents=True, exist_ok=True)
    for code, name in SHIPPED.items():
        original = next((source / f"{code}{ext}" for ext in (".png", ".jpg", ".jpeg") if (source / f"{code}{ext}").is_file()), None)
        if original is None:
            sys.exit(f"shipped-pictures.py: no {code}.png or .jpg in {source} ({name})")
        with Image.open(original) as image:
            face = image.convert("RGB").resize(FACE, Image.LANCZOS)
        face.save(ASSETS / f"{code}.png", optimize=True)
        print(f"{code}.png  {name}  from {original.name}")


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("source", nargs="?", type=Path, help="the folder of the originals, named <passcode>.png or .jpg")
    p.add_argument("--check", action="store_true", help="check the committed faces instead")
    args = p.parse_args(argv)
    if args.check or not args.source:
        found = problems()
        for line in found:
            print(line, file=sys.stderr)
        return 1 if found else 0
    write(args.source)
    return 0


if __name__ == "__main__":
    sys.exit(main())
