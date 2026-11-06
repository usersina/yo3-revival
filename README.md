# yo3-revival

A private server for Yu-Gi-Oh! ONLINE 3 (Duel Accelerator), for playing with friends. Background research is in
[YO3-revival-research.md](YO3-revival-research.md).

## Status

| Step | State |
|---|---|
| Patch check (HTTP `yo3.idx`) | works |
| Gate → front redirect, version check (`"2010.03.23 skelton"`) | works |
| Auth / login / player list / player select | works |
| World entry `0x1FA4` → `9000` (trunk data) | works; the client enters the main screen |
| Main screen: `0x2CF0`, `0x1F41`, `0x2072` | **not answered yet** |
| Lobby, shop, decks, duels | not started |

## Run

```bash
pip install pycryptodome
cd server && python3 -m yo3.server --log-dir ../logs
```

In the game folder's `yo3.ini` (Konami's original is kept as `yo3.ini.konami`):

```ini
GateServerAddress=127.0.0.1
GateServerPort=5730
PatchUpdateUrl=http://127.0.0.1:8080/yo3/
PatchDataUrl=http://127.0.0.1:8080/ygoda/patch/
InfoURL=http://127.0.0.1:8080/app/
```

Run the client from a terminal to get its own debug trace (`LoginWindow.cpp L370 : …`) on stdout:

```bash
cd ~/.wine/drive_c/Program\ Files\ \(x86\)/Konami/Yu-Gi-Oh!\ ONLINE\ 3 && wine yo3.exe > client-stdout.log
```

Every packet in both directions is logged decoded to `logs/server-*.log` and `logs/packets-*.jsonl`.

Tests: `python3 server/tests/test_flow.py` round-trips the client's own data files through the codec and plays the
login flow against a live server.

## Layout

- `server/yo3/codec.py`: framing, Blowfish, and the EasyMultiPlay key/value format
- `server/yo3/front.py`: front-server request handlers and the account/player model
- `server/yo3/server.py`: asyncio gate/front/HTTP servers and logging
- `tools/`: `ypk.py` (archives), `y3dec.py`/`emp.py` (data files), `cards.py` (card table), `yodis.py`/`rtti.py`
  (disassembly with string and RTTI annotations for `yo3.exe`)

## Client notes

- The login dispatcher is `0x469C00` (`LoginWindow.cpp`). Reply 9000 copies `item[0]` wholesale into the trunk
  dataset at `UserData+0x8C8`. The singleton `UserData` lives at `0x82EC78` (getter `0x40F320`).
- `UserData+0x28` is the selected `PlayerData`, filled by the loader `0x51C260` from the 0x2260 list record:
  `pname`, `title`, `avatar1`, `pbin`, `speech00`–`22`, `comment00`–`01`, `n_win`, `n_lose`, `n_draw`.
- **`pbin` is required.** The main screen dereferences it at `0x4727DE`; without it the client crashes. We send
  512 zero bytes, a guess from the 4,096-entry `yo3pbininfo.bin`.
