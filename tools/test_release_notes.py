#!/usr/bin/env python3
"""Release notes and SHA256SUMS (#38): every archive is listed once, in sha256sum's format, and the notes say how to check it.
SHA256SUMS.sig (#32): the workflow's signing script signs with the key Desktop checks and refuses any other."""
import hashlib
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCRIPT = REPO / ".github/scripts/release_notes.py"
SIGN = REPO / ".github/scripts/sign_release.sh"
SPEC = importlib.util.spec_from_file_location("release_notes", SCRIPT)
notes = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(notes)

ARCHIVES = ("yo3-revival-desktop-v9.9.9-linux-x86_64.tar.gz", "yo3-revival-desktop-v9.9.9-windows-x86_64.zip",
            "yo3-revival-scripts-v9.9.9-linux-x86_64.tar.gz", "yo3-revival-scripts-v9.9.9-windows-x86_64.zip",
            "yo3-revival-server-v9.9.9-linux-x86_64.tar.gz")


class ReleaseNotes(unittest.TestCase):
    def test_release_requires_cargo_and_changelog_versions_to_match(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            self.release(root)
            manifest=root / "Cargo.toml"
            manifest.write_text('[workspace.package]\nversion = "9.9.8"\n')
            mismatch=subprocess.run([sys.executable,SCRIPT,"check",root / "CHANGELOG.md",manifest],capture_output=True,text=True)
            self.assertNotEqual(mismatch.returncode,0)
            self.assertIn("9.9.9",mismatch.stderr)
            manifest.write_text('[workspace.package]\nversion = "9.9.9"\n')
            match=subprocess.run([sys.executable,SCRIPT,"check",root / "CHANGELOG.md",manifest],capture_output=True,text=True)
            self.assertEqual(match.returncode,0,match.stderr)

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

    def test_the_signing_script_signs_with_desktops_key_only_and_the_signature_is_not_listed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            assets = self.release(root)
            subprocess.run([sys.executable, SCRIPT, "sums", assets], check=True)
            key = root / "test.pem"  # made for this test; the project's key is only a CI secret
            subprocess.run(["openssl", "genpkey", "-algorithm", "ed25519", "-out", key], check=True, capture_output=True)
            der = subprocess.run(["openssl", "pkey", "-in", key, "-pubout", "-outform", "DER"], check=True, capture_output=True).stdout
            public = der[-32:].hex()
            done = subprocess.run([SIGN, assets, key, public], capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertEqual(len((assets / "SHA256SUMS.sig").read_bytes()), 64)
            # Desktop's built-in key is not this one: the default check refuses it before signing anything.
            (assets / "SHA256SUMS.sig").unlink()
            refused = subprocess.run([SIGN, assets, key], capture_output=True, text=True)
            self.assertNotEqual(refused.returncode, 0)
            self.assertIn("is not Desktop's (dbb25eca", refused.stderr)
            self.assertFalse((assets / "SHA256SUMS.sig").exists())
            subprocess.run([SIGN, assets, key, public], check=True, capture_output=True)
            # A rerun of `sums` and the notes leave the signature out of the list.
            subprocess.run([sys.executable, SCRIPT, "sums", assets], check=True)
            self.assertNotIn("SHA256SUMS.sig", (assets / "SHA256SUMS").read_text())
            text = subprocess.run([sys.executable, SCRIPT, "notes", root / "CHANGELOG.md", assets], check=True,
                                  capture_output=True, text=True).stdout
            self.assertNotIn("  SHA256SUMS.sig", text)
            self.assertIn("`SHA256SUMS.sig`", text)


if __name__ == "__main__":
    unittest.main()
