"""Map a client reply handler's `case 0x....:` labels to their field reads.

Reads the Ghidra export (re/yo3_decompiled.c). Pick the function by address, or by any text
in its body (a field name, say):

    python3 tools/re/handlers.py 0x435950
    python3 tools/re/handlers.py '"room_id"'

For each case it prints the field names it builds strings for (the keys it reads or writes)
and which typed getters it calls. The `logs` column is usually empty: the export drops the
client's log strings (they reach the printf helper through data pointers), so tie a log line
such as "chat room list success" to its case from the disassembly (tools/re/yodis.py).
"""
import re
import sys
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
GETTERS = {"FUN_00408f50": "uint", "FUN_00409020": "int", "FUN_004090f0": "string", "FUN_0040f1d0": "raw",
           "FUN_00410d60": "item[n]", "FUN_0040f290": "request"}


def functions(src):
    heads = [(m.start(), m.group(1), int(m.group(2), 16)) for m in re.finditer(r"// ===== (\S+) @ ([0-9a-f]{8})", src)]
    for i, (pos, name, addr) in enumerate(heads):
        end = heads[i + 1][0] if i + 1 < len(heads) else len(src)
        yield name, addr, src[pos:end]


def summarize(name, addr, body):
    print(f"== {name} @ {addr:#x}")
    blocks = re.split(r"\n\s*(case 0x[0-9a-f]+:|default:)", body)
    head, rest = blocks[0], blocks[1:]
    labels = [(rest[i], rest[i + 1]) for i in range(0, len(rest), 2)]
    if not labels:
        labels = [("(whole function)", head)]
    for label, text in labels:
        logs = [s for s in re.findall(r'"([^"]+)"', text) if " : " in s or s.startswith("front server")]
        keys = [s for s in re.findall(r'"([a-zA-Z_][a-zA-Z0-9_%]{1,30})"', text) if s not in ("request",)]
        calls = [GETTERS[c] for c in re.findall(r"\b(FUN_[0-9a-f]{8})\(", text) if c in GETTERS]
        if logs or keys or calls:
            print(f"  {label:18} logs={logs[:2]} keys={sorted(set(keys))[:14]} getters={sorted(set(calls))}")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    target = sys.argv[1]
    src = (ROOT / "re" / "yo3_decompiled.c").read_text(errors="replace")
    for name, addr, body in functions(src):
        if target.startswith("0x") and addr == int(target, 16) or not target.startswith("0x") and target in body:
            summarize(name, addr, body)


if __name__ == "__main__":
    sys.exit(main())
