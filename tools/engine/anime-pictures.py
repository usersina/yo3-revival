#!/usr/bin/env python3
"""Builds engine/anime/pictures.tsv, the list of the anime card pictures Desktop can fetch (issue #60): per card its Konami
id, the anime passcode to ask ProjectIgnis' picture server for (0 when the card has none) and the official passcode whose
normal picture stands in when the server has no anime one. The list holds no image and is compiled into Desktop
(crates/yo3-launcher/src/art/get.rs). The source is engine/anime-cards.json, so the one reviewed map decides which cards
have a picture, and engine/added-cards.json: the cards the game never had (#75) have no picture of their own, so each is
asked by its official passcode (anime 0); `--check` compares the committed list with a fresh build.

It also builds engine/anime/fields.tsv: the duel field pictures Desktop fetches with them (EDOPro's `field/<passcode>.png`),
one per added Field Spell (kind 13, subtype 2 in word b: the game has none for a card it never had), by the passcode its
card picture is fetched by."""
import argparse
import json
import os
import sys

ENGINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'engine')
OUT = os.path.join(ENGINE, 'anime', 'pictures.tsv')
FIELDS = os.path.join(ENGINE, 'anime', 'fields.tsv')


def build(data, added=()):
    lines = ['cid\tanime\tofficial']
    mapped = {c['cid'] for c in data['cards']}
    # An added card with an anime version is in the map already: its anime picture, the official one standing in.
    # An anime-only added card has only its anime picture: its passcode both ways.
    cards = list(data['cards']) + [{'cid': c['cid'], 'anime': c['passcode'] if c.get('anime_only') else 0, 'official': c['passcode']}
                                   for c in added if c['cid'] not in mapped]
    for card in sorted(cards, key=lambda c: c['cid']):
        anime, official = int(card.get('anime') or 0), int(card.get('official') or 0)
        if not anime and not official:
            sys.exit('card %s has no passcode to fetch a picture by' % card['cid'])
        lines.append('%d\t%d\t%d' % (card['cid'], anime, official))
    return '\n'.join(lines) + '\n'


def field_spell(card):
    """A Field Spell by its row's word b: kind 13 (Spell), subtype 2 (Field), as the game's own Field Spells have it."""
    return (card['b'] >> 3) & 0x1F == 13 and (card['b'] >> 16) & 7 == 2


def build_fields(added):
    lines = ['cid\tpasscode']
    for card in sorted((c for c in added if field_spell(c)), key=lambda c: c['cid']):
        lines.append('%d\t%d' % (card['cid'], card['passcode']))
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--check', action='store_true', help='compare the committed list with a fresh build')
    args = parser.parse_args()
    with open(os.path.join(ENGINE, 'anime-cards.json'), encoding='utf-8') as f:
        anime = json.load(f)
    added = []
    if os.path.exists(os.path.join(ENGINE, 'added-cards.json')):
        with open(os.path.join(ENGINE, 'added-cards.json'), encoding='utf-8') as f:
            added = json.load(f)['cards']
    outputs = [(OUT, build(anime, added)), (FIELDS, build_fields(added))]
    if args.check:
        for path, text in outputs:
            with open(path, encoding='utf-8') as f:
                if f.read() != text:
                    sys.exit('engine/anime/%s is out of date: run tools/engine/anime-pictures.py' % os.path.basename(path))
        return
    for path, text in outputs:
        with open(path, 'w', encoding='utf-8') as f:
            f.write(text)


if __name__ == '__main__':
    main()
