#!/usr/bin/env python3
"""Release deployment transaction: validate and pull first, snapshot stopped data, check health, roll back failures."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
SPEC=importlib.util.spec_from_file_location('deploy_release',ROOT/'deploy/deploy-release.py')
deploy=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(deploy)
TRANSPORT_SPEC=importlib.util.spec_from_file_location('deploy_vps',ROOT/'.github/scripts/deploy_vps.py')
transport=importlib.util.module_from_spec(TRANSPORT_SPEC)
TRANSPORT_SPEC.loader.exec_module(transport)

def request(version='v0.1.1'):
    return {'version':version,'commit':'a'*40,'server':'ghcr.io/usersina/yo3-revival-server@sha256:'+'1'*64,
            'dashboard':'ghcr.io/usersina/yo3-revival-dashboard@sha256:'+'2'*64}

class Deployment(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.repo=self.root/'app';self.state=self.root/'state';self.data=self.root/'volume'
        for folder in [self.repo,self.state,self.data]:folder.mkdir()
        (self.repo/'.env').write_text('YO3_DOMAIN=localhost\n')
        (self.repo/'compose.production.yaml').write_text('old-images')
        (self.data/'server.db').write_text('old-database')
        (self.data/'server.key').write_text('original-identity')
        (self.state/'first-provisioned.json').write_text(json.dumps({'domain':'localhost','server_image':'old-server','dashboard_image':'old-dashboard'}))
        self.calls=[];self.failure=None;self.new=False
        self.patch=patch.object(deploy,'run',side_effect=self.fake_run);self.patch.start();self.addCleanup(self.patch.stop)
        self.worker=deploy.Deployer(self.repo,self.state,'yo3-test')

    def fake_run(self,args,**kwargs):
        self.calls.append(args)
        if args[:3]==['docker','image','inspect']:
            labels={'org.opencontainers.image.version':'v0.1.1','org.opencontainers.image.revision':'a'*40}
            if self.failure=='labels':labels['org.opencontainers.image.revision']='wrong'
            return json.dumps([{'Config':{'Labels':labels}}])
        if args[:2]==['docker','inspect']:
            return json.dumps([{'Mounts':[{'Type':'volume','Destination':'/var/lib/yo3','Source':str(self.data)}]}])
        if args[:2]==['docker','pull'] and self.failure=='pull':raise RuntimeError('pull failed')
        if 'ps' in args:return 'container-id\n'
        if 'up' in args:
            self.new=(self.repo/'compose.production.yaml').read_text()!='old-images'
            if self.new:
                (self.data/'server.db').write_text('new-schema')
                if self.failure=='start':raise RuntimeError('startup failed')
        if 'http://127.0.0.1:8080/health' in args:
            if self.failure=='health' and self.new:raise RuntimeError('health failed')
            if self.failure=='wrong-version' and self.new:return json.dumps({'ok':True,'release':'v0.1.0'})
            return json.dumps({'ok':True,'release':'v0.1.1' if self.new else 'old'})
        if '--resolve' in args and self.failure=='https' and self.new:raise RuntimeError('HTTPS failed')
        return ''

    def test_success_pulls_both_images_before_stop_and_retains_a_rollback_snapshot(self):
        self.worker.deploy(request())
        stop=next(i for i,args in enumerate(self.calls) if 'stop' in args)
        self.assertEqual(sum(args[:2]==['docker','pull'] for args in self.calls[:stop]),2)
        self.assertEqual((self.state/'release-rollback/data/server.db').read_text(),'old-database')
        self.assertEqual((self.data/'server.key').read_text(),'original-identity')
        self.assertEqual(json.loads((self.state/'current-release.json').read_text())['version'],'v0.1.1')
        setup=json.loads((self.state/'first-provisioned.json').read_text())
        self.assertEqual(setup['server_image'],request()['server'])
        self.assertEqual(setup['domain'],'localhost')

    def test_pull_or_wrong_image_identity_never_stops_the_old_server(self):
        for failure in ['pull','labels']:
            with self.subTest(failure=failure):
                self.calls.clear();self.failure=failure
                with self.assertRaises(RuntimeError):self.worker.deploy(request())
                self.assertFalse(any('stop' in args for args in self.calls))
                self.assertEqual((self.data/'server.db').read_text(),'old-database')

    def test_start_health_and_https_failures_restore_the_old_images_and_data(self):
        for failure in ['start','health','https','wrong-version']:
            with self.subTest(failure=failure):
                self.failure=failure
                with self.assertRaisesRegex(RuntimeError,'rolled back'):self.worker.deploy(request())
                self.assertEqual((self.repo/'compose.production.yaml').read_text(),'old-images')
                self.assertEqual((self.data/'server.db').read_text(),'old-database')
                self.assertEqual((self.data/'server.key').read_text(),'original-identity')
                self.assertFalse((self.state/'current-release.json').exists())

    def test_a_failed_receipt_write_after_rename_also_rolls_back_the_receipt(self):
        original=deploy.atomic_write
        def write(path,content):
            original(path,content)
            if path.name=='current-release.json':raise OSError('simulated fsync failure')
        with patch.object(deploy,'atomic_write',side_effect=write):
            with self.assertRaisesRegex(RuntimeError,'rolled back'):self.worker.deploy(request())
        self.assertFalse((self.state/'current-release.json').exists())
        self.assertEqual((self.data/'server.db').read_text(),'old-database')

    def test_a_duplicate_release_checks_health_without_restarting(self):
        self.worker.deploy(request());self.calls.clear()
        self.worker.deploy(request())
        self.assertFalse(any('stop' in args or 'up' in args for args in self.calls))

    def test_a_stale_release_or_reused_version_cannot_replace_a_newer_deployment(self):
        self.worker.deploy(request());self.calls.clear()
        for other in [request('v0.1.0'),request()|{'commit':'b'*40}]:
            with self.assertRaises(ValueError):self.worker.deploy(other)
        self.assertFalse(any('stop' in args for args in self.calls))

    def test_only_this_repositories_digest_pinned_images_are_accepted(self):
        for changes in [{'server':'ubuntu:latest'},{'version':'v0.1.1;id'},{'commit':'main'},
                        {'dashboard':'ghcr.io/other/project@sha256:'+'2'*64}]:
            with self.assertRaises(ValueError):self.worker.deploy(request()|changes)
        self.assertEqual(self.calls,[])

    def test_a_failed_snapshot_restarts_without_replacing_the_database(self):
        with patch.object(deploy,'copy_data',side_effect=OSError('disk full')):
            with self.assertRaisesRegex(RuntimeError,'rolled back'):self.worker.deploy(request())
        self.assertEqual((self.data/'server.db').read_text(),'old-database')
        self.assertEqual((self.repo/'compose.production.yaml').read_text(),'old-images')

class Transport(unittest.TestCase):
    def test_secrets_use_private_temporary_files_and_payload_stdin_not_shell_arguments(self):
        release=request()
        env={'VPS_HOST':'192.0.2.1','VPS_SSH_KEY':'private-test-key','VPS_KNOWN_HOSTS':'trusted-test-host-key',
             'SERVER_IMAGE':release['server'],'DASHBOARD_IMAGE':release['dashboard'],'VERSION':release['version'],
             'SOURCE_REF':release['commit'],'GITHUB_ACTOR':'usersina','GITHUB_TOKEN':'temporary-test-token'}
        paths=[]
        def ssh(args,**kwargs):
            self.assertEqual(args[0],'ssh');self.assertEqual(args[-1],'yo3-deploy@192.0.2.1')
            self.assertIn('StrictHostKeyChecking=yes',args)
            self.assertNotIn('private-test-key',args);self.assertNotIn('temporary-test-token',args)
            key=Path(args[args.index('-i')+1]);paths.append(key)
            self.assertEqual(key.stat().st_mode&0o777,0o600)
            self.assertEqual(key.read_text(),'private-test-key\n')
            payload=json.loads(kwargs['input']);self.assertEqual(payload['server'],release['server'])
            self.assertEqual(payload['registry_token'],'temporary-test-token')
        with patch.dict(os.environ,env,clear=True),patch.object(transport.subprocess,'run',side_effect=ssh):
            transport.main()
        self.assertTrue(paths);self.assertFalse(paths[0].exists())

    def test_missing_secrets_fail_before_ssh(self):
        with patch.dict(os.environ,{},clear=True),patch.object(transport.subprocess,'run') as ssh:
            with self.assertRaisesRegex(SystemExit,'VPS_SSH_KEY'):transport.main()
            ssh.assert_not_called()


if __name__=='__main__':unittest.main()
