# yo3-revival: working guide

A private server for the retired Konami PC game Yu-Gi-Oh! ONLINE 3 (Duel Accelerator), so the user can play with
friends. The server is reverse-engineered from the client: **the client binary is the spec.**

Progress and next steps: [ROADMAP.md](ROADMAP.md). Protocol facts: [docs/protocol.md](docs/protocol.md). Client
addresses and data formats: [docs/client-internals.md](docs/client-internals.md).

## Ground rules

1. **Never touch the user's screen or mouse.** Run the game only on the private Xvfb display through
   `tools/client/vclient.sh` (display `:77`, and `:78`+ for extra clients). The user asked for this explicitly.
2. **Evidence, not invention.** Every reply shape cites client evidence (an address, a `File.cpp Lnnn` trace line,
   or a logged packet) in `docs/protocol.md` with a status: confirmed / shape / guess / open. Send only fields the
   client reads. Label guesses and add them to the guesses table in ROADMAP.md.
3. **Test first.** New client requirement → failing contract test → implementation → green → live check. Keep
   `cargo nextest run --workspace` green and `cargo clippy --workspace --all-targets` clean before every commit.
4. **Keep four places in step:** `docs/protocol.md`, `crates/yo3-server/src/front.rs`, the contract tests
   (`crates/yo3-server/tests/front_contract.rs`, `chat_contract.rs`, one file per screen family) and `ROADMAP.md`.
5. **Small, logical commits** (conventional style: `feat(server): …`, `fix(…)`, `docs: …`, `refactor(…)`,
   `chore: …`) on a feature branch per slice of work. **`main` is the source of truth:** when a slice is done, tests
   are green and the live check passes, fast-forward `main` and delete the branch (the user's standing preference).
   Ask before pushing anywhere.
6. **Code only in git.** Never commit game files, Konami data, decompiled code or packet logs. `re/`, `logs/` and
   `target/` are ignored. Docs may cite addresses and field names, but not paste decompiled code.

## Repo map

| Path | What |
|---|---|
| `crates/yo3-proto` | Wire codec: frame, Blowfish (`BlowfishLE`), key/value messages, the version-string opener |
| `crates/yo3-data` | Client data: `.ypk` archives, card table, UI text table, regulations, preset decks, packs (golden tests skip without the game) |
| `crates/yo3-server` | Gate / front / duel / patch-HTTP servers. `front.rs` = the reply contract and the `Hub` of online players, `duel.rs` = the duel connection (engine commands, the janken), `table.rs` = a duel between two players' connections, `shop.rs` = packs on sale and what a pack holds, `social.rs` = friends and ignore lists, `translate.rs` = engine messages to duel commands, `accounts.rs` = accounts by client address, `net.rs` = framing and per-connection outbox, `log.rs` = packet log |
| `engine/` | Rules engine sidecar: `duelcore.ts` (EDOPro's core via ocgcore-wasm, under Node), `fetch.sh` (packages, card scripts, card database, Konami ids into the ignored `engine/vendor/`) |
| `tools/client/vclient.sh` | Headless client: `xvfb`, `start [--trace]`, `login [SLOT]`, `check`, `shot`, `click`, `pixel`, `key`, `type`, `stop`, `prefix` |
| `tools/tracer/` | Field-read tracer: `yo3trace.dll` + injector (C, mingw-w64), `summarize.py` |
| `tools/re/` | Python reverse-engineering helpers (see Commands) |
| `tools/ghidra/` | Headless export script |
| `docs/` | Protocol catalog, client internals, original research |
| `re/` (ignored) | Ghidra project `re/ghidra`, `re/yo3_decompiled.c`, GhidraMCP jar |
| `logs/` (ignored) | `packets-*.jsonl`, `client-stdout.log` (client trace), `client-wine.log`, `shots/` |

## Commands

```bash
engine/fetch.sh                                                 # once: the rules engine's dependencies (engine tests skip without them)
cargo nextest run --workspace                                   # all tests
cargo clippy --workspace --all-targets                          # lints
cargo run --release -p yo3-server -- --log-dir logs --players Yugi,Kaiba   # server (in the background); one account, up to 4 players
#   --regulation 1: the login banlist (an id or a name from the client's 20; default 3, "unlimited", no banlist)
#   testing aids: --duel-lp 1000 (reach a duel's end in one attack), --duel-deck 1149,1449 (the player's main deck as
#   CARD_Prop rows repeated to 40: here Marauding Captain and Giant Orc, for a yes/no question and a card choice),
#   --duel-extra 3437 (the extra deck as rows: Goyo Guardian, a Synchro for Krebons 3397 and Giant Orc)
ss -ltnp | grep ':5730 '                                        # find the server's pid to stop it
#   state: accounts and records persist in data/ (ignored; --data-dir), backed up into data/backups/ at each start
target/release/yo3-server accounts                              # admin (server stopped): list, add-player ADDR NAME, rename OLD NEW
tools/release.sh                                                # the host's bundles in dist/ (Linux, Windows); the friends' guide is docs/friends.md
#   once: rustup target add x86_64-pc-windows-gnu (mingw-w64 links it). A Windows build under Wine needs its own prefix
#   (WINEPREFIX=.wine-c): vclient.sh stop kills every process of the default prefix

tools/client/vclient.sh xvfb                                    # once per boot
tools/client/vclient.sh start && tools/client/vclient.sh login  # cold start to main screen, about 30 s
tools/client/vclient.sh check                                   # process, client trace, crash, last packets
tools/client/vclient.sh shot NAME                               # logs/shots/NAME.png, then view it
tools/client/vclient.sh stop                                    # also clears a crashed client

export YO3_INSTANCE=b                                           # a second client: display :78, logs/b/, prefix .wine-b/
tools/client/vclient.sh xvfb && tools/client/vclient.sh prefix  # once: its display and reflinked Wine prefix
tools/client/vclient.sh start && tools/client/vclient.sh login 1   # log in as the second player (slot 1)
#   a duel between them: on each, the menu bar's duel button (195,752) → Auto Duel (503,401) → Confirm (420,522), then
#   Select Deck OK (393,533). A player needs a local deck first: open Main Menu → Edit Deck once (it saves the starter)
#   the shop: map (370,190) → greeting (540,150) → Purchase Pack (510,216) → a pack (348,225) → Confirm and Purchase
#   (430,485 twice) → SKIP (636,530) per pack

tools/client/vclient.sh start --trace                          # same, logging every field lookup -> logs/fields.tsv
YO3_TRACE_HITS=0x473ae0,0x4725b0 tools/client/vclient.sh start --trace   # also log when these addresses run (H lines)
python3 tools/tracer/summarize.py --missing                     # per reply: fields the client read but didn't find
python3 tools/tracer/summarize.py --reply 0x2398                # everything read for one reply (top/sub level, caller)
tools/tracer/build.sh                                           # rebuild the tracer after editing it

python3 tools/re/dispatch.py 0x4f4a4f 0x2398    # route a reply id through the global dispatcher; fields it reads
python3 tools/re/handlers.py 0x435950           # a screen's handler: per case, the field names and getters
python3 tools/re/yodis.py 0x435950 400          # annotated disassembly (shows the log strings Ghidra's C drops)
python3 tools/re/rtti.py ClassName              # vtable of a class (MSVC RTTI)
python3 tools/re/emp.py FILE.y3d                # decode a client data file
python3 tools/re/ypk.py x re/gfx '*main.swf' "<game dir>"/package/*.ypk     # extract files from the .ypk archives
ffdec -export script re/gfx/as/main re/gfx/data002.ypk/data/gfx/main.swf  # a Flash screen's ActionScript
grep -n '0x2775' re/yo3_decompiled.c            # find a request id in the decompiled client
```

Ghidra export (stop GhidraMCP first, because Ghidra locks the project):
`/opt/ghidra/support/analyzeHeadless re/ghidra yo3 -import "<game dir>/yo3.exe" -overwrite -scriptPath tools/ghidra -postScript ExportDecompiled.java "$PWD/re/yo3_decompiled.c"`

GhidraMCP tools (`mcp__ghidra__*`): `decompile_function_by_address`, `get_xrefs_to`, `disassemble_function`,
`list_strings`, `rename_function_by_address`, `set_decompiler_comment`. Name what we understand, so the project keeps
the knowledge.

## The loop: answering a new request

1. **Observe.** Start the server and run `vclient.sh start --trace && vclient.sh login && vclient.sh check`, then
   `tools/tracer/summarize.py --missing`. Look for:
   - an unanswered request (the last `<-` in `check`, or `UNHANDLED` in the server output)
   - fields the client looked up and didn't find (the tracer); `top` means the reply itself
   - a "There is no response from the server. (N)" dialog (N is the request id in decimal)
   - a Wine "page fault … at address X" in `logs/client-wine.log`, or an `E` line in `logs/fields.tsv` (traced runs):
     the exception and the game-code addresses on the stack, a rough backtrace even when Wine prints none
   - clicks that do nothing: the screen may still be locked (the main screen locks itself until its startup steps
     finish; see the main-screen section of the protocol). `YO3_TRACE_HITS` shows which step it stopped at.
2. **Locate.** Find the request's builder and the reply ids its screen compares (grep the id in
   `re/yo3_decompiled.c`, or use GhidraMCP xrefs). Read which fields the handler reads.
3. **Check the global dispatcher.** Run `tools/re/dispatch.py 0x4f4a4f <reply id>`. Every reply also passes through
   it, and unchecked reads there have crashed us before.
4. **Record** the pair in `docs/protocol.md` with its evidence and status.
5. **Red:** add a contract test in `front_contract.rs` naming the client address it protects.
6. **Green:** implement in `front.rs` with the minimum the client reads.
7. **Live:** restart the server, rerun the traced client, confirm the tracer now shows the fields as `found`, check
   the client's own trace line (e.g. `… recv success`), and take a screenshot.
8. **Commit** test, code and docs together; tick ROADMAP.md.

## Conventions

- Build messages with `Message::reply(0x2398).with("field", Value::…).item(Message::record()…)`. Request and reply
  ids are hex literals.
- Binary blobs are `Value::Bin`. Text (`Value::Text`) is read by the client up to the first NUL.
- Times sent to the client are JST (`jst_now()` in `front.rs`).
- Lists the client indexes (`item[0]`) need at least one record, or the client crashes.
- Game content (cards, banlists, decks, packs) comes from the client's files through `yo3-data`, never typed by
  hand. Decks and packs use CARD_Prop rows; regulations use Konami card ids.
- Unknown meaning: name the arm by its id, comment "meaning unknown", and list it as a guess.
- Python is only for tools (`tools/re/`, `tools/tracer/summarize.py`). Run it with system `python3` (capstone,
  pefile and pycryptodome are installed). The tracer DLL is C, built with mingw-w64 (`i686-w64-mingw32-gcc`).
- Message ids and UI text ids overlap: a number passed to `FUN_0051E6C0(table, id)` is a text string, not a reply.

## Environment gotchas

- The user's shell aliases `ls` → eza and `cat` → bat. Use `command ls` / `command cat` in scripts.
- Don't run `cargo fmt`: there is no `rustfmt.toml` and the code was never formatted with the defaults (lines run to
  about 120 columns), so it rewraps every file. Match the surrounding style by hand.
- `pgrep -f` / `pkill -f` match the calling shell's own command line. Find processes by port (`ss -ltnp`) instead.
- Ports: gate 5730, front 1213, duel 1214 (our choice), HTTP 8080. The game's `yo3.ini` points at 127.0.0.1; Konami's original is saved
  next to it as `yo3.ini.konami`.
- Game install: `~/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3` (override with `YO3_GAME_DIR`).
- A crashed client stays frozen in winedbg. Run `vclient.sh stop` before relaunching.
- Software rendering under Xvfb: boot takes about 20 s, and there's a "Click the screen" logo between the splash and
  the menu (`login` handles both).
- Ghidra 12.1.2 lives at `/opt/ghidra`. GhidraMCP 2.0 runs headless (`--no-analyze`) on `re/ghidra`, registered in
  Claude Code's local config. Only one Ghidra process can hold the project at a time.
- Ghidra's addresses equal the exe's virtual addresses (image base `0x400000`).
- With `--trace`, the client's own stdout trace reaches `logs/client-stdout.log` in buffered chunks, so recent lines
  can lag. `logs/fields.tsv` is written straight away. Wine's `winepath` prints startup noise on stderr; silence it.
- Wine assumes one X display per prefix: two clients in one prefix on two displays die with X `BadWindow`. Each extra
  client gets its own prefix (`vclient.sh prefix`, a btrfs reflink copy of `~/.wine`, so near-instant).
- The server keys accounts by client address, so every local client shares one account; pick different players
  with `login SLOT`. A player already online can still be chosen twice (no guard yet).
- The Ghidra C export drops the client's log strings, so a log line ("chat room list success") can't be grepped
  there. Use `yodis.py` on the handler to see which case prints it.
- A Windows 11 VM (libvirt `win11`, 192.168.122.71, shared folder `/data/shared`) runs the real client natively: it
  reaches the host at 192.168.122.1 once firewalld's `libvirt` zone allows the four ports (its catch-all reject blocks
  them; the user runs `sudo firewall-cmd --zone=libvirt --add-port={5730,1213,1214,8080}/tcp`). `virsh -c
  qemu:///system screenshot win11 FILE` shows its display without touching it; the user drives it.
- Card drags (Edit Deck, Card Swap) need a hover before the press: `xdotool` on the client's display, moving onto the
  card's left edge, pausing, then `mousedown`, moving in steps and `mouseup`. `vclient.sh click` alone can't drag.
- Chat request fields arrive nested in one item (tag 10), not at the top level. Check a real packet before assuming
  a request's layout.
- The tracer's reply attribution is a 300 ms window after each `request` read, so local data read just after a reply
  (e.g. banlist files) can be listed under that reply. The `top`/`sub` column tells you which dataset was read.
