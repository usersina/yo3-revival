# Custom NPCs

Characters the game never had as Kaiba Corporation opponents (first **Dartz**, 2026-11-26), with their own decks for
both card modes, their own lines, and a body: a game NPC's, or one of our own figures (the clothes-store mannequin).
This guide covers adding one, the art it can have, and authoring a 3D body; the model format itself is in
[client internals](client-internals.md#3d-models-mdl).

**Contents**

- [How it works](#how-it-works)
- [Adding a character](#adding-a-character)
- [Lines](#lines)
- [Art](#art)
- [Bodies](#bodies)
- [Authoring a 3D body](#authoring-a-3d-body)
- [Checking it](#checking-it)

## How it works

| Piece | Where | What |
| --- | --- | --- |
| The definition | `engine/added/npcs.json` | name, help text, tab, body, three levels' decks (printed and anime), lines |
| Carried to everyone | `engine/added-cards.json` (`tools/engine/added-cards.py` copies `npcs.json` in) | the server reads it at start, and serves it to each launcher with the added cards |
| The roster | `yo3_server::add_custom_npcs` (`lib.rs`) | the levels' ids join the `0x2711` roster on their tab; their decks (each card by its English name) and bot style (`npc.rs` `CHARACTERS`) |
| The decks | `World::npc_deck` | the anime deck while the server has `anime-cards` on, else the printed one |
| The game's files | `yo3_data::added::text_files`, `npc_files` (the launcher writes them with the added cards) | the UI texts (name, help, deck titles, lines), the body (`model/np025/`), the portrait (`npc/npcsel025.png`) and the faces (`face/np025f0-4.png`) |
| The duel's cut-ins | the connection DLL | the game's cut-in loads a body only for slots below 25 (`0x4E3C1D`) and times its sounds from a 25-row table (`0x78BBF8`): raised to 30, the custom slots' rows their donor's (`model/npNNN/donor.txt`) |
| Serving them | the connection DLL (`tools/mod/card-profile.h`, `added-cards-logic.h` `yo3_added_npc_path`) | redirects the game's opens of `data/npc/npcsel025.png` and `data/face/np025f*.png`; gives the game's 25-entry model table room for 30 (`npc_models_install`, the read at `0x4E430F`) and points slots 25-29 at the launcher folder's bodies by their full path (the resource manager looks in the archives before opening anything, so a relative path never reaches the file hook) |

Kaiba Corporation's NPCs are the slots `k` = 0, 1, 2...: slot `k`'s three levels are NPC ids `120 + 3k` to `122 + 3k`.
The game has 25 (0-24, Yami Yugi to Kalin Kessler); custom ones take **slots 25-29** (the duel cut-ins' table at
`0x788C40` has 30 entries; the players' NPC experience reaches id 197, slot 25's last level, so slots 26-29 also need
`npcbin` widened before their levels count experience).

## Adding a character

Add an entry to `engine/added/npcs.json`:

```json
{"npcs": [{
  "slot": 25,
  "name": "Dartz",
  "help": "The ancient king of Atlantis …",
  "series": 0,
  "body": 5,
  "figure": "mannequin",
  "levels": [
    {"title": "The Seal of Orichalcos", "deck": ["Giant Soldier of Stone", "…40 names"], "anime_deck": ["…40 names"]},
    {"title": "The Soldiers of Atlantis", "deck": ["…"], "anime_deck": ["…"]},
    {"title": "The Great Leviathan Awakens", "deck": ["…"], "anime_deck": ["…"]}
  ],
  "lines": {"0": "So you have come.\nThen let the Orichalcos judge you.", "1": "My turn.\nI draw.", "…": "…"}
}]}
```

| Field | Meaning |
| --- | --- |
| `slot` | 25-29, one per character |
| `name`, `help` | Kaiba Corporation's list and info panel (text ids `0x5208 + 2·slot` and `+1`). Keep the help in the character's world (no "anime cards") |
| `series` | the tab: 0 Yu-Gi-Oh!, 1 GX, 2 5D's |
| `body` | the game NPC (0-24) whose skeleton and 53 animations it moves with |
| `figure` | `"mannequin"` for our clothes-store mannequin on that skeleton; absent: the game NPC's own body and skins |
| `levels` | three, easiest first: a deck `title` (text `0x5FB4 + 2·id`), the printed `deck` (and optional `extra`), and an `anime_deck` (and `anime_extra`) played while `anime-cards` is on; 40-60 cards, each by its exact English name as the server knows it (the game's cards and the added ones) |
| `lines` | what they say, by line number (below) |

Then run `python3 tools/engine/added-cards.py` (writes `added-cards.json`), add the character's bot style to
`crates/yo3-server/src/npc.rs` `CHARACTERS` (aggression, caution, whether it sees your hand) and its chat lines to
`crates/yo3-server/src/npc_lines.txt` (what the server says for it in the duel's chat window: greet, turn, ace, direct,
hurt, win, lose; `banter.rs`'s test requires them for every character), and check it ([below](#checking-it)). A name with no card is logged at the server's start
("custom NPC …: no card named …") and left out of the deck: `tests/custom_npc.rs` checks every deck has its 40.

## Lines

Each NPC has a block of 300 UI texts at `62000 + 300·slot` (Pegasus's "My turn now!" is 63501). The game's own NPCs use
three sections of a hundred, one per level (0-99, 100-199, 200-260); within a section (what each line is for, read from
the game's own NPCs' texts, e.g. Seto Kaiba's at 62300):

| Lines | When |
| --- | --- |
| 0 | the duel's opening (`@@s` is the player's name) |
| 1-12 | the duel's moments: 1 the draw, 2 a summon, 3-5 attacks, 6 an effect, 7 a Spell, 9 a trap sprung, 10 a set monster, 12 the end of the turn |
| 13-21 | reactions: 13-16 taking damage, 17 the ace monster's call, 18-19 surprise, 20 taunting, 21 not done yet |
| 24 | the ace monster (`@@m` its name) |
| 25-28 | Kaiba Corporation's talk the first time; 30-31 coming back; 35 after the player won; 36 a refusal ("Who the hell are you?") |
| 37-38 | winning; 42-48 the end of a won duel; 52-54 a draw; 62-68 losing; 72-79 the prize; 82-86 asking the player's name |

A section's empty lines say nothing. The talk needs at least its lines (25-28): with an empty block the talk opens and
closes on nothing. Dartz's are in `npcs.json` as a model.

## Art

Without art of its own, a character shows its body's pictures: a game NPC's portrait and faces, or the mannequin's
(its bust, generated by `yo3_data::doll`). Art the owner draws replaces them, under the same names:

| File | Size | What |
| --- | --- | --- |
| `npcsel0SS.png` (SS the slot) | 512×512 PNG with transparency | Kaiba Corporation's portrait: a bust, the character in the lower two thirds, cut at the chest |
| `np0SSf0.png`-`f4.png` | 128×128 PNG | duel faces: 0 neutral, 1 a confident smile, 2 angry, 3 shocked or hurt (taking damage), 4 eyes closed (smug or laughing); tight head-and-shoulders crops on a gradient, the same framing in all five |
| the body's skins | as the body's (a game body's `np0050.png` is a 256×256 atlas) | only for a game body (no `figure`): repainted in place, each region kept where it is |

The faces show at the janken and beside the LP (loaded at the duel's start; without them the game shows Yami Yugi's);
the portrait on Kaiba Corporation's info panel.

## Bodies

A body is a `.mdl` model ([format](client-internals.md#3d-models-mdl)) the talk scene, the picker and the duel's
cut-ins show and animate. The game's 25 bodies are Konami's, so a body of ours is **built on the player's machine** by
the launcher from their own game files: our shapes (committed code or files) on a game body's skeleton and animations.

- **A game body** (`"body": 5`, no `figure`): the launcher copies `np005.mdl` into the character's own folder
  (`model/np025/np025.mdl`) with the skins it names (`np0050.png`...), which it loads from its own folder by name.
- **The mannequin** (`"figure": "mannequin"`): `yo3_data::doll::mannequin` keeps the body's skeleton, its 53 animations,
  the duel disk and cards, drops its face, hair, eyes and mouth, and replaces its body mesh with ours: an egg head, a
  sculpted chest and shoulder line, waist and hips, tapered limbs, ball joints at the shoulders, elbows, hips and knees,
  paddle hands and feet, each part rigidly on one joint. Its colour is painted: each corner's UV is its smooth normal
  seen from the front, on a lit ivory sphere (`mannequin.png`, a "matcap"), as the game's bodies carry their shading in
  their skins.

## Authoring a 3D body

A body is the game's `.mdl` ([format](client-internals.md#3d-models-mdl)). Ours keep a game body's skeleton and
animations (the "donor") and bring their own meshes, because the 53 animations the game plays by index are the hard
part and the donor's are on every player's machine. `crates/yo3-data/src/doll.rs` is the worked example (the
mannequin); `tools/re/yo3mdl.py` reads and writes the same format in Python, to look at a body.

**Required**

- **A weighted joint named `under`** (the hips). The game classifies a body by which of `under`, `disc_root`, `neck`,
  `neck_face`, `hair_root` carry weights; with none, it drops every animation. Keep the donor's joint names for
  everything you weight: names only have to match between frames, weights and animation tracks.
- **The donor's animation sets, in its order.** The game plays them by index: 46 (`posture_00`, looping) in the picker
  and the talk, 0-52 in the duel's cut-ins (draw, summon, attack, effects, set, damage, the turn call). A body with
  fewer leaves the pose unchanged where a set is missing (untested in a duel: a cut-in waits for its animation's end).
- **The bind pose**: a T-pose about 172 units tall, feet at y 0, the face towards −z, +x the character's left, scale 1.
  Model each part where the donor's joint is; the joint positions of np005 (from its inverse bind matrices):

  | Joint | Position (x, y, z) |
  | --- | --- |
  | `neck_face` | 0, 153.1, −0.4 |
  | `neck` | 0, 146.4, 0.9 |
  | `upper_02` (chest) | 0, 126.3, −4.7 |
  | `upper_01`, `under` | 0, 113.8, −4.3 |
  | `L_arm_01` (shoulder) | 12.4, 141.3, 2.1 |
  | `L_arm_02` (elbow) | 37.5, 141.3, 2.1 |
  | `L_hand` | 64.5, 141.3, 2.1 |
  | `L_leg_01` (hip) | 7.4, 100.3, −3.7 |
  | `L_leg_03` (knee) | 7.4, 56.5, −2.2 |
  | `L_foot_01` (ankle), `L_foot_02` (toes) | 7.4, 12.4, 3.5; 7.4, 3.1, −7.9 |

  The right side's `R_` joints mirror x. Another donor has its own positions: read them from its weights' matrices.
- **Faces**: each face owns its corners (3-7, unshared), written **clockwise seen from outside** (the loader reverses
  them; a closed mesh's faces in file order have a negative signed volume). u16 corners, faces and weights per joint;
  at most 255 ids per run.
- **UVs** per corner, v from the top.
- **Weights**: per joint, its corners and a weight each, with the donor's inverse bind matrix for that joint; a corner's
  weights sum to 1 (a rigid part: 1 on one joint).
- **A material the mesh names, defined before the frames** (keep the donor's `body_Material`, its texture
  `.\<name>.png`). Lighting is off for NPCs: **paint the shading into the texture**. The mannequin's trick: each
  corner's UV is its smooth normal seen from the front, on a picture of a lit sphere (a matcap). PNG, a power of two up
  to 256.
- **Replace** the donor's `bodyShape` with the new mesh, drop the meshes the figure doesn't have (`faceShape`,
  `hairShape`, `eyesShape`, `ha_upperShape`, `ha_lowerShape`, `kuchiShape`, `millenniumeyeShape`), and keep the rest:
  the duel disk (on `disc_root`), the hand of cards (`opt01`, `opt07` on `R_hand`), the single card, the janken hands
  (shown and hidden by the animations' visibility keys).

**Optional**: the `01` frame rate (30 by default), `0C` identity frames, `lambert1`, normals (never used), camera tracks
(the cut-ins' camera moves with the donor's; without, a fixed view), extra meshes.

**A new figure in the launcher**: add it to `doll.rs` (its parts, its picture) and a name for `figure` in
`yo3_data::added::npc_files`; `tests/models.rs` builds it on the donor and checks its 53 animations survive and every
corner is weighted. **A model from Blender** (a later step): export glTF and convert its meshes, UVs and per-joint
weights into the mesh above, on the donor's joint names and inverse bind matrices.

**Still to settle in the game**: a body with fewer than 53 animations in a duel (does the cut-in end?), textures larger
than 256 or not a power of two, a mesh with no visibility track (shown by default?).

## Checking it

- `cargo nextest run -p yo3-server --test custom_npc` (roster, tab, both decks complete), `-p yo3-data --test models`
  (every game model round-trips; the mannequin builds and keeps its 53 animations).
- Live, with a scratch server (`task dev:server`, `scratch-server.py admin set anime-cards on|off`) and the headless
  client: Kaiba Corporation (765, 275), click through the greeting to the list, select the character (its portrait and
  body on the right, its help on the left), select again for the talk (its lines, its body), then Yes, Select Deck's
  OK and the janken: its face, its lines in the duel. Switching `anime-cards` needs a new Play (a game showing the other
  card versions is refused).
