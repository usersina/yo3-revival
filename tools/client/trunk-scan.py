#!/usr/bin/env python3
"""Reads Edit Deck's trunk list from top to bottom on a headless client, as the game handed it to its Flash screen
(the connection DLL's UI probe, `vclient.sh ui --raw`): per card its place, name, ATK and DEF (65535 is "?"). It scrolls
the list one page at a time with the mouse wheel on the client's private display, so it never touches the real screen.

    YO3_INSTANCE=b tools/client/trunk-scan.py [--pages N] [--json FILE]

Run it with the Edit Deck screen open (and the sort or search you want to look at already applied). Output: one line per
card, `place name atk def`. The list ends where a page shows nothing new."""
import argparse
import json
import os
import re
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
VCLIENT = os.path.join(ROOT, "tools", "client", "vclient.sh")
CALL = re.compile(r'respond\((\d+), (\d+), (-?\d+), (-?\d+), "([^"]*)", "((?:[^"\\]|\\.)*)", (-?\d+), (-?\d+),')
LEN = re.compile(r"^respond\(\d+, (\d+), \d+\)$")   # the list handed over: its length and the trunk's card count
LENGTH = None
PAGE = 7                        # cards the trunk list shows


def display():
    instance = os.environ.get("YO3_INSTANCE", "")
    return os.environ.get("YO3_DISPLAY") or {"": ":77", "b": ":78", "c": ":79"}.get(instance, ":80")


def window(env):
    out = subprocess.run(["xdotool", "search", "--name", "^Yu-Gi-Oh! ONLINE 3"], env=env, capture_output=True, text=True).stdout.split()
    if not out:
        sys.exit("no game window on " + env["DISPLAY"])
    return out[0]


def page(env):
    """The cards of the list as last handed to Flash: {place: (name, atk, def, row)}; the call's third number is the card's place
    in the whole list (a scroll tick moves every card up one)."""
    raw = subprocess.run([VCLIENT, "ui", "--raw"], env=env, capture_output=True, text=True).stdout
    # Only the newest calls are the list as it is now: older ones are earlier lists (a search or sort before this one).
    global LENGTH
    lengths = [int(m.group(1)) for m in (LEN.search(e["call"]) for e in json.loads(raw)["flash"]) if m]
    if lengths:
        LENGTH = lengths[-1]
    hits = [m for m in (CALL.search(e["call"]) for e in json.loads(raw)["flash"]) if m and int(m.group(3)) >= 0]
    seen = {}
    for m in ([] if LENGTH == 0 else hits[-min(PAGE, LENGTH or PAGE):]):
        seen[int(m.group(3))] = (m.group(6), int(m.group(7)), int(m.group(8)), int(m.group(4)))
    return seen


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--pages", type=int, default=100)
    ap.add_argument("--json")
    args = ap.parse_args()
    env = dict(os.environ, DISPLAY=display())
    env.pop("WAYLAND_DISPLAY", None)
    w = window(env)
    # The list keeps its scroll position through a search or sort: go to the top first.
    subprocess.run(["xdotool", "mousemove", "--window", w, "860", "350", "click", "--repeat", "700", "--delay", "3", "4"], env=env)
    time.sleep(1.0)
    top = 0
    cards = {}
    stalled = 0
    for _ in range(args.pages * 3):
        # After a scroll the probe may still show the page before: ask again until a new place appears. A scroll that
        # brings nothing new three times in a row, or the list's last place, is the end.
        grew = False
        for _ in range(4):
            fresh = {k: v for k, v in page(env).items() if k not in cards}
            cards.update(fresh)
            grew = grew or bool(fresh)
            if fresh and max(fresh) >= top + PAGE - 1:
                break
            time.sleep(0.4)
        if LENGTH is not None and cards and max(cards) >= LENGTH - 1 and all(i in cards for i in range(LENGTH)):
            break
        stalled = 0 if grew else stalled + 1
        if stalled >= 3:
            break
        top = max(cards) - PAGE + 1 if cards else 0
        subprocess.run(["xdotool", "mousemove", "--window", w, "860", "350", "click", "--repeat", str(PAGE), "--delay", "120", "5"], env=env)
        time.sleep(0.5)
    for place in sorted(cards):
        name, atk, df, row = cards[place]
        print(place, name, atk, df)
    if args.json:
        json.dump([{"place": k, "name": v[0], "atk": v[1], "def": v[2], "row": v[3]} for k, v in sorted(cards.items())], open(args.json, "w"))


if __name__ == "__main__":
    main()
