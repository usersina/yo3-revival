#!/usr/bin/env python3
"""Exercise a built dashboard against an existing server image. Never compiles Rust or uses live data.

CI uses the deployed release's exact server digest. The synthetic archive contains no Konami data and is
only for dashboard/API checks, not gameplay. --game-dir allows an additional local run with the real game.
"""
import argparse
import json
import os
from pathlib import Path
import socket
import ssl
import struct
import subprocess
import tempfile
import uuid
import zlib

ROOT = Path(__file__).resolve().parents[2]
PASSWORD = 'GateCheck12345'


def synthetic_game(path):
    package = path/'package'
    package.mkdir(parents=True)
    # Invented cards and a legal 40-card deck, solely to pass normal hosting's data checks. No game content.
    # Use OpenSSL's existing legacy cipher rather than adding a Python dependency or compiling a test server.
    def record(fields):
        plain = b'\x03'+struct.pack('>H', len(fields))
        for name, value in fields.items():
            encoded = name.encode()
            plain += bytes([len(encoded)])+encoded
            plain += b'\x05'+struct.pack('>I', value) if isinstance(value, int) else b'\x08'+struct.pack('>I', len(value))+value
        plain += b'\x00\x00'
        plain += b'\0'*((-len(plain)) % 8)
        def swap_words(data):
            return b''.join(data[i:i+4][::-1] for i in range(0, len(data), 4))
        # Repeating the 8-byte key gives the same Blowfish key schedule with enc's 16-byte key length.
        body = subprocess.run(['openssl', 'enc', '-bf-ecb', '-provider', 'legacy', '-nopad',
                               '-K', '31323334353637383132333435363738'], input=swap_words(plain),
                              check=True, capture_output=True).stdout
        return struct.pack('>I', len(body) ^ 0x77777777)+swap_words(body)

    names, descs, index = bytearray(), bytearray(), bytearray(8)
    for card in range(1,41):
        index += struct.pack('<II',len(names),len(descs))
        names += f'Example card {card:02}\0'.encode('utf-16-le')
        descs += 'Invented fixture effect.\nOriginal game text stays readable.\0'.encode('utf-16-le')
    files = {'data/bin#/CARD_Prop.bin': b'\0'*8+b''.join(struct.pack('<II', i, 4 << 12) for i in range(1, 41)),
             'data/bin#/CARD_Indx_E.bin': index, 'data/bin#/CARD_Name_E.bin': names,
             'data/bin#/CARD_Desc_E.bin': descs, 'data/text/txttbl_E.bin': b'',
             'data/extra/yo3pbininfo.bin': b'',
             'data/regulation/1.y3r': record({'regulation_id': 1}),
             'data/recipe/1.y3d': record({'deck_id': 1, 'deck_type': 1,
                                        'main': b''.join(struct.pack('>H', i) for i in range(1, 41))})}
    def archive(filename, files):
        with (package/filename).open('wb') as file:
            for name, payload in files.items():
                encoded = name.encode()
                header = bytearray(30)
                header[:4] = b'YO\x03\x04'
                struct.pack_into('<II', header, 18, len(payload), len(payload))
                struct.pack_into('<H', header, 26, len(encoded))
                file.write(header+encoded+payload)
    archive('data002.ypk', files)
    def png(width, height):
        def chunk(kind, body):
            return struct.pack('>I',len(body))+kind+body+struct.pack('>I',zlib.crc32(kind+body))
        return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+bytes([53,92,138])*width)*height))+chunk(b'IEND',b'')
    for filename,folder,width,height in [('data000.ypk','mini',50,72),('data001.ypk','card',200,290)]:
        archive(filename,{f'data/{folder}/{i}_0.png':png(width,height) for i in range(1,41)})
    (path/'yo3.idx').write_bytes(b'')


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def main(argv=None, exercise=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server', required=True, help='Existing released server image; never built here')
    parser.add_argument('--dashboard', required=True, help='Exact candidate image to verify')
    parser.add_argument('--game-dir', type=Path)
    args = parser.parse_args(argv)
    project = 'yo3-dashboard-check-'+uuid.uuid4().hex[:10]
    with tempfile.TemporaryDirectory(prefix=project) as folder:
        scratch = Path(folder)
        scratch.chmod(0o755)
        for name in ['tls', 'acme']:
            (scratch/name).mkdir()
        game = args.game_dir.resolve() if args.game_dir else scratch/'game'
        if args.game_dir is None:
            synthetic_game(game)
        https, http, auth = free_port(), free_port(), free_port()
        origin = f'https://localhost:{https}'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '14',
                        '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost',
                        '-keyout', str(scratch/'tls/privkey.pem'), '-out', str(scratch/'tls/fullchain.pem')],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        (scratch/'tls/privkey.pem').chmod(0o644)  # Disposable test identity only.
        common = {'read_only': True, 'cap_drop': ['ALL'], 'security_opt': ['no-new-privileges:true'],
                  'tmpfs': ['/tmp:mode=1777,noexec,nosuid,nodev'], 'pull_policy': 'never',
                  'environment': {'YO3_DOMAIN': 'localhost', 'YO3_HTTPS_PORT': str(https)}}
        config = {'services': {
            'server': common | {'image': args.server,
                'environment': common['environment'] | {'YO3_DASHBOARD_ORIGIN': origin},
                'ports': [f'127.0.0.1:{auth}:7443'],
                'volumes': ['data:/var/lib/yo3', 'socket:/run/yo3-dashboard', f'{game}:/game:ro']},
            'dashboard': common | {'image': args.dashboard,
                'ports': [f'127.0.0.1:{https}:8443', f'127.0.0.1:{http}:8080'],
                'volumes': ['socket:/run/yo3-dashboard:ro', f'{scratch}/tls:/etc/yo3/tls:ro',
                            f'{scratch}/acme:/var/www/acme:ro']},
        }, 'volumes': {'data': {}, 'socket': {}}}
        compose_file = scratch/'compose.json'
        compose_file.write_text(json.dumps(config))
        compose = ['docker', 'compose', '--project-name', project, '-f', str(compose_file)]

        def run(*command, input=None, check=True):
            result = subprocess.run([*compose, *command], input=input, text=True, capture_output=True, timeout=240)
            if check and result.returncode:
                raise RuntimeError(result.stdout+result.stderr)
            return result

        try:
            run('run', '--rm', '--no-deps', 'server', 'init-auth', 'localhost')
            run('run', '--rm', '--no-deps', 'server', 'bootstrap-admin', 'CheckAdmin', input=PASSWORD+'\n')
            run('run', '--rm', '--no-deps', 'server', 'add-player', 'CheckAdmin', 'CheckAdmin')
            run('run', '--rm', '--no-deps', 'server', 'set', 'registration', 'open')
            run('up', '-d', '--no-build', '--wait', '--wait-timeout', '180', 'server')
            cli = ('exec', '-T', 'server', '/opt/yo3/yo3-server', '--config', '/etc/yo3/server.toml')
            version = run(*cli, '--version').stdout.strip().split()[-1]
            certificate = run('exec', '-T', 'server', 'cat', '/var/lib/yo3/server.crt').stdout
            context = ssl.create_default_context(cadata=certificate)
            body = json.dumps({'op': 'register', 'user': 'CheckModerator', 'password': PASSWORD,
                               'rules': True, 'launcher': version}).encode()
            with context.wrap_socket(socket.create_connection(('127.0.0.1', auth), timeout=30),
                                     server_hostname='localhost') as connection:
                connection.sendall(b'YO3TLS02\x06'+struct.pack('>H', len(body))+body)

                def read(count):
                    data = b''
                    while len(data) < count:
                        chunk = connection.recv(count-len(data))
                        if not chunk:
                            raise RuntimeError('Truncated account registration response')
                        data += chunk
                    return data

                reply = json.loads(read(struct.unpack('>H', read(2))[0]))
                if not reply.get('ok'):
                    raise RuntimeError('Test account registration failed: '+str(reply.get('error')))
            run(*cli, 'set-role', 'CheckModerator', 'moderator')
            run(*cli, 'add-player', 'CheckModerator', 'CheckModerator')
            run('up', '-d', '--no-build', '--wait', '--wait-timeout', '90', 'dashboard')
            print(f'Browser gate: {origin}; server {args.server}; dashboard {args.dashboard}', flush=True)
            def verify():
                subprocess.run(['npx', '--no-install', 'playwright', 'test', '--config', 'playwright.release.config.ts'],
                               cwd=ROOT/'dashboard', check=True, timeout=600,
                               env=os.environ | {'YO3_RELEASE_DASHBOARD_URL': origin})
            if exercise is None:verify()
            else:exercise(scratch,compose,args,verify)
        except BaseException:
            print(run('logs', '--no-color', '--tail', '60', check=False).stdout)
            raise
        finally:
            run('down', '--volumes', '--remove-orphans', check=False)


if __name__ == '__main__':
    main()
