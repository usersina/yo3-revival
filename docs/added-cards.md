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
  Effect, Tuner, Normal, Spells and Traps all work; **Xyz, Link and Pendulum cards can't be added** (the client has no
  such kind and no place on the field for them). ATK and DEF go up to 5100 (or `?`), in tens; Levels 1 to 12.
- **The game doesn't have it already** (a card the game has keeps its row and is only changed through the
  [anime versions](anime-cards.md)).
- **Its script works under the rules the duels use.** Duels run EDOPro's core in Master Rule 2 mode (no Extra Monster
  Zone, no Pendulum Zones). A card from 2013 on may name mechanics the game never had. Read the script
  (`engine/vendor/` after `engine/fetch.sh`) before adding a card that mentions Xyz Materials, Link Arrows, the Extra
  Monster Zone or Pendulum Scales: such a card is a poor fit even when the client could draw it.
- **The pinned data knows it.** A card newer than the pins needs `engine/fetch.sh --update`
  ([development](development.md#the-rules-engine)).

`tools/engine/added-cards.py --find` checks the first two and the stats for you.

## Add one card

1. **Check it fits** and get its line, by its English name (any case):

   ```bash
   tools/engine/added-cards.py --find "Black Rose Moonlight Dragon"
   ```

   It says `ok` with the line for `engine/added/cards.tsv` (the next row, the Konami id, the name), or why the card
   can't be added.
2. **Add the line** to `engine/added/cards.tsv` with a short note after the last tab (who asked, the issue number).
3. **Build the list:** `tools/engine/added-cards.py`. It writes `engine/added-cards.json`, the file the server and the
   launcher read, and refuses anything wrong with the table: a row out of order, a card the game has, a kind the client
   can't show. `--check` compares the committed file with a fresh build (the tests run it).
4. **Add it to the picture list:** `tools/engine/anime-pictures.py` ([its picture](#its-picture)).
5. **Write its scenario test** and check it live ([test it](#test-it)).
6. **Record it:** the card in `docs/features.md`'s row for added cards, and a line in `CHANGELOG.md` in players' words.

The data steps take minutes. Most of the work is step 5: each effect needs a test that plays it.

## Test it

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

The self-played duels of `tests/duel_check.rs` use the game's own cards only, so an added card is covered by its
scenarios and the live check, not by them.

**Live**, on the headless client ([which tool sees what](development.md#which-tool-sees-what)), never the player's own
setup: a scratch server (`task dev:server`), then either

- `yo3-server give-cards --data-dir DIR TestPlayer1 "Card Name"` and Edit Deck (the new-card reveal, the card panel,
  the trunk's row and its search), or
- the server's `--duel-field 0:mzone:ID,1:hand:ID` testing aid (`task dev:server -- -- --duel-field ...`): every duel
  between two players starts with those cards in place, with `--duel-check` on to catch any difference.

Look at: its name, type line, Level, attribute, race, ATK/DEF and text in the card panel; it sits in the Extra Deck
when it should; its summon animation; each effect's dialogs on both clients; no `ERROR duel check` in the server log.

## Its picture

The game draws a card from two pictures: a 200x290 face and a 50x72 thumbnail. **The project ships no card picture**:
like the anime cards', an added card's picture comes from ProjectIgnis' picture server, by its official passcode, on
each player's PC (Desktop's **Card pictures**, [how](anime-cards.md#artwork)). `tools/engine/anime-pictures.py` adds
the added cards to Desktop's picture list (`engine/anime/pictures.tsv`); the list is compiled into Desktop, so a new
card's picture reaches players with the next release.

Until its picture arrives, a card shows the stand-in that does ship, the "Artwork not found" card
(`crates/yo3-launcher/assets/no-picture.png`, the project's own picture). To replace it:
`tools/engine/no-picture.py IMAGE` crops any image to a card's shape and writes it at 200x290.

To see a card's picture on the headless client, point `YO3_ANIME_ART` at a folder with `card/<Konami id>.png`
(200x290) and `mini/<Konami id>.png` (50x72); an empty folder shows the stand-in.

## Its anime version

With the `anime-cards` setting on, an added card can play as its anime version like any card of the game: add its row
to `engine/anime/cards.tsv` (Konami id, name, passcode, anime passcode, `Anime`, a note), run
`tools/engine/anime-map.py` and then `tools/engine/anime-pictures.py` (the picture list then asks for the anime picture,
its official one standing in). Test it with `Scenario::start_anime` or `start_anime_field`, and live with
`yo3-server set anime-cards on`. The [anime cards page](anime-cards.md) has the rules for anime versions.

## Give it to players

No pack holds an added card yet. A host gives it: `yo3-server give-cards PLAYER "Card Name" [--count N]` (or the admin
panel's Accounts page); the player sees it at their next map visit.

## Add a batch

The data side doesn't grow with the batch: one table, one command.

1. `tools/engine/added-cards.py --find` with every candidate's name. It prints each verdict and the lines for the
   ones that fit, in order. Choose the order before adding: rows are permanent ([below](#rules-that-never-bend)).
2. Read each script for mechanics the game never had (Xyz Materials, Link, Pendulum, the Extra Monster Zone) and drop
   those cards.
3. Paste the lines, build, update the picture list (`added-cards.py`, `anime-pictures.py`), and run the tests.
4. Write the scenarios. They are independent of each other, so they can be written in parallel, one card each.
5. One live session for the whole batch: `give-cards` each card, then Edit Deck and a duel or two with
   `--duel-field` placing several of them at once.

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
- **No hand-typed card data.** Everything comes from the pinned data and the player's game. `engine/added-cards.json`
  is generated: edit `engine/added/cards.tsv` and rebuild.
- **No Konami files or artwork in the repository.** The pictures come from ProjectIgnis on each player's PC; the
  only shipped picture is the project's own stand-in.
- **Each effect is tested.** A card goes in with a scenario per effect, as any card effect a player reports does.

## When something goes wrong

| Message or symptom | Cause and fix |
| --- | --- |
| `no game card has its type (0x…)` | An Xyz, Link or Pendulum card, or a kind the client never showed: it can't be added |
| `no card of that name with a Konami id in the pinned data` | The spelling differs from the database's (`--find` ignores case only), or the card is newer than the pins: `engine/fetch.sh --update` |
| `row N, the next is M` | Rows must follow the game's last row without a gap: use the row `--find` prints |
| `ATK … is beyond what the client can show` | Above 5100 or not in tens: it can't be added |
| The card is missing from Edit Deck | The launcher that started the game predates the server's list, or the game was opened without the launcher before its first PLAY: close it and PLAY from the launcher |
| "Artwork not found" in the game | Its picture isn't downloaded yet (Settings, Game, Card pictures), or it isn't in the picture list: run `tools/engine/anime-pictures.py` |
| A spawned card isn't on the field in a scenario | `start_field` takes it from that player's deck or Extra Deck: put a copy there |
| `ERROR duel check` with an added card | The players' views and the engine differ after one of its effects: a scenario that plays that effect will name the step |
