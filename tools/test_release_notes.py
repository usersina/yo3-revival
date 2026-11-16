#!/usr/bin/env python3
"""Release notes and SHA256SUMS (#38): every archive is listed once, in sha256sum's format, and the notes say how to check it."""
import hashlib
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCRIPT = REPO / ".github/scripts/release_notes.py"
SPEC = importlib.util.spec_from_file_location("release_notes", SCRIPT)
notes = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(notes)

ARCHIVES = ("yo3-revival-desktop-v9.9.9-linux-x86_64.tar.gz", "yo3-revival-desktop-v9.9.9-windows-x86_64.zip",
            "yo3-revival-scripts-v9.9.9-linux-x86_64.tar.gz", "yo3-revival-scripts-v9.9.9-windows-x86_64.zip",
            "yo3-revival-server-v9.9.9-linux-x86_64.tar.gz")


class ReleaseNotes(unittest.TestCase):
    def release(self, root):
        assets = root / "assets"
        assets.mkdir()
        for name in ARCHIVES:
            (assets / name).write_bytes(name.encode() * 3)
        (root / "CHANGELOG.md").write_text("# Changelog\n\n## v9.9.9 (2026-11-16)\n\n- Something new.\n\n## v9.9.8 (2026-11-15)\n\n- Older.\n")
        return assets

    def test_sha256sums_lists_every_archive_once_and_sha256sum_accepts_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets = self.release(Path(tmp))
            subprocess.run([sys.executable, SCRIPT, "sums", assets], check=True)
            # Written twice (a rerun): the file never lists itself.
            subprocess.run([sys.executable, SCRIPT, "sums", assets], check=True)
            lines = (assets / "SHA256SUMS").read_text().splitlines()
            self.assertEqual([line.split("  ", 1)[1] for line in lines], sorted(ARCHIVES))
            for line in lines:
                digest, name = line.split("  ", 1)
                self.assertEqual(digest, hashlib.sha256((assets / name).read_bytes()).hexdigest())
            if subprocess.run(["sh", "-c", "command -v sha256sum"], capture_output=True).returncode == 0:
                subprocess.run(["sha256sum", "--quiet", "-c", "SHA256SUMS"], cwd=assets, check=True)

    def test_notes_list_the_same_checksums_and_say_how_to_check_on_windows(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            assets = self.release(root)
            subprocess.run([sys.executable, SCRIPT, "sums", assets], check=True)
            text = subprocess.run([sys.executable, SCRIPT, "notes", root / "CHANGELOG.md", assets], check=True,
                                  capture_output=True, text=True).stdout
            self.assertIn("- Something new.", text)
            self.assertNotIn("Older.", text)
            for line in (assets / "SHA256SUMS").read_text().splitlines():
                self.assertIn(line, text)
            self.assertNotIn("  SHA256SUMS", text)
            self.assertIn("Get-FileHash .\\yo3-revival-desktop-v9.9.9-windows-x86_64.zip", text)
            self.assertIn("`SHA256SUMS`", text)
            self.assertIn(notes.ANTIVIRUS, text)
            self.assertTrue((REPO / "docs/antivirus.md").is_file(), "the notes link the antivirus help page")


if __name__ == "__main__":
    unittest.main()
