"""Follow a reply ID through one of yo3.exe's `switch (request)` dispatchers.

Concretely evaluates the compare/sub/jump-table chain that follows `call 0x40f290`
(the "request" getter) for each given ID, then lists the field names and typed getters
used by the case body it lands in.

    python3 tools/dispatch.py 0x4f4a4f 0x2398 0x21fc 0x2cf7
"""
import re
import struct
import sys

import capstone

import yodis

GETTERS = {0x409020: "int", 0x4090F0: "str", 0x40F1D0: "raw", 0x408F50: "int?", 0x410D60: "item[n]",
           0x410F10: "item*"}
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)


def insn_at(va):
    return next(md.disasm(yodis.rd(va, 16), va))


def imm(op):
    v = int(op, 0)
    return v - (1 << 32) if v >= 1 << 31 else v


def route(start, value):
    """Concretely run the dispatch chain for `value`; return the case body address."""
    eax, flags, va = value, None, start
    for _ in range(200):
        ins = insn_at(va)
        m, o, nxt = ins.mnemonic, ins.op_str, ins.address + ins.size
        if m == "cmp" and o.startswith("eax, "):
            flags = (eax, imm(o[5:]))
        elif m in ("sub", "add") and o.startswith("eax, "):
            k = imm(o[5:])
            eax = (eax - k) if m == "sub" else (eax + k)
            eax = ((eax + (1 << 31)) % (1 << 32)) - (1 << 31)
            flags = (eax, 0)
        elif m == "movzx" and re.match(r"eax, byte ptr \[eax \+ (0x[0-9a-f]+)\]", o):
            eax = yodis.rd(int(re.search(r"0x[0-9a-f]+", o).group(0), 16) + eax, 1)[0]
        elif m == "jmp" and o.startswith("dword ptr [eax*4 + "):
            va = yodis.u32(int(o.split("+ ")[1].rstrip("]"), 16) + 4 * eax)
            return va
        elif m.startswith("j") and o.startswith("0x"):
            a, b = flags if flags else (eax, 0)
            ua, ub = a % (1 << 32), b % (1 << 32)
            taken = {"je": a == b, "jne": a != b, "jg": a > b, "jl": a < b, "jge": a >= b, "jle": a <= b,
                     "ja": ua > ub, "jb": ua < ub, "jae": ua >= ub, "jbe": ua <= ub, "jmp": True}.get(m)
            if taken is None:
                return va
            va = int(o, 16) if taken else nxt
            continue
        elif m in ("mov", "push", "lea") and va == start:
            pass
        else:
            return va
        va = nxt
    return va


def case_fields(va, limit=400):
    out = []
    for ins in md.disasm(yodis.rd(va, 0x2000), va):
        if limit == 0:
            break
        limit -= 1
        for mm in re.finditer(r"0x([0-9a-f]{6,8})", ins.op_str):
            s = yodis.cstr(int(mm.group(1), 16))
            if s and not s.startswith(".\\"):
                out.append(f'"{s}"')
        if ins.mnemonic == "call" and ins.op_str.startswith("0x") and int(ins.op_str, 16) in GETTERS:
            out.append(GETTERS[int(ins.op_str, 16)])
        if ins.mnemonic == "jmp" or ins.mnemonic.startswith("ret"):
            break
    return out


if __name__ == "__main__":
    start = int(sys.argv[1], 16)
    for v in sys.argv[2:]:
        rid = int(v, 16)
        body = route(start, rid)
        print(f"{rid:#06x} -> case @ {body:#x}: {' '.join(case_fields(body))}")
