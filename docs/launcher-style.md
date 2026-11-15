# Launcher style guide

The look of the beta launcher ([#21](https://github.com/usersina/yo3-revival/issues/21)): a 2009–2011 online-game
launcher built from the game's own visual language. The skin kit in `crates/yo3-skin` implements it on iced; this page
is the reference for anyone adding a screen. Try it with:

```bash
cargo run --release -p yo3-skin --example main_screen   # add -- --system-frame for the window manager's frame
```

**Contents**

- [Launcher style guide](#launcher-style-guide)
  - [Principles](#principles)
  - [Colours](#colours)
  - [Fonts](#fonts)
  - [Components](#components)
  - [Layout](#layout)
  - [Words](#words)
  - [Rendering rules](#rendering-rules)

## Principles

- **The game's menus are the model:** brushed-silver chrome frames with bolts, navy pill buttons with silver rims,
  light-grey panel bodies with dark text, and a glowing deep-blue circuit backdrop.
- **All art is ours.** Every shape is drawn in code or in our own SVG. No Konami logo, character, card art, sound or
  screen capture is shipped. The colours below were measured from the game's Select Player screen, so they match it
  without copying anything from it.
- **One big action per screen.** PLAY or LOG IN is the bright blue button; everything else is navy, a link or a tab.
- **Dark outside, light inside.** Bars, tabs and the backdrop are dark with light text; panels are light with dark
  text, as in the game.

## Colours

The constants are in `yo3_skin::color`. Button faces are glossy: a light top, a hard split at the middle and a darker
bottom that lightens again at the edge.

| Use | Constants | Value | From the game |
| --- | --- | --- | --- |
| Navy button face, top to bottom | `NAVY_TOP`, `NAVY`, `NAVY_LOW`, `NAVY_BASE` | #4a63b5, #26306a, #1b2357, #2b3779 | #354c94 to #22295d |
| The big action (PLAY, LOG IN) | `PLAY_TOP`, `PLAY`, `PLAY_LOW`, `PLAY_BASE` | #4c93ff, #2160d2, #17449f, #2a63c9 | brighter, ours |
| Silver rims and frames | `CHROME_LIGHT`, `CHROME`, `CHROME_DARK`, `CHROME_EDGE` | #f4f6f8, #aab3bd, #7b848f, #4c545e | rim #a9b3bd |
| Panel body, top to bottom | `PANEL_TOP`, `PANEL`, `PANEL_LOW` | #f1f1f1, #d4d4d4, #bebebe | #eeeeee to #bebebe |
| List rows, alternating | `ROW`, `ROW_ALT` | #e6e6e6, #d9d9d9 | row #bcbcbc |
| Text on panels | `INK`, `INK_SOFT` | #1b2130, #4d5566 | |
| Frame title strip | `STRIP_TOP`, `STRIP`, `STRIP_LOW` | #11507f, #063058, #021a33 | #052d52 |
| Glow, focus line under strips and tabs | `GLOW` | #43b6ff | the cyan header stripes |
| Disabled button | `STEEL_TOP`, `STEEL`, `STEEL_TEXT` | #5d5f63, #3a3b3e, #8c8e93 | #36373a |
| Title bar and footer | `BAR_TOP`, `BAR`, `BAR_LOW` | #22345c, #0d1834, #070e22 | |
| Text on dark | `ON_DARK`, `ON_DARK_SOFT` | #eef3fb, #9fb3d4 | |
| States | `GOOD`, `WARN`, `BAD` | #2f9e4f, #f0b43c, #c0392b | |
| Links on panels | `LINK`, `LINK_HOVER` | #1f4fb4, #3a78f0 | |
| Close control under the pointer | `CLOSE` | #c4383a | |

## Fonts

Barlow and Barlow Condensed, both under the SIL Open Font License (`crates/yo3-skin/assets/fonts/OFL.txt`).

| Constant | Face | Use |
| --- | --- | --- |
| `BODY` | Barlow Regular | body text, the default font |
| `BODY_BOLD` | Barlow SemiBold | labels, dates in lists, links, navy buttons |
| `DISPLAY` | Barlow Condensed Bold | headings inside panels |
| `DISPLAY_ITALIC` | Barlow Condensed ExtraBold Italic | the logo, tabs, frame titles, the big action; like the game's location banners |

Sizes: 15 px body, 14 px captions and footer, 19 px frame titles, about half the button's height on buttons.

## Components

| Component | Function | States |
| --- | --- | --- |
| Navy button | `pill(label, message, Kind::Primary, width, height)` | normal, hover (lighter, cyan glow), pressed (darker, label down 1 px), disabled (steel, grey label) |
| Big action | `pill(.., Kind::Play, ..)`, about 70 px high | as above, with a soft glow at rest |
| Tab | `pill(.., Kind::Tab { active }, ..)`, a parallelogram | active (silver rim, navy, cyan underline), inactive (dark), hover (lighter, white label) |
| Window control | `pill("", .., Kind::Window(Glyph::..), 34, 28)` | settings, minimise, close (red under the pointer); the symbol is drawn, not a font glyph |
| Frame | `panel(Some(title), content)` | silver rim with four bolts, navy title strip with stripes, light-grey body |
| Title bar, footer | `title_strip(content)`, `footer_strip(content)` | dark bars with a silver line toward the content |
| Text field | `text_input(..).style(field)` | white well, blue ring when focused; `.secure(true)` for passwords |
| Checkbox | `checkbox(..).style(tick)` | navy when ticked |
| Drop-down | `pick_list(..).style(choice).menu_style(choice_menu)` | white well; the open list highlights in navy |
| Link | `link(label, message, on_dark)` | blue on panels, pale on bars; brighter on hover |
| Dialog | `modal(screen, dialog, on_blur)` | darkens the screen; a click outside or Escape closes it |
| Backdrop | `backdrop()` | the circuit board, cropped to fill; always the bottom layer |
| Logo | `logo(height)` | the diamond mark and YO3 REVIVAL |

## Layout

- **Window:** fixed 1024×640 logical pixels, scaled by the display's scale factor. Borderless with our own title bar;
  `--system-frame` (a setting later) falls back to the window manager's frame.
- **Title bar** 50 px: logo left, the two tabs centred, window controls right. Dragging the bar moves the window.
- **Footer** 34 px: version and update state left, Help and Discord right.
- **Body:** 16 px between panels, 18 px from the window's sides. The side panel is 372 px wide.
- **Frames:** 6 px rim, 30 px title strip, 12 px below the strip, 14 px side padding.
- **Dialogs:** 470 px wide, centred.

## Words

- **One plain sentence and one action** for every state and error (Retry, Fix, Report). Paths, error chains, ports and
  flag names only behind **Details**.
- **The game's own words** where it has them: User ID, Password, Save User ID, Information.
- **Capitals** only on the big action, tabs and frame titles, which the font draws in capitals anyway.

## Rendering rules

- **Every canvas drawing is cached** (`Memo` in the kit): it redraws only when what it shows changes.
- **Every frame is a full repaint.** `backdrop()` adds one transparent shape that changes every frame, so tiny-skia
  repaints the whole window. Its partial repaint (iced 0.14) sometimes left a newly shown text unpainted until the next
  frame. With the drawings cached, a full repaint costs about 20 ms on the headless test display and nothing when idle.
- **Keyboard:** Tab and Shift+Tab move between fields, Enter submits the form, Escape closes a dialog. Buttons are not
  focusable in iced 0.14.
