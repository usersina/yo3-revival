# Usage: python3 emp.py <file.y3d|.y3a|.y3r> ...  (decrypts, then parses the EasyMultiPlay key/value format)
"""Exploratory parser for the EasyMultiPlay serializer seen in decrypted YO3 data files."""
import sys, struct, json, collections
sys.path.insert(0, sys.argv[0].rsplit('/', 1)[0])
from y3dec import decrypt

TAGS = collections.Counter()

class P:
    def __init__(s, b): s.b, s.p = b, 0
    def u8(s): v = s.b[s.p]; s.p += 1; return v
    def u16(s): v = struct.unpack_from('>H', s.b, s.p)[0]; s.p += 2; return v
    def u32(s): v = struct.unpack_from('>I', s.b, s.p)[0]; s.p += 4; return v
    def raw(s, n): v = s.b[s.p:s.p+n]; s.p += n; return v

def value(p, tag):
    TAGS[tag] += 1
    if tag in (5, 6): return p.u32()
    if tag == 7: return p.raw(p.u32()).decode('utf-8', 'replace')
    if tag == 8:
        r = p.raw(p.u32())
        return {'bin_u16be': [struct.unpack_from('>H', r, i)[0] for i in range(0, len(r) - 1, 2)]} if len(r) % 2 == 0 else r.hex()
    if tag in (9, 10, 11, 12):
        return dataset(p, tag)
    raise ValueError(f'unknown tag {tag:#x} at {p.p-1}: {p.b[p.p-1:p.p+24].hex(" ")}')

def dataset(p, tag):
    n = p.u16()
    out = {}
    for _ in range(n):
        k = p.raw(p.u8()).decode('latin1')
        out[k] = value(p, p.u8())
    return out

def parse(blob):
    p = P(blob)
    tag = p.u8()
    return {'_root_tag': tag, **dataset(p, tag)}, p.p, len(blob)

if __name__ == '__main__':
    for path in sys.argv[1:]:
        d = decrypt(open(path, 'rb').read())
        try:
            obj, used, total = parse(d)
            s = json.dumps(obj, ensure_ascii=False)
            print(f'== {path.split("/data/")[-1]} used {used}/{total}')
            print(s[:900])
        except Exception as e:
            print(f'== {path.split("/data/")[-1]} FAIL {e}')
    print('tags seen:', dict(TAGS))
