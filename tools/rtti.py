"""Map MSVC RTTI class names to vtable method addresses in yo3.exe."""
import re, struct, sys
sys.path.insert(0, __file__.rsplit('/', 1)[0])
import yodis
IMG, BASE = yodis.IMG, yodis.BASE
RD_LO, RD_HI = 0x758000, 0x7f1000

def vtables():
    td = {m.start() + BASE - 8: m.group(1).decode() for m in re.finditer(rb'\.\?AV([A-Za-z0-9_@]+?)@@\x00', IMG)}
    cols = {}
    for va in range(RD_LO, RD_HI - 20, 4):
        if yodis.u32(va) == 0 and yodis.u32(va + 12) in td:
            cols[va] = td[yodis.u32(va + 12)]
    res = {}
    for va in range(RD_LO, RD_HI - 8, 4):
        c = yodis.u32(va)
        if c in cols:
            meths, p = [], va + 4
            while 0x401000 <= yodis.u32(p) < 0x758000:
                meths.append(yodis.u32(p)); p += 4
            res.setdefault(cols[c], []).append((va + 4, meths))
    return res

if __name__ == '__main__':
    vt = vtables()
    for name in sys.argv[1:]:
        for vva, ms in vt.get(name, []):
            print(f'{name} vtable@{vva:#x}: ' + ' '.join(f'{i}:{m:#x}' for i, m in enumerate(ms)))
