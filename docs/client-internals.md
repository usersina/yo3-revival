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
| `0x409020` / `0x4090F0` / `0x40F1D0` | typed field getters: int / string / raw value (NULL when missing) |
| `0x410D60` | `item[edi]` of a message (NULL when out of range) |
| `0x469C00` | login dispatcher (`LoginWindow.cpp`) |
| `0x4F4A00` | global front dispatcher; its switch starts at `0x4F4A4F` (route ids with `tools/re/dispatch.py`) |
| `0x458475` | InfoWindow dispatcher (news) |
| `0x472EDC…0x473A3A` | main-screen poll reply handlers (each re-arms its request through `obj+0xE8`) |
| `0x4A523F`, `0x4A551E`, `0x4A5D05` | `CommunicationAgent` switches (195/106/102 cases) |
| `0x4F17DF`, `0x4F45A5` | heartbeat `0x2724` builders (30 s / 120 s) |

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
- Text values are read as C strings: anything after a NUL is lost. Use binary for blobs (`avatar1`, `pbin`,
  `regulations`).
- Unanswered polls time out with a dialog naming the decimal request id, and the client then drops back to the title.

## Crash history (and the rule each one taught)

| Symptom | Cause | Rule |
|---|---|---|
| Crash at `0x46C750` after selecting a player | `avatar1` sent as text, so the array came back empty | Blobs are binary (vtype 8) |
| Null read at `0x4727DE` entering the main screen | player record without `pbin` | Send every field the loader reads |
| Crash in the 9000 handler (if no items) | it dereferences `item[0]` | Lists the client indexes need at least one record |
| Null read at `0x4F54C8` | `0x2398` without `regulations` | Check the **global** dispatcher too, not only the screen's own handler |
| (prevented) `0x27D9` | item[0] read unchecked | Route every new reply through `dispatch.py` before shipping it |
