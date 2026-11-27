#!/usr/bin/env python3
"""The anime-only cards still to add (#75), in batches by the deck that used them in the anime: the work list of phase 2
(docs/added-cards.md, "Add a batch").

Usage:
  anime-batches.py --build INVENTORY.csv   write engine/added/anime-batches.tsv from the anime inventory's
                                           missing-unofficial_database.csv (2026-11-25, logs/anime-missing-20261125/):
                                           every card tools/engine/added-cards.py --find accepts, or would with its Tokens
  anime-batches.py --batch N               batch N as a --find-file / --add-file list: its Tokens first, then its cards
  anime-batches.py --list                  the batches: number, series, decks, cards, how many are added already

The table (engine/added/anime-batches.tsv): batch, series (DM, GX, 5Ds), deck (the first deck the inventory names), the
anime passcode, the card's name, and the Tokens it makes that the client has no card for (token passcodes, comma). A
deck of more than 60 cards is split; smaller ones are merged, in their series' order, into batches of 30 to 60. Rows and
stand-ins are given when a batch is added, not here: this list can be rebuilt until then."""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]) or len(_sys.argv) < 2:
    print(__doc__); raise SystemExit(0)
import csv
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
TABLE = os.path.join(ROOT, "engine", "added", "anime-batches.tsv")
SERIES = ["DM", "GX", "5Ds"]
SMALLEST, LARGEST = 30, 60


def added_cards():
    spec = importlib.util.spec_from_file_location("added_cards", os.path.join(HERE, "added-cards.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def build(inventory):
    added = added_cards()
    ctx = added.load()
    with open(inventory, encoding="utf-8-sig") as f:
        rows = list(csv.DictReader(f))
    names = ["anime:" + r["unofficial_ids"].split(" | ")[0] for r in rows]
    found = added.find(names, ctx)
    cards = []
    for r, (name, line, why) in zip(rows, found):
        token_only = why and all("is no card the client has" in w for w in why)
        if not line and not token_only:
            continue
        code = int(r["unofficial_ids"].split(" | ")[0])
        tokens = sorted({int(t) for w in why for t in re.findall(r"token:(\d+)", w)})
        series = next((s for s in (r.get("series") or "").split(" | ") if s in SERIES), "DM")
        deck = ((r.get("decks") or "").split(" | ")[0] or "(no deck)").strip()
        cards.append({"series": series, "deck": deck, "anime": code, "name": name, "tokens": tokens})
    # Decks in their series' order, the largest first; a deck past LARGEST split, small ones merged up to SMALLEST.
    batches, current = [], []
    for series in SERIES:
        decks = {}
        for c in cards:
            if c["series"] == series:
                decks.setdefault(c["deck"], []).append(c)
        for deck in sorted(decks, key=lambda d: (-len(decks[d]), d)):
            group = decks[deck]
            while len(group) > LARGEST:
                batches.append(group[:LARGEST // 2 + 15])
                group = group[LARGEST // 2 + 15:]
            if current and len(current) + len(group) > LARGEST:
                batches.append(current)
                current = []
            current = current + group
            if len(current) >= SMALLEST:
                batches.append(current)
                current = []
        if current:
            batches.append(current)
            current = []
    with open(TABLE, "w", encoding="utf-8") as f:
        f.write("batch\tseries\tdeck\tanime\tname\ttokens\n")
        for n, batch in enumerate(batches, 1):
            for c in batch:
                f.write(f"{n}\t{c['series']}\t{c['deck']}\t{c['anime']}\t{c['name']}\t{','.join(map(str, c['tokens']))}\n")
    print(f"wrote {TABLE}: {len(cards)} cards in {len(batches)} batches")
    return 0


def table():
    with open(TABLE, encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def main(args):
    if args[0] == "--build":
        return build(args[1])
    rows = table()
    if args[0] == "--batch":
        batch = [r for r in rows if r["batch"] == args[1]]
        tokens = sorted({t for r in batch for t in r["tokens"].split(",") if t})
        # A Token the inventory also lists as a card of the deck (Stone Giant Token) is added once, as the Token.
        print("\n".join([f"token:{t}" for t in tokens] + [f"anime:{r['anime']}" for r in batch if r["anime"] not in tokens]))
        return 0
    if args[0] == "--list":
        with open(os.path.join(ROOT, "engine", "added", "cards.tsv"), encoding="utf-8") as f:
            have = {line.split("\t")[4] for line in f.read().splitlines()[1:]}
        for n in sorted({int(r["batch"]) for r in rows}):
            batch = [r for r in rows if r["batch"] == str(n)]
            decks = sorted({r["deck"] for r in batch}, key=lambda d: -sum(r["deck"] == d for r in batch))
            done = sum(r["anime"] in have for r in batch)
            print(f"{n:2}  {batch[0]['series']:3}  {len(batch):2} cards  added {done:2}  {', '.join(decks)[:90]}")
        return 0
    print(__doc__)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
