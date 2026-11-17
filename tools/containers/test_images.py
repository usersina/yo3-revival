#!/usr/bin/env python3
"""Smoke-test the built runtime images, without game files, published ports or production data."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tarfile
import unittest

SERVER = "yo3-revival-server:local"
DASHBOARD = "yo3-revival-dashboard:local"
RELEASED_DASHBOARD = None


def run(image, *args, entrypoint="/bin/sh", input=None, mounts=()):
    volumes = [arg for target in mounts for arg in ["--mount", f"type=volume,dst={target}"]]
    return subprocess.run(["docker", "run", "--rm", "--network", "none", "--read-only", "--cap-drop", "ALL",
                           "--security-opt", "no-new-privileges:true", "--tmpfs", "/tmp:mode=1777,noexec,nosuid,nodev",
                           *volumes, "--entrypoint", entrypoint, "-i", image, *args], input=input, text=True,
                          capture_output=True, timeout=60)


class RuntimeImages(unittest.TestCase):
    def test_dashboard_only_changes_static_assets(self):
        if not RELEASED_DASHBOARD:
            self.skipTest('Supply --released-dashboard for an independent asset image')
        spec = importlib.util.spec_from_file_location('deployer', Path(__file__).resolve().parents[2]/'deploy/deploy-release.py')
        deployer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(deployer)
        deployer.assert_dashboard_runtime(RELEASED_DASHBOARD, DASHBOARD)

    def test_server_files_are_root_owned_and_have_no_setuid_or_setgid_bits(self):
        container = subprocess.check_output(["docker", "create", SERVER], text=True).strip()
        try:
            expected = {"opt/yo3/yo3-server": 0, "opt/yo3/yo3-engine": 0, "usr/bin/curl": 0,
                        "etc/yo3/server.toml": 0, "var/lib/yo3": 10001, "run/yo3-dashboard": 10001}
            seen = set()
            with subprocess.Popen(["docker", "export", container], stdout=subprocess.PIPE) as process:
                with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                    for member in archive:
                        self.assertEqual(member.mode & 0o6000, 0, member.name)
                        if member.name in expected:
                            self.assertEqual((member.uid, member.gid), (expected[member.name],) * 2, member.name)
                            seen.add(member.name)
                self.assertEqual(process.wait(), 0)
            self.assertEqual(seen, set(expected))
        finally:
            subprocess.run(["docker", "rm", container], check=True, stdout=subprocess.DEVNULL)

    def test_server_rootfs_is_minimal_and_non_root_with_compatible_tools(self):
        result = run(SERVER, "-ec", """
            test "$(id -u)" = 10001
            test "$(id -g)" = 10001
            test ! -e /etc/debian_version
            test ! -e /usr/bin/apt-get
            test ! -e /usr/bin/git
            test ! -e /usr/bin/gcc
            test -r /etc/ssl/certs/ca-certificates.crt
            test -r /etc/nsswitch.conf
            test -r /etc/passwd
            test -r /etc/group
            test -r /etc/os-release
            test -r /var/lib/dpkg/status
            test -w /var/lib/yo3
            test -w /run/yo3-dashboard
            test ! -w /opt/yo3
            test ! -w /opt/yo3/yo3-server
            test ! -w /etc/yo3/server.toml
            /opt/yo3/yo3-server --version
            /opt/yo3/yo3-engine --version
            curl --version
            cat /etc/passwd
        """, mounts=("/var/lib/yo3", "/run/yo3-dashboard"))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("yo3:x:10001:10001:", result.stdout)
        self.assertIn("EDOPro core", result.stdout)

    def test_native_engine_reads_its_bundled_database_scripts_and_real_cards(self):
        requests = [{"op": "identity"}, {"op": "cards", "cards": [
            {"cid": 4007, "name": "Blue-Eyes White Dragon"}, {"cid": 7734, "name": "Stardust Dragon"}]},
            {"op": "new", "seed": 7, "lp": 8000, "decks": [{"main": [4007] * 40, "extra": []}] * 2}]
        result = run(SERVER, entrypoint="/opt/yo3/yo3-engine", input="".join(json.dumps(r) + "\n" for r in requests))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(len(replies), 3)
        for field in ["database", "scripts", "card_ids", "core"]:
            self.assertRegex(replies[0][field], r"^[0-9a-f]{64}$")
        self.assertEqual(replies[1]["status"], "cards")
        cards = replies[1]["cards"]
        self.assertEqual(len(cards), 2)
        for card in cards:
            self.assertFalse(card["errors"], card)
            self.assertIsInstance(card["passcode"], int)
        self.assertEqual(cards[0]["data"]["name"], "Blue-Eyes White Dragon")
        self.assertEqual(cards[1]["data"]["name"], "Stardust Dragon")
        self.assertFalse(cards[1]["missing_scripts"], cards[1])
        self.assertEqual(replies[2]["status"], "await", replies[2])
        self.assertTrue(replies[2]["messages"], replies[2])
        self.assertFalse(replies[2].get("errors"), replies[2])

    def test_server_keeps_name_resolution_and_dropped_privileges(self):
        result = run(SERVER, "--noproxy", "*", "--max-time", "2", "http://localhost:9", entrypoint="/usr/bin/curl")
        self.assertEqual(result.returncode, 7, result.stderr)  # Resolved localhost, then refused the unused port.
        result = run(SERVER, "/proc/self/status", entrypoint="/bin/cat")
        self.assertEqual(result.returncode, 0, result.stderr)
        status = dict(line.split(":", 1) for line in result.stdout.splitlines())
        self.assertEqual(int(status["CapEff"].strip(), 16), 0)
        self.assertEqual(status["NoNewPrivs"].strip(), "1")
        mounts = run(SERVER, "/proc/self/mountinfo", entrypoint="/bin/cat")
        self.assertEqual(mounts.returncode, 0, mounts.stderr)
        temporary = next(line.split()[5].split(",") for line in mounts.stdout.splitlines() if line.split()[4] == "/tmp")
        self.assertTrue({"noexec", "nosuid", "nodev"}.issubset(temporary), temporary)

    def test_dashboard_runs_non_root_and_has_no_node_runtime(self):
        result = run(DASHBOARD, "-ec", """
            test "$(id -u)" = 10001
            test "$(id -g)" = 10001
            test ! -e /usr/local/bin/node
            test ! -w /usr/share/nginx/html/index.html
            test -r /usr/share/nginx/html/index.html
            nginx -v && openssl version
        """)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", default=SERVER)
    parser.add_argument("--dashboard", default=DASHBOARD)
    parser.add_argument("--released-dashboard")
    parser.add_argument("--dashboard-only", action="store_true")
    args, remaining = parser.parse_known_args()
    SERVER, DASHBOARD = args.server, args.dashboard
    RELEASED_DASHBOARD = args.released_dashboard
    if args.dashboard_only:
        names = [name for name in unittest.defaultTestLoader.getTestCaseNames(RuntimeImages) if name.startswith('test_dashboard')]
        suite = unittest.TestSuite(RuntimeImages(name) for name in names)
        raise SystemExit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
    unittest.main(argv=[__file__, *remaining])
