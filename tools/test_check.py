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
        for group in check.GROUPS + check.LOCAL_GROUPS:
            self.assertTrue(check.steps(group), group)

    def test_local_groups_run_only_by_name(self):
        self.assertFalse(set(check.GROUPS) & set(check.LOCAL_GROUPS))
        self.assertNotIn('mod', [g for g, _ in check.plan(check.GROUPS)])
        self.assertEqual({g for g, _ in check.plan(['mod'])}, {'mod'})
        workflow = (ROOT / '.github/workflows/release.yml').read_text()
        self.assertNotIn('check.py mod', workflow)

    def test_ci_runs_the_list_instead_of_copying_it(self):
        workflow = (ROOT / '.github/workflows/release.yml').read_text()
        self.assertIn('python3 tools/check.py py', workflow)
        self.assertIn('python3 tools/check.py docs', workflow)
        copied = [t for t in check.PY if re.search(rf'python3? {re.escape(t)}\b', workflow)]
        self.assertEqual(copied, [], 'release.yml runs these itself as well as through tools/check.py py')

    def test_the_engine_sources_come_once_before_the_first_group_that_compiles_them(self):
        fetches = lambda groups: [g for g, step in check.plan(groups) if step == check.ENGINE]
        self.assertEqual(fetches(check.GROUPS), ['lint'])
        self.assertEqual(fetches(['rust']), ['rust'])
        self.assertEqual(fetches(['py', 'docs']), [])

    def test_dashboard_packages_are_installed_again_after_the_lock_changes(self):
        import os, tempfile
        with tempfile.TemporaryDirectory() as d:
            d = Path(d)
            (d / 'package-lock.json').write_text('{}')
            self.assertFalse(check.installed(d))
            (d / 'node_modules').mkdir()
            self.assertFalse(check.installed(d), 'node_modules without an install record')
            (d / 'node_modules' / '.package-lock.json').write_text('{}')
            os.utime(d / 'package-lock.json', (1, 1))
            self.assertTrue(check.installed(d))
            os.utime(d / 'package-lock.json', None)
            os.utime(d / 'node_modules' / '.package-lock.json', (1, 1))
            self.assertFalse(check.installed(d), 'the lock changed after the install')

    def test_unknown_groups_are_refused(self):
        self.assertEqual(check.main(['nonsense']), 2)


if __name__ == '__main__':
    unittest.main()
