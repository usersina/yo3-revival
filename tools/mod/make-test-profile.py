#!/usr/bin/env python3
"""Writes a hand-made card profile (docs/client-internals.md "Card profiles") for checking the connection DLL's card
layer: Flame Swordsman as a Normal Monster (Main Deck), Mystical Elf as an Effect Monster with 1 ATK and "?" DEF, and Negate
Attack as a Quick-Play Spell, each with a test text and art. Reads the game's CARD_Prop.bin (a .ypk extract: --bin DIR
holds CARD_Prop.bin, CARD_Indx_E.bin, CARD_Name_E.bin; `tools/re/ypk.py x` makes it). Usage: make-test-profile.py BIN_DIR OUT"""
import struct
import sys


def main(bin_dir, out):
    prop = open(bin_dir + "/CARD_Prop.bin", "rb").read()
    names = {}
    index = open(bin_dir + "/CARD_Indx_E.bin", "rb").read()
    name_table = open(bin_dir + "/CARD_Name_E.bin", "rb").read()
    for row in range(len(prop) // 8):
        offset = struct.unpack("<I", index[row * 8:row * 8 + 4])[0]
        end = offset
        while name_table[end:end + 2] != b"\0\0" or (end - offset) % 2:
            end += 1
        names.setdefault(name_table[offset:end].decode("utf-16le"), []).append(row)

    def rewrite(b, kind, attribute, level, subtype, race):
        mask = (0x1F << 3) | (0xF << 8) | (0xF << 12) | (7 << 16) | (0x1F << 19)
        return (b & ~mask) | kind << 3 | attribute << 8 | level << 12 | subtype << 16 | race << 19

    lines = ["yo3-card-profile 1", "profile test", "hash 0000000000000000"]
    arts = []
    specs = [("Flame Swordsman", dict(kind=0), "-", "-", "Test text: Flame Swordsman, a Normal Monster."),
             ("Mystical Elf", dict(kind=1), "1", "?", "Test text: Mystical Elf has an effect and 1 ATK."),
             ("Negate Attack", dict(kind=13, attribute=8, level=0, subtype=5, race=23), "-", "-", "Test text: Negate Attack is a Quick-Play Spell.")]
    for name, change, atk, defense, text in specs:
        for row in names[name]:
            a, b = struct.unpack("<II", prop[row * 8:row * 8 + 8])
            fields = dict(kind=b >> 3 & 31, attribute=b >> 8 & 15, level=b >> 12 & 15, subtype=b >> 16 & 7, race=b >> 19 & 31)
            fields.update(change)
            lines.append("card %d %d %x %x 0 %s %s" % (row, a & 0x3FFF, a, rewrite(b, **fields), atk, defense))
        lines.append('text %d "%s"' % (names[name][0], text))
        arts.append(a & 0x3FFF)
    lines += ["art %d" % cid for cid in arts]
    open(out, "w", newline="\n").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
