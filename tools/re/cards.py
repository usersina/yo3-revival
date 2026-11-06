# Usage: python3 cards.py <dir with CARD_Prop.bin, CARD_Indx_E.bin, CARD_Name_E.bin>
import sys, struct, collections
X = sys.argv[1]
prop = open(f'{X}/CARD_Prop.bin', 'rb').read()
idx = open(f'{X}/CARD_Indx_E.bin', 'rb').read()
names = open(f'{X}/CARD_Name_E.bin', 'rb').read()
def name(i):
    off = struct.unpack_from('<I', idx, i * 8)[0]
    end = names.find(b'\0\0', off)
    while end % 2: end = names.find(b'\0\0', end + 1)
    return names[off:end].decode('utf-16le', 'replace')
n = len(prop) // 8
rows = []
for i in range(n):
    a, b = struct.unpack_from('<II', prop, i * 8)
    cid, atk, dfn = a & 0x3FFF, ((a >> 14) & 0x1FF) * 10, ((a >> 23) & 0x1FF) * 10
    rows.append((i, cid, atk, dfn, b, name(i)))
real = [r for r in rows if r[1]]
print('entries', n, 'nonzero', len(real), 'cid range', min(r[1] for r in real), max(r[1] for r in real), 'dups', len(real) - len({r[1] for r in real}))
for r in real[:5] + sorted(real, key=lambda r: r[1])[-12:]:
    i, cid, atk, dfn, b, nm = r
    print(f'{cid:5d} b={b:08x} k={b & 0x3f:2d} atk={atk:4d} def={dfn:4d}  {nm}')
kinds = collections.Counter(r[4] & 0x3f for r in real)
print('kind histogram', sorted(kinds.items()))
for k in sorted(kinds):
    ex = [r[5] for r in real if r[4] & 0x3f == k][:3]
    print(f'  kind {k:2d}: {ex}')
