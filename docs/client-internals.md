# Client internals

What we know about `yo3.exe` and its data. Addresses are virtual addresses in the unpacked exe (image base
`0x400000`); Windows ASLR may map the running image elsewhere. Decompiled C for every function is in
`re/yo3_decompiled.c` (git-ignored; regenerate with the Ghidra
export, see [AGENTS.md](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md)).

**Contents**

- [Client internals](#client-internals)
  - [The binary](#the-binary)
  - [Key objects and functions](#key-objects-and-functions)
  - [The connection mod](#the-connection-mod)
    - [Skipping the stock login form](#skipping-the-stock-login-form)
    - [Automatic local deck initialization](#automatic-local-deck-initialization)
    - [Editor saves reach the server](#editor-saves-reach-the-server)
    - [What else runs in the game (#68)](#what-else-runs-in-the-game-68)
    - [Native duel observer](#native-duel-observer)
    - [The patcher](#the-patcher)
  - [The field tracer](#the-field-tracer)
  - [The UI probe](#the-ui-probe)
  - [Rendering, the window and the mouse](#rendering-the-window-and-the-mouse)
    - [Higher resolution](#higher-resolution)
  - [Data formats](#data-formats)
  - [Behaviours worth knowing](#behaviours-worth-knowing)
  - [Crash history (and the rule each one taught)](#crash-history-and-the-rule-each-one-taught)

## The binary

- Window title "Yu-Gi-Oh! ONLINE 3 ver.1.153". Built 2011-07-01 with MSVC 2008, 32-bit, unpacked, no anti-cheat.
  Irrlicht 3D, Scaleform GFx (Flash UI), Konami's statically linked EasyMultiPlay networking.
- MSVC RTTI is intact, so classes have names: `python3 tools/re/rtti.py ClassName` lists a class's vtable.
- Source paths are embedded: `.\src\App\Test\kunieda\LoginWindow.cpp`, `InfoWindow.cpp`, `ChatWindow.cpp`,
  `shopmain.cpp`, `HolderSelect.cpp`, `CharMake.cpp`, `PresentWindow.cpp`, …
- Debug trace: the helper `0x4020C0` does `printf("%s L%d : ", file, line)` plus the message. Running
  `wine yo3.exe` from a shell sends it to stdout, so `tools/client/vclient.sh` keeps it in
  `logs/client-stdout.log`. Received packets are dumped there too (`<-------- … request = N`).

## Key objects and functions

| Address                                                                     | What                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0x40F320`                                                                  | `UserData` singleton getter; the object is static at `0x82EC78`, built by `0x51CCB0`                                                                                                                                                                                                                                                                                                                                                                                                      |
| `UserData+0x28`                                                             | the selected `PlayerData` (vtable `0x7B385C`); a second one sits at `+0x430`                                                                                                                                                                                                                                                                                                                                                                                                              |
| `PlayerData+0x40`                                                           | pointer into the player record's `pbin` bytes                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `UserData+0x838…+0x8C8`                                                     | datasets, `0x24` apart; `+0x8C8` is the trunk (`YO3Trunk`, a plain `YO3NetObject`)                                                                                                                                                                                                                                                                                                                                                                                                        |
| `0x51C260`                                                                  | PlayerData loader: `pname`, `title`, `avatar1`/`avatar`, `pbin`, `speech%02d` ×23, `comment%02d` ×2, `n_win/lose/draw`                                                                                                                                                                                                                                                                                                                                                                    |
| `0x40F290`                                                                  | read the `request` id of a received message                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `0x408F50` / `0x409020` / `0x4090F0` / `0x40F1D0`                           | typed field getters: uint / int / string / raw value (NULL when missing); all call `0x41A980`                                                                                                                                                                                                                                                                                                                                                                                             |
| `0x41A980`                                                                  | `std::map<std::string, Value*>::find`: ESI = tree (map+4), `[esp+4]` iterator out, `[esp+8]` key, `ret 8`; end node at `tree+0x18`, value at `node+0x28`                                                                                                                                                                                                                                                                                                                                  |
| `0x408640` / `0x4084A0`                                                     | setters (set-int / set-value): `find`, then insert if absent; the client uses them to build its own messages                                                                                                                                                                                                                                                                                                                                                                              |
| `0x51E6C0`                                                                  | UI text-table lookup `(table, text id)`. Text ids overlap message ids (e.g. `0x2774`)                                                                                                                                                                                                                                                                                                                                                                                                     |
| `0x410D60`                                                                  | `item[edi]` of a message (NULL when out of range)                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `0x469C00`                                                                  | login dispatcher (`LoginWindow.cpp`)                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `0x4F4A00`                                                                  | global front dispatcher; its switch starts at `0x4F4A4F` (route ids with `tools/re/dispatch.py`)                                                                                                                                                                                                                                                                                                                                                                                          |
| `0x458410`                                                                  | InfoWindow reply handler: news (`0x21FC`, from `0x458475`), player search (`0x2CF7`), friend list (`0x251C`)                                                                                                                                                                                                                                                                                                                                                                              |
| `0x472EDC…0x473A3A`                                                         | main-screen startup-step reply handlers (each waits on its request through `obj+0xE8`)                                                                                                                                                                                                                                                                                                                                                                                                    |
| `0x46D8A0`                                                                  | MainGamen constructor: loads `main.swf`, `sisetu_block.swf`, `main_menubar.swf`, queues the startup steps (see the protocol's main-screen section)                                                                                                                                                                                                                                                                                                                                        |
| `0x472230`                                                                  | MainGamen lock bits `(this = ESI, bits)`: bit 0 facilities (`_root.sisetuEnable`), bit 1 menu (`menubtnEnable`), bit 2 duel button (`duelbtnEnable`)…; `0x3F` locks all, `0` unlocks                                                                                                                                                                                                                                                                                                      |
| `0x4725B0`                                                                  | startup step that unlocks the main screen (reads `event_id`, `npc_type`; event players take another path)                                                                                                                                                                                                                                                                                                                                                                                 |
| `0x476240` / `0x476DE0`                                                     | PresentWindow constructor / reply handler (window id `0xFFFFD90C` in the factory `0x4EEB90`)                                                                                                                                                                                                                                                                                                                                                                                              |
| `0x48AA60` / `0x48F080`                                                     | shop (`shopmain.cpp`) reply handler / open routine; the open routine reads slot 8 unchecked                                                                                                                                                                                                                                                                                                                                                                                               |
| `0x531B90`                                                                  | item factory: builds a received item's object by its tag (6 → `YO3Deck` `0x414240`)                                                                                                                                                                                                                                                                                                                                                                                                       |
| `0x43FC60`                                                                  | deck editor (`DeckEditTest`) reply handler; `0x22C8` saves missing decks locally (`0x52EE60`); `0x22CA` asks for the list again, `0x22CB` opens a dialog                                                                                                                                                                                                                                                                                                                                  |
| `0x4BD880`                                                                  | NPC duel deck check: regulation (`0x52DB30`), collection (`0x4F6340`), decks (`0x51AB00`); fails with text `0x295B`                                                                                                                                                                                                                                                                                                                                                                       |
| `0x51AA80` / `0x51AC50` / `0x51A9A0` / `0x51A930`                           | validate one deck / sizes / regulation / per-card limit                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `0x410DB0`                                                                  | a card's count from the trunk: `cards` (or `noTrade`) byte at index row − 1, bounds-checked                                                                                                                                                                                                                                                                                                                                                                                               |
| `0x4F6340`                                                                  | builds the playable collection `(conn, flags)`: copies the trunk (slot 18), subtracts cards lent in rental records (slot 8, flag 4), and with flag `0x10` zeroes cards 5280, 5434 and 6276 (table `0x7878A0`) in `cards`/`noTrade` unchecked                                                                                                                                                                                                                                              |
| `0x4BEB40` / `0x4BFBA0` / `0x4C0C80` / `0x4BE390`                           | SelectNpcScene: builds the tab lists from the roster (slot 4, three items per NPC) / `getNpcInfo` / `decideNpc (tab, index, level)` / duel start step (agent mode 3)                                                                                                                                                                                                                                                                                                                      |
| `0x4A0B50`                                                                  | `DuelMatchingAgent` start (`EBX` = mode: 0 DUEL panel, 1 password, 2, 3 NPC, 4 duel server); steps `0x4A0E90` deck check, `0x4A1090` `0x1784`, `0x4A1790` `0x17AC`, `0x4A3340` `0x1838`                                                                                                                                                                                                                                                                                                   |
| `0x50E470` / `0x50ECF0` / `0x50F030` / `0x50F690` / `0x50F320`              | `DuelManager` (scene `0xFFFFD8FD`): constructor / Select Deck (`0x1F4F`) / avatars and `0x17D4` / waits for `0x22C4`, connects the duel server, sends `0x200A` / mode 4: connect, then `0x1838`                                                                                                                                                                                                                                                                                           |
| `0x5233A0` / `0x5201A0`                                                     | duel stream pump (pops a duel-connection message, switches on `duelRequestName` 0x10–0x15) / reads `duelRequestName`                                                                                                                                                                                                                                                                                                                                                                      |
| `0x522A50`                                                                  | pops one engine command `{id, request, param1..param4}` for a scene; handles a few itself (2, 4, 6, `0x1A`, 1000…)                                                                                                                                                                                                                                                                                                                                                                        |
| `0x49F990`                                                                  | client duel message header: `request 0x200C`, `duelRequestName` = the argument                                                                                                                                                                                                                                                                                                                                                                                                            |
| `0x517C80` / `0x518400`                                                     | `JankenScene` constructor / update (pops commands 1500–1505 when idle; 1505 there is the match's end)                                                                                                                                                                                                                                                                                                                                                                                     |
| `0x513370`                                                                  | `DuelScene` constructor (`duel_setup.swf`)                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `0x515A30` / `0x515D30` / `0x515E10`                                        | `DuelScene` step: pops commands until command 2, then opens the result / result step (`DuelResult` in slot `0x13`) / leaves the scene                                                                                                                                                                                                                                                                                                                                                     |
| `0x5083A0` / `0x5084D0`                                                     | `DuelEndEffect` constructor `(result, reason, …)` / update: plays `duel_win/lose/draw.swf`, done when the movie ends                                                                                                                                                                                                                                                                                                                                                                      |
| `0x510950` / `0x510B70` / `0x510D50`                                        | `DuelResult` constructor / its nine steps (`0x512070` waits for 1500/1504 or front flag `0x20000`, `0x512430` match movie…) / Flash callbacks (`requestData`, `getResult` `0x511200`, `requestStarCount`, `finish`)                                                                                                                                                                                                                                                                       |
| `0x4FF000` / `0x50007C` / `0x4FB7F0` / `0x4FC520` / `0x5000E0`              | `DuelCardManager` (DuelScene `+0xBC`): card commands `0x12`–`0x29` / its switch by command id − `0x12` / a card's effect modes (0 move, 3 vanish, …: queues of card states via `0x516180`) / the vanish state: label off, effect `0x33` (the shatter), card hidden (`0x4FB220`) / a card's move (always mode 0; a pile it leaves counts down)                                                                                                                                             |
| `0x500470` / `0x5003A0` / `0x4F8240` / `0x5005D0` / `0x52A400` / `0x500640` | `DuelCardManager`'s card objects: by uid (a map at `+0x2C`) / made for a uid when missing (0x138 bytes) / the object at a place (field zones: the slot array at `+0x4C + (zone + player·0x11)·4`; the hand: by index) / a hand card taken out by index (the hand lists: `+0xD4 + side·0x18`, a vector at `+0xE0`) / a card put at a hand's end (push_back) / a hand renumbered by position (each card's index into `+0x82`, then `0x4FB7F0`). What the duel check (`duelcheck.rs`) models |
| `0x4FF3A5` / `0x5240F0` / `0x4FE4D0`                                        | `DuelCardManager`: DeckReset `0x16` {player, pile zone} / the engine's count for a player's zone (setup `MainDeckNum`, `ExtraDeckNum`) / a pile's count: shown, hidden at 0, the field label's count (`0x50BED0`)                                                                                                                                                                                                                                                                         |
| `0x4FEE70` / `0x4FFF14` / `0x4FBAA0` / `0x4FFD8E`                           | `DuelCardManager`: event entry (every `0x2001` command, whatever its id) / the tail after each command: monster records re-read (`0x5222D0`), a face-up card whose record changed takes it and flags its label (`+0xD5`) / the label redraw (fields 6 ATK, 7 DEF, 4 and 5 from record values 7 and 8) / `0x27`: a card's counters (map at `+0x130`, by kind; shown by effect mode 5 `0x4FC7E0`, icons `0x789020`, names UI text 200 + kind)                                               |
| `0x501560` / `0x501250` / `0x4E5410` / `0x4E7990`                           | `DuelChain` (DuelScene `+0xC0`): commands `0x33`/`0x34` / its states (1–2 a new link, 3–4 the `data/gfx/duel_chain.swf` banner, 5–6 a link resolving) / an effect at a field place (none past place 16) / place → field position from the table at `0x81AE38` (17 places, turned for the opponent)                                                                                                                                                                                        |
| `0x5165A0` / `0x506FA0` / `0x507750` / `0x507940`                           | `DuelSummon` (DuelScene `+0xB4`): commands `0x2A`–`0x32`, Tribute / Material / Tuning Set, Reset, Run {player, zone, uid, variant}; Tuning Run's variant keeps the level sums (`+0x64` Tuners, `+0x6C` their count, `+0x68` the others) / `DuelFusion` (`duel_fusion.swf`) and `DuelSynchro` (`duel_synchro.swf`), made by RunFusion `0x3A` by its kind / DuelSynchro's steps: it walks the material list (engine state `+0x1330`, command 1015/1016) from the Tuner count on             |
| `0x50FC00` / `0x50FDF0` / `0x510220`                                        | `DuelManager` after a duel: match result set → leave (`0x510220`), else 1504 → Card Swap (`0x50FDF0`), 1500 → next duel (`0x50F690`)                                                                                                                                                                                                                                                                                                                                                      |
| `0x790C50`                                                                  | DuelMenu's Rule Format table: `match_type` by button, 1 Single Mode, 3 Match Mode, 6 Both                                                                                                                                                                                                                                                                                                                                                                                                 |
| `0x46FEB0`                                                                  | `GoDuel` (menu bar `go_duel`): the matching agent (`0xFFFFD8F7`) idle → DuelMenu (component `0xFFFFD908`); waiting (`0x4A1D10`) → cancel (`0x4A0BD0`)                                                                                                                                                                                                                                                                                                                                     |
| `0x49D070` / `0x49D310` / `0x49D8F0`                                        | `DuelMenu` constructor / its first step (`0x17FC` → `0x17FD` counts) / Flash callbacks (`requestDataPlayers` `0x49DB30`, `requestDataAuto` `0x49DD10`, `decideAuto` `0x49DF50` → agent mode 2, `requestDataDuel` `0x49E210`, `decideDataDuel` `0x49EE90`, `cancelMenu`, `openRegSelect`)                                                                                                                                                                                                  |
| `0x4A0B50` / `0x4A0E90`                                                     | `DuelMatchingAgent` start (mode in EBX: 0 DUEL panel, 1 a player's duel, 2 auto, 3 NPC, 4 `0x1838`) / deck check: regulation by the record's `regulation_id`, `duel_regulation_id`, else the login one (connection `+0xC8`), loaded from `data/regulation/` (`0x52DB30`), then `0x51AB00` over the local decks                                                                                                                                                                            |
| `0x4A1380` / `0x4A15D0` / `0x4A1D10` / `0x4A2D30` / `0x4A2FB0`              | agent steps: auto match `0x1798` / its cancel `0x17A2` / the wait (`0x17B6` offers answered by `0x17CA`, `0x1978` → next) / the match push `0x1978` (`vsPlayerData` → MainGamen `+0x408`, `duelData` → `+0x88C`) / the opponent dialog ("LV1 Kaiba", match type), then the duel scene                                                                                                                                                                                                     |
| `0x4C8CD0`                                                                  | duel dialog factory by kind (command `0x36`; table `0x4C9178`): 0 OK, 1 Info, 2 Yes/No, 3 Effect, 4 Select, 5–6 Type, 7 Attribute, 8 Position, 9 Coin, 10 Dice, 11 Number, 13 Discard, 14 Ritual; ECX = param3, EDX = param2                                                                                                                                                                                                                                                              |
| `0x4C2090` / `0x4C29D0` / `0x4C6410`                                        | every duel dialog's Flash `decide` → vtable 16 (keeps the Flash result at `+0x38`) → vtable 15, which sends it as `duelRequestName 4 {DLGResult}` (`0x521A70`)                                                                                                                                                                                                                                                                                                                            |
| `0x4C5120` / `0x4C51A0` / `0x4C5560`                                        | `DuelEffectDialog` (kind 3) constructor / show (`dlg_yesno.swf`, or `dlg_yesno_card.swf` with up to five rows from `+0x9BC` when `+0x2A0` is `0xFF`) / whether to show: level param3 ≥ 4, else the `invoke_conf` duel option                                                                                                                                                                                                                                                              |
| `0x4C5750` / `0x4C5880` / `0x4C5B20`                                        | `DuelSelectDialog` (kind 4) constructor / show (`dlg_select.swf`: labels from the `0x3F0` values with bit 15, text = low 14 bits, at most four) / result: the label's value index                                                                                                                                                                                                                                                                                                         |
| `0x4C5E50` / `0x4C5FC0`                                                     | `DuelSelectStandDialog` (kind 8) constructor (param3: uid, bit 16 face-up defense) / show (`dlg_selstand.swf`, `dialog.faceType`, `dialog.cardId` from the uid's row)                                                                                                                                                                                                                                                                                                                     |
| `0x4C72D0` / `0x4C73A0` / `0x4C74F0`                                        | `DuelCardListDialog` (command `0x37`) init (kind → picks OK needs) / show (`dlg_cardlist.swf`, message `0x519870`) / close: sends `6`, `7` per pick, `4`                                                                                                                                                                                                                                                                                                                                  |
| `0x4C76B0` / `0x4C78A0`                                                     | the card list's Flash callbacks `getListInfo` (count, max, accept) / `getCardInfo(i)` (row from `+0x9BC`, mark from `+0xE6C`, image id, attribute, ATK/DEF)                                                                                                                                                                                                                                                                                                                               |
| `0x50B700`                                                                  | DuelFieldManager mouse handler: left click on a card → its menu / card command (`0x4F9510`); right click with other-flag 6 set → `duelRequestName 5`                                                                                                                                                                                                                                                                                                                                      |
| `0x531B90`                                                                  | item factory by tag: 3 record, 4 request, 5 `YO3PasswordKey`, 6 deck, 7 `YO3DuelData` (children by name: `gameServer`, `regulation`, `rule`, `players`, `result`; `0x5315C0`)                                                                                                                                                                                                                                                                                                             |
| `0x4EEB90`                                                                  | object factory by id: `0xFFFFD8F5` FrontServerConnection, `0xFFFFD8F6` DuelServerConnection, `0xFFFFD8F7` DuelMatchingAgent, `0xFFFFD8F8` AutoMatchingAgent, `0xFFFFD8F9` CommunicationAgent, `0xFFFFD8FD` DuelManager                                                                                                                                                                                                                                                                    |
| `0x4A523F`, `0x4A551E`, `0x4A5D05`                                          | `CommunicationAgent` switches (195/106/102 cases)                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `0x413FE0`                                                                  | address of front dataset slot `EDI` (vector at `+0x16C`); the global dispatcher keeps some replies there: `0x2711` → 4, `0x251C` friend list → 6, `0x252A` black list → 7, `0x2D6C` rental records → 8, `0x21FC` news → 10, `9000` trunk → 18, `0x22C4` duel data → 20 (flag `0x8000`), `0x2009` duel's end → 21 (flag `0x20000`), `0x2774` → 23, `0x24BA` → 24                                                                                                                           |
| `0x45E6D0` / `0x45E810`                                                     | "is this player a friend / ignored?": read slots 6 / 7 with no NULL check, for every player listed                                                                                                                                                                                                                                                                                                                                                                                        |
| `0x45F390`                                                                  | player search request builder (Detailed Search criteria → `search_flag` bits)                                                                                                                                                                                                                                                                                                                                                                                                             |
| `0x475A20`                                                                  | fills a list entry (`ppInfo`) from an item; ECX picks the kind: 0 player, 1 NPC, 2 friend, 3 ignored, 4 team member                                                                                                                                                                                                                                                                                                                                                                       |
| `0x435950`                                                                  | ChatWindow dispatcher: `0x2455` push, then a jump table for `0x2456 + n` (send / list / enter / exit)                                                                                                                                                                                                                                                                                                                                                                                     |
| `0x434D20`                                                                  | reads one chat room item (`id`, `name`, `capacity`, `num`, `region`, `status`, `type`)                                                                                                                                                                                                                                                                                                                                                                                                    |
| `0x4F3C10`, `0x4F3CF0`, `0x4F4280`                                          | chat requests `0x2135` (list), `0x2136` (enter), `0x2137` (leave)                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `0x4F17DF`, `0x4F45A5`                                                      | heartbeat `0x2724` builders (30 s / 120 s)                                                                                                                                                                                                                                                                                                                                                                                                                                                |

## The connection mod

The authenticated launcher injects `tools/mod/auth.c` before resuming the game. Its import hooks
for `GetPrivateProfileStringA`/`GetPrivateProfileIntA` supply local endpoints, `InternetOpenUrlA`
redirects asset downloads, and Winsock connect/send/close restrict sockets to the launcher and add its local cookie.
The import table of this ver.1.153 client uses WSOCK32 ordinals: connect 4 at `0x7587D0`, send 19 at `0x7587F4`,
closesocket 3 at `0x7587EC`. Both named and ordinal layouts have a native/Wine test executable. A missing import
or environment value rejects DLL initialization; the injector terminates the suspended process instead of resuming.
Account credentials remain in the Rust TLS launcher. No binary or INI file is patched on disk.

See [authentication.md](authentication.md) for the session binding and [protocol.md](protocol.md#authenticated-transport)
for the outer protocol. These hooks describe our mod, not claims about Konami's authentication.

Before injection, `yo3-launcher::components::inspect_game` maps executable RVAs through the PE section table and
checks the same four byte signatures used by the native hooks: `0x46964B`, `0x4F44BD`, `0x514212` and `0x5159EA`.
Each must lie in file-backed executable section bytes. The SHA-256 fingerprint and mod release/commit/ABI are logged
for identifying builds; the supported layout remains the contract, rather than trusting a version string.

The original executable imports `d3dx9_40.dll`, `MSVCR90.dll` and `MSVCP90.dll`; its manifest requests the x86
`Microsoft.VC90.CRT` assembly, version `9.0.21022.8`. The 32-bit injector's `--check-runtime GAME.exe` reads that
manifest as data, activates its context and probes only these known dependencies from the game/system runtime
locations. It then reads the executable's import table as data and looks up every imported function where the
game's loader would find it (the game folder, then Windows; WINMM in Windows' copy, which our loader forwards to). A
missing one is `stage=runtime-import windows_error=127`, naming the function, the DLL and which copy lacks it. It does
not invoke the game or its local WINMM loader. Desktop validation and script startup both run this check before
connecting. This covers dependency loadability, not graphics initialization or UAC consent.

Windows loads a suspended game's DLLs when its first thread runs, and that is the injector's `LoadLibraryW` thread. A
missing DLL or function ends the process there, and the thread's exit code is the process's, `0xc0000139`, not a
module address ([#41](https://github.com/usersina/yo3-revival/issues/41), Windows 10 LTSC 2019: the injector used it
as the DLL's base and reported `stage=mod-ready-thread windows_error=5`). The injector now checks the process after
the loader thread and reports `stage=game-startup process_exit_code=…` with what the code means (`0xc0000139` entry
point not found, `0xc0000135` DLL not found, `0xc0000142` DLL initialization failed, `0xc000007b` wrong architecture);
Desktop shows "Windows stopped the game while loading its DLLs." Wine resolves the executable's imports before any
injected thread runs, so only native Windows reproduces this; `yo3-missing-import-test.exe` (built by
`tools/mod/build.sh`, it imports a function KERNEL32 lacks) is the fixture for both checks.

Native Windows 11 (build 22000.2538) exposed a path-boundary failure: the same supported executable passed the
32-bit checker with a normal drive path and failed `stage=runtime-image windows_error=2` with the `\\?\` form
returned by Rust's `canonicalize`. `yo3-launcher::runtime::native_path` uses `dunce::canonicalize` on Windows,
preserving file identity while choosing the compatible spelling where possible. Runtime checks, injection paths
and Desktop's direct-loader configuration share this boundary. The Windows-only `native_paths` regression failed
before the fix and passed afterward with spaces and Japanese characters; actual Desktop validation also passed.

Native failures log stable `stage=...` labels, Win32 errors where available and process exit codes. DLL hook
rejections distinguish launch state, imports, login, starter and duel hooks, retaining fail-closed behavior.
The Rust helper includes a bounded, redacted native stderr tail in startup errors so Desktop details and exported
logs preserve the cause instead of reporting only EOF. Full native Windows play/elevation checks remain separate
from these dependency and path checks.

Native Windows 11 also mapped the supported game at `0x006D0000`, exposing the DLL's old fixed-base guard.
`tools/mod/game-image.h` resolves preferred evidence addresses through the actual module base. Login string
imports, starter code/data/vtables and duel callbacks/globals all use that mapping. Relative CALL signatures stay
exact; the MOV update-callback guard checks its relocated operand. ASLR remains enabled. Synthetic full-size
fixtures contain real PE callback relocations and also link at `0x600000`: the old DLL failed and the corrected DLL
passed on native Windows and Wine. The real native client reached player selection, the main map and a deck editor
showing a selected 40-card starter and 498 Trunk cards.

The original executable's administrator manifest also caused native startup to prompt for UAC, then fail injection
with `VirtualAllocEx` error 5 when the game ran above the standard-user injector. The Windows launcher now appends
`RunAsInvoker` to the injector child's `__COMPAT_LAYER`, preserving inherited compatibility flags and leaving the
launcher/server environment unchanged. A Windows-only regression went red/green for that boundary. Actual Desktop
Normal mode then resumed the game without a UAC prompt; token inspection confirmed the app, helper, server and
game were all non-elevated. A fresh NativeSmoke player received its selected starter before entering matchmaking,
without opening Deck Edit. No executable manifest or machine UAC policy is changed.

### Skipping the stock login form

The launcher authenticates before the game resumes. The original form adds no authentication: **ONLINE** can go
straight to player selection. `tools/mod/login.h` redirects only the verified native `CALL` at `0x46964B`
(`E8 50 6C 08 00`, originally `0x4F02A0`, invoking the movie's `StartLogin`). `LoginWindow`'s frame handler
`0x469440` reaches it once, after its startup countdown and normal scene preparation.

The replacement performs the gate-connect step of native `goLogin` (`0x46B5FF…0x46B62C`): copy the window's
gate host (`+0x10DC`) into its gate connection (`+0xB0`, string at `+0x78`), set port `+0x94` from window
`+0x10F8`, then call the connection's virtual slot `+0x38`. String assignment uses the game's own import
`0x7583B4`. The call-site bridge takes the window in ESI, preserves callee-saved registers and matches `ret 4`.
Saved login strings, remember flags and native error state stay untouched; no password or account key is supplied.

The original `login.swf` frame 1 starts with a fully transparent login form and stops. Avoiding `StartLogin` leaves
the form hidden. The normal successful player-list handler still invokes `StartPSelect`; its frame check accepts any frame
below 12 and enters player selection. Native failure dialogs, player creation and Cancel remain responsible for
their own transitions. No stock files are patched, and an unexpected call instruction refuses mod readiness.

The shared C fixture checks both fresh and saved login data, preservation of unrelated native state, and byte-guard
rejection. `tools/mod/test-dll.sh` also runs it as 32-bit Windows code under Wine and independently checks changed
login and starter hook sites. Live verification is recorded in [the history](history.md).

### The game's own login form (#34, research R1)

The required mod skips the form at ONLINE (above), except when the helper has no saved sign-in for the server: then
`login.h` sends the one call at `0x46964B` on to `StartLogin` (`0x4F02A0`) itself, registers and stack as they were, and
`login.swf` shows "Welcome to the world of Yu-Gi-Oh! ONLINE!" with User ID, Password, Save User ID and Save Password
(texts 10300–10305). It asks the helper first (`login-form.h`, `GET …/revival/login-state`: "form" or "ready").

- **goLogin** (`0x46B550`, the form's OK) stores the User ID and Password boxes in the LoginWindow's std::strings at
  `+0x4C` and `+0x68` and copies them to UserData `+0x338`/`+0x354`, starts the gate connection (`+0xB0`, slot
  `+0x38`), then reads the checkboxes `login_win.isbtn.Checkd` and `login_win.psbtn.Checkd` into `+0x88` and `+0x8C`
  (1 = ticked). The strings are MSVC 2008 `std::string`s: text inline at `+4` while the capacity (`+0x18`) is under 16,
  else a pointer there; size at `+0x14`.
- **The hand-over.** When the game then connects to the front, the DLL posts the User ID, the password and Save Password
  to the helper on the local HTTP port and blanks `+0x68`. The helper signs in (account operation `login`, `stay` with
  Save Password, the device token into the credential vault) and the front answers the game's `0x1F47` with the result
  or a refusal (protocol, "Direct start's sign-in"). After a refusal the game returns to the title screen; its next
  ONLINE shows the form again.
- **What the game saves.** On `0x219A` the handler `0x469C00` writes options.y3o's `lastLogin` (the User ID with Save
  User ID, else empty) and a `users\<md5>.y3u` record with `isSavePassword`, `uname` and `password`, the last from the
  window's `+0x68` **whatever Save Password says** (`0x46A2C7…0x46A33A`). With `+0x68` blanked first, the record holds an
  empty password; Save User ID keeps working as the game's own. Live (2026-11-16): after a login with both boxes ticked,
  `{"isSavePassword": 1, "password": "", "uname": "formtester"}` and `lastLogin` "formtester"; the password appears in no
  file of the game folder, Documents or the Wine prefix. UserData `+0x354` keeps it in memory, as the stock client did.
- **The form's limits** (`login.swf` sprite 67): `uid.restrict` and `pwd.restrict` are `"A-Z a-z 0-9"`, and the boxes
  take 16 characters (live: 16 asterisks for 17 typed). So the server takes User IDs of 3–16 and
  passwords of 8–16 letters or digits only (`logins.rs`), and every account can sign in through the form.
- **Delete Player's prompt** (R4, #23) compares locally in Flash: `login.swf` frame 60 checks
  `_root.passWord == _root.delpassword` and sends only `goDelete(player)`. `passWord` is the form's password, so with the
  form it is the real one; when the form was skipped it is empty.

Tests: `tools/mod/login-test.c` (native through `login_flow.rs`, and under Wine in `test.sh`: the strings, the blanking,
the requests), launcher `direct_start.rs`, and `tools/mod/test-direct.sh`'s #34 rows (form, wrong password, Save
Password, ban).

### A Documents folder per server (#35, research R5)

The client finds its folder in two places only, both `SHGetFolderPath(CSIDL_PERSONAL | CSIDL_FLAG_CREATE)` (`0x8005`)
followed by `PathAppend` "Yu-Gi-Oh! ONLINE 3": the wide call at `0x52B93F` (in `0x52B8E0`, run once at startup from
`0x40B800`; its result is cached behind `0x82D2C8` and becomes the file component's root) and the narrow one in
`0x52BF40` (the duel-log paths, `%s\players\%s\duellogs\%s\%s` at `0x52BF40`'s caller). Everything else is built on
those roots: `options.y3o` (read at startup, written on a login and at exit), `users\<md5>.y3u` (written on `0x219A`),
and per player `players\<pname>\` with `<pname>.y3p`, `decks\*.y3d` (Edit Deck saves, our starter deck), `mail`,
`avatars`, `regulations`, `duellogs\cache` and `duellogs\myduels` (after a duel). The executable imports no other
folder lookup (no `SHGetSpecialFolderPath`, Known Folder or environment call; its registry reads are the OS version,
CPU and keyboard layouts). Answering those two calls is therefore enough, and it touches no other path: `documents.h`
does it per server (`YO3_SERVER_ID`, from the launcher, the Desktop's Play and direct start's ready file alike). The
first server played on keeps the original folder (`revival-server.txt`); every other gets `Documents\YO3 Revival\
servers\<id>\Yu-Gi-Oh! ONLINE 3`, starting with `options.y3o` copied. **D13: yes.** Live (2026-11-16, a fresh
Documents, two scratch servers each with a player "Duelist"): server A claimed the original folder; with a second deck
"Only On A" in it, A's Select Deck listed both decks and B's only its own Starter Deck, kept under
`YO3 Revival/servers/35b82924973c0938/`; A's folder was unchanged. An extra game of the Desktop's Play as… (#64) is given a
derived id the same way (`yo3_launcher::game_data_id`: the server's id and the account, 16 hex digits, via the helper's `--data-key`),
so each account's game has a data folder of its own without any change to the DLL; the main game keeps the server's.

### Direct start (#33)

`yo3.exe` opened directly loads our `winmm.dll` from its folder (the game imports seven WINMM functions, which it
forwards). Its import hooks wait for the game's first `[Yo3]` INI read, then start the helper named by the private
configuration (`HKCU\Software\YO3 Revival\LoaderConfig`), load the connection DLL once the helper's ports are ready,
and let the game go on; the helper signs in meanwhile (protocol, "Direct start's sign-in"). What the player sees when
that fails is the game's own: the helper answers the front's login with one of LoginWindow's refusals.

- **No helper** (missing files, a changed DLL, a helper that exits early). The loader keeps the game offline: it
  redirects `0x46964B` itself (the same login skip as the DLL, `login.h`), answers `GateServerAddress` with 127.0.0.1,
  refuses the game's connects and WinInet requests, and shows one line once. The stock client's patch check runs at
  start, so ONLINE then shows "You will need an active Internet connection to play ONLINE." and the client closes
  itself after OK (live, 2026-11-16). A DLL that loaded but failed its readiness check still stops the game: it has
  already hooked the imports.
- **Delete Login Data** (`signout.h`, the Yes branch's save call `0x4CA488`) asks the helper over the local HTTP
  port to revoke the device's saved sign-in. The DLL's own Winsock imports carry no launch-cookie preface (only the
  game's imports are hooked), which the local HTTP port expects.
- **Wine** runs a Linux program started by a Windows one (the helper starting your own server) through a double fork:
  its parent becomes the session's init, its standard input `/dev/null`, its environment Wine's own, not the one asked
  for, and its exit code meaningless. The launcher therefore starts the Linux server as `/bin/sh -c WRAPPER` with
  `/usr/bin/env` for the variables; the wrapper feeds the server's input while the helper touches a heartbeat file
  (`yo3-launcher` `direct::WRAPPER`). A Windows program started under Wine reads Wine's own credential store, so the
  Linux launcher copies a saved device token there (`yo3-launcher --vault-set`).

### Automatic local deck initialization

`tools/mod/starter.h` also redirects the verified CALL at `0x4F44BD` to a wrapper around the original global
front dispatcher `0x4F4A00`. The expected bytes are `E8 3E 05 00 00`; an unexpected game-sized executable rejects
DLL readiness before login. Only the in-memory displacement changes. Native/Wine fixtures cover changed bytes
and the suspended-process failure path. This targets ver.1.153; the executable and INI on disk remain unchanged.

The wrapper observes `0x22C8` after native dispatch, on the game thread. Player selection at `0x46BDB0` already
loads the local cache through `0x52F240`. The front connection retains player id at `+0xD4`, regulation at `+0xC8`
and dataset pointers at `+0x16C`/`+0x170`; collection slot 18 and rentals slot 8 must exist before initialization.
FileManager is `*(0x82D2C8)`, its root string is `+0x50`, deck-map tree `+0x1000D8`, reload flag `+0x1000F8`.
`0x51AB00` checks cached decks; `0x51AA80` checks one candidate with EAX=deck, EBX=three pointers: card table
`*(0x82EBA8)`, regulation from `0x52DB30` (parsed by `0x414630` with ESI=regulation), collection from
`0x4F6340(connection, 0x1D)`. Both collection/regulation objects are released with their native virtual destructor.

The native serializer `0x52C320` is thiscall(FileManager, deck, by-value GameString), returns zero on success and
consumes 32 stack bytes. The 28-byte MSVC string has proxy at `+0`, inline buffer/heap pointer at `+4`, length at
`+0x14` and capacity at `+0x18`; constructor/destructor imports are `0x7583CC`/`0x7583D4`. We serialize to a unique
staging file, atomically move without replacement, then reload the native cache; reply objects are never retained.
The player's name comes from the user-data map at `0x82ECA4`. Profile config `*(0x82EC10)` contains `deckInfo`;
only an empty `cursor_deck_name` is set through its native string setter and saved by `0x52E200`.

### Editor saves reach the server

`tools/mod/deck-upload.h` redirects the verified CALL at `0x415A4D` (bytes `56 E8 BE FB FF FF` from `0x415A4C`: `push
esi; call 0x415610`) to a stdcall wrapper. `0x4159B0` is reached only from the deck editor's save `0x4442E0`
(`DeckEditTest`: Save, its overwrite confirmation and New); it stores the name in the editor's deck model, makes the
record with `0x415610(model)` (stdcall, a new `YO3Deck`, vtable `0x794BEC`, type byte `+0x34` = 6) and writes it with
`0x52EE60` before deleting it. The wrapper sends that record before the write, built as Select Deck builds its upload
(`0x50EEAD`-`0x50EF8C`): `0x40A190` (stdcall, the 0x6C-byte request), YO3Request vtable `0x7955F4`, type byte 4,
`0x41BCA0(0x1F4F)` (thiscall), `update_type` 1 through the map at `+0x60` (vtable `+0x14`: by-value key string, then
the int), the item with `0x40A320`, ownership byte `+0x50` of the item list at `+0x68` cleared (as `0x50EF63`), send
`(*(front+0x74))->vtable[0x50](request)`, destroy with `0x40A210`; the record's parent pointer `+0xC` is restored.
The front connection is the one the editor finds at `0x441340`: UserData (`0x40A500`) `+0x60` registry, vtable `+0x30`,
id `0xFFFFD8F5`. Like `netlog.h`, the hook treats an image smaller than `0x42E000` as a contract fixture (their code
occupies these low addresses); in the game an unexpected byte refuses DLL readiness (`stage=mod-deck-hook`).
Names and paths are bounded and checked before filesystem operations. This changes local behavior, not the deck
wire format or exported mod ABI. Any existing local deck or nonempty saved selection prevents setup, regardless of validity. A per-player
`revival-starter.initialized` file records setup or discovered existing decks/selection; automatic setup never undoes subsequent manual edits or deletions. Existing selections take precedence; no legal candidate leaves the profile untouched. See the [deck protocol](protocol.md#decks-deck-editor-deckedittest-handler-0x43fc60).

### Card search (issue #54)

Edit Deck's Search Text (`deck_sort_text.swf`: `TextSearchExecute(text, Card Name, Card Text)`, both boxes ticked by
default) and the card lists at Blister's both register `TextSearchExecute` (`0x4955C0`, `0x466FA0`), which converts the
text from UTF-8 and filters **the list shown** with `0x416C10`: a card stays when the text is in its reading
(`0x537570`, only when `0x406680()+4` is 0) or name (`0x5374B0`), or, with Card Text ticked, its text (`0x537510`).
Neither box ticked keeps the list. Live (2026-11-19, 498-card trunk): "Red" found 2, "red" 0; "Dragon" found 12, then
"Red" 1 of those 12; after a search with no result every search finds 0 until Reset (`KabanSearchReturn(5, false)`).
The comparisons are msvcrt's `wcsstr`, loaded into `edi` at `0x416D40` and `0x416DDD` (`8B 3D 80 85 75 00`, `mov edi,
[0x758580]`; the operand is relocated under ASLR). `tools/mod/card-search.h` replaces both with `mov edi, imm32; nop`
pointing at `yo3_wcsistr` (case-blind, `card-search-logic.h`); `0x416C10` has no other caller. An unexpected byte refuses
DLL readiness (`stage=mod-card-search`); a contract fixture's small image is left alone, as `deck-upload.h` does. The
narrowing is kept: it is how the original searched.

### Card profiles (issue #60)

The client draws a card's kind, type, level, attribute, race, ATK, DEF, text and picture from its own files; a *card profile*
(the server's `profile.rs`, served at `/web/card-profile`, or a file in `YO3_CARD_PROFILE`)
tells `tools/mod/card-profile.h` what to show instead. The format (`card-profile-logic.h`, v1) is `yo3-card-profile 1`, then
`profile NAME`, `hash 16-hex`, `card ROW CID A_HEX B_HEX GENRE_HEX64 ATK DEF` (ATK/DEF a number, `?` or `-`), `text ROW "json"`
and `art CID`; unknown lines are skipped, every list is bounded, a line whose row does not hold that CID is refused.

- **The table.** The game loads `CARD_Prop`, `CARD_Genre`, `CARD_Indx`... into one object, `env+8` with `env = 0x406680()`,
  in `0x40DEE0` (fastcall, ECX env), called only from `0x4AC30E` (`E8 CD 1B F6 FF`, the load state of `0x4AC2C0`). The DLL redirects that
  CALL to `card_profile_load`, which runs the load and then patches the profile's rows in place: `CARD_Prop` rows are 8 bytes at
  `DB+0x24` (begin) / `DB+0x28` (end), `CARD_Genre` rows 8 bytes at `DB+0x3C` / `DB+0x40`. Every reader (`0x537230` row by Konami id,
  `0x537B30` row by index, the inlined `shr eax, 0xE` ATK and `shr eax, 0x17` DEF readers at `0x4565CB`, `0x4C795A`, `0x4C8772`...) then sees the
  patched row, so the deck editor's Main/Extra rule, the Edit Deck and trunk lists, the card panels and the card search follow the profile
  with no further hook. The load is once per process: **a different profile needs a client restart.**
- **What the bits mean** (measured on all 4,834 rows against the official database, `tools/mod/genre-bits.txt`): word `a` is the Konami
  id (bits 0-13), ATK/10 (14-22) and DEF/10 (23-31); `0x1FF` shows `?` (`0xFFFF` in the getters `0x5373C0`/`0x5373F0`, 0 in the deck
  editor's ATK/DEF filter). Word `b`: kind (bits 3-7), attribute (8-11), level (12-15), spell/trap subtype (16-18), race (19-23), artwork
  (27-29). Monster kinds carry Normal/Effect and the families (0 Normal, 1 Effect, 2/3 Fusion, 4/5 Ritual, 6 Toon, 7 Spirit, 8 Union, 9 Gemini,
  10 Token, 15/16 Tuner, 17/18 Synchro, 19 Tuner Synchro); 13 is a Spell and 14 a Trap, with attribute 8/9 and race 23/24. Subtype: Spell 0 Normal,
  2 Field, 3 Equip, 4 Continuous, 5 Quick-Play, 6 Ritual; Trap 0 Normal, 1 Counter, 4 Continuous. "Special Summon only" and Flip are not
  kinds (Flip is genre bit 22): the client cannot show Special Summon only. Bits 24-26 are the card's **rarity**, the same for every artwork
  of a card: 0 Common, 4 Rare, 3 Super Rare, 2 Ultra Rare (as the pack files' tiers place them), 1 and 5 the top tiers, 6 and 7 a few
  promotional cards. `0x418890` reads them for the new-card reveal and the pack opening (`0x4784f0`, `0x479690`), which hand the Flash screen a
  glow of 0 (0, 5), 1 (4: silver), 2 (3) or 3 (2, 1, 6, 7: gold). Bits 0-2 have no reader among the card table's getters (every
  `0x537230` caller and `0x537420`); they are 1 on most rows and 0 on the newest. `CARD_Genre` is **not** a type bitmap: it holds the effect categories
  the card panel names (`0x4529F0`, `0x417A10` test one bit per category through `0x5372A0`).
- **Exact ATK/DEF.** An `a` word holds tens (0-5100) and `?`; the profile's number is only exact when it is such a value. The getters
  `0x5373C0`/`0x5373F0` (prologue `sub esp, 0xC; call 0x537230`) and the trunk list's `0x418600`/`0x418660` (a stub on their `CALL 0x537230`
  notes the id) are also wrapped. The card panels that do the math themselves (each `call 0x537230`, `shr eax, 0xE`/`0x17`, then `lea R, [eax+eax*4]; add R, R`)
  are patched at six sites, and the deck editor's range filters at four more (`card-profile-logic.h` `yo3_card_mul_sites`: ATK `0x4565C6`, `0x4C7955`, `0x4C876D`, DEF `0x4565F4`, `0x4C7982`, `0x4C879A`, the register R being eax,
  ecx or edx): the CALL becomes a stub that notes the id, the five bytes of `ten times` become a CALL to a stub that answers the profile's number (any integer, so
  1) or, with none, ten times the nine bits. The deck editor's four ATK/DEF range-filter reads (`0x418AB7` and `0x418ADE` the ATK minimum and maximum, `0x418CBF` and `0x418CE1` the DEF ones: a card is kept
  when min <= stat <= max, "?" is 0 and is taken before the multiplication) are the same pattern with R = eax, so they are the last four rows of that table (`call`, then the `lea; add` at the jump target of the "?" test),
  and a card with 1 ATK is found by a range that starts at 1. All ten are checked before any is written (a game whose bytes differ refuses readiness). The range filters' "?" is 0 for a profile's own "?" too (their stubs, `card_cb_filter_atk`/`def`, answer 0 for it; the getters answer `0xFFFF`).
- **Sorting and range search** (Edit Deck's ATK and DEF dialogs: a minimum and a maximum field, Sort Up, Sort Down). The dialog's handler (`0x465ED0` family, and its twin at `0x495332`) reads the fields with `0x6D6CB0`, a blank minimum is 0 and a blank
  maximum is 5000 (`mov eax, 0x1388` at `0x465F73`, `0x465FBC`, `0x4953AC`, `0x4953ED`; the search with the fourth argument 9 has its own `0x100`), calls the filter `0x418930` (its fourth argument 0 ATK, 1 DEF) and then the sort `0x419050`
  (EAX the sort kind, one stack word the direction). The filter walks the list and drops what is not a Monster (`0x416340`: id nonzero and kind not `0xD`/`0xE`) or is outside min <= stat <= max, the stat
  read by the four sites above. The sort writes a key at `+0x18` of every list entry and then orders by it (the entries stay in their list order for equal keys): for ATK/DEF the key is
  `(movsx16(getter) - 2 * isMonster) * 10000 + place` going up and `(0xFFFF - 2 * isMonster - movsx16(getter)) * 10000 + place` going down (`0x41926E`/`0x4192A5` call the ATK getter `0x5373C0`, `0x419362`/`0x419399` the DEF
  getter `0x5373F0`; the other kinds read the `b` word: attribute `>> 8`, Level `>> 12`, race `>> 19`, Spell/Trap kind `>> 16`). So the sort needs no hook of its own: the getters are
  the wrapped ones, and the keys hold the profile's numbers; `?` (the word `0xFFFF`, read as -1) is the lowest key going up and the last going down, 1 ATK sorts before 9 and 2501 after 2500, and equal numbers keep the list's order. A
  number above 32767 would read as negative there, so a profile's numbers are shown as 32767 at most (`YO3_STAT_MAX_SHOWN`). The one thing the profile had wrong was the blank maximum: a card of 5100 or 6000 ATK (the anime has them) was dropped by an
  open search, so with such a card in the profile the four immediates become 100000 (`card_open_max`, bytes checked first; nothing is touched without such a card or without a profile). Proof: `card-profile-test.c` and
  `card-profile-hook-test.c` (the real wrappers and stubs of `card-profile.h` over synthetic code, every pair of bounds and both sort directions, with and without a profile), and live (`tools/client/trunk-scan.py`,
  `trunk-range-check.py`; `tools/mod/make-sort-profile.py` makes a profile that gives the game's Monsters 0, 1, 9, 10, 2500, 2501, 5100, 6000 and `?`; `YO3_CARD_PROFILE` points the client at it).
- **Effect text.** `0x537510` (EAX the Konami id, ESI the table, answers the text's address) is wrapped (prologue `push ecx; mov ecx, [esi+0xD0]`); the
  card panel, the card search (Card Text) and the lists read the profile's text. The card name and reading are the game's.
- **Artwork.** Faces (`data/card/<cid>_<art>.png`) and thumbnails (`data/mini/...`) are opened, in the Flash screens and as the duel's Irrlicht textures,
  by the file system's `createAndOpenFile` `0x5505E0` (thiscall, ECX the file system, one `char *` argument, prologue `push ebp; mov ebp, esp; and
  esp, -8; sub esp, 0x24`): it asks its archives (the `.ypk` files) first and opens a name they do not hold as a file on disk, so a loose file at
  `data/card/` is ignored (live: it was) while an absolute path works. The DLL wraps it: for a card the profile lists with `art`, whose
  `card/<cid>.png` (200x290) or `mini/<cid>.png` (50x72) is in the art folder (`YO3_ANIME_ART`, else `<game folder>\anime-art`; one picture per Konami
  id for all its artworks), the name becomes that file's path. A missing file leaves the original picture (a missing thumbnail keeps the original thumbnail, so
  the launcher makes a `mini/` from each face). No image is shipped or fetched.
- **Where the profile comes from.** `YO3_CARD_PROFILE` (an absolute path, for checks), else `GET /session/<cookie>/web/card-profile` on the launcher's local relay (as
  `web-links.h`; the launcher forwards `/web/` to the server's `/web/card-profile`), else none. A game whose bytes differ at a hook refuses readiness
  (`stage=mod-card-profile`); a contract fixture is left alone; a bad profile changes nothing (`not a profile`, logged with its first bytes).
- **Live (2026-11-19, headless client, private prefix and game copy, scratch server).** A hand-made profile (`tools/mod/make-test-profile.py`):
  Mystical Elf showed `[Spellcaster/Effect]`, the profile's text, an orange (Effect) trunk bar and `1/?` in the trunk list; Flame Swordsman
  showed `[Warrior]` with a yellow (Normal) bar, counted as a Normal monster in the deck editor's Main row after it was added (the Extra row stayed 0);
  Negate Attack showed `[SPELL/Quick-Play]`, the Spell colours and icon. The text search found each by the profile's text. With generated placeholder
  pictures (`tools/mod/make-placeholder-art.py`) the face and the thumbnail of each listed card were the placeholder, the card without one the original. With the
  server's own `anime` profile (320 cards, 319 texts) over the launcher's relay: `card profile anime (hash): 320 cards patched, 0 refused`, and Flame Swordsman
  read `[Warrior]` with the anime Normal Monster text. Not proven here: a duel's field cards (the engine sends their stats), Windows, saved decks.
- **Live, round 2 (2026-11-19, private prefix, scratch server, the server's real `anime` profile over the relay, a copy of the user's image pack in `YO3_ANIME_ART`).**
  With a hand profile giving Earthbound Immortal Wiraqocha Rasca 1/1 and Mystical Elf 37/1: the trunk list, Edit Deck's card panel (`ATK 1/DEF 1`), the present
  dialog's panel and the duel's hand-card panel all read 1/1 (the picture's printed `ATK/ 100 DEF/ 100` is the card face, the pack replaces it). With the real profile
  and pack: Flame Swordsman (anime Normal Monster face, `[Warrior]`, the anime text, Main Deck), Mystical Elf (`ATK 800/DEF 2000`, anime text) and Card of Sanctity
  (`[SPELL/Normal]`, "Each player draws until they have 6 cards in their hand.") read the anime data and art in Edit Deck, the trunk list, the present dialog and the duel's hand
  panel and hover. Negate Attack read the anime art and text, but its type line said `[TRAP/Quick-Play]`: the server's profile row had kind Spell and subtype Quick-Play
  but kept the Trap's attribute (9) and race (24), which the line prints; a Spell row needs attribute 8 and race 23 (the hand profile did and read `[SPELL/Quick-Play]`).
- **Saved decks.** The DLL does not move cards between a deck's lists (the lists are the game's `std::vector`s). A local deck file (`.y3d`) is best adapted by the
  launcher before the game starts (the same rule as the server's: a card whose profile kind is Fusion or Synchro goes to the Extra Deck, any other to the Main Deck).

### Server texts and NPC duel logs (issues #10 and #12)

`tools/mod/dialog-text.h` lets duel dialogs show the server's own text. Two byte-guarded hooks: the duel pump's call
to its `duelRequestName` reader (`0x523411`, `E8 8A CD FF FF` → `0x5201A0`, ESI the message) is redirected to a thunk
that calls the reader and, for `0x7A12`, keeps the message's `text` (UTF-8, 512 UTF-16 units at most) for its `slot`;
the id still reaches the pump's default case, which drops it. And `0x40BB20` (`55 8B EC 83 E4 F8`, the duel prompt
table's `vector::at`: EDI the vector, EBX the id, returns a pointer to the entry the caller adds to the string base) is
detoured: for EBX in `0x3F00`–`0x3FFF`, EDI the vector at data manager (`0x82EBA0`) +8 `+0x84`, and the id past the
vector's end, it returns a pointer to `kept text − *(+8 +0x9C)`; every other call runs the original. Texts are kept on
the game's thread (the pump) and read there (dialogs).

`tools/mod/npc-duel-log.h` NOPs the six-byte `jne 0x5123BD` at `0x5122E6`, after `cmp dword [eax+0x804],0` at
`0x5122DF` (13 bytes checked), so DuelResult's `dlogURL` step also runs for NPC duels. Both headers fail closed like
the others (the mod refuses the game). The launcher's preflight (`yo3-launcher::components::inspect_game`) still checks
only the four original sites; the DLL checks every site it patches.

### Animation settle answers (issue #11)

`tools/mod/duel-observer.h` also takes engine command `0x5F1 {seq}` at the duel dispatcher (keeping it from the stock
`0x514290`) and, after each scene update, tests the same settlement as the snapshot (battle `+0x74`, chain `+0x58`,
every card's `+0x114`); once settled it sends `duelRequestName 0x7A13 {seq}`: a `YO3Request` (`0x40A190`, vtable
`0x7955F4`, type 4) with `request 0x200C` (`0x41BCA0`) and its fields set through the map's vtable `+0x14`, sent by the
duel connection `[0x82D2C4]`'s vtable `+0x50`, as the phase button does (`0x521180`). It skips a replay (engine
`[0x82D2C0]` `+0` is 1) but not the player's input flag `+0x284`, which the input wait 4 and a card list `0x37` set
(`0x523062`): the marker comes before them. Unsettled after 30 s it gives up (the server's allowance stands).

### Rental Card List's Return button (issue #13)

`tools/mod/rental-return.h` (pure parts in `rental-return-logic.h`, tested by `rental-return-test.c`) gives the Borrowed
tab the Return button its movie wires but never places. It writes no code bytes and no movie bytes: it replaces three
RentalRecord vtable entries in `.rdata`, after checking each still holds the game's own function at the actual base
(else the mod refuses the game, stage `mod-rental-return`):

- **Accept** (FxDelegateHandler vtable `0x7A7A54` slot 1, `0x7A7A58` → `0x481D70`, thiscall `(handler, processor)`):
  after the original registers ExitRental … AdjustRental, the wrapper registers `BorrowReturan` the same way: a
  `GString` (`0x5C8C90`, thiscall, one argument; released as the game does, refcount at node `+4`, freed by stdcall
  `0x5BBC90`) and the processor's vtable `+4` `(name, callback)`. Callbacks are cdecl `(FxDelegateArgs *)`: `+0x10` the
  handler (RentalRecord `+0x2C`), `+0x14` the movie, `+0x18` the GFx arguments, `+0x1C` their count.
- **The callback** takes argument 0 (a number, the absolute row) and the borrowed list at RentalRecord `+0x90`/`+0x94`.
  Its entries are field maps, not records: the `0x2D6E` handler stores each item's `+0x68` map's `+4` copy
  (`0x48217C`–`0x4821A2`; reading them as records faulted in the map lookup `0x41A980`). It reads `rental_id` and
  `confirm_to`, sends `0x7A14 {rental_id}` as the screen sends `0x2C34` (`0x482870`: a `YO3Request`, the front
  connection RentalRecord `+0x34`, its sender `+0x74`, vtable `+0x50`) and asks the next tick to grey the row.
- **The tick** (main vtable `0x7A7A14` slot 14, `0x7A7A4C` → `0x482250`, thiscall `(this, event)`, the step that
  builds the movie at state `+0x40` 2): once built (state 3 on), the movie view is `*(*(RentalRecord+0x5C)+0x48)+4`.
  GFxMovie's `SetVariable` is vtable `+0x40` `(path, value *, set type)` and `GetVariable` `+0x44` `(value *, path)`
  (the game's `_root.canbtn.disabled`, `_root.xpos`); the movie root's Invoke `0x5CB1B0` `(name, args, count)` takes a
  path. On `_root._currentframe` 3–8 (the Borrowed tab; 9–14 are Lent) with no `_root.borrow_btn1`, it calls
  `_root.attachMovie("btn_main01", "borrow_btn1", 1000)` (the library's SoundButton with a return arrow, ~40 px),
  sets `_x` 470, `_y` 470, `helpText` "$14282", `clickSound` "SE_DECIDE", `overSound` "SE_CURSOR", and adds the click
  to `_root.click_return` (`addEventListener` with `_root` from GetVariable; managed values released with
  `0x5C98C0`). Attached clips outlive the timeline's frames, so `_visible` follows the tab. A pending row gets
  `call("BorrowSetFlag", row)`, the movie's own greying: invoked inside the callback it changed nothing (live).
- **Events** (main vtable slot 6, `0x7A7A2C` → `0x482030`): a front `0x2D6C` reaching RentalRecord makes it ask for
  its lists again (`0x482760`), and each `0x2D6E` answer is stored at `+0x6C + 0x18 × state` (`0x48211B`), counting the
  state up: past the two lists into the card view's vectors and, from state 5, past them. So once the screen
  asked for its lists (state ≠ 0) the wrapper keeps `0x2D6C` from it; the global dispatcher has already stored it
  (dataset slot 8). Without this a return, or any rental's term ending, while the screen is open would corrupt it.

Like the other wave hooks, the launcher's preflight (`inspect_game`) doesn't check these slots; the DLL does.

### The chat joins the Lobby

`tools/mod/chat-lobby.h` (pure parts in `chat-lobby-logic.h`, tested by `chat-lobby-test.c`) makes the chat window
enter room 1, the Lobby, by itself once per login, as a click in its room list would. Without it the first tab reads
"----" and a line typed on the map goes nowhere until the player picks a room. The server can't do it alone: ChatWindow's
`0x245A {room_id}` case acts only while its pending choice `+0x58` (an index into its room list) is ≥ 0,
else it just closes the list. It writes no code bytes: it replaces two ChatWindow vtable entries (`0x7A48DC`) in
`.rdata`, after checking each still holds the game's own function at the actual base (another build: nothing changes,
never a reason to refuse the game).

- **Armed** by the player-select reply `0x2266` (the global dispatch hook, `starter.h`): once per login.
- **The reset** (slot 14, `0x7A4914` → `0x435710`, thiscall `(window)`: clears the tabs and prints the three welcome
  lines when the map's chat appears) asks for the room list, `0x4F3C10` (`0x2135`, the chat agent `+0x12C` in EDI, plain
  `ret`), when the window is in no room (`+0x55` 0 or `+0x50` < 0) and picking none (`+0x58` < 0).
- **Events** (slot 6, `0x7A48F4` → `0x4358F0`, thiscall `(window, event)`, `ret 4`; the agent's `0x1004` events go to
  the dispatcher `0x435950`): once the `0x2458` case has filled the room list (`+0x13C`/`+0x140`, `0x34`-byte entries:
  `id` `+0`, `name` `+4`, `type` `+0x30`; it fills it with or without the list's movie at `+0x128`), the wrapper finds
  room 1 (type 0) and does what the click callback `0x437300` does: `+0x58` = its index, then `0x4F3CF0` (`0x2136
  {room_id}`, EDI the agent, the id on the stack, `ret 4`). The game's own `0x245A` case prints "In>>Lobby", names the
  tab and adds TRADE (live, 2026-11-22).
- A player already in a room, picking one, or who leaves or switches rooms later is left alone: it acts once per login.

### What else runs in the game (#68)

Evidence for a player whose shop disconnected them (#65): the game's own connection was aborted on their PC (Windows
error 10053) while the server and the helper's connection to it were fine, and a `winmm.dll` that wasn't ours sat in their
game folder. Windows loads anything the game imports from its own folder before Windows' copy, and any module in the process
can redirect the same network functions. The connection DLL therefore **looks, and changes nothing** (`tools/mod/survey.h`,
decisions and outputs in `survey-logic.h`, tested without Windows by `survey-test.c`):

- **Before it hooks anything** (`DllMain`, `survey_scan_imports`): for each import of `WSOCK32` (the game imports it by ordinal:
  `connect` 4, `send` 19, `closesocket` 3, and the rest of 1 to 23, `yo3_wsock32_name`), `WS2_32`, and the three
  functions the DLL redirects besides (`GetPrivateProfileStringA`/`IntA`, `InternetOpenUrlA`), where the game's import table
  points. In a module of Windows' folder it is as Windows made it; in another module, or in no module at all, another
  program has redirected it (`how: "import"`); in a Windows function whose first bytes jump (`E9`, `EB`, `FF 25`, the hot
  patch's `8B FF`, push+ret, mov+jmp eax: `yo3_jump_target`) into such a module, it is a detour (`"detour"`). Our own
  loader's redirects (a module exporting `Yo3LoaderInfo`) and this DLL's are not findings.
- **From `Yo3ModReady`** (outside the loader lock, before the game runs; `survey_run`): the modules loaded from outside
  Windows' folders with file size and SHA-256 (the DLL's own implementation: no CryptoAPI under the lock), the game folder's other
  `.dll` and `.asi` files whether loaded or not, and the 32-bit Winsock providers (`WSAEnumProtocolsW` and
  `WSCGetProviderPath` from this 32-bit process: the game goes through that catalog, the 64-bit helper never does).
  A provider is Microsoft's when its DLL is one of Windows' own (`mswsock.dll`, `rsvpsp.dll`, …) or, with no path, by name.
  Under Wine nothing is flagged (its own providers, and its `WSCGetProviderPath` answers with another text).
- **Outputs.** One log line each, on the game's standard error, which the helper keeps in the game's session log and so in a
  report (`[YO3 MOD] Survey: …`; the SHA-256 in groups of 16 with colons, because the log's credential filter hides 20 or more
  letters and digits in a row; paths are masked at export like the rest), and, when the helper names `YO3_SURVEY_FILE`, a JSON
  summary (`hooked`, `modules`, `providers`, `providers_known`).
- **The launcher judges** (`yo3-launcher` `survey.rs`), after the DLL says it is ready and before the game is released:
  an `import` redirect, or a SHA-256 on `KNOWN_BAD` (empty for now), **stops the start** with no way past; another
  program's `.dll`/`.asi` in the game folder, a detour (a security product named when its DLL's name is known,
  `yo3_security_product`) or a non-Microsoft provider **holds** it until the player chooses (`--accept-foreign`: Continue
  anyway); an overlay's DLL and the like are in the report only. The helper writes its `decision` back into the survey file;
  Desktop reads that when the helper ends, and shows its dialog (`foreign.rs`) instead of an error.
- **Tests.** `test-dll.sh` runs the test executable in a folder of its own with a fixture DLL (`survey-fixture.c`) and an
  `.asi` file beside it, then with the fixture's function patched into the executable's `send` import, then with a jump to it
  written at the start of ws2_32's `closesocket`: the report must name each (module, size, SHA-256) and nothing for the clean run.
  Not tested: a real non-Microsoft Winsock provider (Wine has none), security software.
- **Direct start**: the loader sets `YO3_SURVEY_FILE` beside the helper's ready file (`direct-PID-NONCE.survey.json`); the
  helper logs what it finds (`Survey: …`) after the DLL's acknowledgement and removes the file. Nothing stops or holds a game
  opened directly: there is no window of ours to tell the player in. The DLL's own log lines go nowhere there (no console).
- Not done: a host's "clean game" setting (the server's status naming what it refuses), the issue's slice 4.

### How the client receives a frame (#65)

The front and duel connections are non-blocking sockets read by EasyMultiPlayPacketBase::recv_nonblock (`0x535190`,
called from Yo3Client's slot 21, `0x51FD70`) into a 1 MiB packet buffer (`0x100000`, set up at `0x51FBD0` and
`0x535900`; free room is EasyMultiPlayPacketBuffer `0x533F00`). Each call: `select` (ordinal 18), peek 1 byte (EOF or
error there: log `0x813`, print `disconnect`, return -1); while the buffer holds under 4 bytes, peek 4 and read them
(a short peek: log `0x84C`, return 0 and try later; a short read: log, return -1); the body's length (`^ 0x77777777`)
must fit the room left, else `recv_nonblock length error` on stdout and -1; then **peek the whole body**, and only when
the peek returns all of it read it (a short peek: log `0x83A`, return 0; a short read: -1). -1 closes the connection.

- **It waits for a whole frame visible to a peek.** Microsoft's KB 192599 warns that peeking for a complete message on
  Winsock "might never succeed". The helper therefore writes each server frame to the game in one write
  (`yo3-launcher` `relay`), with a 2 MiB send buffer on its side, and the connection DLL gives the game's front and duel
  sockets a 2 MiB receive buffer before they connect, room for the largest frame the game takes.
- **Its socket reads check the CRT's `errno`, not Winsock's error** (`0x534C10` read, `0x534CE0` peek): `errno` 4 retries
  and drops what that `recv` returned, 11 returns early. Winsock never sets `errno`, so only a stale value from another
  CRT call could trigger either. Neither has been seen; the logs below would show it (`recv errno=%d`, `recv EINTR`).
- **Every failure logs through EasyMultiPlayWriteLog** (global `0x82D214`), which this build never creates, so the DLL
  supplies one (`netlog.h`). It writes the game's own message to the session log: `[YO3 MOD] Game network:
  EasyMultiPlay PacketBase::recv_nonblock 2067 (at 0x535227)` (2067 = `0x813`; the space keeps the log's credential
  filter, which hides 20 letters in a row, off the name); the waiting ones (`0x83A`, `0x84C`) five
  times at most, the rest forty. The DLL also logs each time the game closes its front or duel connection, with the bytes
  it left unread, the last socket error and the game code that called `closesocket`, found by scanning the stack for return
  addresses (the game has no frame pointers). Live, 2026-11-21, the server stopped under a game in the shop: `recv_nonblock
  2067`, then `The game closed its front connection: 0 bytes unread, last socket error 0; called from 0x534dc5 0x4f78c6
  0x401e95`.
- The server never sends a frame the buffer can't take: a body past `0x100000 - 4` is logged as an error and left out
  (`net.rs`, `CLIENT_FRAME_MAX`). The largest in a session is the shop's avatar items, about 60 KB.

### Native duel observer

`tools/mod/duel-observer.h` installs two byte-guarded hooks for ver.1.153: the CALL at `0x514212`
(`E8 79 00 00 00` → fastcall dispatcher `0x514290`, ECX=id, EDX=params, scene on stack, `ret 4`) and the callback
assignment at `0x5159EA` (`B8 30 5A 51 00` → thiscall update `0x515A30`). Ordinary dispatch and updates call the
original functions. A mismatched site rejects DLL readiness before login. Our checkpoint `0x5F0` only records
expectations; the update wrapper reads state after the original callback. No game state is written by the observer.

The independently compiled reader in `duel-snapshot.h` accepts a checked read callback (`ReadProcessMemory` of this
process in the DLL). DuelScene has card manager `+0xBC`, cockpit `+0x8C`, battle `+0xA8` and chain `+0xC0`.
Battle `+0x74`, chain `+0x58` and every card's `+0x114` gate settlement, including cards travelling to piles.
The card-manager map has sentinel `+0x44`, root sentinel `+4`, nodes left `+0`, right `+8`, card object `+0x10`.
Traversal is bounded (512 cards, stack 64); unreadable or oversized structures refuse a snapshot. It never calls
`0x515300`, whose busy check also clears effects and can hide cards.

Card-manager slots start at `+0x4C`, 17 pointers per physical player. Hand vectors are begin/end `+0xE0/+0xE4`,
24 bytes per player. Card UID is `+0x30`, place u16 `+0x82`, visible mode `+0x44`, displayed monster stats
`+0xB8..+0xC8`. The per-card counter tree is `+0x130` (node kind `+0xC`, amount `+0x10`; getter `0x4FB6C0`).
Pile counts are `+0x14` for zones 12–14 (`0x4FE4D0`); banished zone 15 is a different class, count `+0xC`
(`0x4FE8A0`). Cockpit LP is `+0x50/+0x54`. Logical/physical player numbering XORs the local player at
`*(0x82D2C0)+0x1B0`. Every offset is from the local binary and is covered by synthetic-memory tests; live checks
remain necessary to establish that the selected fields match the running game's behavior.

The DLL writes `[YO3 CHECK]` records to stderr, already captured by the launcher's normal session logger and both
diagnostic ZIP scopes. Busy state waits at most 15 seconds; failures and superseded checkpoints are explicit.
The schema and comparison limits are in [the protocol](protocol.md#duel-connection). The reader fixture mutates
visibility, LP, stats, counters, busy flags and invalid pointers, and asserts that its reads never change memory.

### The patcher

The stock client checks for updates at start (`PatchUpdateScene`, constructor `0x4B35E0`, `patch_main.swf`). It reads
`PatchUpdateUrl` and `PatchDataUrl` from `yo3.ini`, which the connection DLL answers with `…/session/<cookie>/yo3/`
and `…/ygoda/patch/` on the launch's local HTTP port (`tools/mod/auth.c`, `override`); every other WinINet URL is
rewritten under the same prefix, and any URL holding `/yo3.idx` becomes `…/yo3/yo3.idx`. It fetches `yo3.idx` first
(the first WinINet request), and the downloader `0x497090` writes whatever a URL answers to a stream. What the patch
component (`patch::PatchComponent`) then does with a patch archive in `temp\` (`????????.ypt`, ZIP local headers
`PK\3\4` or `0x4034F59`), from the code (security review, #39):

- `0x4EA470` creates each entry under **its own name**, relative to the game folder (`CreateFileW(name, GENERIC_WRITE,
  …, CREATE_ALWAYS)`), and writes its bytes. No check on the name was found: a name with `..\` or a DLL beside
  `yo3.exe` would be written as given.
- `0x4EAA50` takes the entry for `yo3.exe`, writes it to `yo3Temp.swap` and moves it over `yo3.exe` (`MoveFileW`, the
  old one kept as `yo3Temp.back`, or `MoveFileExW` with replace).
- `0x4EAE30` moves files under `package\` into place (`MoveFileW`) and writes `merge.log`.

So whoever answers the index and the patch files can write into the game folder, including the executable and DLLs it
loads: code execution on the player's PC. The launcher's proxy therefore answers `yo3/yo3.idx` from the game folder's
own `yo3.idx` on every Play (normal and direct start, `Proxy::local_index_from`), so the game has nothing to update,
and carries only `web/`, `duellogs/` and `app/` to the server; `ygoda/patch/…` and anything else get a local 404
(`yo3-launcher` `relayed`; test `proxy.rs`
`the_patch_check_gets_the_games_own_index_and_no_server_reaches_the_patcher`). Without the game's own index the check
gets a 404 and ONLINE says it needs an Internet connection, as with no helper. A patched index was not tried on the
private client: the code was enough to close the path. **Status: code (the write path), live (the local index reaching
the map).**

## The field tracer

`tools/tracer/` builds `yo3trace.dll` (32-bit, mingw-w64) and `yo3trace-launch.exe`. The launcher starts `yo3.exe`
suspended, loads the DLL with `CreateRemoteThread(LoadLibraryA)` and resumes it, so the game folder is untouched.
The DLL patches two entry points, and only if their original bytes match:

- **`0x41A980` (`find`):** the hook runs the real lookup through a trampoline to learn found/missing. It reads the
  key from the MSVC 2008 `std::string`: 28 bytes, the text inline at `+4` when the capacity at `+0x18` is under 16,
  otherwise a pointer at `+4`, with the size at `+0x14`. It names the value's class from the game's RTTI and reads
  the value itself when the key is `request`. For getter calls the key is the getter's by-value argument, so the
  asking code is the return address at `key − 8`.
- **`0x410D60` (`item[n]`):** logs the index, the list size and the caller.

Each line carries the dataset (tree) pointer. A lookup on the same dataset as the message's `request` field is a
top-level field (`top`); anything else is an item or local data (`sub`). Lookups that happen with no message being
parsed (local settings such as `bgm`, `se`, `lastLogin`, the saved login, `yo2Succession`) appear as
"outside any reply".

## The UI probe

`tools/mod/ui-probe.h` (in the connection DLL) answers `tools/client/vclient.sh ui`: what the client shows, as text,
so a check doesn't need a screenshot. It is off unless `YO3_UI_PROBE` names an existing folder; `vclient.sh` sets it to
`logs/<instance>/ui` for its private clients, and the launchers never set it. Its own thread checks that folder for a
`request` file every 200 ms, then writes `ui.json` (a temporary file renamed into place). `vclient.sh ui` prints it
readably, `vclient.sh ui --raw` as JSON. Its readers (`ui-state.h`) take a checked read callback and are tested on
synthetic memory (`ui-state-test.c`, run by `tools/mod/test-dll.sh`); it writes no game memory.

- **Components.** Every screen, window and dialog is a component made by the factory `0x4EEB90` and added to its
  parent with the parent's vtable slot 14 (`+0x38`). For a container (`BroadcastComponent`, add-child `0x403C00`,
  remove `0x403C50`) the child's parent goes to child `+4` and the child into the parent's `std::map`: head node
  pointer at `+0x44`; nodes left `+0`, parent `+4`, right `+8`, key `+0xC`, child `+0x10`, nil flag `+0x15`
  (iterator `0x404570`). The root is Yo3App's component part, `0x82F7C0` (Yo3App is the static `0x82F760`). The
  probe opens only components whose slot 14 is `0x403C00` (others, such as `TitleScene` and `InfoWindow`, keep other
  data at `+0x44`), names each one from its MSVC RTTI (vtable `-4` → locator, `+0xC` → type descriptor, `+8` →
  `.?AVName@@`), and leaves a subtree it can't read closed (`unreadable_children`) instead of failing the walk.
- **Texts.** The text lookup `0x51E6C0` (stdcall `(table, id)` → `wchar_t *`, `ret 8`) is detoured when its nine
  prologue bytes are `55 8B EC 83 E4 F8 83 EC 0C`; the last 40 lookups are kept with their id and age, newest last.
  They show the dialogs, labels and menus that just appeared ("#1109: Change card position", "#0: Attack"). A
  mismatch leaves the lookup alone and reports `text_hook: false`.
- **Flash calls.** The screens are Flash (GFx) movies, and the game's code talks to them through one invoke,
  `0x5CB1B0` (thiscall `(movie, name, args, count)`, `ret 0xC`), detoured when its first six bytes are
  `83 EC 38 56 8B F1`. Its name is `"call"` for an ActionScript method, which `0x539760` (234 callers) passes as the
  first argument (`setButtonLabel` from `0x4C35C0`), or `"respond"` for data the movie asked for. Each argument is a
  16-byte GFx value (copied by `0x40F0F0`): `+4` type, `+8` value; 2 bool, 3 number (double), 4 `char *`,
  5 `wchar_t *`, bit `0x40` managed (`0x44`: `char **`). The last 60 calls are kept as `method(arguments)`, a call
  repeated straight after itself counted instead of kept again; `MousePos`, sent every frame, is left out.
  `flash_hook: false` reports a mismatch, and `YO3_UI_PROBE_HOOKS=0` leaves both hooks out.
- **Duel.** When a `DuelScene` is in the tree, each side's 11 field slots from its card manager (`+0xBC`), as the
  [native duel observer](#native-duel-observer) reads them: uid `+0x30`, place `+0x82` (`0x4000` face up, `0x8000`
  defense). `you` is the local player (`*(0x82D2C0)+0x1B0`).

Live (2026-11-13, NPC duel): the map showed `MainGamen` with its menu labels, the NPC screen `SelectNpcScene` and
`CutinForNPC`, the duel `DuelScene` with `DuelFieldManager`, `DuelCockpit`, `DuelBattle` and the others; after
Goblin Attack Force attacked, the probe showed it `up ATK`, then `up DEF` once the Battle Phase ended, as drawn.
The Flash calls showed Zigzix's and Kaiba's talk (`respond(1, "Zigzix", …)`, `setSpeech("Seto Kaiba", …)`, every
line at once), the hovered opponent's record, the "Start Duel?" box opening (`showDialog()`), the chosen deck's
counts (`respond(93, 40, 0, 18, 0, 9, 13)`), the janken (`Janken.resolveJanken("guuuu", "choki", "win")`) and the
duel's opening draws (`setDeckCount`).

Limits: words written into a movie itself never pass through the probe: the "Start Duel?" box shows as
`showDialog()`, its wording only on screen or in the extracted movie (ffdec). A talk's lines come all at once, so
which one is showing still needs a click count or a screenshot; the card info panel's hover text uses neither path.
A text id or a call seen means it happened, not that it is still on screen (read `ago_ms`). The duel field is the
card manager's data, not the rendered card's rotation or animation.

## Rendering, the window and the mouse

The client draws one fixed frame, 1024×768 (or 1024×600, the third **Screen** choice in the title screen's
**Options**), with Irrlicht's Direct3D 9 driver; Scaleform (GFx 3) draws the Flash screens through the same device.
Everything below lays out, draws and hit-tests in that frame's pixels.

- **Startup and the frame.** `0x40B370` refuses screens smaller than 1024×600 (`GetSystemMetrics`), then builds up to
  three Irrlicht creation parameters from the `screen` option (`0x40B280`), `0x24` bytes each: driver type 4
  (Direct3D 9) `+0`, width 1024 `+4`, height 768 or 600 `+8`, 32 bits `+0xC`, fullscreen `+0xE`, vsync `+0x10`.
  `0x53C270` makes the first device that works; the mode it got goes back into `screen` (0 Full, 1 1024×768, 2
  1024×600). Live (Xvfb, which offers no 1024×768 display mode): Full failed at 768, then at 600, and the game opened a
  1024×600 window. Changing **Screen** in Options and confirming ends the game; the mode applies at the next start
  (live, with and without the mod).
- **The window.** `CIrrDeviceWin32` (`0x53AF30`) registers the class `CIrrDeviceWin32` with the window procedure
  `0x53A720` (`0x53B05A`), sizes the window from the client size with `AdjustWindowRect` (`0x53B0F1`; style `WS_POPUP`
  for fullscreen, else `0x06C80000`, captioned and not resizable), centres it, creates it (`0x53B158`) and moves it
  there. `0x53BE20` (Irrlicht's `setResizable`) recomputes the same with the screen size. The window procedure's
  `WM_SIZE` forces any client size other than the driver's screen size back to 1024 wide (`SetWindowPos`); Irrlicht's
  own resize (`0x53B600`, "Resizing window") ignores sizes while the screen is 1024 wide.
- **Direct3D 9.** `yo3.exe` doesn't import `d3d9.dll` (only `d3dx9_40.dll`, for `D3DXAssembleShader` and
  `D3DXLoadSurfaceFromMemory`): the driver's init loads it (`0x5441CC`) and asks `GetProcAddress` for
  `Direct3DCreate9` (`0x5441FF`). Present parameters sit at driver `+0x144` (back buffer = the screen size), the
  device at `+0x480`. Fullscreen first requires an adapter mode of exactly the screen size (`EnumAdapterModes`).
  Frames go out through `Present` at `0x544FF6`; "Resetting D3D9 device." is the reset path.
- **Viewport and drawing.** Irrlicht's `setViewPort` (`0x545680`) clips to the screen size and calls `SetViewport`
  (`0x54573F`); render-to-texture (`0x545460`) saves the back buffer with `GetRenderTarget`, draws into a texture
  and restores the saved pointer. Only transformed vertex formats are set (`0x152`, `0x252`, `0x140352`, around
  `0x547000`): no pre-transformed (`XYZRHW`) vertices, so the viewport places every 3D and 2D draw. 2D drawing uses
  Irrlicht's orthographic mode, built from the screen size.
- **The 3D camera.** The camera (`0x585D10`) takes its view rectangle (`0x5866B0`) from the driver's current
  render-target size (vtable `+0xD4`), 1024×768 when there is none; its aspect ratio follows. The duel field's
  picking (DuelFieldManager `0x50B700`, pick areas `0x509CD0`) casts from mouse positions in the frame.
- **Scaleform.** Each Flash screen (`GFxMovieSprite`, vtable `0x7B1B3C`) sets its viewport through `0x473F10`: a
  `GViewport` with the buffer = the driver's screen size, the position = the sprite's, the size = the movie's own,
  scale and aspect 1.0, passed to the movie's `SetViewport` (vtable `+0x64`) by `0x4EFE60` (load) and `0x4F00C0` (move).
  `0x4F0650` draws it (renderer `+0x10`, movie `Display` `+0x90`, renderer `+0x14`); the renderer sets the Direct3D
  viewport itself (`0x6D56D0`, seen live). Mouse input reaches a movie through `HandleEvent` (`+0xA4`) as the position
  minus the sprite's (`0x4F04C0`); keys and text through `0x4F03A0`.
- **The mouse.** Irrlicht's events carry the client position from the messages' `lParam` (`WM_MOUSEWHEEL`: screen
  position minus the client origin, `ClientToScreen`); the game's receiver `Yo3Mouse` reads `X` `+4`, `Y` `+8`
  (`0x40ED10`). The cursor control (`CCursorControl`, vtable `0x7B5200`, made by `0x53A2C0`) works out the position
  as `GetCursorPos` minus the window rectangle and a border (`0x53A620`): `SM_CXDLGFRAME`, `SM_CYDLGFRAME` +
  `SM_CYCAPTION` for a window, none in fullscreen (`0x53A37B`); `0x53A480` moves the pointer with `SetCursorPos`
  (a duel moves it onto the card that can act). The game hides the system cursor (`WM_SETCURSOR`) and draws its own
  (`MouseCursol`), whose position it sends to Flash every frame (`MousePos`). The IME's composition window goes to
  the text cursor (`0x53C300`, `CFS_POINT`).
- **Not used.** Irrlicht's `createScreenShot` (`0x549140`, driver vtable slot 87) exists, but the game has no
  screenshot feature that writes a file.

### Higher resolution

`tools/mod/resolution.h` (in the connection DLL), off unless `YO3_RESOLUTION` is set: `WIDTHxHEIGHT` (320 to 8192 each)
for a window of that client size, `auto` for the largest the screen holds, `off`. The launcher sets it from
`--resolution`, Desktop from **Game setup → Display**. The game is told nothing: it keeps its frame, and the window and
back buffer are larger, the frame scaled to fit and centred, with black bars when the shapes differ
(`tools/mod/resolution-fit.h`, integer arithmetic; tested by `resolution-test.c`).

- `AdjustWindowRect` given a client of exactly the game's frame returns the real size: the request, or the work area
  for `auto`, or the whole screen when the game asks for its fullscreen window (`WS_POPUP`), which stays a borderless
  window.
- The window stays in the work area: Irrlicht centres it on the whole screen and moves it there again after showing it,
  so a window sized for the work area (`auto`) reached under the taskbar on the Windows 11 VM. `WM_WINDOWPOSCHANGING`
  moves each move the game makes (and the first showing) into the monitor's work area (`yo3_keep_inside`); the
  player's own drags (`WM_ENTERSIZEMOVE` to `WM_EXITSIZEMOVE`) and the fullscreen window are left alone.
- `GetProcAddress("Direct3DCreate9")` returns a wrapper whose interface has `CreateDevice` wrapped: the back buffer is
  the window's real client area, windowed. The device's `Reset`, `Present`, `SetRenderTarget`, `Clear`,
  `SetViewport`/`GetViewport`, `SetScissorRect`/`GetScissorRect` and `StretchRect` slots are wrapped: while the back
  buffer is the target, rectangles from `yo3.exe` (Irrlicht and Scaleform) are scaled into the content area and read
  back in the frame. Calls from anywhere else pass untouched: a duel at 1600×900 showed `SetViewport` and
  `SetScissorRect` called from outside `yo3.exe` (`0x759B4F0B`, `0x759B4F44`; most likely DXVK's state blocks, which
  re-apply what they captured through the same slots), and those values are already real. `Present` clears the bars.
- The class registration gets a window procedure in front of `0x53A720`: mouse messages arrive in the frame, and
  `WM_SIZE` reports the frame. Every window of the class is translated: each mode the game tries makes a new one.
- `GetCursorPos`/`SetCursorPos` give and take positions that make `0x53A620`'s sum come out in the frame;
  `ImmSetCompositionWindow`/`ImmSetCandidateWindow` place the IME in the real client area.
- Every site above is checked byte for byte first; a different executable, or a value the DLL doesn't accept, stops
  startup with `stage=mod-resolution`. Direct start loads the DLL at the first `[Yo3]` INI read, after `0x40B370`
  made the window, so the DLL logs there that it isn't used (see below for what installs it instead).

Live (2026-11-14, Wine with DXVK on a private 1920×1080 Xvfb display): `1440x1080` (Wine kept the window inside the
screen at 1440×1060: frame 1413×1060, 13-pixel bars), `1600x900` (1200×900, 200-pixel bars), `auto` (1397×1048),
the game's Full through a Wine virtual desktop (a borderless 1920×1080 window, frame 1440×1080) and its 1024×600 mode
(1788×1048). The title, Options, login, map, main menu, deck editor (hover preview, card drag), NPC talk, "Start Duel?",
deck choice, janken and duel field (hand-card menus, Summon by click, the BP button, the game moving the pointer)
drew and answered where they should. Flash shapes and text are drawn at the new size; bitmaps (card art,
backgrounds) are scaled up from their files, and render-to-texture effects keep their texture's size. Not checked:
native Windows, an IME, more than one monitor.

**Opening `yo3.exe` directly (#16).** The loader (`winmm.dll`, which Windows loads before the game runs) installs the same
hooks itself: `loader-start.h` `direct_resolution` reads `resolution` from the direct-start profile (`profiles/direct.json`,
which each Play writes from Settings' screen size, `direct.rs` `direct_resolution`) and calls `resolution_install_from`,
the function the DLL's `resolution_install` uses with `YO3_RESOLUTION`. A profile without one, a game whose bytes differ,
or a configuration the loader can't read changes nothing: the game runs at its own size, and one line (`[YO3 LOADER]
resolution … not used: …`, on a console only) says why. Later the DLL loads, finds `YO3_RESOLUTION` unset (direct start
doesn't pass it) and leaves the hooks the loader installed. The size is the last Play's, not Settings' now: changing it
needs a Play before opening `yo3.exe` alone.

### Numbers in the duel: LP and ATK (investigated 2026-11-24)

How the duel shows LP and ATK, for showing more than 99999 (the engine's infinite ATK is 9,999,999). Nothing is built on
this yet. All of it is drawn by Flash movies (Scaleform GFx), which the game feeds numbers through its one invoke
(`0x5CB1B0`, see "The UI probe"):

- **LP**: DuelCockpit `0x502EB0` handles the LP commands. `0xF` stores the LP as sent and calls `0x5035A0`, which
  invokes `setLp` (with `setLpColor` and `setLpGauge`) in `data/gfx/duel_cockpit.swf`. `0x10` adds a change and clamps
  the result to 0..99999 (the only 99999 in the binary), then `0x5041D0` starts the counter's animation (frames =
  |change| / 20 + 1, at most 240, so the counter alone can't take minutes) and `0x503980` invokes `setLpAnim`. The shown
  change goes to `0x503AD0` and the movie's `showDamage`. In the movie, `setLp` sets `lpCounter.number`, a `NumberSprite`
  (ActionScript class): it clamps to its `maxNumber` and draws each digit as a clip jumping to a frame `num0`..`num9`
  or `minus` of a bitmap font sprite (`duel_font_b_00`, `duel_font_e_anim_00`...). There is no frame for "∞" or "+".
- **ATK and DEF under a field card**: `0x50C190` invokes `setProp(prop, value...)` in `data/gfx/duel_fieldinfo.swf`
  (callers such as `0x4FBAA0` send the card's ATK, card + `0xBC`, as prop 6 and DEF, + `0xC0`, as 7, orgAtk/orgDef as 4
  and 5, unclamped). The movie sets the card info's `atk`, and `txAtk` is a text field bound to that variable (right
  aligned, fixed width, no maximum length: 9,999,999 shows as its last five digits, 99999), coloured white, blue or red
  against `orgAtk`. Its font is `$DefaultFontBold`, which `0x4068D0` maps (`0x5C7850`) to a system font by language;
  `$DefaultFont` comes from `data/gfx/gfxfontlib.swf`, an embedded Arial of 7,648 glyphs, "∞" among them.

So a text field can show "∞" given a string instead of a number (the invoke takes `char *` and `wchar_t *` values, as
the connection DLL's Rental Card List Return button already passes), while the LP counter and the damage numbers would
need a picture the game doesn't have.

## Data formats

- **`.ypk`**: ZIP local headers with a `YO\x03\x04` signature and no central directory; method 0 (stored) or 8 (raw
  deflate). Read with the `yo3-data` crate or `tools/re/ypk.py`.
- **`.y3d` / `.y3a` / `.y3r`** (preset decks / packs / regulations): one framed message each (see the protocol).
  - Decks and packs list **CARD_Prop rows**; regulations list **Konami card IDs**.
  - One deck ships as `data/recipe/232.y3a`.
- **`CARD_Prop.bin`**: 8 bytes per row. `a = u32 LE`: card ID = `a & 0x3FFF`, ATK = `((a >> 14) & 0x1FF) × 10`,
  DEF = `((a >> 23) & 0x1FF) × 10`. Online 3 uses 14 bits for the ID where Tag Force uses 13. 4,824 cards; IDs run
  3900–9448 (4007 = Blue-Eyes, 39xx = tokens).
- **`CARD_Indx_<lang>`**: `{u32 name offset, u32 desc offset}` per row into `CARD_Name_<lang>` / `CARD_Desc_<lang>`
  (UTF-16LE, NUL-terminated).
- **`CARD_IntID.bin`**: a row (u16) per Konami id from the table's lowest id (`base + i`); 0 for an id with no card.
  The loader computes the id bounds (`+0xD0`/`+0xD4`) from the rows (`0x536FF0`), and `0x537230` looks a row up by id.
- **`CARD_Sort_<lang>` / `CARD_Sort2_<lang>`**: the rows (u16) in two display orders: `Sort2` alphabetical by name,
  `Sort` the game's own (alternate artworks left out). Row 0 leads both.
- **`CARD_Indx_<lang>`** has one more entry than rows: the name and text files' ends.
- **Cards the game never had (#75).** The loader `0x40DEE0` reads each card file to its end into a growable array
  (`0x537770` 8-byte rows, `0x537890` 4-byte, `0x537970` 2-byte, `0x537A60` the text blobs), so longer files are all a new
  row needs: no count is fixed in the game. The connection DLL serves the launcher's extended copies through the file
  system's `createAndOpenFile` (`0x5505E0`, `data/bin#/CARD_...`, `tools/mod/added-cards-logic.h`). Edit Deck lists trunk
  rows `1 .. len - 1` (`0x4125D8`) and reads a count at `row - 1` (`0x410DB0`): the trunk's arrays hold one byte more than
  the highest row (before, the game's last card, Blue Rose Dragon, was never listed).
- **`DLG_Text_<lang>`**: 597 duel-engine prompt strings, indexed by `DLG_Indx_<lang>`.
- **`data/text/txttbl_<lang>.bin`**: the UI text table (the `$NNNNN` ids and every `name_id`/`discription_id`):
  `u32 count`, then `count` pairs `{u32 id, u32 byte offset}`, then 4 bytes, then UTF-16LE NUL-terminated strings
  from byte `8 + 8 × count`. English has 15,863 entries; pack names are 23000+, preset deck names 24000+ (the
  description at `name_id + 1`).
- **`data/extra/yo3pbininfo.bin`**: 4,096 eight-byte entries (item catalogue, text ids 26000+).
- **`data/bin#/s_cardnum.inc`**: a header left over from Konami's duel-engine source.
- The card pool ends around Storm of Ragnarok (early 2011): no Xyz.

## Behaviours worth knowing

- Every field zone, empty or not, is a click target (pick areas `0x509CD0`; the hover place `0x50B510`); a click sends the
  card command of the command map's entry for that zone with the zone itself (`0x4F9510`: `{CommandID, player,
  position: zone, index 0}`), so a zone can be asked for with an ordinary command map (see "Places" in
  [protocol](protocol.md)). DuelInvalidZone (`FieldManager+0x34`, vtable `0x7B30D8`) draws unusable zones from the
  mask command 1006 stores at engine `+0x124`.
- The stock duel dialogs show only the client's own texts (`0x519870`); the required DLL supplies server texts for
  messages and option labels (see [Server texts and NPC duel logs](#server-texts-and-npc-duel-logs-issues-10-and-12)). The position dialog still has two
  buttons (`dlg_selstand.swf`), the type dialog one pick (`dlg_seltype.swf`), and the card list orders picks itself
  (`dlg_cardlist.swf` `onCardClick`, badges 1..N): see "What the dialogs can't do" in [protocol](protocol.md).
- `0x14` (the clocks) is handled when popped, one server message per tick (`0x4F1730`); only engine commands wait for
  the animations (`0x523541`). The stock protocol has no animation-completion request; the required DLL observes
  settle marker `0x5F1` and sends Revival request `0x7A13` after the batch settles (see
  [Animation settle answers](#animation-settle-answers-issue-11)).

- The required mod skips the stock username/password form after **ONLINE**; the revival launcher authenticates
  first. The native connection/error/player-selection flow stays in use. Up to 4 players per account.
- The client treats formatted server date/time strings as JST; duel-log epoch timestamps render in local time.
- The client keeps its own data under `<My Documents>\Yu-Gi-Oh! ONLINE 3` (`SHGetFolderPath` `0x8005`, `0x52B8E0`;
  one folder per server with the connection DLL, see "A Documents folder per server"):
  `options.y3o`, `users\`, and per player `players\<pname>\` with `decks`, `mail`, `regulations`, `duellogs`,
  `local`, `avatars` and `<pname>.y3p`. Under Wine, My Documents is usually a link to the Linux `~/Documents`, and
  every prefix made by `vclient.sh prefix` shares it.
- Startup `0x40B820` loads `options.y3o` through `0x52D9F0` (language, screen/audio settings and `lastLogin`), then
  reads the selected cached stock user through `0x52DDD0`. These files still influence startup with the login form
  skipped. Player config read/save `0x52E0B0` / `0x52E200` includes `deckInfo.cursor_deck_name`; preserve it together
  with local decks and `revival-starter.initialized`. Desktop's [local reset](launcher.md#reset-local-game-data)
  keeps the entire player tree by default. This is a recovery option; old local data has not been established as
  the cause of the reported native access violation.
- Text values are read as C strings: anything after a NUL is lost. Use binary for blobs (`avatar1`, `pbin`,
  `regulations`).
- Unanswered polls time out with a dialog naming the decimal request id, and the client then drops back to the title.
- The stock game never sends login credentials. The required launcher sends its account key inside TLS; see
  [authentication.md](authentication.md).
- Several clients can run at once (unnamed mutexes only, so no single-instance guard). Under Wine each needs its own
  prefix when it gets its own X display; otherwise Wine dies with X `BadWindow` errors.
- The player list hides the searcher's own entry and entries with `player_id` 0, and pages by 100.
- The main screen's map is eight hex buttons, `btn0`–`btn7` in `main.swf`. Its `sisetuStart` (called by the client)
  wires them to GameDelegate calls: `btn0` Shop (`GoShop`), `btn1` Official Event, `btn2` Championship, `btn3` User
  event (all three `MenuSelect`, each only if `_root.ofseve_enable` / `wcup_enable` / `ueve_enable`), `btn4` NPC duel
  (`GoCpuDuel`), `btn7` character settings (`GoCharMake`, the tent on the right; Avatar Settings, CharMake.cpp:
  update `0x51C960`, replies `0x42AC70`; its part and title lists show only owned items, and `0x433AC0` checks the
  worn ones against the pbin); `btn5` and `btn6` have no handler. While the screen is
  locked, `sisetu_block.swf` lays an invisible block over them (`sisetuEnable` false).
- In chat, the client prints its own lines only from the server's `0x2455` echo, its own name in cyan and others' in
  white.

## Crash history (and the rule each one taught)

| Symptom                                                                                           | Cause                                                                                                                                                                                                                                                                                                                 | Rule                                                                                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Null read at `0x535210` when a connection drops (the host stopped or crashed, the network went)    | Every failed receive logs through the error logger at `0x82D214`, which this build never creates (only `0x5339A0` makes it, and nothing runs that) | The connection DLL fills the null global with a do-nothing logger (`tools/mod/netlog.h`); the client then shows its own disconnect dialogs and returns to the title screen (live, 2026-11-13) |
| Null read at `0x535210` during automatic login                                                    | Local gate closed immediately after `0x21FE`; its EOF raced the redirect handler and the receive thread dereferenced the null error logger at `0x82D214`                                                                                                                                                              | Keep the redirect connection open until its client closes it; launcher shutdown must also release accepted sockets                                                                                                             |
| Crash at `0x46C750` after selecting a player                                                      | `avatar1` sent as text, so the array came back empty                                                                                                                                                                                                                                                                  | Blobs are binary (vtype 8)                                                                                                                                                                                                     |
| Null read at `0x4727DE` entering the main screen                                                  | player record without `pbin`                                                                                                                                                                                                                                                                                          | Send every field the loader reads                                                                                                                                                                                              |
| Crash in the 9000 handler (if no items)                                                           | it dereferences `item[0]`                                                                                                                                                                                                                                                                                             | Lists the client indexes need at least one record                                                                                                                                                                              |
| Null read at `0x4F54C8`                                                                           | `0x2398` without `regulations`                                                                                                                                                                                                                                                                                        | Check the **global** dispatcher too, not only the screen's own handler                                                                                                                                                         |
| (prevented) `0x27D9`                                                                              | item[0] read unchecked                                                                                                                                                                                                                                                                                                | Route every new reply through `dispatch.py` before shipping it                                                                                                                                                                 |
| Null read at `0x45E734` once a search listed a player                                             | friend list never pushed, so dataset slot 6 was NULL                                                                                                                                                                                                                                                                  | A reply's first non-empty list can wake code that needs other state: test with real items live                                                                                                                                 |
| Null read at `0x48F0F6` opening the shop                                                          | rental records `0x2D6C` never pushed, so dataset slot 8 was NULL                                                                                                                                                                                                                                                      | Before shipping a screen, check which dataset slots it reads (`0x413FE0`, or `[+0x16C] + 4·slot`)                                                                                                                              |
| CRT abort `0xC0000417` opening character settings (from `0x42923F`)                               | `avatar1` had 17 entries; the screen reads 17–40                                                                                                                                                                                                                                                                      | Checked-index failures show as `_invalid_parameter`: the tracer's `E` line gives the caller                                                                                                                                    |
| Write to `0x42F` at `0x4F6A82` opening the NPC duel                                               | trunk without `cards`/`noTrade`, so the collection builder wrote through NULL                                                                                                                                                                                                                                         | Per-card arrays are sized by CARD_Prop rows                                                                                                                                                                                    |
| Division by zero at `0x4BFDC4`, hovering an NPC                                                   | all three levels sent with `need_ex` 0; `getNpcInfo` divides by their sum                                                                                                                                                                                                                                             | Zero is not a safe default for a divisor: read what a field feeds before choosing a value                                                                                                                                      |
| Null read at `0x516185` (TaskQueue append) in the CPU's turn, the move after a draw into its hand | MSG_SHUFFLE_HAND not translated: after the CPU's Sangan search the hands fell out of step, and a card's move out of the hand named the engine's index; the client took a hand card out by that index (`0x5005D0`), kept the moved card in its hand list, and the next renumbering (`0x500640` → `0x4FB7F0`) ran on it | The client trusts places, not uids: every engine message that reorders a zone needs translating, and a move names the place the client has the card at. The duel check (`duelcheck.rs`, `tests/duel_check.rs`) now enforces it |
| Null read at `0x485CA1` clicking the eagle at the top left, or on a BOOSTERPASS code accepted     | `premium_expire` never sent: the client parses it (`0x485B50`), `mktime` fails and `localtime` returns null (`0x485C00`)                                                                                                                                                                                              | A field a screen parses (a date, a number in text) needs a valid value, not just to be present; read where every `0x23F0`-like global case stores what other screens read later                                                |
| (no crash) only the chat answered clicks, for a player with a friend                              | the Online 2 takeover step (`0x472780`) sent `0x24C4`, which went unanswered, so the main screen's startup never reached its unlock                                                                                                                                                                                   | An `UNHANDLED` request during startup is a lock: answer every request a startup step can send, and set the flags (`pbin[0x4B]`) that skip steps we have nothing for                                                            |
| (no crash) every click on the map ignored                                                         | `0x2775` answered with `0x2774`, so PresentWindow never closed and the main screen stayed locked                                                                                                                                                                                                                      | A screen that ignores input may be waiting on a reply: `YO3_TRACE_HITS` on its steps shows where                                                                                                                               |
| Write at `0x50E639` on native Windows, closing the window mid-duel with the higher resolution on | At the end of WinMain DuelManager's destructor flags its duel component `[this+0x70]` for removal (`+0x28`), but that component's pool was already decommitted (eax in a `MEM_RESERVE` region; stack back to `0x401E95`). The resolution mode only changed where the component landed; the stock run can get away with it | `tools/mod/teardown.h` (always on): the write happens only while the memory is there (`teardown-guard.h`, `teardown-test.c`). Live: 2 of 2 crashed before, 0 of 2 after. The game's own memory at exit is not ours to trust |
