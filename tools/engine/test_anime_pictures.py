#!/usr/bin/env python3
"""tools/engine/anime-pictures.py and engine/anime/pictures.tsv (issue #60): the list of the pictures Desktop offers to
fetch is exactly what the reviewed map says, one row per card, and holds no image."""
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('anime_pictures', Path(__file__).with_name('anime-pictures.py'))
pictures = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pictures)

MAP = json.loads((ROOT / 'engine/anime-cards.json').read_text(encoding='utf-8'))
ADDED = json.loads((ROOT / 'engine/added-cards.json').read_text(encoding='utf-8'))['cards']


class AnimePictures(unittest.TestCase):
    def test_the_committed_list_is_what_the_map_builds(self):
        self.assertEqual((ROOT / 'engine/anime/pictures.tsv').read_text(encoding='utf-8'), pictures.build(MAP, ADDED))

    def test_each_added_card_is_fetched_by_its_official_passcode(self):
        # #75: an added card has no picture in the game: Get pictures fetches its normal picture with the anime ones.
        rows = {int(r.split('\t')[0]): r.split('\t') for r in pictures.build(MAP, ADDED).splitlines()[1:]}
        mapped = {c['cid'] for c in MAP['cards']}
        for card in ADDED:
            if card['cid'] in mapped:  # it has an anime version: that picture first, its official one standing in
                self.assertEqual(rows[card['cid']][2], str(card['passcode']))
            elif card.get('anime_only'):  # an anime-only card has its anime picture alone
                self.assertEqual(rows[card['cid']][1:], [str(card['passcode'])] * 2)
            else:
                self.assertEqual(rows[card['cid']][1:], ['0', str(card['passcode'])])

    def test_every_card_of_the_map_has_one_row_with_a_passcode_to_fetch(self):
        rows = [line.split('\t') for line in pictures.build(MAP).splitlines()[1:]]
        self.assertEqual([int(r[0]) for r in rows], sorted(c['cid'] for c in MAP['cards']))
        self.assertGreaterEqual(len(rows), 418, 'the 418 researched anime cards, and the added cards with anime versions')
        for cid, anime, official in rows:
            self.assertTrue(int(official) > 0, cid)
            self.assertTrue(anime.isdigit(), cid)
        by = {int(r[0]): r for r in rows}
        # The Gods use ProjectIgnis' anime passcodes like any card; Temple of the Kings has none, so its official picture is fetched.
        self.assertEqual(by[4998][1:], ['513000135', '10000000'])
        self.assertEqual(by[5231][1:], ['0', '29762407'])
        self.assertEqual(by[8035][1:], ['513000079', '32646477'])

    def test_a_card_without_any_passcode_is_refused(self):
        with self.assertRaises(SystemExit):
            pictures.build({'cards': [{'cid': 1, 'anime': 0, 'official': 0}]})


if __name__ == '__main__':
    unittest.main()
