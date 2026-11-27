# Custom NPCs

Characters the game never had as Kaiba Corporation opponents (first **Dartz**, 2026-11-26), with their own decks for
both card modes, their own lines, and a body: a game NPC's, or one of our own figures (the clothes-store mannequin).
This guide covers adding one, the art it can have, and authoring a 3D body; the model format itself is in
[client internals](client-internals.md#3d-models-mdl).

**Contents**

- [How it works](#how-it-works)
- [How many](#how-many)
- [The characters](#the-characters)
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
| The duel's cut-ins | the connection DLL | the game's cut-in loads a body only for slots below 25 (`0x4E3C1D`), times its sounds from a 25-row table (`0x78BBF8`) and picks its background swf from a 26-entry table (`0x788C40`): raised to 100, the custom slots' timing rows their donor's and their background their own `cutin_bg` or the donor's (`model/npNNN/donor.txt`) |
| Serving them | the connection DLL (`tools/mod/card-profile.h`, `added-cards-logic.h` `yo3_added_npc_path`) | redirects the game's opens of `data/npc/npcsel025.png` and `data/face/np025f*.png`; gives the game's 25-entry model table room for 100 (`npc_models_install`, the read at `0x4E430F`) and points slots 25-99 at the launcher folder's bodies by their full path (the resource manager looks in the archives before opening anything, so a relative path never reaches the file hook) |

Kaiba Corporation's NPCs are the slots `k` = 0, 1, 2...: slot `k`'s three levels are NPC ids `120 + 3k` to `122 + 3k`.
The game has 25 (0-24, Yami Yugi to Kalin Kessler); custom ones take **slots 25-99**, 75 characters
([how many](#how-many)).

## How many

What bounds a custom slot `k` (its levels are ids `120 + 3k` to `122 + 3k`), from the client (2026-11-27), and what
the connection DLL does about it:

| What | Where | The game's bound | Now |
| --- | --- | --- | --- |
| Names and help, text `0x5208 + 2k` | `txttbl_*.bin` | free up to slot 99 (slot 25's are placeholders: "main NPC name" in J and K, empty elsewhere); slots 100-102 are other texts | **the limit: slot 99**. Past it, move the base at `0x51CC56` and `0x4BFE45` (`lea [r+r+imm32]`), or skip 100-102 (free again to 499) |
| Deck titles, text `0x5FB4 + 2·id` | lookups at `0x4BB299`, `0x4BFE05`, `0x5174A8` | free up to id 248 (slot 42), but for slot 25's (ids 195-197: the game's unlisted 26th NPC's) and 24896 (a placeholder in J and K); id 250's text is a player title | each lookup's `lea` is a call to the DLL's stub: the game's text below id 250, `200000 + 2·id` from there (`yo3_data::added::deck_title_text`) |
| The players' experience (`npcbin`) | `0x51C680` (`cmp esi,0x400` at `0x51C68F`, `esi = 4·(id − 1)`) | id 256 (slot 44); past it the stars and the record read 0 (no crash) | raised to id 512 (`NPC_MAX_ID`); the server sends 4096 bytes |
| Lines, text `50000 + 100·id + kind` (slot k's from `62000 + 300k`) | `txttbl_*.bin` | free above 69486, so every slot from 25 | |
| Bodies (`0x7F5B48`), cut-in timing (`0x78BBF8`), the cut-in's limit (`0x4E3C1D`), its background (`0x788C40`) | the connection DLL's own tables | 25 (the background table 26 entries) | `NPC_SLOTS` (100) in `tools/mod/card-profile.h`; the limit is a signed byte (127 without a stub) |
| Kaiba Corporation's list | `kaiba_main.swf`, a CLIK ScrollingList | 10 rows show; the mouse wheel scrolls it a row a tick (live: 17 rows on one tab). Its scrollbar is placed but hidden and unwired, and the keyboard is off (`handleInput` returns false) | none, though nothing on screen says there is more: a scrollbar needs a changed `kaiba_main.swf` |
| The tabs | `0x4BFBA0` (tab > 2 refused: the tab indexes three vectors), the Flash's three | three; any character on any tab | |
| NPC ids | every `npc_id`, `npc_type`, `cpu_id`, `need_ex` read (`0x409020`) and `0x17AC`'s `cpu_id` | 32 bits | |

The server and the launcher take slots in `yo3_data::added::CUSTOM_NPC_SLOTS` (25-99): `Added::parse` refuses a slot
outside it, a slot two characters share, a body past 24, a character without three levels and a `cutin_bg` outside
1-10. No other per-slot bound was found: the duel start (`0x4BE390`), `Duel_createCutins` (`0x50F450`), the faces
(`0x4F8080`), the talk's gate (`0x4BDAB0`), the free NPCs (`0x79039C`: never a custom id, they need `premium`, which
every account has) and the NPC texts take any slot. Characters past slot 29 need a launcher and DLL of this version:
an older launcher refuses them when it writes the added files.

## The characters

31 so far, each on the mannequin with its portrait and faces, three levels (titles easiest first), printed and anime
decks that pass the server's deck check in both modes (`tests/custom_npc.rs`), talk, chat lines and a bot style.
"Waiting" counts the later-batch anime-only cards their anime decks list (`anime_waiting`): each plays its stand-in
until the server has the card.

| Slot | Name | Tab | Decks | Waiting |
| --- | --- | --- | --- | --- |
| 25 | Dartz | Yu-Gi-Oh! | The Seal of Orichalcos / The Soldiers of Atlantis / The Great Leviathan Awakens |  |
| 26 | Rafael | Yu-Gi-Oh! | Guardians of the Gods / The Guardian Assault / The Ultimate Guardian |  |
| 27 | Valon | Yu-Gi-Oh! | The Armor of a Street Fighter / Knuckles and Armor / The Full Armor Duelist | 29 |
| 28 | Alister | Yu-Gi-Oh! | The Machines of the Sky / Tanks and Fortresses / The Air Fortress Ziggurat |  |
| 29 | Noah Kaiba | Yu-Gi-Oh! | The Virtual World / Noah's Phantom Army / Shinato, King of a Higher Plane |  |
| 30 | Ishizu Ishtar | Yu-Gi-Oh! | The Tablet's Secret / The Millennium Sphinx / The Forbidden One |  |
| 31 | Weevil Underwood | Yu-Gi-Oh! | The Insect Swarm / Lair of the Parasites / Perfectly Ultimate Great Moth | 2 |
| 32 | Rex Raptor | Yu-Gi-Oh! | Primal Instinct / Jurassic Rampage / The Tyranno Pride | 5 |
| 33 | Zigfried von Schroeder | Yu-Gi-Oh! | The Prince of the Nibelung / Wotan's Judgment / Valkyrie's Wrath |  |
| 34 | Odion | Yu-Gi-Oh! | The Tomb Keeper / The Sealed Soul / Embodiment of Apophis |  |
| 35 | Mako Tsunami | Yu-Gi-Oh! | The Rage of the Sea / Fishermen of the Deep / Legendary Ocean's Fury | 3 |
| 36 | Yubel | GX | The Gaze of the Spirit / Love Beyond Rebirth / The Ultimate Nightmare | 17 |
| 37 | Sartorius | GX | The First Card of Fortune / Reading the Arcana / The Destiny of the Light Ruler | 16 |
| 38 | Bastion Misawa | GX | Magnetic Attraction / Polarity of the Warriors / The Magna Warrior and the Water Dragon | 15 |
| 39 | Nightshroud | GX | A Spotlight in the Dark / Red-Eyes in the Night / The Darkness Takes the Stage | 16 |
| 40 | Tyranno Hassleberry | GX | Dino Rush / Jurassic Ambush / Rise of the Ultimate Tyranno | 13 |
| 41 | Axel Brodie | GX | Burning Streets / Volcanic Eruption / Doomfire of the Volcano King | 21 |
| 42 | Camula | GX | Bloodline of the Night / The Vampire Court / Genesis of the Vampire Queen | 13 |
| 43 | Lyman Banner | GX | The Alchemist's Laboratory / Transmutation of Gold / The Great Work of Amnael | 25 |
| 44 | Jim Crocodile Cook | GX | Fossil Dig Site / Fossil Fusion Awakened / The Skull Convoy Rolls Out | 13 |
| 45 | Blair Flannigan | GX | Love at First Sight / The Maidens' Charm / The Mystic Dragon's Promise | 15 |
| 46 | Z-one | 5D's | Echoes of Tomorrow / Sandaion's Standby / The Last of the Timelords | 12 |
| 47 | Rex Goodwin | 5D's | Spirits of the Bound Land / The Immortals Rise / The Wrath of Wiraqocha Rasca | 7 |
| 48 | Aporia | 5D's | Armies of the Meklords / Emperors of the Machine World / The Meklord Emperors ∞ | 17 |
| 49 | Primo | 5D's | Wisel's Vanguard / Carriers of the Meklord Army / Primo, Guardian of the Future | 18 |
| 50 | Antinomy | 5D's | Genus Trials / Technical Genus Strike / Halberd Cannon, the Final Genus |  |
| 51 | Misty Tredwell | 5D's | Cold Gaze of the Reptilianne / Sad Stories of the Serpent / The Gaze That Turns to Stone | 12 |
| 52 | Roman Goodwin | 5D's | Webs in the Dark / The Bound Deity Stirs / Roman Goodwin, the Weaver of Fate | 13 |
| 53 | Sherry LeBlanc | 5D's | The Garden of Fleur-de-lis / Thorns of the Noble Rose / Chevalier de Fleur, Knight of the Lily | 15 |
| 54 | Greiger | 5D's | Spirits in the Quake / The Earth Cries Out / Chacu Challhua, the Fury of Earth | 17 |
| 55 | Tetsu Trudge | 5D's | Sector Security Patrol / The Security Barricade / Gate Guardian, Sector Security's Pride | 18 |

Ishizu Ishtar, Sartorius and Z-one have the heroes' Heart of the Cards (`npc.rs` `SEERS`): before each draw the bot
puts the card it needs most on top of their Deck. Measured in self-play before it was kept (24 duels each, the same
seeds): wins 13 to 16, 10 to 19 and 4 to 21.

The Dark Signers (Rex Goodwin, Roman Goodwin, Greiger, and Trudge's last level) play Dark Tuner Nightmare Hand and
Catastrogue: their effect Tributes the Dark Tuner and a lower non-Tuner to bring any Synchro Monster of the Levels'
difference (Underground Arachnid, Hundred Eyes Dragon, Dark Diviner, which with anime cards on are Dark Synchro Monsters,
and the printed Frozen Fitzgerald and Dark Highlander; `tests/dark_synchro.rs`). A true Dark Synchro Summon needs an
anime Dark Tuner (setcode 0x600, batches 32-34), whose Level minus the non-Tuner's is the Dark Synchro's: Roman's
Spider Cocoon (5) makes Dark Diviner with a Level 1, Greiger's Doom Submarine (9) Underground Arachnid with a Level 3,
Trudge's Chaos Rogue (8) Dark Diviner with a Level 4. Rex's Dark Goddess Witaka (3) takes a Synchro Monster's
Level for 1000 LP: at Hundred Eyes Dragon's 8 she makes Dark Diviner with a Level 4, or Underground Arachnid with Man-Eater Bug (2).

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
| `slot` | 25-99, one per character |
| `name`, `help` | Kaiba Corporation's list and info panel (text ids `0x5208 + 2·slot` and `+1`). Keep the help in the character's world (no "anime cards") |
| `series` | the tab: 0 Yu-Gi-Oh!, 1 GX, 2 5D's |
| `body` | the game NPC (0-24) whose skeleton and 53 animations it moves with |
| `figure` | `"mannequin"` for our clothes-store mannequin on that skeleton; absent: the game NPC's own body and skins |
| `cutin_bg` | the duel cut-in's background, `data/gfx/cutin_bg_npc01.swf` to `10` (optional; absent: the body's) |
| `levels[].anime_waiting` | anime-only cards of a later batch the anime deck wants (optional): `[{"card": "…", "instead": "…"}]`, each in place of one copy of `instead` (in `anime_deck`, or `anime_extra` for an Extra Deck card) until the server has the card, then swapped in by itself (`AddedNpcLevel::anime_cards`) |
| `levels` | three, easiest first: a deck `title` (text `0x5FB4 + 2·id`, from id 250 `200000 + 2·id`), the printed `deck` (and optional `extra`), and an `anime_deck` (and `anime_extra`) played while `anime-cards` is on; 40-60 cards, each by its exact English name as the server knows it (the game's cards and the added ones) |
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

- `cargo nextest run -p yo3-server --test custom_npc` (roster, tab, every character's decks known and legal in both
  modes, a bot style each), `task check:npcs` (`tests/custom_npc_selfplay.rs`: each level's deck in duels the bot plays
  on both sides against a hero of its series, anime cards off and on, under the duel check; a report of results, stalls
  and the bosses summoned), `-p yo3-data --test models`
  (every game model round-trips; the mannequin builds and keeps its 53 animations).
- Live, with a scratch server (`task dev:server`, `scratch-server.py admin set anime-cards on|off`) and the headless
  client: Kaiba Corporation (765, 275), click through the greeting to the list, select the character (its portrait and
  body on the right, its help on the left), select again for the talk (its lines, its body), then Yes, Select Deck's
  OK and the janken: its face, its lines in the duel. Switching `anime-cards` needs a new Play (a game showing the other
  card versions is refused).
