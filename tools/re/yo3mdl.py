#!/usr/bin/env python3
"""The game's 3D models (`.mdl`: NPC bodies data/model/np000-np024.mdl, avatar parts data/avatar/*.mdl), read and
written: `parse` gives a tree, `encode` writes it back byte-identical (all 25 NPC bodies and 285 avatar parts, 2026-11-26).
The loader is CYO3FileLoader (yo3.exe createMesh 0x594C00): Irrlicht's .X loader in binary form, giving a skinned mesh
(joints, skin weights, animation sets the duel cut-ins pick by index). The format: docs/client-internals.md, "3D models".

    python3 tools/re/yo3mdl.py FILE...    parse and re-encode each file, and say whether the bytes match"""
import struct, sys

MAGIC = b'\x89YO3\r\n\x1a\n'

class R:
    def __init__(s, d, p=0): s.d, s.p = d, p
    def u8(s): v = s.d[s.p]; s.p += 1; return v
    def u16(s): v = struct.unpack_from('<H', s.d, s.p)[0]; s.p += 2; return v
    def f32(s): v = struct.unpack_from('<f', s.d, s.p)[0]; s.p += 4; return v
    def raw(s, n):
        if s.p + n > len(s.d): raise EOFError
        v = s.d[s.p:s.p+n]; s.p += n; return bytes(v)
    def fl(s, n): return struct.unpack_from('<%df' % n, s.raw(n*4))
    def str(s): n = s.u8(); return s.raw(n)   # bytes, not NUL terminated
    def eof(s): return s.p >= len(s.d)

class W:
    def __init__(s): s.b = bytearray()
    def u8(s, v): s.b.append(v)
    def u16(s, v): s.b += struct.pack('<H', v)
    def f32(s, v): s.b += struct.pack('<f', v)
    def raw(s, v): s.b += v
    def fl(s, vs): s.b += struct.pack('<%df' % len(vs), *vs)
    def str(s, v): assert len(v) < 256; s.u8(len(v)); s.raw(v)

# ---- float RLE (skin weights): ctl byte = flag(0x80: every item stored) | run(1..127); first value after ctl
def read_rle_f32(r, n):
    out = []; runs = []
    while len(out) < n:
        ctl = r.u8(); run = ctl & 0x7f; flag = ctl >> 7
        v = r.raw(4); vals = [v]
        if flag: vals += [r.raw(4) for _ in range(run-1)]
        else: vals += [v]*(run-1)
        out += vals; runs.append((flag, run))
    assert len(out) == n, 'rle overshoot'
    return out, runs
def write_rle_f32(w, runs, vals):
    i = 0
    for flag, run in runs:
        w.u8(flag << 7 | run)
        w.raw(vals[i]); i += 1
        if flag:
            for k in range(run-1): w.raw(vals[i]); i += 1
        else: i += run-1

def parse_material(r):
    m = {}
    m['face'] = r.raw(4)        # R,G,B,A bytes
    m['power'] = r.raw(4)       # float
    m['specular'] = r.raw(3)    # RGB
    m['emissive'] = r.raw(3)    # RGB
    m['textures'] = []
    while True:
        t = r.u8()
        if t == 0: break
        assert t == 3, 'mat tag %d' % t
        m['textures'].append(r.str())
    return m
def write_material(w, m):
    w.raw(m['face']); w.raw(m['power']); w.raw(m['specular']); w.raw(m['emissive'])
    for t in m['textures']: w.u8(3); w.str(t)
    w.u8(0)

def parse_mesh(r):
    me = {'name': r.str()}
    nv = r.u16(); me['pos'] = r.raw(nv*12)           # per face-corner positions (3 floats each)
    nf = r.u16()
    sizes = []; ctl = []
    cnt = 0; size = None; sizes_run = []
    while len(sizes) < nf:
        c = r.u8(); ctl.append(c)
        run = (c >> 3) + 1; size = c & 7
        assert 3 <= size <= 7, 'face size'
        sizes += [size]*run
    assert len(sizes) == nf and sum(sizes) == nv, ('faces', len(sizes), nf, sum(sizes), nv)
    me['face_ctl'] = ctl; me['face_sizes'] = sizes
    me['sub'] = []
    while True:
        t = r.u8()
        if t == 0: break
        if t == 0x15:   # normals
            n = r.u16(); normals = r.raw(n*12)
            nfn = r.u16(); faces = []
            for i in range(nfn):
                k = r.u16(); faces.append(list(struct.unpack_from('<%dH' % k, r.raw(k*2))))
            me['sub'].append((t, dict(normals=normals, faces=faces)))
        elif t == 0x16:  # texcoords
            n = r.u16(); me['sub'].append((t, r.raw(n*8)))
        elif t == 0x17:  # vertex colours
            n = r.u16(); items = [(r.u16(), r.raw(4)) for _ in range(n)]
            me['sub'].append((t, items))
        elif t in (0x18, 0x1a):
            nm = r.u16(); k = r.u16()
            idx = list(r.raw(k)) if t == 0x18 else None
            d = dict(nmat=nm, k=k, idx=idx, mats=[])
            while True:
                s = r.u8()
                if s == 0: break
                if s == 0x19: d['mats'].append(('ref', r.str()))
                elif s == 2: d['mats'].append(('def', r.str(), parse_material(r)))
                else: raise ValueError('matlist sub %x' % s)
            me['sub'].append((t, d))
        elif t == 0x3c:
            me['sub'].append((t, (r.u16(), r.u16(), r.u16())))
        elif t == 0x46:  # skin weights for one joint
            name = r.str(); n = r.u16()
            ids = []; idruns = []
            while len(ids) < n:
                start = r.u16(); run = r.u8(); idruns.append((start, run)); ids += list(range(start, start+run))
            assert len(ids) == n
            vals, vruns = read_rle_f32(r, n)
            mat = r.raw(64)
            me['sub'].append((t, dict(name=name, n=n, idruns=idruns, vals=vals, vruns=vruns, mat=mat)))
        else:
            raise ValueError('mesh sub tag 0x%x at 0x%x' % (t, r.p-1))
    return me
def write_mesh(w, me):
    w.str(me['name']); nv = len(me['pos'])//12; w.u16(nv); w.raw(me['pos']); w.u16(len(me['face_sizes']))
    for c in me['face_ctl']: w.u8(c)
    for t, d in me['sub']:
        w.u8(t)
        if t == 0x15:
            w.u16(len(d['normals'])//12); w.raw(d['normals']); w.u16(len(d['faces']))
            for f in d['faces']: w.u16(len(f)); w.raw(struct.pack('<%dH' % len(f), *f))
        elif t == 0x16: w.u16(len(d)//8); w.raw(d)
        elif t == 0x17:
            w.u16(len(d))
            for i, c in d: w.u16(i); w.raw(c)
        elif t in (0x18, 0x1a):
            w.u16(d['nmat']); w.u16(d['k'])
            if t == 0x18: w.raw(bytes(d['idx']))
            for m in d['mats']:
                if m[0] == 'ref': w.u8(0x19); w.str(m[1])
                else: w.u8(2); w.str(m[1]); write_material(w, m[2])
            w.u8(0)
        elif t == 0x3c:
            for v in d: w.u16(v)
        elif t == 0x46:
            w.str(d['name']); w.u16(d['n'])
            for s, run in d['idruns']: w.u16(s); w.u8(run)
            write_rle_f32(w, d['vruns'], d['vals']); w.raw(d['mat'])
    w.u8(0)

def parse_node(r):
    nd = dict(name=r.str(), items=[])
    while True:
        t = r.u8()
        if t == 0: break
        if t == 10: nd['items'].append((10, parse_node(r)))
        elif t == 0xb: nd['items'].append((0xb, r.raw(64)))
        elif t == 0xc: nd['items'].append((0xc, None))
        elif t == 0x14: nd['items'].append((0x14, parse_mesh(r)))
        else: raise ValueError('node tag 0x%x at 0x%x' % (t, r.p-1))
    return nd
def write_node(w, nd):
    w.str(nd['name'])
    for t, v in nd['items']:
        w.u8(t)
        if t == 10: write_node(w, v)
        elif t == 0xb: w.raw(v)
        elif t == 0x14: write_mesh(w, v)
    w.u8(0)

def parse_keys(r, cam=False):
    ktype = r.u8(); n = r.u16(); sz = {0: 18, 1: 14, 2: 14, 9: 3}.get(ktype)
    assert sz, 'key type %d' % ktype
    return (ktype, [r.raw(sz) for _ in range(n)])

def parse(data):
    assert data[:8] == MAGIC
    r = R(data, 8); top = []
    while not r.eof():
        t = r.u8()
        if t == 0xff:
            top.append((0xff, None)); assert r.eof(), 'trailing bytes'; break     # end of file marker
        if t == 0: top.append((0, None)); continue       # stray terminators
        if t == 1: top.append((1, r.u16()))
        elif t == 2: top.append((2, (r.str(), parse_material(r))))
        elif t == 10: top.append((10, parse_node(r)))
        elif t == 0x28:
            an = dict(name=r.str(), items=[])
            while True:
                s = r.u8()
                if s == 0: break
                if s == 0x29:
                    jn = r.str(); keys = []
                    while True:
                        k = r.u8()
                        if k == 0: break
                        assert k == 0x2a; keys.append(parse_keys(r))
                    an['items'].append((0x29, jn, keys))
                elif s == 0x32:
                    keys = []
                    while True:
                        k = r.u8()
                        if k == 0: break
                        assert k == 0x33
                        ktype = r.u8(); n = r.u16(); sz = {0: 14, 1: 14, 2: 6}[ktype]
                        keys.append((ktype, [r.raw(sz) for _ in range(n)]))
                    an['items'].append((0x32, keys))
                else: raise ValueError('anim tag 0x%x at 0x%x' % (s, r.p-1))
            top.append((0x28, an))
        else: raise ValueError('top tag 0x%x at 0x%x' % (t, r.p-1))
    return top

def encode(top):
    w = W(); w.raw(MAGIC)
    for t, v in top:
        w.u8(t)
        if t == 0xff: break
        if t == 1: w.u16(v)
        elif t == 2: w.str(v[0]); write_material(w, v[1])
        elif t == 10: write_node(w, v)
        elif t == 0x28:
            w.str(v['name'])
            for it in v['items']:
                w.u8(it[0])
                if it[0] == 0x29:
                    w.str(it[1])
                    for kt, ks in it[2]:
                        w.u8(0x2a); w.u8(kt); w.u16(len(ks))
                        for k in ks: w.raw(k)
                    w.u8(0)
                else:
                    for kt, ks in it[1]:
                        w.u8(0x33); w.u8(kt); w.u16(len(ks))
                        for k in ks: w.raw(k)
                    w.u8(0)
            w.u8(0)
    return bytes(w.b)

if __name__ == '__main__':
    if sys.argv[1:2] in (['-h'], ['--help']) or not sys.argv[1:]:
        print(__doc__); sys.exit(0)
    ok = bad = 0
    for f in sys.argv[1:]:
        d = open(f, 'rb').read()
        try:
            t = parse(d); e = encode(t)
            res = 'OK' if e == d else 'MISMATCH len %d vs %d' % (len(e), len(d))
        except Exception as ex:
            res = 'ERR %r' % (ex,)
        if res == 'OK': ok += 1
        else: bad += 1; print(f, res)
    print('roundtrip ok=%d bad=%d' % (ok, bad))
