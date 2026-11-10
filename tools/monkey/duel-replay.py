#!/usr/bin/env python3
"""Replay one diagnostic duel's exact engine inputs and compare every answer, including dependency fingerprints.

This replays the rules engine, not the game UI. Use duel-report.py for actual-client checkpoint validation.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess

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
    identity = calls[0][1]
    if not isinstance(identity.get('node'), str) or any(
            not isinstance(identity.get(k), str) or len(identity[k]) != 64 or any(c not in '0123456789abcdef' for c in identity[k].lower())
            for k in ['sidecar', 'core', 'bindings', 'scripts', 'database', 'card_ids']):
        raise ValueError('Recording has no valid dependency fingerprints.')
    return calls

def run(calls, command):
    for request, _ in calls:
        if request.get('op') not in ['identity', 'new', 'respond', 'state']:
            raise ValueError('Unsupported recorded operation.')
    completed = subprocess.run(command, input=''.join(json.dumps(r)+'\n' for r, _ in calls), text=True,
                               capture_output=True, timeout=max(30, len(calls)*2), check=True)
    answers = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    problems = []
    if len(answers) != len(calls): problems.append(f'Expected {len(calls)} answers, got {len(answers)}.')
    for index, (actual, (_, expected)) in enumerate(zip(answers, calls), 1):
        if actual != expected:
            problems.append(f'Call {index}: '+('dependency identity differs; use the recorded engine files/Node version.'
                            if index == 1 else 'engine answer differs.'))
            break
    return problems

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server', nargs='+', type=Path, required=True, help='one server session, including rotated parts')
    parser.add_argument('--duel', type=int, help='recording ID; omit to list available IDs')
    parser.add_argument('--engine', type=Path, default=Path('engine'))
    args = parser.parse_args()
    lines = [line for p in args.server for line in p.read_text(errors='replace').splitlines()]
    records = report.records(lines, '[YO3 ENGINE] ')
    if args.duel is None:
        print('Recorded duels:', ', '.join(str(i) for i in sorted({r['duel'] for r in records})) or 'none')
        return 0
    calls = assemble(records, args.duel)
    bundled = args.engine/'node'/('node.exe' if os.name == 'nt' else 'node')
    problems = run(calls, [str(bundled.resolve()) if bundled.is_file() else 'node', str((args.engine/'duelcore.ts').resolve())])
    for problem in problems: print('FAIL:', problem)
    if not problems: print(f'PASS: all {len(calls)} recorded engine calls replay exactly, with matching dependencies.')
    return int(bool(problems))

if __name__ == '__main__':
    try: raise SystemExit(main())
    except (ValueError, subprocess.SubprocessError, OSError) as error: raise SystemExit(f'FAIL: {error}')
