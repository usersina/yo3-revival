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
            self.assertIn('no-new-privileges:true',service['security_opt'])
            self.assertEqual(set(service['tmpfs'][0].split(',')[1:]),{'noexec','nosuid','nodev'})
        game=next(v for v in server['volumes'] if v['target']=='/game')
        self.assertTrue(game['read_only']);self.assertFalse(game['bind']['create_host_path'])
        self.assertFalse(any(v['target'] in ['/var/lib/yo3','/game','/var/run/docker.sock'] for v in dashboard['volumes']))
        socket=next(v for v in dashboard['volumes'] if v['target']=='/run/yo3-dashboard')
        self.assertTrue(socket['read_only'])
    def test_bootstrap_does_not_start_game_or_require_a_certificate(self):
        cfg=self.config('deploy/compose.acme.yaml');acme=cfg['services']['acme']
        self.assertFalse(acme.get('depends_on'));self.assertEqual(acme['profiles'],['certificate'])
        self.assertFalse(any(v['target']=='/etc/yo3/tls' for v in acme['volumes']))
    def test_nginx_allows_every_staff_api_route(self):
        """Each path of the generated OpenAPI document reaches the server through nginx's allowlist, unless nginx blocks
        it on purpose: a route added to the Rust router but not to the allowlist would 404 only behind nginx."""
        import json, re
        paths = json.loads((ROOT / 'dashboard/api/openapi.json').read_text())['paths']
        conf = (ROOT / 'deploy/nginx/dashboard.conf.template').read_text()
        allowed = [re.compile(m) for m in re.findall(r'location ~ (\^/panel/api/v1/\S+\$) \{', conf)]
        blocked = set(re.findall(r'location = (/panel/api/v1/\S+) \{ return 404; \}', conf))
        self.assertTrue(allowed)
        sample = lambda p: re.sub(r'\{[a-z_]+\}', '7', p.replace('{size}', 'mini'))
        missing = [p for p in paths if p not in blocked and not any(r.match(sample(p)) for r in allowed)]
        self.assertEqual(missing, [], 'add these routes to deploy/nginx/dashboard.conf.template')

    def test_nginx_location_regexes_need_no_quotes(self):
        """nginx ends an unquoted location regex at `{` (v0.1.10's deployment failed on `{1,16}`: pcre2 "missing closing
        parenthesis", the dashboard unhealthy and rolled back). Keep braces out of them, or quote the whole regex."""
        import re
        conf = (ROOT / 'deploy/nginx/dashboard.conf.template').read_text()
        for regex in re.findall(r'location ~\*? (\S+) \{', conf):
            self.assertFalse(regex[0] not in '"\'' and ('{' in regex or '}' in regex), f'quote or rewrite: {regex}')

    def test_host_scripts_parse(self):
        for name in ['server-entrypoint.sh','nginx/entrypoint.sh','install-certificate.sh','check-https.sh']:
            subprocess.run(['sh','-n',str(ROOT/'deploy'/name)],check=True)
        subprocess.run(['sh','-n',str(ROOT/'tools/containers/server-rootfs.sh')],check=True)
if __name__=='__main__':unittest.main()
