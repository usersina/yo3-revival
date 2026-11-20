#!/usr/bin/env python3
"""A card's text from the engine's pinned data, by Konami id (the numbers the client and the scenario tests use).

    tools/engine/cardtext.py 7696 7687 ...     name, Level, ATK/DEF and the card text of each

The text is the pinned database's wording (ProjectIgnis' BabelCDB), which may be newer than the client's 2010-2012 card
text: a scenario test's doc comment names the text it follows and any difference (docs/development.md).
"""
import glob
import json
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main(argv):
    if not argv or argv[0] in ('-h', '--help'):
        print(__doc__)
        return 0 if argv else 2
    info = ROOT / 'engine/vendor/cardinfo.json'
    if not info.is_file():
        print('engine data missing: run engine/fetch.sh', file=sys.stderr)
        return 2
    passcodes = {}
    for card in json.loads(info.read_text())['data']:
        for misc in card.get('misc_info', []):
            if 'konami_id' in misc:
                passcodes.setdefault(misc['konami_id'], card['id'])
    dbs = [sqlite3.connect(f'file:{p}?mode=ro', uri=True) for p in sorted(glob.glob(str(ROOT / 'engine/vendor/BabelCDB/*.cdb')))]
    for arg in argv:
        cid = int(arg)
        code = passcodes.get(cid)
        row = None
        for db in dbs:
            row = db.execute('SELECT t.name, t.desc, d.level, d.atk, d.def, d.race, d.type FROM texts t JOIN datas d ON d.id = t.id WHERE t.id = ?', (code,)).fetchone()
            if row:
                break
        if not row:
            print(f'{cid}: not in the data (passcode {code})')
            continue
        name, desc, level, atk, df, race, kind = row
        print(f'{cid} {name}  passcode {code}  L{level & 0xFF} {atk}/{df} race {race:#x} type {kind:#x}\n  {desc.replace(chr(10), " ")}\n')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
