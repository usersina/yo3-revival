#!/usr/bin/env python3
"""Builds engine/anime-cards.json, the reviewed map from the client's cards to their anime versions (issue #60).

Input (committed): engine/anime/cards.tsv, one row per card of the game: its Konami id, name, EDOPro passcode (the
official card the engine plays, as engine/card-ids.json gives it), its optional anime_id (the EDOPro passcode of its anime
version in the pinned unofficial database; empty: the card has no anime version, or none we use), a status (Anime,
Anime / Normal, Reconstruction for our own record and script; Held for a card waiting on research; Kept for a card whose
upstream anime entry is equivalent or another card) and a note. Nothing is derived from upstream aliases: the table is the
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
NAMED_BITS = sum(TYPE_NAMES)
SCRIPT_FOLDERS = ["official", "pre-errata", "pre-release"]
# engine/anime/cards.tsv: its columns, and the statuses of a card with an anime version (an anime_id) or one under review.
TABLE = os.path.join(ENGINE, "anime", "cards.tsv")
COLUMNS = ["cid", "name", "edopro_id", "anime_id", "status", "note"]
WITH_ANIME = {"Anime", "Anime / Normal", "Reconstruction", "Manga"}  # Manga: the manga version, the owner's choice
STATUSES = WITH_ANIME | {"Held", "Kept", ""}
# The longest card text the connection DLL shows (YO3_PROFILE_TEXT_CHARS in tools/mod/card-profile-logic.h).
DLL_TEXT_CHARS = 4096


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


def table():
    """engine/anime/cards.tsv as dicts, checked: every Konami id once, a known status, and an anime_id exactly when the status
    is one of WITH_ANIME."""
    lines = read(TABLE).splitlines()
    if lines[0].split("\t") != COLUMNS:
        sys.exit("engine/anime/cards.tsv: the header must be " + "\t".join(COLUMNS))
    rows, seen, bad = [], set(), []
    for n, line in enumerate(lines[1:], 2):
        r = dict(zip(COLUMNS, (line.split("\t") + [""] * 6)[:6]))
        if not r["cid"].isdigit() or int(r["cid"]) in seen:
            bad.append("line %d: Konami id %r missing or twice" % (n, r["cid"]))
            continue
        seen.add(int(r["cid"]))
        if r["status"] not in STATUSES:
            bad.append("line %d: unknown status %r" % (n, r["status"]))
        if bool(r["anime_id"]) != (r["status"] in WITH_ANIME) or (r["anime_id"] and not r["anime_id"].isdigit()):
            bad.append("line %d (%s): anime_id %r with status %r" % (n, r["name"], r["anime_id"], r["status"]))
        rows.append(r)
    if bad:
        sys.exit("engine/anime/cards.tsv:\n  " + "\n  ".join(bad))
    return rows


def unofficial_card(code):
    """The pinned unofficial database's row of an anime passcode, or our corrected record of it: name, text, type, alias."""
    rec = records().get(code)
    if rec:
        return {"name": rec["name"], "text": rec.get("text", ""), "type": rec["type"], "alias": rec["alias"]}
    db = sqlite3.connect(os.path.join(ENGINE, "vendor", "BabelCDB", "cards-unofficial.cdb"))
    row = db.execute("select t.name, t.desc, d.type, d.alias from datas d join texts t on t.id = d.id where d.id = ?", (code,)).fetchone()
    return row and {"name": row[0], "text": row[1], "type": row[2], "alias": row[3]}


def card_report(query, data):
    """--card: one card's row, what the map decided and why, the script that plays it and its text, as lines."""
    q = query.strip().lower()
    rows = [r for r in table() if r["cid"] == q or q in r["name"].lower()]
    exact = [r for r in rows if r["name"].lower() == q]
    rows = exact or rows
    if not rows:
        return ["No card of the game is named like %r." % query]
    if len(rows) > 8:
        return ["%d cards match %r: %s, ... Be more specific." % (len(rows), query, ", ".join(r["name"] for r in rows[:8]))]
    entries = {e["cid"]: e for e in data["cards"]}
    out = []
    for r in rows:
        out.append("%s (Konami id %s): EDOPro %s, anime_id %s, status %s" % (r["name"], r["cid"], r["edopro_id"] or "none",
                   r["anime_id"] or "none", r["status"] or "none"))
        if r["note"]:
            out.append("  note: " + r["note"])
        e = entries.get(int(r["cid"]))
        if not e:
            out.append("  plays and shows as the game's card")
        else:
            out.append("  map: %s" % e["status"] + ("; script %s" % e["script"] if e.get("script") else ""))
            for why in [e.get("reason")] + e.get("holds", []):
                if why:
                    out.append("  why: " + why)
            for shown in e.get("profile", []):
                out.append("  the game shows: " + shown)
            card = e.get("anime") and unofficial_card(e["anime"])
            if card:
                units = len(card["text"].encode("utf-16-le")) // 2
                out.append("  anime text: %d characters%s" % (units, " (over the connection DLL's %d: the game would show its own)" % DLL_TEXT_CHARS if units > DLL_TEXT_CHARS else ""))
                out.append("    " + card["text"].replace("\r", "").replace("\n", "\n    "))
    return out


def compare_report(edopro):
    """--compare: each anime card the map uses whose name, text or script differs between the pinned data and an EDOPro install
    (its `expansions/cards-unofficial.cdb` and `script/unofficial/`), as lines."""
    db = os.path.join(edopro, "expansions", "cards-unofficial.cdb")
    if not os.path.isfile(db):
        sys.exit("%s has no expansions/cards-unofficial.cdb: give EDOPro's app folder" % edopro)
    theirs = sqlite3.connect(db)
    version = os.path.join(edopro, "..", "VERSION")
    out = ["EDOPro %s against the pins (scripts %s, database %s)" % (read(version).strip() if os.path.isfile(version) else "(version unknown)",
           pins()["scripts"][:7], pins()["database"][:7])]
    same = 0
    for r in table():
        if not r["anime_id"]:
            continue
        code = int(r["anime_id"])
        ours = unofficial_card(code)
        row = theirs.execute("select t.name, t.desc from texts t where t.id = ?", (code,)).fetchone()
        diffs = []
        if code in records():
            diffs.append("ours is our own record (engine/anime/records.json)")
        elif not row:
            diffs.append("EDOPro has no entry %d" % code)
        else:
            if row[0] != ours["name"]:
                diffs.append("name: EDOPro %r, ours %r" % (row[0], ours["name"]))
            if row[1].replace("\r", "") != ours["text"].replace("\r", ""):
                diffs.append("text differs")
        mine = os.path.join(ENGINE, "vendor", "CardScripts", "unofficial", "c%d.lua" % code)
        their = os.path.join(edopro, "script", "unofficial", "c%d.lua" % code)
        if os.path.isfile(os.path.join(ENGINE, "overrides", "c%d.lua" % code)):
            diffs.append("ours plays engine/overrides/c%d.lua" % code)
        elif os.path.isfile(mine) and os.path.isfile(their) and read(mine).rstrip() != read(their).rstrip():
            diffs.append("script differs (diff %s %s)" % (their, os.path.relpath(mine)))
        if diffs:
            out.append("%s (%s, anime %d): %s" % (r["name"], r["cid"], code, "; ".join(diffs)))
        else:
            same += 1
    out.append("%d anime cards the same in both" % same)
    return out


def candidates():
    """--candidates: anime cards in the pinned unofficial database that may be a game card's anime version while its row has no
    anime_id: the same name (" (Anime)" aside) or an alias to its EDOPro passcode. A passcode the table already uses or a
    row's note names (reviewed as another card) is left out. Run after moving the pins (engine/fetch.sh --update)."""
    rows = table()
    used = {r["anime_id"] for r in rows if r["anime_id"]}
    db = sqlite3.connect(os.path.join(ENGINE, "vendor", "BabelCDB", "cards-unofficial.cdb"))
    # Anime cards, not the Action Cards (0x10000000) or Skills (0x8000000) of later series, which share names with real cards.
    anime = db.execute("select d.id, d.alias, t.name from datas d join texts t on t.id = d.id where d.ot & 4 and d.type & 0x18000000 = 0").fetchall()
    by_name, by_alias = {}, {}
    for code, alias, name in anime:
        if str(code) in used:
            continue
        base = re.sub(r"\s*\(anime\)$", "", name, flags=re.I).strip().lower()
        if base == name.strip().lower() and name.endswith(")"):
            continue  # another medium's version: (Manga), (VG), (Pre-Errata)...
        by_name.setdefault(base, []).append((code, name))
        if alias:
            by_alias.setdefault(alias, []).append((code, name))
    out = []
    for r in rows:
        if r["anime_id"]:
            continue
        found = by_name.get(r["name"].lower(), []) + (by_alias.get(int(r["edopro_id"]), []) if r["edopro_id"] else [])
        found = [(c, n) for c, n in dict(found).items() if str(c) not in r["note"]]
        for code, name in found:
            out.append("%s (%s): %s %d" % (r["name"], r["cid"], name, code))
    return out


def official_passcodes(alias):
    """Konami id (or an added card's stand-in id) -> the database's original passcode, as crates/yo3-engine's Cards::open
    picks it."""
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
    # The cards the game never had, by the id the client knows them by: a newer card's stand-in (docs/added-cards.md).
    added = os.path.join(ENGINE, "added-cards.json")
    if os.path.exists(added):
        out.update({c["cid"]: c["passcode"] for c in json.loads(read(added))["cards"]})
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
    # Only the type bits the client has a name for: an anime-only mark (the anime Plus and Minus monsters' bits) is no
    # difference the client could show.
    rest = (ot ^ at) & ~EXTRA & ~7 & NAMED_BITS
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
# An effect the Deck triggers (a chain window offers the card in the Deck: Theinen's EVENT_LEAVE_FIELD), released one card and
# one effect code at a time, never as a class: each entry is a card whose offer, accept and decline were proven end to end (the
# engine offers it, the translator turns it into the client's chain list, the costs and placement are right, no hidden Deck card
# is shown to the opponent, and the real client under Wine took both answers). Thousand Dragon and Time Wizard are default-retained.
#   6089 Theinen the Great Sphinx: tests/anime_cards.rs `theinen_*`, tests/translate_contract.rs `a_deck_trigger_*`, docs/features.md
DECK_TRIGGERS = {6089: {"EVENT_LEAVE_FIELD"}}


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
    rows = table()
    # The table's EDOPro passcode is the one the engine plays for the Konami id (crates/yo3-engine's Cards::open).
    wrong = ["%s (%s): %r, the pins say %s" % (r["name"], r["cid"], r["edopro_id"], passcode.get(int(r["cid"])) or "none")
             for r in rows if r["edopro_id"] != str(passcode.get(int(r["cid"])) or "")]
    if wrong:
        sys.exit("engine/anime/cards.tsv: edopro_id differs from engine/card-ids.json:\n  " + "\n  ".join(wrong))
    for r in rows:
        if r["status"] not in WITH_ANIME | {"Held"}:
            continue
        cid, name, anime, label, note = int(r["cid"]), r["name"], r["anime_id"], r["status"], r["note"]
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
                if rec.get("upstream_script"):
                    entry["upstream_record"] = True
            else:
                arow = card(unofficial, entry["anime"])
            entry["alias"] = arow[6]
            script = None
            for folder in ["unofficial"]:
                p = os.path.join(vendor, "CardScripts", folder, "c%d.lua" % entry["anime"])
                if os.path.exists(p) and (not rec or rec.get("upstream_script")):
                    script = "unofficial/c%d.lua" % entry["anime"]
            if rec and not rec.get("upstream_script") and os.path.exists(os.path.join(ENGINE, "overrides", "c%d.lua" % entry["anime"])):
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
                unverified = [c for c in deck_effects(text) if c not in DECK_ROUTES and c not in DECK_TRIGGERS.get(cid, ())]
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

### Skill Drain and the Gods

See above: not a reconstruction but a fix (`engine/overrides/c82732705.lua`); it applies to the six manga Gods.

### The Gods' manga versions

Obelisk the Tormentor, Slifer the Sky Dragon and The Winged Dragon of Ra, and the Wicked Gods (Dreadroot, Avatar and Eraser) play
ProjectIgnis' manga scripts (513000134-139), labelled `[Manga]` in the profile. Each has a Divine Hierarchy, a rule of the duel rather than
an effect of the card, so Skill Drain does not take it: Ra and Avatar are 2, the others 1. A God is unaffected by Spell/Trap effects
that would make it leave the field and by other monsters' effects, except a monster with the same or a higher Hierarchy; it is not destroyed
by battle with a monster of a lower Hierarchy (an ordinary monster is 0), and its controller takes no damage from that battle; Special
Summoned, it goes back where it came from at the End Phase, except a Ra that The Winged Dragon of Ra - Sphere Mode summons (a house rule:
it is Ra out of its sphere form, `proc_unofficial.lua`), which also keeps Sphere Mode's 4000 ATK/DEF past the 1-turn limit on other cards' effects. Their own effects: Obelisk's Soul Energy Max (in the Battle Phase, Tribute
two monsters: it attacks with infinite ATK, damage of a hundred times the opponent's LP); Slifer's ATK of 1000 per card in the hand;
Ra's the Tributes' total; Dreadroot halves every other monster's ATK and DEF but a Divine-Beast's; Avatar is 1 stronger than the strongest
other monster; Eraser is 1000 per card the opponent controls and takes every monster with it when it goes. The Wicked Gods' rows keep the
game's Type and Attribute; the profile shows them as the manga cards' (DIVINE, Divine-Beast).

Infinite ATK (Obelisk's Soul Energy Max, the anime Geh) is one value to the engine: any ATK, DEF or LP of 9,999,999 or more
is ∞, and finite changes leave it (below), and the Hierarchy settles every meeting
(`tests/manga_gods.rs`): Avatar matches infinity rather than going 1 above it, a tie that Avatar survives and Obelisk
does not; Ra can't be destroyed by Soul Energy Max, nor Obelisk by Geh, and their controllers take no damage from it; Geh,
with no Hierarchy, is destroyed in a tie with Obelisk. Ring of Destruction can't destroy a God, so it deals no damage;
the anime Geh it destroys deals its infinite ATK to both players at once, a draw. ∞ is one value (`crates/yo3-engine/src/guards.lua`,
rules of ours, `tests/infinite_lp.rs`): any LP, ATK or DEF of 9,999,999 or more becomes exactly ∞, and a finite cost,
damage, gain or ATK change leaves it as it is (Premature Burial's 800, Dian Keto's 1000, Axe of Despair, Shadow Spell,
Megamorph's half); ∞ against ∞ is a tie. An amount that is itself ∞ moves as is: Mystik Wok on Geh gives ∞ LP, Solemn
Judgment's or Riryoku's half of ∞ leaves ∞ (and Riryoku's monster gains ∞), Yu-Jo Friendship gives both players ∞, Ra
revived pays all but 1 and has ∞ ATK, De-Fusion on it gives ∞ back, Granel's ATK is the ∞ LP. "LP becomes X" is X (Life
Equalizer, the anime Rasca, Self-Destruct Button; Reversal Quiz swaps ∞). Infinite damage is infinite: the scripts' own
"100 times the LP" (Soul Energy Max, Geh's attack), effect damage of 9,999,999 or more (Ring of Destruction on Geh) and
battle damage from a monster with infinite ATK (Avatar risen to infinity), directly or through a monster with finite
ATK, all take every LP a player has, ∞ LP too; the core's 32 bits once wrapped the largest into a gain. Half of infinity
is still infinite: Dreadroot halves every other monster but an infinite ATK or DEF (`engine/overrides/c513000137.lua`),
so Geh under it keeps its infinite damage and Avatar ties it at infinity.
The anime Geh's infinity is printed (its card says "ATTACK ∞"): it is the card's ATK, not an effect, though the script can
only give it as ATK "?" and an effect. So Skill Drain negates Geh's effects but leaves its infinite ATK, and other cards'
effects that change ATK (The Seal of Orichalcos, Axe of Despair, Shrink, Shield & Sword) don't touch it. Its original ATK
is infinite too, so the field shows its ∞ in white and an effect's ∞ (Soul Energy Max, Avatar) in blue (`guards.lua`'s
list of printed infinities, a rule of ours; [adding cards](added-cards.md#infinite-atk-and-lp)).
A God's control can't switch (Change of Heart and Creature Swap find no target), the opponent can't Tribute it (Sphere
Mode takes other monsters) and Shrink halves it for the turn. Book of Moon turns it face-down, as the manga text only keeps
a God from Spell/Trap effects that would make it leave the field; but face-down it is still a God (its Hierarchy is the
card's, as against Skill Drain: Raigeki Break still can't destroy it), and as other cards' effects apply to it for 1 turn,
it is face-up in Attack Position again from the end of that turn. `tests/duel_check.rs` `duels_with_the_gods` self-plays
random decks of the Gods, Geh's Orichalcos line and these cards.

One fix to the shared procedure (`engine/overrides/proc_unofficial.lua`): the upstream Hierarchy check compared a God with a monster
without a Hierarchy as if that monster outranked it, so a stronger ordinary monster destroyed a God by battle. A monster without one is
now Hierarchy 0. Infinite ATK shows on the field as ∞ and ∞ LP in the LP disc (the connection DLL's `duel-numbers.h`)."""


def doc(data):
    """docs/anime-cards.md: every disposition in words, for players and hosts."""
    mapped = [c for c in data["cards"] if c["status"] == "mapped"]
    held = [c for c in data["cards"] if c["status"] == "held"]
    default = [c for c in data["cards"] if c["status"] == "default"]
    shown = [c for c in mapped if c.get("profile")]
    releases = release_report(data)
    out = ["# Anime card versions", "",
           "<!-- Made by tools/engine/anime-map.py from engine/anime/cards.tsv: do not edit by hand. -->", "",
           "With the `anime-cards` setting on (a host sets it; a single player turns on **Anime cards** in Desktop's Settings → Singleplayer: [playing](playing.md#anime-card-versions)), the %d cards below play by their"
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
           "| Card display | The anime card's effect text under an **[Anime]** line and, when you provide it, its artwork ([below](#artwork)) |",
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
           "**Which picture a card shows**, first wins: a picture in the player's `pics` folder; a face the player put in `anime-art/card` by hand; a picture the project ships; a downloaded picture; else the game's own artwork (a card the game never had: the stand-in, below). Everything happens on the player's own PC and needs no server (Singleplayer works alone).",
           "",
           "- **The `pics` folder, as EDOPro has it** (`crates/yo3-launcher/src/art/pics.rs`): a card's picture named by its passcode, `pics/<passcode>.jpg` or `.png`, any size up to 4096 pixels a side and 24 MB, the shape of a card (EDOPro's 177x254 and 813x1185 pictures, a scan, a drawing of your own). The folder is `pics` beside `anime-art` in the launcher's data folder (`%LOCALAPPDATA%\\YO3 Revival\\pics`, `~/.local/share/yo3-revival/pics`), or beside `yo3.exe`; EDOPro's own `pics` folder can be copied in as it is. At each start the launcher makes each picture the game's 200x290 face and its thumbnail, for every card of the picture list (the anime and manga versions and the cards the game never had), by the card's anime passcode first and then its official one. The faces made so are listed in `anime-art/card/.pics` with their picture's size and time: a changed picture is made again, and a face whose picture was taken away is removed. A picture that can't be used (not a PNG or JPEG, not a card's shape) is refused and logged, and the card keeps the face it had.",
           "- **Pictures the project ships**: the owner's own artwork for the manga Gods (Obelisk, Slifer, Ra, Dreadroot, Avatar, Eraser) and the anime Divine Serpent Geh, scaled to 200x290 by `tools/engine/shipped-pictures.py` (`--check`; the originals stay with their author) into `crates/yo3-launcher/assets/pics/` and built into the launcher. They take the place of a downloaded picture, never of the player's own.",
           "- **Get pictures** (Desktop, Settings → Game → [Card pictures](launcher.md#card-pictures)): on the player's click, or by itself when the player left **Download automatically** on (setup's **Download card pictures** tick, ticked by default; off for a launcher set up before it), Desktop downloads about 10 MB from ProjectIgnis' picture server (`https://pics.projectignis.org:2096/pics/<passcode>.jpg`, the same pictures EDOPro shows), one card at a time with 0.4 s between requests, a generic `User-Agent` that says only that it is YO3 Revival and nothing about the player, timeouts, three tries per picture (a failed request or a picture that cannot be read, waiting 0.4, 2 and 4 s), and **Cancel** at any moment; the download goes on while the player leaves Settings. Which passcode each card is asked by comes from the reviewed map: `engine/anime/pictures.tsv` (one row per card: Konami id, anime passcode, official passcode), generated from `engine/anime-cards.json` by `tools/engine/anime-pictures.py` (`--check` compares) and compiled into Desktop; the three Gods are fetched by ProjectIgnis' anime passcodes like any card. The cards the game never had are in the same list, by their official passcode (or their anime one when they have an anime version). When the server has no anime picture of a card (a 404) the official card's normal picture is fetched instead (Damage Polarizer, Dark Strike Fighter, Fortune Lady Fire; Temple of the Kings has no anime passcode, so its official picture). Each JPEG is checked before it is decoded (a JPEG, at most 2 MB, between 32 and 2048 pixels, the shape of a card) and then made the game's 200x290 face (bilinear enlargement from the server's 177x254) and its 50x72 thumbnail; anything else is refused and that card keeps the game's artwork. A second click only fetches the cards that still have no picture.",
           "- **Add pictures from a file** takes a `.zip` in the folder's own layout, `card/<Konami id>.png` (200x290) and optionally `mini/<Konami id>.png` (50x72), at the root or inside the zip's single top folder; **Save pictures to a file** writes the folder as such a zip, so a player can pass theirs to a friend or keep them if the server ever goes away. Only those names are read (everything else is ignored and counted, a name that leaves the folder is refused), each entry has a size cap whatever the zip claims (1 MB each, 96 MB in all, 4000 entries), and each picture must be a PNG of exactly the size the folder wants.",
           "",
           "**What a card without a picture does.** A card the game never had (#75) has no artwork of the game's to fall back on: until its picture arrives it shows a stand-in, an *Artwork not found* card face that ships with the launcher (`crates/yo3-launcher/assets/no-picture.png`, made by `tools/engine/no-picture.py`, the project's own picture; `added.rs` writes it and its thumbnail into the added cards' folder at each start, and the card's own picture replaces it at the next start after it arrives). For any other card, a picture that is not there for any reason (not downloaded yet, cancelled, the server unreachable or without it, a file refused, or a download that finishes while a game is already open) leaves that card on **the game's own artwork, with the profile's stats and text unchanged**: never a blank or broken picture. Pictures are written whole or not at all (a temporary name, then renamed), so the game never reads a half-written file, and a partly finished download is simply a folder with some cards' pictures (`art.rs`, `art/get.rs` `a_partial_download_leaves_whole_pictures_and_cards_that_keep_the_games_art`). The game reads the folder when it starts, so pictures that arrive while it is open show after it is closed and PLAY is pressed again.",
           "",
           "**Never over your own pictures.** Desktop lists the faces its download made in `card/.downloaded`. A face not listed there is the player's own, and neither a later download nor a zip import replaces it (an import reports how many it kept); a face Desktop downloaded is replaced by a picture from a file. Thumbnails keep the existing rule below.",
           "",
           'The folder and its files:',
           "",
           "- `anime-art/card/<Konami id>.png`: the whole card face as the anime version reads (frame, name, picture, text, stats), a 200x290 PNG like the game's own `data/card/<id>_<art>.png`. One file per Konami id serves all of that card's artworks (Flame Swordsman's two). A player may also copy such files in by hand.",
           "- `anime-art/mini/<Konami id>.png`: optional, the 50x72 thumbnail like `data/mini/<id>_<art>.png` (Edit Deck lists, the duel's field). When it is missing the launcher makes it from the face (the thumbnails it made are listed in `mini/.derived`, made again when the face changes, and never replace one you provided).",
           "- The folder is `%LOCALAPPDATA%\\YO3 Revival\\anime-art` on Windows and `~/.local/share/yo3-revival/anime-art` on Linux (the launcher's data folder; `$YO3_DESKTOP_DATA/anime-art` when that is set). The launcher creates it, passes it to the game as `YO3_ANIME_ART` (and writes it into the direct-start profile) and logs how many faces it found and which files it skipped (not a PNG, not 200x290). Games started without the launcher look in `anime-art` beside `yo3.exe`.",
           "- A host's players each get their own pictures: the server never sends artwork.", ""]
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
            " and Thousand Dragon, reviewed together, and Temple of the Kings), or held for a technical fix (none now). Power Bond (it stays Power Bond and"
            " is treated as Polymerization), Multiply and Cloning (the copy shows as the monster itself), the Sphinxes that summon themselves from the"
            " Deck, Theinen the Great Sphinx (its trigger from the Deck: offered, accepted and declined, in tests and in the real client) and Clear Vice Dragon and"
            " Blackwing - Aurora the Northern Lights (the game\'s \"?\" ATK kept by a corrected record, [below](#upstream-scripts-with-the-games-)) are released, each with a test in `tests/anime_cards.rs`.", "",
            "**Source of truth for effects:** EDOPro/ProjectIgnis's anime scripts at the pinned commit. Effects are not re-checked against the"
            " episodes; a disagreement is fixed upstream or with a documented override in `engine/overrides/`. Three exceptions (#60): cards for which"
            " ProjectIgnis has only a placeholder (Dark Strike Fighter and Fortune Lady Fire) are researched and get our own record and script"
            " ([below](#reconstructions)); the six Gods play their manga versions, not the anime ones (the owner's choice, [below](#the-gods-manga-versions));"
            " and Skill Drain against the Gods was a bug, fixed by an override of Skill Drain"
            " (`engine/overrides/c82732705.lua`, which changes only how it treats the Gods; every other monster is as before). The Gods are"
            " unaffected by most effects, but Skill Drain, when activated, negates the effects of the Gods already on the field until the end of"
            " that turn; then they come back, though Skill Drain stays face up. Ra's ATK (the Tributes' total) is 0 for the rest of that turn and the"
            " total again from the next one. A God summoned while Skill Drain is already face up is not affected at all. Each activation counts"
            " on its own, as the Gods' text says other cards' effects apply to them for 1 turn only: a second copy, or the same card activated"
            " again, negates the Gods on the field for that turn, and a God that leaves the field and comes back is a new card, unaffected by the"
            " Skill Drain already face up and negated by the next activation. The end-of-turn recovery is the engine's reset of a marker on the God, not an effect, so no negation can hold it back."
            " ProjectIgnis' own Skill Drain negates the Gods for as long as it stays face up, which their text rules out. With the"
            " setting off, the official Gods and Skill Drain are the game's. Each point has a real-core test in `tests/anime_reconstructions.rs` and `tests/manga_gods.rs`.", "",
            "## Reconstructions", "",
            "Where ProjectIgnis has no usable script, the data record and the script are ours (`engine/anime/records.json`, `engine/overrides/`) and"
            " the card's text in the game says it is a reconstruction. Everything else about the card (the engine, the profile, the duel check,"
            " the deck placement) is as for any other mapped card; the official (standard) card, its record and its script do not change.", "",
            "| Card | Anime card | Text the game shows | Basis |", "| --- | --- | --- | --- |"]
    for r in data["records"]:
        if not r.get("upstream_script"):
            out.append("| %s | %d | %s | %s |" % (r["name"].replace(" (Anime)", ""), r["code"], r["text"].replace("|", "/").replace("\n", "<br>"), r["basis"].replace("|", "/")))
    out += ["", RECONSTRUCTION_DETAILS, "",
            "### Upstream scripts with the game's \"?\"", "",
            "Clear Vice Dragon and Blackwing - Aurora the Northern Lights play by ProjectIgnis' scripts, unchanged. Their pinned database rows spell the"
            " card's \"?\" ATK as 0, where the game shows \"?\" (-2, as the official card has it), and the profile has no way to show a 0 as \"?\". So the"
            " row is corrected to the game's \"?\" in our record and the script is the upstream one (the record's `upstream_script`): the card keeps the"
            " game's \"?\" in every client panel, and the core plays a \"?\" base ATK as 0 on the field, exactly what the 0 would give. Neither script sets"
            " an ATK of its own: Clear Vice Dragon doubles its target's ATK in the Damage Step, Aurora copies the Synchro Monster it banishes (base ATK included).", "",
            "| Card | Anime card | Corrected row |", "| --- | --- | --- |"]
    for r in data["records"]:
        if r.get("upstream_script"):
            out.append("| %s | %d | ATK ? (-2), DEF %d, Level %d |" % (r["name"].replace(" (Anime)", ""), r["code"], r["def"], r["level"]))
    out += ["",
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
    if not held:
        out.append("| None now | Every reviewed card is enabled or retained on purpose. |")
    rows = table()
    out += ["", "## Kept on purpose", "", "| Card | Why |", "| --- | --- |"]
    for r in sorted((r for r in rows if r["status"] == "Kept"), key=lambda r: r["name"].lower()):
        out.append("| %s | %s |" % (r["name"], r["note"].replace("|", "/")))
    others = [r for r in rows if not r["status"]]
    out += ["", "Every other card of the game (%s of %s in `engine/anime/cards.tsv`) has no anime_id: no anime version, or none we use,"
            " and keeps its current behaviour." % (format(len(others), ","), format(len(rows), ","))]
    noted = sorted((r for r in others if r["note"]), key=lambda r: r["name"].lower())
    if noted:
        out += ["", "| Card | Note |", "| --- | --- |"] + ["| %s | %s |" % (r["name"], r["note"].replace("|", "/")) for r in noted]
    out.append("")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="fail if engine/anime-cards.json is not what the table builds")
    ap.add_argument("--releases", action="store_true", help="print how many held cards each category of the card profile would release")
    ap.add_argument("--list", action="store_true", help="print the held cards and why")
    ap.add_argument("--card", metavar="NAME", help="print one card's row (a Konami id or part of a name): its status, why, script and anime text")
    ap.add_argument("--compare", metavar="EDOPRO_APP", help="list the anime cards whose name, text or script differ from an EDOPro install's (its app folder)")
    ap.add_argument("--candidates", action="store_true", help="list anime cards that may be a game card's anime version while its row has no anime_id; exit 1 if any")
    args = ap.parse_args()
    if args.compare:
        print("\n".join(compare_report(args.compare)))
        return
    if args.candidates:
        found = candidates()
        print("\n".join(found) if found else "No candidates: every same-named or aliased anime card is in the table or noted.")
        sys.exit(1 if found else 0)
    if args.card:
        print("\n".join(card_report(args.card, json.loads(read(os.path.join(ENGINE, "anime-cards.json"))))))
        return
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
