#!/usr/bin/env python3
"""Runs the native rules engine and the old Node sidecar side by side, as one engine, and fails on any difference.

The server starts it in place of the engine (YO3_ENGINE=tools/engine/parity.py): each request goes to both
(`target/release/yo3-engine`, or YO3_PARITY_NATIVE; and `node ENGINE_DIR/duelcore.ts`, which needs the npm packages of
engine/fetch.sh), and the answers must be the same JSON, numbers compared as JavaScript numbers (the sidecar wrote
64-bit values through doubles). The native answer goes back; a difference answers {"status": "error"} instead, so the
duel fails, and is described in YO3_PARITY_LOG (default: parity-failures.log in the current directory).
Usage: YO3_ENGINE=$PWD/tools/engine/parity.py cargo test --release -p yo3-server --test duel_check
"""
import json, os, subprocess, sys

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
engine_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(repo, 'engine')
if sys.argv[1:2] == ['--version']:
    print('yo3-engine parity'); sys.exit(0)
native_path = os.environ.get('YO3_PARITY_NATIVE') or os.path.join(repo, 'target', 'release', 'yo3-engine')
log_path = os.environ.get('YO3_PARITY_LOG') or 'parity-failures.log'


def start(cmd):
    return subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)


def norm(v):
    if isinstance(v, bool) or v is None or isinstance(v, str):
        return v
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, list):
        return [norm(x) for x in v]
    return {k: norm(x) for k, x in v.items()}


def canonical(answer):
    """The answer with each run of consecutive card and player hints (MSG_CARD_HINT 160, MSG_PLAYER_HINT 165) sorted: the
    core emits those while walking
    an unordered set of effects keyed by pointer (effect resets, field::shuffle), so their order
    follows the build's pointer hashing, not the game. Nothing else may differ."""
    if not isinstance(answer, dict) or not isinstance(answer.get('messages'), list):
        return answer
    out, run = [], []
    for m in answer['messages'] + [None]:
        if isinstance(m, dict) and m.get('type') in (160, 165):
            run.append(m)
            continue
        out += sorted(run, key=lambda h: json.dumps(norm(h), sort_keys=True)); run = []
        if m is not None:
            out.append(m)
    return {**answer, 'messages': out}


def first_difference(a, b, path='$'):
    if type(a) is not type(b) and not (isinstance(a, (int, float)) and isinstance(b, (int, float))):
        return path, a, b
    if isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                return f'{path}.{k}', a.get(k, '<absent>'), b.get(k, '<absent>')
            d = first_difference(a[k], b[k], f'{path}.{k}')
            if d:
                return d
        return None
    if isinstance(a, list):
        for i, (x, y) in enumerate(zip(a, b)):
            d = first_difference(x, y, f'{path}[{i}]')
            if d:
                return d
        return (f'{path}.length', len(a), len(b)) if len(a) != len(b) else None
    return None if norm(a) == norm(b) else (path, a, b)


node = start(['node', os.path.join(engine_dir, 'duelcore.ts')])
native = start([native_path, engine_dir])
calls = []
for line in sys.stdin:
    if not line.strip():
        continue
    request = json.loads(line)
    calls.append(request)
    native.stdin.write(line if line.endswith('\n') else line + '\n'); native.stdin.flush()
    answer = native.stdout.readline()
    if request.get('op') != 'identity':
        node.stdin.write(line if line.endswith('\n') else line + '\n'); node.stdin.flush()
        expected = node.stdout.readline()
        a, b = json.loads(expected or 'null'), json.loads(answer or 'null')
        if norm(a) != norm(b) and norm(canonical(a)) != norm(canonical(b)):
            a, b = canonical(a), canonical(b)
            where, x, y = first_difference(a, b)
            context = ''
            if where.startswith('$.messages['):
                i = int(where[len('$.messages['):].split(']')[0])
                context = f'\n  node message:   {json.dumps(a["messages"][i])}\n  native message: {json.dumps(b["messages"][i] if i < len(b["messages"]) else None)}'
            with open(log_path, 'a') as log:
                log.write(f'call {len(calls)} {json.dumps(request)[:300]}\n  at {where}: node {json.dumps(x)[:300]} native {json.dumps(y)[:300]}{context}\n'
                          f'  calls: {json.dumps(calls)}\n\n')
            answer = json.dumps({'messages': [], 'status': 'error', 'error': f'parity: {where}: node {json.dumps(x)[:120]} native {json.dumps(y)[:120]}', 'errors': []}) + '\n'
    sys.stdout.write(answer); sys.stdout.flush()
