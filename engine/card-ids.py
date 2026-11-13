#!/usr/bin/env python3
"""Writes engine/card-ids.json from YGOPRODeck's card info (`fetch.sh --update`): for every card with a Konami id, its
passcode, its artworks' passcodes and its Konami ids, in the shape the engine reads (`vendor/cardinfo.json`, see
crates/yo3-engine/src/cards.rs). One card per line, so that a pin change reads as a diff.
Usage: python3 card-ids.py CARDINFO.json OUT.json
"""
import json, sys

source, out = sys.argv[1:3]
data = []
for c in json.load(open(source, encoding='utf-8'))['data']:
    misc = [{'konami_id': m['konami_id']} for m in c.get('misc_info') or [] if m.get('konami_id')]
    if misc:
        data.append({'id': c['id'], 'card_images': [{'id': i['id']} for i in c.get('card_images') or []], 'misc_info': misc})
text = json.dumps({'data': data}, separators=(',', ':'), ensure_ascii=False).replace('},{"id"', '},\n{"id"')
open(out, 'w', encoding='utf-8').write(text + '\n')
print(f'{len(data)} cards with Konami ids -> {out}')
