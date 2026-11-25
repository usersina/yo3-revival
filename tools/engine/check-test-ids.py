#!/usr/bin/env python3
"""Every card id a scenario test file names (`const NAME: u16 = N;`) is one a player can hold: a card of the game or an
added card (engine/added-cards.json, by the id the client knows: the stand-in for a newer card, not its Konami id).

An id the game lacks still runs in the rules engine, so a test using it passes while testing nothing a player can meet.

Usage:
  check-test-ids.py crates/yo3-server/tests/anime_printed_*.rs   prints each id that is neither; exit 1 if any

Needs the pinned data (engine/fetch.sh) and the game (YO3_GAME_DIR, else the default Wine path)."""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]) or len(_sys.argv) < 2:
    print(__doc__); raise SystemExit(0)
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def main(paths):
    spec = importlib.util.spec_from_file_location("added_cards", os.path.join(HERE, "added-cards.py"))
    added = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(added)
    game = set(added.load()["in_game"])
    with open(os.path.join(ROOT, "engine", "added-cards.json")) as f:
        extra = {c["cid"] for c in json.load(f)["cards"]}
    bad = 0
    for path in paths:
        with open(path) as f:
            for name, n in re.findall(r"const (\w+): u16 = (\d+);", f.read()):
                if int(n) not in game and int(n) not in extra:
                    print(f"{path}: {name} = {n} is no card of the game and no added card")
                    bad += 1
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
