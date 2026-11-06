"""Summarize a yo3trace field log: which fields the client read for each reply, and which were missing.

    python3 tools/tracer/summarize.py [logs/fields.tsv] [--missing] [--reply 0x2398]

Field lookups are attributed to the last `request` value read on the same thread within
WINDOW_MS (the client reads `request` in every dispatcher before reading a message's fields).
Callers are shown with their containing function when re/yo3_decompiled.c exists.

Only reads through the typed getters (int/uint/string/raw) are shown by default. Direct
`find` calls come from the setters (0x408640 set-int, 0x4084A0 set-value) while the client
builds its own messages; show them with --all.
"""
import argparse
import bisect
import collections
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
WINDOW_MS = 300


def function_index():
    path = ROOT / "re" / "yo3_decompiled.c"
    if not path.exists():
        return [], []
    starts, names = [], []
    with path.open(errors="replace") as f:
        for line in f:
            m = re.match(r"// ===== (\S+) @ ([0-9a-f]{8})", line)
            if m:
                starts.append(int(m.group(2), 16))
                names.append(m.group(1))
    order = sorted(range(len(starts)), key=starts.__getitem__)
    return [starts[i] for i in order], [names[i] for i in order]


STARTS, NAMES = function_index()


def where(addr):
    if not addr or not STARTS:
        return f"{addr:#x}"
    i = bisect.bisect_right(STARTS, addr) - 1
    return f"{addr:#x} ({NAMES[i]}+{addr - STARTS[i]:#x})" if i >= 0 else f"{addr:#x}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("log", nargs="?", default=str(ROOT / "logs" / "fields.tsv"))
    ap.add_argument("--missing", action="store_true", help="only show lookups that found nothing")
    ap.add_argument("--reply", help="only this reply id, e.g. 0x2398")
    ap.add_argument("--all", action="store_true", help="include direct find calls (setters)")
    args = ap.parse_args()

    context = {}  # thread -> (reply id, time of the request read)
    fields = collections.defaultdict(collections.Counter)  # reply -> (key, getter, found, type, caller) -> n
    items = collections.defaultdict(collections.Counter)  # reply -> (index, count, caller) -> n
    header = None
    for line in open(args.log, errors="replace"):
        cols = line.rstrip("\n").split("\t")
        if cols[0] == "#":
            header = " ".join(cols[1:])
            continue
        if cols[0] == "F" and len(cols) >= 10:
            _, ms, tid, getter, key, found, vtype, req, caller, _ = cols[:10]
            tree = cols[10] if len(cols) > 10 else None
            ms = int(ms)
            if key == "request" and found == "1":
                context[tid] = (req, ms, tree)
                continue
            if getter == "find" and not args.all:
                continue
            reply, at, msg_tree = context.get(tid, ("-", -10**9, None))
            reply = reply if ms - at <= WINDOW_MS else "-"
            # Same dataset as the message's "request" field = a top-level field of that message.
            level = "?" if tree is None or msg_tree is None else ("top" if tree == msg_tree else "sub")
            fields[reply][(key, getter, found == "1", vtype, int(caller, 16), level)] += 1
        elif cols[0] == "I" and len(cols) >= 6:
            _, ms, tid, index, count, caller = cols[:6]
            reply, at, _ = context.get(tid, ("-", -10**9, None))
            reply = reply if int(ms) - at <= WINDOW_MS else "-"
            items[reply][(int(index), int(count), int(caller, 16))] += 1

    if header:
        print(f"# {header}")
    want = args.reply and hex(int(args.reply, 16))
    for reply in sorted(set(fields) | set(items), key=lambda r: (r == "-", r)):
        if want and reply != want:
            continue
        rows = [(k, n) for k, n in fields[reply].items() if not args.missing or not k[2]]
        bad_items = [(k, n) for k, n in items[reply].items() if k[0] >= k[1]]
        if not rows and not (bad_items if args.missing else items[reply]):
            continue
        title = "outside any reply" if reply == "-" else f"reply {reply}"
        print(f"\n== {title}")
        for (key, getter, found, vtype, caller, level), n in sorted(rows, key=lambda r: (r[0][2], r[0][0])):
            status = "found  " if found else "MISSING"
            print(f"  {status} {level:3} {key:24} {getter:6} {vtype if found else '':28} x{n:<4} {where(caller)}")
        for (index, count, caller), n in sorted(items[reply].items()):
            if args.missing and index < count:
                continue
            status = "item   " if index < count else "NO ITEM"
            print(f"  {status}     [{index}] of {count:<17} {'':6} {'':28} x{n:<4} {where(caller)}")


if __name__ == "__main__":
    sys.exit(main())
