#!/usr/bin/env python3
"""Actual checkout cleanup and Cargo rebuilds, plus bounded, repository-owned cache eviction."""
import importlib.util
from pathlib import Path
import subprocess
import shutil
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('local_cargo_cache', ROOT / '.github/scripts/local_cargo_cache.py')
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


class LocalCargoCache(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name)
        self.repo, self.home = root / 'repo', root / 'home'
        self.repo.mkdir(); self.home.mkdir()
        self.saved = self.home / '.cache/yo3-revival/cargo-target'
        # Fixtures may live on a small /tmp tmpfs; free-space behavior is tested explicitly below.
        disk = shutil.disk_usage(self.home)._replace(free=100 * 1024**3)
        space = patch.object(cache.shutil, 'disk_usage', return_value=disk)
        space.start(); self.addCleanup(space.stop)

    def run_cmd(self, *args):
        return subprocess.check_output(args, cwd=self.repo, text=True, stderr=subprocess.STDOUT)

    def test_checkout_cleanup_preserves_cache_and_changed_source_rebuilds(self):
        self.run_cmd('git', 'init', '-q')
        (self.repo / '.gitignore').write_text('/target/\n')
        (self.repo / 'Cargo.toml').write_text('[package]\nname="cache-proof"\nversion="0.0.0"\nedition="2024"\n')
        (self.repo / 'src').mkdir()
        source = self.repo / 'src/main.rs'
        source.write_text('fn main() { println!("first"); }\n')
        self.run_cmd('git', 'add', '.')
        cache.prepare(self.repo, self.home)
        self.assertEqual(self.run_cmd('cargo', 'run', '--quiet', '--release').strip(), 'first')
        sentinel = self.saved / 'kept'
        sentinel.write_text('cache survives')
        self.run_cmd('git', 'clean', '-ffdx')
        self.assertFalse((self.repo / 'target').exists())
        self.assertEqual(sentinel.read_text(), 'cache survives')
        cache.prepare(self.repo, self.home)
        source.write_text('fn main() { println!("second"); }\n')
        self.assertEqual(self.run_cmd('cargo', 'run', '--quiet', '--release').strip(), 'second')
        self.assertTrue(sentinel.exists())

    def test_only_owned_cache_is_evicted_at_size_or_free_space_limits(self):
        cache.prepare(self.repo, self.home)
        tool = self.home / '.cargo/bin/rustup'
        tool.parent.mkdir(parents=True); tool.write_text('tool installation')
        sentinel = self.saved / 'old'
        disk = shutil.disk_usage(self.saved)
        for limit in [patch.object(cache, 'MAX_BYTES', 0),
                      patch.object(cache.shutil, 'disk_usage', side_effect=[disk._replace(free=0), disk])]:
            sentinel.write_text('old artifact')
            with limit:
                self.assertIn('reset', cache.prepare(self.repo, self.home))
            self.assertFalse(sentinel.exists())
            self.assertEqual(tool.read_text(), 'tool installation')
            self.assertEqual((self.saved / '.yo3-cache-owner').read_text(), cache.OWNER)
            self.assertEqual((self.repo / 'target').resolve(), self.saved)

    def test_low_space_after_eviction_fails_before_a_build(self):
        cache.prepare(self.repo, self.home)
        low = shutil.disk_usage(self.saved)._replace(free=0)
        with patch.object(cache.shutil, 'disk_usage', return_value=low):
            with self.assertRaisesRegex(RuntimeError, 'free disk space'):
                cache.prepare(self.repo, self.home)

    def test_foreign_or_existing_target_and_unowned_cache_are_preserved(self):
        target = self.repo / 'target'; target.mkdir(); (target / 'keep').write_text('keep')
        with self.assertRaises(ValueError): cache.prepare(self.repo, self.home)
        self.assertEqual((target / 'keep').read_text(), 'keep')
        shutil.rmtree(target)
        target.symlink_to(self.home, target_is_directory=True)
        with self.assertRaises(ValueError): cache.prepare(self.repo, self.home)
        target.unlink()
        self.saved.mkdir(parents=True); (self.saved / 'foreign').write_text('keep')
        with self.assertRaises(ValueError): cache.prepare(self.repo, self.home)
        self.assertEqual((self.saved / 'foreign').read_text(), 'keep')

    def test_release_uses_local_cache_and_has_no_archive_action(self):
        import yaml
        flow = yaml.load((ROOT / '.github/workflows/release.yml').read_text(), Loader=yaml.BaseLoader)
        steps = flow['jobs']['release']['steps']
        self.assertFalse(any(s.get('uses', '').startswith('Swatinem/rust-cache@') for s in steps))
        names = [s.get('name') for s in steps]
        step = steps[names.index('Reuse the runner’s local Cargo output')]
        self.assertIn('local_cargo_cache.py', step['run'])
        self.assertLess(names.index(step['name']), names.index('Lints'))
        self.assertIn('test_local_cargo_cache.py', '\n'.join(s.get('run', '') for s in steps))


if __name__ == '__main__': unittest.main()
