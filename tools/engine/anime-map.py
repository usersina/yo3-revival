#!/usr/bin/env python3
"""Builds engine/anime-cards.json, the reviewed map from the client's cards to their anime versions (issue #60).

Input (committed): engine/anime/research.tsv, the issue's reviewed table (client Konami id, name, anime passcode in the
pinned unofficial database, the research label, a note). Nothing is derived from upstream aliases: the table is the
only source of which anime card a game card gets, and an upstream alias that points at a different card (anime Power
Bond aliases Polymerization) holds the entry back.

An entry is `mapped` only if the client can show what the engine will play. The client draws a card's type, level,
ATK, DEF, attribute and race from its own files, and puts Fusion and Synchro Monsters in the Extra Deck by its own kinds,
so the anime version has to agree with the game's card in all of them. Anything else is `held` (the game card keeps
its current behaviour) with the reasons listed: the game card is the official database's row for its Konami id, which
the card check (crates/yo3-server/tests/card_data.rs) holds equal to the client's, and a Rust test re-checks every
mapped card against the real client data.

Needs the pinned data (engine/fetch.sh). `--check` compares the committed file with a fresh build.
"""
import argparse
import json
import os
import re
import sqlite3
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ENGINE = os.path.join(HERE, "..", "..", "engine")

MONSTER, SPELL, TRAP = 0x1, 0x2, 0x4
# What the client's deck editor puts in the Extra Deck, and every other monster flag it shows.
EXTRA = 0x40 | 0x2000 | 0x800000 | 0x4000000
TYPE_NAMES = {0x10: "Normal", 0x20: "Effect", 0x40: "Fusion", 0x80: "Ritual", 0x200: "Spirit", 0x400: "Union", 0x800: "Gemini",
              0x1000: "Tuner", 0x2000: "Synchro", 0x4000: "Token", 0x10000: "Quick-Play", 0x20000: "Continuous", 0x40000: "Equip",
              0x80000: "Field", 0x100000: "Counter", 0x200000: "Flip", 0x400000: "Toon", 0x800000: "Xyz", 0x1000000: "Pendulum",
              0x2000000: "Special Summon only", 0x4000000: "Link"}
SCRIPT_FOLDERS = ["official", "pre-errata", "pre-release"]


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def pins():
    text = read(os.path.join(ENGINE, "fetch.sh"))
    get = lambda name: re.search(rf"^{name}=(\w+)", text, re.M).group(1)
    return {"scripts": get("SCRIPTS_COMMIT"), "database": get("DATABASE_COMMIT")}


def records():
    """engine/anime/records.json: the corrected anime records (cards whose upstream entry is a placeholder), by passcode."""
    return {r["code"]: r for r in json.loads(read(os.path.join(ENGINE, "anime/records.json")))["records"]}


def official_passcodes(alias):
    """Konami id -> the database's original passcode, as crates/yo3-engine's Cards::open picks it."""
    out = {}
    for card in json.loads(read(os.path.join(ENGINE, "card-ids.json")))["data"]:
        ids = list(dict.fromkeys([card["id"]] + [i["id"] for i in card.get("card_images", [])]))
        known = [i for i in ids if i in alias]
        if not known:
            continue
        original = next((i for i in known if alias[i] == 0), known[0])
        for misc in card.get("misc_info", []):
            cid = misc.get("konami_id", 0)
            if cid and cid not in out:
                out[cid] = original
    return out


def type_names(t):
    return [n for bit, n in TYPE_NAMES.items() if t & bit]


def stat_differs(official, anime):
    """ATK/DEF, -2 being "?": the client shows its own number for a "?" (5110 ATK for Clear Vice Dragon), so a "?" and
    a 0 are not the same card, and neither are two numbers."""
    return official != anime


# The card profile (issue #60, crates/yo3-server/src/profile.rs): the client's card table is rewritten in memory by the
# connection DLL (tools/mod), so a difference between the game's card and its anime version in these categories can be
# shown as the anime card. A category goes into PROFILE_COVERS once the client side of it is proven live (docs/anime-cards.md,
# "Release order"): until then a card with such a difference stays held, apart from the proof cards.
#   kind      the card is a Monster, a Spell or a Trap in the other version
#   type      the client's kind of monster (Normal, Effect, Fusion, Ritual, Tuner, Synchro...) or a Spell/Trap's kind
#   subtype   a Spell/Trap's own kind: Continuous, Quick-Play, Equip, Field, Counter
#   flags     a type flag the client shows nowhere (Flip, Special Summon only): its row is the same in both versions
#   deck      Main/Extra Deck placement (Fusion and Synchro Monsters are Extra Deck cards)
#   level, atk, def, attribute, race
PROFILE_COVERS = {"flags", "subtype", "type", "deck", "level", "attribute", "race", "atk", "def", "kind"}
# The cards the whole path is proven with first (Normal -> Effect, Monster -> Spell/Trap, Main/Extra Deck placement,
# text and artwork): each is released for the categories it needs.
PROOF_SET = {4021: "Flame Swordsman", 4008: "Mystical Elf", 4804: "Negate Attack"}
PROOF_COVERS = {"kind", "type", "subtype", "deck", "flags"}
# Exact ATK/DEF (not in tens): the profile carries the number itself and the connection DLL answers the ATK/DEF readers with
# it. The cards here are released for "atk"/"def" alone, once EXACT_STATS_PROVEN says the card panel, the trunk list and the
# duel hover all show the number (docs/client-internals.md, "Card profiles"): Earthbound Immortal Wiraqocha Rasca is 1/1.
EXACT_STAT_CARDS = {8461: "Earthbound Immortal Wiraqocha Rasca"}
EXACT_STATS_PROVEN = True
SUBTYPE_BITS = 0x10000 | 0x20000 | 0x40000 | 0x80000 | 0x100000
FLAG_BITS = 0x200000 | 0x2000000


# Anime cards whose upstream database row aliases another card on purpose, and are kept: the engine reports a card by its own
# code and Konami id, never by its alias (crates/yo3-engine/src/cards.rs `cid_of`), so the game shows the card it has. Power Bond
# is treated as Polymerization by the core's alias rule; tests/anime_cards.rs `power_bond_keeps_its_identity_and_doubles_the_machine`.
ALIAS_KEPT = {6398: "treated as Polymerization (24094653)"}

# Every reviewed card ends with one disposition (#60): "mapped" (anime enabled), "default" (the game's version retained, with the
# reason below: a decision, not a bug to fix) or "held" (temporarily, until a technical fix lands; the reasons are computed).
DEFAULT_RETAINED = {
    4022: "Default retained, reviewed with Thousand Dragon: the pinned anime script's roulette wins on 2 of the die's 6 numbers"
          " (1 in 3) where the anime's wins 1 in 2, and its aging effect is not the episodes'. Both keep the game's version until a"
          " corrected script is written and verified.",
    4075: "Default retained with Time Wizard, which summons it: the two are reviewed together (Time Wizard's roulette and aging"
          " effect differ from the episodes).",
    5231: "Default retained: the anime rules need a reconstruction of Duel Monsters episodes 87 and 89 (which cards the sealed-card"
          " effect may seal, the set restrictions, how far the protection reaches, the release costs, when the card is lost). Upstream"
          " 513000016 mixes the original, later and printed rules and its cost check is wrong (hand vs field), so it is never enabled.",
}


def covered(cid, category):
    """Whether the profile layer proven so far covers a difference in `category` for the card `cid`."""
    if cid in EXACT_STAT_CARDS and EXACT_STATS_PROVEN and category in ("atk", "def"):
        return True
    return category in PROFILE_COVERS or (cid in PROOF_SET and category in PROOF_COVERS)


def differences(o, a):
    """What the client would show differently from the engine: o and a are (type, atk, def, level, race, attribute).
    A list of (category, text); the categories are PROFILE_COVERS's."""
    out = []
    ot, at = o[0], a[0]
    if (ot & 7) != (at & 7):
        out.append(("kind", "kind: the game's card is a %s, the anime one a %s" % (card_class(ot), card_class(at))))
        return out
    if (ot & EXTRA) != (at & EXTRA):
        out.append(("deck", "deck: Main/Extra Deck placement differs (%s -> %s)" % (" ".join(type_names(ot & EXTRA)) or "Main Deck", " ".join(type_names(at & EXTRA)) or "Main Deck")))
    rest = (ot ^ at) & ~EXTRA & ~7
    for category, bits in (("type", rest & ~SUBTYPE_BITS & ~FLAG_BITS), ("subtype", rest & SUBTYPE_BITS), ("flags", rest & FLAG_BITS)):
        if bits:
            out.append((category, "%s: %s" % ("type" if category == "type" else category, ", ".join(("-" if ot & b else "+") + TYPE_NAMES[b] for b in TYPE_NAMES if bits & b))))
    if ot & MONSTER:
        for i, name in ((1, "atk"), (2, "def")):
            if o[i] == -2 and a[i] == 0:
                # The game's card has "?" and the anime database's 0 stands for it (the script sets the number): showing 0 would
                # be less true than the "?" the game shows now, and the profile has no way to say "keep the game's".
                out.append(("placeholder", "placeholder: the game shows %s ?, the anime database has 0 for the card whose script sets it" % name.upper()))
            elif stat_differs(o[i], a[i]):
                out.append((name, "%s: %d -> %d" % (name, o[i], a[i])))
        for i, name in ((3, "level"), (4, "race"), (5, "attribute")):
            if o[i] != a[i]:
                out.append((name, "%s: %d -> %d" % (name, o[i], a[i])))
    return out


def card_class(t):
    return "Monster" if t & MONSTER else "Spell" if t & SPELL else "Trap" if t & TRAP else "card"


def core_api(vendor):
    """The functions the pinned core registers for Lua (Duel.X, Card.X ...), and those the scripts define themselves."""
    known = set()
    for name in sorted(os.listdir(os.path.join(vendor, "ygopro-core"))):
        if name.startswith("lib") and name.endswith(".cpp"):
            text = read(os.path.join(vendor, "ygopro-core", name))
            module = re.search(r"#define LUA_MODULE (\w+)", text)
            if module:
                known |= {(module.group(1), n) for n in re.findall(r"LUA_(?:STATIC_)?FUNCTION(?:_ALIAS|_EXISTING)?\((\w+)", text)}
    folders = ["", "official", "unofficial", "pre-errata", "pre-release"]
    for folder in folders:
        base = os.path.join(vendor, "CardScripts", folder)
        for name in os.listdir(base):
            if name.endswith(".lua"):
                text = read(os.path.join(base, name))
                known |= set(re.findall(r"function\s+(Duel|Card|Group|Effect|Debug)\.(\w+)", text))
                known |= {(m[0], m[1]) for m in re.findall(r"^(Duel|Card|Group|Effect|Debug)\.(\w+)\s*=", text, re.M)}
    return known


TOKEN_CALL = re.compile(r"(?:CreateToken|IsPlayerCanSpecialSummonMonster)\(\s*[\w.:()]+?\s*,\s*(\d{6,9})\b")


# A token made with another card's code (a copy of it). The translator shows it as the client's card of that Konami id
# (`token_card_rows`: a token with a Konami id is the client's card of that id), which is the copied monster itself.
COPY_TOKEN = re.compile(r"CreateToken\(\s*[\w.:()]+?\s*,\s*[\w.:()]*Get(?:Original)?Code\(")


# An effect or procedure used from the Deck: the client's card menus open on cards in the hand and on the field (and the
# Extra Deck's Summons), never on a card in the Deck, so a Special Summon from it can be offered and not answered.
FROM_DECK = re.compile(r"SetRange\([^)\n]*LOCATION_DECK")
# What the Deck pile offers (the translator's `command_place`: a listed card in the Deck is clickable on the pile at index 0):
# a Special Summon procedure (EFFECT_SPSUMMON_PROC), verified by translate_contract.rs
# `a_summon_from_the_deck_is_offered_on_the_deck_pile` and a real-engine duel of Andro Sphinx (tests/anime_cards.rs). An effect
# the Deck triggers (a chain window, Theinen and Thousand Dragon) is not verified.
DECK_ROUTES = {"EFFECT_SPSUMMON_PROC"}


def deck_effects(text):
    """The effect codes of the script's effects that work from the Deck: for each SetRange naming the Deck, the SetCode of the
    effect it belongs to (the block since the last Effect.CreateEffect / Clone), or "?" when the block sets none."""
    found = []
    for m in FROM_DECK.finditer(text):
        start = max(text.rfind("Effect.CreateEffect", 0, m.start()), text.rfind(":Clone()", 0, m.start()))
        codes = re.findall(r"SetCode\(([\w.+|]+)\)", text[max(start, 0):m.start() + 200])
        found.append(codes[-1] if codes else "?")
    return found


def client_tokens():
    """The names of the token cards the client has (engine/anime/client-tokens.tsv, a Konami id and a name each, kept equal
    to the game's by crates/yo3-server/tests/card_data.rs): a token it has no card of is not shown."""
    return {line.split("\t", 1)[1] for line in read(os.path.join(ENGINE, "anime/client-tokens.tsv")).splitlines() if line}


def script_path(script):
    """A mapped card's script file: ProjectIgnis' unofficial one, or ours in engine/overrides/."""
    if script.startswith("overrides/"):
        return os.path.join(ENGINE, script)
    return os.path.join(ENGINE, "vendor", "CardScripts", script)


def build():
    vendor = os.path.join(ENGINE, "vendor")
    official = sqlite3.connect(os.path.join(vendor, "BabelCDB/cards.cdb"))
    unofficial = sqlite3.connect(os.path.join(vendor, "BabelCDB/cards-unofficial.cdb"))
    alias = {r[0]: r[1] for r in official.execute("select id, alias from datas")}
    passcode = official_passcodes(alias)
    artworks = {}
    for code, a in alias.items():
        if a:
            artworks.setdefault(a, set()).add(code)
    card = lambda db, code: db.execute("select d.type, d.atk, d.def, d.level & 255, d.race, d.attribute, d.alias, t.name from datas d join texts t on t.id = d.id where d.id = ?", (code,)).fetchone()
    # Official scripts that name each number, to see who refers to a card whose anime version has another name.
    mentions = {}
    for folder in SCRIPT_FOLDERS:
        base = os.path.join(vendor, "CardScripts", folder)
        for name in sorted(os.listdir(base)):
            if name.endswith(".lua"):
                for n in set(re.findall(r"\b\d{5,9}\b", read(os.path.join(base, name)))):
                    mentions.setdefault(int(n), set()).add(name)
    recs = records()
    anime_codes = {r[0] for r in unofficial.execute("select id from datas")} | set(recs)
    api = core_api(vendor)
    tokens = client_tokens()
    name_of = lambda code: next((r[0] for db in (official, unofficial) for r in db.execute("select name from texts where id = ?", (code,))), None)
    entries = []
    for line in read(os.path.join(ENGINE, "anime/research.tsv")).splitlines()[1:]:
        cid, name, anime, label, note = (line.split("\t") + [""] * 5)[:5]
        cid = int(cid)
        entry = {"cid": cid, "name": name, "anime": int(anime) if anime.isdigit() else None, "label": label}
        if note:
            entry["note"] = note
        holds = []
        diffs = []
        o = passcode.get(cid)
        entry["official"] = o
        if label == "Held":
            holds.append("research: " + note)
        elif o is None:
            holds.append("the engine has no official card for this Konami id")
        elif entry["anime"] not in anime_codes:
            holds.append("the pinned unofficial database has no entry %s" % anime)
        else:
            orow = card(official, o)
            rec = recs.get(entry["anime"])
            if rec:
                # A corrected record replaces the placeholder's row; its script is ours.
                arow = (rec["type"], rec["atk"], rec["def"], rec["level"] & 255, rec["race"], rec["attribute"], rec["alias"], rec["name"])
                entry["reconstruction"] = bool(rec.get("reconstruction"))
            else:
                arow = card(unofficial, entry["anime"])
            entry["alias"] = arow[6]
            script = None
            for folder in ["unofficial"]:
                p = os.path.join(vendor, "CardScripts", folder, "c%d.lua" % entry["anime"])
                if os.path.exists(p) and not rec:
                    script = "unofficial/c%d.lua" % entry["anime"]
            if rec and os.path.exists(os.path.join(ENGINE, "overrides", "c%d.lua" % entry["anime"])):
                script = "overrides/c%d.lua" % entry["anime"]
            entry["script"] = script
            if arow[6] not in (0, o) and arow[6] not in artworks.get(o, set()) and cid not in ALIAS_KEPT:
                holds.append("alias: upstream aliases %d (%s), another card" % (arow[6], (card(official, arow[6]) or ("?",) * 8)[7]))
            if script is None and not (arow[0] & 0x10 and not arow[0] & 0x20):
                holds.append("the anime card has an effect but no script")
            if arow[6] == 0:
                named_by = sorted(mentions.get(o, set()) - {"c%d.lua" % o})
                if named_by:
                    entry["named_by"] = named_by
                    holds.append("the anime card has no alias to the game's card and %d official script(s) name that card" % len(named_by))
            diffs = differences(orow[:6], arow[:6])
            if diffs:
                entry["differs"] = sorted({c for c, _ in diffs})
            if script:
                text = read(script_path(script))
                missing = sorted({"%s.%s" % m for m in re.findall(r"\b(Duel|Card|Group|Effect|Debug)\.(\w+)", text)} - {"%s.%s" % k for k in api})
                if missing:
                    holds.append("the script calls %s, which the engine's core does not have" % ", ".join(missing))
                made = sorted({n for n in (name_of(int(code)) for code in TOKEN_CALL.findall(text)) if n and n.endswith("Token")})
                if made:
                    entry["tokens"] = made
                unverified = [c for c in deck_effects(text) if c not in DECK_ROUTES]
                if unverified:
                    holds.append("the script has an effect used from the Deck (%s), which the client's menus have not been shown to offer" % ", ".join(sorted(set(unverified))))
                if COPY_TOKEN.search(text):
                    # A token made with another card's code is that card to the engine (its Konami id, its row): the client
                    # shows the card itself, as the anime shows a copy. Multiply and Cloning, tests/anime_cards.rs.
                    entry["copies"] = True
                lacking = [n for n in made if n not in tokens]
                if lacking:
                    holds.append("the script creates %s, which the client has no card for" % ", ".join(lacking))
                text = read(script_path(script))
                deps = sorted({int(n) for n in re.findall(r"\b\d{9}\b", text) if int(n) in anime_codes and int(n) != entry["anime"]})
                if deps:
                    entry["depends"] = deps
        # What the profile shows as the anime card is no hold; a difference it does not cover yet is.
        if holds and diffs:
            entry["blocked"] = True
        shown = [t for c, t in diffs if covered(cid, c)]
        holds += [t for c, t in diffs if not covered(cid, c)]
        entry["status"] = "default" if cid in DEFAULT_RETAINED else "held" if holds else "mapped"
        if cid in DEFAULT_RETAINED:
            entry["reason"] = DEFAULT_RETAINED[cid]
        elif holds:
            entry["holds"] = holds + shown
        elif shown:
            entry["profile"] = shown
        entries.append(entry)
    # A script that names another game card's anime version by its anime passcode only works if that card plays as
    # its anime version too: hold it until every such dependency is mapped (a fixpoint, since a hold can chain).
    by_anime = {e["anime"]: e for e in entries if e["anime"]}
    changed = True
    while changed:
        changed = False
        for e in entries:
            if e["status"] != "mapped":
                continue
            for dep in e.get("depends", []):
                other = by_anime.get(dep)
                if other and other["status"] != "mapped":
                    e["status"] = "held"
                    e.setdefault("holds", []).append("depends on %s (%d), whose anime version is %s" % (other["name"], dep, "retained as the default" if other["status"] == "default" else "held"))
                    changed = True
    official.close()
    unofficial.close()
    entries.sort(key=lambda e: e["cid"])
    counts = {s: sum(1 for e in entries if e["status"] == s) for s in ("mapped", "default", "held")}
    return {"issue": 60, "pins": pins(), "counts": counts, "records": list(recs.values()), "cards": entries}


CATEGORY_ORDER = ["flags", "subtype", "type", "deck", "level", "attribute", "race", "atk", "def", "kind"]
# What each released category is, and the card that proves it (tests/card_profile.rs, tests/anime_cards.rs).
CATEGORY_NOTES = {
    "flags": ("nothing: Flip and Special Summon only are shown nowhere, the engine plays the anime card", "Wall of Ivy: its token comes when it is attacked face-up"),
    "subtype": ("a Spell or Trap's own kind (Continuous, Quick-Play, Equip, Field, Counter)", "Fiend's Sanctuary: a Quick-Play Spell, activated on the opponent's turn"),
    "type": ("Normal, Effect, Tuner, Ritual and the other kinds of monster", "Boost Warrior: a Tuner, so it Synchro Summons"),
    "deck": ("Main or Extra Deck: a Fusion Monster is an Extra Deck card, a Normal one a Main Deck card", "Perfect Machine King: a Fusion Monster, Fusion Summoned from the Extra Deck"),
    "level": ("Level", "Rapid Warrior: Level 3, so it makes the Level 6 Synchro Monster"),
    "attribute": ("attribute", "Granadora: DARK, so Umiiruka does not raise it"),
    "race": ("type of monster (Warrior, Beast...)", "Chiron the Mage: a Beast, so Sogen does not raise it"),
    "atk": ("ATK, also a number that is not a multiple of ten, and `?`", "Pumpking the King of Ghosts: 1500 ATK, the damage it takes"),
    "def": ("DEF, as ATK", "Earthbound Immortal Chacu Challhua: 1600 DEF"),
    "kind": ("Monster, Spell or Trap", "Mind Control: a Trap, activated on the opponent's turn"),
}


def release_report(data):
    """What each category would release: the held cards that differ from the game's card in exactly the categories enabled so
    far and in nothing else a hold names (a script, an alias, research). Alone, then cumulatively in CATEGORY_ORDER. The
    cards the proof set already released are not held, so are not counted."""
    held = [c for c in data["cards"] if c["status"] == "held" and not c.get("blocked") and c.get("differs")
            and all(h.split(":")[0] in CATEGORY_ORDER + ["type"] for h in c["holds"])]
    rows, enabled = [], set()
    for category in CATEGORY_ORDER:
        enabled.add(category)
        alone = [c for c in held if set(c["differs"]) <= {category}]
        together = [c for c in held if set(c["differs"]) <= enabled]
        rows.append((category, len(alone), len(together)))
    return rows


def dump(data):
    head = {k: v for k, v in data.items() if k != "cards"}
    lines = [json.dumps(c, ensure_ascii=False, sort_keys=False) for c in data["cards"]]
    return json.dumps(head, ensure_ascii=False)[:-1] + ',"cards":[\n' + ",\n".join(lines) + "\n]}\n"


HELD_NOTES = {
    "8035": "Our own record and script (engine/anime/records.json, engine/overrides/c513000079.lua): upstream 513000079 is a placeholder. A reconstruction of 5D's episodes 21 and 134, not verified.",
    "8302": "Our own record and script (engine/anime/records.json, engine/overrides/c511247002.lua): upstream 511247002 is a placeholder. The official card with the destroy trigger on any face-up Attack Position Special Summon; the target is the official card's.",
    "5231": "Default retained: needs a reconstruction of the anime card (Duel Monsters episodes 87 and 89: which cards the sealed-card effect may seal, the set restrictions, the scope of the protection, the release costs, when the card is lost) before any script. Upstream 513000016 mixes the original, later and printed rules and its cost check is wrong (hand vs field), so it is never enabled. Nothing is implemented.",
    "4804": "Era and type decision open (Quick-Play in the Japanese anime, Trap or Counter Trap elsewhere); the ZEXAL entry 511756005 is a different card and is not used.",
    "6398": "Upstream aliases Polymerization (24094653); never follow it.",
}


def from_triage(folder):
    """Rewrites engine/anime/research.tsv and retained.tsv from the issue #60 triage files (animifiable-cards.csv: the 415
    reviewed mappings; all-game-cards.csv: every card's disposition). The files stay outside git."""
    import csv
    def rows(name):
        with open(os.path.join(folder, name), encoding="utf-8-sig", newline="") as f:
            return list(csv.DictReader(f))
    out = ["cid\tname\tanime\tlabel\tnote"]
    for r in sorted(rows("animifiable-cards.csv"), key=lambda r: int(r["cid"])):
        label = "Anime / Normal" if r["anime_type"] and int(r["anime_type"]) & 0x30 == 0x10 else "Anime"
        out.append("\t".join([r["cid"], r["name"], r["anime_id"], label, HELD_NOTES.get(r["cid"], "")]))
    allc = rows("all-game-cards.csv")
    held = [r for r in allc if r["disposition"] == "review_required"]
    own = {str(r["cid"]): r for r in records().values()}
    out += ["\t".join([r["cid"], r["name"], str(own[r["cid"]]["code"]) if r["cid"] in own else "-", "Reconstruction" if r["cid"] in own else "Held", HELD_NOTES[r["cid"]]]) for r in held]
    with open(os.path.join(ENGINE, "anime/research.tsv"), "w", encoding="utf-8") as f:
        f.write("\n".join(out) + "\n")
    kept = ["cid\tname\tdisposition\tupstream entry"]
    for r in allc:
        if r["disposition"] in ("retain_current_equivalent_normal_metadata", "retain_current_rejected_counterpart"):
            kept.append("\t".join([r["cid"], r["name"], r["disposition"], r["anime_ids"]]))
    with open(os.path.join(ENGINE, "anime/retained.tsv"), "w", encoding="utf-8") as f:
        f.write("\n".join(kept) + "\n")
    print("%d reviewed, %d held for custom work, %d retained with a note, %d with no catalogue match" % (
        len(out) - 1 - len(held), len(held), len(kept) - 1, sum(1 for r in allc if r["disposition"] == "retain_current_no_catalogue_match")))


RECONSTRUCTION_DETAILS = """### Dark Strike Fighter (8035)

A Level 7 DARK Machine Synchro/Effect Monster, 2600/1800 (the upstream placeholder had 2700 ATK). The rules below are the community's
reconstruction of 5D's episodes 21 and 134, not verified against the footage; where it left room, the choice is ours.

- Summoned with the Synchro procedure the game already has (1 Tuner + 1 or more non-Tuner monsters).
- An ignition effect, usable in either of your Main Phases, once per turn **for each copy** (not "by name").
- Cost: Tribute another monster you control (this card itself can never be the Tribute). Its Level is taken when it is Tributed.
- Effect: inflict that Level x 200 damage to your opponent. The amount belongs to the activation, so it is dealt even if Dark Strike Fighter
  has left the field before the effect resolves.
- Attack restriction, our interpretation of "this card cannot attack during the turn you activate this effect": the effect cannot be
  activated if that copy has already declared an attack this turn, and activating it forbids that copy to attack for the rest of the turn.
  The restriction is installed at activation, so negating the effect does not lift it, and it ends with the turn.

### Fortune Lady Fire (8302)

The official card (ATK/DEF = its Level x 200, the Level up by 1 in each of your Standby Phases to at most 12) with one change: its destroy
effect triggers whenever it is Special Summoned face-up in Attack Position, by any means. No "Fortune Lady" source is needed, and a Special
Summon procedure with no effect that activates counts (Monster Reborn, a Fortune Lady effect, anything). Special Summoned in Defense
Position, or Normal Summoned, it does not trigger. The target is the official card's (1 face-up monster your opponent controls),
the rule the anime card's text gives too; if the opponent has none, the trigger is not activated. The destruction and the damage go together: nothing destroyed
(a protected monster, one that left the field before the effect resolved) means no damage, and the damage is the destroyed monster's ATK
as it was before it was destroyed.

### Skill Drain and the Egyptian Gods

See above: not a reconstruction but a fix (`engine/overrides/c82732705.lua`); the three Gods are mapped, so it applies to all of them."""


def doc(data):
    """docs/anime-cards.md: every disposition in words, for players and hosts."""
    mapped = [c for c in data["cards"] if c["status"] == "mapped"]
    held = [c for c in data["cards"] if c["status"] == "held"]
    default = [c for c in data["cards"] if c["status"] == "default"]
    shown = [c for c in mapped if c.get("profile")]
    releases = release_report(data)
    out = ["# Anime card versions", "",
           "<!-- Made by tools/engine/anime-map.py from engine/anime/research.tsv: do not edit by hand. -->", "",
           "With the host's `anime-cards` setting on ([playing](playing.md#anime-card-versions)), the %d cards below play by their"
           " anime effects, ProjectIgnis' unofficial scripts (CardScripts `%s`), in NPC and player duels, and the game shows them as"
           " their anime versions: the **card profile**. The other cards, and the %d retained or held back below, play and show as they always did."
           " The reviewed table is the issue [#60](https://github.com/usersina/yo3-revival/issues/60); an unofficial script loading"
           " without an error is not proof of anime accuracy, only that it plays." % (len(mapped), data["pins"]["scripts"][:7], len(held) + len(default)),
           "", "## The card profile", "",
           "A card keeps its Konami id, so it is the card a player already owns, in the decks they already have: under the anime"
           " profile it becomes its anime version. A profile is a named set of card data that every part reads (`yo3-server`'s"
           " `profile.rs`; the `anime` profile is `engine/anime-cards.json`).", "",
           "| Part | What changes |", "| --- | --- |",
           "| Rules engine | The anime card's script: its effects, stats, types and summoning procedures |",
           "| Client metadata | The client's own card table is rewritten in memory by the connection DLL: kind (Normal, Effect, Spell, Trap...), Spell/Trap kind, Level, ATK and DEF (also numbers that are not multiples of ten, and `?`), attribute, race, and whether the card is a Main or an Extra Deck card |",
           "| Card display | The anime card's effect text and, when you provide it, its artwork ([below](#artwork)) |",
           "| Saved decks | Cards keep their place in the collection and their quantities in every deck; a card whose class changes (Flame Swordsman: Extra Deck in the game, Main Deck in the anime) moves to the list the profile puts it in, in the server's decks (list, uploads, checks, NPC and official decks) and in your own local deck files (the launcher places them before the game starts, [below](#local-deck-files)), with no manual editing |",
           "| Duel check | `--duel-check` compares the client's view with the engine, and holds the profile's cards to what the profile changes of them (what is the game's own is the client's data, which `card_data.rs` holds equal to the engine's for every card): the first time a profile monster is face-up on the field its base ATK and DEF (a `?` too) must be the profile's, and the self-play (`tests/duel_check.rs`, anime decks of 30 duels by default, 300 in a full run, and decks made of the cards the profile changes) also checks each deck card's Level, race and attribute against the engine's card data (on the field those are the effects' as much as the card's), so a profile that disagrees with what the engine plays fails the check (`a_profile_that_differs_from_the_engine_fails_the_duel_check` proves it fails). The native checkpoints (`[YO3 EXPECT]`) hash the stats the client actually received, which come from the profile's row and the engine's field reports |",
           "", "The server picks the profile (the `anime-cards` setting, later an event) and pins it: the game fetches it once, when it starts"
           " (`GET /web/card-profile` through the launcher, which only a signed-in launcher reaches), and reads its card table once."
           " **A change of profile therefore needs a restart of the game** (Play again), until it is proven that every cached card"
           " refreshes safely.", "",
           "### When the profile changes", "",
           "- **A server restart activates the profile the saved setting selects**; a live change (`yo3 set anime-cards on|off`) applies to the next request"
           " for it. The rules engine reads the same setting at each duel, so it plays the anime scripts exactly while the pinned profile is the anime one.",
           "- **The server remembers what each signed-in game fetched** (the profile's name and hash, by launcher session) and **an open game that loaded"
           " another profile than the pinned one is refused every duel**, not just warned: it would play with the card data it started with. It is refused at"
           " every way in with a reply the client shows (Auto Duel and the official events' Participate `0x179A`, NPC duels and Create Duel `0x17AE`, a"
           " challenge `0x17B8` 201, creating or joining a user event `0x33F5`/`0x33FB`), each with a line from \"Server\" in the player's chat: close the game"
           " and press Play again. Select Deck (`0x22CB`, back to the map; the opponent is released as for any refused deck) is the last gate, so a game that"
           " went stale while queued, matched or hosting cannot start either, and a host's Accept from such a game is a decline"
           " ([protocol](protocol.md), under \"A game with stale card data cannot duel\").",
           "- **A game that loaded nothing counts as stale while a profile is pinned**: one that started before the host turned the setting on, or without the"
           " connection DLL's profile support (it never fetches), cannot be shown to display the pinned cards. The launcher's own read of the profile, to place"
           " the local decks (`?for=decks`), is not the game's fetch and proves nothing. **With the game's own cards pinned (the setting off, or no anime data on the"
           " server) every client duels as before**, except one that loaded a profile.",
           "- **A relaunch matches**: Play again fetches the pinned profile (and the launcher places the local decks by it first), so client and engine agree."
           " Desktop's helper polls the pinned profile every 20 seconds and turns the Play button into RESTART THE GAME while it differs from what the helper"
           " saw the game load; that is a convenience, the server's refusal is the rule.",
           "- **Switching off and on again keeps every deck's cards and quantities**, in the server's decks and in the local deck files (each pass only moves a card"
           " between the Main and Extra lists; nothing is lost, a deck is never deleted, the first version of a changed file is kept once).",
           "- **Missing artwork is not a mismatch**: a card without a picture in `anime-art` keeps the game's own artwork while its stats and text still follow"
           " the profile. (The connection DLL reports nothing back, so a profile it fetched but could not apply is not noticed by the server: its log says so.)", "",
           "## Local deck files", "",
           "The game keeps your decks as `players/<name>/decks/*.y3d` in its Documents folder, and its deck editor files a card by the card's kind, so a deck"
           " saved under another profile holds Flame Swordsman in the wrong list. **The launcher places them before the game is released** (the helper, once it has"
           " the session and the profile the game will fetch; `yo3_launcher::decks`): each card the profile covers goes to the list the profile puts it in, by the"
           " server's own rule, with every card and copy kept and the others in their order. A deck is never deleted. The first change to a file keeps"
           " a copy beside it (`<deck>.y3d.pre-profile`, never overwritten), the profile a player was last adapted to is noted in"
           " `players/<name>/revival-profile.txt` (with where each moved card was, so the game's own profile brings it back), and a start with the same profile"
           " changes nothing. A file that is not a deck the game wrote, or cannot be written, is left alone and named in the game's log. The folder is the game's own for"
           " that server (the original `Yu-Gi-Oh! ONLINE 3` folder for the first server played on, `YO3 Revival/servers/<id>/Yu-Gi-Oh! ONLINE 3` for every"
           " other; the Desktop tells the helper where Documents is). A game opened without the launcher (direct start) adapts at the next Play; the server places the deck for every duel meanwhile.",
           "",
           "## Artwork", "",
           "Nothing in this project ships, fetches or converts Konami's or the anime's card images. A card shows its anime artwork"
           " only if you put the picture in a folder the launcher reads at start; a card without a file keeps the game's own artwork.", "",
           "- `anime-art/card/<Konami id>.png`: the whole card face as the anime version reads (frame, name, picture, text, stats), a"
           " 200x290 PNG like the game's own `data/card/<id>_<art>.png`. One file per Konami id serves all of that card's artworks"
           " (Flame Swordsman's two).",
           "- `anime-art/mini/<Konami id>.png`: optional, the 50x72 thumbnail like `data/mini/<id>_<art>.png` (Edit Deck lists, the duel's"
           " field). When it is missing the launcher makes it from the face (the thumbnails it made are listed in `mini/.derived`, made"
           " again when the face changes, and never replace one you provided).",
           "- The folder is `%LOCALAPPDATA%\\YO3 Revival\\anime-art` on Windows and `~/.local/share/yo3-revival/anime-art` on Linux (the"
           " launcher's data folder; `$YO3_DESKTOP_DATA/anime-art` when that is set). The launcher creates it, passes it to the game as"
           " `YO3_ANIME_ART` (and writes it into the direct-start profile) and logs how many faces it found and which files it skipped"
           " (not a PNG, not 200x290). Games started without the launcher look in `anime-art` beside `yo3.exe`.",
           "- A host's players each provide their own pictures: the server never sends artwork.", ""]
    if shown:
        out += ["## Shown as the anime card", "",
                "These cards differ from the game's in what the client draws; the profile covers every difference. (The cards that do not"
                " differ are shown with the anime card's text only.)", "", "| Card | What the client shows now |", "| --- | --- |"]
        for c in sorted(shown, key=lambda c: c["name"].lower()):
            out.append("| %s | %s |" % (c["name"], "<br>".join(h.replace("|", "/") for h in c["profile"])))
        out.append("")
    counts = {c: sum(1 for e in mapped if c in e.get("differs", [])) for c in CATEGORY_ORDER}
    out += ["## Released categories", "",
            "A card is released only when the client side of each difference is proven: the connection DLL shows it, the deck rule places"
            " it and the duel check holds the engine to it. The proof set (%s) came first: Normal and Effect, Monster and Spell/Trap, Main and"
            " Extra Deck, text and artwork, end to end. Then, category by category, the held cards that differ in that category (and in nothing"
            " else a hold names) were released, each with a test card (`tests/card_profile.rs`: the row and its deck placement), a real-core"
            " scenario in which the changed property matters (`tests/anime_cards.rs`) and the anime self-play duel checks with the category on"
            " (`tests/duel_check.rs`). `PROFILE_COVERS` in `tools/engine/anime-map.py` lists them; `tools/engine/anime-map.py --releases` says"
            " what a category would still release (nothing now)." % ", ".join(PROOF_SET.values()),
            "", "| Category | What the client shows | Cards that differ | Test card and scenario |", "| --- | --- | ---: | --- |"]
    for category in CATEGORY_ORDER:
        out.append("| %s | %s | %d | %s |" % (category, CATEGORY_NOTES[category][0], counts[category], CATEGORY_NOTES[category][1]))
    out += ["", "Negate Attack is the pinned database's anime card (a Quick-Play Spell that negates the attack and ends the Battle Phase), not the"
            " Counter Trap the game has; the version is the issue's open era decision, shown here so it can be checked in play.", "",
            "Every reviewed card has one disposition: anime enabled, the game's version retained as the default with a reason (Time Wizard"
            " and Thousand Dragon, reviewed together, and Temple of the Kings), or held for a technical fix: Theinen the Great Sphinx (an effect"
            " the Deck triggers, not a procedure: the client's menus have not been shown to offer it) and Clear Vice Dragon and Blackwing -"
            " Aurora the Northern Lights, whose \"?\" ATK the anime database spells as 0 (the profile would show 0 where the game shows \"?\"). Power Bond (it stays Power Bond and"
            " is treated as Polymerization), Multiply and Cloning (the copy shows as the monster itself) and the Sphinxes that summon themselves from the"
            " Deck are released, each with a test in `tests/anime_cards.rs`.", "",
            "**Source of truth for effects:** EDOPro/ProjectIgnis's anime scripts at the pinned commit. Effects are not re-checked against the"
            " episodes; a disagreement is fixed upstream or with a documented override in `engine/overrides/`. Two exceptions (#60): cards for which"
            " ProjectIgnis has only a placeholder (Dark Strike Fighter and Fortune Lady Fire) are researched and get our own record and script"
            " ([below](#reconstructions)); and Skill Drain against the three Egyptian Gods was a bug, fixed by an override of Skill Drain"
            " (`engine/overrides/c82732705.lua`, which changes only how it treats the anime Gods; every other monster is as before). The Gods are"
            " unaffected by most effects, but Skill Drain, when activated, negates the effects of the Gods already on the field until the end of"
            " that turn; then they come back, though Skill Drain stays face up. Ra's ATK (the Tributes' total) is 0 for the rest of that turn and the"
            " total again from the next one. A God summoned while Skill Drain is already face up is not affected at all. Each point has a real-core"
            " test in `tests/anime_cards.rs`.", "",
            "## Reconstructions", "",
            "Where ProjectIgnis has no usable script, the data record and the script are ours (`engine/anime/records.json`, `engine/overrides/`) and"
            " the card's text in the game says it is a reconstruction. Everything else about the card (the engine, the profile, the duel check,"
            " the deck placement) is as for any other mapped card; the official (standard) card, its record and its script do not change.", "",
            "| Card | Anime card | Text the game shows | Basis |", "| --- | --- | --- | --- |"]
    for r in data["records"]:
        out.append("| %s | %d | %s | %s |" % (r["name"].replace(" (Anime)", ""), r["code"], r["text"].replace("|", "/").replace("\n", "<br>"), r["basis"].replace("|", "/")))
    out += ["", RECONSTRUCTION_DETAILS, "",
            "Temple of the Kings (5231) keeps the game's version: its anime rules would need a reconstruction of Duel Monsters episodes 87 and 89"
            " (which cards the sealed-card effect may seal, the set restrictions, how far the protection reaches, the release costs and when the card"
            " is lost). The upstream script mixes the original, later and printed rules, so it is never enabled; nothing is implemented.", "",
            "## Played as their anime versions", "", "| Card | Anime card | Notes |", "| --- | --- | --- |"]
    for c in sorted(mapped, key=lambda c: c["name"].lower()):
        notes = []
        if c.get("tokens"):
            notes.append("makes " + ", ".join(c["tokens"]))
        if c.get("depends"):
            notes.append("names other anime cards")
        if c.get("copies"):
            notes.append("makes copies of a monster")
        if c["cid"] in ALIAS_KEPT:
            notes.append(ALIAS_KEPT[c["cid"]])
        out.append("| %s | %d | %s |" % (c["name"], c["anime"], "; ".join(notes)))
    out += ["", "## The game's version retained", "",
            "Reviewed, and kept as the game has them on purpose: a decision with its reason, not something waiting for a fix.", "",
            "| Card | Why |", "| --- | --- |"]
    for c in sorted(default, key=lambda c: c["name"].lower()):
        out.append("| %s | %s |" % (c["name"], c["reason"].replace("|", "/")))
    out += ["", "## Held for a technical fix", "",
            "A card is held back when the profile does not cover a difference between its anime version and the game's card yet (the",
            "client draws a card's type, level, ATK, DEF, attribute and race from its own files and keeps Fusion and Synchro Monsters in the",
            "Extra Deck), or when the research or the script is not settled. Each reason is kept in `engine/anime-cards.json` too.", "", "| Card | Why |", "| --- | --- |"]
    for c in sorted(held, key=lambda c: c["name"].lower()):
        out.append("| %s | %s |" % (c["name"], "<br>".join(h.replace("|", "/") for h in c["holds"])))
    kept = [l.split("\t") for l in read(os.path.join(ENGINE, "anime/retained.tsv")).splitlines()[1:]]
    out += ["", "## Kept on purpose", "", "| Card | Why |", "| --- | --- |"]
    for cid, name, why, up in sorted(kept, key=lambda k: k[1].lower()):
        out.append("| %s | %s |" % (name, "the anime Normal Monster is equivalent" if "equivalent" in why else "upstream entry %s is a different or prize card" % up))
    out += ["", "Every other card of the game (4,263 of 4,696) has no anime version in the catalogue and keeps its current behaviour.", ""]
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="fail if engine/anime-cards.json is not what the table builds")
    ap.add_argument("--releases", action="store_true", help="print how many held cards each category of the card profile would release")
    ap.add_argument("--list", action="store_true", help="print the held cards and why")
    ap.add_argument("--triage", metavar="DIR", help="first rewrite engine/anime/research.tsv and retained.tsv from the issue #60 triage files in DIR")
    args = ap.parse_args()
    if args.triage:
        from_triage(args.triage)
    data = build()
    out = os.path.join(ENGINE, "anime-cards.json")
    if args.releases:
        print("%-10s %6s %10s" % ("category", "alone", "cumulative"))
        for category, alone, together in release_report(data):
            print("%-10s %6d %10d" % (category, alone, together))
        return
    if args.list:
        for e in data["cards"]:
            if e["status"] != "mapped":
                print(e["cid"], e["name"], "|", e["status"], "|", e.get("reason") or "; ".join(e["holds"]))
    page = os.path.join(HERE, "..", "..", "docs", "anime-cards.md")
    if args.check:
        if read(out) != dump(data) or read(page) != doc(data):
            sys.exit("engine/anime-cards.json or docs/anime-cards.md is out of date: run tools/engine/anime-map.py")
        return
    open(out, "w", encoding="utf-8").write(dump(data))
    open(page, "w", encoding="utf-8").write(doc(data))
    print("mapped %(mapped)d, default %(default)d, held %(held)d" % data["counts"])


if __name__ == "__main__":
    main()
