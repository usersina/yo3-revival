#!/usr/bin/env python3
"""Replay one duel's exact engine inputs and compare every answer, including dependency fingerprints.

Two sources: a server's duel recording (`--recording FILE`, the `recordings/` files every duel writes, and the pinned
ones a player's `bug` report names), or a diagnostic session's log with the duel check on (`--server`).

This replays the rules engine, not the game UI. Use duel-report.py for actual-client checkpoint validation. The duel
bot's `think` calls are not replayed: their answers depend on the machine's speed (the bot stops trying options after a
time budget), and the duel itself is replayed exactly through the `respond` calls, which carry the answer it gave.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import zlib

spec = importlib.util.spec_from_file_location('duel_report', Path(__file__).with_name('duel-report.py'))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

def assemble(records, duel):
    selected = [r for r in records if r['duel'] == duel]
    if not selected: raise ValueError(f'No engine recording for duel {duel}.')
    groups = {}
    for r in selected:
        group = groups.setdefault((r['call'], r['direction']), {})
        if r['part'] in group: raise ValueError('Duplicate recording chunk; select matching session parts only.')
        group[r['part']] = r
    calls = []
    numbers = sorted({call for call, direction in groups})
    if numbers != list(range(1, max(numbers)+1)): raise ValueError('Missing initial/intermediate engine calls.')
    for call in numbers:
        pair = []
        for direction in ['request', 'answer']:
            parts = groups.get((call, direction), {})
            if not parts: raise ValueError(f'Incomplete call {call}: missing {direction}.')
            count = next(iter(parts.values()))['parts']
            if set(parts) != set(range(count)) or any(p['parts'] != count for p in parts.values()):
                raise ValueError(f'Incomplete chunks for call {call} {direction}.')
            if any(p.get('encoding') != 'hex' for p in parts.values()): raise ValueError('Unknown recording encoding.')
            pair.append(json.loads(bytes.fromhex(''.join(''.join(parts[p]['data']) for p in range(count)))))
        calls.append(pair)
    if calls[0][0] != {'op': 'identity'}: raise ValueError('Recording has no dependency identity.')
    check_identity(calls[0][1])
    return calls

def check_identity(identity):
    if not isinstance(identity, dict) or not isinstance(identity.get('node'), str) or any(
            not isinstance(identity.get(k), str) or len(identity[k]) != 64 or any(c not in '0123456789abcdef' for c in identity[k].lower())
            for k in ['sidecar', 'core', 'bindings', 'scripts', 'database', 'card_ids']):
        raise ValueError('Recording has no valid dependency fingerprints.')

def read_recording(path):
    """A server's duel recording (crates/yo3-server/src/recordings.rs): its header and the calls as (request, answer)
    pairs, the dependency identity first. A duel still running, or a server that stopped mid-duel, leaves the gzip
    stream without its end: the complete lines before it count."""
    # A decompressor object hands back everything decodable, where gzip.open would raise at the open end and drop it.
    text = zlib.decompressobj(16 + zlib.MAX_WBITS).decompress(Path(path).read_bytes())
    lines = text.decode('utf-8', errors='replace').split('\n')
    rows = []
    for number, line in enumerate(lines):
        if not line.strip(): continue
        try: rows.append(json.loads(line))
        except json.JSONDecodeError:
            if number == len(lines) - 1: break  # the last line, cut off mid-write
            raise ValueError(f'Line {number+1} of the recording is not JSON.')
    if not rows or rows[0].get('format') != 'yo3-duel-recording': raise ValueError('Not a duel recording (no yo3-duel-recording header).')
    header = rows[0]
    if header.get('version') != 1: raise ValueError(f'Unknown recording version {header.get("version")}.')
    check_identity(header.get('identity'))
    body = rows[1:]
    if [r.get('call') for r in body] != list(range(1, len(body)+1)): raise ValueError('Missing engine calls in the recording.')
    return header, [({'op': 'identity'}, header['identity'])] + [(r['request'], r['answer']) for r in body]

def canonical(answer):
    """The answer with each run of consecutive card and player hints (MSG_CARD_HINT 160, MSG_PLAYER_HINT 165) sorted.
    The core emits those while walking effects ordered by address, so their order may follow the heap (Windows
    randomises it); nothing else may differ (tools/engine/parity.py)."""
    if not isinstance(answer, dict) or not isinstance(answer.get('messages'), list):
        return answer
    out, run = [], []
    for m in answer['messages'] + [None]:
        if isinstance(m, dict) and m.get('type') in (160, 165):
            run.append(m)
            continue
        out += sorted(run, key=lambda h: json.dumps(h, sort_keys=True)); run = []
        if m is not None:
            out.append(m)
    return {**answer, 'messages': out}

def engine_program(engine):
    """The engine program: YO3_ENGINE, else the release bundle's (beside the engine folder), else a source build's."""
    name = 'yo3-engine.exe' if os.name == 'nt' else 'yo3-engine'
    for candidate in [os.environ.get('YO3_ENGINE'), engine.resolve().parent/name, Path(__file__).resolve().parents[2]/'target'/'release'/name]:
        if candidate and Path(candidate).is_file():
            return str(candidate)
    raise ValueError(f'No {name}: build it (cargo build --release -p yo3-engine) or set YO3_ENGINE.')

def run(calls, command, env=None):
    for request, _ in calls:
        if request.get('op') not in ['identity', 'new', 'respond', 'state', 'think']:
            raise ValueError('Unsupported recorded operation.')
    # The bot's thinking reads the duel without changing it, and its answer depends on time: not replayed.
    calls = [c for c in calls if c[0].get('op') != 'think']
    completed = subprocess.run(command, input=''.join(json.dumps(r)+'\n' for r, _ in calls), text=True,
                               capture_output=True, timeout=max(30, len(calls)*2), check=True, env=env)
    answers = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    problems = []
    if len(answers) != len(calls): problems.append(f'Expected {len(calls)} answers, got {len(answers)}.')
    for index, (actual, (_, expected)) in enumerate(zip(answers, calls), 1):
        if canonical(actual) != canonical(expected):
            problems.append(f'Call {index}: '+('dependency identity differs; use the recorded engine program and files.'
                            if index == 1 else 'engine answer differs.'))
            break
    return problems

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--recording', type=Path, help="a server's duel recording (recordings/…/ID.jsonl.gz)")
    source.add_argument('--server', nargs='+', type=Path, help='one server session, including rotated parts')
    parser.add_argument('--duel', type=int, help='with --server: the recording ID; omit to list available IDs')
    parser.add_argument('--engine', type=Path, default=Path('engine'), help='the engine folder (its vendor/ data)')
    args = parser.parse_args()
    if args.recording:
        header, calls = read_recording(args.recording)
        players = ' vs '.join(p.get('name') or '?' for p in header.get('players', []))
        print(f'Duel {header.get("id")} ({header.get("kind")}, {players}, started {header.get("started")}, server {header.get("server")}'
              + (', anime cards' if header.get('anime') else '') + f'): {len(calls)-1} engine calls')
        env = {k: v for k, v in os.environ.items() if k != 'YO3_ANIME'}
        if header.get('anime'): env['YO3_ANIME'] = '1'
        problems = run(calls, [engine_program(args.engine), str(args.engine.resolve())], env)
        for problem in problems: print('FAIL:', problem)
        if not problems: print(f'PASS: all {len(calls)} recorded engine calls replay exactly, with matching dependencies.')
        return int(bool(problems))
    lines = [line for p in args.server for line in p.read_text(errors='replace').splitlines()]
    records = report.records(lines, '[YO3 ENGINE] ')
    if args.duel is None:
        print('Recorded duels:', ', '.join(str(i) for i in sorted({r['duel'] for r in records})) or 'none')
        return 0
    calls = assemble(records, args.duel)
    problems = run(calls, [engine_program(args.engine), str(args.engine.resolve())])
    for problem in problems: print('FAIL:', problem)
    if not problems: print(f'PASS: all {len(calls)} recorded engine calls replay exactly, with matching dependencies.')
    return int(bool(problems))

if __name__ == '__main__':
    try: raise SystemExit(main())
    except (ValueError, subprocess.SubprocessError, OSError) as error: raise SystemExit(f'FAIL: {error}')
