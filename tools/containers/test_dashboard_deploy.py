#!/usr/bin/env python3
"""Real dashboard transaction on a disposable stack and loopback registry; never the VPS or default ports.

First build dashboard/dist. Arguments match test_dashboard_browser.py: --server IMAGE --dashboard RELEASED_IMAGE.
The production command's repository constant is replaced ONLY in this test process with our private test registry.
All Docker/Compose, runtime verification, state writes, HTTPS and browser checks execute normally.
"""
import importlib.util
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request
import uuid
from unittest.mock import patch
import test_dashboard_browser as browser

ROOT=Path(__file__).resolve().parents[2]
SPEC=importlib.util.spec_from_file_location('deployer',ROOT/'deploy/deploy-release.py')
deploy=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(deploy)


def command(*args):
    result=subprocess.run(args,check=False,text=True,capture_output=True,timeout=300)
    if result.returncode:raise RuntimeError(result.stdout+result.stderr)
    return result.stdout.strip()


def rehearse(scratch,compose,args,verify):
    name='yo3-ui-registry-'+uuid.uuid4().hex[:12];port=browser.free_port()
    prefix=f'127.0.0.1:{port}/yo3-{uuid.uuid4().hex[:10]}'
    references=[]
    command('docker','run','--rm','-d','--name',name,'-p',f'127.0.0.1:{port}:5000','registry:2')
    try:
        for attempt in range(40):
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{port}/v2/',timeout=2):break
            except OSError:
                if attempt==39:raise
                time.sleep(0.2)

        def publish(source,service,tag):
            reference=f'{prefix}-{service}:{tag}'
            command('docker','tag',source,reference);references.append(reference)
            command('docker','push',reference)
            inspected=json.loads(command('docker','image','inspect',reference))[0]
            digest=next(ref for ref in inspected['RepoDigests'] if ref.startswith(prefix+'-'+service+'@'))
            references.append(digest)
            return digest

        server=publish(args.server,'server','release')
        released=publish(args.dashboard,'dashboard','release')
        details=json.loads(command('docker','image','inspect',server))[0]
        labels=details['Config']['Labels']
        state={'version':labels['org.opencontainers.image.version'],'commit':labels['org.opencontainers.image.revision'],
               'server':server,'dashboard':released}
        deploy.IMAGE_PREFIX=prefix
        state_dir=scratch/'deployment';state_dir.mkdir()
        record=state_dir/'current-release.json';record.write_text(json.dumps(state))
        setup=state_dir/'first-provisioned.json'
        setup.write_text(json.dumps({'domain':'localhost','server_image':server,'dashboard_image':released}))
        override=scratch/'compose.production.yaml';override.write_text(deploy.image_override(state))
        config=json.loads((scratch/'compose.json').read_text())
        https=config['services']['dashboard']['environment']['YO3_HTTPS_PORT']
        (scratch/'.env').write_text(f'YO3_DOMAIN=localhost\nYO3_HTTPS_PORT={https}\n')
        worker=deploy.Deployer(scratch,state_dir)
        worker.compose=compose+['-f',str(override)]

        def container(service):return worker.command('ps','--quiet',service).strip()
        def accounts():return worker.command('exec','-T','server','/opt/yo3/yo3-server','--config','/etc/yo3/server.toml','accounts')

        original_server=container('server')
        certificate=worker.command('exec','-T','server','cat','/var/lib/yo3/server.crt')
        assets=ROOT/'dashboard/dist'
        if not (assets/'index.html').is_file():raise RuntimeError('Build dashboard/dist before this rehearsal')
        candidates=[]
        for revision in ['b'*40,'c'*40]:
            tag=f'yo3-ui-rehearsal-{uuid.uuid4().hex}:candidate';references.append(tag)
            command('docker','build','-q','-f',str(ROOT/'deploy/Dockerfile.dashboard-assets'),
                    '--build-arg',f'RELEASED_DASHBOARD={released}','--build-arg',f'TESTED_SERVER={server}',
                    '--build-arg',f'DASHBOARD_COMMIT={revision}','-t',tag,str(assets))
            candidates.append(publish(tag,'dashboard','dashboard-'+revision))
        command('python3',str(ROOT/'tools/containers/test_images.py'),'--dashboard-only',
                '--released-dashboard',released,'--dashboard',candidates[0])

        def request(image,revision,expected):
            return state|{'kind':'dashboard','dashboard':image,'dashboard_commit':revision,
                'released_dashboard':released,'expected_dashboard':expected['dashboard'],
                'expected_dashboard_commit':expected.get('dashboard_commit',expected['commit'])}

        with patch.dict(os.environ,{'CURL_CA_BUNDLE':str(scratch/'tls/fullchain.pem')}):
            first=request(candidates[0],'b'*40,state)
            worker.execute(first);verify()
            dashboard=container('dashboard');saved=record.read_text();people=accounts()
            worker.execute(first)
            assert container('dashboard')==dashboard,'Duplicate restarted the dashboard'
            try:worker.execute(request(candidates[1],'c'*40,state))
            except ValueError:pass
            else:raise AssertionError('Stale deployment was accepted')
            second=request(candidates[1],'c'*40,json.loads(saved))
            healthy=worker.health;failed=False
            def transient_failure(expected=None):
                nonlocal failed
                healthy(expected)
                if not failed:
                    failed=True;raise RuntimeError('Injected temporary HTTPS health failure')
            with patch.object(worker,'health',side_effect=transient_failure):
                try:worker.execute(second)
                except RuntimeError as error:assert 'rolled back' in str(error),str(error)
                else:raise AssertionError('Injected health failure did not roll back')
            assert record.read_text()==saved,'Rollback changed the deployment receipt'
            assert worker.execute({'kind':'state'})['dashboard']==candidates[0]
            worker.execute(second)
            current=worker.execute({'kind':'state'})
            assert current['previous_dashboard']=={'image':candidates[0],'commit':'b'*40}
            dashboard=container('dashboard');worker.execute(state)
            assert container('dashboard')==dashboard,'Retrying the release reverted a newer dashboard'
            assert container('server')==original_server,'The server container was replaced'
            assert accounts()==people,'Dashboard deployment changed account data'
            assert worker.command('exec','-T','server','cat','/var/lib/yo3/server.crt')==certificate
        print('PASS: actual dashboard replacement, browser gate, duplicate/stale requests, health rollback and retry; '
              'server container, accounts and identity unchanged.',flush=True)
    finally:
        subprocess.run([*compose,'down','--volumes','--remove-orphans'],capture_output=True,timeout=180)
        command('docker','stop',name)
        # Only references created by this private test; retain user images and build caches.
        for reference in reversed(references):
            subprocess.run(['docker','image','rm',reference],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server',required=True)
    parser.add_argument('--dashboard',required=True)
    parser.add_argument('--game-dir')
    args=parser.parse_args()
    # Local image builds have no CI labels. Add test-only metadata to frozen copies before starting the stack.
    version=command('docker','run','--rm','--entrypoint','/opt/yo3/yo3-server',args.server,'--version').split()[-1]
    aliases=[]
    try:
        for service,source in [('server',args.server),('dashboard',args.dashboard)]:
            tag='yo3-ui-baseline-'+uuid.uuid4().hex+':'+service
            recipe=f'FROM {source}\nLABEL org.opencontainers.image.version="{version}" org.opencontainers.image.revision="'+('a'*40)+'"\n'
            subprocess.run(['docker','build','-q','-t',tag,'-'],input=recipe,text=True,capture_output=True,check=True)
            aliases.append(tag)
        argv=['--server',aliases[0],'--dashboard',aliases[1]]
        if args.game_dir:argv+=['--game-dir',args.game_dir]
        browser.main(argv,exercise=rehearse)
    finally:
        for alias in reversed(aliases):
            subprocess.run(['docker','image','rm',alias],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
