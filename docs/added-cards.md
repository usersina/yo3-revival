# Adding cards the game never had

Yu-Gi-Oh! ONLINE 3 closed in 2012 with 4834 cards. YO3 Revival can add cards printed since then, each played by its
real effects in duels and shown in the game like its own cards: Edit Deck, the trunk, the card panel, the duel field.
The first is **Shooting Quasar Dragon** (#75). This guide is for whoever adds the next ones: one card or a batch.

**Contents**

- [What a card needs](#what-a-card-needs)
- [Add one card](#add-one-card)
- [Test it](#test-it)
- [Its picture](#its-picture)
- [Its anime version](#its-anime-version)
- [Anime-only cards](#anime-only-cards)
- [Infinite ATK and LP](#infinite-atk-and-lp)
- [Give it to players](#give-it-to-players)
- [Add a batch](#add-a-batch)
- [How it reaches the game](#how-it-reaches-the-game)
- [Rules that never bend](#rules-that-never-bend)
- [When something goes wrong](#when-something-goes-wrong)

## What a card needs

Nothing about a card is typed by hand. Its name, text, type, attribute, race, Level and stats come from the pinned
EDOPro card database, its Konami id from the pinned `engine/card-ids.json`, and its effects from ProjectIgnis' card
script (all fetched by `engine/fetch.sh`). The client's own way of writing a card down is learned from the game's cards.
So a card can be added when:

- **The client can show it.** Some game card already has its type, attribute and race. Fusion, Synchro, Ritual,
  Effect, Tuner and Normal monsters work, and every kind of Spell and Trap the game has (Normal, Quick-Play, Continuous,
  Equip, Field and Ritual Spells; Normal, Continuous and Counter Traps); **Xyz, Link and Pendulum cards can't be added**
  (the client has no such kind and no place on the field for them). ATK and DEF go up to 5100 (or `?`), in tens;
  Levels 1 to 12.
- **The game doesn't have it already** (a card the game has keeps its row and is only changed through the
  [anime versions](anime-cards.md)).
- **Its script works under the rules the duels use.** Duels run EDOPro's core in Master Rule 2 mode (no Extra Monster
  Zone, no Pendulum Zones). A card from 2013 on may use mechanics the game never had: Pendulum Call is a Spell the
  client can show, but it works on Pendulum Zones. `--find` reads each card's script and notes the lines that use Xyz
  Materials or Monsters, Link, Pendulum Zones and Scales or the Extra Monster Zone, or a card with no script ("read
  first"). Read those lines: drop the card, or add it with `--checked` when they are harmless (a filter that only skips
  Xyz Monsters). About one card in eight that fits otherwise gets a note.
- **The Tokens it makes are the game's.** The client shows a Token only by a card of its own (the game's 75 Token
  cards, found by name), and `--find` doesn't check this yet: before adding a card whose script calls `Duel.CreateToken`,
  look up the Token's name among the game's. A Token the game never had (Blackwing - Steam the Cloak's Steam Token)
  isn't shown, and the duel check fails on it: such a card waits for token rows ([roadmap](roadmap.md)).
- **The pinned data knows it.** A card newer than the pins needs `engine/fetch.sh --update`
  ([development](development.md#the-rules-engine)).

`task cards:find` (`tools/engine/added-cards.py --find`) checks all of these for you, the script's by its notes.

**A newer card's id.** The client keeps a card's Konami id in 14 bits (16383 at most) and finds a card through a table
that starts at its lowest id (3900). Cards printed from about 2017 on have Konami ids beyond that, so such a card gets a
**stand-in id**: the lowest id from 3900 that no card has (not the game's, no Konami card's in the pinned data, no other
added card's). `--find` picks it and writes it in the line's `stand_in` column; the client, the server and the rules
engine (`engine/added-cards.json`'s `cid`) all know the card by it, and `konami_id` keeps the real one. Everything that
takes a card's id (a scenario test, `--duel-field`, `tools/engine/cardtext.py`, the picture's file name) takes the
stand-in. About 1,200 ids are free that way; when they run out, `--find` says so.

**What the tool also writes.** Edit Deck's **Search Key** filters (Special Summon, Destroy monster, Pierce, Flip
Effect...) read a card's genre bits (`CARD_Genre`). The tool learns them from the game's own cards: each genre bit is
set from the EDOPro categories and type flags that mark it on the game's cards with at least 85% precision (95% over
all), so Shooting Quasar Dragon is found under Special Summon, Cancel out Effect, Destroy monster, Multiple Attack and
Synchro. Bits with no such rule (Recover cards from Graveyard, Hand destruction, Increase/Decrease ATK/DEF, about half
of the game's marks) are left unset. A card's **rarity** (the gold or silver frame of the new-card reveal) is the
table's optional `rarity` column: empty for Common, or `rare`, `super`, `ultra`. The pinned data has no rarity, so
`--find` leaves it empty.

## Add one card

1. **Check it fits**, by its English name (any case), its passcode, or `cid:N` (its Konami id):

   ```bash
   task cards:find -- "Black Rose Moonlight Dragon"
   task cards:find -- 18847598           # a passcode: names that differ between sources, homonyms
   ```

   It says `ok`, or why the card can't be added, and under an `ok` the script lines to read first, if any. It also
   prints the lines both tables will get (the next row, the Konami id, the stand-in id when it needs one; for a printed
   card, the line for `engine/anime/cards.tsv`, which every card a player can hold has) and what
   `tools/engine/anime-map.py` will make of its anime version: mapped, or held and why, with the fix when there is a
   usual one (a "?" ATK the anime database spells 0 needs a corrected record in `engine/anime/records.json`).
2. **Add it**, with a note for the table (who asked, the issue):

   ```bash
   task cards:add -- "Black Rose Moonlight Dragon" --note "asked by Alice (#80)"
   ```

   This writes its line at the end of `engine/added/cards.tsv` and a printed card's line in `engine/anime/cards.tsv` (in
   id order), then runs the three generators: `tools/engine/added-cards.py` (`engine/added-cards.json`, the file the
   server and the launcher read), `tools/engine/anime-map.py` and `tools/engine/anime-pictures.py` (Desktop's
   [picture list](#its-picture)). It writes nothing unless every card fits, nor while a card has notes you haven't
   read (`--checked` once you have). The build refuses anything wrong with the table (a row out of order, a card the
   game has, a kind the client can't show, a Konami id beyond 16383 without a stand-in, a stand-in some card already
   has); a table edited by hand is checked the same way.
3. **Write its scenario tests** and check it live ([test it](#test-it)), then `task check:added`.
4. **Record it:** the card in `docs/features.md`'s row for added cards, and a line in `CHANGELOG.md` in players' words.

The data steps take a minute. Most of the work is step 3: each effect needs a test that plays it.

## Test it

**Without writing anything**, every added card is already covered by two tests: `added_cards_smoke.rs` puts each one on
the field (and a copy in hand) and plays two turns each way through the real core, so a script that errors or a card the
client's view can't follow fails at once; and the duel check's `duels_with_the_added_cards` self-plays duels full of
added cards (anime versions off, then on with the anime-only ones). `task check:added` runs them, with the added
cards' scenarios and the generated files' checks, after a fresh build of the rules engine.

**A scenario test for each effect** in `crates/yo3-server/tests/added_cards.rs`. A scenario plays a duel through the
real rules engine and checks both players' views of the board after every step, the same check every duel gets with
`--duel-check`. Two ways to start:

- **From the game's cards**, when how it is summoned is the point (`Scenario::start` with both decks, then
  `Summon`, `SpSummon`, `Activate`, `Attack`, `Chain`, `EndTurn`): Quasar's test Synchro Summons it from two Ally of
  Justice Catastor and Formula Synchron, all cards of the game.
- **With the board in place** (`Scenario::start_field`, the spawn tool): the card already on the field, in a hand or
  the Graveyard, so one effect is tested without the turns it takes to summon. `Spawn { team, place, cid }` with `place`
  one of `Hand`, `Monster`, `MonsterDefense`, `MonsterSet`, `Spell`, `SpellSet`, `Grave`, `Banished`; a spawned
  card comes from that player's deck or Extra Deck, so put it there.

Choices are steered as a player makes them: `s.prefer(&[(team, cid)])` for the cards picked (targets, discards,
Tributes), `s.prefer_most(true)` to pick as many of them as the choice allows ("up to 2", "any number"),
`s.choose_options(&[i])` for an effect's options ("Select effect to activate"), `s.declare_card(Some(cid))` for a
declared card name, `s.declare_number(Some(n))` for a declared number and `s.prefer_position(..)` for a position.

Each test names the card text it checks, and asserts the result, not only that nothing broke:
`s.attack_of(team, cid)`, `s.count(team, place, cid)`, `s.lp(team)`, and `s.problems.is_empty()` after each step (a
difference between the engine and the players' views).

```rust
let field = [Spawn { team: 0, place: Monster, cid: QUASAR }, Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
let mut s = Scenario::start_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[SHOOTING_STAR]), (&[KURIBOH; 40], &[])], &field).unwrap();
s.run_both([&[EndTurn, Chain(QUASAR)], &[Activate(DARK_HOLE)]]).unwrap();
assert!(s.problems.is_empty(), "{:#?}", s.problems);
assert_eq!(s.attack_of(0, QUASAR), Some(4000), "Dark Hole was negated");
```

The self-played duels play an added card by its scripts at random; a scenario is what proves each effect does what
its text says.

Pitfalls the 2026-11-25 batch's scenarios met (`tests/anime_printed_*.rs`):

- **Only cards a player can hold.** An id the game lacks still runs in the engine; a newer added card goes by its
  stand-in, not its Konami id. `python3 tools/engine/check-test-ids.py FILE...` lists any other.
- **A spawned Fusion, Synchro or Ritual Monster was never properly summoned**, so nothing revives it from the Graveyard:
  summon it in the test when an effect revives it.
- **The anime Kuriboh** (anime cards on) negates an attack on itself and takes its ATK as damage, and offers to reduce
  damage from the hand: a filler deck of Kuriboh changes battles with anime cards on.
- **Check the helper card's own data** (`tools/engine/cardtext.py`): a "Rock" that is a Pyro tests nothing.

**Two patterns that broke duels**, which `task cards:find` points out under "script:" without holding the card back
(`tools/engine/script-risks.py` lists them for every script the engine plays):

- **A first card used unchecked after a cost**: a script checks that a target exists, pays its cost, then takes the first
  card chosen and uses it (`local tc=Duel.GetFirstTarget()` then `tc:IsRelateToEffect(e)`). When the cost leaves nothing
  to choose (the anime Orichalcos Sword of Sealing's discard set off a card that wiped the field), the Lua error stops
  the effect halfway; a live duel logs it and goes on, the self-play fails. Read the lines; when the cost can do that,
  write an override with a check (`engine/overrides/proc_equip.lua`, `c511004015.lua`).
- **An effect that widens a summon's materials** (Synchro Materials from the hand, Ritual Materials from the Graveyard):
  each time the engine lists the summons it searches more combinations, and the anime T.G. Cyber Magician (any monster
  in the hand) froze it past the live server's 30 s. `tests/material_search.rs` puts every such card on a crowded board
  and wants the summons listed within 5 s; `task check:added` runs it. The self-play fails any answer slower than 5 s too
  (`task check:selfplay` for 300 duels).

**Live**, on the headless client ([which tool sees what](development.md#which-tool-sees-what)), never the player's own
setup: a scratch server (`task dev:server`), then either

- `tools/dev/scratch-server.py admin give-cards TestPlayer1 "Card Name"` and Edit Deck (the new-card reveal, the card
  panel, the trunk's row and its search), or
- the server's `--duel-field 0:hand:ID,1:mzone:ID` testing aid (`task dev:server -- -- --duel-field ... --duel-check`):
  every duel starts with those cards in place. Against the computer (an NPC duel, one client) team 0 is you, so the card
  is in your hand from the first turn, whatever the shuffle; between two players team 0 goes first. `--duel-check`
  catches any difference between the game and the engine.

`tools/client/vclient.sh npc` takes the client from the map into a duel against the computer (the talk, the deck, the
janken, going first; `--second`). An added card shows the "Artwork not found" picture until its own arrives (to see
one, [its picture](#its-picture)), so which card is which isn't always visible: `tools/client/vclient.sh duel` lists each side's hand, field and Graveyard by name,
and the question the duel asks now with each choice by name, in the dialog's order (from the scratch server's log, with
`--duel-check` on).

Look at: its name, type line, Level, attribute, race, ATK/DEF and text in the card panel; it sits in the Extra Deck
when it should; its summon animation; each effect's dialogs on both clients; no `ERROR duel check` in the server log.

## Its picture

The game draws a card from two pictures: a 200x290 face and a 50x72 thumbnail. The launcher makes them on the player's
PC, for every card of Desktop's picture list (`engine/anime/pictures.tsv`, which `task cards:add` updates through
`tools/engine/anime-pictures.py`; it is compiled into Desktop, so a new card's place in it reaches players with the
next release). The first of these a card has wins ([which picture](anime-cards.md#artwork)):

1. **The player's own**, in the `pics` folder as EDOPro has it: `pics/<passcode>.jpg` or `.png`, beside `anime-art` in
   the launcher's data folder or beside `yo3.exe` ([players' guide](launcher.md#card-pictures)). It works for every
   card of the list, an added card included, by its anime passcode first and then its official one.
2. **A face the player put in `anime-art/card` by hand** (200x290, named by the id the client knows).
3. **The project's own pictures**, shipped in the launcher: the owner's artwork for the manga Gods and the anime Divine
   Serpent Geh, as 200x290 faces (`crates/yo3-launcher/assets/pics/`, made by `tools/engine/shipped-pictures.py`,
   listed in `crates/yo3-launcher/src/art/pics.rs`'s `SHIPPED`). A picture goes there only when the owner gives it.
4. **A downloaded picture**: ProjectIgnis' picture server, by the card's passcode (Desktop's **Card pictures**,
   [how](anime-cards.md#artwork)). The project hosts none of these.
5. **The stand-in**, "Artwork not found" (`crates/yo3-launcher/assets/no-picture.png`, the project's own picture).
   `tools/engine/no-picture.py IMAGE` crops any image to a card's shape and writes it at 200x290 to replace it.

To see a card's picture on the headless client, point `YO3_ANIME_ART` at an empty folder of your own (never the
player's): the launcher fills it with the shipped pictures and with any `pics/<passcode>.png` beside it (the folder's
parent's `pics`), so a picture of the card there shows it. A picture already made at 200x290 can go in
`card/<id>.png` instead (the id the client knows: the Konami id or the stand-in id).

## Its anime version

With the `anime-cards` setting on, an added card can play as its anime version like any card of the game: add its row
to `engine/anime/cards.tsv` (Konami id or stand-in id, name, passcode, anime passcode, `Anime`, a note), run
`tools/engine/anime-map.py` and then `tools/engine/anime-pictures.py` (the picture list then asks for the anime picture,
its official one standing in). Test it with `Scenario::start_anime` or `start_anime_field`, and live with
`yo3-server set anime-cards on`. The [anime cards page](anime-cards.md) has the rules for anime versions.

## Anime-only cards

A card never printed, one the anime showed (**Orichalcos Gigas**, the first, row 4835), can be added too. It has no
Konami id and no official data: its data and script are ProjectIgnis' anime ones (the pinned unofficial database and
`unofficial/`), and it plays only with the `anime-cards` setting on.

- **Find it** by name or `anime:PASSCODE`: `tools/engine/added-cards.py --find "Orichalcos Gigas"`. It must be marked
  Anime in the unofficial database (not a video game or manga version, not an anime version of a printed card: those are
  [anime versions](#its-anime-version)) and have a script that registers effects, not a placeholder. The line has an
  empty Konami id, a stand-in id and the passcode in the `anime` column.
- **Its text** starts with the `[Anime]` line, like every anime card. Its genre bits come from its script's effects and
  its type (the unofficial database numbers its categories its own way).
- **With anime cards off** it stays in trunks and decks, and every duel with a deck holding it is refused at Select Deck
  (Auto Duel, NPC duels, hosted duels, events), whatever the banlist, with a chat line: "Orichalcos Gigas is an anime
  card: this server has anime cards off, so a deck with it can't duel." `give-cards` gives it, and says so. The rules
  engine loads it only with the setting on.
- **Review it like an anime version**: its script is the community's reconstruction of what the anime showed; a card
  whose anime text is unknown waits. Test it with `Scenario::start_anime` or `start_anime_field`.

## Infinite ATK and LP

ProjectIgnis' scripts give an infinite ATK as 9,999,999. The engine makes ∞ one value (`crates/yo3-engine/src/guards.lua`):
any LP, ATK or DEF of 9,999,999 or more becomes exactly 19,999,998, which finite changes leave as it is (+1000, -700,
half, double), while an "ATK becomes X" or "LP becomes X" still applies. So a new card scripted that way has it with
nothing more to do: ∞ on the field (the connection DLL), a tie with another ∞, its battle damage and effect damage of
9,999,999 or more take every LP (∞ LP too), Avatar matches it rather than going 1 above, and Dreadroot leaves it alone.
A card that takes or gives LP as ATK (Ra's "pay all but 1", Granel) turns ∞ LP into ∞ ATK the same way
(`tests/infinite_lp.rs`). Two kinds need a look:

- **A printed ∞** (the card itself says "ATTACK ∞", like the anime Divine Serpent Geh): its script can only give it as
  ATK "?" and an effect, which Skill Drain would negate and other cards would change. Add its passcode to
  `PRINTED_INFINITY` in `crates/yo3-engine/src/guards.lua`: it then keeps ∞ in a Monster Zone whatever negates its
  effects, other cards' ATK changes don't touch it, and its original ATK is ∞ too (white on the field, where an effect's ∞
  is blue; the card profile's "?" accepts it). Its DEF stays an ordinary number. Test it as
  `tests/manga_gods.rs` tests Geh (`skill_drain_leaves_a_printed_infinity`, `other_cards_leave_a_printed_infinity`).
- **An effect that makes an ATK ∞** (Obelisk's Soul Energy Max, a spell "its ATK becomes ∞"): read how the script sets
  it. A final value (`EFFECT_SET_ATTACK_FINAL`, as Soul Energy Max) is what the card keeps; an amount added
  (`EFFECT_UPDATE_ATTACK`) is ∞ too while the total is 9,999,999 or more, and other cards' finite changes leave it. Give it a
  scenario test with Dreadroot, The Seal of Orichalcos and Avatar beside it, and add it to the Gods' self-play pool
  (`duel_check.rs` `duels_with_the_gods`) if it meets the Gods.

`task cards:find` points these cards out under "at ∞", without holding them back: a script that sets LP, works an amount
out from LP, or makes an ∞ of its own (9,999,999), and a card whose ATK is "?" with an ∞ in its script (maybe a printed
∞). Before adding one:

- **Script ∞ as 9,999,999**, in an override too: the engine makes anything from there up its one ∞. Never `math.huge` or
  `0x7fffffff`: the core works in 32 bits, and a sum past them wraps into a negative ATK or LP.
- **A printed ∞ ATK goes in `PRINTED_INFINITY`** (above). A printed ∞ DEF has no rule yet: ask before adding one.
- **No loop once per LP point.** A list as long as the LP is fine (Bark of Dark Ruler builds 200,000 entries at ∞, and a
  declaration of more than 255 numbers reaches the player as 255, up to the largest), a loop over every LP point is 20
  million steps.
- **Give it a case at ∞** in `crates/yo3-server/tests/infinite_lp.rs`: the card played with ∞ LP or against an ∞ ATK,
  what the LP and ATK become, and `clean` (both players' views). The rules to check it by are above: a finite change
  leaves ∞, an amount that is itself ∞ moves as is, "becomes X" is X, ∞ against ∞ ties. If it is a card many decks will
  hold, add it to `duel_check.rs` `duels_at_infinite_lp`'s pool too (both players at ∞, self-played).

## Give it to players

No pack holds an added card yet. A host gives it: `yo3-server give-cards PLAYER "Card Name" [--count N]` (or the admin
panel's Accounts page); the player sees it at their next map visit.

## Add a batch

The data side doesn't grow with the batch: one table, one command.

1. `task cards:find -- --find-file FILE`, FILE listing every candidate (a name, passcode or `cid:N` per line,
   `#` comments). It prints each verdict and the lines for the ones that fit, in order, each newer card with its own
   stand-in. Choose the order before adding: rows and stand-ins are permanent ([below](#rules-that-never-bend)).
2. Read the lines each "read first" note names, and drop the cards that need a mechanic the game never had.
3. Add the ones you keep with `task cards:add -- --add-file FILE --note "the batch, #issue"` (the same file, cut down;
   `--checked` when you read every noted script and kept the cards anyway), then `task check:added`.
4. Write the scenarios. They are independent of each other, so they can be written in parallel, one card each.
5. One live session for the whole batch: `give-cards` each card, then Edit Deck and a duel or two with
   `--duel-field` placing several of them at once (an NPC duel needs one client), `vclient.sh duel` naming them.

A batch of about twenty cards of one era, mostly Synchro monsters, is about half a day to a day, almost all of it
scenarios. Simple cards (no effect, or one effect of a kind the game already has) are minutes each.

## How it reaches the game

1. **The server** adds the rows after the game's own (`yo3_data::added`: `CardDb::add`), so decks, trunks, trades,
   duels and the rules engine all know the cards, and serves the list at `/web/added-cards`.
2. **The launcher** fetches that list before the game is released, builds the game's card files with it from the
   player's own game (`yo3_data::added::extend_files`: every original row unchanged, the new rows after them, in every
   language the game has; an added card's text is its English text in each), and keeps them in `added-cards/<hash>/`
   beside its profile with the cards' pictures (or the stand-in). A later start with the same list reuses the folder.
   Nothing in the game folder is written.
3. **The connection DLL** hands the game those files in place of its own when it loads its card data, and the
   pictures in place of `data/card/<id>_<art>.png` and `data/mini/` (`tools/mod/added-cards-logic.h`). The game's
   loader grows its arrays to fit, so the number of added cards has no fixed limit
   ([internals](client-internals.md#data-formats)).

So a new card needs a server update (its row and its script) to be played. Its picture's place in the picture list
comes with the next Desktop release; until then it shows the stand-in.

## Rules that never bend

- **Rows are permanent.** Decks, trunks and saves store a card's row. A row is never reused, reordered or removed: a
  card added by mistake stays (it can be left out of every pack and never given).
- **Stand-in ids are permanent** too: the game's pictures, regulations and logs name a card by its id. If a later
  update of the pinned data gives a real card a stand-in's id, `added-cards.py` refuses to build until that is sorted
  out; the added card keeps its id.
- **No hand-typed card data.** Everything comes from the pinned data and the player's game. `engine/added-cards.json`
  is generated: edit `engine/added/cards.tsv` and rebuild.
- **No Konami files or artwork in the repository.** Downloaded pictures come from ProjectIgnis on each player's PC; the
  only shipped pictures are the project's own (the stand-in, and the owner's artwork in `crates/yo3-launcher/assets/pics/`).
- **Each effect is tested.** A card goes in with a scenario per effect, as any card effect a player reports does.

## When something goes wrong

| Message or symptom | Cause and fix |
| --- | --- |
| `no game card has its type (0x…)` | An Xyz, Link or Pendulum card, or a kind the client never showed: it can't be added |
| `no card of that name, passcode or Konami id in the pinned data` | The spelling differs from the database's (`--find` ignores case only: try the passcode), or the card is newer than the pins: `engine/fetch.sh --update` |
| `Konami id N is beyond the client's 14 bits` | A newer card's line without its stand-in: take the line `--find` prints |
| `stand-in N is a card of the game` / `already a card's Konami id` | A stand-in must be an id no card has: take the one `--find` prints |
| `N is no anime card (ot 4, no alias)` / `registers no effect (a placeholder)` | An anime-only card must be the unofficial database's anime card itself, with a working script |
| A deck is refused with "… is an anime card: this server has anime cards off" | It holds an anime-only card: take it out, or turn `anime-cards` on |
| The card is missing from one of Edit Deck's Search Key filters | Its genre bits are learned from the EDOPro categories (below, [what a card needs](#what-a-card-needs)): about half of the game's marks have such a rule, the others (Recover cards from Graveyard, Hand destruction...) are never set for an added card |
| `read first: Pendulum: ... lines N` (or Xyz, Link, the Extra Monster Zone) | Its script uses a mechanic the game never had: read the lines; drop the card, or add it with `--checked` when they are harmless |
| `nothing added: ...` | `task cards:add` writes all the cards or none: drop the ones that can't go in, or read their notes and add `--checked` |
| `no stand-in id is left` | Every id from 3900 to 16383 is taken: no newer card can be added |
| `row N, the next is M` | Rows must follow the game's last row without a gap: use the row `--find` prints |
| `ATK … is beyond what the client can show` | Above 5100 or not in tens: it can't be added |
| The card is missing from Edit Deck | The launcher that started the game predates the server's list, or the game was opened without the launcher before its first PLAY: close it and PLAY from the launcher |
| "Artwork not found" in the game | Its picture isn't downloaded yet (Settings, Game, Card pictures), or the player has none in `pics`, or it isn't in the picture list yet: `task cards:add` runs `tools/engine/anime-pictures.py`, and the list reaches players with the next Desktop release |
| A spawned card isn't on the field in a scenario | `start_field` takes it from that player's deck or Extra Deck: put a copy there |
| `the engine has card 0, the client nothing` in a self-played duel | A Token the game has no card for ([what a card needs](#what-a-card-needs)): leave the card out until token rows exist |
| `ERROR duel check` with an added card | The players' views and the engine differ after one of its effects: a scenario that plays that effect will name the step |
