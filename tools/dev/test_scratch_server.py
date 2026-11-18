#!/usr/bin/env python3
"""scratch-server.py: its checks without a server, and a real start/env/stop when a built yo3-server is available
(YO3_SERVER or target/release/yo3-server)."""
import importlib.util
import json
import os
import socket
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('scratch', HERE / 'scratch-server.py')
scratch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scratch)
BINARY = Path(os.environ.get('YO3_SERVER') or scratch.ROOT / 'target/release/yo3-server')


def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]


class Checks(unittest.TestCase):
    def test_a_taken_port_is_seen(self):
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            s.listen()
            self.assertFalse(scratch.port_free(s.getsockname()[1]))
        self.assertTrue(scratch.port_free(free_port()))

    def test_the_live_ports_and_a_missing_server_are_refused(self):
        self.assertEqual(scratch.main(['start', '--port', '7443']), 2)
        self.assertEqual(scratch.main(['start', '--server', '/nonexistent/yo3-server', '--port', str(free_port()),
                                       '--http', str(free_port())]), 2)
        self.assertEqual(scratch.main(['start', '--profile-for', 'Yugi']), 2)

    def test_the_profile_is_read_after_the_heading(self):
        text = 'Request ID: 1\nPrivate launcher profile (save as client.json):\n{"certificate":"server.crt","key":"k","server":"localhost:17443"}\n'
        self.assertEqual(scratch.profile_from_key_output(text)['key'], 'k')
        with self.assertRaises(scratch.Fail):
            scratch.profile_from_key_output('Error: no account')

    def test_a_database_copy_includes_its_wal(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp, 'server.db')
            db = sqlite3.connect(source)
            db.execute('pragma journal_mode=wal')
            db.execute('create table t(v)')
            db.execute("insert into t values ('in the wal')")
            db.commit()  # still open: the row is only in server.db-wal
            scratch.copy_database(source, Path(tmp, 'copy.db'))
            db.close()
            copy = sqlite3.connect(Path(tmp, 'copy.db'))
            self.assertEqual(copy.execute('select v from t').fetchall(), [('in the wal',)])
            copy.close()


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
            self.assertFalse(scratch.port_free(port), 'the server listens')
            profile = json.loads((directory / 'client-2.json').read_text())
            self.assertTrue(profile['server'].endswith(f':{port}'))
            self.assertTrue((directory / 'server.crt').is_file())
            self.assertIn(f'YO3_AUTH_PROFILE={directory / "client-2.json"}', self.run_tool('env', str(directory), '--account', '2').stdout)
            self.assertEqual(self.run_tool('status', str(directory)).returncode, 0)
        finally:
            self.assertEqual(self.run_tool('stop', str(directory)).returncode, 0)
        self.assertEqual(self.run_tool('status', str(directory)).returncode, 1)
        self.assertTrue(scratch.port_free(port), 'the port is released')


if __name__ == '__main__':
    unittest.main()
