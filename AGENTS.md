# yo3-revival: working guide

A private server for the retired Konami PC game Yu-Gi-Oh! ONLINE 3 (Duel Accelerator), so the user can play with
friends. The server is reverse-engineered from the client: **the client binary is the spec.** This file is the guide
for anyone, human or AI agent, working on the repo. The project overview is the [README](README.md).

**Contents**

- [yo3-revival: working guide](#yo3-revival-working-guide)
  - [Where things are documented](#where-things-are-documented)
  - [Ground rules](#ground-rules)
  - [Repo map](#repo-map)
  - [Commands](#commands)
  - [The loop: answering a new request](#the-loop-answering-a-new-request)
  - [Conventions](#conventions)
  - [Environment gotchas](#environment-gotchas)

## Where things are documented

The [docs index](docs/README.md) says what each document is for. The ones you will use most:
[docs/roadmap.md](docs/roadmap.md) (progress, next steps, the guesses table),
[docs/protocol.md](docs/protocol.md) (every request/reply pair with its client evidence),
[docs/client-internals.md](docs/client-internals.md) (client addresses, data formats, crash history) and
[docs/launcher.md](docs/launcher.md) (graphical play and hosting, the bundle README) and
[docs/guide.md](docs/guide.md) (command-line reference and source builds).

## Ground rules

1. **Never touch the user's screen or mouse.** Run the game only on the private Xvfb display through
   `tools/client/vclient.sh` (display `:77`, and `:78`+ for extra clients). The user asked for this explicitly.
2. **Evidence, not invention.** Every reply shape cites client evidence (an address, a `File.cpp Lnnn` trace line,
   or a logged packet) in `docs/protocol.md` with a status: confirmed / shape / guess / open. Send only fields the
   client reads. Label guesses and add them to the guesses table in `docs/roadmap.md`.
3. **Test first.** New client requirement → failing contract test → implementation → green → live check. Keep
   `cargo nextest run --workspace` green and `cargo clippy --workspace --all-targets` clean before every commit.
4. **Keep four places in step:** `docs/protocol.md`, `crates/yo3-server/src/front.rs`, the contract tests
   (`crates/yo3-server/tests/front_contract.rs`, `chat_contract.rs`, one file per screen family) and
   `docs/roadmap.md`. A change a host or player sees also goes in `docs/guide.md`. Run `tools/docs-check.py` after
   moving or renaming anything in the docs: it checks every relative link and anchor.
5. **Small, logical commits** (conventional style: `feat(server): …`, `fix(…)`, `docs: …`, `refactor(…)`,
   `chore: …`) on a feature branch per slice of work. **`main` is the source of truth:** when a slice is done, tests
   are green and the live check passes, fast-forward `main` and delete the branch (the user's standing preference).
   Ask before pushing anywhere.
6. **Code only in git.** Never commit game files, Konami data, decompiled code or packet logs. `re/`, `logs/` and
   `target/` are ignored. Docs may cite addresses and field names, but not paste decompiled code. The one exception
   is the screenshots in `docs/images/`: they show our launcher and the game running, they are not game data. Keep game
   screenshots to 1024x768 client frames, optimised PNGs under about 600 KB, with test player names and no host paths. The picture guide in
   `docs/images/guide/` (shipped in the release bundles) is made of crops of such frames and of browser pages with red
   boxes drawn on them, about 800 px wide and under about 300 KB each. A drawn game-folder listing must be labelled
   as an illustration. Launcher screenshots show the real current UI with test accounts and no private credentials.
7. **Don't disturb a live setup.** The user may have a server on the default ports (7443, 8080; older setups may use
   5730/1213/1214) and a client queued on display `:77`. Never stop, restart or touch them, or their `data/`. For your own checks run a
   dev server on other ports and scratch dirs (see Commands) and the second client (`YO3_INSTANCE=b`, display `:78`).

## Repo map

| Path | What |
|---|---|
| `crates/yo3-proto` | Wire codec: frame, Blowfish (`BlowfishLE`), key/value messages, the version-string opener |
| `crates/yo3-data` | Client data: `.ypk` archives, card table, UI text table, regulations, preset decks, packs (golden tests skip without the game) |
| `crates/yo3-desktop` | Iced GUI (`yo3-revival`): local/remote connections, independent hosting, live accounts/invitations, activity; separate SQLite `launcher.db` for preferences and credentials |
| `crates/yo3-server` | Gate / front / duel / patch-HTTP servers (`main.rs` also holds the admin subcommands). `front.rs` = the reply contract and the `Hub` of online players, `duel.rs` = the duel connection (engine commands, the janken), `table.rs` = a duel between two players' connections, `shop.rs` = packs on sale and what a pack holds, `social.rs` = friends and ignore lists, `translate.rs` = engine messages to duel commands, `duelcheck.rs` = the duel check (a model of the client's card manager compared with the engine after each step, and the self-play that drives it), `accounts.rs` = persistent accounts and TLS credential lookup, `auth.rs`/`transport.rs` = session tickets and authenticated connections, `admin.rs` = the BP commands on a running server (`/admin`, loopback with a token), `deckcheck.rs` = the server-side deck check, `engine.rs` = the rules engine process, `trade.rs`, `rental.rs`, `mail.rs`, `teams.rs`, `events.rs` = the player features of those names, `duellog.rs` = duel logs, `clock.rs` = turn and total clocks, `store.rs` = what is kept in `data/`, `net.rs` = framing and per-connection outbox, `log.rs` = packet log |
| `crates/yo3-launcher`, `crates/yo3-tunnel` | Authenticated launcher, private loopback proxy, shared TLS wire protocol |
| `tools/mod/` | Required 32-bit connection DLL, injector and native/Wine regression tests |
| `engine/` | Rules engine sidecar: `duelcore.ts` (EDOPro's core via ocgcore-wasm, under Node), `fetch.sh` (packages, card scripts, card database, Konami ids into the ignored `engine/vendor/`) |
| `tools/client/vclient.sh` | Headless client: `xvfb`, `start [--trace]`, `login [SLOT]`, `check`, `shot`, `click`, `pixel`, `key`, `type`, `stop`, `prefix` |
| `tools/tracer/` | Field-read tracer: `yo3trace.dll` + injector (C, mingw-w64), `summarize.py` |
| `tools/re/` | Python reverse-engineering helpers (see Commands) |
| `tools/ghidra/` | Headless export script |
| `crates/yo3-monkey`, `tools/monkey/` | The monkeys: `yo3-monkey` (protocol: replayed, mutated, random and garbage traffic, floods, a canary, Auto Duel and NPC duel clients) and `ui-monkey.py` (random input on the two headless clients) |
| `docs/` | The documentation (index in `docs/README.md`): `guide.md`, `roadmap.md`, `protocol.md`, `client-internals.md`, `research.md`, `images/` |
| `tools/docs-check.py` | Checks every relative link and anchor in the Markdown, prints a table of contents (`--toc FILE`) |
| `tools/release.sh` | Builds the host's release bundles into `dist/` (ignored) |
| `CHANGELOG.md`, `.github/workflows/release.yml` | Releases: each `## vX.Y.Z (date)` section is one; the workflow publishes a new top version on push to `main` (`.github/scripts/release_notes.py` reads the version and writes the notes) |
| `LICENSE` | MIT |
| `.github/ISSUE_TEMPLATE/` | Issue forms for players (bug, setup help, idea) and the new-issue page's links. Labels: `bug`, `setup help`, `idea` (set by the forms); areas `duels`, `cards & decks`, `accounts & joining`, `hosting`; status `needs info`, `confirmed`, `client limitation`, `duplicate` |
| `re/` (ignored) | Ghidra project `re/ghidra`, `re/yo3_decompiled.c`, GhidraMCP jar |
| `logs/` (ignored) | `packets-*.jsonl`, `client-stdout.log` (client trace), `client-wine.log`, `shots/` |

## Commands

Build and test:

```bash
engine/fetch.sh                                   # once: the rules engine's dependencies (engine tests skip without them)
cargo build --release -p yo3-server -p yo3-launcher -p yo3-desktop
tools/mod/build.sh                               # 32-bit connection DLL, injector and native/Wine contract harnesses
tools/mod/test.sh                                # named/ordinal imports, local cookies and fail-closed injection (private :78)
tools/mod/test-loader.sh                         # WINMM forwarding, Unicode config, missing/changed DLL and missing state
# Live direct-start fixture: first build the Windows yo3-launcher; use a fresh private prefix without LoaderConfig.
YO3_WINEPREFIX=/path/to/private/prefix tools/mod/test-direct.sh /path/to/private/scratch-server-profile.json
python3 tools/client/test_vclient.py              # cold display startup, captured pipes and diagnostics (no X server/game needed)
python3 tools/monkey/test_duel_report.py          # missing/duplicate/failed native snapshots and known coverage gaps
python3 tools/monkey/test_duel_replay.py          # exact engine replay and incomplete/corrupt recording rejection
cargo nextest run --workspace                     # all tests (or: cargo test --workspace); about a minute, most of it the duel
#   check's 240 self-played duels (tests/duel_check.rs, YO3_CHECK_DUELS=n per test for more): run it after any change to
#   translate.rs or the sidecar. A failure names the seed, the step and the engine messages before the difference
cargo clippy --workspace --all-targets            # lints
target/release/yo3-revival                        # graphical launcher/server manager after building
tools/docs-check.py                               # every relative link and anchor in root Markdown files and docs/**/*.md
tools/release.sh                                  # bundles in dist/ (Linux .tar.gz, Windows .zip); docs/launcher.md is their README.md
#   and the whole rules engine (its own Node.js, NODE_VERSION in the script, checksum-checked; the scripts and card
#   database from engine/vendor/): hosts install nothing. The server prefers engine/node/node[.exe] to the PATH's node
#   releasing: add a section at the top of CHANGELOG.md (`## v0.0.2 (date)`, players' words, what changed for them)
#   and push to main (ask first, rule 5). The Release workflow lints, tests, builds both bundles and publishes the
#   version as a GitHub release (title "vX.Y.Z (pre-alpha)", not ticked pre-release so /releases/latest finds it).
#   Never move or reuse a released tag: fix forward with the next version.
#   once: rustup target add x86_64-pc-windows-gnu (mingw-w64 links it). A Windows build under Wine needs its own prefix
#   (WINEPREFIX=.wine-c): vclient.sh stop kills every process of the default prefix
```

The server (`yo3-server --help` lists everything). The first commands are initial host setup reference;
for agent checks beside an existing setup, use the complete scratch example below.

```bash
target/release/yo3-server init-auth localhost 127.0.0.1  # once, before starting; add names/IPs friends reach
target/release/yo3-server add-account Yugi              # once per account; data persists
target/release/yo3-server profile Yugi client.json --advertise localhost
# copy data/server.crt beside the private client.json as server.crt
cargo run --release -p yo3-server -- --log-dir logs     # run in the background; authenticated clients only
ss -ltnp | rg ':7443 '                          # the server's pid, to stop it
#   --bind 0.0.0.0 (default 127.0.0.1)  --advertise HOST (the address friends dial, for port forwarding)
#   --regulation 1: the login banlist (an id or a name from the client's 20; default "unlimited", no banlist)
#   --auth-port --auth-cert --auth-key --http-port --data-dir --log-dir --game-dir: listeners, TLS identity and state
#   testing aids: --duel-lp 1000 (reach a duel's end in one attack), --duel-deck 1149,1449 (the player's main deck as
#   CARD_Prop rows repeated to 40: here Marauding Captain and Giant Orc, for a yes/no question and a card choice),
#   --duel-extra 3437 (the extra deck as rows: Goyo Guardian, a Synchro for Krebons 3397 and Giant Orc),
#   --duel-regulation 4 (check every duel's deck against that banlist, whatever the duel names: the starter deck breaks 4),
#   --duel-time 0,0 (the turn and total clocks of duels between players off: live checks are slower than 180 s a turn),
#   --duel-check (compare model/engine state; the required DLL also logs native checkpoints; ERROR duel check on drift)
#   Detailed diagnostics also records exact engine inputs/answers and dependency fingerprints for replay.
#   state: server.db (SQLite) in data/ (ignored; --data-dir), backed up into data/backups/ at each start
#   old JSON files import once transactionally and remain untouched; storage.version prevents stale reimport

# A dev server beside a live one: first check that 17443 and 18080 are free.
SCRATCH=$(mktemp -d /tmp/yo3-dev.XXXXXXXX)
target/release/yo3-server --data-dir "$SCRATCH/data" init-auth localhost 127.0.0.1
target/release/yo3-server --data-dir "$SCRATCH/data" add-account TestPlayer
target/release/yo3-server --data-dir "$SCRATCH/data" --auth-port 17443 \
  profile TestPlayer "$SCRATCH/client.json" --advertise localhost
cp "$SCRATCH/data/server.crt" "$SCRATCH/server.crt"
export YO3_AUTH_PROFILE="$SCRATCH/client.json"
target/release/yo3-server --auth-port 17443 --http-port 18080 \
  --data-dir "$SCRATCH/data" --log-dir "$SCRATCH/logs"
# Keep that server running in its own terminal. Export the same absolute YO3_AUTH_PROFILE in the client terminal.
# Do not edit yo3.ini: the connection DLL supplies the local endpoints.

# admin subcommands (each takes --data-dir; all but the BP ones need the server stopped)
target/release/yo3-server accounts                # list: address, key, players
#   add-account NAME [--also B,C]: make an account; profile PLAYER FILE --advertise HOST exports its private profile
#   key ADDR|PLAYER [--advertise HOST]: print (or make) an existing account profile
#   add-player ADDR|PLAYER NAME, rename OLD NEW, set-country ADDR|PLAYER CODE|auto (`countries` lists the codes)
#   move-account OLD NEW [--replace]: a friend's address changed; --replace discards an account already at NEW
#   give-bp WHO N, add-code CODE BP [--uses N], remove-code CODE, codes: BP and BOOSTERPASS codes; these four work
#   while the server runs (admin.rs: the server's data/admin.json port and token, loopback only), the rest refuse then
#   TLS is required; there is no stock-client/IP fallback. The profile holds the account key and the public
#   certificate's filename. See docs/authentication.md. /join is retired; accounts are created by the host.
```

The headless client (use private `:78` / `:79`; leave an existing `:77` client alone):

```bash
export YO3_INSTANCE=b                             # :78, logs/b/, prefix .wine-b/
export YO3_AUTH_PROFILE=/path/to/private/client.json  # absolute scratch-server profile path from above
# Create the private display and prefix only when they are not already in use.
tools/client/vclient.sh xvfb && tools/client/vclient.sh prefix
YO3_DISPLAY=:88 YO3_DESKTOP_DATA=/tmp/yo3-gui tools/client/vclient.sh desktop # foreground GUI; create its private display first
# YO3_DESKTOP_BIN=/absolute/path/to/bundle/yo3-revival tests an extracted bundle using the same private display/prefix.
# Configure alternate host ports, scratch server data, and a private Wine prefix/Documents in its Settings before Play.
tools/client/vclient.sh start && tools/client/vclient.sh login 0   # cold start, about 30 s
tools/client/vclient.sh start-stock               # after GUI Enable direct start; same private game/display/prefix
tools/client/vclient.sh check
tools/client/vclient.sh shot NAME                  # logs/b/shots/NAME.png, then view it
tools/client/vclient.sh click X Y                  # also: key KEY, type TEXT, pixel X Y
tools/client/vclient.sh stop                       # only your own instance; also clears its crashed client

# In a second terminal, use YO3_INSTANCE=c (:79), a separate prefix, and another account's profile.
# Each account's first player is slot 0; login 1 means the second player within the same account.
#   a duel between two clients: on each, the menu bar's duel button (195,752) -> Auto Duel (503,401) -> Confirm
#   (420,522), then Select Deck OK (393,533). The mod prepares a legal starting deck once after a fresh player's login;
#   Edit Deck is optional. NPC duel: map -> Kaiba Corporation (765,275), click through the talk, Yes (270,277),
#   pick a deck, OK (393,533), janken. In a duel a left click on a hand card does its default action (summon or set); a
#   right click opens the menu (xdotool button 3). Phase buttons: BP (645,240), M2 (710,240), EP (795,240).
#   the shop: map (370,190) -> greeting (540,150) -> Purchase Pack (510,216) -> a pack (348,225) -> Confirm and Purchase
#   (430,485 twice) -> SKIP (636,530) per pack
```

The tracer (what the client reads from each reply):

```bash
tools/client/vclient.sh start --trace                          # with the exports above: logs/b/fields.tsv
YO3_TRACE_HITS=0x473ae0,0x4725b0 tools/client/vclient.sh start --trace   # also log when these addresses run (H lines)
python3 tools/tracer/summarize.py logs/b/fields.tsv --missing                    # per reply: fields the client read but didn't find
python3 tools/tracer/summarize.py logs/b/fields.tsv --reply 0x2398               # everything read for one reply (top/sub level, caller)
tools/tracer/build.sh                                          # rebuild the tracer after editing it
```

Reverse-engineering helpers (`re/` holds the Ghidra project and the decompiled client, both ignored):

```bash
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

Monkey testing (a scratch dir for the server's data and logs, never the repo's `data/`; the server's ports must be free,
so never while a live server runs; exit status 1 on a failure):

```bash
tools/monkey/protocol-monkey.sh fuzz --seed 1 --plans 2000 --par 8      # plans of recorded, mutated, random, garbage requests
tools/monkey/protocol-monkey.sh duel --seed 1 --pairs 20                # Auto Duel pairs and NPC duels, random duel commands
tools/monkey/protocol-monkey.sh soak --secs 300 --conns 24 --duel-pairs 4   # all at once + floods + canary + monitor
tools/monkey/protocol-monkey.sh repro FILE                              # a saved plan ($SCRATCH/failures/*/plan-*.txt)
tools/monkey/protocol-monkey.sh minimize FILE [--sig TEXT]              # cut a failing plan down (FILE.min.txt)
#   SCRATCH=/tmp/yo3-monkey holds server.log, stats.csv (rss, fds, data size over time) and failures/. The corpus is the
#   packet logs in logs/ (--corpus DIR). Add `--server-arg=--duel-lp --server-arg=1000` to end duels quickly. duel and
#   soak start the server with --duel-check (`--no-duel-check` to leave it off)
tools/monkey/ui-monkey.py --secs 1800 --seed 1                          # random clicks/drags/keys on both headless clients
#   uses private displays :78/:79 and isolated prefix/document copies, refuses busy displays, needs a release build.
#   Failures (screenshots, packets, logs): /tmp/yo3-ui-monkey/ui-failures/.
python3 tools/monkey/duel-report.py --server SERVER.log --client GAME_A.log GAME_B.log
#   Include all rotated parts of matching sessions. Every native acknowledgement is required; gaps fail validation.
python3 tools/monkey/duel-replay.py --server SERVER.log                  # list engine recording IDs in one session
python3 tools/monkey/duel-replay.py --server SERVER.log --duel 1         # replay exact recorded inputs/answers
#   Requires identical engine files/Node version; replays the engine, not the UI. Includes exact shuffled deck order.
```

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
8. **Commit** test, code and docs together; tick `docs/roadmap.md`.

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
- Ports: authenticated TLS 7443, internal HTTP/admin 8080 (loopback). The launcher assigns private local game ports.
  Set `YO3_AUTH_PROFILE` before `vclient.sh start`; the INI is unchanged and the connection DLL is required.
- Game install: `~/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3` (override with `YO3_GAME_DIR`).
- A crashed client stays frozen in winedbg. Run `vclient.sh stop` before relaunching.
- Software rendering under Xvfb: boot takes about 20 s, and there's a "Click the screen" logo between the splash and
  the menu (`login` handles both).
- Ghidra 12.1.2 lives at `/opt/ghidra`. GhidraMCP 2.0 runs headless (`--no-analyze`) on `re/ghidra`, registered in
  the local MCP configuration. Only one Ghidra process can hold the project at a time.
- Ghidra's addresses equal the exe's virtual addresses (image base `0x400000`).
- With `--trace`, the client's own stdout trace reaches `logs/client-stdout.log` in buffered chunks, so recent lines
  can lag. `logs/fields.tsv` is written straight away. Wine's `winepath` prints startup noise on stderr; silence it.
- Wine assumes one X display per prefix: two clients in one prefix on two displays die with X `BadWindow`. Each extra
  client gets its own prefix (`vclient.sh prefix`, a btrfs reflink copy of `~/.wine`, so near-instant).
- Each launcher authenticates its account key over TLS. Two clients sharing an IP keep different identities; each
  needs its own private profile. Additional players of one account use `login SLOT`. No legacy client path exists.
- The Ghidra C export drops the client's log strings, so a log line ("chat room list success") can't be grepped
  there. Use `yodis.py` on the handler to see which case prints it.
- A Windows 11 VM (libvirt `win11`, 192.168.122.71, shared folder `/data/shared`) runs the real client natively: it
  needs a launcher profile and a certificate covering 192.168.122.1. The new launcher has not yet had a native
  Windows VM end-to-end check. Networking reaches the host once firewalld's `libvirt` zone allows TLS port 7443 (its catch-all reject blocks
  them; the user runs `sudo firewall-cmd --zone=libvirt --add-port=7443/tcp`). `virsh -c
  qemu:///system screenshot win11 FILE` shows its display without touching it; the user drives it.
- Card drags (Edit Deck, Card Swap) need a hover before the press: `xdotool` on the client's display, moving onto the
  card's left edge, pausing, then `mousedown`, moving in steps and `mouseup`. `vclient.sh click` alone can't drag.
- Chat request fields arrive nested in one item (tag 10), not at the top level. Check a real packet before assuming
  a request's layout.
- The tracer's reply attribution is a 300 ms window after each `request` read, so local data read just after a reply
  (e.g. banlist files) can be listed under that reply. The `top`/`sub` column tells you which dataset was read.
