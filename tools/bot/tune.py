#!/usr/bin/env python3
"""Tunes the duel bot's numbers (crates/yo3-engine/src/bot `Weights`) in the arena (tests/bot_arena.rs): each number in
turn is tried lower and higher against the current set, on the same duels; a change is kept when it wins clearly more
than it loses. Prints each trial and the set found, as the JSON a style's "weights" takes.

    tools/bot/tune.py [--duels 40] [--seed 7000] [--rounds 1] [--start '{"card": 700}'] [--only card,strength]

Run `cargo build --release -p yo3-engine` first: the arena runs the engine program in target/. Each trial is one arena
run (a few minutes). Confirm the result on other duels: YO3_ARENA_SEED=9000 YO3_ARENA_OURS='<the set>' ... (see
docs/development.md, "The duel bot").
"""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULTS = {'card': 700.0, 'strength': 0.6, 'exposure': 0.2, 'trap_set': 60.0, 'spell_set': -100.0, 'flip_set': 700.0, 'margin': 30.0,
            'boss': 800.0, 'piece': 250.0}
# How a number moves in a trial: a factor for the scale-like ones, a step for those that may cross zero.
STEPS = {'spell_set': 150.0, 'margin': 60.0, 'trap_set': 150.0}


def arena(ours, theirs, duels, seed):
    """(won, lost) of `ours` against `theirs` in one arena run."""
    env = dict(os.environ, YO3_ARENA_OURS=json.dumps(ours), YO3_ARENA_THEIRS=json.dumps(theirs),
               YO3_ARENA_DUELS=str(duels), YO3_ARENA_SEED=str(seed))
    env.pop('YO3_RIVAL_ENGINE', None)
    out = subprocess.run(['cargo', 'nextest', 'run', '--release', '-p', 'yo3-server', '--test', 'bot_arena', '--run-ignored', 'only',
                          '--no-capture'], cwd=ROOT, env=env, capture_output=True, text=True)
    found = re.search(r'this build won (\d+), lost (\d+)', out.stdout + out.stderr)
    if not found:
        sys.exit(f'the arena did not report:\n{(out.stdout + out.stderr)[-2000:]}')
    return int(found.group(1)), int(found.group(2))


def candidates(key, value):
    if key in STEPS:
        return [value - STEPS[key], value + STEPS[key]]
    return [value * 0.5, value * 1.5]


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--duels', type=int, default=40)
    p.add_argument('--seed', type=int, default=7000)
    p.add_argument('--rounds', type=int, default=1)
    p.add_argument('--start', default='{}', help='the numbers to start from (JSON, any subset of the defaults)')
    p.add_argument('--only', help='the numbers to try, comma-separated (default: all)')
    p.add_argument('--margin', type=int, default=4, help='a change is kept when it wins this many more duels than it loses')
    args = p.parse_args(argv)
    best = dict(DEFAULTS, **json.loads(args.start))
    keys = args.only.split(',') if args.only else list(DEFAULTS)
    for round_ in range(args.rounds):
        for key in keys:
            for value in candidates(key, best[key]):
                trial = dict(best, **{key: round(value, 3)})
                won, lost = arena(trial, best, args.duels, args.seed)
                kept = won - lost >= args.margin
                print(f'round {round_ + 1} {key}={trial[key]}: won {won}, lost {lost}' + (' -> kept' if kept else ''), flush=True)
                if kept:
                    best = trial
                    break
    print(json.dumps(best))
    return 0


if __name__ == '__main__':
    sys.exit(main())
