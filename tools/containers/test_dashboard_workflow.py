#!/usr/bin/env python3
"""Delivery wiring: trusted main only, one queue, exact image gates before mutation, private runner credentials."""
from pathlib import Path
import unittest
import yaml

ROOT=Path(__file__).resolve().parents[2]
def workflow(name):return yaml.load((ROOT/'.github/workflows'/name).read_text(),Loader=yaml.BaseLoader)

class DashboardWorkflow(unittest.TestCase):
    def test_trigger_reconciles_every_successful_main_run_and_allows_manual_retry(self):
        value=workflow('dashboard.yml')
        self.assertEqual(set(value['on']),{'workflow_run','workflow_dispatch'})
        self.assertEqual(value['on']['workflow_run'],{'workflows':['Release'],'types':['completed'],'branches':['main']})
        condition=value['jobs']['deliver']['if']
        for check in ["github.ref == 'refs/heads/main'", "github.event.workflow_run.conclusion == 'success'",
                      'github.event.workflow_run.head_repository.full_name == github.repository']:
            self.assertIn(check,condition)
        checkout=next(s for s in value['jobs']['deliver']['steps'] if s.get('uses','').startswith('actions/checkout@'))
        self.assertEqual(checkout['with']['ref'],'main')
        self.assertEqual(checkout['with']['fetch-depth'],'0')
        self.assertEqual(checkout['with']['persist-credentials'],'false')

    def test_release_and_ui_share_a_non_cancelling_fifo_and_use_the_home_runner(self):
        release,ui=workflow('release.yml'),workflow('dashboard.yml')
        self.assertEqual(release['concurrency'],ui['concurrency'])
        self.assertEqual(ui['concurrency']['queue'],'max')
        self.assertEqual(ui['concurrency']['cancel-in-progress'],'false')
        for name in ['release.yml','containers.yml','dashboard.yml']:
            for job in workflow(name)['jobs'].values():
                if 'runs-on' in job:self.assertEqual(job['runs-on'],['self-hosted','home'])
                for step in job.get('steps',[]):
                    if step.get('uses','').startswith('actions/checkout@'):
                        self.assertEqual(step.get('with',{}).get('persist-credentials'),'false')

    def test_eligibility_and_both_image_gates_precede_deployment_and_safe_cleanup(self):
        job=workflow('dashboard.yml')['jobs']['deliver']
        self.assertNotIn('VPS_SSH_KEY',job['env'],'npm/build/browser steps do not need the deployment key')
        steps=job['steps'];text='\n'.join(s.get('run','') for s in steps)
        names=[s.get('name') for s in steps]
        expected=['Read deployed state and plan against current main','Check and build dashboard',
                  'Build the asset-only image','Verify exact runtime and released-server browser behavior',
                  'Publish the verified dashboard','Deploy the verified dashboard',
                  'Prove workflow-token package deletion','Retain release and active dashboard images']
        self.assertEqual(sorted(expected,key=names.index),expected)
        for name in expected[1:]:
            step=steps[names.index(name)]
            self.assertEqual(step['if'],"steps.plan.outputs.eligible == 'true'")
        self.assertIn('test_dashboard_browser.py --server',text)
        self.assertIn('--released-dashboard',text)
        self.assertNotIn('cargo ',text,'UI delivery never builds unreleased Rust')
        self.assertNotIn('Dockerfile.server',text)
        self.assertIn('deploy_vps.py state',steps[names.index(expected[-1])]['run'])
        self.assertIn('--state',steps[names.index(expected[-1])]['run'])
        self.assertIn('RUNNER_TEMP',steps[0]['run'])
        self.assertIn('DOCKER_CONFIG',steps[0]['run'])
        self.assertEqual(steps[-1]['if'],'always()')
        self.assertIn('DOCKER_CONFIG',steps[-1]['run'])

    def test_actionlint_uses_its_own_checksummed_release_not_a_rust_installer(self):
        steps=workflow('release.yml')['jobs']['release']['steps']
        installer=next(s for s in steps if s.get('uses','').startswith('taiki-e/install-action@'))
        self.assertEqual(installer['with']['tool'],'cargo-nextest')
        lint=next(s for s in steps if s.get('name')=='Install actionlint')
        self.assertIn('rhysd/actionlint/releases/download/v1.7.12/',lint['run'])
        self.assertIn('sha256sum -c',lint['run'])
        self.assertIn('GITHUB_PATH',lint['run'])

    def test_release_cleanup_reads_host_state_and_probes_before_deleting(self):
        steps=workflow('release.yml')['jobs']['cleanup']['steps']
        names=[s.get('name') for s in steps]
        self.assertLess(names.index('Prove workflow-token package deletion'),names.index('Remove old downloads and image versions after healthy deployment'))
        cleanup=steps[names.index('Remove old downloads and image versions after healthy deployment')]
        self.assertIn('deploy_vps.py state',cleanup['run'])
        self.assertIn('--state',cleanup['run'])

if __name__=='__main__':unittest.main()
