#!/usr/bin/env python3
"""Link real PE fixtures with a larger CRT/code section; no game, Wine or third-party Python modules needed."""
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
CC = os.environ.get("CC", "i686-w64-mingw32-gcc")


class FixtureLayout(unittest.TestCase):
    def link(self, root, base, padding):
        extra = root / "padding.s"
        extra.write_text(f'.section .text,"xr"\n.space {padding},0x90\n')
        output = root / "fixture.exe"
        result = subprocess.run([
            CC, "-O2", "-Wall", "-Wextra", "-Werror", f"-Wl,--image-base,{base:#x},-T,{ROOT / 'fixture-hooks.ld'}",
            "-o", str(output), str(ROOT / "test.c"), str(ROOT / "fixture-hooks.s"), str(extra),
            "-lws2_32", "-lwininet", "-lwinmm",
        ], capture_output=True, text=True)
        return result, output

    def assert_image(self, path, base):
        data = path.read_bytes()
        u16 = lambda offset: struct.unpack_from("<H", data, offset)[0]
        u32 = lambda offset: struct.unpack_from("<I", data, offset)[0]
        pe = u32(0x3c)
        self.assertEqual(data[pe:pe + 4], b"PE\0\0")
        optional = pe + 24
        self.assertEqual(u16(optional), 0x10b)  # PE32, like the game and required DLL.
        self.assertEqual(u32(optional + 28), base)
        alignment = u32(optional + 32)
        table = optional + u16(pe + 20)
        sections = [struct.unpack_from("<8sIIII", data, table + 40 * i) for i in range(u16(pe + 6))]
        end = (u32(optional + 60) + alignment - 1) // alignment * alignment
        for name, size, rva, raw_size, _ in sections:
            self.assertEqual(rva, end, f"Windows needs contiguous aligned sections: {name!r}")
            end += (max(size, raw_size) + alignment - 1) // alignment * alignment
        self.assertEqual(u32(optional + 56), end)
        text = next(section for section in sections if section[0].rstrip(b"\0") == b".text")
        self.assertGreater(text[1], 0x3000, "exercise the old 12 KiB limit")

        def at(rva, size):
            for _, _, start, raw_size, raw in sections:
                if start <= rva and rva + size <= start + raw_size:
                    return data[raw + rva - start:raw + rva - start + size]
            self.fail(f"RVA {rva:#x} is not backed by fixture bytes")

        for rva, expected in [(0x6964b, b"\xe8\x50\x6c\x08\x00"), (0xf44bd, b"\xe8\x3e\x05\x00\x00"),
                              (0x114212, b"\xe8\x79\x00\x00\x00"), (0x115a30, b"\xc3")]:
            self.assertEqual(at(rva, len(expected)), expected)
        self.assertEqual(at(0x1159ea, 5), b"\xb8" + struct.pack("<I", base + 0x115a30))
        reloc_rva, reloc_size = struct.unpack_from("<II", data, optional + 96 + 5 * 8)
        relocations = at(reloc_rva, reloc_size)
        fixed = set()
        cursor = 0
        while cursor < len(relocations):
            page, size = struct.unpack_from("<II", relocations, cursor)
            self.assertGreaterEqual(size, 8)
            self.assertLessEqual(cursor + size, len(relocations))
            for offset in range(cursor + 8, cursor + size, 2):
                entry = struct.unpack_from("<H", relocations, offset)[0]
                if entry >> 12 == 3:  # IMAGE_REL_BASED_HIGHLOW: Windows rebases the absolute callback.
                    fixed.add(page + (entry & 0xfff))
            cursor += size
        self.assertIn(0x1159eb, fixed)

    def test_larger_code_keeps_contiguous_sections_fixed_hooks_and_callback_relocation(self):
        for base in (0x400000, 0x600000):
            with self.subTest(base=hex(base)), tempfile.TemporaryDirectory() as directory:
                result, output = self.link(Path(directory), base, 0x9000)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assert_image(output, base)

    def test_code_cannot_overwrite_the_first_hook(self):
        with tempfile.TemporaryDirectory() as directory:
            result, _ = self.link(Path(directory), 0x400000, 0x70000)
            self.assertNotEqual(result.returncode, 0, "overlapping code must fail instead of moving the hook sites")


if __name__ == "__main__":
    unittest.main()
