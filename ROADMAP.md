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
- [ ] Saving character settings (Confirm; the player update `0x2CEF` carries `avatar1`, `speech%02d`, …)
- [ ] Creating and joining user events
- **Exit:** two headless clients log in as different players, find each other and chat (met 2026-11-06:
  Yugi and Kaiba chat in the Lobby; the room list shows the other player)

## M4: Accounts, collection and economy

- [ ] Real accounts: user id and password, several accounts; persistence (SQLite)
- [ ] Player creation and deletion (`CharMake.cpp`, `0x2CF5/0x2CF6`)
- [ ] Trunk contents: the card collection
- [ ] Deck list, deck editor saves (`HolderSelect.cpp` "deck list")
  - [x] Every player owns a starter deck's cards and gets it in the deck list (`0x1F4E` → `0x22C8`, tag-6
        items); Edit Deck saves it locally and the NPC duel accepts it
  - [x] Deleting a deck (`0x1F50` → `0x22CC`); saving never reaches the server (local files only)
  - [ ] Decks on the server, so they follow a player to another machine (the client only uploads deletes)
- [ ] Shop: booster packs and structure decks from the client's own pack/deck files, BP economy
- [ ] Presents, card trade and rental, if wanted
- **Exit:** create a player, open packs, build a deck, restart the server, everything is still there

## M5: Duels

- [ ] The main screen's DUEL panel (`duelbtnEnable`, `GoDuel`, `DuelWait`): duel invitations or matchmaking
- [ ] NPC duels (Kaiba Corporation): the scene now accepts the starter deck and shows "Select an opponent". The
      roster is `0x2711` (`{npc_type, npc_id}` + `need_ex` sub-items); about 24 NPCs have portraits
      (`data/npc/npcsel*.png`) and 132 `deck_type` 3 presets look like their decks. Still unknown: where an NPC's
      name and details come from (`npc_name`, NPC `PlayerData` with `npc_id` at `+0x400`, `0x51C4F0`) and what
      choosing one sends. Whether CPU duels run inside the client would show how PvP duels work
- [ ] Map the duel server tier (`DuelServerConnection`): how a duel is handed off, ports, handshake. Handoff read
      statically (docs/protocol.md, "Duel start"): `0x1784` → `0x17AC` → `0x1838` → `{host, port}`. Next: answer
      it with a stub duel listener and log what the client sends on the duel connection
- [ ] Confirm in Ghidra that Online 3's duel stream is Konami's engine command set (as in Online 2)
- [ ] Rules engine: ocgcore as a separate process (AGPL), as in YOLaucher's `duelcore`
- [ ] Translator: ocgcore messages ↔ Online 3 duel commands and prompts (`DLG_Text` ids), card ID ↔ passcode map
- [ ] Regulations enforced (2011-era rules; card pool limited to the client's 4,824 cards)
- [ ] PvP between two clients; results and stats written back
- **Exit:** two clients play a full duel with real decks to a finish

## M6: Play with friends

- [ ] Run on a LAN or VPN (`--advertise`), firewall notes (TCP 5730, 1213, 8080, duel port)
- [ ] Release builds (Linux, Windows), a one-page setup guide for friends (`yo3.ini`, Wine or Windows)
- [ ] Backups and a few admin commands
- **Exit:** friends connect from their own machines and duel

## Open questions and guesses (revisit)

| Item | Current choice | Why it's uncertain |
|---|---|---|
| `pbin` size | 512 zero bytes | Guessed from the 4,096 entries of `yo3pbininfo.bin` |
| Starter deck and collection | the first preset of type 1 that regulation 1 allows (preset 2); one copy of each of its cards | The original let new players choose; a real collection comes with the economy |
| Trunk (`9000`) contents | one record: `cards` (the starter deck's), `noTrade` all zero | Other fields later screens may read are unknown |
| `regulations` list | all 20 ids | Which banlists the UI should offer |
| `0x21FC` `lang` | `"1"` | Read as a string; semantics unknown |
| `0x2583/0x271A/0x2710/0x2AFB/0x2AFD/0x27D8` | minimal replies | Meanings unknown; they only stop timeouts |
| `avatar1` length and slots | 41 zero entries (a default male avatar) | Slot meanings unmapped |
| `bgm`, `icon`/`lefttime`/`maintenance`, `npc_id`, `cpuUse`/`notRelease`, `premium` | left out (read as 0 / NULL) | The tracer shows the client reads them; see "Fields the client reads that we leave out" in docs/protocol.md |
| Chat room `region`, `status`, `type`; `0x2458.lang` | 0 / 1 | Shown fine; meanings unknown |
| Whisper `chat_type` values | any type with `to_name` is a whisper | Only room chat (type 1) was seen live |
| Player search paging | skip `base_point`, return at most `front_num` | Only page 1 (`0, 100`) was seen |
| Player search by name | case-insensitive substring | Konami's matching rule is unknown |
| Friend and black list pushes | sent once, right after `9000` | The original timing is unknown; the client only needs them before it lists a player |
| Shop lists, `perk_num`, rental terms | all empty or 0 | Real stock comes with the economy (M4); `perk_num`'s meaning is unknown |
| Event lists (`0x177B`), own user event (`0x33FE`) | empty, 0 | No events are held; paging by `base_point`/`front_num` like player search is assumed |
| Rental records push (`0x2D6C`) | sent once after `9000`, empty | The original timing is unknown; the shop only needs it before it opens |
| Accounts | one per client address | The client sends no credentials; friends behind one address share an account |

## Leads (outside the code)

- Old Online 3 duel logs (`players\<name>\duellogs\`) would be official duel recordings.
- The DuelEvolution Discord reportedly has Online 3 traffic captures (deferred).
- archive.org's `yo3setup_20110728.exe` is three weeks newer than our client; check its protocol version.
