#!/usr/bin/env python3
"""Builds engine/added-cards.json: the cards the game never had, each a complete row of the client's card table (#75).

    tools/engine/added-cards.py            write engine/added-cards.json
    tools/engine/added-cards.py --check    compare the committed file with a fresh build (exit 1 when it differs)
    tools/engine/added-cards.py --find CARD [CARD...]
                                           whether each card can be added, and the line for engine/added/cards.tsv
                                           when it can (rows continue after the table's). CARD is its English name
                                           (any case), its passcode, or `cid:N` (its Konami id)
                                           `anime:N` (an anime-only card's passcode in the unofficial database)
    tools/engine/added-cards.py --find-file FILE   the same for a file of cards, one per line (`#` comments)
    tools/engine/added-cards.py --add CARD [CARD...] --note TEXT [--checked]
                                           add them: each line with the note at the end of engine/added/cards.tsv, a
                                           printed card's line in engine/anime/cards.tsv (in id order), then this build,
                                           tools/engine/anime-map.py and tools/engine/anime-pictures.py. All or nothing:
                                           a card that can't be added, or one whose script --find notes (below) before
                                           you read it and pass --checked, writes nothing
    tools/engine/added-cards.py --add-file FILE --note TEXT [--checked]   the same for a file of cards

--find also reads the script each card would play (our override, else ProjectIgnis') and notes the lines that use a
mechanic the game never had (Xyz Materials or Monsters, Link, Pendulum Zones and Scales, the Extra Monster Zone; "cannot
be used as Xyz/Link Material" is left alone), or a missing script: the client may show such a card, but the duel can't
play it as written. Read the lines; drop the card, or add it with --checked when they are harmless (a filter that only
skips Xyz Monsters). It also points out, without holding the card back, a script that sets LP, works an amount out
from LP or makes an ∞ of its own: give such a card a case at ∞ in crates/yo3-server/tests/infinite_lp.rs.

Input (committed): engine/added/cards.tsv, one line per added card: its permanent `row` (4834 on, after the game's last;
never reused or reordered: decks and saves store rows), its Konami id, its `stand_in` id (below), its `rarity` (below,
empty for Common), `anime` (an anime-only card's passcode, below), its name (for reading the table) and a note.

An **anime-only** card (one never printed: Orichalcos Gigas) has no Konami id: its `anime` column names its passcode in
the pinned unofficial database (marked Anime there, `ot` 4), it always has a stand-in, and it needs a working script in
`unofficial/` (one that registers effects, not a placeholder). Its text starts with the `[Anime]` line. The list marks it
`anime_only`: the server refuses every duel with it while the `anime-cards` setting is off, whatever the banlist, and the
engine plays it only with the setting on.

The client keeps a card's Konami id in 14 bits (16383 at most) and looks cards up from its lowest id (3900). A newer card
whose Konami id is beyond that gets a permanent stand-in: the lowest id from 3900 that no card has (not the game's, no
Konami card's in the pinned ids, no other added card's). The client, the server and the rules engine know the card by
that id (the JSON's `cid`; `konami_id` keeps the real one); --find picks it. Like a row, it is never changed. They are few:
--find gives none of the last STAND_IN_RESERVE free ones, and reports how many are left.

Each card's data comes from the pinned EDOPro database through the pinned Konami ids (engine/card-ids.json), never typed
by hand. The client's encoding of a row (`CARD_Prop`: word `a` = Konami id, ATK/10, DEF/10; word `b` = kind, attribute,
level, race, artwork; docs/client-internals.md, Card data) is learned from the game's own cards: every game card the
database also has gives its type, attribute and race next to the client's kind, attribute and race (and, for a Spell or
Trap, its subtype: Field, Equip, Quick-Play...), and an added card may only use values the game itself shows. So no Xyz,
Link or Pendulum card can be added (the client has no such kind), a stat beyond the nine bits (5110) is refused, and so
is a Konami id beyond the row's 14 bits (16383). `CARD_Genre` (Edit Deck's search by effect: Special Summon, Destroy monster, Pierce, Flip Effect...) is learned the
same way: for each of its bits, the features that mark it on the game's cards with at least 85% precision
(`learn_genre`): type flags, the effect words of the card's script (CATEGORY_SPECIAL_SUMMON, EFFECT_PIERCE...) and the
official database's effect categories (`datas.category`; an anime-only card is judged without them, as the unofficial
database numbers them its own way). Bits no such rule predicts
(Recover cards from Graveyard, Hand destruction...) stay unset: an added card is not found under them.

Rarity (word b bits 24-26: 0 Common, 4 Rare, 3 Super Rare, 2 Ultra Rare, as the game's pack files place the cards in
their tiers; 1 and 5 its top tiers; the new-card reveal's and the pack opening's glow, `_root.rale_bg`) is the table's
optional `rarity` column: `common` (empty), `rare`, `super` or `ultra`. Bits 0-2 have no reader in the client's card
code and are 0 on the game's newest rows; added cards leave them 0.

Needs the pinned data (engine/fetch.sh) and the game (YO3_GAME_DIR, else the default Wine path).
"""
import argparse
import collections
import contextlib
import csv
import importlib.util
import io
import json
import os
import re
import sqlite3
import struct
import subprocess
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..'))
ENGINE = os.path.join(ROOT, 'engine')
TABLE = os.path.join(ENGINE, 'added', 'cards.tsv')
ANIME_TABLE = os.path.join(ENGINE, 'anime', 'cards.tsv')
OUT = os.path.join(ENGINE, 'added-cards.json')
NPCS = os.path.join(ENGINE, 'added', 'npcs.json')
FORMAT = 'yo3-added-cards'
STAT_MAX = 0x1FE * 10  # nine bits of tens; 0x1FF is the client's "?"
FIRST_ROW = 4834


def game_dir():
    return os.environ.get('YO3_GAME_DIR') or os.path.expanduser('~/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3')


def ypk_file(name):
    """One file from the game's archives (tools/re/ypk.py's reader)."""
    spec = importlib.util.spec_from_file_location('ypk', os.path.join(ROOT, 'tools', 're', 'ypk.py'))
    ypk = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ypk)
    package = os.path.join(game_dir(), 'package')
    for archive in sorted(os.listdir(package)):
        if archive.endswith('.ypk'):
            for entry, method, size, _, _, data, offset in ypk.entries(os.path.join(package, archive)):
                if entry == name:
                    raw = data[offset:offset + size]
                    return raw if method == 0 else zlib.decompress(raw, -15)
    raise SystemExit(f'{name} is not in the game\'s archives ({package})')


def client_rows():
    """The game's CARD_Prop rows: (a, b) per row."""
    prop = ypk_file('data/bin#/CARD_Prop.bin')
    return [struct.unpack_from('<II', prop, i * 8) for i in range(len(prop) // 8)]


def passcodes(alias):
    """Konami id -> the database's original passcode, as crates/yo3-engine's Cards::open picks it (anime-map.py's rule)."""
    out = {}
    with open(os.path.join(ENGINE, 'card-ids.json')) as f:
        data = json.load(f)['data']
    for card in data:
        ids = list(dict.fromkeys([card['id']] + [i['id'] for i in card.get('card_images', [])]))
        known = [i for i in ids if i in alias]
        if not known:
            continue
        original = next((i for i in known if alias[i] == 0), known[0])
        for misc in card.get('misc_info', []):
            cid = misc.get('konami_id', 0)
            if cid and cid not in out:
                out[cid] = original
    return out


def fields(b):
    """The client's fields of a row's word b."""
    return {'kind': (b >> 3) & 0x1F, 'attribute': (b >> 8) & 0xF, 'level': (b >> 12) & 0xF, 'subtype': (b >> 16) & 7,
            'race': (b >> 19) & 0x1F, 'art': (b >> 27) & 7}


def learn(rows, data, passcode):
    """What the client calls each database value, from the game cards both have: {type: kind}, {attribute: attribute},
    {race: race} (monsters), {type: (kind, attribute, race, subtype)} (Spells and Traps), and the artwork number of a
    card's first print."""
    seen = {k: collections.defaultdict(collections.Counter) for k in ('kind', 'attribute', 'race', 'spell_trap')}
    arts = collections.Counter()
    for a, b in rows:
        cid = a & 0x3FFF
        code = passcode.get(cid)
        if not cid or code not in data:
            continue
        f, d = fields(b), data[code]
        arts[f['art']] += 1
        if not d['type'] & 1:
            # A Spell or Trap: its whole type (Spell, Quick-Play Spell, Counter Trap...) decides the client's kind,
            # attribute, race and subtype together (13 Spell with 8 and 23, 14 Trap with 9 and 24).
            seen['spell_trap'][d['type']][(f['kind'], f['attribute'], f['race'], f['subtype'])] += 1
            continue
        seen['kind'][d['type']][f['kind']] += 1
        seen['attribute'][d['attribute']][f['attribute']] += 1
        seen['race'][d['race']][f['race']] += 1
    learned = {k: {v: c.most_common(1)[0][0] for v, c in table.items()} for k, table in seen.items()}
    learned['art'] = min(arts) if arts else 1  # the lowest artwork number is a card's first print
    return learned


GENRE_PRECISION = 0.85
TYPE_TOKEN = 0x4000
# A copy Token's stand-in Type and Attribute (`encode`): Fiend, DARK.
COPY_TOKEN_RACE, COPY_TOKEN_ATTRIBUTE = 0x8, 0x20
RARITY = {'': 0, 'common': 0, 'rare': 4, 'super': 3, 'ultra': 2}


EFFECT_WORD = re.compile(r'\b(?:CATEGORY|EFFECT)_[A-Z_]+')
SCRIPT_FOLDERS = ['official', 'pre-errata', 'pre-release', 'unofficial']


def script_path(code):
    """The script the engine plays for `code`: our override, else ProjectIgnis' (crates/yo3-engine cards.rs' order)."""
    for path in [os.path.join(ENGINE, 'overrides', f'c{code}.lua')] + [
            os.path.join(ENGINE, 'vendor', 'CardScripts', folder, f'c{code}.lua') for folder in SCRIPT_FOLDERS]:
        if os.path.exists(path):
            return path
    return None


def script_text(code):
    path = script_path(code)
    if not path:
        return None
    with open(path, encoding='utf-8', errors='replace') as f:
        return f.read()


def script_words(code):
    """The effect words (CATEGORY_..., EFFECT_...) of the script the engine plays for `code`."""
    return set(EFFECT_WORD.findall(script_text(code) or ''))


# Mechanics the game never had, as ProjectIgnis' scripts name them. A line that only forbids using the card as Xyz or
# Link Material is harmless: Master Rule 2 never asks.
MECHANICS = [
    ('Xyz', re.compile(r'TYPE_XYZ|Xyz\.|Overlay|IsXyzLevel|IsRank|GetRank|REASON_XYZ|SUMMON_TYPE_XYZ')),
    ('Link', re.compile(r'TYPE_LINK|Link\.|GetLinked|LinkMarker|LINK_MARKER|IsLink|GetLink|REASON_LINK|SUMMON_TYPE_LINK')),
    ('Pendulum', re.compile(r'TYPE_PENDULUM|Pendulum\.|LOCATION_PZONE|Scale|SUMMON_TYPE_PENDULUM')),
    ('the Extra Monster Zone', re.compile(r'ExtraMonsterZone|InExtraMZone|LOCATION_EMZONE|IsInMainMZone')),
]
HARMLESS = re.compile(r'EFFECT_CANNOT_BE_(?:XYZ|LINK)_MATERIAL')


def mechanics(text):
    """[(mechanic, its line numbers)] in a script's `text`, in MECHANICS' order."""
    out = []
    lines = [l.split('--', 1)[0] for l in text.split('\n')]  # Lua comments name mechanics too
    for what, pattern in MECHANICS:
        hits = [i for i, l in enumerate(lines, 1) if pattern.search(l) and not HARMLESS.search(l)]
        if hits:
            out.append((what, hits))
    return out


# What plays differently against ∞ (crates/yo3-engine/src/guards.lua: any LP, ATK or DEF of 9,999,999 or more is one ∞
# value, which finite changes leave as it is). Not a reason to hold a card back, only one to give it a case in
# crates/yo3-server/tests/infinite_lp.rs: it sets LP, works an amount out from LP, or gives or takes an ∞ of its own.
INFINITY_SENSITIVE = [
    ('sets LP', re.compile(r'Duel\.SetLP')),
    ('works an amount out from LP', re.compile(r'Duel\.GetLP')),
    ('an ∞ of its own (9,999,999)', re.compile(r'9999999|math\.huge')),
]


def infinity_hints(line, ctx):
    """Non-blocking: what in the script of a card --find approved deserves a case at ∞ LP or ATK, and, for a card whose
    database ATK is "?" with an ∞ in its script, whether its ∞ is printed (guards.lua's PRINTED_INFINITY)."""
    if not line:
        return []
    parts = line.split('\t')
    if parts[5]:
        return []  # a Token: no script of its own
    code = int(parts[4]) if parts[4] else ctx['passcode'].get(int(parts[1]))
    d = (ctx['anime'] if parts[4] else ctx['data']).get(code)
    text = script_text(code)
    if text is None:
        return []
    lines = [l.split('--', 1)[0] for l in text.split('\n')]
    rel = os.path.relpath(script_path(code), ROOT)
    out = []
    for what, pattern in INFINITY_SENSITIVE:
        hits = [i for i, l in enumerate(lines, 1) if pattern.search(l)]
        if hits:
            out.append(f'{what}: {rel} lines {", ".join(map(str, hits))}')
    if out:
        out.append('give it a case at ∞ in crates/yo3-server/tests/infinite_lp.rs (docs/added-cards.md, "Infinite ATK and LP")')
    if d is not None and d['atk'] == -2 and any(INFINITY_SENSITIVE[2][1].search(l) for l in lines):
        out.append('its ATK is "?" and its script makes an ∞: if the card itself prints ∞, add it to PRINTED_INFINITY in guards.lua')
    return out


def risk_hints(line, ctx):
    """Non-blocking: lines of the script of a card --find approved that match a pattern which broke duels
    (tools/engine/script-risks.py): a first card used unchecked after a cost, or an effect that widens a summon's
    materials (crates/yo3-server/tests/material_search.rs covers those)."""
    if not line:
        return []
    parts = line.split('\t')
    if parts[5]:
        return []  # a Token: no script of its own
    code = int(parts[4]) if parts[4] else ctx['passcode'].get(int(parts[1]))
    text = script_text(code)
    if text is None:
        return []
    import importlib.util
    spec = importlib.util.spec_from_file_location('script_risks', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'script-risks.py'))
    risky = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(risky)
    rel = os.path.relpath(script_path(code), ROOT)
    advice = {'first card unchecked': 'if its cost can leave nothing to choose, the effect stops on a Lua error: read them, '
                                      'and write an override with a check (engine/overrides/) when it can happen',
              'widens materials': 'tests/material_search.rs puts it on a crowded board: run it (`task check:added` does)'}
    return [f'{what}: {rel} lines {", ".join(map(str, lines))}; {advice[what]}' for what, lines in risky.risks(text)]


def notes(line, ctx):
    """What to read in the script of a card --find approved (its cards.tsv `line`): mechanics the game never had, or no
    script at all (a Normal Monster needs none)."""
    if not line:
        return []
    parts = line.split('\t')
    if parts[5]:
        return []  # a Token: no script of its own
    if parts[4]:
        code, d = int(parts[4]), ctx['anime'].get(int(parts[4]))
    else:
        code = ctx['passcode'].get(int(parts[1]))
        d = ctx['data'].get(code)
    text = script_text(code)
    if text is None:
        normal = d is not None and d['type'] & 0x11 == 0x11 and not d['type'] & 0x20
        return [] if normal else [f'no script c{code}.lua: it would play with no effect']
    rel = os.path.relpath(script_path(code), ROOT)
    return [f'{what}: {rel} lines {", ".join(map(str, hits))}' for what, hits in mechanics(text)]


def features(d, code, category=True):
    """What a genre rule may test: the card's type flags, its script's effect words and (`category`) the official
    database's effect categories. The unofficial database numbers its categories its own way, so an anime-only card is
    judged without them."""
    out = {('type', bit) for bit in range(32) if d['type'] >> bit & 1} | script_words(code)
    if category:
        out |= {('category', bit) for bit in range(64) if d['category'] >> bit & 1}
    return out


def game_cards(rows, genres, data, passcode, category=True):
    """(the game's genre word, the features) for each game card the database has."""
    cards = []
    for (a, _), genre in zip(rows, genres):
        code = passcode.get(a & 0x3FFF)
        if a & 0x3FFF and code in data:
            cards.append((genre, features(data[code], code, category)))
    return cards


def learn_genre(cards):
    """{genre bit: [feature, ...]}: a card gets the genre bit when it has any of the listed features. Learned from
    `cards` (genre word, features): features chosen greedily, each marking the genre bit with at least GENRE_PRECISION
    on the cards it marks and adding at least 2% (and 3 cards) of the genre bit's cards."""
    every = sorted(set().union(*(f for _, f in cards)), key=str) if cards else []
    marks = {f: sum(1 << i for i, (_, fs) in enumerate(cards) if f in fs) for f in every}
    rules = {}
    for g in range(64):
        has = sum(1 << i for i, (genre, _) in enumerate(cards) if genre >> g & 1)
        if has.bit_count() < 5:
            continue
        chosen, covered = [], 0
        while True:
            best = None
            for f, m in marks.items():
                if f in chosen or m.bit_count() < 5 or (m & has).bit_count() < GENRE_PRECISION * m.bit_count():
                    continue
                gain = (m & has & ~covered).bit_count()
                if gain >= max(3, 0.02 * has.bit_count()) and (best is None or gain > best[0]):
                    best = (gain, f)
            if best is None:
                break
            chosen.append(best[1])
            covered |= marks[best[1]]
        if chosen:
            rules[g] = chosen
    return rules


def genre_of(feats, rules):
    """An added card's CARD_Genre word from its features, by the learned rules."""
    return sum(1 << g for g, fs in rules.items() if any(f in feats for f in fs))


def card_text(desc):
    """The card's text as the game shows it: CR line breaks, without the database's note that EDOPro's text is unofficial
    ("* The above text is unofficial and describes the card's functionality in the OCG."): no card of the game says so."""
    text = desc.replace('\r\n', '\r').replace('\n', '\r')
    lines = [l for l in text.split('\r') if not l.lstrip().startswith('* The above text is unofficial')]
    return '\r'.join(lines).rstrip('\r ')


def encode(d, cid, learned):
    """A database card as the client's row words, `cid` the id the client knows it by (its Konami id or stand-in): (a, b, the
    reasons it can't be one). A Spell or Trap has no level or
    stats; a monster's type, attribute and race each need a game card that has them."""
    why = []
    if not 0 < cid <= 0x3FFF:
        why.append(f'id {cid} does not fit the client\'s 14 bits')
    if not d['type'] & 1:
        st = learned['spell_trap'].get(d['type'])
        if st is None:
            why.append(f'no game card has its type ({d["type"]:#x}), so the client can\'t show it')
            return None, None, why
        kind, attribute, race, subtype = st
        return cid & 0x3FFF, kind << 3 | attribute << 8 | subtype << 16 | race << 19 | learned['art'] << 27, why
    if d['type'] & TYPE_TOKEN:
        # A Token whose Type, Attribute or Level its card's effect gives (Spirit Illusion's Doppelganger Token copies the
        # equipped monster's; Natural Selection's and Cell Division's copy a monster's) has none of its own in the database.
        # Its row stands in with Fiend, DARK, Level 1: the engine's field report brings the real ones as it is summoned, and
        # the server sends a Type or Attribute other than the row's in the monster's record (translate's `field`).
        d = dict(d, attribute=d['attribute'] or COPY_TOKEN_ATTRIBUTE, race=d['race'] or COPY_TOKEN_RACE, level=d['level'] or 1)
    kind, attribute, race = (learned['kind'].get(d['type']), learned['attribute'].get(d['attribute']), learned['race'].get(d['race']))
    for what, value, given in (('type', d['type'], kind), ('attribute', d['attribute'], attribute), ('race', d['race'], race)):
        if given is None:
            why.append(f'no game card has its {what} ({value:#x}), so the client can\'t show it (Xyz, Link, Pendulum and newer kinds)')
    if not 1 <= d['level'] <= 12:
        why.append(f'level {d["level"]}')
    stats = []
    for what in ('atk', 'def'):
        v = d[what]
        if v == -2:
            stats.append(0x1FF)
        elif 0 <= v <= STAT_MAX and v % 10 == 0:
            stats.append(v // 10)
        else:
            why.append(f'{what.upper()} {v} is beyond what the client can show')
            stats.append(0)
    if None in (kind, attribute, race):
        return None, None, why
    return (cid & 0x3FFF | stats[0] << 14 | stats[1] << 23,
            kind << 3 | attribute << 8 | d['level'] << 12 | race << 19 | learned['art'] << 27, why)


def table():
    with open(TABLE, newline='') as f:
        return list(csv.DictReader(f, delimiter='\t'))


def load():
    """Everything a card is checked against: the database, the passcodes, the game's rows and what they teach."""
    db = sqlite3.connect(os.path.join(ENGINE, 'vendor', 'BabelCDB', 'cards.cdb'))
    alias = {r[0]: r[1] for r in db.execute('select id, alias from datas')}
    data = {r[0]: {'type': r[1], 'atk': r[2], 'def': r[3], 'level': r[4] & 0xFF, 'race': r[5], 'attribute': r[6], 'name': r[7], 'desc': r[8],
                   'category': r[9]}
            for r in db.execute('select d.id, d.type, d.atk, d.def, d.level, d.race, d.attribute, t.name, t.desc, d.category from datas d join texts t on t.id = d.id')}
    passcode = passcodes(alias)
    rows = client_rows()
    with open(os.path.join(ENGINE, 'card-ids.json')) as f:
        konami = {m['konami_id'] for c in json.load(f)['data'] for m in c.get('misc_info', []) if m.get('konami_id', 0) > 0}
    in_game = {a & 0x3FFF for a, _ in rows} - {0}
    unofficial = sqlite3.connect(os.path.join(ENGINE, 'vendor', 'BabelCDB', 'cards-unofficial.cdb'))
    anime = {r[0]: {'type': r[1], 'atk': r[2], 'def': r[3], 'level': r[4] & 0xFF, 'race': r[5], 'attribute': r[6], 'name': r[7], 'desc': r[8],
                    'category': r[9], 'alias': r[10]}
             for r in unofficial.execute('select d.id, d.type, d.atk, d.def, d.level, d.race, d.attribute, t.name, t.desc, d.category, d.alias '
                                         'from datas d join texts t on t.id = d.id where d.ot & 4')}
    # Tokens (TYPE_TOKEN): the official database's and the unofficial one's, by passcode. They have no Konami id.
    tokens, official_tokens = {}, set()
    for source in (db, unofficial):
        for r in source.execute('select d.id, d.type, d.atk, d.def, d.level, d.race, d.attribute, t.name, t.desc, d.category '
                                'from datas d join texts t on t.id = d.id where d.type & ?', (TYPE_TOKEN,)):
            tokens.setdefault(r[0], {'type': r[1], 'atk': r[2], 'def': r[3], 'level': r[4] & 0xFF, 'race': r[5], 'attribute': r[6],
                                     'name': r[7], 'desc': r[8], 'category': r[9]})
            if source is db:
                official_tokens.add(r[0])
    with open(os.path.join(ENGINE, 'anime', 'client-tokens.tsv'), encoding='utf-8') as f:
        game_tokens = {line.split('\t', 1)[1].strip().casefold() for line in f if '\t' in line}
    genre = ypk_file('data/bin#/CARD_Genre.bin')
    genres = [struct.unpack_from('<Q', genre, i * 8)[0] for i in range(len(genre) // 8)]
    return {'data': data, 'passcode': passcode, 'rows': rows, 'learned': learn(rows, data, passcode),
            'genre': learn_genre(game_cards(rows, genres, data, passcode)), 'genres': genres,
            'anime_genre': learn_genre(game_cards(rows, genres, data, passcode, category=False)),
            'in_game': in_game, 'konami': konami, 'base': min(in_game), 'anime': anime, 'tokens': tokens, 'official_tokens': official_tokens, 'game_tokens': game_tokens}


ANIME_TAG = '[Anime]'  # crates/yo3-server/src/profile.rs ANIME_TAG: an anime card's text starts with it


def anime_problem(code, d):
    """Why an anime-only card's script can't be played, or None: it must exist in unofficial/ and register effects unless
    the card is a Normal Monster."""
    path = os.path.join(ENGINE, 'vendor', 'CardScripts', 'unofficial', f'c{code}.lua')
    if not os.path.exists(path):
        return f'no script unofficial/c{code}.lua'
    normal = d['type'] & 0x11 == 0x11 and not d['type'] & 0x20
    with open(path, encoding='utf-8', errors='replace') as f:
        text = f.read()
    if not normal and not any(w in text for w in ('RegisterEffect', 'aux.Add', 'Procedure', '.Add')):
        return f'unofficial/c{code}.lua registers no effect (a placeholder)'
    return None


def stand_in_problem(sid, ctx, taken):
    """Why `sid` can't be a stand-in id, or None."""
    if not ctx['base'] <= sid <= 0x3FFF:
        return f'stand-in {sid} is outside {ctx["base"]}..16383 (the client\'s lookup starts at its lowest id)'
    if sid in ctx['in_game']:
        return f'stand-in {sid} is a card of the game'
    if sid in ctx['konami'] or sid in taken:
        return f'stand-in {sid} is already a card\'s Konami id'
    return None


# Stand-in ids kept back: --find hands none of the last ones out. The client holds 12,484 ids (3900-16383); the game's
# cards, every Konami id in the pinned ids (kept for the printed cards they are) and the added cards' stand-ins take most.
# On 2026-11-26 1,002 were free, and the anime-only batches still to add needed about 860 (837 cards and their Tokens):
# the reserve keeps room for fixes and for the printed cards whose Konami ids are past 16383, until Tokens share an id or
# the budget otherwise grows (docs/roadmap.md).
STAND_IN_RESERVE = 100


def free_stand_ins(ctx, taken):
    """The ids a stand-in could still take (`taken` the stand-ins given already), reserve included."""
    return [i for i in range(ctx['base'], 0x4000) if stand_in_problem(i, ctx, taken) is None]


def next_stand_in(ctx, taken):
    """The lowest free stand-in id, or None when only the reserve is left."""
    free = free_stand_ins(ctx, taken)
    return free[0] if len(free) > STAND_IN_RESERVE else None


NO_STAND_IN = (f'no stand-in id is left outside the reserve: the client holds ids 3900 to 16383, and the last {STAND_IN_RESERVE} '
               'free ones are kept back (STAND_IN_RESERVE in tools/engine/added-cards.py)')


def build():
    ctx = load()
    data, passcode, rows, learned, in_game = ctx['data'], ctx['passcode'], ctx['rows'], ctx['learned'], ctx['in_game']
    out, problems = [], []
    taken = set()
    for i, line in enumerate(table()):
        row = int(line['row'])
        anime_code = int(line['anime']) if (line.get('anime') or '').strip() else None
        cid = int(line['konami_id']) if (line.get('konami_id') or '').strip() else 0
        sid = int(line['stand_in']) if (line.get('stand_in') or '').strip() else None
        token = int(line['token']) if (line.get('token') or '').strip() else None
        if token is not None and (cid or anime_code is not None or sid is None):
            problems.append(f'{line["name"]}: a Token has a stand-in, no Konami id and no anime passcode')
        if anime_code is not None:
            if cid:
                problems.append(f'{line["name"]}: an anime-only card has no Konami id (konami_id stays empty)')
            if sid is None:
                problems.append(f'{line["name"]}: an anime-only card needs a stand-in (--find picks one)')
        if sid is not None:
            problem = stand_in_problem(sid, ctx, taken)
            if problem:
                problems.append(f'{line["name"]}: {problem}')
            if anime_code is None and token is None and cid <= 0x3FFF:
                problems.append(f'{line["name"]}: a stand-in, but its Konami id {cid} fits the client')
            taken.add(sid)
        elif cid > 0x3FFF:
            problems.append(f'{line["name"]}: Konami id {cid} is beyond the client\'s 14 bits: give it a stand-in (--find picks one)')
        elif cid in taken:
            problems.append(f'{line["name"]}: Konami id {cid} is an earlier card\'s stand-in')
        if row != len(rows) + i:
            problems.append(f'{line["name"]}: row {row}, the next is {len(rows) + i} (rows are permanent and consecutive after the game\'s)')
        if cid in in_game:
            problems.append(f'{line["name"]}: Konami id {cid} is already a card of the game')
        if token is not None:
            code, d = token, ctx['tokens'].get(token)
            if d is None:
                problems.append(f'{line["name"]}: {token} is no Token in the pinned databases')
                continue
        elif anime_code is not None:
            code, d = anime_code, ctx['anime'].get(anime_code)
            if d is None or d['alias']:
                problems.append(f'{line["name"]}: {anime_code} is no anime card (ot 4, no alias) in the pinned unofficial database')
                continue
            problem = anime_problem(code, d)
            if problem:
                problems.append(f'{line["name"]}: {problem}')
        else:
            code = passcode.get(cid)
            d = data.get(code)
            if d is None:
                problems.append(f'{line["name"]}: Konami id {cid} has no card in the pinned database')
                continue
        if d['name'] != line['name']:
            problems.append(f'{line["name"]}: the database calls {code} "{d["name"]}"')
        rarity = (line.get('rarity') or '').strip().lower()
        if rarity not in RARITY:
            problems.append(f'{line["name"]}: rarity "{rarity}" (common, rare, super or ultra)')
        client = sid if sid is not None else cid
        a, b, why = encode(d, client, learned)
        problems += [f'{line["name"]}: {w}' for w in why if sid is not None or cid <= 0x3FFF or 'bits' not in w]
        if why:
            continue
        desc = card_text(d['desc'])
        if anime_code is not None:
            desc = f'{ANIME_TAG}\r{desc}'
        genre = 0 if token is not None else genre_of(features(d, code, anime_code is None), ctx['anime_genre' if anime_code is not None else 'genre'])
        card = {'row': row, 'cid': client, 'passcode': code, 'name': d['name'], 'desc': desc,
                'a': a, 'b': b | RARITY.get(rarity, 0) << 24, 'genre': genre, 'note': line['note']}
        if token is not None:
            card['token'] = True
            if token not in ctx['official_tokens']:
                card['anime_only'] = True  # only the anime database has it: the engine loads it with anime cards on
        elif anime_code is not None:
            card['anime_only'] = True
        elif sid is not None:
            card['konami_id'] = cid
        out.append(card)
    if problems:
        raise SystemExit('engine/added/cards.tsv:\n  ' + '\n  '.join(problems))
    doc = {'format': FORMAT, 'version': 1, 'first_row': len(rows), 'cards': out}
    # Characters the game never had as NPCs (engine/added/npcs.json), carried as they are: the server and the launcher read them.
    if os.path.isfile(NPCS):
        with open(NPCS, encoding='utf-8') as f:
            npcs = json.load(f)['npcs']
        if npcs:
            doc['npcs'] = npcs
    return doc


def made_tokens(code, ctx):
    """The Tokens the script the engine plays for `code` makes (Duel.CreateToken): their passcodes, read from the numbers
    and `id+N` in its CreateToken lines (a card's Token is usually its own passcode plus one)."""
    text = script_text(code) or ''
    out = []
    for line in text.split('\n'):
        line = line.split('--', 1)[0]
        if 'CreateToken' not in line and 'TOKEN' not in line:
            continue
        found = [int(n) for n in re.findall(r'\b(\d{5,9})\b', line)] + [code + int(n) for n in re.findall(r'\bid\s*\+\s*(\d)\b', line)]
        out += [t for t in found if t in ctx['tokens'] and t not in out]
    return out


def token_code(card, ctx):
    """The passcode of the Token `card` names (`token:N` or its name), or None."""
    text = card.strip()
    if text.lower().startswith('token:') and text[6:].strip().isdigit():
        code = int(text[6:])
        return code if code in ctx['tokens'] else None
    return next((c for c, d in sorted(ctx['tokens'].items()) if d['name'].casefold() == text.casefold()), None)


def lookup(card, data, by_name, cid_of, passcode):
    """The passcode of `card`: an English name (any case), a passcode, or `cid:N`; only cards with a Konami id."""
    card = card.strip()
    if card.lower().startswith('cid:') and card[4:].strip().isdigit():
        code = passcode.get(int(card[4:]))
        return code if code in data else None
    if card.isdigit():
        code = int(card)
        if code in data and code not in cid_of:  # an alternate artwork's passcode: the card's original one
            code = next((c for c in cid_of if data.get(c, {}).get('name') == data[code]['name']), code)
        return code if code in cid_of else None
    codes = [c for c in by_name.get(card.casefold(), []) if c in cid_of]
    return codes[0] if codes else None


def find(names, ctx=None):
    """For each name: (the cards.tsv line or None, the reasons it can't be added). Rows follow the table's last."""
    ctx = ctx or load()
    by_name = {}
    for code, d in ctx['data'].items():
        by_name.setdefault(d['name'].casefold(), []).append(code)
    cid_of = {}
    for cid, code in ctx['passcode'].items():
        cid_of.setdefault(code, cid)
    lines = table()
    listed = {int(line['konami_id']) for line in lines if (line.get('konami_id') or '').strip()}
    listed_anime = {int(line['anime']) for line in lines if (line.get('anime') or '').strip()}
    anime_by_name = {}
    for code, d in ctx['anime'].items():
        if not d['alias'] and not d['type'] & TYPE_TOKEN:  # an anime version of a printed card is no anime-only card
            anime_by_name.setdefault(d['name'].casefold(), code)
    taken = {int(line['stand_in']) for line in lines if (line.get('stand_in') or '').strip()}
    listed_tokens = {int(line['token']) for line in lines if (line.get('token') or '').strip()}
    is_token = lambda n: lookup(n, ctx['data'], by_name, cid_of, ctx['passcode']) is None and token_code(n, ctx) is not None
    names = [n for n in names if is_token(n)] + [n for n in names if not is_token(n)]  # a Token before the cards that make it
    batch_tokens = set()
    # A card's Tokens must be cards the client has: the game's (by name), an added Token, or one in this batch.
    def missing_tokens(code):
        have = listed_tokens | batch_tokens
        return [f'its Token {ctx["tokens"][t]["name"]} (token:{t}) is no card the client has: add the Token too (in this batch '
                f'or before it), or the client shows nothing where the engine has it'
                for t in made_tokens(code, ctx) if t not in have and ctx['tokens'][t]['name'].casefold() not in ctx['game_tokens']]
    row = len(ctx['rows']) + len(lines)
    out = []
    for name in names:
        token = token_code(name, ctx) if lookup(name, ctx['data'], by_name, cid_of, ctx['passcode']) is None else None
        if token is not None:
            d = ctx['tokens'][token]
            why = ['engine/added/cards.tsv already has it'] if token in listed_tokens else []
            if d['name'].casefold() in ctx['game_tokens']:
                why.append('the game has a Token of that name')
            sid = next_stand_in(ctx, taken)
            if sid is None:
                why.append(NO_STAND_IN)
            why += encode(d, sid or ctx['base'], ctx['learned'])[2]
            if why:
                out.append((d['name'], None, why))
            else:
                out.append((d['name'], f'{row}\t\t{sid}\t\t\t{token}\t{d["name"]}\t', []))
                row += 1
                taken.add(sid)
                batch_tokens.add(token)
            continue
        code = lookup(name, ctx['data'], by_name, cid_of, ctx['passcode'])
        anime_code = None
        if code is None:
            text = name.strip()
            anime_code = int(text[6:]) if text.lower().startswith('anime:') and text[6:].strip().isdigit() else anime_by_name.get(text.casefold())
            if anime_code not in ctx['anime'] or ctx['anime'][anime_code]['alias']:
                anime_code = None
        if anime_code is not None:
            d = ctx['anime'][anime_code]
            why = ['engine/added/cards.tsv already has it'] if anime_code in listed_anime else []
            problem = anime_problem(anime_code, d)
            if problem:
                why.append(problem)
            why += missing_tokens(anime_code)
            sid = next_stand_in(ctx, taken)
            if sid is None:
                why.append(NO_STAND_IN)
            why += encode(d, sid or ctx['base'], ctx['learned'])[2]
            if why:
                out.append((d['name'], None, why))
            else:
                out.append((d['name'], f'{row}\t\t{sid}\t\t{anime_code}\t\t{d["name"]}\t', []))
                row += 1
                taken.add(sid)
            continue
        if code is None:
            out.append((name, None, ['no card of that name, passcode or Konami id in the pinned data (check the spelling)']))
            continue
        d, cid = ctx['data'][code], cid_of[code]
        why = []
        if cid in ctx['in_game']:
            why.append('the game already has it')
        if cid in listed:
            why.append('engine/added/cards.tsv already has it')
        why += missing_tokens(code)
        for anime in (c for c, a in ctx['anime'].items() if a['alias'] == code):
            why += [w.replace('its Token', 'its anime version\'s Token', 1) for w in missing_tokens(anime) if w not in why]
        needs = cid > 0x3FFF or cid in taken
        sid = next_stand_in(ctx, taken) if needs else None
        if needs and sid is None:
            why.append(NO_STAND_IN)
        why += encode(d, sid or (ctx['base'] if needs else cid), ctx['learned'])[2]
        if why:
            out.append((d['name'], None, why))
        else:
            out.append((d['name'], f'{row}\t{cid}\t{sid or ""}\t\t\t\t{d["name"]}\t', []))
            row += 1
            if sid:
                taken.add(sid)
    return out


def anime_rows(results, ctx):
    """engine/anime/cards.tsv's line for each printed card `find` approved (every card a player can hold has one): its
    id, name, passcode and, when the pinned unofficial database has one (an anime card aliasing it), its anime version."""
    rows = []
    for name, line, _ in results:
        if not line:
            continue
        parts = line.split('\t')
        if parts[4] or parts[5]:
            continue  # anime-only (its own anime version) or a Token: no row
        konami, stand_in = int(parts[1]), parts[2]
        code = ctx['passcode'].get(konami)
        candidates = sorted((c for c, d in ctx['anime'].items() if d['alias'] == code), key=lambda c: not ctx['anime'][c]['name'].endswith('(Anime)'))
        anime = candidates[0] if candidates else None
        rows.append({'cid': int(stand_in or konami), 'name': name, 'official': code, 'anime': anime,
                     'line': f"{stand_in or konami}\t{name}\t{code}\t{anime or ''}\t{'Anime' if anime else ''}\tadded card (#75)"})
    return rows


def anime_outcomes(rows):
    """What tools/engine/anime-map.py would make of `rows` once added: {cid: (status, reasons)}, by its own rules (a
    temporary table with the rows, the cards' passcodes known as if added)."""
    import tempfile
    from unittest import mock
    spec = importlib.util.spec_from_file_location('anime_map', os.path.join(HERE, 'anime-map.py'))
    amap = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(amap)
    with open(amap.TABLE, encoding='utf-8') as f:
        head, *lines = f.read().rstrip('\n').split('\n')
    taken = {int(r['cid']) for r in rows}
    lines = [l for l in lines if int(l.split('\t')[0]) not in taken] + [r['line'] for r in rows]
    lines.sort(key=lambda l: int(l.split('\t')[0]))
    original = amap.official_passcodes
    def passcodes(alias):
        out = original(alias)
        out.update({r['cid']: r['official'] for r in rows})
        return out
    with tempfile.NamedTemporaryFile('w', suffix='.tsv', delete=False, encoding='utf-8') as f:
        f.write(head + '\n' + '\n'.join(lines) + '\n')
    try:
        with mock.patch.object(amap, 'TABLE', f.name), mock.patch.object(amap, 'official_passcodes', passcodes), \
                contextlib.redirect_stdout(io.StringIO()):
            entries = {e['cid']: e for e in amap.build()['cards'] if e['cid'] in taken}
    finally:
        os.unlink(f.name)
    return {cid: (e['status'], e.get('holds') or e.get('profile') or []) for cid, e in entries.items()}


HELD_FIXES = {
    'placeholder': 'the game shows "?" and the anime database 0: a corrected record in engine/anime/records.json keeps the "?" '
                   '(as Clear Vice Dragon and Divine Serpent Geh, docs/added-cards.md#its-anime-version)',
}


def stand_in_budget(results, ctx):
    """The stand-in ids left once `results`' lines are added: how many, and the reserve among them."""
    taken = {int(line['stand_in']) for line in table() if (line.get('stand_in') or '').strip()}
    taken |= {int(line.split('\t')[2]) for _, line, _ in results if line and line.split('\t')[2]}
    return len(free_stand_ins(ctx, taken)), STAND_IN_RESERVE


def report(results, ctx):
    """Each card's verdict, with the script lines to read, then the stand-in ids left."""
    for name, line, why in results:
        anime_only = line and line.split('\t')[4]
        token = line and line.split('\t')[5]
        verdict = 'ok (a Token: its effect makes it)' if token else 'ok (anime-only: played with anime cards on)' if anime_only else 'ok'
        print(f'{name}: ' + (verdict if line else 'cannot be added: ' + '; '.join(why)))
        for note in notes(line, ctx):
            print(f'  read first: {note}')
        for hint in infinity_hints(line, ctx):
            print(f'  at ∞: {hint}')
        for hint in risk_hints(line, ctx):
            print(f'  script: {hint}')
    free, reserve = stand_in_budget(results, ctx)
    print(f'Stand-in ids: {free} free after these ({free - reserve} to give, {reserve} kept back)')


def regenerate():
    """This build, then tools/engine/anime-map.py and tools/engine/anime-pictures.py (each reads the one before): 0 when
    all three wrote their files."""
    for tool in ('added-cards.py', 'anime-map.py', 'anime-pictures.py'):
        r = subprocess.run([sys.executable, os.path.join(HERE, tool)])
        if r.returncode != 0:
            print(f'tools/engine/{tool} failed: fix the tables (git diff engine/) and run it again', file=sys.stderr)
            return 1
    return 0


def add(cards, note, checked=False):
    """--add: both tables' lines for `cards`, then the generators; nothing written unless every card can go in."""
    ctx = load()
    results = find(cards, ctx)
    report(results, ctx)
    if not all(line for _, line, _ in results):
        print('\nnothing added: every card must fit (drop the ones that cannot)')
        return 1
    noted = [name for name, line, _ in results if notes(line, ctx)]
    if noted and not checked:
        print(f'\nnothing added: read the lines noted for {", ".join(noted)}; drop those cards, or add again with --checked')
        return 1
    note = ' '.join(note.split())
    with open(TABLE, encoding='utf-8') as f:
        text = f.read()
    with open(TABLE, 'w', encoding='utf-8') as f:
        f.write(text if text.endswith('\n') else text + '\n')
        for _, line, _ in results:
            f.write(line + note + '\n')
    rows = anime_rows(results, ctx)
    if rows:
        with open(ANIME_TABLE, encoding='utf-8') as f:
            head, *lines = f.read().rstrip('\n').split('\n')
        have = {int(l.split('\t')[0]) for l in lines}
        lines += [r['line'] for r in rows if r['cid'] not in have]
        lines.sort(key=lambda l: int(l.split('\t')[0]))
        with open(ANIME_TABLE, 'w', encoding='utf-8') as f:
            f.write(head + '\n' + '\n'.join(lines) + '\n')
    print(f'\nadded {len(results)} card(s) to engine/added/cards.tsv' + (' and engine/anime/cards.tsv' if rows else '') + '; rebuilding...')
    if regenerate():
        return 1
    print('\nNext (docs/added-cards.md): a scenario test per effect, `task check:added`, a live check, then '
          'docs/features.md and CHANGELOG.md. Rows and stand-ins are permanent once committed: check the order first.')
    return 0


def render(doc):
    return json.dumps(doc, indent=1, ensure_ascii=False) + '\n'


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--check', action='store_true', help='compare the committed file with a fresh build')
    p.add_argument('--find', nargs='*', metavar='CARD', help='whether these cards (names, passcodes or cid:N) can be added, and their table lines')
    p.add_argument('--find-file', metavar='FILE', help='the same for the cards in FILE, one per line')
    p.add_argument('--add', nargs='*', metavar='CARD', help='add these cards to both tables and rebuild (all or nothing)')
    p.add_argument('--add-file', metavar='FILE', help='the same for the cards in FILE, one per line')
    p.add_argument('--note', help="--add: the table's note for each card (who asked, the issue)")
    p.add_argument('--checked', action='store_true', help='--add: the scripts --find notes were read and are fine')
    args = p.parse_args(argv)
    def from_file(path):
        with open(path, encoding='utf-8-sig') as f:
            return [l.strip() for l in f if l.strip() and not l.lstrip().startswith('#')]
    if args.find_file:
        args.find = (args.find or []) + from_file(args.find_file)
    if args.add_file:
        args.add = (args.add or []) + from_file(args.add_file)
    for flag, cards in (('--add', args.add), ('--find', args.find)):
        if cards == []:
            p.error(f'{flag} needs cards (names, passcodes, cid:N, anime:N) or {flag}-file FILE')
    if args.add:
        if not args.note:
            p.error('--add needs --note (who asked, the issue): the table keeps it')
        return add(args.add, args.note, args.checked)
    if args.find:
        ctx = load()
        results = find(args.find, ctx)
        report(results, ctx)
        lines = [line for _, line, _ in results if line]
        if lines:
            print('\nLines for engine/added/cards.tsv (add a note after the last tab), or add them with --add CARD... --note TEXT:')
            print('\n'.join(lines))
        rows = anime_rows(results, ctx) if lines else []
        if rows:
            outcomes = anime_outcomes(rows)
            print('\nLines for engine/anime/cards.tsv (every printed card needs one; it keeps the table sorted by id), then run '
                  'tools/engine/anime-map.py:')
            print('\n'.join(r['line'] for r in rows))
            for r in rows:
                status, reasons = outcomes.get(r['cid'], ('?', []))
                if not r['anime']:
                    print(f"  {r['name']}: no anime version upstream; it plays its printed effect either way")
                elif status == 'held':
                    print(f"  {r['name']}: its anime version {r['anime']} would be HELD: " + '; '.join(reasons))
                    for key, fix in HELD_FIXES.items():
                        if any(x.startswith(key) for x in reasons):
                            print(f'    fix: {fix}')
                else:
                    shown = f" (the profile shows: {'; '.join(reasons)})" if reasons else ''
                    print(f"  {r['name']}: its anime version {r['anime']} would be {status}{shown}")
        return 0 if all(line for _, line, _ in results) else 1
    text = render(build())
    if args.check:
        with open(OUT, encoding='utf-8') as f:
            same = f.read() == text
        print('engine/added-cards.json is current' if same else 'engine/added-cards.json differs: run tools/engine/added-cards.py')
        return 0 if same else 1
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)
    print(f'wrote {OUT}: {len(json.loads(text)["cards"])} card(s)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
