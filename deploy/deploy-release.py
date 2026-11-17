#!/usr/bin/python3
"""Restricted SSH entry point: install one verified release's image digests, restoring stopped data on failure."""
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile

REPOSITORY='usersina/yo3-revival'
IMAGE_PREFIX=f'ghcr.io/{REPOSITORY}'

def report(message):
    # A lost SSH response must not turn a successful deployment into a rollback.
    try:print(message,flush=True)
    except BrokenPipeError:pass

def run(args, **kwargs):
    result=subprocess.run(args,check=False,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=600,**kwargs)
    if result.returncode:
        raise RuntimeError(f'{args[0]} failed ({result.returncode}): {result.stderr[-3000:]}')
    return result.stdout

def atomic_write(path,content):
    temporary=path.with_name(path.name+'.tmp')
    with temporary.open('w') as file:
        os.chmod(temporary,0o600);file.write(content);file.flush();os.fsync(file.fileno())
    temporary.replace(path)
    fd=os.open(path.parent,os.O_RDONLY)
    try:os.fsync(fd)
    finally:os.close(fd)

def validate(request):
    if not isinstance(request,dict):raise ValueError('Expected a release manifest')
    if not isinstance(request.get('version'),str) or not re.fullmatch(r'v\d+\.\d+\.\d+',request['version']):raise ValueError('Invalid release version')
    if not isinstance(request.get('commit'),str) or not re.fullmatch(r'[0-9a-f]{40}',request['commit']):raise ValueError('Invalid source commit')
    for service in ['server','dashboard']:
        prefix=re.escape(f'{IMAGE_PREFIX}-{service}')
        if not isinstance(request.get(service),str) or not re.fullmatch(prefix+r'@sha256:[0-9a-f]{64}',request[service]):
            raise ValueError('Only this project\'s digest-pinned release images are accepted')
    return {key:request[key] for key in ['version','commit','server','dashboard']}

def version_number(version):return tuple(map(int,version.removeprefix('v').split('.')))

def copy_data(source,destination):
    """Preserve numeric ownership and links when running as root; never follow links out of the volume."""
    shutil.copytree(source,destination,symlinks=True)
    if os.geteuid()==0:
        for root,dirs,files in os.walk(source,followlinks=False):
            for item in [Path(root),*(Path(root)/name for name in dirs+files)]:
                target=destination/item.relative_to(source);stat=item.lstat()
                os.chown(target,stat.st_uid,stat.st_gid,follow_symlinks=False)

class Deployer:
    def __init__(self,repo=Path('/opt/yo3-revival'),state=Path('/srv/yo3'),project='yo3-revival'):
        self.repo=repo;self.state=state
        self.compose=['docker','compose','--project-name',project,'-f','compose.yaml','-f','compose.production.yaml']

    def command(self,*args):return run(self.compose+list(args),cwd=self.repo)

    def health(self,expected=None):
        health=json.loads(self.command('exec','-T','server','curl','--fail','--silent','--show-error',
                                      'http://127.0.0.1:8080/health'))
        if not health.get('ok') or expected and health.get('release')!=expected:
            raise RuntimeError('Server is unhealthy or is not running the requested release')
        settings=dict(line.split('=',1) for line in (self.repo/'.env').read_text().splitlines() if '=' in line and not line.startswith('#'))
        domain=settings['YO3_DOMAIN'];port=settings.get('YO3_HTTPS_PORT','443')
        run(['curl','--fail','--silent','--show-error','--max-time','20','--noproxy','*',
             '--resolve',f'{domain}:{port}:127.0.0.1','--output','/dev/null',f'https://{domain}:{port}/'])

    def deploy(self,request):
        manifest=validate(request)
        self.state.mkdir(parents=True,exist_ok=True)
        with (self.state/'.release-deployment.lock').open('w') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX)
            current_file=self.state/'current-release.json'
            previous_current=current_file.read_text() if current_file.exists() else None
            if current_file.exists():
                current=json.loads(current_file.read_text())
                if manifest==current:
                    self.health(manifest['version']);report('Release already deployed and healthy');return
                if version_number(manifest['version'])<=version_number(current['version']):
                    raise ValueError('Refusing an older release or a different build reusing a released version')
            # Pull and verify BOTH images before touching the running stack. The token is ephemeral, never persisted.
            with tempfile.TemporaryDirectory(prefix='yo3-registry-') as registry:
                env=os.environ|{'DOCKER_CONFIG':registry}
                if request.get('registry_token'):
                    run(['docker','login','ghcr.io','--username',request['registry_user'],'--password-stdin'],
                        input=request['registry_token']+'\n',env=env)
                for service in ['server','dashboard']:
                    image=manifest[service]
                    run(['docker','pull',image],env=env)
                    labels=json.loads(run(['docker','image','inspect',image]))[0]['Config']['Labels'] or {}
                    if any(labels.get('org.opencontainers.image.'+key)!=manifest[value]
                           for key,value in [('version','version'),('revision','commit')]):
                        raise RuntimeError('Image labels do not match the release version and source commit')
            container=self.command('ps','--quiet','server').strip()
            if not container:raise RuntimeError('No existing server container; run first provisioning before deploying releases')
            mounts=json.loads(run(['docker','inspect',container]))[0]['Mounts']
            data=next((Path(m['Source']) for m in mounts if m['Type']=='volume' and m['Destination']=='/var/lib/yo3'),None)
            if data is None or not data.is_dir() or data.is_symlink():raise RuntimeError('Expected the existing local data volume')
            override=self.repo/'compose.production.yaml';previous=override.read_text()
            setup_file=self.state/'first-provisioned.json';setup=setup_file.read_text()
            snapshot=Path(tempfile.mkdtemp(prefix='.release-snapshot-',dir=self.state))
            saved=False
            try:
                report('Stopping for a consistent pre-upgrade snapshot')
                self.command('stop','--timeout','60')
                copy_data(data,snapshot/'data');saved=True
                atomic_write(snapshot/'compose.production.yaml',previous)
                atomic_write(snapshot/'first-provisioned.json',setup)
                # Image-only updates preserve the provisioned volume names, domain, TLS, game assets and authority.
                lines=['services:']
                for service in ['server','dashboard']:
                    lines += [f'  {service}:',f'    image: {json.dumps(manifest[service])}',
                              '    build: !reset null','    pull_policy: never']
                atomic_write(override,'\n'.join(lines)+'\n')
                self.command('up','-d','--no-build','--pull','never','--wait','--wait-timeout','180')
                self.health(manifest['version'])
                updated=json.loads(setup)|{'server_image':manifest['server'],'dashboard_image':manifest['dashboard']}
                atomic_write(setup_file,json.dumps(updated,indent=2)+'\n')
                rollback=self.state/'release-rollback'
                if rollback.exists():shutil.rmtree(rollback)
                snapshot.replace(rollback)
                atomic_write(current_file,json.dumps(manifest,indent=2)+'\n')
                report(f'Deployed {manifest["version"]}; server and HTTPS are healthy')
            except BaseException as error:
                try:report(self.command('logs','--no-color','--tail','60'))
                except (RuntimeError,OSError):pass
                # If shutdown/snapshot fails, restart the unchanged old configuration. Restore only stopped data.
                try:
                    self.command('stop','--timeout','60')
                    atomic_write(override,previous);atomic_write(setup_file,setup)
                    if previous_current is None:current_file.unlink(missing_ok=True)
                    else:atomic_write(current_file,previous_current)
                    if saved:
                        for item in data.iterdir():
                            if item.is_dir() and not item.is_symlink():shutil.rmtree(item)
                            else:item.unlink()
                        restored=data.with_name(data.name+'.restore')
                        if restored.exists():shutil.rmtree(restored)
                        source=(snapshot if snapshot.exists() else self.state/'release-rollback')/'data'
                        copy_data(source,restored)
                        for item in restored.iterdir():item.replace(data/item.name)
                        restored.rmdir()
                    self.command('up','-d','--no-build','--pull','never','--wait','--wait-timeout','180')
                    self.health()
                except BaseException as rollback_error:
                    raise RuntimeError(f'Deployment failed and rollback needs attention: {rollback_error}. Snapshot: {snapshot}') from error
                raise RuntimeError(f'Deployment failed; old images and data rolled back: {error}') from error
            finally:
                if snapshot.exists() and not saved:shutil.rmtree(snapshot)

def main():
    if len(sys.argv)!=1:raise ValueError('This deployment entry point takes JSON on stdin, no command arguments')
    signal.signal(signal.SIGHUP,signal.SIG_IGN)
    body=sys.stdin.buffer.read(32769)
    if len(body)>32768:raise ValueError('Release manifest too large')
    Deployer().deploy(json.loads(body))

if __name__=='__main__':
    try:main()
    except (ValueError,RuntimeError,OSError,KeyError) as error:
        print(f'Release deployment failed: {error}',file=sys.stderr);sys.exit(1)
