# Anime card versions

<!-- Made by tools/engine/anime-map.py from engine/anime/research.tsv: do not edit by hand. -->

With the host's `anime-cards` setting on ([playing](playing.md#anime-card-versions)), the 412 cards below play by their anime effects, ProjectIgnis' unofficial scripts (CardScripts `915c6ff`), in NPC and player duels, and the game shows them as their anime versions: the **card profile**. The other cards, and the 6 retained or held back below, play and show as they always did. The reviewed table is the issue [#60](https://github.com/usersina/yo3-revival/issues/60); an unofficial script loading without an error is not proof of anime accuracy, only that it plays.

## The card profile

A card keeps its Konami id, so it is the card a player already owns, in the decks they already have: under the anime profile it becomes its anime version. A profile is a named set of card data that every part reads (`yo3-server`'s `profile.rs`; the `anime` profile is `engine/anime-cards.json`).

| Part | What changes |
| --- | --- |
| Rules engine | The anime card's script: its effects, stats, types and summoning procedures |
| Client metadata | The client's own card table is rewritten in memory by the connection DLL: kind (Normal, Effect, Spell, Trap...), Spell/Trap kind, Level, ATK and DEF (also numbers that are not multiples of ten, and `?`), attribute, race, and whether the card is a Main or an Extra Deck card |
| Card display | The anime card's effect text and, when you provide it, its artwork ([below](#artwork)) |
| Saved decks | Cards keep their place in the collection and their quantities in every deck; a card whose class changes (Flame Swordsman: Extra Deck in the game, Main Deck in the anime) moves to the list the profile puts it in, in the server's decks (list, uploads, checks, NPC and official decks) and in your own local deck files (the launcher places them before the game starts, [below](#local-deck-files)), with no manual editing |
| Duel check | `--duel-check` compares the client's view with the engine, and holds the profile's cards to what the profile changes of them (what is the game's own is the client's data, which `card_data.rs` holds equal to the engine's for every card): the first time a profile monster is face-up on the field its base ATK and DEF (a `?` too) must be the profile's, and the self-play (`tests/duel_check.rs`, anime decks of 30 duels by default, 300 in a full run, and decks made of the cards the profile changes) also checks each deck card's Level, race and attribute against the engine's card data (on the field those are the effects' as much as the card's), so a profile that disagrees with what the engine plays fails the check (`a_profile_that_differs_from_the_engine_fails_the_duel_check` proves it fails). The native checkpoints (`[YO3 EXPECT]`) hash the stats the client actually received, which come from the profile's row and the engine's field reports |

The server picks the profile (the `anime-cards` setting, later an event) and pins it: the game fetches it once, when it starts (`GET /web/card-profile` through the launcher, which only a signed-in launcher reaches), and reads its card table once. **A change of profile therefore needs a restart of the game** (Play again) until it is proven that every cached card refreshes safely: a duel tells a player whose game started before the change to restart, and the Desktop launcher notices it too: its helper asks the server which profile it pins every 20 seconds, and while it differs from the one the game loaded the big button reads RESTART THE GAME, which closes the game and plays again. Without the setting, or when the game was started without the connection DLL's profile support, the game shows its own cards and the engine still plays the anime effects, as before.

## Local deck files

The game keeps your decks as `players/<name>/decks/*.y3d` in its Documents folder, and its deck editor files a card by the card's kind, so a deck saved under another profile holds Flame Swordsman in the wrong list. **The launcher places them before the game is released** (the helper, once it has the session and the profile the game will fetch; `yo3_launcher::decks`): each card the profile covers goes to the list the profile puts it in, by the server's own rule, with every card and copy kept and the others in their order. A deck is never deleted. The first change to a file keeps a copy beside it (`<deck>.y3d.pre-profile`, never overwritten), the profile a player was last adapted to is noted in `players/<name>/revival-profile.txt` (with where each moved card was, so the game's own profile brings it back), and a start with the same profile changes nothing. A file that is not a deck the game wrote, or cannot be written, is left alone and named in the game's log. The folder is the game's own for that server (the original `Yu-Gi-Oh! ONLINE 3` folder for the first server played on, `YO3 Revival/servers/<id>/Yu-Gi-Oh! ONLINE 3` for every other; the Desktop tells the helper where Documents is). A game opened without the launcher (direct start) adapts at the next Play; the server places the deck for every duel meanwhile.

## Artwork

Nothing in this project ships, fetches or converts Konami's or the anime's card images. A card shows its anime artwork only if you put the picture in a folder the launcher reads at start; a card without a file keeps the game's own artwork.

- `anime-art/card/<Konami id>.png`: the whole card face as the anime version reads (frame, name, picture, text, stats), a 200x290 PNG like the game's own `data/card/<id>_<art>.png`. One file per Konami id serves all of that card's artworks (Flame Swordsman's two).
- `anime-art/mini/<Konami id>.png`: optional, the 50x72 thumbnail like `data/mini/<id>_<art>.png` (Edit Deck lists, the duel's field). When it is missing the launcher makes it from the face (the thumbnails it made are listed in `mini/.derived`, made again when the face changes, and never replace one you provided).
- The folder is `%LOCALAPPDATA%\YO3 Revival\anime-art` on Windows and `~/.local/share/yo3-revival/anime-art` on Linux (the launcher's data folder; `$YO3_DESKTOP_DATA/anime-art` when that is set). The launcher creates it, passes it to the game as `YO3_ANIME_ART` (and writes it into the direct-start profile) and logs how many faces it found and which files it skipped (not a PNG, not 200x290). Games started without the launcher look in `anime-art` beside `yo3.exe`.
- A host's players each provide their own pictures: the server never sends artwork.

## Shown as the anime card

These cards differ from the game's in what the client draws; the profile covers every difference. (The cards that do not differ are shown with the anime card's text only.)

| Card | What the client shows now |
| --- | --- |
| Alchemy Cycle | subtype: +Counter |
| Ape Fighter | level: 4 -> 5 |
| Arcana Force EX - The Dark Ruler | attribute: 16 -> 32 |
| Barox | deck: Main/Extra Deck placement differs (Fusion -> Main Deck)<br>type: +Normal<br>atk: 1380 -> 1800<br>def: 1530 -> 2000 |
| Bickuribox | deck: Main/Extra Deck placement differs (Fusion -> Main Deck)<br>type: +Normal |
| Bicorn Re'em | type: +Tuner |
| Blackwing - Backlash | subtype: +Counter |
| Blast Sphere | atk: 1400 -> 2900 |
| Boost Warrior | type: +Tuner |
| Bowganian | level: 3 -> 4 |
| Call of the Haunted | kind: the game's card is a Trap, the anime one a Spell |
| Castle of Dark Illusions | flags: -Flip |
| Chiron the Mage | race: 32768 -> 16384 |
| Chthonian Blast | kind: the game's card is a Trap, the anime one a Spell |
| Contract with Exodia | type: +Ritual |
| Copycat | kind: the game's card is a Monster, the anime one a Spell |
| Cyber Blader | level: 7 -> 6 |
| Cyber Raider | type: +Normal, -Effect<br>race: 32 -> 1<br>attribute: 32 -> 1 |
| Dark Tinker | level: 2 -> 3 |
| Dark-Eyes Illusionist | flags: -Flip<br>def: 1400 -> 0 |
| Des Feral Imp | type: +Normal, -Effect<br>flags: -Flip<br>race: 524288 -> 8 |
| Different Dimension Dragon | atk: 1200 -> 1400 |
| Doomcaliber Knight | type: +Normal, -Effect<br>race: 8 -> 16 |
| Dragon Capture Jar | kind: the game's card is a Trap, the anime one a Monster |
| Dragon Master Knight | def: 5000 -> 4000 |
| Dragon Piper | flags: -Flip<br>attribute: 4 -> 32 |
| Drillago | type: +Normal, -Effect<br>attribute: 32 -> 1 |
| Dungeon Worm | type: -Normal, +Effect<br>level: 5 -> 6 |
| Earthbound Immortal Chacu Challhua | def: 2400 -> 1600 |
| Earthbound Immortal Wiraqocha Rasca | atk: 100 -> 1<br>def: 100 -> 1 |
| Earthbound Wave | subtype: -Counter |
| Ectoplasmer | subtype: -Continuous, +Field |
| Elemental Hero Divine Neos | atk: 2500 -> 3000 |
| Elemental Hero Flame Wingman | level: 6 -> 8 |
| Elemental Hero Neo Bubbleman | deck: Main/Extra Deck placement differs (Main Deck -> Fusion)<br>flags: -Special Summon only |
| Elemental Hero Prisma | level: 4 -> 5 |
| Energy Drain | kind: the game's card is a Trap, the anime one a Spell |
| Evil Hero Infernal Sniper | type: -Effect |
| Exodia Necross | type: +Ritual<br>flags: -Special Summon only |
| Extra Veiler | type: +Tuner |
| Fiend's Sanctuary | subtype: +Quick-Play |
| Five-Headed Dragon | deck: Main/Extra Deck placement differs (Fusion -> Main Deck)<br>type: +Ritual |
| Flame Swordsman | deck: Main/Extra Deck placement differs (Fusion -> Main Deck)<br>type: +Normal |
| Future Fusion | subtype: -Continuous, +Equip |
| Gernia | type: +Normal, -Effect<br>race: 16 -> 8 |
| Goblin Zombie | type: +Normal, -Effect |
| Gold Sarcophagus | subtype: +Continuous |
| Golden Homunculus | flags: +Special Summon only<br>atk: 1500 -> -2<br>def: 1500 -> -2 |
| Gorgon's Eye | subtype: +Continuous |
| Granadora | attribute: 2 -> 32 |
| Hamon, Lord of Striking Thunder | race: 4096 -> 2097152<br>attribute: 16 -> 64 |
| Infinite Dismissal | subtype: -Continuous |
| Interdimensional Matter Transporter | kind: the game's card is a Trap, the anime one a Spell |
| Ivy Shackles | kind: the game's card is a Trap, the anime one a Spell |
| Jigen Bakudan | flags: -Flip |
| Judgment of Anubis | subtype: -Counter |
| Labyrinth Tank | deck: Main/Extra Deck placement differs (Fusion -> Main Deck)<br>type: +Normal |
| Larvae Moth | type: +Normal, -Effect<br>flags: -Special Summon only |
| Lava Golem | flags: -Special Summon only |
| Level Modulation | kind: the game's card is a Spell, the anime one a Trap |
| Lightforce Sword | kind: the game's card is a Trap, the anime one a Spell |
| Little-Winguard | type: +Normal, -Effect |
| Loki, Lord of the Aesir | race: 2 -> 2097152<br>attribute: 32 -> 64 |
| Machine Conversion Factory | subtype: -Equip |
| Magicians Unite | subtype: +Quick-Play |
| Megasonic Eye | level: 5 -> 4 |
| Meklord Emperor Granel ∞ | flags: -Special Summon only |
| Meklord Emperor Skiel ∞ | flags: -Special Summon only<br>atk: 2200 -> 0<br>def: 2200 -> 0 |
| Meklord Emperor Wisel ∞ | flags: -Special Summon only<br>atk: 2500 -> 0<br>def: 2500 -> 0 |
| Mind Control | kind: the game's card is a Spell, the anime one a Trap |
| Mystical Elf | type: -Normal, +Effect |
| Negate Attack | kind: the game's card is a Trap, the anime one a Spell |
| Neos Wiseman | deck: Main/Extra Deck placement differs (Main Deck -> Fusion)<br>flags: -Special Summon only |
| Odin, Father of the Aesir | race: 4 -> 2097152<br>attribute: 16 -> 64 |
| Over Limit | subtype: +Continuous |
| Perfect Machine King | deck: Main/Extra Deck placement differs (Main Deck -> Fusion)<br>def: 1500 -> 2200 |
| Pumpking the King of Ghosts | atk: 1800 -> 1500 |
| Rainbow Dark Dragon | attribute: 32 -> 16 |
| Rapid Warrior | level: 4 -> 3 |
| Raviel, Lord of Phantasms | race: 8 -> 2097152<br>attribute: 32 -> 64 |
| Reaper of the Cards | flags: -Flip<br>atk: 1380 -> 1800<br>def: 1930 -> 2000 |
| Red-Eyes Darkness Metal Dragon | flags: +Special Summon only |
| Skull Dice | kind: the game's card is a Trap, the anime one a Spell |
| Soul Taker | subtype: +Quick-Play |
| Spell Absorption | subtype: +Quick-Play, -Continuous |
| Supersonic Skull Flame | atk: 2600 -> 3000<br>def: 2000 -> 2300 |
| Swordstalker | type: -Normal, +Effect |
| The Eye of Truth | kind: the game's card is a Trap, the anime one a Spell |
| Thor, Lord of the Aesir | race: 32768 -> 2097152<br>attribute: 1 -> 64 |
| Tragedy | kind: the game's card is a Trap, the anime one a Spell |
| Trap Eater | type: -Tuner |
| Uria, Lord of Searing Flames | race: 128 -> 2097152<br>attribute: 4 -> 64 |
| Virus Cannon | kind: the game's card is a Trap, the anime one a Spell |
| Wall of Ivy | flags: -Flip |
| Wall Shadow | deck: Main/Extra Deck placement differs (Main Deck -> Fusion)<br>flags: -Special Summon only |
| White Night Dragon | attribute: 2 -> 32 |
| Zoma the Spirit | subtype: -Continuous |
| Zombie Warrior | deck: Main/Extra Deck placement differs (Fusion -> Main Deck)<br>type: +Normal |

## Released categories

A card is released only when the client side of each difference is proven: the connection DLL shows it, the deck rule places it and the duel check holds the engine to it. The proof set (Flame Swordsman, Mystical Elf, Negate Attack) came first: Normal and Effect, Monster and Spell/Trap, Main and Extra Deck, text and artwork, end to end. Then, category by category, the held cards that differ in that category (and in nothing else a hold names) were released, each with a test card (`tests/card_profile.rs`: the row and its deck placement), a real-core scenario in which the changed property matters (`tests/anime_cards.rs`) and the anime self-play duel checks with the category on (`tests/duel_check.rs`). `PROFILE_COVERS` in `tools/engine/anime-map.py` lists them; `tools/engine/anime-map.py --releases` says what a category would still release (nothing now).

| Category | What the client shows | Cards that differ | Test card and scenario |
| --- | --- | ---: | --- |
| flags | nothing: Flip and Special Summon only are shown nowhere, the engine plays the anime card | 18 | Wall of Ivy: its token comes when it is attacked face-up |
| subtype | a Spell or Trap's own kind (Continuous, Quick-Play, Equip, Field, Counter) | 16 | Fiend's Sanctuary: a Quick-Play Spell, activated on the opponent's turn |
| type | Normal, Effect, Tuner, Ritual and the other kinds of monster | 24 | Boost Warrior: a Tuner, so it Synchro Summons |
| deck | Main or Extra Deck: a Fusion Monster is an Extra Deck card, a Normal one a Main Deck card | 10 | Perfect Machine King: a Fusion Monster, Fusion Summoned from the Extra Deck |
| level | Level | 9 | Rapid Warrior: Level 3, so it makes the Level 6 Synchro Monster |
| attribute | attribute | 13 | Granadora: DARK, so Umiiruka does not raise it |
| race | type of monster (Warrior, Beast...) | 11 | Chiron the Mage: a Beast, so Sogen does not raise it |
| atk | ATK, also a number that is not a multiple of ten, and `?` | 11 | Pumpking the King of Ghosts: 1500 ATK, the damage it takes |
| def | DEF, as ATK | 11 | Earthbound Immortal Chacu Challhua: 1600 DEF |
| kind | Monster, Spell or Trap | 15 | Mind Control: a Trap, activated on the opponent's turn |

Negate Attack is the pinned database's anime card (a Quick-Play Spell that negates the attack and ends the Battle Phase), not the Counter Trap the game has; the version is the issue's open era decision, shown here so it can be checked in play.

Every reviewed card has one disposition: anime enabled, the game's version retained as the default with a reason (Time Wizard and Thousand Dragon, reviewed together, and Temple of the Kings), or held for a technical fix: Theinen the Great Sphinx (an effect the Deck triggers, not a procedure: the client's menus have not been shown to offer it) and Clear Vice Dragon and Blackwing - Aurora the Northern Lights, whose "?" ATK the anime database spells as 0 (the profile would show 0 where the game shows "?"). Power Bond (it stays Power Bond and is treated as Polymerization), Multiply and Cloning (the copy shows as the monster itself) and the Sphinxes that summon themselves from the Deck are released, each with a test in `tests/anime_cards.rs`.

**Source of truth for effects:** EDOPro/ProjectIgnis's anime scripts at the pinned commit. Effects are not re-checked against the episodes; a disagreement is fixed upstream or with a documented override in `engine/overrides/`. Two exceptions (#60): cards for which ProjectIgnis has only a placeholder (Dark Strike Fighter and Fortune Lady Fire) are researched and get our own record and script ([below](#reconstructions)); and Skill Drain against the three Egyptian Gods was a bug, fixed by an override of Skill Drain (`engine/overrides/c82732705.lua`, which changes only how it treats the anime Gods; every other monster is as before). The Gods are unaffected by most effects, but Skill Drain, when activated, negates the effects of the Gods already on the field until the end of that turn; then they come back, though Skill Drain stays face up. Ra's ATK (the Tributes' total) is 0 for the rest of that turn and the total again from the next one. A God summoned while Skill Drain is already face up is not affected at all. Each point has a real-core test in `tests/anime_cards.rs`.

## Reconstructions

Where ProjectIgnis has no usable script, the data record and the script are ours (`engine/anime/records.json`, `engine/overrides/`) and the card's text in the game says it is a reconstruction. Everything else about the card (the engine, the profile, the duel check, the deck placement) is as for any other mapped card; the official (standard) card, its record and its script do not change.

| Card | Anime card | Text the game shows | Basis |
| --- | --- | --- | --- |
| Dark Strike Fighter | 513000079 | [Anime card, a reconstruction of episodes 21 and 134: not verified] 1 Tuner + 1 or more non-Tuner monsters<br>During either Main Phase, once per turn for each copy of this card: You can Tribute 1 other monster you control; inflict damage to your opponent equal to the Tributed monster's Level (as it was when Tributed) x 200. This card cannot attack during the turn you activate this effect. | The placeholder row has 2700 ATK and no script. Stats: 2600/1800, Level 7, DARK Machine Synchro Effect. The effect is a community reconstruction of 5D's episodes 21 and 134, unverified. |
| Fortune Lady Fire | 511247002 | [Anime card, a reconstruction] This card's ATK/DEF become its Level x 200. Once per turn, during your Standby Phase: Increase this card's Level by 1 (max. 12). When this card is Special Summoned face-up in Attack Position, by any card or procedure (no "Fortune Lady" card needed): Target 1 face-up monster your opponent controls; destroy that target, and if you do, inflict damage to your opponent equal to the destroyed monster's ATK as it was before it was destroyed. | The placeholder row only says 'same as OCG'. The official card with one change: the destroy trigger works when this card is Special Summoned face-up in Attack Position by any means, with no Fortune Lady source required. The target (1 face-up monster the opponent controls) is the official card's, the rule the anime card's text gives too. |

### Dark Strike Fighter (8035)

A Level 7 DARK Machine Synchro/Effect Monster, 2600/1800 (the upstream placeholder had 2700 ATK). The rules below are the community's
reconstruction of 5D's episodes 21 and 134, not verified against the footage; where it left room, the choice is ours.

- Summoned with the Synchro procedure the game already has (1 Tuner + 1 or more non-Tuner monsters).
- An ignition effect, usable in either of your Main Phases, once per turn **for each copy** (not "by name").
- Cost: Tribute another monster you control (this card itself can never be the Tribute). Its Level is taken when it is Tributed.
- Effect: inflict that Level x 200 damage to your opponent. The amount belongs to the activation, so it is dealt even if Dark Strike Fighter
  has left the field before the effect resolves.
- Attack restriction, our interpretation of "this card cannot attack during the turn you activate this effect": the effect cannot be
  activated if that copy has already declared an attack this turn, and activating it forbids that copy to attack for the rest of the turn.
  The restriction is installed at activation, so negating the effect does not lift it, and it ends with the turn.

### Fortune Lady Fire (8302)

The official card (ATK/DEF = its Level x 200, the Level up by 1 in each of your Standby Phases to at most 12) with one change: its destroy
effect triggers whenever it is Special Summoned face-up in Attack Position, by any means. No "Fortune Lady" source is needed, and a Special
Summon procedure with no effect that activates counts (Monster Reborn, a Fortune Lady effect, anything). Special Summoned in Defense
Position, or Normal Summoned, it does not trigger. The target is the official card's (1 face-up monster your opponent controls),
the rule the anime card's text gives too; if the opponent has none, the trigger is not activated. The destruction and the damage go together: nothing destroyed
(a protected monster, one that left the field before the effect resolved) means no damage, and the damage is the destroyed monster's ATK
as it was before it was destroyed.

### Skill Drain and the Egyptian Gods

See above: not a reconstruction but a fix (`engine/overrides/c82732705.lua`); the three Gods are mapped, so it applies to all of them.

Temple of the Kings (5231) keeps the game's version: its anime rules would need a reconstruction of Duel Monsters episodes 87 and 89 (which cards the sealed-card effect may seal, the set restrictions, how far the protection reaches, the release costs and when the card is lost). The upstream script mixes the original, later and printed rules, so it is never enabled; nothing is implemented.

## Played as their anime versions

| Card | Anime card | Notes |
| --- | --- | --- |
| Alchemy Cycle | 511027026 |  |
| Amazoness Chain Master | 810000028 |  |
| Amplifier | 810000042 |  |
| Ancient Gear Explosive | 511024009 |  |
| Andro Sphinx | 513000028 |  |
| Ape Fighter | 511001396 |  |
| Ape Magician | 511001395 |  |
| Arcana Force 0 - The Fool | 511016002 |  |
| Arcana Force EX - The Dark Ruler | 511005639 |  |
| Arcana Force EX - The Light Ruler | 511003201 | names other anime cards |
| Arcana Force VII - The Chariot | 511016000 |  |
| Armityle the Chaos Phantom | 511000253 | names other anime cards |
| Armored Cybern | 511002821 |  |
| Assault Armor | 511777005 |  |
| Assault Mode Activate | 511002480 |  |
| Barox | 511002909 |  |
| Barrel Dragon | 511000769 |  |
| Battle Waltz | 511002844 | makes Waltz Token |
| Berserk Dragon | 511000378 |  |
| Bickuribox | 511002614 |  |
| Bicorn Re'em | 511002882 |  |
| Black Feather Beacon | 810000052 |  |
| Black Garden | 511014000 | makes Rose Token |
| Black-Winged Dragon | 511027112 |  |
| Blackboost | 810000051 |  |
| Blackwing - Backlash | 511030014 |  |
| Blackwing - Boreas the Sharp | 511600076 |  |
| Blackwing - Calima the Haze | 511002623 |  |
| Blackwing - Silverwind the Ascendant | 511001954 |  |
| Blackwing - Vayu the Emblem of Honor | 511002219 |  |
| Blackwing - Zephyrus the Elite | 511001961 |  |
| Blast Held by a Tribute | 511005091 |  |
| Blast Sphere | 511002509 | names other anime cards |
| Blue-Eyes Shining Dragon | 511002418 |  |
| Boost Warrior | 511600376 |  |
| Bowganian | 511005647 |  |
| Brain Control | 511002549 |  |
| Burial from a Different Dimension | 511002813 |  |
| Burning Land | 511002578 |  |
| Byroad Sacrifice | 511756006 |  |
| Call of the Haunted | 511002048 |  |
| Card Breaker | 810000050 |  |
| Card of Sacrifice | 511002552 |  |
| Card of Safe Return | 511000258 |  |
| Card of Sanctity | 511001126 |  |
| Cards for Black Feathers | 810000056 |  |
| Cards of Consonance | 511600013 |  |
| Castle of Dark Illusions | 511002167 |  |
| Catapult Turtle | 511004015 |  |
| Chaos Burst | 511002837 |  |
| Chaos King Archfiend | 511009313 |  |
| Chevalier de Fleur | 511002227 |  |
| Chimeratech Fortress Dragon | 511002463 |  |
| Chimeratech Overdragon | 511002108 |  |
| Chiron the Mage | 511310000 |  |
| Chosen One | 513000116 |  |
| Chthonian Alliance | 511002542 |  |
| Chthonian Blast | 511002541 |  |
| Chthonian Soldier | 511001211 |  |
| Cloning | 511600007 | makes copies of a monster |
| Cloudian - Eye of the Typhoon | 511001786 |  |
| Cocoon of Evolution | 511600069 | names other anime cards |
| Contact | 511600382 |  |
| Contract with Exodia | 511000241 |  |
| Convert Contact | 511600348 |  |
| Copy Plant | 511003025 |  |
| Copycat | 511002387 | names other anime cards |
| Cosmic Compass | 511600432 | makes Compass Token |
| Cost Down | 511002540 |  |
| Creation Resonator | 511002603 |  |
| Crimson Fire | 511010530 |  |
| Crimson Mefist | 511001274 |  |
| Crush Card Virus | 513000009 |  |
| Crystal Beacon | 511002946 |  |
| Crystal Beast Ruby Carbuncle | 511001823 |  |
| Crystal Counter | 511001108 |  |
| Crystal Pair | 511001129 |  |
| Cyber Blader | 511001600 |  |
| Cyber Phoenix | 511007022 |  |
| Cyber Prima | 511001598 |  |
| Cyber Raider | 511002828 |  |
| Cyber Valley | 511002464 |  |
| Cyberdark Dragon | 511001117 |  |
| Cyberdark Edge | 511001146 |  |
| Cyberdark Horn | 511001124 |  |
| Cyberdark Keel | 511001123 |  |
| Cybernetic Hidden Technology | 511002826 |  |
| Cyclone Blade | 511000309 |  |
| Cyclone Boomerang | 511023008 |  |
| D - Time | 511002728 |  |
| D2 Shield | 511002879 |  |
| Damage Polarizer | 511003243 |  |
| Dandylion | 511005643 | makes Fluff Token |
| Dark Bribe | 511013023 |  |
| Dark Cure | 511247006 |  |
| Dark Diviner | 511600138 |  |
| Dark Magic Curtain | 511002532 |  |
| Dark Strike Fighter | 513000079 |  |
| Dark Tinker | 511014001 |  |
| Dark-Eyes Illusionist | 511002416 |  |
| Darkness Neosphere | 511310113 | names other anime cards |
| Darksea Rescue | 511002818 |  |
| Deep Diver | 511021001 |  |
| Des Feral Imp | 511002513 |  |
| Destiny End Dragoon | 511002195 |  |
| Destiny Hero - Blade Master | 511027000 |  |
| Destiny Hero - Captain Tenacious | 511018024 |  |
| Destiny Hero - Dogma | 511002729 |  |
| Destiny Hero - Dreadmaster | 511806001 |  |
| Destiny Hero - Plasma | 511000614 |  |
| Destruction of Destiny | 511001910 |  |
| Different Dimension Dragon | 511010701 |  |
| Dimension Explosion | 511002824 |  |
| Discord | 511001212 |  |
| Divine Relic Mjollnir | 511002988 |  |
| Doomcaliber Knight | 511002514 |  |
| Doppelwarrior | 511600433 | makes Doppel Token |
| Double Spell | 513000055 |  |
| Dragged Down into the Grave | 511000218 |  |
| Dragon Capture Jar | 511001040 |  |
| Dragon Master Knight | 511002859 |  |
| Dragon Piper | 511756001 |  |
| Dreamsprite | 511600193 |  |
| Drill Synchron | 511002910 |  |
| Drill Warrior | 511001982 |  |
| Drillago | 511003056 |  |
| Driven Daredevil | 511001967 |  |
| Dungeon Worm | 511130000 |  |
| Earthbound Immortal Aslla piscu | 511000239 |  |
| Earthbound Immortal Ccapac Apu | 511000255 |  |
| Earthbound Immortal Ccarayhua | 511000263 |  |
| Earthbound Immortal Chacu Challhua | 511000262 |  |
| Earthbound Immortal Cusillu | 511000250 |  |
| Earthbound Immortal Uru | 511000242 |  |
| Earthbound Immortal Wiraqocha Rasca | 511000252 |  |
| Earthbound Wave | 511013025 |  |
| Ectoplasmer | 511003218 |  |
| Elemental Hero Aqua Neos | 511023010 |  |
| Elemental Hero Bubbleman | 810000011 |  |
| Elemental Hero Chaos Neos | 511001652 |  |
| Elemental Hero Darkbright | 511023005 |  |
| Elemental Hero Divine Neos | 511001645 | names other anime cards |
| Elemental Hero Flame Wingman | 511018028 |  |
| Elemental Hero Magma Neos | 511023006 |  |
| Elemental Hero Necroshade | 511023014 |  |
| Elemental Hero Neo Bubbleman | 511015128 |  |
| Elemental Hero Neos Knight | 511002031 |  |
| Elemental Hero Prisma | 511600012 |  |
| Elemental Hero Storm Neos | 511023007 |  |
| Elemental Hero Tempest | 511600286 |  |
| Elemental Hero Thunder Giant | 511002533 |  |
| Enemy Controller | 511000604 |  |
| Energy Drain | 511001307 |  |
| Eradicating Aerosol | 511002426 |  |
| Etoile Cyber | 511001599 |  |
| Evil Hero Infernal Prodigy | 511024010 |  |
| Evil Hero Infernal Sniper | 511023015 | names other anime cards |
| Exodia Necross | 511000240 |  |
| Exodius the Ultimate Forbidden Lord | 511000243 | names other anime cards |
| Exploder Dragonwing | 511013005 |  |
| Expressroid | 511009089 |  |
| Extra Veiler | 511013004 |  |
| Fiend's Sanctuary | 511001596 | makes Metal Fiend Token |
| Fiendish Chain | 511600109 |  |
| Final Attack Orders | 513000111 |  |
| Fire Ant Ascator | 511009950 |  |
| Five-Headed Dragon | 511013020 |  |
| Flame Swordsman | 511001128 |  |
| Fog King | 511001783 | names other anime cards |
| Foolish Burial | 511001056 |  |
| Formula Synchron | 511001963 |  |
| Fortune Lady Earth | 511027035 | names other anime cards |
| Fortune Lady Fire | 511247002 |  |
| Fortune Lady Light | 511021000 |  |
| Fortune Lady Water | 511247003 |  |
| Fossil Dyna Pachycephalo | 511024011 |  |
| Future Fusion | 513000094 |  |
| Gernia | 511027032 |  |
| Glow-Up Bulb | 511002598 |  |
| Goblin Zombie | 511003057 |  |
| Gold Sarcophagus | 511000208 |  |
| Golden Homunculus | 511003217 |  |
| Gorgon's Eye | 511002444 |  |
| Graceful Dice | 511000541 |  |
| Granadora | 511002127 |  |
| Great Moth | 511002501 |  |
| Grinder Golem | 511002836 | makes Grinder Token |
| Guardian Eatos | 511001771 |  |
| Hamon, Lord of Striking Thunder | 511000246 |  |
| Hand Destruction | 511002601 |  |
| Honest | 511021006 |  |
| Hope for Escape | 810000006 |  |
| Hundred Eyes Dragon | 513000004 |  |
| Hyper Synchron | 511002755 |  |
| Ido the Supreme Magical Force | 511003208 |  |
| Infernity Beetle | 511010703 |  |
| Infernity Destroyer | 511023004 |  |
| Infernity Mirage | 511010511 |  |
| Infernity Randomizer | 511130001 |  |
| Inferno Tempest | 511002523 |  |
| Infinite Dismissal | 511002884 |  |
| Interdimensional Matter Transporter | 511002539 |  |
| Ivy Shackles | 511021002 |  |
| Jam Breeding Machine | 511002461 | makes Slime Token |
| Jigen Bakudan | 511000772 |  |
| Jinzo | 513000053 |  |
| Jinzo - Lord | 513000120 |  |
| Jinzo - Returner | 511020007 |  |
| Judgment of Anubis | 511002446 |  |
| Junk Gardna | 511600105 |  |
| Junk Warrior | 511009416 |  |
| Jurassic World | 511002874 |  |
| Justice Bringer | 511010531 |  |
| Kazejin | 511002508 |  |
| King's Knight | 511001594 |  |
| Kuriboh | 511002854 |  |
| Labyrinth Tank | 511002616 |  |
| Larvae Moth | 511002500 |  |
| Last Resort | 511005642 |  |
| Lava Golem | 511600435 |  |
| Level Eater | 511013018 |  |
| Level Modulation | 511756003 |  |
| Liberty at Last! | 511002574 |  |
| Light Barrier | 511005633 |  |
| Lightforce Sword | 511600004 |  |
| Lightning Tricorn | 511002229 |  |
| Lineage of Destruction | 511002484 |  |
| Little-Winguard | 511009331 |  |
| Ljosalf of the Nordic Alfar | 511002796 |  |
| Loki, Lord of the Aesir | 511000260 |  |
| Machine Conversion Factory | 511002415 |  |
| Mad Archfiend | 511013001 |  |
| Magic Formula | 511002515 |  |
| Magic Planter | 511002701 |  |
| Magical Dimension | 511600038 |  |
| Magical Mallet | 511000613 |  |
| Magical Stone Excavation | 511016003 |  |
| Magicians Unite | 511600006 |  |
| Majestic Red Dragon | 513000078 |  |
| Makiu, the Magical Mist | 511002502 |  |
| Malefic Blue-Eyes White Dragon | 513000068 |  |
| Malefic Cyber End Dragon | 513000070 |  |
| Malefic Paradox Dragon | 513000074 |  |
| Malefic Parallel Gear | 513000072 |  |
| Malefic Rainbow Dragon | 513000073 |  |
| Malefic Red-Eyes B. Dragon | 513000069 |  |
| Malefic Stardust Dragon | 513000075 |  |
| Malefic Truth Dragon | 513000076 | names other anime cards |
| Malefic World | 511001597 |  |
| Malevolent Catastrophe | 511000376 |  |
| Mark of the Rose | 511030007 |  |
| Mausoleum of the Emperor | 511756002 |  |
| Megasonic Eye | 511600239 |  |
| Meklord Emperor Granel ∞ | 511003222 |  |
| Meklord Emperor Skiel ∞ | 513000173 |  |
| Meklord Emperor Wisel ∞ | 513000167 |  |
| Mesmeric Control | 511002166 |  |
| Metal Reflect Slime | 511009214 |  |
| Meteor Flare | 511013009 |  |
| Meteor of Destruction | 511021008 |  |
| Mind Control | 511000278 |  |
| Miracle Locus | 511002911 |  |
| Mirage Tube | 511600375 |  |
| Mirror Force | 513000001 |  |
| Mirror Gate | 511310017 |  |
| Mirror Wall | 513000002 |  |
| Monster Recovery | 511000373 |  |
| Monster Reincarnation | 511001651 |  |
| Moon Dragon Quilla | 513000186 | names other anime cards |
| Morphtronic Vacuumen | 511002597 |  |
| Multiplication of Ants | 511018004 | names other anime cards |
| Multiply | 810000034 | makes copies of a monster |
| Mystical Elf | 511009014 |  |
| Natural Disaster | 511027782 |  |
| Negate Attack | 511004341 |  |
| Neo-Spacian Aqua Dolphin | 511231002 |  |
| Neo-Spacian Glow Moss | 511600103 |  |
| Neos Wiseman | 511000497 |  |
| No Entry!! | 511600322 |  |
| Null and Void | 810000055 |  |
| Obelisk the Tormentor | 511600398 |  |
| Odin, Father of the Aesir | 511000264 | names other anime cards |
| Offering to the Immortals | 511247015 | makes Ceremonial Token |
| Orichalcos Shunoros | 170000167 | names other anime cards |
| Over Limit | 511005644 |  |
| Overdoom Line | 511003005 |  |
| Overload Fusion | 511010525 |  |
| Owner's Seal | 810000071 |  |
| Parallel Selection | 511027651 |  |
| Perfect Machine King | 511001057 |  |
| Phantasmal Martyrs | 511002835 | makes Phantasmal Martyr Token |
| Phantom of Chaos | 511001926 |  |
| Phantom Skyblaster | 511002834 | makes Skyblaster Token |
| Phoenixian Cluster Amaryllis | 511027024 |  |
| Poison Chain | 511027135 |  |
| Power Bond | 511002825 | treated as Polymerization (24094653) |
| Power Pickaxe | 511001782 |  |
| Power Tool Dragon | 511600116 |  |
| Pumpking the King of Ghosts | 511001147 |  |
| Puppet Master | 511002194 |  |
| Pyramid of Light | 511001887 |  |
| Quickdraw Synchron | 511002860 |  |
| Rain Storm | 511600439 |  |
| Rainbow Dark Dragon | 511001139 |  |
| Rainbow Dragon | 511600140 |  |
| Rapid Warrior | 511002881 |  |
| Rare Value | 511600378 |  |
| Raviel, Lord of Phantasms | 511000261 | makes Phantasm Token |
| Reaper of the Cards | 511002504 |  |
| Red Dragon Archfiend/Assault Mode | 511002518 |  |
| Red Nova Dragon | 513000013 |  |
| Red Screen | 511600437 |  |
| Red-Eyes Darkness Metal Dragon | 511000670 | names other anime cards |
| Reflect Bounder | 511020012 |  |
| Regenerating Rose | 511005737 | makes Regenerating Rose Token |
| Reinforce Truth | 511018015 |  |
| Release Restraint | 511027011 |  |
| Release Restraint Wave | 511013016 |  |
| Remote Revenge | 511010535 |  |
| Return of the Doomed | 511000548 |  |
| Reversal of Fate | 511016001 |  |
| Revival Jam | 513000142 |  |
| Revival Rose | 511002845 |  |
| Ring of Defense | 511002776 |  |
| Riryoku | 511000474 |  |
| Rise of the Snake Deity | 511002525 |  |
| Rope of Life | 511020009 |  |
| Sacred Knight Joan | 511002960 |  |
| Sage's Stone | 511777009 |  |
| Samsara Lotus | 511002833 |  |
| Sanga of the Thunder | 511002506 |  |
| Satellite Cannon | 511777001 |  |
| Sauropod Brachion | 511003090 |  |
| Shield Worm | 511005645 |  |
| Shining Silver Force | 511013017 |  |
| Shooting Star Dragon | 513000012 |  |
| Shrink | 511002777 |  |
| Silent Doom | 511020001 |  |
| Silver Wing | 511010532 |  |
| Skull Dice | 511000540 |  |
| Skull Flame | 511050012 |  |
| Skyscraper | 511002379 |  |
| Skyscraper 2 - Hero City | 511777006 |  |
| Slifer the Sky Dragon | 511600399 | names other anime cards |
| Solemn Authority | 511777010 |  |
| Sorcerer of Dark Magic | 513000095 |  |
| Soul Exchange | 513000054 |  |
| Soul Rope | 511001593 |  |
| Soul Taker | 511000372 |  |
| Spell Absorption | 511000377 |  |
| Spell Calling | 511024007 |  |
| Spell Chronicle | 511000300 |  |
| Spell Gear | 511777004 |  |
| Spellbinding Circle | 511001016 |  |
| Sphinx Teleia | 513000029 |  |
| Spider Web | 511002589 |  |
| Stardust Phantom | 511002883 |  |
| Stardust Xiaolong | 511002807 |  |
| Starlight Road | 511002520 |  |
| Stealthroid | 511002894 |  |
| Stygian Sergeants | 511002841 |  |
| Suijin | 511002507 |  |
| Summon Cloud | 511001924 |  |
| Sun Dragon Inti | 511001981 |  |
| Sunlight Unicorn | 511018014 |  |
| Super Polymerization | 511003010 |  |
| Super Vehicroid - Stealth Union | 511002959 |  |
| Supersonic Skull Flame | 511001202 |  |
| Survival Instinct | 511600441 |  |
| Swordstalker | 810000114 |  |
| Synchro Material | 511002750 |  |
| The Eye of Truth | 511002295 |  |
| The Golden Apples | 511003080 | makes Malus Token |
| The Shallow Grave | 511002531 |  |
| The Winged Dragon of Ra | 511600400 |  |
| Thor, Lord of the Aesir | 511000247 |  |
| Thunder Unicorn | 511002706 |  |
| Time Machine | 511247012 |  |
| Torrential Tribute | 511310007 |  |
| Tour of Doom | 511011004 |  |
| Tragedy | 511008009 |  |
| Trap Eater | 511013002 |  |
| Trap Jammer | 511013024 |  |
| Treasure Map | 511600359 |  |
| Trust Guardian | 511310011 |  |
| Tuning | 511002846 |  |
| Turbo Cannon | 511002708 |  |
| Tyr of the Nordic Champions | 511002985 |  |
| Ultimate Ancient Gear Golem | 511027033 |  |
| Underground Arachnid | 511600137 |  |
| Uria, Lord of Searing Flames | 511000234 |  |
| Valkyrie of the Nordic Ascendant | 511000251 | makes Einherjar Token |
| Variety Comes Out | 511010537 |  |
| Vehicroid Connection Zone | 511003235 |  |
| Vennominaga the Deity of Poisonous Snakes | 511002110 |  |
| Violet Witch | 511021009 |  |
| Virus Cannon | 513000122 |  |
| Wall of Ivy | 511001522 | makes Ivy Token |
| Wall Shadow | 511000477 |  |
| White Night Dragon | 511001784 |  |
| Wicked Rebirth | 511002989 |  |
| Wild Tornado | 511004111 |  |
| Winged Kuriboh | 511018019 |  |
| Witch of the Black Rose | 511027025 |  |
| Wroughtweiler | 511024001 |  |
| Yubel | 511001391 |  |
| Yubel - Terror Incarnate | 511001392 |  |
| Yubel - The Ultimate Nightmare | 511600424 |  |
| Z-ONE | 511000279 |  |
| Zero Force | 511001873 |  |
| Zoma the Spirit | 511000212 |  |
| Zombie Warrior | 511002615 |  |

## The game's version retained

Reviewed, and kept as the game has them on purpose: a decision with its reason, not something waiting for a fix.

| Card | Why |
| --- | --- |
| Temple of the Kings | Default retained: the anime rules need a reconstruction of Duel Monsters episodes 87 and 89 (which cards the sealed-card effect may seal, the set restrictions, how far the protection reaches, the release costs, when the card is lost). Upstream 513000016 mixes the original, later and printed rules and its cost check is wrong (hand vs field), so it is never enabled. |
| Thousand Dragon | Default retained with Time Wizard, which summons it: the two are reviewed together (Time Wizard's roulette and aging effect differ from the episodes). |
| Time Wizard | Default retained, reviewed with Thousand Dragon: the pinned anime script's roulette wins on 2 of the die's 6 numbers (1 in 3) where the anime's wins 1 in 2, and its aging effect is not the episodes'. Both keep the game's version until a corrected script is written and verified. |

## Held for a technical fix

A card is held back when the profile does not cover a difference between its anime version and the game's card yet (the
client draws a card's type, level, ATK, DEF, attribute and race from its own files and keeps Fusion and Synchro Monsters in the
Extra Deck), or when the research or the script is not settled. Each reason is kept in `engine/anime-cards.json` too.

| Card | Why |
| --- | --- |
| Blackwing - Aurora the Northern Lights | placeholder: the game shows ATK ?, the anime database has 0 for the card whose script sets it |
| Clear Vice Dragon | placeholder: the game shows ATK ?, the anime database has 0 for the card whose script sets it |
| Theinen the Great Sphinx | the script has an effect used from the Deck (EVENT_LEAVE_FIELD), which the client's menus have not been shown to offer<br>flags: -Special Summon only |

## Kept on purpose

| Card | Why |
| --- | --- |
| Alligator's Sword | the anime Normal Monster is equivalent |
| Anthrosaurus | the anime Normal Monster is equivalent |
| Card Guard | upstream entry 511000155 is a different or prize card |
| Fiend's Mirror | upstream entry 511002611 is a different or prize card |
| Hero of the East | the anime Normal Monster is equivalent |
| Kageningen | the anime Normal Monster is equivalent |
| Little D | the anime Normal Monster is equivalent |
| Meotoko | the anime Normal Monster is equivalent |
| Ray of Hope | upstream entry 511000901 is a different or prize card |
| Short Circuit | upstream entry 511000181 is a different or prize card |
| Skull Stalker | the anime Normal Monster is equivalent |
| Stone Armadiller | the anime Normal Monster is equivalent |
| Super War-Lion | upstream entry 511002612 is a different or prize card |
| Wolf | the anime Normal Monster is equivalent |
| Zera the Mant | upstream entry 511002610 is a different or prize card |

Every other card of the game (4,263 of 4,696) has no anime version in the catalogue and keeps its current behaviour.
