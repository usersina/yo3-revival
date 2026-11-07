# Client internals

What we know about `yo3.exe` and its data. Addresses are virtual addresses in the unpacked exe (image base
`0x400000`). Decompiled C for every function is in `re/yo3_decompiled.c` (git-ignored; regenerate with the Ghidra
export, see [CLAUDE.md](../CLAUDE.md)).

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

| Address | What |
|---|---|
| `0x40F320` | `UserData` singleton getter; the object is static at `0x82EC78`, built by `0x51CCB0` |
| `UserData+0x28` | the selected `PlayerData` (vtable `0x7B385C`); a second one sits at `+0x430` |
| `PlayerData+0x40` | pointer into the player record's `pbin` bytes |
| `UserData+0x838…+0x8C8` | datasets, `0x24` apart; `+0x8C8` is the trunk (`YO3Trunk`, a plain `YO3NetObject`) |
| `0x51C260` | PlayerData loader: `pname`, `title`, `avatar1`/`avatar`, `pbin`, `speech%02d` ×23, `comment%02d` ×2, `n_win/lose/draw` |
| `0x40F290` | read the `request` id of a received message |
| `0x408F50` / `0x409020` / `0x4090F0` / `0x40F1D0` | typed field getters: uint / int / string / raw value (NULL when missing); all call `0x41A980` |
| `0x41A980` | `std::map<std::string, Value*>::find`: ESI = tree (map+4), `[esp+4]` iterator out, `[esp+8]` key, `ret 8`; end node at `tree+0x18`, value at `node+0x28` |
| `0x408640` / `0x4084A0` | setters (set-int / set-value): `find`, then insert if absent; the client uses them to build its own messages |
| `0x51E6C0` | UI text-table lookup `(table, text id)`. Text ids overlap message ids (e.g. `0x2774`) |
| `0x410D60` | `item[edi]` of a message (NULL when out of range) |
| `0x469C00` | login dispatcher (`LoginWindow.cpp`) |
| `0x4F4A00` | global front dispatcher; its switch starts at `0x4F4A4F` (route ids with `tools/re/dispatch.py`) |
| `0x458410` | InfoWindow reply handler: news (`0x21FC`, from `0x458475`), player search (`0x2CF7`), friend list (`0x251C`) |
| `0x472EDC…0x473A3A` | main-screen startup-step reply handlers (each waits on its request through `obj+0xE8`) |
| `0x46D8A0` | MainGamen constructor: loads `main.swf`, `sisetu_block.swf`, `main_menubar.swf`, queues the startup steps (see the protocol's main-screen section) |
| `0x472230` | MainGamen lock bits `(this = ESI, bits)`: bit 0 facilities (`_root.sisetuEnable`), bit 1 menu (`menubtnEnable`), bit 2 duel button (`duelbtnEnable`)…; `0x3F` locks all, `0` unlocks |
| `0x4725B0` | startup step that unlocks the main screen (reads `event_id`, `npc_type`; event players take another path) |
| `0x476240` / `0x476DE0` | PresentWindow constructor / reply handler (window id `0xFFFFD90C` in the factory `0x4EEB90`) |
| `0x48AA60` / `0x48F080` | shop (`shopmain.cpp`) reply handler / open routine; the open routine reads slot 8 unchecked |
| `0x531B90` | item factory: builds a received item's object by its tag (6 → `YO3Deck` `0x414240`) |
| `0x43FC60` | deck editor (`DeckEditTest`) reply handler; `0x22C8` saves missing decks locally (`0x52EE60`) |
| `0x4BD880` | NPC duel deck check: regulation (`0x52DB30`), collection (`0x4F6340`), decks (`0x51AB00`); fails with text `0x295B` |
| `0x51AA80` / `0x51AC50` / `0x51A9A0` / `0x51A930` | validate one deck / sizes / regulation / per-card limit |
| `0x410DB0` | a card's count from the trunk: `cards` (or `noTrade`) byte at index row − 1, bounds-checked |
| `0x4F6340` | builds the playable collection `(conn, flags)`: copies the trunk (slot 18), subtracts cards lent in rental records (slot 8, flag 4), and with flag `0x10` zeroes cards 5280, 5434 and 6276 (table `0x7878A0`) in `cards`/`noTrade` unchecked |
| `0x4BEB40` / `0x4BFBA0` / `0x4C0C80` / `0x4BE390` | SelectNpcScene: builds the tab lists from the roster (slot 4, three items per NPC) / `getNpcInfo` / `decideNpc (tab, index, level)` / duel start step (agent mode 3) |
| `0x4A0B50` | `DuelMatchingAgent` start (`EBX` = mode: 0 DUEL panel, 1 password, 2, 3 NPC, 4 duel server); steps `0x4A0E90` deck check, `0x4A1090` `0x1784`, `0x4A1790` `0x17AC`, `0x4A3340` `0x1838` |
| `0x50E470` / `0x50ECF0` / `0x50F030` / `0x50F690` / `0x50F320` | `DuelManager` (scene `0xFFFFD8FD`): constructor / Select Deck (`0x1F4F`) / avatars and `0x17D4` / waits for `0x22C4`, connects the duel server, sends `0x200A` / mode 4: connect, then `0x1838` |
| `0x5233A0` / `0x5201A0` | duel stream pump (pops a duel-connection message, switches on `duelRequestName` 0x10–0x15) / reads `duelRequestName` |
| `0x522A50` | pops one engine command `{id, request, param1..param4}` for a scene; handles a few itself (2, 4, 6, `0x1A`, 1000…) |
| `0x49F990` | client duel message header: `request 0x200C`, `duelRequestName` = the argument |
| `0x517C80` / `0x518400` | `JankenScene` constructor / update (pops commands 1500–1505 when idle; 1505 there is the match's end) |
| `0x513370` | `DuelScene` constructor (`duel_setup.swf`) |
| `0x515A30` / `0x515D30` / `0x515E10` | `DuelScene` step: pops commands until command 2, then opens the result / result step (`DuelResult` in slot `0x13`) / leaves the scene |
| `0x5083A0` / `0x5084D0` | `DuelEndEffect` constructor `(result, reason, …)` / update: plays `duel_win/lose/draw.swf`, done when the movie ends |
| `0x510950` / `0x510B70` / `0x510D50` | `DuelResult` constructor / its nine steps (`0x512070` waits for 1500/1504 or front flag `0x20000`, `0x512430` match movie…) / Flash callbacks (`requestData`, `getResult` `0x511200`, `requestStarCount`, `finish`) |
| `0x50FC00` / `0x50FDF0` / `0x510220` | `DuelManager` after a duel: match result set → leave (`0x510220`), else 1504 → Card Swap (`0x50FDF0`), 1500 → next duel (`0x50F690`) |
| `0x46FEB0` | `GoDuel` (menu bar `go_duel`): the matching agent (`0xFFFFD8F7`) idle → DuelMenu (component `0xFFFFD908`); waiting (`0x4A1D10`) → cancel (`0x4A0BD0`) |
| `0x49D070` / `0x49D310` / `0x49D8F0` | `DuelMenu` constructor / its first step (`0x17FC` → `0x17FD` counts) / Flash callbacks (`requestDataPlayers` `0x49DB30`, `requestDataAuto` `0x49DD10`, `decideAuto` `0x49DF50` → agent mode 2, `requestDataDuel` `0x49E210`, `decideDataDuel` `0x49EE90`, `cancelMenu`, `openRegSelect`) |
| `0x4A0B50` / `0x4A0E90` | `DuelMatchingAgent` start (mode in EBX: 0 DUEL panel, 1 a player's duel, 2 auto, 3 NPC, 4 `0x1838`) / deck check: regulation by the record's `regulation_id`, `duel_regulation_id`, else the login one (connection `+0xC8`), loaded from `data/regulation/` (`0x52DB30`), then `0x51AB00` over the local decks |
| `0x4A1380` / `0x4A15D0` / `0x4A1D10` / `0x4A2D30` / `0x4A2FB0` | agent steps: auto match `0x1798` / its cancel `0x17A2` / the wait (`0x17B6` offers answered by `0x17CA`, `0x1978` → next) / the match push `0x1978` (`vsPlayerData` → MainGamen `+0x408`, `duelData` → `+0x88C`) / the opponent dialog ("LV1 Kaiba", match type), then the duel scene |
| `0x4C8CD0` | duel dialog factory by kind (command `0x36`; table `0x4C9178`): 0 OK, 1 Info, 2 Yes/No, 3 Effect, 4 Select, 5–6 Type, 7 Attribute, 8 Position, 9 Coin, 10 Dice, 11 Number, 13 Discard, 14 Ritual; ECX = param3, EDX = param2 |
| `0x4C2090` / `0x4C29D0` / `0x4C6410` | every duel dialog's Flash `decide` → vtable 16 (keeps the Flash result at `+0x38`) → vtable 15, which sends it as `duelRequestName 4 {DLGResult}` (`0x521A70`) |
| `0x4C5120` / `0x4C51A0` / `0x4C5560` | `DuelEffectDialog` (kind 3) constructor / show (`dlg_yesno.swf`, or `dlg_yesno_card.swf` with up to five rows from `+0x9BC` when `+0x2A0` is `0xFF`) / whether to show: level param3 ≥ 4, else the `invoke_conf` duel option |
| `0x4C5750` / `0x4C5880` / `0x4C5B20` | `DuelSelectDialog` (kind 4) constructor / show (`dlg_select.swf`: labels from the `0x3F0` values with bit 15, text = low 14 bits, at most four) / result: the label's value index |
| `0x4C5E50` / `0x4C5FC0` | `DuelSelectStandDialog` (kind 8) constructor (param3: uid, bit 16 face-up defense) / show (`dlg_selstand.swf`, `dialog.faceType`, `dialog.cardId` from the uid's row) |
| `0x4C72D0` / `0x4C73A0` / `0x4C74F0` | `DuelCardListDialog` (command `0x37`) init (kind → picks OK needs) / show (`dlg_cardlist.swf`, message `0x519870`) / close: sends `6`, `7` per pick, `4` |
| `0x4C76B0` / `0x4C78A0` | the card list's Flash callbacks `getListInfo` (count, max, accept) / `getCardInfo(i)` (row from `+0x9BC`, mark from `+0xE6C`, image id, attribute, ATK/DEF) |
| `0x50B700` | DuelFieldManager mouse handler: left click on a card → its menu / card command (`0x4F9510`); right click with other-flag 6 set → `duelRequestName 5` |
| `0x531B90` | item factory by tag: 3 record, 4 request, 5 `YO3PasswordKey`, 6 deck, 7 `YO3DuelData` (children by name: `gameServer`, `regulation`, `rule`, `players`, `result`; `0x5315C0`) |
| `0x4EEB90` | object factory by id: `0xFFFFD8F5` FrontServerConnection, `0xFFFFD8F6` DuelServerConnection, `0xFFFFD8F7` DuelMatchingAgent, `0xFFFFD8F8` AutoMatchingAgent, `0xFFFFD8F9` CommunicationAgent, `0xFFFFD8FD` DuelManager |
| `0x4A523F`, `0x4A551E`, `0x4A5D05` | `CommunicationAgent` switches (195/106/102 cases) |
| `0x413FE0` | address of front dataset slot `EDI` (vector at `+0x16C`); the global dispatcher keeps some replies there: `0x2711` → 4, `0x251C` friend list → 6, `0x252A` black list → 7, `0x2D6C` rental records → 8, `0x21FC` news → 10, `9000` trunk → 18, `0x22C4` duel data → 20 (flag `0x8000`), `0x2009` duel's end → 21 (flag `0x20000`), `0x2774` → 23, `0x24BA` → 24 |
| `0x45E6D0` / `0x45E810` | "is this player a friend / ignored?": read slots 6 / 7 with no NULL check, for every player listed |
| `0x45F390` | player search request builder (Detailed Search criteria → `search_flag` bits) |
| `0x475A20` | fills a list entry (`ppInfo`) from an item; ECX picks the kind: 0 player, 1 NPC, 2 friend, 3 ignored, 4 team member |
| `0x435950` | ChatWindow dispatcher: `0x2455` push, then a jump table for `0x2456 + n` (send / list / enter / exit) |
| `0x434D20` | reads one chat room item (`id`, `name`, `capacity`, `num`, `region`, `status`, `type`) |
| `0x4F3C10`, `0x4F3CF0`, `0x4F4280` | chat requests `0x2135` (list), `0x2136` (enter), `0x2137` (leave) |
| `0x4F17DF`, `0x4F45A5` | heartbeat `0x2724` builders (30 s / 120 s) |

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
- **`DLG_Text_<lang>`**: 597 duel-engine prompt strings, indexed by `DLG_Indx_<lang>`.
- **`data/extra/yo3pbininfo.bin`**: 4,096 eight-byte entries (item catalogue, text ids 26000+).
- **`data/bin#/s_cardnum.inc`**: a header left over from Konami's duel-engine source.
- The card pool ends around Storm of Ragnarok (early 2011): no Xyz.

## Behaviours worth knowing

- Login is automatic after OK on the saved credentials; only player selection needs a click. Up to 4 players per
  account.
- The client treats server times as JST.
- The client keeps its own data under `<My Documents>\Yu-Gi-Oh! ONLINE 3` (`SHGetFolderPath` `0x8005`, `0x52B8E0`):
  `options.y3o`, `users\`, and per player `players\<pname>\` with `decks`, `mail`, `regulations`, `duellogs`,
  `local`, `avatars` and `<pname>.y3p`. Under Wine, My Documents is usually a link to the Linux `~/Documents`, and
  every prefix made by `vclient.sh prefix` shares it.
- Text values are read as C strings: anything after a NUL is lost. Use binary for blobs (`avatar1`, `pbin`,
  `regulations`).
- Unanswered polls time out with a dialog naming the decimal request id, and the client then drops back to the title.
- Credentials are never sent; see "Accounts" in the protocol catalog.
- Several clients can run at once (unnamed mutexes only, so no single-instance guard). Under Wine each needs its own
  prefix when it gets its own X display; otherwise Wine dies with X `BadWindow` errors.
- The player list hides the searcher's own entry and entries with `player_id` 0, and pages by 100.
- The main screen's map is eight hex buttons, `btn0`–`btn7` in `main.swf`. Its `sisetuStart` (called by the client)
  wires them to GameDelegate calls: `btn0` Shop (`GoShop`), `btn1` Official Event, `btn2` Championship, `btn3` User
  event (all three `MenuSelect`, each only if `_root.ofseve_enable` / `wcup_enable` / `ueve_enable`), `btn4` NPC duel
  (`GoCpuDuel`), `btn7` character settings (`GoCharMake`, the tent on the right; Avatar Settings, CharMake.cpp:
  update `0x51C960`, replies `0x42AC70`); `btn5` and `btn6` have no handler. While the screen is
  locked, `sisetu_block.swf` lays an invisible block over them (`sisetuEnable` false).
- In chat, the client prints its own lines only from the server's `0x2455` echo, its own name in cyan and others' in
  white.

## Crash history (and the rule each one taught)

| Symptom | Cause | Rule |
|---|---|---|
| Crash at `0x46C750` after selecting a player | `avatar1` sent as text, so the array came back empty | Blobs are binary (vtype 8) |
| Null read at `0x4727DE` entering the main screen | player record without `pbin` | Send every field the loader reads |
| Crash in the 9000 handler (if no items) | it dereferences `item[0]` | Lists the client indexes need at least one record |
| Null read at `0x4F54C8` | `0x2398` without `regulations` | Check the **global** dispatcher too, not only the screen's own handler |
| (prevented) `0x27D9` | item[0] read unchecked | Route every new reply through `dispatch.py` before shipping it |
| Null read at `0x45E734` once a search listed a player | friend list never pushed, so dataset slot 6 was NULL | A reply's first non-empty list can wake code that needs other state: test with real items live |
| Null read at `0x48F0F6` opening the shop | rental records `0x2D6C` never pushed, so dataset slot 8 was NULL | Before shipping a screen, check which dataset slots it reads (`0x413FE0`, or `[+0x16C] + 4·slot`) |
| CRT abort `0xC0000417` opening character settings (from `0x42923F`) | `avatar1` had 17 entries; the screen reads 17–40 | Checked-index failures show as `_invalid_parameter`: the tracer's `E` line gives the caller |
| Write to `0x42F` at `0x4F6A82` opening the NPC duel | trunk without `cards`/`noTrade`, so the collection builder wrote through NULL | Per-card arrays are sized by CARD_Prop rows |
| Division by zero at `0x4BFDC4`, hovering an NPC | all three levels sent with `need_ex` 0; `getNpcInfo` divides by their sum | Zero is not a safe default for a divisor: read what a field feeds before choosing a value |
| (no crash) every click on the map ignored | `0x2775` answered with `0x2774`, so PresentWindow never closed and the main screen stayed locked | A screen that ignores input may be waiting on a reply: `YO3_TRACE_HITS` on its steps shows where |
