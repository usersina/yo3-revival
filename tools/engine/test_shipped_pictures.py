"""The shipped card pictures: every listed face is committed as a 200x290 PNG and nothing unlisted ships."""
import importlib.util
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("shipped", Path(__file__).with_name("shipped-pictures.py"))
shipped = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(shipped)


class ShippedPictures(unittest.TestCase):
    def test_the_committed_faces_are_the_list(self):
        self.assertEqual(shipped.problems(), [])

    def test_a_missing_wrong_or_unlisted_file_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            (folder / "1.png").write_bytes(b"\x89PNG\r\n\x1a\n" + b"\0" * 16)
            found = shipped.problems(folder)
            self.assertTrue(any("missing" in line for line in found))
            self.assertIn("1.png: not listed in SHIPPED", found)


if __name__ == "__main__":
    unittest.main()
