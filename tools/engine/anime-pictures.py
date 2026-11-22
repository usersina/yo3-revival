#!/usr/bin/env python3
"""Builds engine/anime/pictures.tsv, the list of the anime card pictures Desktop can fetch (issue #60): per card its Konami
id, the anime passcode to ask ProjectIgnis' picture server for (0 when the card has none) and the official passcode whose
normal picture stands in when the server has no anime one. The list holds no image and is compiled into Desktop
(crates/yo3-launcher/src/art/get.rs). The source is engine/anime-cards.json, so the one reviewed map decides which cards
have a picture, and engine/added-cards.json: the cards the game never had (#75) have no picture of their own, so each is
asked by its official passcode (anime 0); `--check` compares the committed list with a fresh build."""
import argparse
import json
import os
import sys

ENGINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'engine')
OUT = os.path.join(ENGINE, 'anime', 'pictures.tsv')


def build(data, added=()):
    lines = ['cid\tanime\tofficial']
    cards = list(data['cards']) + [{'cid': c['cid'], 'anime': 0, 'official': c['passcode']} for c in added]
    for card in sorted(cards, key=lambda c: c['cid']):
        anime, official = int(card.get('anime') or 0), int(card.get('official') or 0)
        if not anime and not official:
            sys.exit('card %s has no passcode to fetch a picture by' % card['cid'])
        lines.append('%d\t%d\t%d' % (card['cid'], anime, official))
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='compare the committed list with a fresh build')
    args = parser.parse_args()
    with open(os.path.join(ENGINE, 'anime-cards.json'), encoding='utf-8') as f:
        anime = json.load(f)
    added = []
    if os.path.exists(os.path.join(ENGINE, 'added-cards.json')):
        with open(os.path.join(ENGINE, 'added-cards.json'), encoding='utf-8') as f:
            added = json.load(f)['cards']
    text = build(anime, added)
    if args.check:
        with open(OUT, encoding='utf-8') as f:
            sys.exit(0 if f.read() == text else 'engine/anime/pictures.tsv is out of date: run tools/engine/anime-pictures.py')
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)


if __name__ == '__main__':
    main()
