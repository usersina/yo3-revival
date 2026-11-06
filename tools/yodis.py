"""Tiny yo3.exe disassembler: linear sweep of one function with string/RTTI annotations."""
import sys, struct, re
import capstone, pefile

EXE = "/home/sina/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3/yo3.exe"
pe = pefile.PE(EXE, fast_load=True)
BASE = pe.OPTIONAL_HEADER.ImageBase
IMG = pe.get_memory_mapped_image()
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = False

def rd(va, n): return IMG[va - BASE: va - BASE + n]
def u32(va): return struct.unpack('<I', rd(va, 4))[0]

def cstr(va):
    if not (BASE <= va < BASE + len(IMG)): return None
    b = rd(va, 80); e = b.find(b'\0')
    if e >= 3 and all(32 <= c < 127 for c in b[:e]): return b[:e].decode()
    w = rd(va, 160)
    try:
        s = w.decode('utf-16le').split('\0')[0]
        if len(s) >= 3 and all(32 <= ord(c) < 127 for c in s): return 'L"' + s + '"'
    except Exception: pass
    return None

def func(va, maxn=3000, stop_at_ret=True):
    out = []
    code = rd(va, 0x8000)
    for ins in md.disasm(code, va):
        note = ''
        for m in re.finditer(r'0x([0-9a-f]{6,8})', ins.op_str):
            v = int(m.group(1), 16)
            s = cstr(v)
            if s: note += f'  ; "{s}"'
        out.append((ins.address, ins.mnemonic, ins.op_str, note))
        if len(out) >= maxn: break
        if stop_at_ret and ins.mnemonic.startswith('ret') and rd(ins.address + ins.size, 1) == b'\xcc': break
    return out

if __name__ == '__main__':
    va = int(sys.argv[1], 16); n = int(sys.argv[2]) if len(sys.argv) > 2 else 3000
    mode = sys.argv[3] if len(sys.argv) > 3 else 'all'
    for a, m, o, note in func(va, n):
        if mode == 'all' or note or m == 'call' or (mode == 'cmp' and m in ('cmp', 'sub', 'je', 'jne', 'ja', 'jmp')):
            print(f'{a:08x}  {m:6s} {o}{note}')
