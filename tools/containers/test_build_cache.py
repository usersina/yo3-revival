#!/usr/bin/env python3
"""Exercise the real Docker planner: test/source edits reuse dependencies; build inputs invalidate them."""
import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
BUILDER = None


class DependencyRecipe(unittest.TestCase):
    def test_recipe_tracks_dependencies_but_not_test_files_source_or_release_version(self):
        with tempfile.TemporaryDirectory(prefix='yo3-recipe-') as tmp:
            root = Path(tmp)
            context = root / 'context'
            context.mkdir()
            for name in ['Cargo.toml', 'Cargo.lock', '.dockerignore']:
                shutil.copy2(ROOT / name, context / name)
            shutil.copytree(ROOT / 'crates', context / 'crates')
            dockerfile = root / 'Dockerfile'
            dockerfile.write_text((ROOT / 'deploy/Dockerfile.server').read_text() +
                '\nFROM scratch AS recipe-check\nCOPY --from=planner /src/recipe.json /recipe.json\n')
            count = 0

            def recipe():
                nonlocal count
                count += 1
                destination = root / str(count)
                command = ['docker', 'buildx', 'build']
                if BUILDER:
                    command += ['--builder', BUILDER]
                command += ['--progress=plain', '--target', 'recipe-check', '--output',
                            f'type=local,dest={destination}', '-f', str(dockerfile), str(context)]
                result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                self.assertEqual(result.returncode, 0, result.stdout[-6000:])
                return json.loads((destination / 'recipe.json').read_text())

            baseline = recipe()
            main = context / 'crates/yo3-server/src/main.rs'
            main.write_text(main.read_text() + '\n// Cache regression: a production source edit.\n')
            self.assertEqual(recipe(), baseline, 'production source is compiled after the dependency layer')
            test = context / 'crates/yo3-server/tests/cache_regression.rs'
            test.write_text('#[test] fn new_contract() { assert_eq!(1, 1); }\n')
            self.assertEqual(recipe(), baseline, 'adding a contract must not rebuild production dependencies')
            manifest = context / 'Cargo.toml'
            manifest.write_text(re.sub(r'(\[workspace.package\]\nversion = )"[^"]+"',
                                      r'\1"99.0.0"', manifest.read_text()))
            lock = context / 'Cargo.lock'
            lock.write_text(re.sub(r'(name = "yo3-[^"]+"\nversion = )"[^"]+"',
                                  r'\1"99.0.0"', lock.read_text()))
            self.assertEqual(recipe(), baseline, 'a release version must not rebuild third-party dependencies')
            server = context / 'crates/yo3-server/Cargo.toml'
            original = server.read_text()
            self.assertIn('serde_json.workspace = true', original)
            server.write_text(original.replace('serde_json.workspace = true',
                'serde_json = { workspace = true, features = ["raw_value"] }', 1))
            self.assertNotEqual(recipe(), baseline, 'changed dependency features must invalidate the cache')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--builder', help='existing Buildx builder; defaults to the selected builder')
    args, rest = parser.parse_known_args()
    BUILDER = args.builder
    unittest.main(argv=[__file__, *rest])
