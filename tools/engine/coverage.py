#!/usr/bin/env python3
"""Which card effects the duel check's self-play actually exercised (#67).

    tools/engine/coverage.py                     after `cargo nextest run -p yo3-server --test duel_check`
    tools/engine/coverage.py --sweep FILE        a card sweep's card-sweep.tsv instead (every swept card)
    tools/engine/coverage.py --json OUT          also write the merged counts

The duel check writes one file per test part (target/tmp/duel-coverage/part-N.json): per card (Konami id), its chain
links activated, resolved, resolved with a visible result (a card moved, LP or ATK/DEF changed...), negated, and its
summons. This report lists the deck pool's cards with an effect by what happened to them, least tested first. It is a
list to choose scenario tests from, not a pass/fail gate. YO3_CHECK_DUELS=n plays more duels per part.
"""
import argparse
import csv
import glob
import json
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TYPE_SPELL, TYPE_TRAP, TYPE_EFFECT = 0x2, 0x4, 0x20
KEYS = ('activated', 'resolved', 'visible', 'negated', 'summoned')


def merge(parts):
    """The parts' counts added up: (duels, pool, {cid: {key: n}})."""
    duels, pool, cards = 0, set(), {}
    for part in parts:
        duels += part['duels']
        pool.update(part['pool'])
        for cid, counts in part['cards'].items():
            mine = cards.setdefault(int(cid), dict.fromkeys(KEYS, 0))
            for k in KEYS:
                mine[k] += counts.get(k, 0)
    return duels, sorted(pool), cards


def from_sweep(path):
    """A card sweep's report as one part: each swept card's own counts (the filler cards aren't in the file)."""
    cards, duels = {}, 0
    with open(path, newline='', encoding='utf-8') as f:
        for row in csv.DictReader(f, delimiter='\t'):
            if row['result'] != 'ok':
                continue
            duels += 1
            mine = cards.setdefault(row['cid'], dict.fromkeys(KEYS, 0))
            for k in KEYS:
                if row.get(k, '').isdigit():
                    mine[k] += int(row[k])
    return {'duels': duels, 'pool': [int(c) for c in cards], 'cards': cards}


def card_info(engine):
    """{Konami id: (name, type)} from the engine's pinned data: cardinfo.json maps ids to passcodes, the card
    databases name and type them. Empty when the engine isn't fetched."""
    info_file = engine / 'vendor/cardinfo.json'
    if not info_file.is_file():
        return {}
    passcodes = {}
    for card in json.loads(info_file.read_text())['data']:
        for misc in card.get('misc_info', []):
            if 'konami_id' in misc:
                passcodes.setdefault(misc['konami_id'], card['id'])
    data = {}
    for cdb in sorted(glob.glob(str(engine / 'vendor/BabelCDB/*.cdb')), key=lambda p: not p.endswith('/cards.cdb')):
        with sqlite3.connect(f'file:{cdb}?mode=ro', uri=True) as db:
            for code, name, kind in db.execute('SELECT d.id, t.name, d.type FROM datas d JOIN texts t ON t.id = d.id'):
                data.setdefault(code, (name, kind))
    return {cid: data[code] for cid, code in passcodes.items() if code in data}


def classify(pool, cards, info):
    """The pool's cards with an effect, by what happened to them: (title, [(cid, name, counts)]) least tested first.
    A card the engine data doesn't know is kept (its type unknown)."""
    groups = {'never activated': [], 'activated, never resolved': [], 'resolved, nothing visible': [], 'resolved with a visible result': []}
    plain = 0
    for cid in pool:
        name, kind = info.get(cid, (f'#{cid}', TYPE_EFFECT))
        if not kind & (TYPE_SPELL | TYPE_TRAP | TYPE_EFFECT):
            plain += 1
            continue
        c = cards.get(cid, dict.fromkeys(KEYS, 0))
        if c['visible']:
            title = 'resolved with a visible result'
        elif c['resolved']:
            title = 'resolved, nothing visible'
        elif c['activated']:
            title = 'activated, never resolved'
        else:
            title = 'never activated'
        groups[title].append((cid, name, c))
    return list(groups.items()), plain


def report(duels, pool, cards, info):
    groups, plain = classify(pool, cards, info)
    effects = sum(len(g) for _, g in groups)
    lines = [f'Effect coverage: {duels} duel(s), {len(pool)} card(s) in the pool, {effects} with an effect '
             f'({plain} without one left out)']
    for title, group in groups:
        lines.append(f'\n{title}: {len(group)}')
        for cid, name, c in sorted(group, key=lambda g: (g[2]['visible'], g[2]['resolved'], g[2]['activated'], g[1])):
            counts = ', '.join(f'{k} {c[k]}' for k in KEYS if c[k])
            lines.append(f'  {cid:5}  {name}{f"  ({counts})" if counts else ""}')
    return '\n'.join(lines)


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--dir', default=str(ROOT / 'target/tmp/duel-coverage'), help='the duel check parts (default: %(default)s)')
    ap.add_argument('--sweep', help="a card sweep's card-sweep.tsv instead of the duel check's parts")
    ap.add_argument('--engine', default=str(ROOT / 'engine'), help='the engine folder for card names (default: %(default)s)')
    ap.add_argument('--json', help='also write the merged counts here')
    args = ap.parse_args(argv)
    if args.sweep:
        parts = [from_sweep(args.sweep)]
    else:
        files = sorted(Path(args.dir).glob('part-*.json'))
        if not files:
            print(f'no coverage in {args.dir}: run `cargo nextest run -p yo3-server --test duel_check` first', file=sys.stderr)
            return 2
        parts = [json.loads(f.read_text()) for f in files]
    duels, pool, cards = merge(parts)
    print(report(duels, pool, cards, card_info(Path(args.engine))))
    if args.json:
        Path(args.json).write_text(json.dumps({'duels': duels, 'pool': pool, 'cards': {str(k): v for k, v in sorted(cards.items())}}, indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
