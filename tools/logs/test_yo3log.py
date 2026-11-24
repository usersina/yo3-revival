#!/usr/bin/env python3
"""Hermetic checks for yo3log.py: yo3-log/1 and older logs in one timeline, the filters, report ZIPs."""
import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("yo3log", HERE / "yo3log.py")
yo3log = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(yo3log)

GAME = "game-1795292292-1084056-7ac252e391d9"
SERVER = "server-1795508896-1110607-aa0115a48eb5"
OLD_GAME = "\n".join([
    "1795292292 Session started",
    "1795292305 [YO3 MOD] v0.1.13 interface ready; network hooks installed",
    "1795292306 02b8:err:module:import_dll Library X not found",
    "1795292307 2026-11-21T20:18:27.054254Z  WARN [tls] 127.0.0.1:5 user 1001 handshake failed",
]) + "\n"
NEW_SERVER = "\n".join(json.dumps(d) for d in [
    {"ts": "2026-11-21T20:18:26.500Z", "level": "info", "src": "server", "session": SERVER, "msg": "launcher 1 connected", "topic": "tls", "user": 1001, "peer": "127.0.0.1:5", "conn": "front"},
    {"ts": "2026-11-21T20:18:28.000Z", "level": "error", "src": "server", "session": SERVER, "msg": "a seat left", "user": 1002, "duel": "20261121-201800-001"},
]) + "\n"
OLD_PACKET = json.dumps({"t": "2026-11-21T21:18:26.000+01:00", "server": "front", "dir": "<-", "msg": {"fields": {"request": 8007}}, "raw": "", "raw_len": 0, "error": None}) + "\n"


def run(*argv):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        yo3log.main(list(argv))
    return out.getvalue().splitlines()


class Viewer(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        (root / "sessions").mkdir()
        (root / "sessions" / f"{GAME}.0000.log").write_text(OLD_GAME)
        (root / "sessions" / f"{SERVER}.0000.log").write_text(NEW_SERVER)
        (root / "packets-20261121-211800-1-0.jsonl").write_text(OLD_PACKET)
        self.root = root

    def tearDown(self):
        self.tmp.cleanup()

    def test_old_and_new_logs_make_one_timeline_in_utc(self):
        lines = [json.loads(l) for l in run(str(self.root), "--json", "--traces")]
        self.assertEqual([l["ts"] for l in lines], sorted(l["ts"] for l in lines))
        by_msg = {l["msg"]: l for l in lines}
        self.assertEqual(by_msg["v0.1.13 interface ready; network hooks installed"]["src"], "dll")
        self.assertEqual(by_msg["Session started"]["session"], GAME)
        wine = next(l for l in lines if l["src"] == "wine")
        self.assertEqual(wine["level"], "error")
        old_server = next(l for l in lines if l["msg"].startswith("[tls] 127.0.0.1:5"))
        self.assertEqual((old_server["src"], old_server["level"], old_server["ts"]), ("server", "warn", "2026-11-21T20:18:27.054Z"))
        packet = next(l for l in lines if l["src"] == "packet")
        self.assertEqual((packet["ts"], packet["req"], packet["conn"]), ("2026-11-21T20:18:26.000Z", "0x1f47", "front"), "local time to UTC, hex id")

    def test_filters(self):
        self.assertEqual(len(run(str(self.root), "--level", "error")), 2, "the server's error and Wine's err:")
        self.assertTrue(all("packet" not in l for l in run(str(self.root))), "traces only with --traces")
        users = run(str(self.root), "--user", "1001", "--json")
        self.assertEqual([json.loads(l)["msg"] for l in users], ["launcher 1 connected"])
        self.assertEqual(len(run(str(self.root), "--duel", "20261121-201800-001")), 1)
        self.assertEqual(len(run(str(self.root), "--src", "wine")), 1)
        self.assertEqual(len(run(str(self.root), "--since", "20:18:27", "--until", "20:18:27.5")), 1, "a time window on the logs' day")
        self.assertEqual(len(run(str(self.root), "--grep", "seat")), 1)

    def test_readable_lines_and_report_files(self):
        line = run(str(self.root), "--level", "error", "--src", "server")[0]
        self.assertEqual(line, "2026-11-21 20:18:28.000 ERROR server a seat left  user=1002  duel=20261121-201800-001")
        report = self.root / "yo3-revival-report-20261121-201900.zip"
        with zipfile.ZipFile(report, "w") as z:
            z.writestr(f"sessions/{GAME}.0000.log", OLD_GAME)
            z.writestr("report.json", "{}")
        self.assertEqual(len(run(str(report))), 4, "a report's logs, not its other files")


    def test_text_gives_marker_tools_their_older_lines(self):
        # duel-replay.py and duel-report.py find `[YO3 ENGINE] {...}` and `ERROR` in lines: a yo3-log line, whose message
        # is JSON-escaped and whose marker is its src, reads as before through text().
        engine = json.dumps({"ts": "2026-11-24T08:00:00.000Z", "level": "info", "src": "engine", "session": SERVER, "msg": json.dumps({"duel": 1, "call": 0})})
        line = yo3log.text(engine)
        self.assertIn("[YO3 ENGINE] ", line)
        self.assertEqual(json.JSONDecoder().raw_decode(line.split("[YO3 ENGINE] ", 1)[1])[0], {"duel": 1, "call": 0})
        error = json.dumps({"ts": "2026-11-24T08:00:01.000Z", "level": "error", "src": "server", "session": SERVER, "msg": "duel check: drift"})
        self.assertIn("ERROR duel check", yo3log.text(error))
        self.assertEqual(yo3log.text("1795292292 Session started"), "1795292292 Session started")


if __name__ == "__main__":
    unittest.main()
