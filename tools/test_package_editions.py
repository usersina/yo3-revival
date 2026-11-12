#!/usr/bin/env python3
"""Release contracts: stable roots, distinct contents, and literal script arguments."""
import importlib.util
import shutil
import sys
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

SPEC = importlib.util.spec_from_file_location("editions", Path(__file__).with_name("package-editions.py"))
editions = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(editions)


class Editions(unittest.TestCase):
    def test_both_editions_keep_offline_docs_and_screenshots_links_valid(self):
        repo = Path(__file__).resolve().parent.parent
        with tempfile.TemporaryDirectory(prefix="yo3 bundled docs ") as tmp:
            root = Path(tmp)
            source = root / "source"
            source.mkdir()
            for name in ("README.md", "LICENSE"):
                shutil.copyfile(repo / name, source / name)
            for name in ("docs", "screenshots"):
                shutil.copytree(repo / name, source / name)
            for edition in ("desktop", "scripts"):
                destination = root / edition
                editions.prepare(source, destination, edition, False)
                result = subprocess.run([sys.executable, repo / "tools/docs-check.py", "--root", destination], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertTrue((destination / "screenshots/desktop.png").is_file())
                if edition == "scripts":
                    self.assertIn("](docs/launcher.md)", (destination / "README.md").read_text())

    def test_each_platform_has_two_named_editions_with_stable_inner_folders(self):
        branding = editions.REPO / "crates/yo3-desktop/assets/branding"
        icons = {"yo3-revival.ico": (branding / "launcher.ico").read_bytes(),
                 "yo3-revival.png": (branding / "launcher-window.png").read_bytes()}
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for windows in (False, True):
                source = root / ("windows" if windows else "linux")
                source.mkdir()
                ext = ".exe" if windows else ""
                for name in ("yo3-revival" + ext, "yo3-server" + ext, "yo3-launcher" + ext,
                             "yo3-auth-launch.exe", "yo3-auth.dll", "yo3-bootstrap.dll", "yo3-launcher-windows.exe"):
                    (source / name).write_bytes(b"fixture")
                (source / "engine").mkdir()
                (source / "engine/fixture").write_bytes(b"shared engine")
                for edition in ("desktop", "scripts"):
                    destination = root / f"{edition}-{windows}" / f"yo3-revival-{edition}"
                    editions.prepare(source, destination, edition, windows)
                    self.assertEqual((destination / ("yo3-revival" + ext)).exists(), edition == "desktop")
                    self.assertEqual((destination / "yo3-bootstrap.dll").exists(), edition == "desktop")
                    self.assertTrue((destination / "yo3-auth-launch.exe").exists())
                    self.assertTrue((destination / "engine/fixture").exists())
                    self.assertEqual((destination / ("play.cmd" if windows else "play.sh")).exists(), edition == "scripts")
                    archive = editions.archive(destination, root, "v0.0.9", "windows-x86_64" if windows else "linux-x86_64")
                    self.assertEqual(archive.name, f"yo3-revival-{edition}-v0.0.9-{'windows-x86_64.zip' if windows else 'linux-x86_64.tar.gz'}")
                    if windows:
                        with zipfile.ZipFile(archive) as z:
                            roots = {name.split('/')[0] for name in z.namelist()}
                            archived_icons = {Path(name).name: z.read(name) for name in z.namelist() if Path(name).name in icons}
                    else:
                        with tarfile.open(archive) as t:
                            roots = {name.split('/')[0] for name in t.getnames()}
                            archived_icons = {Path(name).name: t.extractfile(name).read() for name in t.getnames() if Path(name).name in icons}
                    self.assertEqual(roots, {f"yo3-revival-{edition}"})
                    self.assertEqual(set(archived_icons), set(icons) if edition == "desktop" else set(), "Desktop downloads must carry the shortcut artwork")
                    for name, content in archived_icons.items():
                        self.assertTrue(content == icons[name], f"{name} must match the launcher artwork")

    def test_play_script_keeps_paths_and_arguments_literal(self):
        with tempfile.TemporaryDirectory(prefix="yo3 scripts ") as tmp:
            root = Path(tmp)
            source = root / "source"
            source.mkdir()
            helper = source / "yo3-launcher"
            helper.write_text('#!/bin/sh\nprintf "%s\\n" "$@"\n')
            helper.chmod(0o755)
            destination = root / "bundle"
            editions.prepare(source, destination, "scripts", False)
            args = ["--profile", "my profile.json", "--game-dir", "game with spaces;literal"]
            output = subprocess.check_output([destination / "play.sh", *args], text=True).splitlines()
            self.assertEqual(output[-len(args):], args)
            self.assertIn(str(destination / "yo3-auth.dll"), output)


if __name__ == "__main__":
    unittest.main()
