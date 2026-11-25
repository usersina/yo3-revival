# The connection DLL's pictures

Pictures the connection DLL draws in the game, made for the project. They are compiled into the DLL (nothing ships
beside it), so a change here needs a new Desktop release.

## `lp-infinity.png`: the LP "∞"

Shown in the duel's LP disc, in place of the five LP digits, while a player's LP is above 99,999 (Mystik Wok on an
infinite ATK). The final geometric sign is `lp-infinity.png`, rendered from the editable `lp-infinity.svg`.
It is original project artwork, permitted for use, modification and distribution under the repository's MIT license.
The 160 × 64 RGBA canvas contains a 120 × 48 white sign with 8 px strokes and a sharp 75% black shadow offset 4 px
right and down. Its visible bounds, including the shadow, are centred. It was inspected at 40 × 16 pixels on slate grey,
and in the game under Wine (2026-11-25, a duel at 200,000 LP: centred in both LP discs, crisp beside the "LP" label).

To render the vector source again (CairoSVG 2.9.1):

```sh
uv run --with cairosvg==2.9.1 python -c 'import cairosvg; cairosvg.svg2png(url="tools/mod/art/lp-infinity.svg", write_to="tools/mod/art/lp-infinity.png")'
python3 tools/mod/make-lp-infinity.py
```

### The request for the artist

> One picture: an **infinity sign (∞)** that replaces the life-point number inside the round LP badge of a duel screen.
> It must look like it belongs to the same font as the digits it replaces.
>
> - **File:** PNG, RGBA (transparent background), **160 × 64 pixels**. Nothing else in the picture: no badge, no
>   background, no "LP" label.
> - **Scale:** the picture is drawn at **4× the size it appears on screen** (the game shows it at 25%: 40 × 16 screen
>   pixels). Design it at 4× and check it reduced to 40 × 16: it must stay crisp and readable at that size.
> - **Size of the sign:** centred on the canvas; about **48 px tall** (12 on screen, the digits' height) and
>   **112–128 px wide** (28–32 on screen, about two and a half digits). The rest of the canvas stays transparent; the
>   game centres the whole canvas on the digits' place.
> - **Style:** the LP digits' own (reference: `logs/lp-infinity-reference.png` on the project machine, the digits next
>   to the first, generated ∞, 6× enlarged):
>   - **pure white** (#FFFFFF) fill, flat, no gradient;
>   - **blocky, geometric strokes** with squared, slightly chamfered corners, like a digital/techno display font (the
>     digits' 0 and 8 are rounded rectangles, not ovals): the ∞ as two squared loops joined at a crossing;
>   - stroke **8 px wide** (2 screen pixels), the same weight as the digits;
>   - a **drop shadow**: black at about 75% opacity, offset **4 px right and 4 px down** (1 screen pixel), no blur;
>   - no outline, glow or colour.
> - **Background it sits on:** a dark slate-grey disc (about #4A5563, a lighter rim) under a white "LP" label; the
>   sign must read well on it.
> - **One version only:** the game's red "low LP" colour never applies to an LP this high.
> - **Deliver:** the 160 × 64 PNG, plus the source file (layers or vector), and a note that it may be used in the
>   project.

### Replacing the picture

1. Put the artist's PNG at `tools/mod/art/lp-infinity.png` (160 × 64, RGBA; another size works if it is drawn at 4×,
   up to 280 × 96).
2. `python3 tools/mod/make-lp-infinity.py` writes `tools/mod/lp-infinity-png.h` from it (`--check` tells when they
   differ; `tools/mod/test_lp_infinity.py` checks it in `task check`).
3. `task build:mod`, then a live look: a scratch server with `--duel-lp 200000` shows ∞ in both discs from the duel's
   start (docs/client-internals.md, "Numbers in the duel: LP and ATK").
