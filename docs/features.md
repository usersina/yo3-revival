# Features

What works today, area by area, and how we know. For what is open next read the [roadmap](roadmap.md); for the dated
records behind a row, the [history](history.md); for request and reply shapes, [protocol](protocol.md). Player-facing
controls are in the [playing guide](playing.md#features-and-limitations).

**Status** is one of:

- **Implemented**: usable end to end, and something in the "Verified by" column shows it.
- **Partial**: works in part, or a handler exists but the feature is not seen working end to end; Notes say what is missing.
- **Not implemented**: the client can show it, we do not build it yet.
- **Unsupported**: the client cannot do it, or it is out of scope. The evidence is in
  [Client limits and what the DLL changed](#client-limits-and-what-the-dll-changed).

**Verified by** names the test file (`crates/yo3-server/tests/`, `crates/yo3-desktop/tests/` or `tools/mod/*-test.c`), a
live check on the private headless client (`tools/client/vclient.sh`), the Windows 11 VM, the duel check
(`duel_check.rs`: the client's model compared with the engine after every step, over self-played duels), or the
monkeys. "Not verified" means nothing did. Test files check what the client reads from a reply, not that a player enjoys
it; a live check is a person or script driving the real client. GitHub issue numbers are in Notes.

**Since** is the first release in which the row worked as described, with its date. Work done before the first
release counts as v0.0.1. A row whose feature was broken until a fix (Avatar Settings) gives the release of that fix,
and later changes are in Notes and the [changelog](https://github.com/usersina/yo3-revival/blob/main/CHANGELOG.md). "Unreleased" marks work on `main` not yet in a
release, and "—" a row that is not implemented or unsupported.

**Contents**

- [Features](#features)
  - [Accounts and login](#accounts-and-login)
  - [Map and menus](#map-and-menus)
  - [Shop and economy](#shop-and-economy)
  - [Deck editor and collection](#deck-editor-and-collection)
  - [Duels: rules and engine](#duels-rules-and-engine)
  - [Duels: presentation](#duels-presentation)
  - [NPCs](#npcs)
  - [PvP and matching](#pvp-and-matching)
  - [Watching and duel logs](#watching-and-duel-logs)
  - [Social: chat, friends, mail, teams, trade, rental](#social-chat-friends-mail-teams-trade-rental)
  - [Events and tournaments](#events-and-tournaments)
  - [Avatars and titles](#avatars-and-titles)
  - [Website links](#website-links)
  - [Desktop app](#desktop-app)
  - [Scripts and command line](#scripts-and-command-line)
  - [Connection DLL and direct start](#connection-dll-and-direct-start)
  - [Platforms](#platforms)
  - [Client limits and what the DLL changed](#client-limits-and-what-the-dll-changed)

## Accounts and login

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Authenticated TLS login by account key, single-use tickets | Implemented | v0.0.4 (2026-11-10) | `auth_flow.rs`, `auth_contract.rs`, `metadata_contract.rs`, `flow.rs`; native and Wine DLL tests; live; Windows VM | TLS port 7443 is the only game listener. No password login exists: the client never sends one. |
| Host-made accounts, private profiles, up to four players per account | Implemented | v0.0.4 (2026-11-10) | `accounts.rs`, `manager_contract.rs`, `cli_options.rs`; live | `add-account`, `profile`, `add-player`, `rename`, `move-account` in [guide](guide.md). The old `/join` page is retired (`join.rs` checks 404). |
| Create and delete players (Select Player) | Implemented | v0.0.1 (2026-11-09) | `player_contract.rs`; live | Four slots per account; names unique ignoring case (a guess in the guesses table). |
| One server per data folder | Implemented | v0.0.19 (2026-11-15) | `data_lock.rs` | v0.0.19: a second server cannot open the same data folder. |
| Account/player changes rolled back when a save fails | Implemented | v0.0.19 (2026-11-15) | `save_failures.rs` | v0.0.19. Other stores, such as mail and duel-result records, still log write failures without rolling back. |
| Server-side deck check and regulation at Select Deck | Implemented | v0.0.1 (2026-11-09) | `regulation_contract.rs`, `deck_contract.rs` | Refused with `0x22CB`. Default banlist Season 16, set in Desktop or with `--regulation` (#15). |
| Country flags | Partial | v0.0.1 (2026-11-09) | `country_contract.rs`; live | The client sends no country, so the host sets it per account (`set-country`) or it follows the login language. |
| Login news | Implemented | v0.0.1 (2026-11-09) | `front_contract.rs` | The news no longer includes the deck-onboarding notice. |

## Map and menus

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Main screen polls, map unlock, startup steps | Implemented | v0.0.1 (2026-11-09) | `flow.rs`, `front_contract.rs`, `startup_contract.rs`; live on Linux and the Windows VM | A cold start reaches a stable main screen (milestone M1). |
| Shop, NPC duel, Avatar Settings and Satellite Plaza tiles | Implemented | v0.0.1 (2026-11-09) | `menu_contract.rs`, `event_contract.rs`; live | Each opens and returns to the map. |
| Official Event and Championship tiles | Partial | v0.0.1 (2026-11-09) | `event_contract.rs`; live | They open and report no valid event (see Events and tournaments). |
| Main Menu: Rental Card List, My Profile, Duel Log, BOOSTERPASS, Friend List, Ignore List, D-Mail, Team | Implemented | v0.0.1 (2026-11-09) | `menu_contract.rs`, `bp_contract.rs`, `friends_contract.rs`, `mail_contract.rs`, `team_contract.rs`; live | Card Album and Duel Options need nothing from the server. |
| Auto Card Placing OFF (choose the zone for each monster) | Implemented | v0.0.18 (2026-11-15) | `duel_contract.rs`, `duel_stream_contract.rs`; live | Shipped in v0.0.18. |
| Map tiles UNIZAWA and market (`btn5`, `btn6`) | Unsupported | — | Client search | No handler in `main.swf`. |

## Shop and economy

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Booster packs for BP | Implemented | v0.0.1 (2026-11-09) | `shop_contract.rs`, `bp_contract.rs`; live | Pack prices and the rarity runs are guesses (guesses table). 5000 BP at the start of an account. |
| BP from duels (win 100, draw 50, loss 30), BOOSTERPASS codes, `give-bp` | Implemented | v0.0.2 (2026-11-09) | `bp_contract.rs`, `shop_contract.rs`; live; Windows VM (a mid-duel BP gift) | `give-bp`, `add-code`, `remove-code`, `codes` also work while the server runs. |
| Structure decks SD001-SD005 | Implemented | v0.0.1 (2026-11-09) | `shop_contract.rs`; live | The starters 1-3 have no art and are not on sale. |
| Protectors and avatar items for BP | Implemented | v0.0.1 (2026-11-09) | `shop_contract.rs`; live |  |
| Trade Card (Blister's exchange pools) | Implemented | v0.0.13 (2026-11-14) | `shop_contract.rs` (`the_exchange_cards_stay_off_the_pack_shelves`); live | #8, done 2026-11-14. Fixed in v0.0.22 (#46): the exchange's cards also sat on Purchase Pack as 10 BP "packs" ("Pack 4") whose purchase failed with "An error occurred."; live 2026-11-16 (headless client): the shelves hold the 75 packs at 100 BP in both sort orders, Trade Card lists and exchanges. |
| Shop rental (5 cards, 100 BP, 7 days) | Implemented | v0.0.13 (2026-11-14) | `shop_contract.rs`, `rental_contract.rs`; live | #8. Returned early with the Return button (see Social). |
| Special Packs | Unsupported | — | Client search | Event prizes whose contents are not in the client. The list is kept empty on purpose (#8). |
| Presents between players | Unsupported | — | Client search | No screen builds a send request. Cards move by trade and rental; the host gives BP. |

## Deck editor and collection

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Collection (trunk), the starter deck | Implemented | v0.0.1 (2026-11-09) | `deck_contract.rs`, `front_contract.rs`; live |  |
| Official character decks on Load Deck's Official tab | Implemented | v0.0.12 (2026-11-13) | `original_decks.rs`, `deck_contract.rs`; live | Nine decks (Yami Yugi, Jaden, Yusei) plus CONCEALERS DECK. |
| Decks follow the player to another machine (Select Deck and Edit Deck saves) | Implemented | v0.0.12 (2026-11-13) | `deck_contract.rs`; live on two machines; Windows VM (an Edit Deck save reached the server) | Edit Deck saves need the connection DLL. A same-named deck on another machine is not replaced (playing guide). |
| Delete a deck; bought structure decks in the deck list | Implemented | v0.0.1 (2026-11-09) | `deck_contract.rs`; live |  |
| Side deck and Card Swap between match duels | Implemented | v0.0.1 (2026-11-09) | `duel_stream_contract.rs`; live | Dragging cards needs the hover-then-press method in AGENTS.md. |
| Forbidden and Limited marks in Edit Deck | Implemented | v0.0.14 (2026-11-14) | Live in the game (v0.0.14) | Follows the banlist chosen. |

## Duels: rules and engine

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Native rules engine (EDOPro core, MR2), one process per duel | Implemented | v0.0.10 (2026-11-13) | `engine.rs`, `engine_cards.rs`, `duel_check.rs`; `tools/engine/parity.py` against the old sidecar | #3. No Node.js since v0.0.10. |
| Card data validation and seeded duel sweeps | Implemented | v0.0.9 (2026-11-12) | `card_data.rs`, `card_sweep.rs` (4,684 cards), `original_decks.rs` (2,270 duels) | The sweep adds each card to the decks and checks engine/client-model consistency. A pass does not require that card to be summoned or activated, or verify the native UI. |
| Card effects match their 2010–2012 text and rulings | Partial | v0.0.13 (2026-11-14) | `engine_cards.rs`, `engine/overrides/`; card reports (#7) | Overrides restore historical behavior and fix known script bugs. Not every effect or ruling has been checked. |
| Duel check: the client's view against the engine after each step | Partial | v0.0.1 (2026-11-09) | `duel_check.rs`, `duelcheck_model.rs` (240 self-played duels); native checkpoints logged by the DLL; live on two clients; summon order rules (v0.0.20, `summon_order.rs`: Konami's 228 duel logs keep them); a position dialog must name a card the client has (v0.0.21) | Complete menu and effect coverage and automatic UI replay are not done (roadmap). |
| Choice windows: targets, tributes, positions, options, declarations, sums, orders | Partial | v0.0.1 (2026-11-09) | `translate_contract.rs`, the duel check's round trip, the card sweep; live for those seen; a token's position (v0.0.21: `tokens.rs`, the 60 tokens client cards make; live, Gorz the Emissary of Darkness), `position_dialog.rs` (the dialog shows a copy of its card the client can see) | Built wherever the client has a window (history: Duel windows). A few answers are automatic because the client cannot show them (limits table). Fixed in v0.0.23: a position dialog no longer shows a blank card when an opponent hides another copy (reported 2026-11-16: Doom Shaman from an NPC's Graveyard). |
| Chain questions follow Activation Message | Implemented | v0.0.1 (2026-11-09) | `duel_contract.rs`; live |  |
| Duel clocks (180 s turn, total clock, run-out loses) | Implemented | v0.0.1 (2026-11-09) | `clock_contract.rs`; live | The settle-marker clock (#11) needs the DLL; `--duel-time 0,0` turns clocks off. |
| Match Mode (best of three), Single, host LP | Implemented | v0.0.1 (2026-11-09) | `pvp_contract.rs`, `duel_contract.rs`; live |  |
| Borrowed and traded cards in decks | Implemented | v0.0.1 (2026-11-09) | `regulation_contract.rs`, `rental_contract.rs` | A deck that used a card no longer owned is invalid until edited. |
| Xyz and Link Monsters | Unsupported | — | No card data in the client | CARD_Prop has 0 Xyz of 4,702. Materials stay handled. |
| Tag duels, Extra Deck reveals, cards removed from the duel, reloaded fields | Unsupported | — | Scripts searched | No card of the client makes them; they would fail the duel check if they came. |

## Duels: presentation

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Summon, battle, chain and destruction animations | Implemented | v0.0.1 (2026-11-09) | `duel_stream_contract.rs`, `translate_contract.rs`; live | Tribute, Fusion, Synchro and Flip Summons as the original showed them (v0.0.17); a Fusion with its materials in the hand no longer crashes the client (v0.0.20, #40: live with King of the Swamp). The original CardBreak shatter is not used; CardVanish stands in. |
| Duel dialogs in the card's own words | Implemented | v0.0.13 (2026-11-14) | `duel_stream_contract.rs`, `tools/mod` dialog logic; live; Windows VM | #10, needs the DLL. A card string as a hint and a Yes/No line were not seen live. |
| Duel music follows the LP gap (3000 LP) | Implemented | v0.0.1 (2026-11-09) | Live (Disadvantage track at 5000 against 8000) | #14. The client does it itself; choosing the opening track is not built. |
| Coin tosses and dice rolls shown (the coin's face and the call, the die's face) | Implemented | Unreleased (`beta` branch) | `translate_contract.rs` (`a_coin_toss_shows_its_coins_on_the_tossing_players_side`, `a_called_coin_toss_wins_on_the_called_side`, `each_die_rolls_on_the_rolling_players_side`); live 2026-11-16 in NPC duels: Time Wizard and Sasuke Samurai #4 (called coins), Graceful Dice (a 5) | Reported by a player: the toss and the roll showed nothing. [Protocol](protocol.md#duel-connection). |
| What targets what: markers on hover between related cards | Partial | v0.0.1 (2026-11-09) | `translate_contract.rs`; duel check (84 equips, 121 lasting targets); live (Axe of Despair on Vorse Raider, both ways) | Equips and lasting targets only. Swords of Revealing Light has no engine target, and what the other six marker types meant is unknown (#16). |
| An NPC's face-down cards hidden | Implemented | v0.0.11 (2026-11-13) | `yusei_contract.rs`, `npc_bot.rs`; live | Fixed 2026-11-13. |
| Position change at the end of the Battle Phase drawn sideways | Partial | v0.0.1 (2026-11-09) | Live, six tries | One upright case on an owner's screen was seen and not reproduced; the duel check agrees with the engine. |
| Higher resolution (4:3, sharper) | Partial | v0.0.13 (2026-11-14) | `tools/mod/resolution-test.c`; live on Linux; Windows VM | #1. Not applied when `yo3.exe` is opened directly. A Japanese or Korean IME and a second monitor are untested. True 16:9 is out of scope. |

## NPCs

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| NPC roster (25 named NPCs), levels, experience | Implemented | v0.0.1 (2026-11-09) | `duel_contract.rs`, `npc_bot.rs`; live |  |
| Duel bot (look-ahead within and through the turn) | Implemented | v0.0.10 (2026-11-13) | `npc_bot.rs` (against the old CPU), `duel_check.rs`; live | Plans only around cards you show. The heroes (Yugi, Jaden, Yusei) always draw what they need. |
| NPC lines in the duel chat, hero skills | Implemented | v0.0.10 (2026-11-13) | Unit tests in `npc.rs` and `banter.rs`; live | Lines are ours (`npc_lines.txt`). |
| NPC duel logs | Implemented | v0.0.13 (2026-11-14) | `tools/mod/` logic; live; Windows VM | #12, needs the DLL. |

## PvP and matching

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Auto Duel pairing (rule format) | Implemented | v0.0.1 (2026-11-09) | `matching_contract.rs`; live | Two clients matched and played to the end. |
| Create Duel, passwords, Opponent's Limits (Friend, Team) | Implemented | v0.0.1 (2026-11-09) | `pvp_contract.rs`, `friends_contract.rs`, `team_contract.rs`; live |  |
| A duel between two players on one table | Implemented | v0.0.1 (2026-11-09) | `pvp_contract.rs`, `duel_check.rs`; live; Windows VM (Windows against Linux) | Both views are kept in step by the duel check. |
| Manner ratings after a player duel | Implemented | v0.0.1 (2026-11-09) | `manner_contract.rs`; live | Not asked after NPC duels or trades (the client has none). |
| Players on separate machines over a LAN or the internet | Partial | v0.0.1 (2026-11-09) | LAN address against loopback (2026-11-07); Windows VM against Linux | Real friends' independent PCs, separate Windows machines and internet connections are not yet checked (roadmap). |
| Duel robustness: engine death, disconnects, unresponsive hosts, crafted traffic | Partial | v0.0.1 (2026-11-09) | `robustness_contract.rs`; protocol monkey and soak runs; live server kill and freeze | A closed connection returns to the title. A frozen server that keeps TCP connections open still leaves the duel waiting without an error ([roadmap](roadmap.md#open-now)). |

## Watching and duel logs

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Watch Duel on a duelling player | Implemented | v0.0.1 (2026-11-09) | `duel_stream_contract.rs`, `pvp_contract.rs`; live | A duel closed to watchers cannot be watched by going around the menu (v0.0.19). |
| Duel logs of player duels (Latest Duel Logs, replay) | Implemented | v0.0.1 (2026-11-09) | `duel_stream_contract.rs`; live | `duellog.rs`; a match keeps its last duel only. |
| Duel logs of NPC duels | Implemented | v0.0.13 (2026-11-14) | See NPCs | #12. |

## Social: chat, friends, mail, teams, trade, rental

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Chat rooms, whispers, room list, team chat | Implemented | v0.0.1 (2026-11-09) | `chat_contract.rs`; live |  |
| Player search by name, level, id, language, chat room, manner, status | Implemented | v0.0.1 (2026-11-09) | `search_contract.rs`; live | The events criterion lists nobody. |
| Friends and the ignore list | Implemented | v0.0.1 (2026-11-09) | `friends_contract.rs`; live |  |
| D-Mail, team mail | Implemented | v0.0.1 (2026-11-09) | `mail_contract.rs`, `team_contract.rs`; live | Mail waits for offline players. |
| Teams | Implemented | v0.0.1 (2026-11-09) | `team_contract.rs`; live | The team ranking page is empty (see Website links). |
| Card trade between players | Implemented | v0.0.1 (2026-11-09) | `trade_contract.rs`; live |  |
| Card rental between players (1, 3, 7, 14 days) | Implemented | v0.0.1 (2026-11-09) | `rental_contract.rs`; live, including a term's end | A Deck rental (three lists) was not seen live. |
| Return a rental early | Implemented | v0.0.15 (2026-11-14) | `tools/mod/rental-return-test.c`; live on two clients; Windows VM | #13, needs the DLL. There is no confirmation question. |

## Events and tournaments

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| User events (Satellite Plaza): create, list, join, lobby, bracket, knockout | Implemented | v0.0.1 (2026-11-09) | `user_event_contract.rs`; live with three players (a bye and a final) | Open events survive a restart; a running one ends at a restart. The client has no Ready/Standby request. |
| Official Events (Stadium): Gunslinger, Duelist Trial, Evolution and other matches | Not implemented | — | `event_contract.rs` (the empty list only) | #18. The tile opens and reports no valid event. Research is in the history; planned for a public server (roadmap). |
| Championship (Tournament Map) | Not implemented | — | `event_contract.rs` (the no-event path only) | #17. Needs a host-scheduled `event_type` 10 event. |
| Leaderboard and team ranking from real results | Not implemented | — | Not verified | The pages exist but are empty (see Website links). Convenience features are tracked in #16. |
| Official-event and Y.O.C titles | Not implemented | — | Not verified | They wait for those events. |

## Avatars and titles

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Avatar Settings: save, parts lists, a new look reaching others at once | Implemented | v0.0.16 (2026-11-14) | `character_contract.rs`; live on two clients (v0.0.16) | An avatar must have the game's own size. |
| Millennium Eye against Pegasus | Implemented | v0.0.16 (2026-11-14) | Unit tests in `npc.rs`; live with and without the Eye |  |
| Titles earned by play (160 in the catalogue; thresholds are ours) | Partial | v0.0.18 (2026-11-15) | `titles_contract.rs`; live (RECKLESS DUELIST) | Some titles are not earned yet (roadmap). Staff and promotion titles come from `grant-title`. |
| Event and Championship titles | Not implemented | — | Not verified | With those events. |

## Website links

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Links to Konami's old website open the host's own `/web/` pages | Implemented | v0.0.18 (2026-11-15) | `tools/mod/web-links-test.c`; live (Leaderboard, Official Website); pages in `web.rs` | v0.0.18, needs the DLL (MOD_ABI 3). The pages say what this server has. |
| Leaderboard, team ranking and BOOSTERPASS page contents | Partial | v0.0.18 (2026-11-15) | Live | Placeholder text only; no data (roadmap, later). |

## Desktop app

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Local play, join a server, host friends, activity, invitations | Implemented | v0.0.4 (2026-11-10) | `connections.rs`, `networking.rs`, `preferences.rs`, `server_options.rs`, `direct_install.rs`; live on Linux; Windows VM | Native Windows 10 is not verified. |
| Application updates (release check, download, activation) | Partial | v0.0.4 (2026-11-10) | `update_check.rs`, `update_install.rs`, `update_activation.rs`; packaged-update test; Windows VM | The first published GUI-to-GUI update and native Windows UAC are not verified. |
| Bug reports and diagnostic exports | Partial | v0.0.7 (2026-11-12) | `reports.rs`, `bug_evidence.rs`; Wine | Real GitHub sign-in and publishing were tested only with mocks. |
| Banlist and Resolution settings | Implemented | v0.0.13 (2026-11-14) | `server_options.rs`, `preferences.rs`; live | #15, #1. |

## Scripts and command line

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Server, accounts, profiles, admin commands, `play.sh` and `play.cmd` | Implemented | v0.0.1 (2026-11-09) | `cli_options.rs`, `manager_contract.rs`; live; Windows VM (Scripts bundle) | See [guide](guide.md) and [scripts](scripts.md). |
| Headless client and the monkeys | Implemented | v0.0.1 (2026-11-09) | `tools/client/test_vclient.py`; monkey runs | Developer tools (`vclient.sh`, `protocol-monkey.sh`, `ui-monkey.py`). |

## Connection DLL and direct start

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Connection DLL: login, ordinal imports, local cookies, fail-closed injection | Implemented | v0.0.4 (2026-11-10) | `tools/mod/test.sh`, `test-loader.sh`; Windows VM | Required. A failed hook byte check stops the game. |
| Opening the game directly (WINMM loader) | Partial | v0.0.4 (2026-11-10) | `tools/mod/test-direct.sh`, `direct_install.rs`; Windows VM, standard user | Protected-folder installation and reinstalling a missing runtime are not verified; Windows 10 is not checked. |
| Exit guard at teardown | Implemented | v0.0.13 (2026-11-14) | `tools/mod/teardown-test.c`; live | Fixed after a crash mid-duel on Windows. |
| Native game exit `0xc0000005` from a player's recording | Partial | — | Not verified | Not reproduced in seven native Windows 11 launches; a crash dump is needed. |

## Platforms

| Feature | Status | Since | Verified by | Notes / issues |
| --- | --- | --- | --- | --- |
| Linux with Wine | Implemented | v0.0.1 (2026-11-09) | Live on the private headless client; all tests | The development platform. |
| Windows 10 and 11, 64-bit | Partial | v0.0.1 (2026-11-09) | Windows 11 VM (build 22000) | Windows 11 passed for Desktop and Scripts. Windows 10 and protected folders are not verified. |
| Windows 7 | Unsupported | — | Out of scope | Do not pursue unless reopened. |
| macOS | Unsupported | — | No supported build | See [launcher](launcher.md). |

## Client limits and what the DLL changed

What the stock 2010 client can't do, whatever the server sends, with the evidence (details in [protocol](protocol.md)
and [client internals](client-internals.md)). Each was searched for, not assumed. Our required connection DLL already
patches the client (the duel dispatcher, the text lookup, the Flash invoke), so the last column says what it would
take: most of these are only out of reach for a server talking to the stock client.

| What | Why the stock client can't | Evidence | Today | With the DLL or the server |
| --- | --- | --- | --- | --- |
| Server text inside a duel dialog | Every dialog message is built from entries that name the client's own texts, card names by Konami id or numbers | `0x519870` (entry kinds 1-8), Select dialog labels through it (`0x4C5880`); the Flash `setMessage` is fed only by the C++ | Done with the DLL (#10): the server's texts as prompt ids past the table's end (`0x40BB20`) | — |
| Clocks that wait for the animations by themselves | The client never says when it has played a batch, and counts down regardless | Duel requests 1-15 (`0x49F990`'s 16 senders), DuelCockpit `0x502660` | Done with the DLL (#11): the client answers a settle marker; the allowance is the fallback | — |
| NPC duel logs | The client fetches a log only for duels against players | protocol.md, "Duel log files" (MainGamen `+0x804`) | Done with the DLL (#12): the jump removed, the server keeps NPC duels' logs | — |
| Returning a rental early | The rental screen's Return button has no C++ callback, and the shipped movie has no button either | protocol.md, `0x481D70`; `rentalrecord.swf` places no `borrow_btn1` | Done with the DLL (#13): it attaches the movie's `btn_main01` as the button and registers the callback; the server takes `0x7A14 {rental_id}` | — |
| Music changing with the duel | Nothing missing: the client switches by itself at a 3000 LP lead (Advantage) or deficit (Disadvantage); only BgmUpdate (command 9) has no handler | DuelCockpit `0x5040F0` on LP commands `0xF`/`0x10`; no case 9 in the duel components or the pump `0x522A50` | Works: live, the Disadvantage track at 5000 against 8000 (#14) | Choosing a duel's opening track (`duel_id % 8`, `event_type` 5) is server-side, not built |
| Three or four positions in one choice | The position dialog has two buttons (face-up attack; face-up or face-down defense) | `dlg_selstand.swf` (`btn0`, `btn1` only), `0x4C5E50` (`param3 >> 16` the only switch) | The two the prompt allows; none came up in 1,200 self-played duels | Server-side: two dialogs in a row. Not done: no card of the client's needs it |
| Several Types declared at once | The type dialog stops at the first pick | `dlg_seltype.swf` `onItemClick`, `0x4C5B50` (`selMax` 1) | Answered automatically | Server-side: one dialog per Type. Not done: no card of the client's asks (0 scripts) |
| Numbers beyond the number dialog | max 255 of a unit, min and step at most 15 | `0x4C6430` (8 and 4-bit fields), formats 302/303 | Answered automatically | Server-side: a coarser unit or several dialogs. Not done: no card of the client's asks |
| The original's destruction effect (CardBreak) | It only drops the zone's label; the shatter came from BattleRun's destroy bit, which the engine sets before a protection is known | `0x4FF000` case `0x20`; Konami's logs (578, battle only) | CardVanish's shatter | Server-side: hold BattleRun until the destruction is known. Not done: the shatter already shows |
| Extra Deck reveals, cards taken out of the duel, a reloaded field, tag swaps | Not a client limit: no card of the client's makes them | Scripts searched: Duel.ConfirmExtratop, RemoveCards, ReloadField (0 of 4,702); tag duels aren't played | They fail the duel check if they ever come | Nothing to do |
| Password logins | The client never serializes a user id or password | protocol.md, "Accounts" | TLS profiles per account | Not needed |
| The client choosing its country | No request carries it | protocol.md, "Country (flags)" | The host sets it per account (`set-country`) | Not needed |
| Presents between players | No screen builds a send-present request | protocol.md, "No presents between players" | The host gives BP (`give-bp`) and codes | A new screen is out of scope; chat commands could do it server-side |
| Ready/Standby in events | No request builder for it exists; the lobby's buttons are the official tournament's entry menu | protocol.md, "Ready/Standby" (`official_event_lobby.swf`, ids `0x3390`-`0x339B`, `0x32C9`-`0x32D5` searched) | The host's Start is the only gate | Possible but not worth it: the host's Start does the job |
| Xyz Monsters | The client has none: no card data, no art | CARD_Prop against cards.cdb: 0 Xyz of 4,702 | Xyz materials stay handled and logged | Out of reach: it would mean new Konami content |

Rows that the table above marks "Done with the DLL" were once Unsupported: the stock client could not do them, and the required connection DLL (`tools/mod/`) now does. They are Implemented in the area tables (#10 dialog text, #11 clocks, #12 NPC duel logs, #13 rental return). "Music changing with the duel" never needed a fix: the client does it by itself (#14).
