#!/usr/bin/env python3
"""tools/engine/coverage.py: parts add up, cards are grouped by what happened to them, a sweep's TSV reads."""
import importlib.util
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('coverage', Path(__file__).with_name('coverage.py'))
coverage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(coverage)

INFO = {1: ('Spell', 0x2), 2: ('Effect', 0x21), 3: ('Normal', 0x11), 4: ('Trap', 0x4), 5: ('Quiet', 0x21)}


class Coverage(unittest.TestCase):
    def test_parts_add_up(self):
        a = {'duels': 2, 'pool': [1, 2], 'cards': {'1': {'activated': 1, 'resolved': 1, 'visible': 1}}}
        b = {'duels': 3, 'pool': [2, 3], 'cards': {'1': {'activated': 2}, '2': {'summoned': 4}}}
        duels, pool, cards = coverage.merge([a, b])
        self.assertEqual((duels, pool), (5, [1, 2, 3]))
        self.assertEqual(cards[1], {'activated': 3, 'resolved': 1, 'visible': 1, 'negated': 0, 'summoned': 0})
        self.assertEqual(cards[2]['summoned'], 4)

    def test_cards_are_grouped_least_tested_first_and_normal_monsters_left_out(self):
        cards = {1: dict(activated=1, resolved=1, visible=1, negated=0, summoned=0),
                 2: dict(activated=2, resolved=0, visible=0, negated=2, summoned=1),
                 4: dict(activated=1, resolved=1, visible=0, negated=0, summoned=0)}
        groups, plain = coverage.classify([1, 2, 3, 4, 5], cards, INFO)
        self.assertEqual(plain, 1)
        names = {title: [cid for cid, _, _ in group] for title, group in groups}
        self.assertEqual(names, {'never activated': [5], 'activated, never resolved': [2],
                                 'resolved, nothing visible': [4], 'resolved with a visible result': [1]})
        self.assertEqual([t for t, _ in groups][0], 'never activated')
        text = coverage.report(7, [1, 2, 3, 4, 5], cards, INFO)
        self.assertIn('7 duel(s)', text)
        self.assertIn('Quiet', text)
        self.assertNotIn('Normal', text)

    def test_the_decks_report_names_the_decks_holding_each_card_and_can_be_filtered(self):
        decks = {'NPC 171 Yusei': [2, 5], 'NPC 120 Yugi': [2, 1]}
        text = coverage.decks_report(3, [1, 2, 5], decks, {}, INFO, ['deck 9: seed 1, step 2'])
        self.assertIn('2 deck(s)', text)
        self.assertIn('Duels that stopped', text)
        only = coverage.decks_report(3, [1, 2, 5], decks, {}, INFO, [], only='yusei')
        self.assertIn('NPC 171 Yusei', only)
        self.assertNotIn('Yugi', only)
        self.assertNotIn('Spell', only)

    def test_unknown_cards_are_kept(self):
        groups, plain = coverage.classify([9], {}, {})
        self.assertEqual((groups[0][1][0][0], plain), (9, 0))

    def test_a_card_sweep_report_reads_as_one_part(self):
        with tempfile.TemporaryDirectory() as d:
            tsv = Path(d, 'card-sweep.tsv')
            tsv.write_text('cid\tname\tseed\tresult\tactivated\tresolved\tvisible\tsummoned\tturns\tdecided for the player\n'
                           '7696\tJunk Warrior\t1000\tok\t1\t1\t1\t1\t9\t\n'
                           '7687\tJunk Synchron\t1007\tFAIL seed 1007\t\t\t\t\t\t\n')
            part = coverage.from_sweep(tsv)
        self.assertEqual(part['duels'], 1)
        self.assertEqual(part['pool'], [7696])
        self.assertEqual(part['cards']['7696']['visible'], 1)

    def test_no_parts_is_an_error(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(coverage.main(['--dir', d, '--engine', d]), 2)


if __name__ == '__main__':
    unittest.main()
