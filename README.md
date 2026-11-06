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
| Main screen polls, news list, player search, regulations | works; the main screen is stable |
| Rust server (`crates/`) | parity with the original Python prototype, verified on the real client |
| `0x2775` (presents?) | **not answered yet** |
| Lobby, shop, decks, duels | not started |

## Run

```bash
cargo run --release -p yo3-server -- --log-dir logs
```

Options: `--bind`, `--advertise` (the front-server address the gate hands out, e.g. your LAN or VPN IP for
friends), `--gate-port 5730`, `--front-port 1213`, `--http-port 8080`, `--game-dir` (defaults to the Wine install).
The server reads the banlists, preset decks and packs from the game's own files.

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

Every packet in both directions is logged to stdout and, decoded, to `logs/packets-*.jsonl`.

## Tests

```bash
cargo nextest run --workspace     # or: cargo test --workspace
```

- `yo3-proto`: golden vectors, truncation safety and property-based round trips for the codec
- `yo3-data`: every data file in the client's archives round-trips byte for byte; card, banlist and deck lookups
  (skipped when the game isn't installed)
- `yo3-server`: one contract test per client requirement, most of them former crashes, plus a TCP flow test

### Headless client

`tools/vclient.sh` runs the game on a private Xvfb display (`:77`) with its own virtual mouse, so it never touches
your screen:

```bash
tools/vclient.sh xvfb && tools/vclient.sh start && tools/vclient.sh login
tools/vclient.sh shot name     # -> logs/shots/name.png
tools/vclient.sh click X Y     # game-window coordinates
```

### Reverse-engineering helpers

- `python3 tools/dispatch.py 0x4f4a4f 0x2398` routes a reply ID through the client's global front dispatcher and
  lists the fields its case reads.
- Ghidra headless export (output stays in the ignored `re/` folder):

```bash
/opt/ghidra/support/analyzeHeadless re/ghidra yo3 -import "<game dir>/yo3.exe" -overwrite \
  -scriptPath tools/ghidra -postScript ExportDecompiled.java "$PWD/re/yo3_decompiled.c"
```

- GhidraMCP (headless) is registered for this project in Claude Code's local config, pointing at `re/ghidra`.
  Ghidra locks a project while it's open, so stop it before re-running the headless export.

## Layout

- `crates/yo3-proto`: framing, Blowfish and the EasyMultiPlay key/value format
- `crates/yo3-data`: `.ypk` archives, Konami's card table, regulations, preset decks and packs
- `crates/yo3-server`: gate, front and patch-HTTP servers (`front.rs` holds the request/reply contract)
- `tools/`: Python reverse-engineering helpers. `ypk.py` (archives), `y3dec.py`/`emp.py` (data files), `cards.py`
  (card table), `yodis.py`/`rtti.py` (annotated disassembly), `dispatch.py` (reply routing), `vclient.sh`
  (headless client), `ghidra/` (export script)

## Client notes

- The login dispatcher is `0x469C00` (`LoginWindow.cpp`). Reply 9000 copies `item[0]` wholesale into the trunk
  dataset at `UserData+0x8C8`. The singleton `UserData` lives at `0x82EC78` (getter `0x40F320`).
- `UserData+0x28` is the selected `PlayerData`, filled by the loader `0x51C260` from the 0x2260 list record:
  `pname`, `title`, `avatar1`, `pbin`, `speech00`–`22`, `comment00`–`01`, `n_win`, `n_lose`, `n_draw`.
- **`pbin` is required.** The main screen dereferences it at `0x4727DE`; without it the client crashes. We send
  512 zero bytes, a guess from the 4,096-entry `yo3pbininfo.bin`.
