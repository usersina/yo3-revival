#!/usr/bin/env python3
"""Makes placeholder card pictures for checking the anime artwork folder (docs/anime-cards.md): a plain, generated PNG
per card id, never a game or anime image. Usage: make-placeholder-art.py FOLDER CID [CID...] [--mini]
FOLDER/card/<cid>.png is 200x290 (the full face); with --mini FOLDER/mini/<cid>.png is 50x72 as well."""
import os
import struct
import sys
import zlib


def png(width, height, rgb):
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    rows = b""
    for y in range(height):
        band = (y // 10) % 2
        row = b"".join(bytes(rgb if (x // 10 + band) % 2 else (255, 255, 255)) for x in range(width))
        rows += b"\0" + row
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b""))


def main(args):
    mini = "--mini" in args
    args = [a for a in args if a != "--mini"]
    folder, cids = args[0], [int(c) for c in args[1:]]
    for cid in cids:
        rgb = ((cid * 53) % 200, (cid * 97) % 200, (cid * 193) % 200 + 40)
        for sub, size in (("card", (200, 290)), ("mini", (50, 72))):
            if sub == "mini" and not mini:
                continue
            os.makedirs(os.path.join(folder, sub), exist_ok=True)
            with open(os.path.join(folder, sub, "%d.png" % cid), "wb") as f:
                f.write(png(size[0], size[1], rgb))


if __name__ == "__main__":
    main(sys.argv[1:])
