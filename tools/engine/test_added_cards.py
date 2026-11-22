#!/usr/bin/env python3
"""tools/engine/added-cards.py (#75): the committed engine/added-cards.json is what the table and the pinned data build,
and the table's rules hold. Needs engine/fetch.sh's data and the game (skipped without them)."""
import importlib.util
import os
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('added', Path(__file__).with_name('added-cards.py'))
added = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(added)
HAVE = os.path.exists(os.path.join(added.ENGINE, 'vendor', 'BabelCDB', 'cards.cdb')) and os.path.isdir(os.path.join(added.game_dir(), 'package'))


@unittest.skipUnless(HAVE, 'needs engine/fetch.sh and the game')
class AddedCards(unittest.TestCase):
    def test_the_committed_file_is_current(self):
        self.assertEqual(added.main(['--check']), 0)

    def test_quasar_is_the_games_kind_of_synchro(self):
        doc = added.build()
        self.assertEqual(doc['first_row'], 4834)
        quasar = doc['cards'][0]
        a, b = quasar['a'], quasar['b']
        self.assertEqual((a & 0x3FFF, (a >> 14 & 0x1FF) * 10, (a >> 23 & 0x1FF) * 10), (9709, 4000, 4000))
        f = added.fields(b)
        self.assertEqual((f['kind'], f['level'], f['art']), (18, 12, 1), 'a Synchro Effect monster, level 12, first artwork')
        self.assertNotIn('\n', quasar['desc'], "the game's line break is \\r")

    def test_an_xyz_monster_has_no_kind_the_client_shows(self):
        import sqlite3
        db = sqlite3.connect(os.path.join(added.ENGINE, 'vendor', 'BabelCDB', 'cards.cdb'))
        alias = {r[0]: r[1] for r in db.execute('select id, alias from datas')}
        data = {r[0]: {'type': r[1], 'attribute': r[2], 'race': r[3]} for r in db.execute('select id, type, attribute, race from datas')}
        learned = added.learn(added.client_rows(), data, added.passcodes(alias))
        self.assertIn(0x2021, learned['kind'], 'a Synchro Effect Monster is a kind the game shows')
        self.assertFalse([t for t in learned['kind'] if t & 0x800000], 'no Xyz kind in the client')


class Pure(unittest.TestCase):
    def test_fields_read_the_rows_second_word(self):
        f = added.fields(18 << 3 | 1 << 8 | 12 << 12 | 1 << 19 | 1 << 27)
        self.assertEqual((f['kind'], f['attribute'], f['level'], f['race'], f['art']), (18, 1, 12, 1, 1))


if __name__ == '__main__':
    unittest.main()
