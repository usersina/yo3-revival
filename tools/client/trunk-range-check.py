#!/usr/bin/env python3
"""Checks Edit Deck's ATK/DEF range search and sort on a headless client against what the list itself shows (issue #60,
docs/client-internals.md "Sorting and range search"). With the trunk open, per stat (ATK, DEF) and per pair of bounds it:
presses Reset, opens the stat's dialog, types the bounds (a blank one stays blank), presses Sort Up (or Sort Down) and reads
the list with trunk-scan.py. The expected list comes from the first run (both bounds blank: every Monster the search keeps,
with its ATK and DEF as the game shows them, "?" counting as 0 in a range): a Monster is kept when lo <= stat <= hi, a
blank lo is 0 and a blank hi is --blank-max (5000 in the game's own search; --blank-max open when the profile has a stat
above that, the connection DLL then opens it). The sort must be ascending, or descending with "?" last, by the shown
number. Exit status 1 on a mismatch.

    YO3_INSTANCE=b tools/client/trunk-range-check.py [--blank-max 5000|open] [--down] [--shots NAME] [PAIR ...]

PAIR is LO:HI (either may be empty), default: the boundaries 0, 1, 9, 10, 2500, 2501, 5100, 6000."""
import argparse
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
VCLIENT = os.path.join(HERE, "vclient.sh")
SCAN = os.path.join(HERE, "trunk-scan.py")
ICON = {"atk": (757, 88), "def": (796, 88)}
MIN_FIELD, MAX_FIELD, SORT_UP, SORT_DOWN, RESET = (517, 106), (645, 106), (577, 146), (577, 176), (1000, 88)
DEFAULT_PAIRS = ["0:0", "1:1", "9:9", "10:10", "2500:2500", "2501:2501", "5100:5100", "6000:6000", "0:1", "1:9", "9:10", "10:2500",
                 "2500:2501", "2501:5100", "5100:6000", "5101:6000", "6001:9999", "1:", ":0", ":9", ":5000", "5000:", "6000:", "6001:"]
OPEN = 1 << 30


def run(*args):
    return subprocess.run([VCLIENT, *args], capture_output=True, text=True, check=True).stdout


def type_field(point, text):
    run("click", *map(str, point))
    run("key", "End", *["BackSpace"] * 6)
    if text:
        run("type", text)


def search(stat, lo, hi, down):
    run("click", *map(str, RESET)); time.sleep(2.5)
    run("click", *map(str, ICON[stat])); time.sleep(1.2)
    type_field(MIN_FIELD, lo); type_field(MAX_FIELD, hi)
    run("click", *map(str, SORT_DOWN if down else SORT_UP)); time.sleep(2)
    path = f"/tmp/trunk-range-{os.getpid()}.json"
    subprocess.run([sys.executable, SCAN, "--json", path], capture_output=True, text=True, check=True)
    cards = json.load(open(path)); os.unlink(path)
    return cards


def shown(v):
    return 0 if v == 65535 else v           # "?" counts as 0 in a range


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--blank-max", default="5000")
    ap.add_argument("--down", action="store_true")
    ap.add_argument("--shots")
    ap.add_argument("pairs", nargs="*")
    args = ap.parse_args()
    blank_max = OPEN if args.blank_max == "open" else int(args.blank_max)
    failures = 0
    for stat in ("atk", "def"):
        base = search(stat, "", "", False)
        every = {c["row"]: c for c in base}
        print(f"{stat}: {len(every)} Monsters kept with both bounds blank")
        for pair in args.pairs or DEFAULT_PAIRS:
            lo_text, hi_text = pair.split(":")
            lo = int(lo_text) if lo_text else 0
            hi = int(hi_text) if hi_text else blank_max
            want = sorted(r for r, c in every.items() if lo <= shown(c[stat]) <= hi)
            got = search(stat, lo_text, hi_text, args.down)
            rows = [c["row"] for c in got]
            keys = [-1 if c[stat] == 65535 else c[stat] for c in got]       # "?" is the lowest going up, the last going down
            ordered = keys == sorted(keys, reverse=args.down)
            ok = sorted(rows) == want and ordered
            failures += not ok
            print(f"{'ok  ' if ok else 'FAIL'} {stat} [{lo_text or '-'}, {hi_text or '-'}] {'down' if args.down else 'up'}: "
                  f"{len(rows)} cards, expected {len(want)}" + ("" if ok else f" missing={sorted(set(want) - set(rows))} extra={sorted(set(rows) - set(want))} ordered={ordered}"))
            if args.shots and pair in ("2501:2501", "6000:6000", "1:1"):
                run("shot", f"{args.shots}-{stat}-{pair.replace(':', '-')}")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
