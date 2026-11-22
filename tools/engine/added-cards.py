#!/usr/bin/env python3
"""Builds engine/added-cards.json: the cards the game never had, each a complete row of the client's card table (#75).

    tools/engine/added-cards.py            write engine/added-cards.json
    tools/engine/added-cards.py --check    compare the committed file with a fresh build (exit 1 when it differs)

Input (committed): engine/added/cards.tsv, one line per added card: its permanent `row` (4834 on, after the game's last;
never reused or reordered: decks and saves store rows), its Konami id, its name (for reading the table) and a note.

Each card's data comes from the pinned EDOPro database through the pinned Konami ids (engine/card-ids.json), never typed
by hand. The client's encoding of a row (`CARD_Prop`: word `a` = Konami id, ATK/10, DEF/10; word `b` = kind, attribute,
level, race, artwork; docs/client-internals.md, Card data) is learned from the game's own cards: every game card the
database also has gives its type, attribute and race next to the client's kind, attribute and race, and an added card
may only use values the game itself shows. So no Xyz, Link or Pendulum card can be added (the client has no such kind),
and a stat beyond the nine bits (5110) is refused. `CARD_Genre` (Edit Deck's genre search) is 0 for now: its bits are
the client's own categories, not EDOPro's.

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
    {race: race} (monsters only), and the artwork number of a card's first print."""
    seen = {'kind': collections.defaultdict(collections.Counter), 'attribute': collections.defaultdict(collections.Counter),
            'race': collections.defaultdict(collections.Counter)}
    arts = collections.Counter()
    for a, b in rows:
        cid = a & 0x3FFF
        code = passcode.get(cid)
        if not cid or code not in data:
            continue
        f, d = fields(b), data[code]
        if not d['type'] & 1:
            continue  # Spells and Traps: not needed by a monster, and their attribute and race carry the card's family
        seen['kind'][d['type']][f['kind']] += 1
        seen['attribute'][d['attribute']][f['attribute']] += 1
        seen['race'][d['race']][f['race']] += 1
        arts[f['art']] += 1
    learned = {k: {v: c.most_common(1)[0][0] for v, c in table.items()} for k, table in seen.items()}
    learned['art'] = min(arts) if arts else 1  # the lowest artwork number is a card's first print
    return learned


def table():
    with open(TABLE, newline='') as f:
        return list(csv.DictReader(f, delimiter='\t'))


def build():
    db = sqlite3.connect(os.path.join(ENGINE, 'vendor', 'BabelCDB', 'cards.cdb'))
    alias = {r[0]: r[1] for r in db.execute('select id, alias from datas')}
    data = {r[0]: {'type': r[1], 'atk': r[2], 'def': r[3], 'level': r[4] & 0xFF, 'race': r[5], 'attribute': r[6], 'name': r[7], 'desc': r[8]}
            for r in db.execute('select d.id, d.type, d.atk, d.def, d.level, d.race, d.attribute, t.name, t.desc from datas d join texts t on t.id = d.id')}
    passcode = passcodes(alias)
    rows = client_rows()
    learned = learn(rows, data, passcode)
    in_game = {a & 0x3FFF for a, _ in rows} - {0}
    out, problems = [], []
    for i, line in enumerate(table()):
        row, cid = int(line['row']), int(line['konami_id'])
        if row != len(rows) + i:
            problems.append(f'{line["name"]}: row {row}, the next is {len(rows) + i} (rows are permanent and consecutive after the game\'s)')
        if cid in in_game:
            problems.append(f'{line["name"]}: Konami id {cid} is already a card of the game')
        if not 0 < cid <= 0x3FFF:
            problems.append(f'{line["name"]}: Konami id {cid} does not fit the client\'s 14 bits')
        code = passcode.get(cid)
        d = data.get(code)
        if d is None:
            problems.append(f'{line["name"]}: Konami id {cid} has no card in the pinned database')
            continue
        if d['name'] != line['name']:
            problems.append(f'{line["name"]}: the database calls {code} "{d["name"]}"')
        kind, attribute, race = (learned['kind'].get(d['type']), learned['attribute'].get(d['attribute']), learned['race'].get(d['race']))
        for what, value, given in (('type', d['type'], kind), ('attribute', d['attribute'], attribute), ('race', d['race'], race)):
            if given is None:
                problems.append(f'{line["name"]}: no game card has its {what} ({value:#x}), so the client can\'t show it')
        if not 1 <= d['level'] <= 12:
            problems.append(f'{line["name"]}: level {d["level"]}')
        stats = []
        for what in ('atk', 'def'):
            v = d[what]
            if v == -2:
                stats.append(0x1FF)
            elif 0 <= v <= STAT_MAX and v % 10 == 0:
                stats.append(v // 10)
            else:
                problems.append(f'{line["name"]}: {what.upper()} {v} is beyond what the client can show')
                stats.append(0)
        if None in (kind, attribute, race):
            continue
        a = cid | stats[0] << 14 | stats[1] << 23
        b = kind << 3 | attribute << 8 | d['level'] << 12 | race << 19 | learned['art'] << 27
        out.append({'row': row, 'cid': cid, 'passcode': code, 'name': d['name'], 'desc': d['desc'].replace('\r\n', '\r').replace('\n', '\r'),
                    'a': a, 'b': b, 'genre': 0, 'note': line['note']})
    if problems:
        raise SystemExit('engine/added/cards.tsv:\n  ' + '\n  '.join(problems))
    return {'format': FORMAT, 'version': 1, 'first_row': len(rows), 'cards': out}


def render(doc):
    return json.dumps(doc, indent=1, ensure_ascii=False) + '\n'


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--check', action='store_true', help='compare the committed file with a fresh build')
    args = p.parse_args(argv)
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
