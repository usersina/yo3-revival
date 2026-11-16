#!/usr/bin/env python3
"""The game-folder winmm.dll exports all of WINMM (#41); no Wine, game or third-party Python modules needed.

Windows' own DLLs in the game process load winmm.dll from the game folder too. Windows 10 1809's DSOUND.dll imports
waveInGetDevCapsW, which the loader once lacked, and the game closed at startup with 0xc0000139.
"""
import os
from pathlib import Path
import re
import struct
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
CC = os.environ.get("CC", "i686-w64-mingw32-gcc")
OWN = {"joyGetNumDevs", "joyGetDevCapsA", "joyGetPosEx", "timeGetTime", "timeBeginPeriod", "timeEndPeriod", "timeGetDevCaps"}
YO3 = {"Yo3LoaderAbi", "Yo3LoaderInfo", "Yo3LoaderValidate"}


def export_names(data):
    u16 = lambda offset: struct.unpack_from("<H", data, offset)[0]
    u32 = lambda offset: struct.unpack_from("<I", data, offset)[0]
    pe = u32(0x3C)
    sections, optional = u16(pe + 6), u16(pe + 20)
    table = pe + 24 + optional
    spans = [(u32(table + i * 40 + 12), u32(table + i * 40 + 8), u32(table + i * 40 + 20)) for i in range(sections)]

    def offset(rva):
        for start, size, raw in spans:
            if start <= rva < start + size:
                return raw + rva - start
        raise AssertionError(f"RVA {rva:#x} outside every section")

    exports = offset(u32(pe + 24 + 96))  # PE32: data directory 0, the export table
    count, names = u32(exports + 24), offset(u32(exports + 32))
    read = lambda rva: data[offset(rva):data.index(b"\0", offset(rva))].decode()
    return [read(u32(names + i * 4)) for i in range(count)]


class WinmmExports(unittest.TestCase):
    def test_the_loader_exports_every_winmm_function(self):
        header = (ROOT / "winmm-exports.h").read_text()
        forwarded = re.findall(r"^FORWARD\((\w+)\)$", header, re.M)
        self.assertEqual(forwarded, sorted(set(forwarded)), "sorted, each once")
        self.assertFalse(OWN & set(forwarded), "loader.c defines these itself")
        self.assertIn("waveInGetDevCapsW", forwarded, "DSOUND.dll on Windows 10 1809 imports it (#41)")
        with tempfile.TemporaryDirectory() as tmp:
            stub = Path(tmp) / "build" / "version.h"
            stub.parent.mkdir()
            stub.write_text('#define YO3_BUILD_VERSION "test"\n#define YO3_BUILD_COMMIT "test"\n#define YO3_MOD_ABI 3\n')
            dll = Path(tmp) / "winmm.dll"
            result = subprocess.run([
                CC, "-O2", "-Wall", "-Wextra", "-Werror", "-shared", "-Wl,--kill-at", f"-I{tmp}", "-o", str(dll),
                str(ROOT / "loader.c"), str(ROOT / "winmm-forward.c"), "-ladvapi32", "-lws2_32", "-lwininet", "-lcomdlg32",
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            names = export_names(dll.read_bytes())
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(set(names), set(forwarded) | OWN | YO3)


if __name__ == "__main__":
    unittest.main()
