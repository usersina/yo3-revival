"""The bundle's documentation: developer documents left out, links to them sent to GitHub, the rest kept."""
import importlib.util
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("bundle_docs", Path(__file__).with_name("bundle-docs.py"))
docs = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(docs)


class BundleDocs(unittest.TestCase):
    def test_links_to_developer_documents_go_to_the_releases_tag(self):
        text = "See [the protocol](protocol.md#login), [Desktop](launcher.md) and [history](./history.md)."
        out = docs.rewrite(text, True, "v0.1.23")
        self.assertIn("](https://github.com/usersina/yo3-revival/blob/v0.1.23/docs/protocol.md#login)", out)
        self.assertIn("](https://github.com/usersina/yo3-revival/blob/v0.1.23/docs/history.md)", out)
        self.assertIn("[Desktop](launcher.md)", out)
        self.assertIn("/blob/main/docs/roadmap.md)", docs.rewrite("[r](docs/roadmap.md)", False, "v0.1.16-52-g6beeb62"))
        self.assertEqual(docs.rewrite("[r](docs/roadmap.md)", True, "v1.0.0"), "[r](docs/roadmap.md)", "not a sibling link")

    def test_the_bundle_has_no_developer_document_and_no_link_left_to_one(self):
        with tempfile.TemporaryDirectory() as tmp:
            docs.copy(tmp, "v0.1.23")
            names = {p.stem for p in (Path(tmp) / "docs").glob("*.md")}
            self.assertFalse(names & set(docs.DEV_DOCS))
            self.assertIn("launcher", names)
            # Every relative link and anchor of the bundle resolves (the release job's own check).
            import subprocess, sys
            check = subprocess.run([sys.executable, str(docs.REPO / "tools/docs/docs-check.py"), "--root", tmp], capture_output=True, text=True)
            self.assertEqual(check.returncode, 0, check.stdout + check.stderr)


if __name__ == "__main__":
    unittest.main()
