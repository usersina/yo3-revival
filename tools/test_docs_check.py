#!/usr/bin/env python3
"""The same link check must validate source docs and an extracted release."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECKER = Path(__file__).with_name("docs-check.py")


class DocsCheck(unittest.TestCase):
    def fixture(self, root):
        (root / "docs").mkdir()
        (root / "screenshots").mkdir()
        (root / "screenshots/desktop.png").write_bytes(b"image fixture")
        (root / "README.md").write_text("# Start\n[Guide](docs/guide.md#play)\n![App](screenshots/desktop.png)\n")
        (root / "docs/guide.md").write_text("# Guide\n## Play\n[Home](../README.md)\n![App](../screenshots/desktop.png)\n")

    def test_an_extracted_bundle_resolves_nested_docs_and_root_screenshots(self):
        with tempfile.TemporaryDirectory(prefix="yo3 docs ") as tmp:
            root = Path(tmp)
            self.fixture(root)
            result = subprocess.run([sys.executable, CHECKER, "--root", root], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_missing_screenshot_and_anchor_are_reported_in_the_requested_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.fixture(root)
            (root / "screenshots/desktop.png").unlink()
            (root / "docs/guide.md").write_text("# Guide\n## Renamed\n")
            result = subprocess.run([sys.executable, CHECKER, "--root", root], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing file screenshots/desktop.png", result.stdout)
            self.assertIn("no anchor docs/guide.md#play", result.stdout)

    def test_code_examples_are_ignored_and_toc_still_works(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.fixture(root)
            with (root / "docs/guide.md").open("a") as file:
                file.write("\n```md\n[Example](missing.md)\n```\n## Play\n")
            result = subprocess.run([sys.executable, CHECKER, "--root", root], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            toc = subprocess.check_output([sys.executable, CHECKER, "--toc", root / "docs/guide.md"], text=True)
            self.assertEqual(toc, "- [Play](#play)\n- [Play](#play-1)\n")


if __name__ == "__main__":
    unittest.main()
