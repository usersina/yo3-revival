#!/usr/bin/env python3
"""The LP "∞" picture the connection DLL carries (tools/mod/make-lp-infinity.py): the header is made from the picture,
and the picture is what the DLL expects. Standard library only."""
import importlib.util
from pathlib import Path
import struct
import unittest
import zlib

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("make_lp_infinity", HERE / "make-lp-infinity.py")
lp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lp)


class LpInfinity(unittest.TestCase):
    def test_the_header_is_made_from_the_picture(self):
        data = (HERE / "art" / "lp-infinity.png").read_bytes()
        self.assertEqual((HERE / "lp-infinity-png.h").read_text(), lp.header(data),
                         "run tools/mod/make-lp-infinity.py after changing art/lp-infinity.png")

    def test_the_picture_is_rgba_at_four_times_the_screen(self):
        data = (HERE / "art" / "lp-infinity.png").read_bytes()
        width, height, kind = lp.size(data)
        self.assertEqual(kind, 6, "the picture needs an alpha channel")
        # Shown at 25%: a whole number of screen pixels, and no taller than the LP disc's room (about 24 screen pixels).
        self.assertEqual((width % 4, height % 4), (0, 0))
        self.assertLessEqual(height, 96)
        self.assertLessEqual(width, 280)

    def test_a_picture_without_alpha_is_refused(self):
        def chunk(kind, body):
            return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        rgb = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 2, 0, 0, 0))
               + chunk(b"IDAT", zlib.compress(b"\0" + b"\0" * 12 * 4)) + chunk(b"IEND", b""))
        with self.assertRaises(ValueError):
            lp.header(rgb)
        with self.assertRaises(ValueError):
            lp.header(b"GIF89a")

    def test_the_placeholder_is_a_transparent_sign(self):
        data = lp.placeholder()
        width, height, kind = lp.size(data)
        self.assertEqual((width, height, kind), (lp.LP_W, lp.LP_H, 6))


if __name__ == "__main__":
    unittest.main()
