#!/usr/bin/env python3
"""Hermetic checks for smoke.py (the VM run itself needs a VM: `task vm:smoke`)."""
import importlib.util
import os
import tempfile
import time
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('smoke', Path(__file__).resolve().with_name('smoke.py'))
smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke)


class Checks(unittest.TestCase):
    def test_the_game_window_is_centred_on_the_vms_screen(self):
        # Seen on the Windows 11 VM (1280x800): the 1024x600 client area starts at (128, 111); the title screen's
        # click target (512, 389) is then (640, 500) and ONLINE (297, 228) is (425, 339).
        origin = smoke.client_origin((1280, 800))
        self.assertEqual(origin, (128, 112))
        self.assertEqual(smoke.at(origin, smoke.ONLINE), (425, 340))

    def test_the_newest_windows_desktop_bundle_is_chosen(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertIsNone(smoke.newest_bundle(d))
            for i, name in enumerate(['yo3-revival-desktop-v1-windows-x86_64.zip', 'yo3-revival-scripts-v2-windows-x86_64.zip',
                                      'yo3-revival-desktop-v2-linux-x86_64.tar.gz', 'yo3-revival-desktop-v3-windows-x86_64.zip']):
                Path(d, name).write_text('x')
                os.utime(Path(d, name), (time.time() + i, time.time() + i))
            self.assertEqual(smoke.newest_bundle(d).name, 'yo3-revival-desktop-v3-windows-x86_64.zip')

    def test_the_guest_batch_starts_the_bundles_launcher_with_its_dll(self):
        bat = smoke.play_bat()
        self.assertIn(r'"C:\YO3-smoke\yo3-revival-desktop\yo3-launcher.exe" --profile C:\YO3-smoke\client-1.json', bat)
        self.assertIn(r'--dll "C:\YO3-smoke\yo3-revival-desktop\yo3-auth.dll"', bat)
        self.assertIn('\r\n', bat)

    def test_the_checks_read_the_servers_packet_log(self):
        log = '\n'.join(['"summary":"<- 0x1f4c {player_id=1001}"', '"summary":"0x2266 {}"', '"summary":"<- 0x2136 {room_id=1}"',
                         '"summary":"0x245a {room_id=1}"', '"chat_str":"\\"vm smoke 42\\""', '"summary":"0x2456 {}"',
                         'INFO [bug] #1 from TestPlayer1 (recording none)'])
        self.assertEqual(smoke.checks(log, 42), {'login': True, 'auto-join': True, 'chat': True, 'command': True})
        fields = log.replace('INFO [bug] #1 from TestPlayer1 (recording none)', 'INFO [bug] from TestPlayer1 player=1001 report=1 duel="none"')
        self.assertTrue(smoke.checks(fields, 42)['command'], "the server's line with fields (yo3-log)")
        missed = smoke.checks(log.replace('0x2136', '0x2137'), 7)
        self.assertEqual((missed['auto-join'], missed['chat']), (False, False))


if __name__ == '__main__':
    unittest.main()
