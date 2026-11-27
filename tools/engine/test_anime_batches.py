#!/usr/bin/env python3
"""tools/engine/anime-batches.py (#75): each batch's work list names a card once, even when the inventory lists a Token
of the batch as one of its cards too (Stone Giant Token, batch 4), so no card is added twice."""
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / 'tools/engine/anime-batches.py'
BATCHES = sorted({line.split('\t')[0] for line in (ROOT / 'engine/added/anime-batches.tsv').read_text(encoding='utf-8').splitlines()[1:]}, key=int)


class AnimeBatches(unittest.TestCase):
    def test_a_batch_names_each_card_once(self):
        for batch in BATCHES:
            out = subprocess.run([sys.executable, str(TOOL), '--batch', batch], capture_output=True, text=True, check=True).stdout.split()
            codes = [line.split(':', 1)[1] for line in out]
            self.assertEqual(len(codes), len(set(codes)), f'batch {batch}: {sorted(c for c in codes if codes.count(c) > 1)}')

    def test_the_token_of_batch_4_is_added_once_as_a_token(self):
        out = subprocess.run([sys.executable, str(TOOL), '--batch', '4'], capture_output=True, text=True, check=True).stdout.split()
        self.assertIn('token:511000127', out)
        self.assertNotIn('anime:511000127', out)


if __name__ == '__main__':
    unittest.main()
