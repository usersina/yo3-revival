#!/usr/bin/env python3
"""From the map to a duel against the computer, in one command: Kaiba Corporation's NPC, through the talk, Start Duel,
Select Deck, the janken (picked again after a draw) and, when the janken is won, going first or second.

    tools/client/npc-duel.py [--second]        or: tools/client/vclient.sh npc [--second]

The client must be on the map, logged in on this instance (the usual YO3_INSTANCE / YO3_AUTH_PROFILE exports;
`vclient.sh start && vclient.sh login`). It drives tools/client/vclient.sh and reads its UI probe, so it never touches
the real screen. With a scratch server started with `--duel-field` (cards in place: team 0 is you) and `--duel-check`,
`vclient.sh duel` then names every card. Exit 1, with the UI view, when a step doesn't happen.
"""
import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VCLIENT = os.environ.get('YO3_VCLIENT', str(ROOT / 'tools/client/vclient.sh'))
# The 1024x768 frame's positions (AGENTS.md, "The headless client").
KAIBA_CORPORATION = (765, 275)
TALK = (320, 190)
YES = (270, 277)
DECK_OK = (393, 533)
ROCK = (350, 470)
FIRST, SECOND = (510, 293), (510, 380)


class Stuck(Exception):
    pass


def vclient(*args):
    return subprocess.run([VCLIENT, *map(str, args)], capture_output=True, text=True, check=False)


def ui():
    out = vclient('ui', '--raw')
    try:
        return json.loads(out.stdout)
    except ValueError:
        return {}


def flash_since(state, text, ms):
    """A Flash call containing `text` made in the last `ms` milliseconds."""
    return next((c['call'] for c in reversed(state.get('flash', [])) if text in c['call'] and c['ago_ms'] <= ms), None)


def wait_for(text, seconds, since_ms=None):
    """Waits for a Flash call containing `text` (made after the wait began, or in the last `since_ms`)."""
    start = time.monotonic()
    while time.monotonic() - start < seconds:
        window = since_ms if since_ms is not None else int((time.monotonic() - start) * 1000) + 400
        call = flash_since(ui(), text, window)
        if call:
            return call
        time.sleep(0.5)
    raise Stuck(f'{text} never came ({seconds} s)')


def has_component(name):
    return any(name in c['class'] for c in ui().get('components', []))


def run(go_second):
    vclient('click', *KAIBA_CORPORATION)
    time.sleep(3)
    began = time.monotonic()
    for _ in range(25):  # the NPC's talk, until the Start Duel? question
        vclient('click', *TALK)
        time.sleep(1.3)
        # Any time since the talk began: reading the UI probe takes long enough that a 1.5 s window missed it.
        if flash_since(ui(), 'showDialog', int((time.monotonic() - began) * 1000) + 400):
            break
    else:
        raise Stuck('the Start Duel? question never came (is the client on the map?)')
    vclient('click', *YES)
    for _ in range(40):
        if has_component('HolderSelect'):
            break
        time.sleep(0.5)
    else:
        raise Stuck('Select Deck never opened')
    time.sleep(2)
    vclient('click', *DECK_OK)
    wait_for('Janken.beginJanken', 25)
    for attempt in range(8):
        time.sleep(1)
        vclient('click', *ROCK)
        result = wait_for('Janken.resolveJanken', 12)
        print(f'janken: {result}')
        if '"win"' in result:
            wait_for('Janken.selectFirst', 10)
            time.sleep(1)
            vclient('click', *(SECOND if go_second else FIRST))
            print('going', 'second' if go_second else 'first')
            break
        if '"lose"' in result:
            print('the computer chooses who goes first')
            break
        wait_for('Janken.selectJanken', 10)  # a draw: pick again
    else:
        raise Stuck('eight janken draws in a row')
    for _ in range(90):  # the duel field, once the opening is dealt
        if ui().get('duel') and flash_since(ui(), 'ModeChange', 3000):
            print('in the duel')
            return
        time.sleep(1)
    raise Stuck('the duel never started')


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--second', action='store_true', help='go second when the janken is won (default: first)')
    args = p.parse_args(argv)
    try:
        run(args.second)
    except Stuck as error:
        print(f'npc-duel.py: {error}', file=sys.stderr)
        print(vclient('ui').stdout, file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
