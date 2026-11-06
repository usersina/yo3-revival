"""List or extract .ypk archives (ZIP local headers with 'YO' signature, no central directory)."""
import sys, struct, zlib, os, fnmatch

def entries(path):
    with open(path, 'rb') as f:
        d = f.read()
    p = 0
    while p + 30 <= len(d) and d[p:p+4] == b'YO\x03\x04':
        _, flags, meth, _, _, crc, csz, usz, nl, el = struct.unpack_from('<HHHHHIIIHH', d, p + 4)
        name = d[p+30:p+30+nl].decode('latin1')
        off = p + 30 + nl + el
        yield name, meth, csz, usz, crc, d, off
        p = off + csz
    if p != len(d):
        print(f'!! {os.path.basename(path)}: stopped at {p}/{len(d)} sig={d[p:p+4]!r}', file=sys.stderr)

if __name__ == '__main__':
    mode, out = sys.argv[1], sys.argv[2]
    pats = sys.argv[3].split(',') if len(sys.argv) > 3 and sys.argv[3] else ['*']
    for path in sys.argv[4:]:
        for name, meth, csz, usz, crc, d, off in entries(path):
            if mode == 'list':
                print(f'{os.path.basename(path)}\t{meth}\t{usz}\t{name}')
            elif any(fnmatch.fnmatch(name, pt) for pt in pats):
                raw = d[off:off+csz]
                data = raw if meth == 0 else zlib.decompress(raw, -15)
                dst = os.path.join(out, os.path.basename(path), name)
                os.makedirs(os.path.dirname(dst), exist_ok=True)
                open(dst, 'wb').write(data)
