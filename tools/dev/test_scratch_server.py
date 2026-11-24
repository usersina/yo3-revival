#!/usr/bin/env python3
"""Hermetic checks for scratch-server.py; the real-server check is in test_scratch_server_live.py."""
import importlib.util
import socket
import subprocess
import sys
import sqlite3
import tempfile
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('scratch', HERE / 'scratch-server.py')
scratch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scratch)


def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]


class Checks(unittest.TestCase):
    def test_the_server_is_known_by_its_data_folder_not_its_program_file(self):
        # A rebuild replaces the program file under a running server; stop must still find it (it left it running).
        with tempfile.TemporaryDirectory() as d:
            mine, other = Path(d) / 'mine', Path(d) / 'other'
            child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)', '--data-dir', str(mine / 'data')])
            try:
                for _ in range(50):  # until the child has replaced the test's own command line with its own
                    if scratch.alive(child.pid, mine):
                        break
                    time.sleep(0.05)
                self.assertTrue(scratch.alive(child.pid, mine))
                self.assertFalse(scratch.alive(child.pid, other))
            finally:
                child.kill()
                child.wait()
            self.assertFalse(scratch.alive(child.pid, mine))

    def test_a_taken_port_is_seen(self):
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            s.listen()
            self.assertFalse(scratch.port_free(s.getsockname()[1]))
        self.assertTrue(scratch.port_free(free_port()))

    def test_a_server_bound_elsewhere_is_seen_listening_there(self):
        # A VM's server binds 192.168.122.1 (`-- --bind ADDRESS`): its readiness is checked on that address.
        self.assertEqual(scratch.bind_address(['--bind', '192.168.122.1']), '192.168.122.1')
        self.assertEqual(scratch.bind_address(['--duel-lp', '1000', '--bind=0.0.0.0']), '0.0.0.0')
        self.assertEqual(scratch.bind_address([]), '127.0.0.1')
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            s.listen()
            port = s.getsockname()[1]
            self.assertTrue(scratch.listening(port, '127.0.0.1'))
            self.assertTrue(scratch.listening(port, '0.0.0.0'))
        self.assertFalse(scratch.listening(free_port(), '127.0.0.1'))

    def test_the_live_ports_and_a_missing_server_are_refused(self):
        self.assertEqual(scratch.main(['start', '--port', '7443']), 2)
        self.assertEqual(scratch.main(['start', '--server', '/nonexistent/yo3-server', '--port', str(free_port()),
                                       '--http', str(free_port())]), 2)
        self.assertEqual(scratch.main(['start', '--profile-for', 'Yugi']), 2)

    def test_dir_prints_the_folder_and_admin_runs_the_servers_command_on_its_data(self):
        # Scripts take the folder from `dir` (never from a listing), and `admin` adds this server's --data-dir itself.
        import contextlib, io, json
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d) / 'yo3-dev.test'
            (folder / 'data').mkdir(parents=True)
            fake = Path(d) / 'yo3-server'
            fake.write_text(f'#!{sys.executable}\nimport sys, pathlib\npathlib.Path(sys.argv[0] + ".args").write_text("|".join(sys.argv[1:]))\n')
            fake.chmod(0o755)
            (folder / 'state.json').write_text(json.dumps({'pid': 0, 'binary': str(fake), 'port': 1, 'http': 2, 'profiles': []}))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(scratch.main(['dir', str(folder)]), 0)
            self.assertEqual(out.getvalue().strip(), str(folder))
            self.assertEqual(scratch.main(['admin', '--dir', str(folder), 'set', 'anime-cards', 'on']), 0)
            self.assertEqual(Path(str(fake) + '.args').read_text(), f'set|anime-cards|on|--data-dir|{folder / "data"}')
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(scratch.main(['admin', '--dir', str(folder)]), 2, 'no command')
                self.assertEqual(scratch.main(['admin', '--dir', str(Path(d) / 'missing'), 'settings']), 2, 'no scratch server there')

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


if __name__ == '__main__':
    unittest.main()
