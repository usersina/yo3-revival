# Roadmap

Goal: play Yu-Gi-Oh! ONLINE 3 with friends on our own server. Each milestone ends in something checked against the
real client (`tools/client/vclient.sh`), not just against tests. Tick boxes in the same commit as the work.

## ✅ M0: Research and tooling

- [x] Feasibility study ([docs/research.md](docs/research.md))
- [x] Wire crypto and message format; the client's data files decode with the same codec
- [x] Headless client on Xvfb with a virtual mouse; the client's own debug trace captured
- [x] Annotated disassembly, RTTI map, reply-dispatch router, Ghidra headless export
- [x] GhidraMCP (headless) registered for the project

## ✅ M1: Stable world

- [x] Gate redirect, version check, auth, login, player list, player select
- [x] World entry (`0x1FA4` → `9000`)
- [x] Every main-screen poll answered; news list; player search; regulations
- [x] Rust server (`crates/`), test-first, at parity on the real client
- **Exit:** a cold start reaches a stable main screen with no crash and no timeout dialog (met 2026-11-06)

## ✅ M2: An instrumented client

Stop finding missing fields one crash at a time.
- [x] Field-read tracer: a 32-bit DLL (mingw-w64) hooking the one lookup behind every field getter
      (`std::map::find` at `0x41A980`) and list-item reads (`0x410D60`). It logs the reply being parsed, field,
      getter type, found/missing, value type (from RTTI), the asking function and top/sub level.
- [x] Loaded by `vclient.sh start --trace` through an injector, so the game folder is never modified
- [x] `tools/tracer/summarize.py`: per reply, the fields read and the ones missing, with the asking function
- [x] First catch: `0x2775` (its reply later corrected to `0x2778`), `0x2CF7.result_number`, and the read-but-absent
      fields in `docs/protocol.md`
- [x] Code hits: `YO3_TRACE_HITS=0x…,0x…` logs when chosen client addresses run (int3 and a vectored handler); it
      found the startup step that kept the map locked
- Standing practice (moved to CLAUDE.md): name functions in GhidraMCP as we understand them
- **Exit:** one session prints every field each reply was asked for, with the missing ones flagged (met 2026-11-06)

## ▶ M3: A social world

- [x] Answer `0x2775` (next present: `0x2778` = none, which closes PresentWindow and unlocks the map and menu)
- [x] Chat rooms (the chat panel's top-left button) and chat messages: room list, enter, leave, room chat
      with echo, whispers
- [x] Sessions registry: who is online, in which room (`Hub`); sessions leave on disconnect
- [x] Accounts by client address (the client never sends credentials), up to four players each
- [x] Two clients at once: `YO3_INSTANCE=b` with its own display and reflinked Wine prefix
- [x] Player search returns the players online, filtered by name, level or id; both clients list each other
      (needed the friend and black list pushes, `0x251C`/`0x252A`)
- [x] Map locations: the map unlocks, and each of its six facilities opens and returns to the map (rooms are the
      chat rooms above; the DUEL panel belongs to duels, M5)
  - [x] Shop: opens to the Shop Menu with empty shelves and returns to the map (rental records pushed at entry)
  - [x] NPC duel: opens (Kaiba Corporation), says there are no valid decks, returns to the map (the trunk now has
        `cards`/`noTrade` arrays, all zero)
  - [x] Character settings: Avatar Settings opens with a default avatar and returns to the map (`avatar1` needs
        41 entries); saving changes is not handled yet
  - [x] Official Event (Stadium) and Championship (tournament map): open, report no valid decks, return
  - [x] User event (Satellite Plaza): lists no events; creating one is not handled yet
- [ ] Friends, mail and teams (as far as the main screen needs). The friend and black lists are pushed empty;
      adding friends needs their requests mapped (`0x2528`, `0x252C`, `0x252E` carry `to_id`)
- [ ] Player search criteria we ignore: language, country, manner level, status, community, event
- [ ] Main menu screens (the menu opens now): Card Album, Rental Card List, My Profile, D-Mail, Duel Log, Team,
      Friend List, Ignore List, Leaderboard, BOOSTERPASS, Duel Options (Edit Deck is M4)
- [x] Saving character settings: Avatar Settings' Confirm sends `0x2CEF` {avatar1, speech00–22, comment00–01,
      title} → `0x2CF5`; kept with the account and sent back in the player record. Live: purple hair after a restart
- [ ] Creating and joining user events
- **Exit:** two headless clients log in as different players, find each other and chat (met 2026-11-06:
  Yugi and Kaiba chat in the Lobby; the room list shows the other player)

## M4: Accounts, collection and economy

- [ ] Real accounts: user id and password, several accounts; persistence (SQLite)
- [ ] Player creation and deletion (`CharMake.cpp`, `0x2CF5/0x2CF6`)
- [x] Trunk contents: the card collection (the starter deck's cards and the player's bought cards, `0x1FA4` → `9000`)
- [ ] Deck list, deck editor saves (`HolderSelect.cpp` "deck list")
  - [x] Every player owns a starter deck's cards and gets it in the deck list (`0x1F4E` → `0x22C8`, tag-6
        items); Edit Deck saves it locally and the NPC duel accepts it
  - [x] Deleting a deck (`0x1F50` → `0x22CC`); saving never reaches the server (local files only)
  - [ ] Decks on the server, so they follow a player to another machine (the client only uploads deletes)
- [x] Shop: every booster pack of the client's pack files on the shelves (`0x1C22`), bought with BOOSTERPASS Points
      (`0x1B68` → `0x1C24`), opened as presents (`0x2775` → `0x2777` with the cards); BP and bought cards kept per
      account and player in `data/accounts.json`. Live: three Collection Pack vol.1 opened card by card, and after a
      server restart the shop showed 4700 BP and the pack 21.4% collected
- [x] BP from duels: each counted result pays into the account (win 100, draw 50, loss 30), which the login replies
      and every purchase re-read. Live: a lost NPC duel took 5000 to 5030, shown at the next login; a pack then left 4930
- [x] Structure decks: SD001–SD005 (the art-carrying `deck_type` 1 recipes) on the STRUCTURE DECK tab, bought as
      type-1 presents with the deck id; the deck's cards join the collection. Live: GADGET BOX (deck 502) for 500 BP,
      shown as its 40-card grid, then 41 cards (with its extra deck) in the account
- [x] Bought structure decks in the deck list (`0x22C8`), under their names from the client's text table
      (`data/text/txttbl_E.bin`, now read by `yo3-data`), kept per player. Live: GADGET BOX was saved locally as
      "GADGET BOX.y3d", listed in Load Deck and loaded with its 40 + 1 cards
- [ ] Protectors, avatar items, card trade and rental
- [ ] Presents, card trade and rental, if wanted
- **Exit:** create a player, open packs, build a deck, restart the server, everything is still there

## ✅ M5: Duels

- [x] The main screen's DUEL panel: the menu bar's duel button opens the Duel Menu (`0x17FC` counts); Auto Duel asks
      for a match (`0x1798`), and the server pairs two players waiting for the same rule format (push `0x1978` with
      the opponent and the duel). Live: two clients matched and saw each other
- [x] Create Duel: a hosted duel shows on its host in player search (Duel Conditions, Send Duel Request); the
      offer reaches the host, who accepts or declines; passwords, a busy host and a withdrawn offer are handled; the
      duel starts with the host's LP. Live: Kaiba challenged Yugi's hosted duel and both reached the janken
- [x] Match Mode: best of three on one duel connection (1500 starts the next duel's janken, `0x2009` carries the
      match result); Auto Duel's Both matches either rule format. Live: Kaiba won a hosted match 2–0
- [x] Duel clocks between players: the timers message (`0x14`) drives each side's turn and total clocks; the server
      runs them while the engine waits on a player and ends the duel when one runs out (reason 11). Live: in an Auto
      Duel Kaiba's clock ran down from 180 while Yugi's stood still, and an idle player lost: "Yugi has ran out of
      time.", then the match went on to its next duel
- [x] Watching duels: Player Information's Watch Duel on a duelling player (mode 4): the front sends the duel
      server and both players (`0x1838` → `0x1839`), the duel server catches the spectator up from seat 0's side
      (janken, setup, the duel so far, both hands hidden) and then sends every event, and at the match's end 1505
      with the result. Live: Joey watched Yugi and Kaiba's Auto Duel and went back to the map when Kaiba quit
- [ ] Card Swap between a match's duels (1504, side deck); Create Duel's Friend/Team limits
- [x] NPC roster and duel start: 26 NPCs (ids 120–197 from the NPC deck presets) listed by name with their
      levels; picking one runs `0x1784` → `0x17AC` → Select Deck (`0x1F4F`) → `0x17D4`, and the duel screen opens
      with both players. The client builds NPC duels itself (DuelManager `0x50ECF0`); no duel server is involved
- [x] Duel server handoff: after `0x17D4` the server pushes `0x22C4` (`duelData {host, port}` + `rule`), the
      client connects to our duel listener (port 1214, logged only) and sends `0x200A {player_id, autoSet}`
- [x] Duel connection mapped (both directions' `duelRequestName`s, the engine command pump) and the janken played
      against the server as the CPU (commands 1500–1503): the duel field opens, Turn 1
- [x] Duel opening: setup `0x15` with both decks' sizes, duel start, LP 8000 (`0xF`), the first player's Draw Phase
      (5); the engine commands are mapped to the components that handle them (docs/protocol.md)
- [x] Cards: both shuffled decks go into the engine's card table (command 1000) and each player draws five (1000 +
      the card move `0x1A`); the opening hand shows, the CPU's face down
- [x] Player actions mapped: phases, the command map `0x12`, the input wait 4, the menu's 14 command bits and the
      client's card command (`duelRequestName` 2), confirmed live with a test build
- [x] Rules engine sidecar: EDOPro's core (ocgcore-wasm, rule MR2) in `engine/duelcore.ts` under Node, one process
      per duel, JSON lines, cards as Konami ids (4,702 of the client's 4,771 map to passcodes through YGOProDeck's
      `konami_id`; the rest look like tokens); `engine/fetch.sh` fetches the core, ProjectIgnis' scripts and database
- [x] Translator, first part: the engine drives the duel session; its draws, turns and phases become engine commands,
      the client's idle prompt a command map of the legal commands, the client's card commands its responses; the CPU
      ends its turns at once (a placeholder bot) and other prompts are answered with defaults
- [x] Card moves: the engine's MOVE becomes a card-table entry plus `0x1A`; place prompts are answered for the client
      (first free zone). Live: a Normal Summon from the hand
- [x] Phase buttons: Main Phase prompts enable BP and EP as the engine allows (command 4 mode 2 + other flags); a
      pressed button ends the phase. Live: summon, EP, the CPU's turn, our Turn 3 with BP and EP lit
- [x] Battle: the battle prompt offers the attackers (Attack) and M2/EP; damage, costs and recovery change the LP
      shown. Live: a direct attack, 8000 → 5700
- [x] A placeholder CPU: it Normal Summons (or Sets) the first monster it may, attacks with everything, never chains;
      other prompts get simple answers (first cards, yes, first option), also for the client's prompts without a window.
      Two such AIs play a full engine duel to a win (test). Live: the CPU summoned, attacked and destroyed our monster
      (LP 8000 → 7350; it went to the graveyard pile)
- [x] Field stats: a monster entering a monster zone gets its record (data commands `0x3E9`–`0x3EB`) with CARD_Prop's
      ATK/DEF, level, attribute and race; the field label and card info show them
- [x] The duel's end on screen: command 2 {result, reason} plays the win/lose/draw movie with its reason; the front
      push `0x2009` (duel data, tag 7) ends the match, so the result screen shows the record against this opponent and
      Next returns to the map. Live: "YOU LOSE", LOSER/WINNER panels, back at Kaiba Corporation
- [x] Players kept across server restarts (accounts and records in `data/`, M6)
- [x] The client's own card choices: targets, tributes, attack targets and other card selections open the card list
      (command `0x37` and its data commands) with a message from the client's duel prompts. Live: an attack target
- [x] Yes/no and effect questions: the Yes/No dialog (command `0x36` kind 2) with the card's name. Live: Marauding
      Captain's optional effect, then its Special Summon choice. Testing aid `--duel-deck ROWS`
- [x] Positions, options and chains: the position dialog (kind 8), the option list (kind 4) for options the client has
      labels for (coin calls, card types), and the chain question (kind 3, with the cards) then the card list. Live:
      Marauding Captain's Special Summon in Defence, Time Wizard's coin call, two Trap Holes chained to the CPU's summon
- [ ] Options that are a card's own strings (51 of the client's cards; the prompt table's 400–490 are the original
      per-card labels, but EDOPro's strings don't pair with them reliably), places, number/race/attribute declarations
      (the Number, Type and Attribute dialogs); chains outside timely chances (EDOPro's "always chain")
- [x] Position changes and flips on the field: command `0x1C` (CardFlipTurn) for MSG_POS_CHANGE and flip summons; the
      command ids named after Konami's view types (Master Duel's `DuelViewType`). Live: Giant Orc to defense after
      attacking, a Set Trap Hole turning face-up
- [x] Battle and summon animations: BattleInit/Attack/Run/End (`0xA`–`0xE`, figures and destruction per player)
      and RunSummon/RunSpSummon (`0x38`/`0x39`). Live: the NPC's attack cut-in, the hit, the destroyed card going,
      summon sparkles
- [x] Chain link animations: ChainSet `0x33` {player, zone, link} at each MSG_CHAINING and ChainRun `0x34` {_, _,
      link} at each MSG_CHAIN_SOLVING. Live: two Trap Holes chained to the CPU's summon showed links 1 and 2, the
      chain banner (`duel_chain.swf`), and each number fading as its link resolved
- [x] Destruction by effects: CardVanish `0x1F` {player, zone | index << 8} before the card's move, for MSG_MOVE's
      reason DESTROY without BATTLE (the sidecar reads the reason back, since ocgcore-wasm drops it). CardBreak `0x20`
      only hides a card's label. Live: Trap Hole shattered Sangan, then the duel went on
- [x] Stat changes from effects: the sidecar reports the monster zones after each engine step when they changed, and
      a face-up monster's record (`0x3E9`–`0x3EB`) goes again, which the client re-reads after every command. Live: Axe
      of Despair took Giant Orc's label and card info to ATK 3200 (in blue)
- [ ] Attack targets on the field instead of the list; counters (`0x27`)
- [ ] Deck piles on the field (the deck count alone shows none)
- [ ] NPC experience (`npcbin`) so higher levels unlock; what `0x17D4` reports; mode 4 (`0x1838` → `{host,
      port}`, started at `0x4D3066`)
- [x] Online 3's duel stream is Konami's engine family but renumbered (turn change 6, LP set `0xF`; docs/protocol.md)
- [ ] Regulations enforced (2011-era rules; card pool limited to the client's 4,824 cards)
- [x] PvP on the duel connection: both matched clients share one table (`table.rs`): the janken between their hands,
      the winner's turn order, one engine with a translator per client (each sees itself as player 0), each prompt to
      the client it asks, the opponent's hidden cards without rows, a player leaving loses. Results reach both front
      connections. Live: Yugi against Kaiba, Giant Orc's direct attack, WINNER / LOSER screens, both back on the map
- [x] Players' records written back (`data/records.json`, M6)
- **Exit:** two clients play a full duel with real decks to a finish (met 2026-11-07: two headless clients matched
  through Auto Duel and played their starter decks to a win)

## ▶ M6: Play with friends

- [x] Run on a LAN or VPN: `--bind 0.0.0.0`; the gate and the front send each client to the address it reached
      (`--advertise` only for port forwarding); firewall notes (TCP 5730, 1213, 1214, 8080). Live: a client through
      the LAN address and one through loopback, two accounts, a duel to a win between them
- [x] Accounts kept in `data/accounts.json` and duel records in `data/records.json`, backed up at each start; new
      players' names made unique (Duelist, Duelist2…)
- [x] Admin commands: `accounts`, `add-player ADDRESS NAME` (name a friend's player before they connect), `rename`
- [x] Linux release bundle (`tools/release.sh`: server, engine sources, `run.sh`) and the setup guide for the host
      and the players (`docs/friends.md`: `yo3.ini`, Windows or Wine, LAN / VPN / port forwarding)
- [ ] A native Windows server build (needs `rustup target add x86_64-pc-windows-gnu`; a Windows host can use WSL)
- [x] Chain questions follow the player's Activation Message option (Manual / Semi Auto Cancel / Auto Cancel): the
      server sends a level per chance (3 after an event, 1 at a phase, 0 when nothing fits). Live: no question at the
      CPU's phases, the question at its summon
- **Exit:** friends connect from their own machines and duel (the network path is verified on one machine: a client
  through the LAN address against one through loopback, 2026-11-07; real friends' machines not tried yet)

## Open questions and guesses (revisit)

| Item | Current choice | Why it's uncertain |
|---|---|---|
| `pbin` size | 512 zero bytes | Guessed from the 4,096 entries of `yo3pbininfo.bin` |
| Starter deck and collection | the first preset of type 1 that regulation 1 allows (preset 2); one copy of each of its cards | The original let new players choose; a real collection comes with the economy |
| Trunk (`9000`) contents | one record: `cards` (the starter deck's), `noTrade` all zero | Other fields later screens may read are unknown |
| `regulations` list | all 20 ids | Which banlists the UI should offer |
| `0x21FC` `lang` | `"1"` | Read as a string; semantics unknown |
| `0x2583/0x271A/0x2AFB/0x2AFD/0x27D8` | minimal replies | Meanings unknown; they only stop timeouts |
| `avatar1` length and slots | 41 zero entries (a default male avatar) | Slot meanings unmapped |
| `bgm`, `icon`/`lefttime`/`maintenance`, `npc_id`, `cpuUse`/`notRelease`, `premium` | left out (read as 0 / NULL) | The tracer shows the client reads them; see "Fields the client reads that we leave out" in docs/protocol.md |
| Chat room `region`, `status`, `type`; `0x2458.lang` | 0 / 1 | Shown fine; meanings unknown |
| Whisper `chat_type` values | any type with `to_name` is a whisper | Only room chat (type 1) was seen live |
| Player search paging | skip `base_point`, return at most `front_num` | Only page 1 (`0, 100`) was seen |
| Player search by name | case-insensitive substring | Konami's matching rule is unknown |
| Friend and black list pushes | sent once, right after `9000` | The original timing is unknown; the client only needs them before it lists a player |
| Shop lists besides packs, `perk_num`, rental terms | all empty or 0 | Only booster packs are stocked; `perk_num`'s meaning is unknown |
| Pack prices and contents | 100 BP a pack, at most 10 per purchase; 5 cards: 4 of the pack list's first ascending run, 1 of a later run (each run half as likely as the one before), read as rarity tiers | The original's prices, pack size and odds are unknown; the runs match the OCG's rarity split (44/18/9/9) |
| Starting BP | 5000 per account (`START_BP`) | The original's amounts are unknown |
| Structure decks | SD001–SD005 only (the starters 1–3 have no art); 500 BP each, at most 3 per purchase, sale id 20000 + deck id; the owned-items bit (`pbin_no`) the client sets isn't kept by the server | The original's prices are unknown; nothing read so far depends on the bit for decks |
| BP from duels | win 100, draw 50, loss 30 per counted result (NPC duels and duels between players; a match counts once) | The original's duels paid no BP (no reply after a duel sets it); the client shows the new total at its next login or purchase |
| Pack presents | one per pack, `present_id` counting from 1 per session, `level`/`form` 0, `item_img` and `end_time` empty; the cards join the collection at purchase | Only the fields the window reads; presents aren't kept across reconnects (the cards are) |
| Event lists (`0x177B`), own user event (`0x33FE`) | empty, 0 | No events are held; paging by `base_point`/`front_num` like player search is assumed |
| Rental records push (`0x2D6C`) | sent once after `9000`, empty | The original timing is unknown; the shop only needs it before it opens |
| NPC roster `npc_type`, `need_ex` | every NPC on tab 0; `need_ex` = the level (0, 1, 2) | No client file gives the tabs (probably the series) or the experience each level needs; `need_ex` can't all be 0 (`0x4BFDC4`) |
| `join_id`, `duel_id` | server-wide counters from 1 | The client only echoes them back |
| `0x17D4` | answered `0x17D5`, nothing kept | Sent once the duel object is ready; may also carry results later |
| `0x22C4` push | right after `0x17D5`; `rule` = the fields of the duel's `0x17AC` item | The original timing and rule contents are unknown; only `match_type` is read so far |
| Server duel messages | request `0x200D` | The pump reads only `duelRequestName` (and needs the id non-negative) |
| CPU janken | random hand; the CPU goes first when it wins | The original CPU's choices are unknown |
| Duel setup `PlayerType001/002` | 0 (client) / 1 (CPU) | Meaning unknown |
| Card table entries (1000) | param3 0, flag byte 0; uids 1.. in table order | Meanings of param3 and the flag unknown |
| Hidden cards | against another player, the opponent's deck, hand, extra deck and face-down cards go with row 0 and face-down monsters get no field record; against the CPU, real rows | Online 2 used placeholders, whose value is unknown; a hidden card that flips without moving keeps row 0 until it moves (position changes aren't translated yet) |
| Automatic matching | the first two players waiting for the same `match_type`, Both (6) with either and two Both players in a Single duel; the level choice is ignored; the second player's `0x1978` goes a second after its `0x1799` | Konami's matching rules are unknown; the delay assumes the agent reads one message per step |
| Match Mode | best of three; every duel opens with a janken; a drawn duel counts for neither; no Card Swap (1504) between duels; the records count the match once | Online 3's rules for who goes first in later duels, draws and the side deck are unknown |
| Hosted LP and `--duel-lp` | the testing aid wins over a hosted `lp_start` when it isn't 8000 | A testing convenience, not the original's |
| Duel Menu counts | `duel_playing` = 2 × matched duels, `duel_ready` = hosted duels, `duel_auto` = players waiting | Whether they count players or duels is unknown |
| Hosted duels | `duel_status` 1 while hosting, 3 while in a duel between players; time limits and Friend/Team limits not applied; `0x17C0` answered `0x17C1` | The original values and the limits' rules are unknown |
| PvP setup | `PlayerType001/002` 0/1 as against the CPU | Their meaning is unknown |
| A player leaving | the other wins (command 2 {1, 14 "disconnected"}) and both records count it | The original's handling (time-outs, `0x2724` heartbeats) is unknown |
| Menu bits for Set | 6 monster set, 7 Spell/Trap set | Both entries read "Set" |
| Card list kind | 1 for min 0, `0x27` for min 1, else 6 with max = min | The kinds' other meanings (title, sounds) are unknown; a choice between 2 and more picks is narrowed to exactly min |
| Card list marks | `0x8000` for the CPU's cards, no icons | The low bits draw icons whose meaning is unknown |
| Choice messages | EDOPro's hint mapped to the client's text for 7 hints; none otherwise | The original server's message per choice is unknown |
| Yes/no questions | 'Activate the effect of "X"?' for optional effects, 'Use the effect of "X"?' for other questions | The cards' own question strings (EDOPro's) aren't in the client |
| Attack targets | chosen in the card list | The original probably picked them on the field (DuelBattle) |
| Position dialog | button 0 face-up attack (face-down if only that), button 1 face-up defense when allowed, else face-down; the flag shows the face-up defense picture | The dialog has two buttons; ocgcore allows any mix of four positions |
| ChainRun `param1`/`param2`, ChainSet's place for a card off the field map | 0; the field's centre (16) | DuelChain reads neither; the original's values are unknown |
| Destruction by effects | CardVanish (`0x1F`, the shatter) before the move; CardBreak (`0x20`) never sent | Which of the two the original sent, and when, is unknown; CardVanish is the one with an effect |
| MSG_MOVE's reason | read back by a query at the card's new place after each processing step | A card that moves again within the same step reports its later reason |
| Stat changes | level, ATK, DEF, base ATK, base DEF from the engine; race and attribute stay CARD_Prop's | The client's race and attribute numbering isn't ocgcore's; type-changing effects are rare in the card pool |
| Duel clocks | turn clock full again at each of the player's turns, both clocks running while the engine waits on that player (also during client animations), a time-out 5 s after 0; Auto Duel 180 s / 600 s, hosted duels their `time_turn` / `time_total` (0: off), NPC duels none | The original's clock rules and Auto Duel's limits are unknown; the clocks follow Create Duel's own defaults |
| Watching | from seat 0's side (the first player to enter); the decisive janken round only; NPC duels can't be watched | The original's spectator view and opening are unknown; a spectator's view of a match's later duels wasn't tried live |
| Login banlist | regulation 1 ("Beta Forbidden/Limited") unless `--regulation` names another | Which list the original used for Auto Duel at each date is unknown; the client calls the login one "Latest Regulation" |
| Option labels | EDOPro's system strings 60/61 and 70–72 mapped to the client's labels; a card's own strings take the first option | The original server's label per card option is unknown |
| Chain chances | level 3 after an event (summon, attack, damage… or during a chain), 1 at a phase or step, 0 when no card fits (`spe_count` 0); the player's Activation Message option decides; messages 7/8 (Spell/Trap zone)/269 | The original server's levels and which windows it judged "low priority" are unknown |
| Rules | EDOPro's `MODE_MR2` | Online 3 (2010) ran Master Rule 2; EDOPro's preset may differ in details |
| CPU play | first summonable monster, attack with all, never chain | The original NPCs' AI is unknown |
| Duel end reason | EDOPro's reason mapped to the client's texts (LP 0, deck out, surrender, Exodia...; others "won with a card") | The original server's values are unknown (100 takes another path) |
| Duel's end push `0x2009` | sent when the engine ends the duel; `duel_end` 1; no `score_<id>`, no `dlogURL` | The original timing and `duel_end` values are unknown (2 and 3 show an abnormal-end dialog) |
| Result screen counts | `win`/`lose`/`draw` = totals against this opponent including this duel, `*_add` = this duel's change; kept in memory | The client only shows them ("Wins 1 (+001)") |
| Duel port | 1214 | The original's port is unknown; the client takes whatever `duelData` says |
| Accounts | one per client address | The client sends no credentials; friends behind one address share an account |
| Player names | the starting names (`--players`, default Duelist) made unique with a number; the host names players with `add-player` / `rename` | The original made players on Konami's website; the client can't create one (CharMake is the appearance editor) |

## Leads (outside the code)

- Old Online 3 duel logs (`players\<name>\duellogs\`) would be official duel recordings.
- The DuelEvolution Discord reportedly has Online 3 traffic captures (deferred).
- archive.org's `yo3setup_20110728.exe` is three weeks newer than our client; check its protocol version.
