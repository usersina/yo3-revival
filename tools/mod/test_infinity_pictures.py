#!/usr/bin/env python3
"""The "∞" pictures the connection DLL carries (tools/mod/make-infinity-pictures.py): the header is made from the
pictures, and each is what the DLL expects. Standard library only."""
import importlib.util
from pathlib import Path
import struct
import unittest
import zlib

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("make_infinity_pictures", HERE / "make-infinity-pictures.py")
pictures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pictures)


class InfinityPictures(unittest.TestCase):
    def test_the_header_is_made_from_the_pictures(self):
        self.assertEqual((HERE / "infinity-pictures.h").read_text(), pictures.header(pictures.read()),
                         "run tools/mod/make-infinity-pictures.py after changing a picture in tools/mod/art/")

    def test_each_picture_is_rgba_at_four_times_the_screen_with_its_source(self):
        for name, data in pictures.read():
            width, height, kind = pictures.size(data)
            self.assertEqual(kind, 6, f"{name} needs an alpha channel")
            # Shown at 25%: a whole number of screen pixels, no taller than a damage digit's room (36 screen pixels).
            self.assertEqual((width % 2, height % 4), (0, 0), name)
            self.assertLessEqual(height, 144, name)
            self.assertTrue((HERE / "art" / name.replace(".png", ".svg")).is_file(), f"{name} has its SVG source")

    def test_a_picture_without_alpha_is_refused(self):
        def chunk(kind, body):
            return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        rgb = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 2, 0, 0, 0))
               + chunk(b"IDAT", zlib.compress(b"\0" + b"\0" * 12 * 4)) + chunk(b"IEND", b""))
        good = pictures.read()
        with self.assertRaises(ValueError):
            pictures.header([("lp-infinity.png", rgb)] + good[1:])
        with self.assertRaises(ValueError):
            pictures.size(b"GIF89a")


if __name__ == "__main__":
    unittest.main()
