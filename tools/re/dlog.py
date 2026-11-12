#!/usr/bin/env python3
"""Parse YO3 duel logs (.dlog): header, run-length body, both player blocks and the engine command stream.

Usage: python3 dlog.py FILE.dlog ...            one summary per file
       python3 dlog.py --commands FILE.dlog      also every command {id, flags, p1..p4}
       python3 dlog.py --json FILE.dlog ...      everything as JSON lines
Layout: crates/yo3-server/src/duellog.rs (the client's loaders 0x538AB0 / 0x5384F0, decoder 0x538360).
"""
import json, struct, sys

MAGIC = 0x09E7
BLOCK = 0x21D0


def cstr(b):
    return b.split(b'\0', 1)[0].decode('latin1')


def unpack(coded):
    """(value, count) byte pairs."""
    out = bytearray()
    for i in range(0, len(coded) - 1, 2):
        out += bytes([coded[i]]) * coded[i + 1]
    return bytes(out)


def side(b):
    head = struct.unpack_from('<11I', b, 0)
    keys = ('player_id', 'dlv', 'main', 'extra', 'title', 'unk5', 'player_type', 'protector', 'wins', 'losses', 'draws')
    s = dict(zip(keys, head))
    at = 44
    s['pname'] = cstr(b[at:at + 32]); at += 32
    s['avatar'] = b[at:at + 128].hex(); at += 128
    s['country'] = b[at:at + 4].hex(); at += 4
    s['speeches'] = [cstr(b[at + 256 * i:at + 256 * (i + 1)]) for i in range(32)]; at += 256 * 32
    s['deck_name'] = cstr(b[at:at + 256]); at += 256
    s['tail_nonzero'] = any(b[at:BLOCK])
    return s


def parse(data):
    magic, version = struct.unpack_from('<HH', data, 0)
    if magic != MAGIC:
        raise ValueError(f'magic {magic:#x}')
    h = dict(zip(('unk4', 'duel_id', 'player_1', 'player_2', 'match_type', 'dlv_1', 'dlv_2', 'end_time'),
                 struct.unpack_from('<8I', data, 4)))
    h['version'] = version
    h['pname_1'] = cstr(data[36:68])
    h['pname_2'] = cstr(data[68:100])
    h['comment'] = data[100:356].split(b'\0', 1)[0].decode('utf-8', 'replace')
    h['comment_tail_nonzero'] = any(data[100 + len(data[100:356].split(b'\0', 1)[0]) + 1:356])
    h['len356'] = struct.unpack_from('<I', data, 356)[0]
    body = unpack(data[360:])
    h['body_len'] = len(body)
    h['ints'] = list(struct.unpack_from('<20I', body, 0))
    at = 80
    h['sides'] = [side(body[at:at + BLOCK]), side(body[at + BLOCK:at + 2 * BLOCK])]
    at += 2 * BLOCK
    n = struct.unpack_from('<I', body, at)[0]; at += 4
    h['count'] = n
    h['commands'] = [list(struct.unpack_from('<6I', body, at + 24 * i)) for i in range(n)]
    h['trailing'] = len(body) - (at + 24 * n)
    return h


if __name__ == '__main__':
    args = sys.argv[1:]
    show = '--commands' in args
    as_json = '--json' in args
    for path in (a for a in args if not a.startswith('--')):
        h = parse(open(path, 'rb').read())
        if as_json:
            print(json.dumps({'file': path, **h}))
            continue
        print(f"== {path}\n  dlog v{h['version']} unk4={h['unk4']} duel {h['duel_id']} type {h['match_type']} "
              f"end {h['end_time']} lens {h['len356']}/{h['body_len']} cmds {h['count']} trailing {h['trailing']}")
        print('  ints', h['ints'])
        for s in h['sides']:
            print('  side', {k: v for k, v in s.items() if k not in ('avatar', 'speeches')},
                  'speeches', sum(1 for x in s['speeches'] if x))
        if show:
            for c in h['commands']:
                print('   ', f'{c[0]:#06x}', f'{c[1]:#x}', *c[2:])
