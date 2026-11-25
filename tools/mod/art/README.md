# The connection DLL's pictures

Pictures the connection DLL draws in the game, made for the project: original project artwork, permitted for use,
modification and distribution under the repository's MIT license. Each PNG is rendered from the editable SVG beside it
and compiled into the DLL (nothing ships beside it), so a change here needs a new Desktop release. All are drawn at
**four times** the size they appear on screen (the DLL shows them at 25%, centred on the digits they replace), on a
transparent background (PNG RGBA).

| Picture | Where | Canvas | Style |
| --- | --- | --- | --- |
| `lp-infinity.png` | the duel's LP disc, in place of the five LP digits, while a player's LP is ∞ | 160 × 64 | a bold white sign: 12 px strokes along a 108 × 32 centre line, a 4 px near-black outline, a sharp 75% black shadow 4 px right and down, as heavy as the "LP" label above it |
| `damage-infinity-loss.png` | the damage pop-up of an ∞ loss ("-∞") | 354 × 136 | the pop-up's loss digits (duel_font_e_anim_01): red #FD134F, 28 px strokes with an 8 px black outline, a minus as the game's |
| `damage-infinity-gain.png` | the damage pop-up of an ∞ gain ("∞", no plus: the game's gains have none) | 244 × 136 | the pop-up's gain digits (duel_font_e_anim_00): cyan #01E9FE, as the loss one |

Checked in the game under Wine (2026-11-25). The LP sign: centred in both LP discs at 200,000 LP; a first version with
8 px strokes and no outline looked thin there. The pop-ups: "-∞" beside the monster an ∞ ATK destroyed while the
opponent's LP counted down to 0, and "∞" over Mystik Wok on the anime Geh while the LP counted up to ∞, the game's
digits hidden.

To render them again (librsvg's `rsvg-convert`; CairoSVG or Inkscape give the same pictures), then the header:

```sh
rsvg-convert -w 160 -h 64 tools/mod/art/lp-infinity.svg -o tools/mod/art/lp-infinity.png
rsvg-convert tools/mod/art/damage-infinity-loss.svg -o tools/mod/art/damage-infinity-loss.png
rsvg-convert tools/mod/art/damage-infinity-gain.svg -o tools/mod/art/damage-infinity-gain.png
python3 tools/mod/make-infinity-pictures.py
```

`make-infinity-pictures.py` writes `tools/mod/infinity-pictures.h` from the three PNGs (`--check` tells when they
differ; `tools/mod/test_infinity_pictures.py` checks it, and that each picture is RGBA at 4× with its SVG, in `task
check`). Then `task build:mod` and a live look: a scratch server with `--duel-lp 200000` shows ∞ in both LP discs from
the duel's start, and `--duel-field 0:mzone:12671,0:hand:5978` with `anime-cards` on (the anime Geh and Mystik Wok)
shows the gain pop-up, Geh's attack the loss one (docs/client-internals.md, "Numbers in the duel: LP and ATK").

## The request for an artist (the LP sign)

The LP sign was first a placeholder, with this request for an artist; the pop-up pictures follow the same rules in the
pop-up's own font.

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
> - **Style:** the LP digits' own: **pure white** (#FFFFFF), **blocky, geometric strokes** with squared, slightly
>   chamfered corners (the digits' 0 and 8 are rounded rectangles, not ovals), the ∞ as two squared loops joined at a
>   crossing, a **drop shadow** in black at about 75% opacity, offset **4 px right and 4 px down**, no blur.
> - **Background it sits on:** a dark slate-grey disc (about #4A5563, a lighter rim) under a white "LP" label.
> - **One version only:** the game's red "low LP" colour never applies to an LP this high.
> - **Deliver:** the PNG, plus the source file (layers or vector), and a note that it may be used in the project.
