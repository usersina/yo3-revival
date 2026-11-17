#!/usr/bin/env python3
"""Real derived images: clean assets and fail closed on runtime, ownership or process-config changes."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('deployer', ROOT/'deploy/deploy-release.py')
deployer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(deployer)


class DashboardRuntime(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix='yo3-dashboard-runtime-')
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.root = Path(cls.temporary.name)
        cls.images = []
        cls.addClassCleanup(cls.remove_images)
        source = os.environ.get('YO3_TEST_DASHBOARD', 'yo3-revival-dashboard:optimized')
        cls.base = cls.build(f'FROM {source}\nUSER 0:0\nRUN touch /usr/share/nginx/html/.obsolete /usr/share/nginx/html/old.js\nUSER 10001:10001\n')
        assets = cls.root/'assets'
        assets.mkdir()
        (assets/'index.html').write_text('<html>New deployment</html>')
        (assets/'new.js').write_text('console.log("new")')
        cls.candidate = 'yo3-dashboard-runtime-'+uuid.uuid4().hex+':candidate'
        subprocess.run(['docker', 'build', '-q', '-f', str(ROOT/'deploy/Dockerfile.dashboard-assets'),
                        '--build-arg', f'RELEASED_DASHBOARD={cls.base}', '--build-arg', 'DASHBOARD_COMMIT='+'a'*40,
                        '--build-arg', 'TESTED_SERVER=ghcr.io/usersina/yo3-revival-server@sha256:'+'1'*64,
                        '-t', cls.candidate, str(assets)], check=True, capture_output=True)
        cls.images.append(cls.candidate)

    @classmethod
    def build(cls, recipe):
        image = 'yo3-dashboard-runtime-'+uuid.uuid4().hex+':fixture'
        (cls.root/'Dockerfile').write_text(recipe)
        subprocess.run(['docker', 'build', '-q', '-t', image, str(cls.root)], check=True, capture_output=True)
        cls.images.append(image)
        return image

    @classmethod
    def remove_images(cls):
        for image in reversed(cls.images):
            subprocess.run(['docker', 'image', 'rm', image], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def test_candidate_changes_only_assets_and_removes_old_including_hidden_files(self):
        deployer.assert_dashboard_runtime(self.base, self.candidate)
        result = subprocess.check_output(['docker', 'run', '--rm', '--network', 'none', '--read-only',
            '--cap-drop', 'ALL', '--entrypoint', '/bin/sh', self.candidate, '-ec',
            'test "$(id -u)" = 10001; test ! -e /usr/share/nginx/html/.obsolete; '
            'test ! -e /usr/share/nginx/html/old.js; test -r /usr/share/nginx/html/new.js; '
            'cat /usr/share/nginx/html/index.html'], text=True)
        self.assertEqual(result, '<html>New deployment</html>')

    def test_runtime_file_and_permission_changes_are_rejected(self):
        for command in ['echo forbidden >> /etc/nginx/nginx.conf', 'chmod 777 /usr/local/bin/yo3-nginx',
                        'touch /new-runtime-file']:
            with self.subTest(command=command):
                bad = self.build(f'FROM {self.candidate}\nUSER 0:0\nRUN {command}\nUSER 10001:10001\n')
                with self.assertRaisesRegex(ValueError, 'runtime'):
                    deployer.assert_dashboard_runtime(self.base, bad)

    def test_process_user_entrypoint_environment_and_unexpected_labels_are_rejected(self):
        for instruction in ['USER 0:0', 'ENTRYPOINT ["/bin/sh"]', 'ENV UNSAFE=changed', 'LABEL unexpected=changed']:
            with self.subTest(instruction=instruction):
                bad = self.build(f'FROM {self.candidate}\n{instruction}\n')
                with self.assertRaisesRegex(ValueError, 'configuration'):
                    deployer.assert_dashboard_runtime(self.base, bad)

    def test_assets_cannot_be_special_files_or_link_outside_the_web_root(self):
        for command in ['rm /usr/share/nginx/html/index.html',
                        'ln -s /etc/passwd /usr/share/nginx/html/private.txt',
                        'chmod 000 /usr/share/nginx/html/index.html',
                        'chmod 666 /usr/share/nginx/html/index.html']:
            with self.subTest(command=command):
                bad = self.build(f'FROM {self.candidate}\nUSER 0:0\nRUN {command}\nUSER 10001:10001\n')
                with self.assertRaisesRegex(ValueError, 'assets'):
                    deployer.assert_dashboard_runtime(self.base, bad)


if __name__ == '__main__':
    unittest.main()
