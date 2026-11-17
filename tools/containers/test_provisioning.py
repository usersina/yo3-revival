#!/usr/bin/env python3
"""Render production templates and validate Compose without touching any VPS/system packages."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from jinja2 import Environment, StrictUndefined
ROOT=Path(__file__).resolve().parents[2]
class Provisioning(unittest.TestCase):
    def check_image_references(self, server_image, dashboard_image):
        with tempfile.TemporaryDirectory(prefix='yo3-provisioning-template-') as tmp:
            path=Path(tmp)
            shutil.copy(ROOT/'compose.yaml',path/'compose.yaml')
            env=Environment(undefined=StrictUndefined);env.filters['to_json']=json.dumps
            values={'yo3_domain':'staff.example','yo3_server_image':server_image,
                'yo3_dashboard_image':dashboard_image}
            for source,dest in [('compose.production.yaml.j2','compose.production.yaml'),('environment.j2','.env')]:
                text=env.from_string((ROOT/'deploy/ansible/templates'/source).read_text()).render(values)
                (path/dest).write_text(text)
            # Ignore the caller's YO3_* values, like an independently provisioned VPS.
            process_env={k:v for k,v in os.environ.items() if not k.startswith('YO3_') and k!='COMPOSE_FILE'}
            result=json.loads(subprocess.check_output(['docker','compose','config','--format','json'],cwd=path,env=process_env))
            for name in ['server','dashboard']:
                service=result['services'][name]
                self.assertNotIn('build',service,'A VPS must load tested images; never compile a different build')
                self.assertEqual(service['image'],values[f'yo3_{name}_image'])
            self.assertEqual(result['services']['server']['environment']['YO3_DOMAIN'],'staff.example')
    def test_local_setup_uses_exact_image_ids(self):
        self.check_image_references('sha256:'+'a'*64,'sha256:'+'b'*64)
    def test_release_images_can_use_exact_registry_digests(self):
        self.check_image_references('ghcr.io/usersina/yo3-revival-server@sha256:'+'a'*64,
            'ghcr.io/usersina/yo3-revival-dashboard@sha256:'+'b'*64)
    def test_generated_password_is_private_and_stable_across_retries(self):
        # Exercise the real Ansible lookup without exposing its generated value in command/output.
        with tempfile.TemporaryDirectory(prefix='yo3-bootstrap-password-') as tmp:
            path=Path(tmp);password=path/'password'
            playbook=path/'lookup.yml'
            playbook.write_text("""- hosts: localhost
  gather_facts: false
  tasks:
    - ansible.builtin.set_fact:
        first: "{{ lookup('ansible.builtin.password', password_path, length=16, chars=['ascii_letters','digits']) }}"
      no_log: true
    - ansible.builtin.assert:
        that:
          - first is match('^[A-Za-z0-9]{16}$')
          - first == lookup('ansible.builtin.password', password_path, length=16, chars=['ascii_letters','digits'])
      no_log: true
""")
            subprocess.run(['ansible-playbook','-i','localhost,','-c','local',str(playbook),'-e',
                json.dumps({'password_path':str(password)})],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            self.assertEqual(password.stat().st_mode & 0o777,0o600)
    def test_site_parses_with_the_pinned_collection(self):
        subprocess.run(['ansible-playbook','-i',str(ROOT/'deploy/ansible/inventory.example.ini'),str(ROOT/'deploy/ansible/site.yml'),'--syntax-check'],check=True,cwd=ROOT)
if __name__=='__main__':unittest.main()
