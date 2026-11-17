#!/usr/bin/python3
"""Restricted SSH: release transactions, dashboard-only replacement and a read-only deployment-state query."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile

REPOSITORY='usersina/yo3-revival'
IMAGE_PREFIX=f'ghcr.io/{REPOSITORY}'
ASSETS='usr/share/nginx/html'
UI_LABELS={'org.opencontainers.image.revision','io.yo3.dashboard.base','io.yo3.server.image'}

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

def deployment_state(current):
    result=validate(current)
    result['dashboard_commit']=current.get('dashboard_commit',result['commit'])
    result['released_dashboard']=current.get('released_dashboard',result['dashboard'])
    if not re.fullmatch(r'[0-9a-f]{40}',str(result['dashboard_commit'])):
        raise ValueError('Invalid deployed dashboard commit')
    validate(result|{'dashboard':result['released_dashboard']})
    if current.get('previous_dashboard') is not None:
        previous=current['previous_dashboard']
        if not isinstance(previous,dict) or not re.fullmatch(r'[0-9a-f]{40}',str(previous.get('commit'))):
            raise ValueError('Invalid previous dashboard revision')
        validate(result|{'dashboard':previous.get('image')})
        result['previous_dashboard']={key:previous[key] for key in ['image','commit']}
    return result

def dashboard_manifest(request):
    allowed={'kind','version','commit','server','dashboard','dashboard_commit','released_dashboard',
             'expected_dashboard','expected_dashboard_commit','registry_user','registry_token'}
    if set(request)-allowed:raise ValueError('Unknown dashboard deployment fields')
    result=validate(request)
    for key in ['dashboard_commit','expected_dashboard_commit']:
        value=request.get(key)
        if not isinstance(value,str) or not re.fullmatch(r'[0-9a-f]{40}',value):raise ValueError('Invalid '+key)
        result[key]=value
    for key in ['released_dashboard','expected_dashboard']:
        validate(result|{'dashboard':request.get(key)})
        result[key]=request[key]
    return result

def image_override(manifest):
    lines=['services:']
    for service in ['server','dashboard']:
        lines += [f'  {service}:',f'    image: {json.dumps(manifest[service])}',
                  '    build: !reset null','    pull_policy: never']
    return '\n'.join(lines)+'\n'

def assert_dashboard_runtime(released,candidate):
    """Only static assets and three provenance labels may change. Used by CI and the forced command."""
    def config(image):
        value=json.loads(run(['docker','image','inspect',image]))[0]['Config'].copy()
        value['Labels']={k:v for k,v in (value.get('Labels') or {}).items() if k not in UI_LABELS}
        return value
    original,updated=config(released),config(candidate)
    if original!=updated or updated.get('User')!='10001:10001':
        raise ValueError('Dashboard runtime configuration differs from the released image')

    def files(image,check_assets=False):
        container=run(['docker','create','--network','none',image]).strip()
        try:
            runtime={};assets={}
            with tempfile.TemporaryDirectory(prefix='yo3-runtime-check-') as directory:
                archive=Path(directory)/'rootfs.tar'
                run(['docker','export','--output',str(archive),container])
                with tarfile.open(archive) as tar:
                    for entry in tar:
                        name=entry.name.removeprefix('./').rstrip('/')
                        if name.startswith('/') or '..' in name.split('/') or name in runtime or name in assets:
                            raise ValueError('Invalid dashboard runtime archive')
                        value=(entry.type,entry.mode,entry.uid,entry.gid,entry.linkname,entry.devmajor,entry.devminor,
                               tuple(sorted((k,v) for k,v in entry.pax_headers.items() if 'xattr' in k)))
                        if entry.isfile():
                            checksum=hashlib.sha256()
                            with tar.extractfile(entry) as content:
                                while chunk:=content.read(1024*1024):checksum.update(chunk)
                            value+=(checksum.hexdigest(),)
                        if name==ASSETS or name.startswith(ASSETS+'/'):
                            assets[name]=value
                            if check_assets and (not (entry.isfile() or entry.isdir()) or entry.uid!=0 or entry.gid!=0
                                                 or entry.mode&0o6022 or entry.isfile() and not entry.mode&0o4
                                                 or entry.isdir() and entry.mode&0o5!=0o5):
                                raise ValueError(f'Unsafe dashboard assets: {name}')
                        else:runtime[name]=value
            if check_assets and (assets.get(ASSETS,('?',))[0]!=tarfile.DIRTYPE
                                 or assets.get(ASSETS+'/index.html',('?',))[0]!=tarfile.REGTYPE):
                raise ValueError('Dashboard assets need a regular index.html and web directory')
            return runtime
        finally:run(['docker','rm','--volumes',container])
    before,after=files(released),files(candidate,True)
    differences=[name for name in before.keys()|after.keys() if before.get(name)!=after.get(name)]
    if differences:raise ValueError('Dashboard runtime files differ: '+', '.join(sorted(differences)[:12]))

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

    def execute(self,request):
        if not isinstance(request,dict):raise ValueError('Expected a deployment request')
        kind=request.get('kind','release')
        if kind=='release':return self.deploy(request)
        if kind=='dashboard':return self.deploy_dashboard(request)
        if kind!='state' or set(request)!={'kind'}:raise ValueError('Unknown deployment request')
        # A fixed read-only query through the same forced command, never an arbitrary filename or shell.
        if not (self.state/'current-release.json').is_file():
            raise ValueError('Complete a full release deployment before querying dashboard state')
        with (self.state/'.release-deployment.lock').open('w') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX)
            return deployment_state(json.loads((self.state/'current-release.json').read_text()))

    def assert_running_images(self,state):
        for service in ['server','dashboard']:
            container=self.command('ps','--quiet',service).strip()
            if not container:raise ValueError(f'No running {service} container')
            actual=json.loads(run(['docker','inspect',container]))[0]['Image']
            expected=json.loads(run(['docker','image','inspect',state[service]]))[0]['Id']
            if actual!=expected:raise ValueError(f'The running {service} differs from recorded deployment state')

    def deploy_dashboard(self,request):
        manifest=dashboard_manifest(request)
        current_file=self.state/'current-release.json'
        if not current_file.is_file():raise ValueError('Complete a full release deployment before updating the dashboard')
        with (self.state/'.release-deployment.lock').open('w') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX)
            previous_current=current_file.read_text();current=deployment_state(json.loads(previous_current))
            if any(manifest[key]!=current[key] for key in ['version','commit','server','released_dashboard']):
                raise ValueError('The tested server release changed; verify the dashboard again')
            if manifest['dashboard']==current['dashboard'] and manifest['dashboard_commit']==current['dashboard_commit']:
                self.assert_running_images(current);self.health(current['version'])
                report('Dashboard already deployed and healthy');return
            if (manifest['expected_dashboard'],manifest['expected_dashboard_commit'])!=(current['dashboard'],current['dashboard_commit']):
                raise ValueError('The dashboard revision changed; refusing a stale deployment')
            self.assert_running_images(current)
            with tempfile.TemporaryDirectory(prefix='yo3-registry-') as registry:
                env=os.environ|{'DOCKER_CONFIG':registry}
                if request.get('registry_token'):
                    run(['docker','login','ghcr.io','--username',request['registry_user'],'--password-stdin'],
                        input=request['registry_token']+'\n',env=env)
                for image in [manifest['released_dashboard'],manifest['dashboard']]:run(['docker','pull',image],env=env)
            labels=json.loads(run(['docker','image','inspect',manifest['dashboard']]))[0]['Config'].get('Labels') or {}
            expected={'org.opencontainers.image.version':manifest['version'],
                      'org.opencontainers.image.revision':manifest['dashboard_commit'],
                      'io.yo3.dashboard.base':manifest['released_dashboard'],'io.yo3.server.image':manifest['server']}
            if any(labels.get(key)!=value for key,value in expected.items()):
                raise ValueError('Dashboard image labels do not match the verified candidate and release')
            assert_dashboard_runtime(manifest['released_dashboard'],manifest['dashboard'])
            override=self.repo/'compose.production.yaml';previous=override.read_text()
            setup_file=self.state/'first-provisioned.json';setup=setup_file.read_text()
            updated=current|{'dashboard':manifest['dashboard'],'dashboard_commit':manifest['dashboard_commit'],
                'previous_dashboard':{'image':current['dashboard'],'commit':current['dashboard_commit']}}
            def start():
                self.command('up','-d','--no-deps','--no-build','--pull','never','--wait','--wait-timeout','90','dashboard')
            try:
                atomic_write(override,image_override(updated))
                start();self.health(current['version']);self.assert_running_images(updated)
                atomic_write(setup_file,json.dumps(json.loads(setup)|{'dashboard_image':updated['dashboard']},indent=2)+'\n')
                atomic_write(current_file,json.dumps(updated,indent=2)+'\n')
                report(f'Deployed dashboard {updated["dashboard_commit"][:12]}; server remains {current["version"]}')
            except BaseException as error:
                try:
                    atomic_write(override,previous);atomic_write(setup_file,setup);atomic_write(current_file,previous_current)
                    start();self.health(current['version']);self.assert_running_images(current)
                except BaseException as rollback_error:
                    raise RuntimeError(f'Dashboard deployment failed and rollback needs attention: {rollback_error}') from error
                raise RuntimeError(f'Dashboard deployment failed; previous dashboard rolled back: {error}') from error

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
                current=deployment_state(json.loads(current_file.read_text()))
                if manifest==validate(current|{'dashboard':current['released_dashboard']}):
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
                atomic_write(override,image_override(manifest))
                self.command('up','-d','--no-build','--pull','never','--wait','--wait-timeout','180')
                self.health(manifest['version'])
                updated=json.loads(setup)|{'server_image':manifest['server'],'dashboard_image':manifest['dashboard']}
                atomic_write(setup_file,json.dumps(updated,indent=2)+'\n')
                rollback=self.state/'release-rollback'
                if rollback.exists():shutil.rmtree(rollback)
                snapshot.replace(rollback)
                recorded=deployment_state(manifest)
                if previous_current is not None:
                    recorded['previous_dashboard']={'image':current['dashboard'],'commit':current['dashboard_commit']}
                atomic_write(current_file,json.dumps(recorded,indent=2)+'\n')
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
    result=Deployer().execute(json.loads(body))
    if result is not None:report(json.dumps(result))

if __name__=='__main__':
    try:main()
    except (ValueError,RuntimeError,OSError,KeyError) as error:
        print(f'Deployment failed: {error}',file=sys.stderr);sys.exit(1)
