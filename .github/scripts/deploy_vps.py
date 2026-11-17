#!/usr/bin/env python3
"""Send release/dashboard requests or query deployed state over pinned, restricted SSH."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

def send(manifest):
    required=['VPS_HOST','VPS_SSH_KEY','VPS_KNOWN_HOSTS']
    missing=[name for name in required if not os.environ.get(name)]
    if missing:raise SystemExit('Missing deployment configuration: '+', '.join(missing))
    host=os.environ['VPS_HOST'];port=os.environ.get('VPS_PORT','22') or '22'
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9.:-]*',host):raise SystemExit('Invalid VPS_HOST')
    if not port.isdecimal() or not 1<=int(port)<=65535:raise SystemExit('Invalid VPS_PORT')
    with tempfile.TemporaryDirectory(prefix='yo3-deploy-ssh-') as folder:
        key=Path(folder)/'key';known=Path(folder)/'known_hosts'
        for path,content in [(key,os.environ['VPS_SSH_KEY']),(known,os.environ['VPS_KNOWN_HOSTS'])]:
            path.touch(mode=0o600);path.write_text(content.rstrip()+'\n')
        result=subprocess.run(['ssh','-T','-i',str(key),'-p',port,'-o','BatchMode=yes','-o','IdentitiesOnly=yes',
                        '-o','StrictHostKeyChecking=yes','-o',f'UserKnownHostsFile={known}',
                        '-o','ClearAllForwardings=yes','-o','ServerAliveInterval=15','-o','ServerAliveCountMax=3',
                        f'yo3-deploy@{host}'],input=json.dumps(manifest),text=True,check=True,timeout=900,
                              stdout=subprocess.PIPE)
        return result.stdout if result is not None else ''

def main(args=()):
    args=list(args)
    if args==['state']:
        print(json.dumps(json.loads(send({'kind':'state'}))))
        return
    if len(args)==2 and args[0]=='dashboard':
        request=json.loads(Path(args[1]).read_text())
        if not isinstance(request,dict) or request.get('kind')!='dashboard':
            raise SystemExit('Expected a dashboard deployment request')
        for key in ['registry_user','registry_token']:request.pop(key,None)
        if os.environ.get('GITHUB_TOKEN'):
            request|={'registry_user':os.environ['GITHUB_ACTOR'],'registry_token':os.environ['GITHUB_TOKEN']}
        print(send(request),end='')
        return
    if args:raise SystemExit('Usage: deploy_vps.py [state | dashboard REQUEST.json]')
    required=['VPS_HOST','VPS_SSH_KEY','VPS_KNOWN_HOSTS','SERVER_IMAGE','DASHBOARD_IMAGE','VERSION','SOURCE_REF']
    missing=[name for name in required if not os.environ.get(name)]
    if missing:raise SystemExit('Missing deployment configuration: '+', '.join(missing))
    manifest={'version':os.environ['VERSION'],'commit':os.environ['SOURCE_REF'],
              'server':os.environ['SERVER_IMAGE'],'dashboard':os.environ['DASHBOARD_IMAGE'],
              'registry_user':os.environ['GITHUB_ACTOR'],'registry_token':os.environ['GITHUB_TOKEN']}
    print(send(manifest),end='')

if __name__=='__main__':main(sys.argv[1:])
