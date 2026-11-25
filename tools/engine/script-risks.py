#!/usr/bin/env python3
"""Two patterns in card scripts that broke duels (2026-11-25), found by a line scan:

  first card unchecked   a script takes the first card of a choice or of its targets and uses it at once
                         (`local tc=Duel.GetFirstTarget()` then `tc:IsRelateToEffect(e)`): a cost or another effect can
                         leave nothing to choose, and the Lua error stops the effect halfway (proc_equip.lua, the anime
                         Catapult Turtle)
  widens materials       an effect that lets a summon take materials from more places (EFFECT_HAND_SYNCHRO...): the
                         engine searches more combinations each time it lists the summons, and the anime T.G. Cyber
                         Magician froze it; crates/yo3-server/tests/material_search.rs puts every such card on a crowded
                         board

Usage:
  script-risks.py              every script the engine plays for a card a player can hold (the game's, the added
                               cards', the mapped anime versions'), with the lines; a list to review, not a gate
  script-risks.py FILE...      the same for these scripts

`task cards:find` prints these for a card being added (tools/engine/added-cards.py). Needs the pinned data
(engine/fetch.sh); the game's cards need the game (YO3_GAME_DIR, else the default Wine path)."""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]):
    print(__doc__); raise SystemExit(0)
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ENGINE = os.path.join(ROOT, "engine")

WIDENING = re.compile(r"EFFECT_(HAND_SYNCHRO|EXTRA_SYNCHRO_MATERIAL|EXTRA_MATERIAL|EXTRA_FUSION_MATERIAL|EXTRA_RITUAL_MATERIAL"
                      r"|SYNCHRO_MATERIAL_CUSTOM|HAND_LINK)\b")
# `local tc=<the first card of a choice or of the targets>`
TARGET_FIRST = re.compile(r"Duel\.SetTargetCard\(\s*\w+:GetFirst\(\)\s*\)")
FIRST = re.compile(r"^\s*local\s+(\w+)\s*=\s*(Duel\.GetFirstTarget\(\)|.*:GetFirst\(\))\s*$")


def risks(text):
    """[(what, [line numbers])] for one script's text."""
    lines = [l.split("--", 1)[0] for l in text.split("\n")]
    unchecked = []
    for i, line in enumerate(lines):
        m = FIRST.match(line)
        if not m:
            continue
        var = m.group(1)
        after = next((l for l in lines[i + 1:] if l.strip()), "")
        used = re.search(rf"\b{var}:\w+\(", after)
        # `if tc and`, `not tc`, `tc~=nil`: a check; `if tc:IsRelateToEffect(e)` is a use
        checked = re.search(rf"\b(if|and|or|not)\s+{var}\b(?!\s*:)|\b{var}\s*(~=|==)\s*nil|\b{var}\s+(and|or|then)\b", after)
        if used and not checked:
            unchecked.append(i + 1)
    # The first card of a choice handed straight on as the target (the anime Catapult Turtle: nil when nothing was left)
    unchecked += [i + 1 for i, l in enumerate(lines) if TARGET_FIRST.search(l) and not re.search(r"\bif\b", l)]
    unchecked.sort()
    out = []
    # A target is chosen as the card is activated and stays even if the card leaves: only a cost, paid between the check
    # that targets exist and the choice, can leave none to choose (Orichalcos Sword of Sealing's discard, Catapult
    # Turtle's Tribute). Scripts without a cost are left out.
    if unchecked and any(re.search(r":SetCost\(", l) for l in lines):
        out.append(("first card unchecked", unchecked))
    widening = [i + 1 for i, l in enumerate(lines) if WIDENING.search(l)]
    if widening:
        out.append(("widens materials", widening))
    return out


def script_path(rel):
    """Ours (engine/overrides/) first, else ProjectIgnis'."""
    ours = os.path.join(ENGINE, "overrides", os.path.basename(rel))
    return ours if os.path.exists(ours) else os.path.join(ENGINE, "vendor", "CardScripts", rel)


def played():
    """[(label, script path)] for every script the engine plays for a card a player can hold."""
    out = []
    try:
        import importlib.util
        spec = importlib.util.spec_from_file_location("added_cards", os.path.join(HERE, "added-cards.py"))
        added = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(added)
        in_game = set(added.load()["in_game"])
    except SystemExit:
        in_game = set()
        print("(the game isn't installed: its own cards are left out)", file=sys.stderr)
    with open(os.path.join(ENGINE, "card-ids.json")) as f:
        for d in json.load(f)["data"]:
            konami = (d.get("misc_info") or [{}])[0].get("konami_id")
            if konami in in_game:
                out.append((f"game {konami}", script_path(f"official/c{d['id']}.lua")))
    with open(os.path.join(ENGINE, "added-cards.json")) as f:
        for c in json.load(f)["cards"]:
            folder = "unofficial" if c.get("anime_only") else "official"
            out.append((f"added {c['cid']} {c['name']}", script_path(f"{folder}/c{c['passcode']}.lua")))
    with open(os.path.join(ENGINE, "anime-cards.json")) as f:
        for c in json.load(f)["cards"]:
            if c.get("status") == "mapped" and c.get("script"):
                out.append((f"anime {c['cid']} {c['name']}", script_path(c["script"])))
    return out


def main(args):
    targets = [(os.path.basename(a), a) for a in args] if args else played()
    counts = {}
    for label, path in targets:
        if not os.path.exists(path):
            continue
        with open(path, encoding="utf-8", errors="replace") as f:
            found = risks(f.read())
        for what, lines in found:
            counts[what] = counts.get(what, 0) + 1
            print(f"{label}: {what}: {os.path.relpath(path, ROOT)} lines {', '.join(map(str, lines))}")
    print("; ".join(f"{n} scripts: {what}" for what, n in sorted(counts.items())) or "nothing found", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
