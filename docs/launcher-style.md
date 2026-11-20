# Launcher style guide

The look of the beta launcher ([#21](https://github.com/usersina/yo3-revival/issues/21)): a 2009–2011 online-game
launcher built from the game's own visual language. The skin kit in `crates/yo3-skin` implements it on iced; this page
is the reference for anyone adding a screen. Try it with:

```bash
cargo run --release -p yo3-skin --example main_screen   # add -- --system-frame for the window manager's frame
cargo run --release -p yo3-skin --example gallery       # every component and its states, for checking a change
```

The spike was checked on 2026-11-15 on X11 (headless), Wayland (a nested KWin), Wine, and Windows 11 in the VM: hover,
tabs, typing with Tab and Enter, sign-in errors, dialogs, dragging, minimising and closing the borderless window.

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
  While the game runs the big button becomes what you can do then: **CLOSE GAME**, or **CANCEL** for a start that hangs.
- **Each button's role shows** ([#50](https://github.com/usersina/yo3-revival/issues/50)): navy for a form's or a
  dialog's main action, light silver for a way back or out (Back, Cancel), red for one that ends or removes something
  (Stop hosting, Sign out, a reset). A button that can't be pressed yet is flat and faded, never like a live one.
- **Dialogs** carry the help **?** and the **×** in their title strip; the footer holds only the actions, the main one
  rightmost and a quiet one left of it.
- **One message per state.** What the server's row, the big button or a field already says isn't repeated in another
  line. Notices about both tabs go behind the title bar's bell, not in a banner over the page.
- **Forms check as you type.** Each rule is a line that turns green when met and red when broken; the form's button
  works once all are met. An address is looked up before Add.
- **Short words.** A dialog says what it does in a sentence or two.
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
| Navy button | `pill(label, message, Kind::Primary, width, height)`: a form's or dialog's main action | normal, hover (lighter, cyan glow), pressed (darker, label down 1 px), disabled (flat, navy faded into the panel) |
| Quiet button | `pill(.., Kind::Secondary, ..)`: Back, Cancel, tools in Settings | light silver face, navy label; same states |
| Red button | `pill(.., Kind::Danger, ..)`: Stop hosting, Sign out, a reset | red face, white label; same states |
| Big action | `pill(.., Kind::Play, ..)`, about 70 px high; any kind 50 px or higher uses the display font | as above, with a soft glow at rest; disabled is steel, its label saying what is happening |
| Tab | `pill(.., Kind::Tab { active }, ..)`, a parallelogram | active (silver rim, navy, cyan underline), inactive (dark), hover (lighter, white label) |
| Window control | `pill("", .., Kind::Window(Glyph::..), 34, 28)` | bell, settings, minimise, close (red under the pointer), help; the symbol is drawn, not a font glyph |
| Count | `badge(n)` on a control's corner | red with a white number, 9+ above nine |
| Frame | `panel(Some(title), content)` | silver rim with four bolts, navy title strip with stripes, light-grey body |
| Title bar, footer | `title_strip(content)`, `footer_strip(content)` | dark bars with a silver line toward the content |
| Settings rows | `setting_group(label, rows)`, `setting(title, line, control, below)`, `switch` (a `toggler` style) | a labelled card of rows: name and one line left, control right; on/off settings are switches |
| Toast | `toast(content, Tone)` | a card in the bottom right corner with a green, red or blue stripe; never moves the page |
| Text field | `text_input(..).style(field)` | white well, blue ring when focused; `.secure(true)` for passwords |
| Checkbox | `checkbox(..).style(tick)` | navy when ticked |
| Drop-down | `pick_list(..).style(choice).menu_style(choice_menu)` | white well; the open list highlights in navy |
| Link | `link(label, message, on_dark)`; `danger_link` in red; `icon_link(icon, ..)` led by an icon; `support_link` the footer's red-heart chip; `dock_button(icon, ..)` the footer's Help and Settings | blue on panels, pale on bars; brighter on hover |
| Icon | `icon(Icon::.., size, colour)`: line icons on a 24 px grid, tinted | copy, save, help, flag, heart, more, check, cross, pending, plus, trash, server, user, open, bell, link, refresh |
| Icon button | `icon_button(icon, message, tip, on_dark)` | 30 px square, a tip under the pointer, a soft face on hover; faded when disabled |
| Menu | `menu(items, width)` of `Item { icon, label, on_press, danger }` | a light card with a shadow; rows highlight on hover; red for a removal |
| Checklist line | `rule(Rule::.., words)` | grey ring (not typed yet), green tick (met), red cross (broken) |
| Progress bar | `progress_bar(Bar::.., label, height)`, full width; `Bar::of(done, total)` | a silver rim, a dark well and a navy fill with a cyan edge that glides to each value, the label ("12 of 418") on it; `Bar::Starting` a band that travels to and fro (length unknown), `Bar::Done` green, `Bar::Failed(fraction)` red; it only asks for frames while it moves |
| Dialog | `dialog(title, help, close, width, height, content)` inside `modal(screen, dialog, on_blur)` | a frame with **?** and **×** in its strip; the modal darkens the screen; a click outside or Escape closes it |
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
