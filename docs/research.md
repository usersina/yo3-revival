# Yu-Gi-Oh! ONLINE 3 (Duel Accelerator) — revival research

Research date: 2026-11-06. Client examined: `yo3.exe` built 2011-07-01, last patch `p11070600`.

> Snapshot from before any server code existed. Its tool paths are out of date (`tools/re/` now), and the plan in
> §5 continues in [ROADMAP.md](../ROADMAP.md). Protocol facts found since then live in [protocol.md](protocol.md).

## Verdict

A private server for you and your friends is realistic. In October 2026, two independent projects revived
Yu-Gi-Oh! Online 2 (Duel Evolution) with full PvP duels. They used ocgcore, EDOPro's open-source rules engine, plus a
translator into Konami's duel command stream. Online 3 can follow the same recipe, and much of that work can be reused.

| Part | Difficulty | Status |
|---|---|---|
| Point the client at your server | Trivial | `yo3.ini` → `GateServerAddress` |
| Patch / info HTTP | Easy | Done in ygo3-research (serves `yo3.idx` and the install folder) |
| Wire crypto and serializer | Done | Blowfish-ECB `"12345678"`, length `^0x77777777`, word swap. Format below. |
| Login → player select | Done | ygo3-research |
| Enter the world (trunk data, `0x1FA4` → `9000`) | **Next blocker** | Reply format unknown. The client reads it into PlayerData (`0x51C260`). |
| Lobby, rooms, matchmaking, shop, decks | Moderate, weeks | Field names are in the exe. Real data samples are in the client files (below). |
| Duel server | Hard, months | Rules engine: reuse ocgcore via YO2's `duelcore`. The work is the translator to Online 3's duel messages. |

Rough effort, for one person working evenings with Ghidra experience:
- Lobby: 2–6 weeks.
- Duels: 2–4 months, if Online 3's duel stream matches the Online 2 command set (likely, see §3). Longer if it doesn't.

These are estimates, not measurements.

## 1. What's inside your install (verified locally)

- **Archives.** `package/*.ypk` are ZIP local headers with signature `YO\x03\x04` and no central directory. Together
  they hold 12,098 files. Extract with `tools/ypk.py`.
- **Card database.** Konami's standard format (`data/bin#/CARD_Prop.bin`, `CARD_Indx_*`, `CARD_Name_*`,
  `CARD_Desc_*`, `CARD_IntID`, `CARD_Genre`), the same family Tag Force, LotD and Master Duel use.
  - **4,824 cards**, keyed by **Konami card IDs**: 4007 = Blue-Eyes, 4041 = Dark Magician, 39xx = tokens.
  - `CARD_Prop` entry layout for Online 3, 8 bytes per card: `a & 0x3FFF` = card ID, `(a>>14)&0x1FF` = ATK/10,
    `(a>>23)&0x1FF` = DEF/10. Checked against known cards. Online 3 uses 14 bits for the ID; Tag Force 1 used 13.
  - Newest cards: Blue Rose Dragon, Z-ONE, Gishki Beast, Laval Lancelord (Storm of Ragnarok / Hidden Arsenal 4 era,
    early–mid 2011). **No Xyz.**
- **Engine leftovers.**
  - `data/bin#/s_cardnum.inc` is a C header from Konami's duel engine (`DEF_ALLCARD_BEGIN_MRK 3900`, …).
  - `DLG_Text_*.bin` holds **597 duel prompt strings** ("Select @21st @3monster@0 as @3Tribute@0.") — every question
    the server-side engine could ask.
- **Encrypted data files decrypt with the network cipher.** This is independently confirmed by xan1242/ygo3_decoder.
  The plaintext is the **EasyMultiPlay key/value format the server sends**. `tools/emp.py` decodes it.
  - `data/recipe/*.y3d`: 163 preset decks (`deck_id`, `deck_type`, `main`/`extra`/`side` as u16 BE
    **CARD_Prop row indexes**, `pbin_no`, `name_id`…).
  - `data/pack/*.y3a`: 82 booster packs (`pack_id`, `pack_type`, `pack` = card list, `pack_image`…).
  - `data/regulation/*.y3r`: 20 banlists (`card0` forbidden … `card3`, as **card IDs**; `regulation_id`, `status`…).
    `1.y3r` decodes to the real forbidden list: Victory Dragon, Chaos Emperor Dragon, BLS…
- **Protocol vocabulary.** About 1,500 lowercase field names in the exe: `duel_id`, `duel_server_id`,
  `duel_regulation_id`, `duel_time_turn`, `selectable`, `answer`, `room_id`, `trade_player_id`, `user_bp`…
  - Duel-specific keys are few, plus `buffer` and `data`. So duel traffic is probably a binary command blob inside an
    EasyMultiPlay message. *Likely, not confirmed.*
- **Client structure (MSVC RTTI is intact, so classes are named).**
  - Network classes: `GateServerConnection`, `FrontServerConnection`, `DuelServerConnection`,
    `CommunicationAgent`, `DuelMatchingAgent`, `AutoMatchingAgent`.
  - Duel UI classes: `DuelManager`, `DuelCardCommand`, `DuelChain`, `DuelSelect{Attr,Type,Stand,Coin,Dice,Number}Dialog`,
    `DuelDiscardDialog`, `DuelRitualDialog`.
  - No rules engine in the client.
  - `CommunicationAgent` contains 195-, 106- and 102-case switches at `0x4a523f`, `0x4a551e` and `0x4a5d05`. These are
    probably the message dispatchers.
- **Duel logs.** The client has `DuelLogManager` and saves to `players\<name>\duellogs\`. In Online 2 the saved log
  is the exact server command stream. So **an old Online 3 duel log is effectively an official duel recording.**

## 2. Wire format (from ygo3-research source, consistent with the file codec)

- **Frame.** `u32 BE (len ^ 0x77777777)`, then the body, zero-padded to 8 bytes.
  The body is `swap32(Blowfish-ECB("12345678", swap32(plain)))`.
- **Message.** `u8 tag | u16 nfields | nfields × (u8 namelen, name, u8 type, value) | u16 nitems | items…`
  - Value types: 5/6 = u32 BE, 7/9 = string (u32 length), 8 = binary (u32 length), 3 = dataset/record,
    4 = top-level message, 1 = array.
  - The message ID is in the `request` field.
- **Flow so far.**
  - Gate on port 5730: any packet → `0x21FE {host, port}`.
  - Front on port 1213: `0x219F`, then:
    - `0x1F47` → `0x219A`
    - `0x1F48` → `0x2201`
    - `0x1F49` → `0x2260` (player list; `avatar1` must be type 8, 34 bytes)
    - `0x1F4C` → `0x2266`
    - `0x1FA4` → **`9000` (0x2328) with the trunk dataset is unknown.** This is where it black-screens.
- **Client addresses** (from ygo3-research comments): login dispatcher `0x469C00`, deserializer `0x5344B0`,
  PlayerData loader `0x51C260`.

## 3. Konami's duel engine vocabulary is stable from 2008 to 2022

- Online 2 (2008), per YOLaucher's translator notes, receives duel updates as `0x6202` messages. Each is a list of
  8-byte engine commands: `u16 (player<<15 | id), u16 P0, P1, P2`, with IDs `0x01–0x6b` all mapped.
  - Their names: `DUEL_START, TURN_CHANGE, PHASE_CHANGE, CHAIN_SET, CHAIN_RUN, BATTLE_ATTACK, LIFE_DAMAGE, CARD_MOVE,
    CARD_BREAK, CARD_LOCKON, RUN_SUMMON, RUN_FUSION, MONST_SHUFFLE…`
- Master Duel (2022), per YgoMaster's `Enums/Duel.cs`, has `DuelViewType` with the **same names**
  (`DuelStart, TurnChange, PhaseChange, ChainSet, ChainRun, BattleAttack, LifeDamage, CardMove, CardBreak, CardLockon,
  RunSummon, RunFusion, MonstShuffle…`), plus `TuningSet/Run` for Synchro and the later Overlay/Link additions.
- Its dialog types (`SelType, SelAttr, SelStand, SelCoin, SelDice, SelNum, Discard, Ritual, YesNo, Effect…`) match
  Online 3's dialog classes one for one.
- **Implication:** Online 3, released in 2010, almost certainly ran the same Konami engine and sent the same kind of
  command stream. The Online 2 command table and translator are therefore the best starting point. *Likely.* Confirm
  it by finding the command executor under `DuelCardCommand`/`DuelManager` in Ghidra and comparing handlers with
  Online 2's table.

## 4. Existing projects to build on

| Project | What it gives you |
|---|---|
| [iteratequickly/ygo3-research](https://github.com/iteratequickly/ygo3-research) (MIT, Nov 2026) | Working Online 3 gate/front stub in Python, HTTP patch server, YPK extractor. Start here. |
| [xan1242/ygo3_decoder](https://github.com/xan1242/ygo3_decoder) (MIT, 2020) | Online 3 file codec |
| [xan1242/YGO3Hack](https://github.com/xan1242/YGO3Hack) (MIT, 2022) | DLL hack: offline boot into any scene; `YGO3_MenuIDs.h` lists scene IDs. Probably the "offline patch" from elitepvpers. |
| [setokaiba666/YOLaucher](https://github.com/setokaiba666/YOLaucher) (Oct 2026, license TODO) | Full Online 2 server. `tools/yo2emu/duelcore/` = ocgcore as a standalone JSON-lines process (game-independent). `research/traductor/NOTES.md` = 170 KB on the engine command set and translation. Lobby, trade and deck notes. |
| [y7ya-com/duel-evolution](https://github.com/y7ya-com/duel-evolution) (MIT code, Oct 2026) | Online 2 recompiled to C/WASM plus a TypeScript server; playable at original.ygoevolution.com |
| [pixeltris/YgoMaster](https://github.com/pixeltris/YgoMaster) (MIT) | Konami engine API, view and dialog enums, headless PvP design |
| [edo9300/ygopro-core](https://github.com/edo9300/ygopro-core) (AGPL-3.0+) | Rules engine with `MR1`/`MR2`/`GOAT` presets; restrict the pool through the card/script readers |
| [ProjectIgnis/CardScripts](https://github.com/ProjectIgnis/CardScripts) (AGPL-3.0+) | Lua card scripts, keyed by passcode |
| YGOProDeck API `misc_info.konami_id` | Konami card ID ↔ passcode mapping |

**Wayback archives of the Online 3 site** (useful for reproducing server behaviour):
- [FAQ](https://web.archive.org/web/20100308030832/http://www.yugioh-online.net/v3/pages/faq/index)
- [Manual](https://web.archive.org/web/20100308025134/http://www.yugioh-online.net/v3/pages/howto/startgame)
- [Final pack list](https://web.archive.org/web/20130119230329/http://www.yugioh-online.net:80/v3/packs/index)
- [Forbidden/Limited list](https://web.archive.org/web/20130121151028/http://www.yugioh-online.net:80/v3/pages/fobi)
- [Tournament rules](https://web.archive.org/web/20110724042922/http://www.yugioh-online.net:80/v3/championships/rule?country=US)

No patch files or traffic captures are archived.

**Game rules a server must reproduce** (from the archived FAQ):
- Decks: main 40–60, extra 0–15, side 0–15.
- Timers: 180 s per turn, 60 s to answer a chain; tournaments use 10 min total.
- Draws are decided by rock-paper-scissors. Disconnecting means you lose.
- Economy: BP, packs cost 15 BP.
- Levels: 30.
- Storage: the trunk (collection) is server-side; decks are stored locally.

## 5. Plan

1. **Setup.**
   - Run ygo3-research's server with `yo3.ini` pointed at `127.0.0.1`.
   - Load `yo3.exe` in Ghidra with RTTI class recovery.
   - Run the client under Wine. x32dbg works on Wine 6.2+. The `winedbg --gdb` / gdbserver route also works.
2. **Get past the black screen.**
   - Reverse the reply to `0x1FA4` (`9000`) from PlayerData (`0x51C260`) and the deserializer (`0x5344B0`).
   - Use the decoded `.y3d`/`.y3a` files as examples of real records.
3. **Lobby to match start.** Rooms, matchmaking, regulation selection, and the hand-off to `DuelServerConnection`.
   Work out how duel commands are carried (probably a binary field).
4. **Duel server.**
   - Confirm the command set against Online 2's table.
   - Port YOLaucher's translator.
   - Run `duelcore` unchanged, with rule `MR2` and the card pool limited to the client's 4,824 card IDs.
   - Build the card ID → passcode map, and map ocgcore hints to the 597 `DLG_Text` prompt IDs.
5. **Content.**
   - Load packs, preset decks and banlists straight from the client files.
   - Add BP economy, timers, friends and trades.
6. **Play with friends.**
   - Each friend sets `GateServerAddress` to your host.
   - Open TCP 5730, 1213 and the duel port, or use a VPN such as Tailscale or ZeroTier so nothing is exposed publicly.

## 6. Leads worth chasing

- **Old Online 3 duel logs** (`players\<name>\duellogs\`) or a whole old install from a former player. These are
  official recordings of duels.
- **The DuelEvolution Discord** (deferred): it reportedly holds Online 3 server traffic captures.
- **`yo3setup_20110728.exe`** on archive.org is 3 weeks newer than your client. Check whether it is the final build
  before you lock in a protocol version.

## 7. Legal hygiene (not legal advice)

- Konami has no known history of acting against server emulators (SaveMGO, bemaniutils, YgoMaster). It does act on
  assets and leaks. YGO art rights sit with NAS.
- Keep it private and non-commercial.
- Publish code only: no game files, card data or decompiled code.
- ocgcore and CardScripts are AGPL. Run the engine as a separate process and/or publish your server source.

## Tools in `tools/`

| Script | Use |
|---|---|
| `ypk.py` | `python3 ypk.py list - '' <archives…>` / `python3 ypk.py extract <outdir> 'data/bin#/*,data/pack/*' <archives…>` |
| `y3dec.py` | Decrypts `.y3d`/`.y3a`/`.y3r` (and is the network cipher) |
| `emp.py` | Decrypts and parses EasyMultiPlay key/value data into JSON |
| `cards.py` | Decodes `CARD_Prop` with English names |
