#!/usr/bin/env python3
"""Regression tests for the headless display helper; no X server, Wine or game needed."""
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
VCLIENT = ROOT / "tools/client/vclient.sh"


@unittest.skipUnless(os.name == "posix", "the headless helper requires Unix")
class DisplayStartup(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="yo3-vclient-test-")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.pid_file = self.directory / "pid"
        self.instance = self.directory.name
        self.logs = ROOT / "logs" / self.instance
        self.addCleanup(lambda: shutil.rmtree(self.logs, ignore_errors=True))
        self.env = {**os.environ, "PATH": f"{self.directory}:{os.environ['PATH']}",
            "YO3_INSTANCE": self.instance, "YO3_DISPLAY": ":9999", "TEST_XVFB_PID": str(self.pid_file)}
        self.program("xdpyinfo", """
from pathlib import Path
import os, sys
if not Path(os.environ['TEST_XVFB_PID']).exists(): sys.exit(1)
print('  dimensions:    1280x960 pixels')
""")
        self.program("Xvfb", """
from pathlib import Path
import os, signal, sys
print('display stdout', flush=True)
print('display stderr', file=sys.stderr, flush=True)
if os.environ.get('TEST_XVFB_FAIL'): sys.exit(1)
Path(os.environ['TEST_XVFB_PID']).write_text(str(os.getpid()))
signal.pause()
""")
        self.addCleanup(self.stop_display)

    def program(self, name, source):
        path = self.directory / name
        path.write_text(f"#!{sys.executable}\n" + source)
        path.chmod(0o755)

    def stop_display(self):
        if self.pid_file.exists():
            try:
                os.kill(int(self.pid_file.read_text()), signal.SIGTERM)
            except ProcessLookupError:
                pass

    def start(self):
        return subprocess.run([str(VCLIENT), "xvfb"], env=self.env, input="", capture_output=True,
            text=True, timeout=5)

    def test_cold_start_releases_captured_pipes_and_reuses_the_live_display(self):
        result = self.start()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("dimensions:", result.stdout)
        self.assertNotIn("display stdout", result.stdout)
        self.assertNotIn("display stderr", result.stderr)
        pid = int(self.pid_file.read_text())
        os.kill(pid, 0)  # The display survives after the helper exits.
        log = self.logs / "xvfb.log"
        self.assertEqual(log.read_text(), "display stdout\ndisplay stderr\n")

        again = self.start()
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertEqual(int(self.pid_file.read_text()), pid)
        self.assertEqual(log.read_text(), "display stdout\ndisplay stderr\n")

    def test_failed_display_start_reports_failure_and_keeps_diagnostics(self):
        self.env["TEST_XVFB_FAIL"] = "1"
        result = self.start()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.pid_file.exists())
        self.assertIn("display stderr", (self.logs / "xvfb.log").read_text())

    def test_bundle_gui_override_keeps_the_private_display_and_prefix(self):
        self.program("bundle-gui", """
import os
assert os.environ['DISPLAY'] == ':9999'
assert os.environ['WINEPREFIX'] == os.environ['YO3_WINEPREFIX']
assert 'WAYLAND_DISPLAY' not in os.environ
print('private bundle GUI')
""")
        self.env.update(YO3_DESKTOP_BIN=str(self.directory / "bundle-gui"),
            YO3_WINEPREFIX=str(self.directory / "private-prefix"), WAYLAND_DISPLAY="user-wayland")
        result = subprocess.run([str(VCLIENT), "desktop"], env=self.env, capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("private bundle GUI", (self.logs / "desktop.log").read_text())

    def test_direct_start_requires_installed_loader_before_running_wine(self):
        game = self.directory / "game"; game.mkdir(); (game / "yo3.exe").write_bytes(b"fixture")
        self.env.update(YO3_GAME_DIR=str(game), TEST_GAME_RAN=str(self.directory / "game-ran"))
        self.program("wine", """
from pathlib import Path
import os
Path(os.environ['TEST_GAME_RAN']).write_text('started')
""")
        self.program("xdotool", "print('999')")
        result = subprocess.run([str(VCLIENT), "start-stock"], env=self.env, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Press Play once in the GUI", result.stderr)
        self.assertFalse((self.directory / "game-ran").exists())

    def login(self, height, online, menu_low, slot):
        """`login select` against fake X tools: a window `height` high whose title menu fingerprints are at
        (297, online) and (297, menu_low), and whose first player slot is at (600, slot). The clicks made."""
        self.env["TEST_LOGIN_STATE"] = str(self.directory / "login-state")
        self.env["TEST_HEIGHT"] = str(height)
        self.env["TEST_PIXELS"] = f"{online},{menu_low},{slot}"
        self.program("sleep", "pass")
        self.program("xdotool", """
from pathlib import Path
import os, sys
state = Path(os.environ['TEST_LOGIN_STATE'])
args = sys.argv[1:]
if args[0] == 'search': print('999')
elif args[0] == 'getwindowgeometry': print('WINDOW=999\\nX=0\\nY=0\\nWIDTH=1024\\nHEIGHT=' + os.environ['TEST_HEIGHT'])
elif args[0] == 'mousemove': state.with_suffix('.position').write_text(' '.join(args[-2:]))
elif args[0] == 'click':
    with state.open('a') as clicks: clicks.write(state.with_suffix('.position').read_text() + '\\n')
""")
        self.program("import", """
from pathlib import Path
import os, sys
state = Path(os.environ['TEST_LOGIN_STATE'])
args = sys.argv[1:]
if '-crop' not in args:
    Path(args[-1]).write_bytes(b'fixture'); sys.exit(0)
pixel = args[args.index('-crop') + 1]
online, low, slot = os.environ['TEST_PIXELS'].split(',')
title = not state.exists() or state.read_text().count('\\n') == 0
rgb = (0,0,0)
if pixel == f'1x1+297+{online}': rgb = (39,61,117)
if pixel == f'1x1+297+{low}': rgb = (38,60,116)
if state.exists() and pixel == f'1x1+600+{slot}': rgb = (38,60,116)
print('0,0: (%d,%d,%d)' % rgb)
""")
        result = subprocess.run([str(VCLIENT), "login", "select"], env=self.env,
            capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        return (self.directory / "login-state").read_text()

    def test_online_goes_straight_to_player_selection_without_a_form_click(self):
        self.assertEqual(self.login(768, 301, 434, 231), "297 301\n")

    def test_a_1024x600_window_logs_in_with_its_own_title_layout(self):
        # Review (2026-11-15): at the game's 1024x600 size (Options -> Screen, or a screen too small for 768) the title
        # menu sits 75 px higher than in the 768 frame (live, Xvfb 1024x600); the player list does not move.
        self.assertEqual(self.login(600, 226, 359, 231), "297 226\n")

if __name__ == "__main__":
    unittest.main()
