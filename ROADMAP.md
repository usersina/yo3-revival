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

## ▶ M2: An instrumented client

Stop finding missing fields one crash at a time.
- [ ] Field-read tracer: a 32-bit DLL (mingw-w64) that hooks the field getters (`0x409020`, `0x4090F0`,
      `0x40F1D0`, `0x410D60`) and the `request` getter (`0x40F290`). It logs `request id, field, type, found/missing,
      caller` to a file.
- [ ] Load it from `vclient.sh start --trace` (a proxy DLL picked up through a `WINEDLLOVERRIDES` native override)
- [ ] A summary script: for each reply id, the fields the client read and the ones it found missing
- [ ] Use GhidraMCP to name the functions we understand (handlers, getters, dispatchers) as we go
- **Exit:** one session prints every field each reply was asked for, with the missing ones flagged

## M3: A social world

- [ ] Answer `0x2775` (presents?) and any further main-screen requests
- [ ] Chat rooms ("select a chatroom" at the top left) and chat messages
- [ ] Entering map locations, lobbies and rooms
- [ ] Sessions registry: who is online, in which room; player search returns real players
- [ ] Two clients at once (a second Wine prefix or display) for multiplayer tests
- [ ] Friends, mail and teams (as far as the main screen needs)
- **Exit:** two headless clients log in as different players, find each other and chat

## M4: Accounts, collection and economy

- [ ] Real accounts: user id and password, several accounts; persistence (SQLite)
- [ ] Player creation and deletion (`CharMake.cpp`, `0x2CF5/0x2CF6`)
- [ ] Trunk contents: the card collection
- [ ] Deck list, deck editor saves (`HolderSelect.cpp` "deck list")
- [ ] Shop: booster packs and structure decks from the client's own pack/deck files, BP economy
- [ ] Presents, card trade and rental, if wanted
- **Exit:** create a player, open packs, build a deck, restart the server, everything is still there

## M5: Duels

- [ ] Map the duel server tier (`DuelServerConnection`): how a duel is handed off, ports, handshake
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
| Trunk (`9000`) contents | one empty record | Which fields later screens read is unknown (M2 will tell) |
| `regulations` list | all 20 ids | Which banlists the UI should offer |
| `0x21FC` `lang` | `"1"` | Read as a string; semantics unknown |
| `0x2583/0x271A/0x2710/0x2AFB/0x2AFD/0x27D8` | minimal replies | Meanings unknown; they only stop timeouts |
| `avatar1` slots | zeros | Slot meanings unmapped |

## Leads (outside the code)

- Old Online 3 duel logs (`players\<name>\duellogs\`) would be official duel recordings.
- The DuelEvolution Discord reportedly has Online 3 traffic captures (deferred).
- archive.org's `yo3setup_20110728.exe` is three weeks newer than our client; check its protocol version.
