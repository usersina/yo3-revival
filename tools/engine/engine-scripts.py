#!/usr/bin/env python3
"""The card scripts a release ships (tools/engine/engine-files.sh): only those the engine can ever load, out of the
~19,300 ProjectIgnis has. A duel's cards are the game's (every row of engine/anime/cards.tsv, by its official passcode,
and every card of the client's era, Konami ids below 10,000 in the pinned ids, as tests/engine_cards.rs loads them),
the cards the game never had (engine/added-cards.json) and their anime versions (engine/anime-cards.json); a script
can bring in others (a token, a card it summons or names, a card it loads), so each kept script's numbers that name a
script (`12345678`, `c12345678.lua`, the card's own `id+1`..`id+3`) are followed in turn, as are database aliases.
The root scripts (utility.lua, the procedures) always ship, and so does every script in the folders that is no card's
(unofficial/proc_unofficial.lua, the anime cards' shared procedures, and rankup_functions.lua).

    tools/engine/engine-scripts.py [--root REPO]      the kept scripts, one path per line relative to CardScripts/
    tools/engine/engine-scripts.py --stats             how many and how large, against all of them
"""
import argparse
import json
import re
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FOLDERS = ("official", "pre-errata", "pre-release", "unofficial")
_NUMBER = re.compile(r"(?<![\w.])(\d{4,9})(?![\w.])")
_FILE = re.compile(r"c(\d{1,9})\.lua")
_OFFSET = re.compile(r"\bid\s*\+\s*([1-9])\b")


def wanted(root=ROOT):
    """The passcodes a duel can hold: the game's cards, the added cards and every anime version."""
    out = set()
    table = root / "engine/anime/cards.tsv"
    for line in (table.read_text(encoding="utf-8").splitlines()[1:] if table.is_file() else []):
        for field in line.split("\t")[2:4]:
            if field.strip().isdigit():
                out.add(int(field))
    info = root / "engine/vendor/cardinfo.json"
    if info.is_file():
        for card in json.loads(info.read_text(encoding="utf-8")).get("data", []):
            konami = [m.get("konami_id") for m in card.get("misc_info", [])]
            if card.get("id", 0) < 100_000_000 and any(k and 4000 <= k < 10000 for k in konami):
                out.add(card["id"])
    for card in json.loads((root / "engine/added-cards.json").read_text(encoding="utf-8")).get("cards", []):
        out.update(c for c in (card.get("passcode"), card.get("anime")) if c)
    for card in json.loads((root / "engine/anime-cards.json").read_text(encoding="utf-8")).get("cards", []):
        out.update(card[k] for k in ("anime", "official", "alias") if card.get(k))
    return out


def scripts(vendor):
    """{passcode: [paths relative to CardScripts]} of every card script."""
    out = {}
    for folder in FOLDERS:
        for path in sorted((vendor / "CardScripts" / folder).glob("c*.lua")):
            if path.stem[1:].isdigit():
                out.setdefault(int(path.stem[1:]), []).append(f"{folder}/{path.name}")
    return out


def aliases(vendor):
    out = {}
    for cdb in ("cards.cdb", "cards-unofficial.cdb"):
        path = vendor / "BabelCDB" / cdb
        if path.is_file():
            try:
                with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as db:
                    out.update((code, alias) for code, alias in db.execute("SELECT id, alias FROM datas WHERE alias != 0"))
            except sqlite3.DatabaseError:  # not a card database (a test's fixture): no aliases to follow
                continue
    return out


def closure(vendor, start):
    files, alias = scripts(vendor), aliases(vendor)
    # The helpers with short numbers (c419.lua...c483.lua, loaded by name by many anime scripts) always ship.
    seen, todo = set(), [c for c in start] + [c for c in files if c < 1000]
    while todo:
        code = todo.pop()
        if code in seen:
            continue
        seen.add(code)
        if code in alias:
            todo.append(alias[code])
        for rel in files.get(code, []):
            text = (vendor / "CardScripts" / rel).read_text(encoding="utf-8", errors="replace")
            found = {int(n) for n in _NUMBER.findall(text)} | {int(n) for n in _FILE.findall(text)}
            found |= {code + int(n) for n in _OFFSET.findall(text)}
            todo.extend(n for n in found if n in files and n not in seen)
    shared = [f"{folder}/{p.name}" for folder in FOLDERS for p in sorted((vendor / "CardScripts" / folder).glob("*.lua"))
              if not (p.stem.startswith("c") and p.stem[1:].isdigit())]
    return sorted(shared + [rel for code in seen for rel in files.get(code, [])])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--root", type=Path, default=ROOT, help="the repository (engine/ in it); default: this one")
    p.add_argument("--stats", action="store_true")
    args = p.parse_args(argv)
    args.vendor = args.root / "engine" / "vendor"
    if not (args.vendor / "CardScripts").is_dir():
        sys.exit("engine-scripts.py: no card scripts (engine/fetch.sh)")
    kept = closure(args.vendor, wanted(args.root))
    if args.stats:
        every = [rel for rels in scripts(args.vendor).values() for rel in rels]
        size = lambda rels: sum((args.vendor / "CardScripts" / r).stat().st_size for r in rels) / 1e6
        print(f"{len(kept)} of {len(every)} scripts, {size(kept):.1f} MB of {size(every):.1f} MB")
        return 0
    print("\n".join(kept))
    return 0


if __name__ == "__main__":
    sys.exit(main())
