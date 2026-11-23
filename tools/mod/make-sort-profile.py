#!/usr/bin/env python3
"""Writes a card profile for checking the deck editor's sort and range search (issue #60, docs/client-internals.md "Sorting
and range search"): every Monster of the game gets one of the stats the anime has (0, 1, 9, 10, 2500, 2501, 5100, 6000, "?"),
by its order, with the DEF shifted so ATK and DEF differ. Rows keep their own type words and genre; only ATK/DEF change.
Reads a .ypk extract (--bin DIR holds CARD_Prop.bin, CARD_Genre.bin, CARD_Indx_E.bin, CARD_Name_E.bin: `tools/re/ypk.py x`).
Usage: make-sort-profile.py BIN_DIR OUT [--table]   (--table also prints name, ATK, DEF per card, to compare with the screen)"""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]):
    print(__doc__); raise SystemExit(0)
import struct
import sys

STATS = [0, 1, 9, 10, 2500, 2501, 5100, 6000, "?"]


def field(value):
    return 0x1FF if value == "?" else min(value // 10, 0x1FE)


def main(bin_dir, out, table):
    prop = open(bin_dir + "/CARD_Prop.bin", "rb").read()
    genre = open(bin_dir + "/CARD_Genre.bin", "rb").read()
    index = open(bin_dir + "/CARD_Indx_E.bin", "rb").read()
    names = open(bin_dir + "/CARD_Name_E.bin", "rb").read()

    def name(row):
        start = struct.unpack("<I", index[row * 8:row * 8 + 4])[0]
        end = start
        while names[end:end + 2] != b"\0\0" or (end - start) % 2:
            end += 1
        return names[start:end].decode("utf-16le")

    lines = ["yo3-card-profile 1", "profile sorting-check", "hash 0000000000000000"]
    chosen = {}
    for row in range(len(prop) // 8):
        a, b = struct.unpack("<II", prop[row * 8:row * 8 + 8])
        cid = a & 0x3FFF
        kind = b >> 3 & 31
        if not cid or kind in (13, 14) or (a >> 14 & 0x1FF) == 0 and (a >> 23 & 0x1FF) == 0:
            continue
        if cid not in chosen:
            n = len(chosen)
            chosen[cid] = (STATS[n % len(STATS)], STATS[(n + 4) % len(STATS)])
        atk, defense = chosen[cid]
        a = a & ~(0x1FF << 14 | 0x1FF << 23) | field(atk) << 14 | field(defense) << 23
        g_lo, g_hi = struct.unpack("<II", genre[row * 8:row * 8 + 8])
        lines.append("card %d %d %x %x %x %s %s" % (row, cid, a, b, g_hi << 32 | g_lo, atk, defense))
        if table:
            print("%-40s %5s %5s" % (name(row), atk, defense))
    open(out, "w", newline="\n").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    args = [x for x in sys.argv[1:] if x != "--table"]
    main(args[0], args[1], "--table" in sys.argv)
