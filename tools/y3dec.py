"""Decrypt YO3 data files (.y3d/.y3a/.y3r): BE u32 length ^ 0x77777777, then Blowfish-ECB("12345678") with per-word byte swap."""
import sys, struct
from Crypto.Cipher import Blowfish

def swap32(b):
    return b''.join(b[i:i+4][::-1] for i in range(0, len(b), 4))

def decrypt(blob, key=b'12345678'):
    n = struct.unpack('>I', blob[:4])[0] ^ 0x77777777
    body = blob[4:4+n]
    bf = Blowfish.new(key, Blowfish.MODE_ECB)
    return swap32(bf.decrypt(swap32(body)))

if __name__ == '__main__':
    for p in sys.argv[1:]:
        d = decrypt(open(p, 'rb').read())
        print('==', p.split('/data/')[-1], len(d)); print(d[:96].hex(' '))
