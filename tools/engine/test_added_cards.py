#!/usr/bin/env python3
"""tools/engine/added-cards.py (#75): the committed engine/added-cards.json is what the table and the pinned data build,
and the table's rules hold. Needs engine/fetch.sh's data and the game (skipped without them)."""
import importlib.util
import os
import tempfile
import unittest
from unittest import mock
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

    def test_find_says_which_cards_fit_and_gives_their_lines(self):
        found = {name: (line, why) for name, line, why in added.find(
            ['black rose moonlight dragon', 'Number 39: Utopia', 'Stardust Dragon', 'Shooting Quasar Dragon', 'Nope Nope'])}
        line, why = found['Black Rose Moonlight Dragon']
        self.assertEqual((line.split('\t')[:4], why), (['4835', '11123', '', 'Black Rose Moonlight Dragon'], []), 'the next row, its Konami id')
        self.assertIn("can't show it", found['Number 39: Utopia'][1][0])
        self.assertEqual(found['Stardust Dragon'][1], ['the game already has it'])
        self.assertEqual(found['Shooting Quasar Dragon'][1], ['engine/added/cards.tsv already has it'])
        self.assertIn('check the spelling', found['Nope Nope'][1][0])

    def test_spells_and_traps_are_the_games_kinds(self):
        ctx = added.load()
        st = ctx['learned']['spell_trap']
        # (kind, attribute, race, subtype): 13 Spell / 14 Trap with attribute 8/9 and race 23/24, as every game card has.
        self.assertEqual(st[0x2], (13, 8, 23, 0))
        self.assertEqual(st[0x80002], (13, 8, 23, 2), 'Field')
        self.assertEqual(st[0x10002], (13, 8, 23, 5), 'Quick-Play')
        self.assertEqual(st[0x100004], (14, 9, 24, 1), 'Counter Trap')
        advanced_dark = ctx['data'][12644061]
        a, b, why = added.encode(advanced_dark, 10214, ctx['learned'])
        f = added.fields(b)
        self.assertEqual((why, a, f['kind'], f['attribute'], f['race'], f['subtype'], f['level']), ([], 10214, 13, 8, 23, 2, 0))

    def test_a_newer_card_gets_a_stand_in_id_the_client_can_hold(self):
        ctx = added.load()
        (name, line, why), = added.find(['18847598'], ctx)  # Advanced Crystal Beast Amber Mammoth, Konami id 17632
        self.assertEqual((name, why), ('Advanced Crystal Beast Amber Mammoth', []))
        _, cid, sid, _, _ = line.split('\t')
        sid = int(sid)
        self.assertEqual(cid, '17632')
        self.assertTrue(ctx['base'] <= sid <= 0x3FFF and sid not in ctx['in_game'] and sid not in ctx['konami'])
        self.assertEqual(added.find(['cid:17632'], ctx)[0][0], name, 'by Konami id too')
        self.assertEqual(added.find(['Advanced Crystal Beast Amber Mammoth', 'Advanced Crystal Beast Amethyst Cat'], ctx)[1][1].split('\t')[2],
                         str(added.next_stand_in(ctx, {sid})), 'each card of a batch its own stand-in')
        # The table takes it: the client's id is the stand-in, the real one is kept.
        with tempfile.TemporaryDirectory() as tmp:
            table = os.path.join(tmp, 'cards.tsv')
            with open(added.TABLE) as f:
                text = f.read()
            with open(table, 'w') as f:
                f.write(text + line + 'test\n')
            with mock.patch.object(added, 'TABLE', table):
                card = added.build()['cards'][-1]
            self.assertEqual((card['cid'], card['konami_id'], card['a'] & 0x3FFF, card['passcode']), (sid, 17632, sid, 18847598))
            for wrong, says in ((line.replace(f'\t{sid}\t', '\t\t'), 'give it a stand-in'),
                                (line.replace(f'\t{sid}\t', '\t3900\t'), 'is a card of the game'),
                                (line.replace(f'\t{sid}\t', '\t9709\t'), "already a card's Konami id")):
                with open(table, 'w') as f:
                    f.write(text + wrong + 'test\n')
                with mock.patch.object(added, 'TABLE', table), self.assertRaises(SystemExit) as e:
                    added.build()
                self.assertIn(says, str(e.exception))


class Pure(unittest.TestCase):
    def test_fields_read_the_rows_second_word(self):
        f = added.fields(18 << 3 | 1 << 8 | 12 << 12 | 1 << 19 | 1 << 27)
        self.assertEqual((f['kind'], f['attribute'], f['level'], f['race'], f['art']), (18, 1, 12, 1, 1))


if __name__ == '__main__':
    unittest.main()
