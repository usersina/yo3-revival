#!/usr/bin/env python3
"""Real Compose/HTTPS/SQLite rehearsal. Own project, temporary volumes, unused loopback ports; no live data writes.
Build first: docker compose build. Then: python3 tools/containers/test_stack.py --game-dir '/your/game'
Requires Python 3, Docker Compose, OpenSSL; uses only stdlib. Test-only self-signed TLS is verified by its certificate.
"""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import socket
import ssl
import struct
import subprocess
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[2]
PASSWORD = 'DeployTest12345'
USER = 'DeployAdmin'

def port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game-dir', required=True, type=Path)
    parser.add_argument('--keep', action='store_true', help='Keep only this disposable project for inspection')
    a = parser.parse_args()
    for f in ['package/data002.ypk', 'yo3.idx']:
        if not (a.game_dir / f).is_file():
            parser.error(f'Your read-only game folder is missing {f}')
    project = 'yo3-container-test-' + uuid.uuid4().hex[:10]
    scratch = Path(tempfile.mkdtemp(prefix=project))
    scratch.chmod(0o755)
    for f in ['tls', 'acme']:
        (scratch/f).mkdir(mode=0o755)
    game_port, http_port, https_port = port(), port(), port()
    env = os.environ | {'YO3_DOMAIN':'localhost', 'YO3_GAME_DIR':str(a.game_dir.resolve()),
        'YO3_TLS_DIR':str(scratch/'tls'), 'YO3_ACME_DIR':str(scratch/'acme'),
        'YO3_GAME_BIND':'127.0.0.1', 'YO3_WEB_BIND':'127.0.0.1', 'YO3_GAME_PORT':str(game_port),
        'YO3_HTTP_PORT':str(http_port), 'YO3_HTTPS_PORT':str(https_port), 'YO3_IMAGE_TAG':'local'}
    compose = ['docker','compose','--env-file','/dev/null','--project-name',project,'-f',str(ROOT/'compose.yaml')]
    def run(args, *, check=True, input=None):
        return subprocess.run(compose+args, cwd=ROOT, env=env, check=check, input=input, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=240)
    def cli(*args, check=True):
        return run(['exec','-T','server','/opt/yo3/yo3-server','--config','/etc/yo3/server.toml',*args],check=check)
    def certificate():
        return run(['exec','-T','server','cat','/var/lib/yo3/server.crt']).stdout
    print(f'Disposable test project: {project}; scratch {scratch}', flush=True)
    if a.keep:
        (scratch/'compose-env.json').write_text(json.dumps({k:v for k,v in env.items() if k.startswith('YO3_')},indent=2))
    try:
        subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','14','-subj','/CN=localhost',
            '-addext','subjectAltName=DNS:localhost','-keyout',str(scratch/'tls/privkey.pem'),'-out',str(scratch/'tls/fullchain.pem')],
            check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        # Public test-only key; real deployment exports it as owner 10001, mode 0600.
        (scratch/'tls/privkey.pem').chmod(0o644)
        run(['run','--rm','--no-deps','server','init-auth','localhost'])
        assert run(['run','--rm','--no-deps','server','init-auth','localhost'],check=False).returncode, 'Never replace a used game identity'
        assert 'created' in run(['run','--rm','--no-deps','server','bootstrap-admin',USER],input=PASSWORD+'\n').stdout
        assert 'unchanged' in run(['run','--rm','--no-deps','server','bootstrap-admin',USER],input='DifferentPass123\n').stdout
        run(['up','-d','--wait','--wait-timeout','180','server'])
        assert run(['run','--rm','--no-deps','server','bootstrap-admin',USER],input=PASSWORD+'\n',check=False).returncode, 'Bootstrap cannot bypass a live data lock'
        game_cert = certificate()
        game_identity = hashlib.sha256(game_cert.encode()).hexdigest()
        cli('set','registration','open')
        # The real authenticated account protocol: exact magic/op/BE length from yo3-tunnel::Endpoint::account.
        ctx = ssl.create_default_context(cadata=game_cert)
        payload = json.dumps({'op':'register','user':'GamePlayer','password':PASSWORD,'rules':True}).encode()
        with ctx.wrap_socket(socket.create_connection(('127.0.0.1',game_port),timeout=20),server_hostname='localhost') as s:
            s.sendall(b'YO3TLS02'+b'\x06'+struct.pack('>H',len(payload))+payload)
            def read(n):
                out=b''
                while len(out)<n:
                    chunk=s.recv(n-len(out))
                    assert chunk, 'Truncated account reply'
                    out+=chunk
                return out
            length=struct.unpack('>H',read(2))[0]
            assert json.loads(read(length))['ok'], 'Real game registration failed'
        run(['up','-d','--wait','--wait-timeout','90','dashboard'])
        web_ctx=ssl.create_default_context(cafile=str(scratch/'tls/fullchain.pem'))
        origin=f'https://localhost:{https_port}'
        cookie,csrf='',''
        def web(path='/',body=None,extra=None,method=None):
            headers={'Origin':origin}
            if cookie: headers['Cookie']=cookie
            if body is not None:
                headers|={'Content-Type':'application/json','Idempotency-Key':uuid.uuid4().hex,'X-CSRF-Token':csrf}
            headers|=extra or {}
            conn=http.client.HTTPSConnection('localhost',https_port,context=web_ctx,timeout=30)
            conn.request(method or ('POST' if body is not None else 'GET'),path,json.dumps(body) if body is not None else None,headers)
            r=conn.getresponse();data=r.read();out=(r.status,{k.lower():v for k,v in r.getheaders()},data);conn.close();return out
        status,headers,html=web()
        assert status==200 and b'<div id="root">' in html
        for name in ['content-security-policy','strict-transport-security','x-content-type-options']:
            assert name in headers, headers
        assert headers['cache-control']=='no-cache'
        for path in ['/revival-mark.png','/fonts/Barlow-Regular.ttf','/fonts/Barlow-SemiBold.ttf']:
            assert web(path)[0]==200, f'Bundled static file missing: {path}'
        for path in ['/accounts','/settings','/backups']:
            assert web(path)[0]==200
        for path in ['/api/admin/v1/status','/panel/','/health','/panel/api/v1/restore','/panel/api/v1/backups/download','/design/','/.env','/unknown']:
            assert web(path)[0]==404,path
        assert web('/panel/openapi.json',extra={'Host':'forged.example'})[0]==421
        # Must pass through the REAL nginx overwrites, not only a synthetic Rust guard.
        assert web('/panel/openapi.json',extra={'X-Forwarded-For':'127.0.0.1, forged','X-Forwarded-Proto':'http','Forwarded':'for=127.0.0.1'})[0]==200
        def login():
            nonlocal cookie,csrf
            status,headers,body=web('/panel/api/v1/sign-in',{'user':USER,'password':PASSWORD})
            assert status==200,body
            cookie=headers['set-cookie'].split(';')[0];csrf=json.loads(body)['csrf_token']
            assert all(flag in headers['set-cookie'] for flag in ['Secure','HttpOnly','SameSite=Strict'])
        login()
        assert web('/panel/api/v1/sign-out',{},extra={'Origin':'https://evil.example'})[0]==403
        assert web('/panel/api/v1/sign-out',{},extra={'X-CSRF-Token':'wrong'})[0]==403
        status,_,data=web('/panel/api/v1/accounts');assert status==200,data
        account=next(x for x in json.loads(data)['accounts'] if x['login']==USER)
        key=uuid.uuid4().hex
        grant={'user_id':account['user_id'],'amount':11}
        status,_,data=web('/panel/api/v1/give-bp',grant,extra={'Idempotency-Key':key});assert status==200,data
        result=json.loads(data);assert result['bp']==account['bp']+11
        assert json.loads(web('/panel/api/v1/give-bp',grant,extra={'Idempotency-Key':key})[2])==result
        assert json.loads(web('/panel/api/v1/backups')[2])['downloads_allowed'] is False
        # Public backup metadata/creation, but never public database downloads.
        status,_,data=web('/panel/api/v1/backups',{});assert status==200,data
        backup=json.loads(data);assert backup['backup']=='completed' and backup['available'],backup
        name=backup['result']['name']
        assert run(['run','--rm','--no-deps','server','restore',name],check=False).returncode,'Live restore must be locked out'
        assert web('/panel/api/v1/give-bp',grant)[0]==200
        # Also exercise a real nginx-generated JSON error without echoing the body.
        status,headers,data=web('/panel/api/v1/sign-out',{'large':'x'*270000})
        assert status==413 and json.loads(data)['code']=='payload_too_large'
        assert 'strict-transport-security' in headers and headers['cache-control']=='no-store'
        run(['stop','server'])
        run(['run','--rm','--no-deps','server','restore',name])
        run(['up','-d','--wait','--wait-timeout','180','server'])
        login()
        assert hashlib.sha256(certificate().encode()).hexdigest()==game_identity,'Game identity changed on restore'
        assert json.loads(web('/panel/api/v1/give-bp',grant,extra={'Idempotency-Key':key})[2])==result,'Receipt did not survive restore'
        status,_,data=web('/panel/api/v1/accounts');assert status==200
        restored=next(x for x in json.loads(data)['accounts'] if x['login']==USER)
        assert restored['bp']==result['bp'],'Restore did not roll back the later grant'
        # No management TCP port is published; data and game files never reach nginx.
        config=json.loads(run(['config','--format','json']).stdout)
        assert [p['target'] for p in config['services']['server']['ports']]==[7443]
        assert not any(v['target'] in ['/var/lib/yo3','/game'] for v in config['services']['dashboard']['volumes'])
        assert cli('accounts').returncode==0
        assert run(['exec','-T','server','sh','-c','echo forbidden > /game/.yo3-write-probe'],check=False).returncode
        conn=http.client.HTTPConnection('127.0.0.1',http_port,timeout=10);conn.request('GET','/accounts');r=conn.getresponse();assert r.status==308 and r.getheader('Location')==origin+'/accounts';r.read();conn.close()
        # ACME challenge path survives HTTPS startup for unattended renewals.
        challenge=scratch/'acme/.well-known/acme-challenge';challenge.mkdir(parents=True);(challenge/'probe').write_text('acme-ok')
        conn=http.client.HTTPConnection('127.0.0.1',http_port,timeout=10);conn.request('GET','/.well-known/acme-challenge/probe');r=conn.getresponse();assert r.status==200 and r.read()==b'acme-ok';conn.close()
        print('PASS: HTTPS, real account sign-in, nginx spoofing/allowlist/errors, CSRF, receipts, durable data, offline restore, identities, read-only mounts, ACME and CLI.')
        if a.keep:
            (scratch/'compose-env.json').write_text(json.dumps({k:v for k,v in env.items() if k.startswith('YO3_')},indent=2))
            print(f'Disposable project retained: {project}; HTTPS {origin}; scratch {scratch}')
    except BaseException:
        print(run(['logs','--tail','80'],check=False).stdout)
        raise
    finally:
        if not a.keep:
            # Explicitly only THIS randomly named test project. Never use this for a deployment.
            run(['down','--volumes','--remove-orphans'],check=False)
            shutil.rmtree(scratch)
if __name__=='__main__':
    main()
