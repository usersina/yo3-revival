#!/usr/bin/env python3
"""The hosting scripts (#37): each monitor alert fires once when its problem starts and once when it ends, through a
webhook and a command; the off-machine backup copies the newest backup with the server's identity and stamps success.
A fake health endpoint, webhook, restic and rclone stand in for the server and the real destinations."""
import http.server
import json
import os
import subprocess
import tempfile
import threading
import time
import unittest
from pathlib import Path

HOST = Path(__file__).resolve().parent / "host"


class Fake(http.server.BaseHTTPRequestHandler):
    """GET answers the health JSON set on the server; POST records a webhook's body."""

    def do_GET(self):
        body = json.dumps(self.server.health).encode()
        self.send_response(self.server.status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        self.server.posts.append(json.loads(self.rfile.read(length)))
        self.send_response(204)
        self.end_headers()

    def log_message(self, *args):
        pass


def serve():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fake)
    server.health, server.status, server.posts = {}, 200, []
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


HEALTHY = {"ok": True, "saving": True, "engine": True, "backup_age_seconds": 30, "errors_recent": 0, "online": 3, "duels": 1,
           "maintenance": False, "release": "v0.0.30"}


class Monitor(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="yo3-monitor-")
        self.root = Path(self.tmp.name)
        self.health = serve()
        self.hook = serve()
        self.commanded = self.root / "commanded.txt"
        command = self.root / "notify.sh"
        command.write_text(f'#!/bin/sh\nprintf "%s\\n" "$1" >> "{self.commanded}"\n')
        command.chmod(0o755)
        self.env = {**os.environ, "YO3_HEALTH_URL": f"http://127.0.0.1:{self.health.server_port}/health",
                    "YO3_ALERT_WEBHOOK": f"http://127.0.0.1:{self.hook.server_port}/hook", "YO3_ALERT_COMMAND": str(command),
                    "YO3_MONITOR_STATE": str(self.root / "state"), "YO3_DATA_DIR": str(self.root),
                    "YO3_OFFSITE_STAMP": str(self.root / "last-success")}

    def tearDown(self):
        for server in (self.health, self.hook):
            server.shutdown()
            server.server_close()
        self.tmp.cleanup()

    def run_monitor(self, **env):
        before = len(self.hook.posts)
        result = subprocess.run(["sh", HOST / "yo3-monitor.sh"], env={**self.env, **env}, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        sent = [p["content"] for p in self.hook.posts[before:]]
        self.assertEqual(sent, [l for l in result.stdout.splitlines() if l.startswith("[yo3 ")], "the journal says what was sent")
        return sent

    def test_a_healthy_server_is_all_clear(self):
        self.health.health = HEALTHY
        result = subprocess.run(["sh", HOST / "yo3-monitor.sh"], env=self.env, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.stdout.strip(), "all clear")
        self.assertEqual(self.hook.posts, [])

    def test_each_alert_fires_once_when_it_starts_and_once_when_it_ends(self):
        problems = {
            "the server can't save": dict(health={**HEALTHY, "saving": False, "ok": False}),
            "the rules engine can't start": dict(health={**HEALTHY, "engine": False, "ok": False}),
            "the newest backup is 99999 seconds old": dict(health={**HEALTHY, "backup_age_seconds": 99999}),
            "7 errors in the server's log": dict(health={**HEALTHY, "errors_recent": 7}),
            "disk is": dict(health=HEALTHY, env={"YO3_DISK_MAX_PERCENT": "0"}),
            "off-machine backup is over": dict(health=HEALTHY, stamp=str(int(time.time()) - 200_000)),
        }
        for words, case in problems.items():
            with self.subTest(words):
                self.health.health = case["health"]
                stamp = self.root / "last-success"
                if "stamp" in case:
                    stamp.write_text(case["stamp"])
                env = case.get("env", {})
                started = self.run_monitor(**env)
                self.assertEqual(len(started), 1, started)
                self.assertIn("PROBLEM", started[0])
                self.assertIn(words, started[0])
                self.assertEqual(self.run_monitor(**env), [], "not again while it lasts")
                self.health.health = HEALTHY
                if "stamp" in case:
                    stamp.write_text(str(int(time.time())))
                ended = self.run_monitor()
                self.assertEqual(len(ended), 1, ended)
                self.assertIn("RESOLVED", ended[0])
        self.assertEqual(len(self.commanded.read_text().splitlines()), 2 * len(problems), "the command got each alert too")

    def test_a_server_that_doesnt_answer_is_down(self):
        self.env["YO3_HEALTH_URL"] = "http://127.0.0.1:9/health"
        sent = self.run_monitor()
        self.assertEqual(len(sent), 1)
        self.assertIn("doesn't answer its health check", sent[0])
        self.assertEqual(self.run_monitor(), [])

    def test_a_503_still_reports_what_is_wrong(self):
        # The server answers 503 when saving or the engine fails; the body says which.
        self.health.health, self.health.status = {**HEALTHY, "saving": False, "ok": False}, 503
        sent = self.run_monitor()
        self.assertEqual(len(sent), 1)
        self.assertIn("can't save", sent[0])

    def test_without_a_channel_alerts_go_to_the_journal(self):
        self.health.health = {**HEALTHY, "saving": False}
        env = {**self.env, "YO3_ALERT_WEBHOOK": "", "YO3_ALERT_COMMAND": ""}
        result = subprocess.run(["sh", HOST / "yo3-monitor.sh"], env=env, capture_output=True, text=True, timeout=30)
        self.assertIn("PROBLEM: the server can't save", result.stdout)
        self.assertEqual(self.hook.posts, [])


class Offsite(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="yo3 offsite ")
        self.root = Path(self.tmp.name)
        self.data = self.root / "data"
        for name in ("20261115-010000.000000000", "20261116-010000.000000000"):
            (self.data / "backups" / name).mkdir(parents=True)
            (self.data / "backups" / name / "server.db").write_text(name)
        (self.data / "server.crt").write_text("certificate")
        (self.data / "server.key").write_text("key")
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.calls = self.root / "calls.txt"
        for tool in ("restic", "rclone"):
            fake = self.bin / tool
            fake.write_text(f'#!/bin/sh\necho "{tool} $*" >> "{self.calls}"\n'
                            f'if [ "$1" = copy ]; then mkdir -p "{self.root}/remote" && cp "$2"/* "{self.root}/remote/"; fi\n')
            fake.chmod(0o755)
        self.env = {**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}", "YO3_DATA_DIR": str(self.data),
                    "YO3_OFFSITE_STATE": str(self.root / "state")}

    def tearDown(self):
        self.tmp.cleanup()

    def run_backup(self, *args, **env):
        return subprocess.run(["sh", HOST / "yo3-offsite-backup.sh", *args], env={**self.env, **env}, capture_output=True, text=True, timeout=30)

    def test_unconfigured_it_fails_and_says_what_to_set(self):
        result = self.run_backup()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not configured", result.stderr)
        self.assertFalse((self.root / "state/last-success").exists())
        self.assertIn("RESTIC_REPOSITORY", self.run_backup(YO3_OFFSITE="restic").stderr)

    def test_restic_backs_up_the_newest_backup_and_the_identity_then_prunes(self):
        result = self.run_backup(YO3_OFFSITE="restic", RESTIC_REPOSITORY="s3:example/bucket")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.calls.read_text().splitlines()
        newest = self.data / "backups/20261116-010000.000000000/server.db"
        self.assertEqual(calls[0], f"restic backup --tag yo3 --host yo3-server {newest} {self.data}/server.crt {self.data}/server.key")
        self.assertTrue(calls[1].startswith("restic forget --tag yo3 --host yo3-server --keep-daily 14 --keep-weekly 8 --prune"))
        stamp = int((self.root / "state/last-success").read_text())
        self.assertLess(abs(stamp - time.time()), 60)

    def test_rclone_copies_the_files_into_a_folder_per_backup(self):
        result = self.run_backup(YO3_OFFSITE="rclone", YO3_RCLONE_REMOTE="remote:yo3")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sorted(p.name for p in (self.root / "remote").iterdir()), ["server.crt", "server.db", "server.key"])
        self.assertEqual((self.root / "remote/server.db").read_text(), "20261116-010000.000000000")
        calls = self.calls.read_text()
        self.assertIn("remote:yo3/20261116-010000.000000000", calls)
        self.assertIn("rclone delete --min-age 30d remote:yo3", calls)

    def test_a_missing_identity_fails_before_copying(self):
        (self.data / "server.key").unlink()
        result = self.run_backup(YO3_OFFSITE="restic", RESTIC_REPOSITORY="s3:example/bucket")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("server.key is missing", result.stderr)
        self.assertFalse(self.calls.exists())


if __name__ == "__main__":
    unittest.main()
