#!/usr/bin/env python3
"""Render production templates and validate Compose without touching any VPS/system packages."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tarfile
import uuid
import yaml
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
                self.assertEqual(service['pull_policy'],'never')
            self.assertEqual(result['services']['server']['environment']['YO3_DOMAIN'],'staff.example')
    def test_local_setup_uses_portable_versioned_tags(self):
        self.check_image_references('yo3-revival-server:setup-'+'a'*64,'yo3-revival-dashboard:setup-'+'b'*64)
    def test_actual_controller_export_preserves_names_in_the_archive(self):
        # Execute the playbook's real prepare/tag/export tasks with tiny unique images. An export by bare ID
        # loses repository names, which failed to register usable images in the fresh VPS containerd store.
        images=[];token=uuid.uuid4().hex
        try:
            for name in ['server','dashboard']:
                ref=f'yo3-export-test-{token}-{name}:fixture'
                with tempfile.TemporaryDirectory(prefix='yo3-export-fixture-') as tmp:
                    context=Path(tmp);(context/'marker').write_text(token)
                    (context/'Dockerfile').write_text(f'FROM scratch\nCOPY marker /marker\nLABEL yo3-fixture={token}-{name}\n')
                    subprocess.run(['docker','build','-q','-t',ref,str(context)],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                images.extend(json.loads(subprocess.check_output(['docker','image','inspect',ref])))
            with tempfile.TemporaryDirectory(prefix='yo3-image-export-') as tmp:
                path=Path(tmp);archive=path/'images.tar';playbook=path/'export.yml'
                tasks=yaml.safe_load((ROOT/'deploy/ansible/site.yml').read_text())[0]['tasks']
                controller=next(t['block'] for t in tasks if t['name']=='Build and transfer the local images for first setup')
                selected=[t for t in controller if t['name'] in ['Prepare the immutable setup record','Name portable setup images']]
                transfer=next(t['block'] for t in controller if t['name']=='Export and upload only our two images')
                selected+= [t for t in transfer if t['name']=='Export local images']
                playbook.write_text(yaml.safe_dump([{'hosts':'localhost','gather_facts':False,'vars':{
                    'yo3_domain':'staff.example','local_images':{'stdout':json.dumps(images)},'image_archive':{'path':str(archive)}},'tasks':selected}]))
                result=subprocess.run(['ansible-playbook','-i','localhost,','-c','local',str(playbook)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
                self.assertEqual(result.returncode,0,result.stdout)
                with tarfile.open(archive) as tar:
                    manifest=json.load(tar.extractfile('manifest.json'))
                refs=[ref for item in manifest for ref in (item.get('RepoTags') or [])]
                for name,item in zip(['server','dashboard'],images):
                    self.assertIn(f"yo3-revival-{name}:setup-{item['Id'].removeprefix('sha256:')}",refs)
        finally:
            for item in images:
                result=subprocess.run(['docker','image','inspect',item['Id']],text=True,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
                if result.returncode==0:
                    for ref in json.loads(result.stdout)[0]['RepoTags']:
                        subprocess.run(['docker','image','rm',ref],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

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
