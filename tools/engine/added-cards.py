#!/usr/bin/env python3
"""Builds engine/added-cards.json: the cards the game never had, each a complete row of the client's card table (#75).

    tools/engine/added-cards.py            write engine/added-cards.json
    tools/engine/added-cards.py --check    compare the committed file with a fresh build (exit 1 when it differs)
    tools/engine/added-cards.py --find CARD [CARD...]
                                           whether each card can be added, and the line for engine/added/cards.tsv
                                           when it can (rows continue after the table's). CARD is its English name
                                           (any case), its passcode, or `cid:N` (its Konami id)
    tools/engine/added-cards.py --find-file FILE   the same for a file of cards, one per line (`#` comments)

Input (committed): engine/added/cards.tsv, one line per added card: its permanent `row` (4834 on, after the game's last;
never reused or reordered: decks and saves store rows), its Konami id, its `stand_in` id (below), its `rarity` (below,
empty for Common), its name (for reading the table) and a note.

The client keeps a card's Konami id in 14 bits (16383 at most) and looks cards up from its lowest id (3900). A newer card
whose Konami id is beyond that gets a permanent stand-in: the lowest id from 3900 that no card has (not the game's, no
Konami card's in the pinned ids, no other added card's). The client, the server and the rules engine know the card by
that id (the JSON's `cid`; `konami_id` keeps the real one); --find picks it. Like a row, it is never changed.

Each card's data comes from the pinned EDOPro database through the pinned Konami ids (engine/card-ids.json), never typed
by hand. The client's encoding of a row (`CARD_Prop`: word `a` = Konami id, ATK/10, DEF/10; word `b` = kind, attribute,
level, race, artwork; docs/client-internals.md, Card data) is learned from the game's own cards: every game card the
database also has gives its type, attribute and race next to the client's kind, attribute and race (and, for a Spell or
Trap, its subtype: Field, Equip, Quick-Play...), and an added card may only use values the game itself shows. So no Xyz,
Link or Pendulum card can be added (the client has no such kind), a stat beyond the nine bits (5110) is refused, and so
is a Konami id beyond the row's 14 bits (16383). `CARD_Genre` (Edit Deck's search by effect: Special Summon, Destroy monster, Pierce, Flip Effect...) is learned the
same way: for each of its bits, the EDOPro categories (`datas.category`, the official database's effect categories) and
type flags that mark it on the game's cards with at least 85% precision (`learn_genre`). Bits no such rule predicts
(Recover cards from Graveyard, Hand destruction...) stay unset: an added card is not found under them.

Rarity (word b bits 24-26: 0 Common, 4 Rare, 3 Super Rare, 2 Ultra Rare, as the game's pack files place the cards in
their tiers; 1 and 5 its top tiers; the new-card reveal's and the pack opening's glow, `_root.rale_bg`) is the table's
optional `rarity` column: `common` (empty), `rare`, `super` or `ultra`. Bits 0-2 have no reader in the client's card
code and are 0 on the game's newest rows; added cards leave them 0.

Needs the pinned data (engine/fetch.sh) and the game (YO3_GAME_DIR, else the default Wine path).
"""
import argparse
import collections
import csv
import importlib.util
import json
import os
import sqlite3
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..'))
ENGINE = os.path.join(ROOT, 'engine')
TABLE = os.path.join(ENGINE, 'added', 'cards.tsv')
OUT = os.path.join(ENGINE, 'added-cards.json')
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
RARITY = {'': 0, 'common': 0, 'rare': 4, 'super': 3, 'ultra': 2}


def learn_genre(rows, genres, data, passcode):
    """{genre bit: [('category' | 'type', bit), ...]}: a card gets the genre bit when it has any of the listed bits.
    Learned from the game's cards: features chosen greedily, each marking the genre bit with at least
    GENRE_PRECISION on the cards it marks and adding at least 2% (and 3 cards) of the genre bit's cards."""
    cards = []
    for (a, _), genre in zip(rows, genres):
        code = passcode.get(a & 0x3FFF)
        if a & 0x3FFF and code in data:
            cards.append((genre, data[code]['category'], data[code]['type']))
    def mask(test):
        return sum(1 << i for i, c in enumerate(cards) if test(c))
    features = [('category', bit) for bit in range(64)] + [('type', bit) for bit in range(32)]
    marks = {f: mask(lambda c, f=f: (c[1] if f[0] == 'category' else c[2]) >> f[1] & 1) for f in features}
    rules = {}
    for g in range(64):
        has = mask(lambda c: c[0] >> g & 1)
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


def genre_of(d, rules):
    """An added card's CARD_Genre word, by the learned rules."""
    return sum(1 << g for g, fs in rules.items() if any((d['category'] if k == 'category' else d['type']) >> bit & 1 for k, bit in fs))


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
    genre = ypk_file('data/bin#/CARD_Genre.bin')
    genres = [struct.unpack_from('<Q', genre, i * 8)[0] for i in range(len(genre) // 8)]
    return {'data': data, 'passcode': passcode, 'rows': rows, 'learned': learn(rows, data, passcode),
            'genre': learn_genre(rows, genres, data, passcode), 'genres': genres,
            'in_game': in_game, 'konami': konami, 'base': min(in_game)}


def stand_in_problem(sid, ctx, taken):
    """Why `sid` can't be a stand-in id, or None."""
    if not ctx['base'] <= sid <= 0x3FFF:
        return f'stand-in {sid} is outside {ctx["base"]}..16383 (the client\'s lookup starts at its lowest id)'
    if sid in ctx['in_game']:
        return f'stand-in {sid} is a card of the game'
    if sid in ctx['konami'] or sid in taken:
        return f'stand-in {sid} is already a card\'s Konami id'
    return None


def next_stand_in(ctx, taken):
    return next(i for i in range(ctx['base'], 0x4000) if stand_in_problem(i, ctx, taken) is None)


def build():
    ctx = load()
    data, passcode, rows, learned, in_game = ctx['data'], ctx['passcode'], ctx['rows'], ctx['learned'], ctx['in_game']
    out, problems = [], []
    taken = set()
    for i, line in enumerate(table()):
        row, cid = int(line['row']), int(line['konami_id'])
        sid = int(line['stand_in']) if (line.get('stand_in') or '').strip() else None
        if sid is not None:
            problem = stand_in_problem(sid, ctx, taken)
            if problem:
                problems.append(f'{line["name"]}: {problem}')
            if cid <= 0x3FFF:
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
        card = {'row': row, 'cid': client, 'passcode': code, 'name': d['name'], 'desc': d['desc'].replace('\r\n', '\r').replace('\n', '\r'),
                'a': a, 'b': b | RARITY.get(rarity, 0) << 24, 'genre': genre_of(d, ctx['genre']), 'note': line['note']}
        if sid is not None:
            card['konami_id'] = cid
        out.append(card)
    if problems:
        raise SystemExit('engine/added/cards.tsv:\n  ' + '\n  '.join(problems))
    return {'format': FORMAT, 'version': 1, 'first_row': len(rows), 'cards': out}


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
    listed = {int(line['konami_id']) for line in lines}
    taken = {int(line['stand_in']) for line in lines if (line.get('stand_in') or '').strip()}
    row = len(ctx['rows']) + len(lines)
    out = []
    for name in names:
        code = lookup(name, ctx['data'], by_name, cid_of, ctx['passcode'])
        if code is None:
            out.append((name, None, ['no card of that name, passcode or Konami id in the pinned data (check the spelling)']))
            continue
        d, cid = ctx['data'][code], cid_of[code]
        why = []
        if cid in ctx['in_game']:
            why.append('the game already has it')
        if cid in listed:
            why.append('engine/added/cards.tsv already has it')
        sid = next_stand_in(ctx, taken) if cid > 0x3FFF or cid in taken else None
        why += encode(d, sid or cid, ctx['learned'])[2]
        if why:
            out.append((d['name'], None, why))
        else:
            out.append((d['name'], f'{row}\t{cid}\t{sid or ""}\t\t{d["name"]}\t', []))
            row += 1
            if sid:
                taken.add(sid)
    return out


def render(doc):
    return json.dumps(doc, indent=1, ensure_ascii=False) + '\n'


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--check', action='store_true', help='compare the committed file with a fresh build')
    p.add_argument('--find', nargs='+', metavar='CARD', help='whether these cards (names, passcodes or cid:N) can be added, and their table lines')
    p.add_argument('--find-file', metavar='FILE', help='the same for the cards in FILE, one per line')
    args = p.parse_args(argv)
    if args.find_file:
        with open(args.find_file, encoding='utf-8-sig') as f:
            args.find = (args.find or []) + [l.strip() for l in f if l.strip() and not l.lstrip().startswith('#')]
    if args.find:
        results = find(args.find)
        for name, line, why in results:
            print(f'{name}: ' + ('ok' if line else 'cannot be added: ' + '; '.join(why)))
        lines = [line for _, line, _ in results if line]
        if lines:
            print('\nLines for engine/added/cards.tsv (add a note after the last tab):')
            print('\n'.join(lines))
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
