#!/usr/bin/env python3
"""Exercise certificate export/permissions/reload recovery in an isolated container, never on the host's /etc.
Run after building: python3 tools/containers/test_certificates.py [--image yo3-revival-dashboard:local]
"""
import argparse
from pathlib import Path
import subprocess
import tempfile
ROOT=Path(__file__).resolve().parents[2]
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--image',default='yo3-revival-dashboard:local');args=parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='yo3-cert-hook-') as tmp:
        fixture=Path(tmp);(fixture/'bin').mkdir()
        (fixture/'bin/docker').write_text('''#!/bin/sh
set -eu
printf '%s\\n' "$*" >> /fixture/calls
case "$*" in 'compose ps --status running --quiet dashboard') echo test-dashboard;; esac
''');(fixture/'bin/docker').chmod(0o755)
        for name,domain,days in [('valid','hook.example','14'),('wrong','other.example','14'),('short','hook.example','1')]:
            subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days',days,'-subj',f'/CN={domain}',
                '-addext',f'subjectAltName=DNS:{domain}','-keyout',str(fixture/f'{name}.key'),'-out',str(fixture/f'{name}.crt')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        script='''
set -eu
export PATH=/fixture/bin:$PATH
mkdir -p /etc/letsencrypt/live/hook.example
cp /fixture/valid.crt /etc/letsencrypt/live/hook.example/fullchain.pem
cp /fixture/valid.key /etc/letsencrypt/live/hook.example/privkey.pem
/repo/deploy/install-certificate.sh hook.example /srv/yo3/tls > /fixture/first
[ "$(stat -c '%a:%u:%g' /srv/yo3/tls/privkey.pem)" = 600:10001:10001 ]
/repo/deploy/install-certificate.sh hook.example /srv/yo3/tls > /fixture/second
! grep -q 'Certificate exported' /fixture/second
[ "$(grep -c 'nginx -s reload' /fixture/calls)" = 2 ]
sha256sum /srv/yo3/tls/fullchain.pem /srv/yo3/tls/privkey.pem > /fixture/before
cp /fixture/wrong.key /etc/letsencrypt/live/hook.example/privkey.pem
if /repo/deploy/install-certificate.sh hook.example /srv/yo3/tls; then echo 'Accepted mismatched key' >&2; exit 1; fi
cp /fixture/wrong.crt /etc/letsencrypt/live/hook.example/fullchain.pem
if /repo/deploy/install-certificate.sh hook.example /srv/yo3/tls; then echo 'Accepted wrong host' >&2; exit 1; fi
cp /fixture/short.crt /etc/letsencrypt/live/hook.example/fullchain.pem
cp /fixture/short.key /etc/letsencrypt/live/hook.example/privkey.pem
if /repo/deploy/install-certificate.sh hook.example /srv/yo3/tls; then echo 'Accepted near-expiry certificate' >&2; exit 1; fi
sha256sum /srv/yo3/tls/fullchain.pem /srv/yo3/tls/privkey.pem > /fixture/after
cmp /fixture/before /fixture/after
'''
        subprocess.run(['docker','run','--rm','--user','0:0','--network','none','--entrypoint','sh',
            '-v',f'{ROOT}:/repo:ro','-v',f'{fixture}:/fixture',args.image,'-ec',script],check=True)
        print('PASS: certificate export, owner/mode, retry reload, wrong key/host/expiry refusal; deployed files preserved.')
if __name__=='__main__':main()
