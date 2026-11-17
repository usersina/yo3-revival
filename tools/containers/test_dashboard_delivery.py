#!/usr/bin/env python3
"""Cumulative dashboard eligibility: real Git histories, never production SSH or data."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('delivery', ROOT/'.github/scripts/dashboard_delivery.py')
delivery = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(delivery)


class Eligibility(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.invalid')
        self.git('config', 'user.name', 'Test')
        for path in ['dashboard/src/App.tsx', 'dashboard/src/api/generated/client.ts',
                     'dashboard/api/openapi.json', 'dashboard/public/logo.svg',
                     'dashboard/package.json', 'dashboard/package-lock.json',
                     'dashboard/vite.config.ts', 'deploy/nginx/nginx.conf',
                     'crates/server.rs', 'README.md']:
            self.write(path, 'initial\n')
        self.release = self.commit()
        self.git('tag', 'v0.1.0')
        self.state = {'version': 'v0.1.0', 'commit': self.release,
                      'server': 'ghcr.io/usersina/yo3-revival-server@sha256:'+'1'*64,
                      'dashboard': 'ghcr.io/usersina/yo3-revival-dashboard@sha256:'+'2'*64}

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], text=True).strip()

    def write(self, path, text):
        target = self.repo/path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'fixture', '--allow-empty')
        return self.git('rev-parse', 'HEAD')

    def plan(self):
        return delivery.plan(self.repo, self.state, self.git('rev-parse', 'HEAD'))

    def test_source_assets_vite_and_dependency_changes_are_deliberately_allowed(self):
        for path in ['dashboard/src/New.tsx', 'dashboard/public/new.png', 'dashboard/index.html',
                     'dashboard/vite.config.ts', 'dashboard/package.json', 'dashboard/package-lock.json']:
            with self.subTest(path=path):
                self.write(path, 'changed\n')
                self.commit()
                result = self.plan()
                self.assertTrue(result['eligible'], result)
                self.assertIn(path, result['changes'])
                self.assertEqual(result['server'], self.state['server'])
                self.assertEqual(result['expected_dashboard'], self.state['dashboard'])

    def test_unrelated_rust_and_docs_do_not_block_or_trigger_ui_delivery(self):
        self.write('crates/server.rs', 'unreleased behaviour\n')
        self.write('dashboard/README.md', 'documentation\n')
        self.commit()
        self.assertFalse(self.plan()['eligible'])
        self.write('dashboard/src/App.tsx', 'UI\n')
        self.commit()
        self.assertTrue(self.plan()['eligible'])

    def test_unknown_dashboard_inputs_wait_for_release(self):
        for path in ['dashboard/tsconfig.json', 'dashboard/new.config.ts', 'dashboard/scripts/build.mjs']:
            with self.subTest(path=path):
                self.write(path, 'unknown\n')
                self.write('dashboard/src/App.tsx', path)
                self.commit()
                result = self.plan()
                self.assertFalse(result['eligible'])
                self.assertIn(path, result['blocked'])

    def test_runtime_changes_wait_even_after_a_later_ui_only_push(self):
        self.write('deploy/nginx/nginx.conf', 'new CSP\n')
        self.commit()  # This run fails or is skipped.
        self.write('dashboard/src/App.tsx', 'apparently UI-only next push\n')
        self.commit()
        result = self.plan()
        self.assertFalse(result['eligible'])
        self.assertIn('deploy/nginx/nginx.conf', result['blocked'])

    def test_new_compose_overrides_and_root_docker_recipes_require_release(self):
        for path in ['compose.override.yaml', 'docker-compose.production.yml', 'Dockerfile.dashboard']:
            with self.subTest(path=path):
                self.write(path, 'new deployment input\n')
                self.write('dashboard/src/App.tsx', path)
                self.commit()
                result = self.plan()
                self.assertFalse(result['eligible'])
                self.assertIn(path, result['blocked'])

    def test_contract_and_generated_files_compare_with_release_not_last_ui(self):
        for path in ['dashboard/api/openapi.json', 'dashboard/src/api/generated/client.ts',
                     'dashboard/src/api/generated/new.ts']:
            with self.subTest(path=path):
                self.write(path, 'unreleased API\n')
                previous = self.commit()
                self.state['dashboard_commit'] = previous
                self.write('dashboard/src/App.tsx', path)
                self.commit()
                result = self.plan()
                self.assertFalse(result['eligible'])
                self.assertIn(path, result['contract_changes'])

    def test_renames_and_deletions_check_both_paths(self):
        self.git('mv', 'dashboard/src/App.tsx', 'dashboard/new-build.ts')
        self.commit()
        self.assertFalse(self.plan()['eligible'])

    def test_dashboard_baseline_is_the_actual_deployed_commit(self):
        self.write('dashboard/src/App.tsx', 'already deployed\n')
        deployed = self.commit()
        self.state['dashboard_commit'] = deployed
        self.write('README.md', 'docs only\n')
        self.commit()
        self.assertFalse(self.plan()['eligible'])
        self.write('dashboard/src/App.tsx', 'new UI\n')
        self.commit()
        result = self.plan()
        self.assertTrue(result['eligible'])
        self.assertEqual(result['expected_dashboard_commit'], deployed)

    def test_delayed_candidate_cannot_overwrite_a_newer_dashboard(self):
        self.write('dashboard/src/App.tsx', 'newer deployed UI\n')
        self.state['dashboard_commit'] = self.commit()
        with self.assertRaisesRegex(ValueError, 'ancestor'):
            delivery.plan(self.repo, self.state, self.release)

    def test_release_tag_must_resolve_to_recorded_server_commit(self):
        self.write('dashboard/src/App.tsx', 'UI\n')
        self.commit()
        self.state['commit'] = self.git('rev-parse', 'HEAD')
        with self.assertRaisesRegex(ValueError, 'release tag'):
            self.plan()

    def test_rejects_non_digest_images_invalid_commits_and_missing_state(self):
        for change in [{'server': 'ubuntu:latest'}, {'dashboard': 'ghcr.io/other/dashboard@sha256:'+'2'*64},
                       {'commit': 'main'}, {'dashboard_commit': '--help'}, {'version': 'main'}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                delivery.plan(self.repo, self.state | change, self.release)
        with self.assertRaises(ValueError):
            delivery.plan(self.repo, {}, self.release)

    def test_allowlisted_symlinks_cannot_read_files_outside_the_frontend(self):
        (self.repo/'dashboard/src/escape.ts').symlink_to('../../crates/server.rs')
        self.commit()
        self.assertFalse(self.plan()['eligible'])

    def test_real_command_writes_a_plan_and_returns_success_for_a_deferred_change(self):
        self.write('dashboard/new.config.ts', 'deferred\n')
        candidate = self.commit()
        state = self.repo/'state.json'
        state.write_text(json.dumps(self.state))
        result = subprocess.run(['python3', str(ROOT/'.github/scripts/dashboard_delivery.py'),
                                 '--repo', str(self.repo), '--state', str(state), '--candidate', candidate],
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(json.loads(result.stdout)['eligible'])

    def test_only_an_eligible_plan_becomes_a_minimal_dashboard_request(self):
        candidate_image='ghcr.io/usersina/yo3-revival-dashboard@sha256:'+'3'*64
        with self.assertRaises(ValueError):delivery.deployment_request(self.plan(),candidate_image)
        self.write('dashboard/src/App.tsx','UI\n');candidate=self.commit()
        result=delivery.deployment_request(self.plan(),candidate_image)
        self.assertEqual(result['kind'],'dashboard')
        self.assertEqual(result['dashboard_commit'],candidate)
        self.assertEqual(result['expected_dashboard'],self.state['dashboard'])
        self.assertNotIn('eligible',result)
        self.assertNotIn('changes',result)
        with self.assertRaises(ValueError):delivery.deployment_request(self.plan(),'ubuntu:latest')


if __name__ == '__main__':
    unittest.main()
