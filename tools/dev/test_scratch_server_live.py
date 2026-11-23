#!/usr/bin/env python3
"""Real scratch-server start/env/status/stop: needs a built server and the original game installation.

Run explicitly with YO3_SERVER and YO3_GAME_DIR as needed. Ordinary Python CI checks are game-file-free.
"""
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

from test_scratch_server import HERE, scratch, free_port

BINARY = Path(os.environ.get('YO3_SERVER') or scratch.ROOT / 'target/release/yo3-server')


@unittest.skipUnless(BINARY.is_file(), f'no built server at {BINARY}')
class Live(unittest.TestCase):
    def run_tool(self, *args):
        return subprocess.run([sys.executable, HERE / 'scratch-server.py', *args], capture_output=True, text=True,
                              env=os.environ | {'YO3_SERVER': str(BINARY)})

    def test_start_env_status_stop(self):
        port, http = free_port(), free_port()
        started = self.run_tool('start', '--accounts', '2', '--port', str(port), '--http', str(http))
        self.assertEqual(started.returncode, 0, started.stderr)
        directory = Path(started.stdout.split()[2])
        try:
            set_ = subprocess.run([BINARY, 'set', '--data-dir', directory / 'data', 'turn-seconds', '0'], capture_output=True, text=True)
            self.assertEqual(set_.returncode, 0, set_.stderr)
            self.assertFalse(scratch.port_free(port), 'the server listens')
            profile = json.loads((directory / 'client-2.json').read_text())
            self.assertTrue(profile['server'].endswith(f':{port}'))
            self.assertTrue((directory / 'server.crt').is_file())
            self.assertIn(f'YO3_AUTH_PROFILE={directory / "client-2.json"}', self.run_tool('env', str(directory), '--account', '2').stdout)
            self.assertEqual(self.run_tool('status', str(directory)).returncode, 0)
            # An admin command right after start reaches the running server (it once found the data folder locked:
            # start returned before the server had written data/admin.json).
            set_ = subprocess.run([BINARY, 'set', '--data-dir', directory / 'data', 'ultimate-cpu-wait', '3'], capture_output=True, text=True)
            self.assertEqual(set_.returncode, 0, set_.stderr)
        finally:
            self.assertEqual(self.run_tool('stop', str(directory)).returncode, 0)
        self.assertEqual(self.run_tool('status', str(directory)).returncode, 1)
        self.assertTrue(scratch.port_free(port), 'the port is released')


if __name__ == '__main__':
    unittest.main()
