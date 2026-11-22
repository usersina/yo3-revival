#!/usr/bin/env python3
"""Makes the picture an added card shows when no picture of it was downloaded (#75): crates/yo3-launcher/assets/no-picture.png,
a 200x290 card face the launcher ships and writes (with its 50x72 thumbnail) for such a card.

    tools/engine/no-picture.py IMAGE     the image, cropped to a card's shape from its centre and resized to 200x290
    tools/engine/no-picture.py --drawn   a plain drawn stand-in (a card frame with a question mark)

Needs Pillow. The image must be the project's own (never Konami's artwork).
"""
import argparse
import os
import sys

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, '..', '..', 'crates', 'yo3-launcher', 'assets', 'no-picture.png'))
FACE = (200, 290)


def fitted(image):
    """`image` cropped from its centre to a card's shape, then resized to the face's size."""
    image = image.convert('RGBA')
    w, h = image.size
    want = FACE[0] / FACE[1]
    if w / h > want:
        cw = round(h * want)
        image = image.crop(((w - cw) // 2, 0, (w - cw) // 2 + cw, h))
    else:
        ch = round(w / want)
        image = image.crop((0, (h - ch) // 2, w, (h - ch) // 2 + ch))
    return image.resize(FACE, Image.LANCZOS)


def font(size):
    for path in ('/usr/share/fonts/TTF/DejaVuSans-Bold.ttf', '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def drawn():
    """A plain stand-in: a dark card with a frame, an empty art box and a question mark."""
    image = Image.new('RGBA', FACE, (28, 34, 48, 255))
    d = ImageDraw.Draw(image)
    for y in range(FACE[1]):
        shade = 34 + y * 18 // FACE[1]
        d.line([(0, y), (FACE[0], y)], fill=(shade - 6, shade, shade + 22, 255))
    d.rectangle([3, 3, FACE[0] - 4, FACE[1] - 4], outline=(150, 160, 185, 255), width=3)
    d.rectangle([14, 14, FACE[0] - 15, 36], outline=(110, 120, 145, 255), width=2)
    d.rectangle([22, 50, FACE[0] - 23, 200], fill=(18, 22, 32, 255), outline=(110, 120, 145, 255), width=2)
    d.text((FACE[0] // 2, 126), '?', font=font(96), fill=(170, 180, 205, 255), anchor='mm')
    d.text((FACE[0] // 2, 236), 'No picture yet', font=font(15), fill=(170, 180, 205, 255), anchor='mm')
    return image


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('image', nargs='?', help='the source image')
    p.add_argument('--drawn', action='store_true', help='draw the stand-in instead')
    p.add_argument('--out', default=OUT, help=argparse.SUPPRESS)
    args = p.parse_args(argv)
    if bool(args.image) == args.drawn:
        p.error('give an IMAGE or --drawn')
    face = drawn() if args.drawn else fitted(Image.open(args.image))
    face.save(args.out, optimize=True)
    print(f'wrote {args.out} ({os.path.getsize(args.out)} bytes)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
