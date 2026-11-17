#!/usr/bin/env python3
"""Deployment constraints checked without a daemon, game files or third-party Python packages."""
import json
import os
from pathlib import Path
import subprocess
import unittest

ROOT=Path(__file__).resolve().parents[2]
class DeploymentConfig(unittest.TestCase):
    def config(self, *files):
        env=os.environ|{'YO3_DOMAIN':'staff.example','YO3_GAME_DIR':'/nonexistent/read-only-game',
            'YO3_TLS_DIR':'/nonexistent/tls','YO3_ACME_DIR':'/nonexistent/acme'}
        # Never consume the operator's .env during tests.
        cmd=['docker','compose','--env-file','/dev/null','-f',str(ROOT/'compose.yaml')]
        for f in files:cmd+=['-f',str(ROOT/f)]
        return json.loads(subprocess.check_output(cmd+['--profile','certificate','config','--format','json'],cwd=ROOT,env=env))
    def test_only_game_and_web_ports_are_published(self):
        cfg=self.config();server=cfg['services']['server'];dashboard=cfg['services']['dashboard']
        self.assertEqual([p['target'] for p in server['ports']],[7443])
        self.assertEqual(sorted(p['target'] for p in dashboard['ports']),[8080,8443])
        self.assertNotIn('network_mode',server);self.assertNotIn('network_mode',dashboard)
        for service in [server,dashboard]:
            self.assertTrue(service['read_only']);self.assertEqual(service['cap_drop'],['ALL'])
        game=next(v for v in server['volumes'] if v['target']=='/game')
        self.assertTrue(game['read_only']);self.assertFalse(game['bind']['create_host_path'])
        self.assertFalse(any(v['target'] in ['/var/lib/yo3','/game','/var/run/docker.sock'] for v in dashboard['volumes']))
        socket=next(v for v in dashboard['volumes'] if v['target']=='/run/yo3-dashboard')
        self.assertTrue(socket['read_only'])
    def test_bootstrap_does_not_start_game_or_require_a_certificate(self):
        cfg=self.config('deploy/compose.acme.yaml');acme=cfg['services']['acme']
        self.assertFalse(acme.get('depends_on'));self.assertEqual(acme['profiles'],['certificate'])
        self.assertFalse(any(v['target']=='/etc/yo3/tls' for v in acme['volumes']))
    def test_host_scripts_parse(self):
        for name in ['server-entrypoint.sh','nginx/entrypoint.sh','install-certificate.sh','check-https.sh']:
            subprocess.run(['sh','-n',str(ROOT/'deploy'/name)],check=True)
if __name__=='__main__':unittest.main()
