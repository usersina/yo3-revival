#!/usr/bin/env python3
"""A duel as the rules engine holds it, by card name: each side's LP, hand, field and Graveyard, from a server's log.

The connection DLL's probe (`vclient.sh ui`) sees the game's field slots but not which card is which, and every added
card shows the same "Artwork not found" picture until its own arrives; the server, with the duel check on (`--duel-check`,
as `task dev:server` and the monkeys start it), logs every engine state. This reads the newest one.

    tools/client/duel-state.py [--server SERVER.log] [--duel N] [--json]
    tools/client/vclient.sh duel                       the same for the scratch server of $YO3_SCRATCH

SERVER.log defaults to $YO3_SCRATCH/server.log (`scratch-server.py env`); --duel to the newest duel in it. Team 0 went
first. Names come from the pinned card data (engine/vendor) and engine/added-cards.json.
"""
import argparse
import glob
import importlib.util
import json
import os
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_SPEC = importlib.util.spec_from_file_location("yo3log", ROOT / "tools" / "logs" / "yo3log.py")
yo3log = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(yo3log)
MARKER = "[YO3 ENGINE] "


def records(lines):
    """The engine's logged chunks ({call, direction, duel, part, parts, data}) in the lines (yo3-log/1 or older text)."""
    out = []
    for line in lines:
        text = yo3log.text(line.rstrip("\n"))
        if MARKER in text:
            try:
                out.append(json.JSONDecoder().raw_decode(text.split(MARKER, 1)[1])[0])
            except ValueError:
                continue
    return out


def newest_state(chunks, duel=None):
    """(duel, the newest complete answer that holds the teams), or None."""
    duels = sorted({c.get("duel") for c in chunks if isinstance(c.get("duel"), int)})
    if not duels:
        return None
    duel = duel if duel is not None else duels[-1]
    groups = {}
    for c in chunks:
        if c.get("duel") == duel and c.get("direction") == "answer" and c.get("encoding") == "hex":
            groups.setdefault(c["call"], {})[c["part"]] = c
    for call in sorted(groups, reverse=True):
        parts = groups[call]
        count = next(iter(parts.values()))["parts"]
        if set(parts) != set(range(count)):
            continue
        try:
            answer = json.loads(bytes.fromhex("".join("".join(parts[p]["data"]) for p in range(count))))
        except ValueError:
            continue
        if isinstance(answer, dict) and isinstance(answer.get("teams"), list):
            return duel, answer
    return None


def names(root=ROOT):
    """{passcode: English name} from the pinned card databases."""
    out = {}
    for cdb in sorted(glob.glob(str(root / "engine" / "vendor" / "BabelCDB" / "*.cdb")), key=lambda p: not p.endswith("/cards.cdb")):
        try:
            with sqlite3.connect(f"file:{cdb}?mode=ro", uri=True) as db:
                for code, name in db.execute("SELECT id, name FROM texts"):
                    out.setdefault(code, name)
        except sqlite3.Error:
            continue
    return out


def describe(answer, duel, known):
    def name(card):
        return known.get(card.get("code"), f"card {card.get('cid')}")
    def cards(cards):
        return [c for c in cards or [] if c]
    lines = [f"duel {duel} (team 0 went first)"]
    for team, side in enumerate(answer["teams"]):
        lines.append(f"team {team}: LP {side.get('lp')}, Deck {side.get('deck')}")
        lines.append("  hand: " + (", ".join(name(c) for c in cards(side.get("hand"))) or "empty"))
        monsters = []
        for zone, c in enumerate(side.get("mzone") or []):
            if c:
                face = "face-down" if c.get("position", 1) & 0xA else ("DEF" if c.get("position", 1) & 0x4 else "ATK")
                monsters.append(f"{name(c)} {c.get('attack')}/{c.get('defense')} {face} (zone {zone})")
        lines.append("  monsters: " + ("; ".join(monsters) or "none"))
        spells = [f"{name(c)}{' (set)' if c.get('position', 1) & 0xA else ''}" for c in cards(side.get("szone"))]
        lines.append("  spells and traps: " + (", ".join(spells) or "none"))
        lines.append("  Graveyard: " + (", ".join(name(c) for c in cards(side.get("grave"))) or "empty"))
        removed = cards(side.get("removed"))
        if removed:
            lines.append("  banished: " + ", ".join(name(c) for c in removed))
    return "\n".join(lines)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    scratch = os.environ.get("YO3_SCRATCH")
    p.add_argument("--server", default=os.path.join(scratch, "server.log") if scratch else None, help="the server's log")
    p.add_argument("--duel", type=int, help="a duel's number in the log (default: the newest)")
    p.add_argument("--json", action="store_true", help="the engine's state as it is")
    args = p.parse_args(argv)
    if not args.server:
        sys.exit("duel-state.py: --server SERVER.log, or YO3_SCRATCH (scratch-server.py env)")
    with open(args.server, encoding="utf-8", errors="replace") as f:
        found = newest_state(records(f), args.duel)
    if not found:
        sys.exit("duel-state.py: no engine state in the log (is the duel check on? --duel-check)")
    duel, answer = found
    print(json.dumps(answer["teams"], indent=1) if args.json else describe(answer, duel, names()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
