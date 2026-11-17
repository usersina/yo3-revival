#!/usr/bin/env python3
"""Release deployment transaction: validate and pull first, snapshot stopped data, check health, roll back failures."""
import importlib.util
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
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

    def test_state_query_exposes_only_deployed_revisions_without_docker_commands(self):
        self.worker.deploy(request())
        current=self.state/'current-release.json'
        current.write_text(json.dumps(json.loads(current.read_text())|{'registry_token':'never-export-this'}))
        self.calls.clear()
        snapshot=self.worker.execute({'kind':'state'})
        self.assertEqual(snapshot['server'],request()['server'])
        self.assertEqual(snapshot['dashboard_commit'],request()['commit'])
        self.assertEqual(snapshot['released_dashboard'],request()['dashboard'])
        self.assertNotIn('registry_token',snapshot)
        self.assertEqual(self.calls,[])

    def test_state_query_requires_a_completed_release_and_unknown_modes_do_nothing(self):
        with self.assertRaisesRegex(ValueError,'release'):
            self.worker.execute({'kind':'state'})
        for body in [{'kind':'shell','command':'id'},{'kind':'state','path':'/etc/shadow'},[]]:
            with self.assertRaises(ValueError):self.worker.execute(body)
        self.assertEqual(self.calls,[])

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

    def test_a_new_release_replaces_the_ui_and_records_both_revisions(self):
        old=request('v0.1.0')|{'commit':'c'*40,'dashboard_commit':'d'*40,
            'dashboard':'ghcr.io/usersina/yo3-revival-dashboard@sha256:'+'3'*64,
            'released_dashboard':'ghcr.io/usersina/yo3-revival-dashboard@sha256:'+'4'*64}
        (self.state/'current-release.json').write_text(json.dumps(old))
        self.worker.deploy(request())
        state=self.worker.execute({'kind':'state'})
        self.assertEqual(state['dashboard_commit'],request()['commit'])
        self.assertEqual(state['released_dashboard'],request()['dashboard'])
        self.assertEqual(state['previous_dashboard'],{'image':old['dashboard'],'commit':old['dashboard_commit']})


def dashboard_request(image='3',commit='b'):
    return request()|{'kind':'dashboard','dashboard':'ghcr.io/usersina/yo3-revival-dashboard@sha256:'+image*64,
        'dashboard_commit':commit*40,'released_dashboard':request()['dashboard'],
        'expected_dashboard':request()['dashboard'],'expected_dashboard_commit':request()['commit']}


class DashboardDeployment(unittest.TestCase):
    def setUp(self):
        self.ui_mode=False
        Deployment.setUp(self)
        self.worker.deploy(request())
        self.ui_mode=True;self.calls.clear()
        self.running_dashboard=request()['dashboard']
        self.actual_server='sha256:'+'1'*64
        (self.data/'server.db').write_text('live-database')
        self.before={name:(self.state/name).read_text() for name in ['current-release.json','first-provisioned.json']}
        self.override=(self.repo/'compose.production.yaml').read_text()
        runtime_patch=patch.object(deploy,'assert_dashboard_runtime',side_effect=self.check_runtime)
        self.runtime=runtime_patch.start();self.addCleanup(runtime_patch.stop)

    def check_runtime(self,*args):
        if self.failure=='runtime':raise ValueError('Unexpected runtime change')

    def fake_run(self,args,**kwargs):
        if not self.ui_mode:return Deployment.fake_run(self,args,**kwargs)
        self.calls.append(args)
        if args[:3]==['docker','image','inspect']:
            ref=args[-1];number=ref[-1]
            revision={'1':'a','2':'a','3':'b','4':'c'}[number]*40
            labels={'org.opencontainers.image.version':'v0.1.1','org.opencontainers.image.revision':revision,
                'io.yo3.dashboard.base':request()['dashboard'],'io.yo3.server.image':request()['server']}
            if self.failure=='labels' and number in '34':labels['io.yo3.dashboard.base']='wrong'
            return json.dumps([{'Id':'sha256:'+number*64,'Config':{'User':'10001:10001','Labels':labels}}])
        if args[:2]==['docker','inspect']:
            identifier=self.actual_server if args[-1]=='server-id' else 'sha256:'+self.running_dashboard[-1]*64
            return json.dumps([{'Image':identifier}])
        if args[:2]==['docker','pull'] and self.failure=='pull':raise RuntimeError('pull failed')
        if 'ps' in args:return 'server-id' if args[-1]=='server' else 'dashboard-id'
        if 'up' in args:
            text=(self.repo/'compose.production.yaml').read_text()
            self.running_dashboard=next((dashboard_request(i)['dashboard'] for i in ['3','4']
                if dashboard_request(i)['dashboard'] in text),request()['dashboard'])
            if self.failure=='start' and self.running_dashboard!=request()['dashboard']:raise RuntimeError('start failed')
        if 'http://127.0.0.1:8080/health' in args:
            if self.failure=='health' and self.running_dashboard!=request()['dashboard']:raise RuntimeError('health failed')
            return json.dumps({'ok':True,'release':'v0.1.1'})
        return ''

    def unchanged_data(self):
        self.assertEqual((self.data/'server.db').read_text(),'live-database')
        self.assertEqual((self.data/'server.key').read_text(),'original-identity')
        self.assertFalse(any('stop' in args or 'down' in args for args in self.calls))
        for args in self.calls:
            if 'up' in args:
                self.assertIn('--no-deps',args)
                self.assertEqual(args[-1],'dashboard')

    def test_success_replaces_only_dashboard_and_keeps_previous_good_and_released_images(self):
        self.worker.execute(dashboard_request())
        state=self.worker.execute({'kind':'state'})
        self.assertEqual(state['dashboard_commit'],'b'*40)
        self.assertEqual(state['dashboard'],dashboard_request()['dashboard'])
        self.assertEqual(state['released_dashboard'],request()['dashboard'])
        self.assertEqual(state['server'],request()['server'])
        self.assertEqual(state['previous_dashboard'],{'image':request()['dashboard'],'commit':'a'*40})
        self.runtime.assert_called_once_with(request()['dashboard'],dashboard_request()['dashboard'])
        self.unchanged_data()

    def test_stale_expected_revisions_and_tested_server_are_refused_before_pulling(self):
        for change in [{'expected_dashboard':dashboard_request()['dashboard']}, {'expected_dashboard_commit':'c'*40},
                       {'server':request()['server'][:-64]+'9'*64}, {'commit':'c'*40},
                       {'released_dashboard':dashboard_request()['dashboard']}]:
            with self.subTest(change=change),self.assertRaises(ValueError):self.worker.execute(dashboard_request()|change)
        self.assertEqual(self.calls,[])

    def test_foreign_images_tags_shell_fields_and_invalid_commits_are_refused(self):
        for change in [{'dashboard':'ubuntu:latest'},{'dashboard_commit':'main'},
                       {'expected_dashboard_commit':'--help'},{'command':'id'},
                       {'dashboard':'ghcr.io/other/project@sha256:'+'3'*64}]:
            with self.subTest(change=change),self.assertRaises(ValueError):self.worker.execute(dashboard_request()|change)
        self.assertEqual(self.calls,[])

    def test_actual_running_server_must_match_the_tested_digest(self):
        self.actual_server='sha256:'+'9'*64
        with self.assertRaisesRegex(ValueError,'running'):self.worker.execute(dashboard_request())
        self.assertFalse(any('up' in args for args in self.calls))

    def test_pull_labels_or_runtime_failures_do_not_change_the_stack(self):
        for failure in ['pull','labels','runtime']:
            self.failure=failure
            with self.subTest(failure=failure),self.assertRaises((ValueError,RuntimeError)):
                self.worker.execute(dashboard_request())
            self.assertEqual((self.repo/'compose.production.yaml').read_text(),self.override)
            self.assertFalse(any('up' in args for args in self.calls))
        self.unchanged_data()

    def test_start_or_health_failure_restores_only_dashboard_and_records(self):
        for failure in ['start','health']:
            self.failure=failure
            with self.subTest(failure=failure),self.assertRaisesRegex(RuntimeError,'rolled back'):
                self.worker.execute(dashboard_request())
            self.assertEqual((self.repo/'compose.production.yaml').read_text(),self.override)
            for name,content in self.before.items():self.assertEqual((self.state/name).read_text(),content)
            self.unchanged_data()

    def test_receipt_fsync_failure_after_rename_also_rolls_back(self):
        original=deploy.atomic_write;failed=False
        def write(path,content):
            nonlocal failed
            original(path,content)
            if path.name=='current-release.json' and not failed:
                failed=True;raise OSError('simulated fsync failure')
        with patch.object(deploy,'atomic_write',side_effect=write):
            with self.assertRaisesRegex(RuntimeError,'rolled back'):self.worker.execute(dashboard_request())
        for name,content in self.before.items():self.assertEqual((self.state/name).read_text(),content)
        self.unchanged_data()

    def test_lost_response_duplicate_and_concurrent_duplicates_do_not_restart_twice(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            list(pool.map(self.worker.execute,[dashboard_request(),dashboard_request()]))
        self.assertEqual(sum('up' in args for args in self.calls),1)
        self.calls.clear()
        self.worker.execute(dashboard_request())
        self.assertFalse(any('up' in args or args[:2]==['docker','pull'] for args in self.calls))
        self.unchanged_data()

    def test_late_job_cannot_overwrite_the_winner_and_release_retry_keeps_the_new_ui(self):
        self.worker.execute(dashboard_request());self.calls.clear()
        with self.assertRaises(ValueError):self.worker.execute(dashboard_request('4','c'))
        self.assertEqual(self.calls,[])
        self.worker.execute(request())
        self.assertFalse(any('up' in args or 'stop' in args for args in self.calls))
        self.assertEqual(self.worker.execute({'kind':'state'})['dashboard_commit'],'b'*40)

class Transport(unittest.TestCase):
    def test_dashboard_request_uses_a_json_file_and_ephemeral_registry_credentials(self):
        with tempfile.TemporaryDirectory() as directory:
            file=Path(directory)/'request.json';file.write_text(json.dumps(dashboard_request()))
            env={'GITHUB_ACTOR':'usersina','GITHUB_TOKEN':'temporary-token'}
            with patch.dict(os.environ,env,clear=True),patch.object(transport,'send',return_value='healthy') as send,\
                    patch('builtins.print'):
                transport.main(['dashboard',str(file)])
            expected=dashboard_request()|{'registry_user':'usersina','registry_token':'temporary-token'}
            send.assert_called_once_with(expected)
            self.assertNotIn('temporary-token',file.read_text())
    def test_state_query_uses_the_same_pinned_ssh_without_release_or_registry_credentials(self):
        env={'VPS_HOST':'192.0.2.1','VPS_SSH_KEY':'test-key','VPS_KNOWN_HOSTS':'test-host-key'}
        def ssh(args,**kwargs):
            self.assertEqual(args[-1],'yo3-deploy@192.0.2.1')
            self.assertIn('StrictHostKeyChecking=yes',args)
            self.assertEqual(json.loads(kwargs['input']),{'kind':'state'})
            return SimpleNamespace(stdout=json.dumps(request('v0.1.0')))
        with patch.dict(os.environ,env,clear=True),patch.object(transport.subprocess,'run',side_effect=ssh),\
                patch('builtins.print') as output:
            transport.main(['state'])
        self.assertEqual(json.loads(output.call_args.args[0]),request('v0.1.0'))

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
