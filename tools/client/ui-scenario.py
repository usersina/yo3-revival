#!/usr/bin/env python3
"""Play a scripted UI scenario on the headless client and check what the game shows.

    tools/client/ui-scenario.py tools/client/scenarios/card-search.scn
    tools/client/ui-scenario.py --check FILE...      parse only (no client)

The client must already be running and logged in on this instance (`vclient.sh start && vclient.sh login`, with the
usual YO3_INSTANCE/YO3_AUTH_PROFILE exports): the runner drives tools/client/vclient.sh and reads its UI probe.

A scenario is one step per line (# starts a comment; "quoted" arguments may hold spaces):

    click X Y                    left click at game coordinates (1024x768 frame)
    key KEY...                   xdotool key names (Return, Escape, BackSpace...)
    type TEXT                    type text
    sleep SECONDS
    shot NAME                    a screenshot (logs/<instance>/shots/NAME.png)
    expect text TEXT [SECONDS]   a UI text containing TEXT shown since the last action (waits, default 10 s)
    expect flash TEXT [SECONDS]  a Flash call containing TEXT since the last action
    expect component NAME [S]    an open component whose class contains NAME
    expect no-text TEXT [S]      no such UI text since the last action, for the whole time (default 3 s)
    expect no-flash TEXT [S]     no such Flash call since the last action, for the whole time

A failing step prints the client's UI view, takes a screenshot (scenario-fail-LINE) and exits 1.
"""
import json
import os
import shlex
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VCLIENT = os.environ.get('YO3_VCLIENT', str(ROOT / 'tools/client/vclient.sh'))
ACTIONS = {'click': 2, 'key': None, 'type': 1, 'sleep': 1, 'shot': 1}
EXPECTS = {'text': 10.0, 'flash': 10.0, 'component': 10.0, 'no-text': 3.0, 'no-flash': 3.0}


class ScenarioError(Exception):
    pass


def parse(text):
    """The steps: (line number, verb, args)."""
    steps = []
    for n, line in enumerate(text.splitlines(), 1):
        try:
            words = shlex.split(line, comments=True)
        except ValueError as e:
            raise ScenarioError(f'line {n}: {e}')
        if not words:
            continue
        verb, args = words[0], words[1:]
        if verb in ACTIONS:
            want = ACTIONS[verb]
            if (want is None and not args) or (want is not None and len(args) != want):
                raise ScenarioError(f'line {n}: {verb} takes {want or "at least one"} argument(s): {line.strip()}')
            if verb == 'click' and not all(a.isdigit() for a in args):
                raise ScenarioError(f'line {n}: click takes two numbers')
            if verb == 'sleep':
                float(args[0])
        elif verb == 'expect':
            if len(args) not in (2, 3) or args[0] not in EXPECTS:
                raise ScenarioError(f'line {n}: expect {"|".join(EXPECTS)} TEXT [SECONDS]: {line.strip()}')
            if len(args) == 3:
                float(args[2])
        else:
            raise ScenarioError(f'line {n}: unknown step {verb!r}')
        steps.append((n, verb, args))
    return steps


def vclient(*args, capture=False):
    r = subprocess.run([VCLIENT, *args], capture_output=capture, text=True)
    if r.returncode != 0:
        raise ScenarioError(f'vclient.sh {" ".join(args)} failed ({r.returncode}): {(r.stderr or "").strip()}')
    return r.stdout


def ui():
    return json.loads(vclient('ui', '--raw', capture=True))


def seen(state, kind, needle, since):
    """Whether `state` shows `needle` of `kind`, among texts and Flash calls newer than `since` seconds."""
    needle = needle.lower()
    if kind == 'component':
        return any(needle in c.get('class', '').lower() for c in state.get('components', []))
    entries = state.get('texts' if kind == 'text' else 'flash', [])
    field = 'text' if kind == 'text' else 'call'
    return any(needle in str(e.get(field, '')).lower() and e.get('ago_ms', 0) / 1000 <= since + 0.5 for e in entries)


def expect(kind, needle, timeout, last_action, read=ui, clock=time.monotonic, sleep=time.sleep):
    negative = kind.startswith('no-')
    base = kind[3:] if negative else kind
    deadline = clock() + timeout
    while True:
        found = seen(read(), base, needle, clock() - last_action)
        if found and negative:
            raise ScenarioError(f'{base} {needle!r} was shown')
        if found and not negative:
            return
        if clock() >= deadline:
            if negative:
                return
            raise ScenarioError(f'no {base} {needle!r} within {timeout:g} s')
        sleep(0.5)


def run(steps, name):
    last_action = time.monotonic()
    for n, verb, args in steps:
        try:
            print(f'{name}:{n}: {verb} {" ".join(args)}', flush=True)
            if verb == 'expect':
                kind = args[0]
                expect(kind, args[1], float(args[2]) if len(args) == 3 else EXPECTS[kind], last_action)
                continue
            if verb == 'sleep':
                time.sleep(float(args[0]))
                continue
            vclient(verb, *args)
            if verb != 'shot':
                last_action = time.monotonic()
        except ScenarioError as e:
            print(f'FAILED {name}:{n}: {e}', file=sys.stderr)
            try:
                print(vclient('ui', capture=True), file=sys.stderr)
                vclient('shot', f'scenario-fail-{n}')
            except ScenarioError as more:
                print(f'(and the UI view failed: {more})', file=sys.stderr)
            return 1
    print(f'{name}: passed')
    return 0


def main(argv):
    if not argv or argv[0] in ('-h', '--help'):
        print(__doc__)
        return 0 if argv else 2
    check = argv[0] == '--check'
    files = argv[1:] if check else argv
    status = 0
    for f in files:
        try:
            steps = parse(Path(f).read_text(encoding='utf-8'))
        except (OSError, ScenarioError, ValueError) as e:
            print(f'{f}: {e}', file=sys.stderr)
            return 2
        if check:
            print(f'{f}: {len(steps)} steps')
        else:
            status |= run(steps, Path(f).name)
    return status


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
