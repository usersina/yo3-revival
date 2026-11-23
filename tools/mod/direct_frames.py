"""Game frames for tools/mod/test-direct.sh (the fixture sends them, this reads the replies): the opener, the login request
0x1F47, and the reply ids. Blowfish with key "12345678", each 32-bit word little-endian (crates/yo3-proto)."""
import sys as _sys
if _sys.argv[1:2] in (["-h"], ["--help"]):
    print(__doc__); raise SystemExit(0)
import struct, sys
from Crypto.Cipher import Blowfish

def _swap(data):
    return b''.join(data[i:i + 4][::-1] for i in range(0, len(data), 4))

def frame(plain):
    plain += b'\0' * (-len(plain) % 8)
    body = _swap(Blowfish.new(b'12345678', Blowfish.MODE_ECB).encrypt(_swap(plain)))
    return struct.pack('>I', len(body) ^ 0x77777777) + body

def reply_id(hexframe):
    data = bytes.fromhex(hexframe)
    body = _swap(Blowfish.new(b'12345678', Blowfish.MODE_ECB).decrypt(_swap(data[4:])))
    assert body[0] == 4 and body[3:11] == b'\x07request', body[:16]
    return struct.unpack('>I', body[12:16])[0]

OPENER = frame(b'\x00\x122010.03.23 skelton')
LOGIN = frame(bytes([4]) + struct.pack('>H', 1) + b'\x07request' + bytes([5]) + struct.pack('>I', 0x1F47) + struct.pack('>H', 0))

if __name__ == '__main__':
    if sys.argv[1] == 'frames':
        print(OPENER.hex(), LOGIN.hex())
    else:  # replies FILE: the ids the fixture received, one line per connection
        for line in open(sys.argv[2]):
            kind, _, rest = line.strip().partition(' ')
            if kind in ('gate', 'front') and rest and not rest.startswith('refused'):
                print(kind, hex(reply_id(rest)))
            else:
                print(line.strip())
