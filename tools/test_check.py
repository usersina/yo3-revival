#!/usr/bin/env python3
"""tools/check.py stays the one list: every Python test is registered, and CI runs the list instead of copying it."""
import importlib.util
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('check', ROOT / 'tools/check.py')
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class Registry(unittest.TestCase):
    def test_every_python_test_is_registered_once(self):
        found = {p.relative_to(ROOT).as_posix() for p in ROOT.glob('tools/**/test_*.py')}
        registered = set(check.PY) | set(check.OTHER)
        self.assertEqual(found - registered, set(), 'add these to PY (hermetic) or OTHER (with what they need) in tools/check.py')
        self.assertEqual(registered - found, set(), 'these registered tests no longer exist')
        self.assertEqual(len(check.PY), len(set(check.PY)))
        self.assertFalse(set(check.PY) & set(check.OTHER))

    def test_every_group_lists_its_steps(self):
        for group in check.GROUPS:
            self.assertTrue(check.steps(group), group)

    def test_ci_runs_the_list_instead_of_copying_it(self):
        workflow = (ROOT / '.github/workflows/release.yml').read_text()
        self.assertIn('python3 tools/check.py py', workflow)
        self.assertIn('python3 tools/check.py docs', workflow)
        copied = [t for t in check.PY if re.search(rf'python3? {re.escape(t)}\b', workflow)]
        self.assertEqual(copied, [], 'release.yml runs these itself as well as through tools/check.py py')

    def test_unknown_groups_are_refused(self):
        self.assertEqual(check.main(['nonsense']), 2)


if __name__ == '__main__':
    unittest.main()
