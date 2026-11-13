#!/usr/bin/env python3
"""Readable view of the connection DLL's UI probe answer (`vclient.sh ui`; raw JSON with `--raw`)."""
import json
import sys


def view(state):
    lines = ['components:']
    for c in state.get('components', []):
        lines.append(f"  {'  ' * c['depth']}{c['class']} (key {c['key']})")
    if not state.get('components_readable', True):
        lines.append('  (the component tree was being changed; ask again)')
    lines.append('recent texts (newest last):')
    if not state.get('text_hook', True):
        lines.append('  (the text hook is off: the lookup at 0x51E6C0 was not as expected, or YO3_UI_PROBE_TEXTS=0)')
    for t in state.get('texts', [])[-15:]:
        lines.append(f"  {t['ago_ms'] / 1000:6.1f}s ago  #{t['id']}: {t['text']}")
    duel = state.get('duel')
    if duel:
        lines.append('duel field (as the client holds it):')
        for side in duel['sides']:
            cards = [f"zone {s['zone']}: uid {s['uid']} {'up' if s['face_up'] else 'down'} "
                     f"{'DEF' if s['defense'] else 'ATK' if s['zone'] < 5 else ''}".rstrip() for s in side['slots']]
            lines.append(f"  {side['side']}: " + ('; '.join(cards) if cards else 'empty'))
    return '\n'.join(lines)


if __name__ == '__main__':
    print(view(json.load(open(sys.argv[1], encoding='utf-8'))))
