# Development guide

These commands run in a **source checkout**, not an extracted release. For playing, use
[Desktop](launcher.md) or [Scripts](scripts.md). Contributors should first read the
[working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md).

The [dashboard development guide](dashboard.md#local-development) covers the React dashboard, disposable server,
Rust/OpenAPI/Orval generation and contract checks.

- [Development guide](#development-guide)
  - [Build from source](#build-from-source)
  - [Checks](#checks)
  - [Which tool sees what](#which-tool-sees-what)
  - [Duel diagnostics](#duel-diagnostics)
    - [The rules engine](#the-rules-engine)
  - [Build release bundles](#build-release-bundles)
  - [Verify the packaged updater](#verify-the-packaged-updater)
  - [Publish a release](#publish-a-release)

## Build from source

On Linux, install Rust/Cargo **1.98.1 or newer**, a C++17 compiler (`g++`; the rules engine builds EDOPro's core), Git,
curl, and 32-bit MinGW GCC/binutils (`i686-w64-mingw32-gcc`, `i686-w64-mingw32-dlltool`). Linux game testing needs Wine.
The GUI also needs pkg-config and X11/Wayland/xkbcommon development libraries; on Debian/Ubuntu these are
`pkg-config libx11-dev libwayland-dev libxkbcommon-dev`.
X11 startup also needs `libxcursor1 libx11-xcb1 libxi6 libxkbcommon-x11-0` at runtime.
The release runner installs them before testing the packaged Desktop window.

The root `Cargo.toml` owns the shared crate version, edition, license, Rust baseline, dependency versions and lint
settings. Every crate inherits them; crate-specific features such as Iced's renderer and Windows APIs stay in that
crate's manifest. Change shared versions there and review `Cargo.lock` when updating dependencies. Internal crate
dependencies use workspace paths and the workspace crates advance together. Published bundle versions still come from
`CHANGELOG.md` through `YO3_BUILD_VERSION`; reorganizing package metadata does not publish or bump a release.

```bash
git clone https://github.com/usersina/yo3-revival.git
cd yo3-revival
engine/fetch.sh
cargo build --release -p yo3-server -p yo3-launcher -p yo3-desktop -p yo3-engine
tools/mod/build.sh
```

Build metadata resolves Git's actual HEAD/reference paths, including linked worktrees and packed references.
Source archives without `.git` use `YO3_BUILD_COMMIT` or `unknown`; neither case forces repeated unchanged builds.
`yo3-tunnel/tests/build_metadata.rs` checks these layouts.

For an existing checkout, skip cloning. Open `./target/release/yo3-revival` for the Desktop app. For CLI setup, follow
[Scripts setup](scripts.md#play-on-your-own-pc), replacing `./yo3-server` with `./target/release/yo3-server` and staying
in the repository root. The play equivalent is:

```bash
./target/release/yo3-launcher --profile /absolute/path/client.json \
  --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3" \
  --injector tools/mod/build/yo3-auth-launch.exe --dll tools/mod/build/yo3-auth.dll
```

Source builds use the checkout's `engine/` data and the `yo3-engine` program beside the server in `target/release/`.
The release bundles carry both. The rules engine runs natively; dashboard development and builds use Node.js.
Game content comes from the installed client; never commit game files, extracted data or reverse-engineering exports.
The [repo map and exact scratch commands](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md#commands)
cover server setup beside an existing live session.

## Checks

```bash
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 tools/mod/test_fixture_layout.py
python3 tools/mod/test_winmm_exports.py
python3 tools/client/test_vclient.py
python3 tools/monkey/test_duel_report.py
python3 tools/monkey/test_duel_replay.py
python3 tools/test_package_editions.py
python3 tools/test_release_notes.py
python3 tools/test_host_scripts.py
python3 tools/test_docs_check.py
tools/docs-check.py
```

Fetch engine dependencies before engine tests; game-data tests can skip without an original installation.
DLL/loader changes also need `tools/mod/test.sh` and `tools/mod/test-loader.sh`. See the working guide for the
additional direct-launch fixture and prerequisites.

Keep the user's screen, default ports, live data and `:77` client untouched. Use private Xvfb displays, separate
Wine prefixes/documents, scratch server data and free alternate ports through `tools/client/vclient.sh`.
Never run `cargo fmt` across this repository; match its existing style.

## Check the launcher

The beta launcher ([#20](https://github.com/usersina/yo3-revival/issues/20), on the `beta` branch) is built from the
skin kit ([style guide](launcher-style.md)). Two example programs show the kit without the game:

```bash
cargo run --release -p yo3-skin --example gallery       # every component and state
cargo run --release -p yo3-skin --example main_screen   # the #21 look spike
```

The launcher has no setting for its servers' ports, so a live check beside a real setup starts from a scratch app-data
folder whose settings already name free ports (Singleplayer's `listen_port`, Host for friends' `host_port`) and a
private Wine prefix. Run it on a private display only:

```bash
YO3_INSTANCE=d tools/client/vclient.sh prefix          # .wine-d: a private prefix, not shared with a running client
DATA=$(mktemp -d /tmp/yo3-gui.XXXXXXXX)
python3 - "$DATA" "$PWD/.wine-d" <<'PY'
import json, os, sqlite3, sys
db = sqlite3.connect(os.path.join(sys.argv[1], "launcher.db"))
db.executescript("CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);"
                 "CREATE TABLE servers (id INTEGER PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1;")
db.execute("INSERT INTO settings VALUES (1, ?)", (json.dumps({"wine_prefix": sys.argv[2], "listen_port": 17443, "host_port": 17543,
           "bind_address": "127.0.0.1", "host_address": "localhost", "game_dir": ""}),))
db.commit()
PY
env -u WAYLAND_DISPLAY DISPLAY=:88 YO3_DESKTOP_DATA=$DATA ./target/release/yo3-revival
```

An empty `game_dir` opens Setup first. A development launcher needs a connection DLL built with the same version
(`tools/mod/build.sh`); a release DLL left in `tools/mod/build/` shows Repair, and Setup's Details says why.

Two more things keep a live check private:

- **The credential store.** On Linux the launcher reaches the Secret Service through the session bus, which it finds
  under `XDG_RUNTIME_DIR` even without `DBUS_SESSION_BUS_ADDRESS`. Start it with both pointing away from your desktop
  (`env -u DBUS_SESSION_BUS_ADDRESS XDG_RUNTIME_DIR=$DATA/run`, a private folder) so Multiplayer's Stay signed in can't
  write to your own store; it then shows as unavailable, which is a state worth seeing too.
- **The prefix's user folders.** A copied prefix keeps Wine's links from `drive_c/users/<you>/Documents` (and Desktop,
  Downloads…) to your real home folders, so the game would write there. Replace those links with empty folders in the
  copy, and point the launcher's game folder at the copy's own game.

**Updates (#32) without publishing a release.** A development build (debug assertions on) reads a fake newer release
from `YO3_UPDATE_FAKE_METADATA` (GitHub's release JSON: `tag_name`, `body`, and one asset with its `sha256:` digest) and
stages `YO3_UPDATE_FAKE_ARCHIVE` in place of the download, without the signature check (`tools/test-packaged-update.sh`
checks that); release builds ignore both. Build it with a stable version
(`YO3_BUILD_VERSION=v0.0.21`, in its own `CARGO_TARGET_DIR` so the main build isn't rebuilt), or the check says it is a
development build. The archive needs a `bundle.json` whose components' checksums match, like the fixtures in
`crates/yo3-desktop/tests/update_activation.rs`; a `yo3-revival` script that writes the readiness acknowledgement plays
the new version. The ribbon, What's new and Restart to install then work as with a real release, and a script that
exits shows the failed-start notice. The debug window redraws slowly on Xvfb: wait a few seconds after each click.

On Xvfb there is no window manager: give the launcher keyboard focus (`xdotool windowfocus`) before typing, and raise it
(`xdotool windowraise`) before clicking it while the game is open. Copy in the launcher reaches that display's
clipboard (`DISPLAY=:88 xsel -o -b` reads it).

Host for friends listens on every interface of the PC on its port: use a free scratch port. A friend's check is a
second launcher with its own scratch data, prefix and display (`:89`), pasting the host's invite link under Other
servers. Avoid clicking what opens the browser or a folder (Report a problem, Open logs folder) on a private display:
the system's opener may reach your own desktop.

### The official server list

The Multiplayer tab reads a server list signed with an ed25519 key whose public half is built into the launcher
(`multiplayer::OFFICIAL_LIST_KEYS`, empty until the project publishes a list). `yo3-server-list` makes the key, signs a
list and checks a signed file:

```bash
cargo build -p yo3-desktop --bin yo3-server-list
target/debug/yo3-server-list keygen /safe/place/list.key          # prints the public key; keep the file private
target/debug/yo3-server-list sign servers-list.json /safe/place/list.key servers.json
target/debug/yo3-server-list verify servers.json PUBLIC_KEY_HEX
```

The list is `{"version": N, "servers": [{"name", "address", "identities": [...], "region", "description",
"invite_only"}]}`: each server's identities are the SHA-256 of its public key (the one `yo3-server` shows), two while it
rotates its key. A higher `version` replaces the cached list. To rotate the list's own key, ship a launcher that knows
both public keys before signing with the new one.

## Storage cost

The server keeps one row per account (with its players), record, friendship, mail, team, event and rental in
`server.db` (#22). The accounts are copy-on-write: a change copies only the account it touches, and a save serializes and
writes only accounts whose copy changed. Measured on 2026-11-15 (release build, NVMe), with 300 accounts of 4 players
owning 1,500 different cards each: one purchase, with its rollback copy and its save, takes about 2 ms; before, building
the accounts document alone took 115 ms, under the hub's lock. Re-measure with
`cargo test --release -p yo3-server --test accounts save_cost -- --ignored --nocapture`. The smaller domains still build
their whole document on each change and write only the changed rows.

## Capacity (R7, #27)

Measured on 2026-11-15 (release build, Ryzen 5 3600, 12 threads), by sampling each `yo3-engine` process's memory and
CPU time from `/proc` while tests played whole duels:

| What | Memory | CPU |
| --- | --- | --- |
| A duel between players (the duel check's self-play) | about 47 MB per engine | 0.16–0.3 s for the whole duel |
| An NPC duel with the bot's look-ahead (`npc_bot`, both sides thinking) | 81–99 MB per engine | 3–45 s per duel; with one NPC about half, spread over the duel |
| The server, idle / under 100 flooding connections | 17 MB / up to 275 MB | |
| A save (#22) | | about 2 ms |

So `max-duels` defaults to 30 (about 3 GB of engines at most) and `max-players` to 500: a 4 GB, 2–4 vCPU server
covers the closed beta's expected peak (about 50 players, 15 duels) with room. Each NPC duel's bot can take a core
while it thinks.

The soak at twice that peak passes: `tools/monkey/protocol-monkey.sh soak --secs 300 --players 100 --conns 40
--duel-pairs 30 2>proxies.log` (2026-11-16, the same machine, the monkey on it too). `--players` gives each simulated
player its own password account, sign-in and launcher proxy, as each real player's launcher has: 60 of them in 30 duel
pairs (Auto Duels between two accounts, and NPC duels), 40 sending fuzzing plans from their own sessions. In 300 s:
1,273 duels matched, 683 began and 518 ended with a result; no failure; the canary never stalled (median 310 ms, p99
944 ms, worst 1.86 s against the 2 s limit); the server peaked at 317 MB and 639 file descriptors and restarted cleanly
on the run's data. The in-process proxies report each dropped connection on stderr, hence `2>proxies.log`. The same
soak through the one key account's proxy (no `--players`) stalls at 100 flooding connections: that proxy, not the
server, is the bottleneck. The same soak on a VPS waits with the VPS (#47).

## Which tool sees what

Each debugging tool sees one layer. Start from the question and pick the tool that sees that layer. Commands are in
the [working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md#commands).

| Tool | What it sees | Use it for | Blind spots |
| --- | --- | --- | --- |
| Packet log (`--log-dir`: `packets-*.jsonl`, `[YO3 PACKET]` lines) | Every request and reply on the wire, decoded | Unanswered requests, reply contents, duel commands the server sent | What the client did with a reply |
| `vclient.sh check` | Client alive, its own `File.cpp Lnnn` trace lines, Wine crashes, the last packet | A quick health check after each step | The trace arrives in buffered chunks; nothing about the screen |
| Field tracer (`vclient.sh start --trace`, `tools/tracer/summarize.py`) | Every field the client looked up in each reply, found or missing; `E` lines for exceptions | New reply shapes, missing fields, crash addresses | Attribution is a 300 ms window; local data read then is listed too |
| `YO3_TRACE_HITS` (with `--trace`) | Whether given client addresses ran | Where a screen's startup steps stopped | Only the addresses you name |
| UI probe (`vclient.sh ui`, [details](client-internals.md#the-ui-probe)) | Open screens, windows and dialogs by class; the last UI texts looked up, by id; the game's calls into its Flash screens (NPC talk, dialogs opening, janken, list data); each duel slot's card, face and position | Which screen or dialog is open, which menu, prompt or line just appeared, what the client holds on the field | Words written into a movie itself (the "Start Duel?" box's wording); which talk line is showing; rendering and animation |
| `vclient.sh shot`, `pixel` | The rendered frame, or one pixel's colour | Final visual checks, Flash-only prompts, animations | Needs reading an image; a pixel is only a colour |
| Duel check (`--duel-check`), native checkpoints, `duel-report.py` | The server's model of each client, the engine, and the client's own card manager, compared after every step; the summon commands' order (`duelcheck/summon.rs`) | State drift between engine, server and client; summon commands in an order the client's animations crash on | Rendering, menus, animations themselves |
| Duel replay (`duel-replay.py`) | The engine run again on the recorded inputs | Reproducing an engine-side problem exactly | Mouse input and the client |
| Self-play tests (`duel_check.rs`, card sweep, real-deck sweep, `npc_bot.rs`) | Thousands of duel steps through translator and engine, offline | Regressions after translator, engine or bot changes | The real client |
| Protocol monkey (`protocol-monkey.sh`) | The server under replayed, mutated, random and garbage traffic and floods | Server crashes, hangs and leaks | The client's reaction |
| UI monkey (`ui-monkey.py`) | Random input on two clients; crashes, "no response" dialogs (OCR), server errors | Finding crashes nobody thought to click | Why it happened; reproducing it needs the saved packets and shots |
| Reverse-engineering helpers (`tools/re/`, GhidraMCP) | The client binary: handlers, fields read, disassembly with log strings | Locating what a screen sends and reads | Runtime state |
| Session logs and diagnostic ZIPs (Desktop) | A player's server, game and engine output | Bug reports from players | Only what was logged at the time |

## Duel diagnostics

Game output, native mod diagnostics, decoded packet traces and engine stderr are captured automatically in session
logs. The viewer hides verbose payloads until searched; exports include them by default. Privacy filtering at export
omits payload traces and player activity and masks common personal paths/addresses without changing local evidence.

**Engine messages the client doesn't see.** `ENGINE_MESSAGES` in `crates/yo3-server/src/translate/mod.rs` lists every
message type the rules engine sends, as shown, asked (a prompt), ignored with a reason, or undecided (#52). The test
`every_engine_message_type_is_accounted_for` fails when the engine decodes a type the list doesn't have, and a duel logs
`engine message N (NAME) is not shown to the player` the first time it drops an undecided or unknown one. The duel check
can't catch these: a message that shows something without changing the cards (a coin toss, a die) leaves both views
equal. When a feature handles a new type, change its entry from `Undecided` to `Shown`.

**Commands the client understands.** `CLIENT_COMMANDS` in `crates/yo3-server/src/client_commands.rs` lists every duel
command the client's components handle or Konami's server sent: its handlers, how often Konami sent it in the 228
duel logs, and whether we send it, don't (with the evidence), it has no handler, or it is undecided. The card info
panel's level and stats (1018-1020) went unsent because nothing put "Konami sent it 1,322 times" next to "we never
do". `tests/client_commands.rs` decodes the dispatch tables from `yo3.exe` and fails when a handler differs from the
table, and recounts Konami's logs and fails when a count or an id differs (each skipped without the game or the logs).
The duel check takes its known commands from the table (`duelcheck::coverage::native`): sending a command the table
doesn't mark sent fails the duel. To send one, mark it `Sent` with its coverage; to settle an undecided one, read its
handler and Konami's uses, then mark it.

**Names, not numbers.** Every duel id the server uses is defined once, with its client evidence: the engine's in
`crates/yo3-ocg` (message and response types, actions, hints, positions, locations, reasons; shared with the engine
program, whose bot and answer encoder use it too) and the
client's in `wire.rs` (`cmd` commands, `request` duelRequestNames, `zone`, the card `menu`'s bits, other `flag`s, input
`mode`s, `dialog` kinds, message `entry` kinds, `list` kinds, prompt `text`s, `result`s). Tests tie `wire::cmd` to
`CLIENT_COMMANDS` and `ocg::msg` to `ENGINE_MESSAGES`, so a new id needs its table entry. The commands both the server
and the duel check's client model pack (the battle's, moves, card table entries, dialogs) go through `wire::Cmd`, one
encoder and one decoder (`tests/wire.rs` round-trips them); the engine's answers are built as `ocg::Response` (the
engine's `every_answer_the_server_builds_is_the_bytes_the_core_reads` runs each through its encoder); the client's duel
requests are read once as `duel::ClientRequest`; and what the client is answering besides a prompt's own window is one
`Asking` state in the translator. Add a new id or packing there, not as a local constant.

**Typed engine messages.** The server reads the engine's messages as structs, never `m["field"]`:
`yo3_ocg::message` has one per prompt (`Ask`) and per event it reads (`Event`), field for field as `yo3-engine` sends
them. The server reads leniently (a field a message lacks takes its default, as it always did); the duel check reads
strictly and fails a duel whose messages don't fit, so the 240 self-played duels check every struct against what the
engine really sends. A new field the server needs goes in its struct first; a hand-written test message must carry the
fields the engine sends. Only the engine's state report (the duel check's own `state` query) is still JSON.

**Questions the client answers by itself.** Below level 2, the client's Effect dialog answers No on its own under the
default Activation Message (Semi Auto Cancel), so the player never sees that chance. `CHAIN_WINDOWS` in `translate/mod.rs`
classifies every window the core marks with `MSG_HINT` `HINT_EVENT`. A **Phase** window (Draw, Standby, Battle start
and end, End) puts its optional triggers inside the chain question, so it must show. A **Step** window asks its triggers
one by one first, so its chain question may follow the player's option. Two checks keep this honest:
- `the_cores_chain_windows_are_all_classified` reads the vendored `processor.cpp` and fails when the core sends a
  window the table lacks, or when the Phase rows stop matching its PhaseEvent.
- The duel check plays every chance as the default option would: a chance that matters (`spe_count` above 0) left to
  the client's own No in a Phase window, or a window the table doesn't know, fails the duel. So does the server
  answering an optional effect's yes/no (`MSG_SELECT_EFFECTYN`) for the player. `Played::hidden_by_default` counts the
  chances the default hides, by window.

#53 (Stardust Dragon's End Phase return declined for the player) is the case this catches: with that fix undone, 14
of the 240 duels fail.

For state validation and exact engine replay, run the server with `--duel-check` (the Desktop launcher has no server
flags on the `beta` branch: use a server from the command line or the Scripts edition). Each player's play command
already captures client output. This extra engine work is optional and separate from automatic logging.

The server compares its model of both clients with the rules engine after each step. The required DLL independently
reports native slots, card visibility, hands/piles, LP, stats, counters, turns and phases. Exact engine inputs, shuffled
deck order and dependency fingerprints are recorded for replay. This catches model/native drift; it does not prove
that every rendered animation or legal menu interaction is correct.

The diagnostic dependency scan reads thousands of files and may be slow on cold Windows storage, especially during
extraction or security scanning. It has a separate two-minute deadline; normal duel-engine calls remain limited to
30 seconds. A recording failure still fails the report, and is not evidence that the engine and client model disagreed.
Heavy I/O can still exhaust that budget: failed recordings must remain failed. Keep live validation separate from
archive extraction and collect the complete session; do not discard an error to make a report pass.

The engine integration tests accept `YO3_ENGINE_DIR` to check an extracted bundle, including a copied Windows test
binary on a native VM. Without an override they use the source checkout's engine. Set the path explicitly and verify
that output does not say dependencies were skipped before treating a native test as evidence.

Collect the server and both players' matching session logs, including **every rotated part**. GUI users can export
[diagnostic ZIPs](launcher.md#logs-and-support). Full reports can reveal deck contents, player names and chat;
review them before sharing. Privacy-filtered exports omit replay inputs and cannot prove an exact replay. Disable
`--duel-check` when finished; ordinary log capture continues.

```bash
python3 tools/monkey/duel-report.py --server SERVER.log --client GAME_A.log GAME_B.log
python3 tools/monkey/duel-replay.py --server SERVER.log
python3 tools/monkey/duel-replay.py --server SERVER.log --duel 1
```

The report fails on missing/failed checkpoints and reports coverage gaps. Replay requires the same engine program, card
scripts, database and card-ID mapping; it compares exact recorded answers and state queries (card and player hints in
any order: see [the rules engine](#the-rules-engine)). The duel bot's `think` calls are skipped: the bot stops trying
options after a time budget, so its answer depends on the machine, and the answer it gave is replayed by `respond`.
Replay exercises the engine, not mouse input or pixels. Use focused live checks for remaining UI behavior.

After translator or engine changes, run the full duel check: its 240 self-played duels compare modeled client and
engine state after every step. A failure reports the seed, step and preceding engine messages. It also checks the
summon commands against rules learnt from client crashes (`duelcheck/summon.rs`: each material's effect right before
its own move, a Synchro's Tuner first, an animation's material list); `tests/summon_order.rs` replays Konami's own duel
logs through the same rules, which must find nothing there (`YO3_ORIGINAL_LOGS`, skipped without the logs). When a
client crash comes from the order or state of duel commands, add its cause there as a rule, check it against the logs,
and keep a seed that reaches it in `the_seeds_reach_the_cases_that_drifted`. The client's dialogs are checked too: a
position dialog must name a card the client's card table has, or it shows nothing to click and the duel stops (a
token's position is asked before it appears; every seventh self-play deck carries cards that make such tokens).

The token check (`tests/tokens.rs`, with the game installed and the engine fetched) finds every token a client card's
script makes and requires a client token card for it (by Konami id, by name, or by `TOKEN_ALIASES` in `lib.rs` when the
names differ) and a position dialog that shows it. A new script or a data update that makes an unknown token fails it.

The card check (`crates/yo3-server/tests/card_data.rs`, with the game installed and the engine fetched) loads each of
the client's 4,771 cards in the engine: every one must be found, load its script without an error and have the
client's level, ATK and DEF. Only the 12 game exclusives EDOPro lacks (`engine::NOT_IN_THE_ENGINE`) may be missing; no
pack or preset deck holds them and the deck check refuses them.

**Engine data is pinned.** `engine/fetch.sh` installs ProjectIgnis' card scripts and card database at the commits it
names (`SCRIPTS_COMMIT`, `DATABASE_COMMIT`) and copies `engine/card-ids.json`, YGOPRODeck's Konami ids for each
passcode and its artworks, kept in git. The tests and the release use exactly that data, and
`tests/engine_cards.rs` checks that the installed copy matches the pins. Its other test, which needs no game files,
loads every Konami id of the client's era in the engine; CI runs it on the release bundle's own engine too. To move to
newer data, deliberately:

```bash
engine/fetch.sh --update            # newest scripts, database and ids; rewrites the pins and card-ids.json
cargo nextest run --workspace       # includes the card check and the 24-card sweep
YO3_CARD_SWEEP=all cargo nextest run --release -p yo3-server --test card_sweep --no-capture
```

Commit `engine/fetch.sh` and `engine/card-ids.json` only when all of these pass.

**Card script overrides.** `engine/overrides/` holds our own card scripts, AGPL like ProjectIgnis' scripts they start
from: the engine loads `engine/overrides/c<passcode>.lua` instead of ProjectIgnis' script of that name. One is for a
card that should play by the client's 2010–2012 text rather than today's (Goyo Guardian takes any Tuner, Sangan has
no once-per-turn limit), or for an upstream bug (Double Spell). The folder's
[README](https://github.com/usersina/yo3-revival/blob/main/engine/overrides/README.md) lists each one and why. To add
one:

1. Read the client's text: `yo3_data::Card::desc` (CARD_Desc_E), against the database's (`texts.desc` in
   `engine/vendor/BabelCDB/cards.cdb`).
2. Write a failing engine test in `crates/yo3-engine/src/session/errata.rs`: `Play` sets cards out, answers every
   prompt but the idle and battle ones (using only the optional effects of `use_cards`, choosing other cards before
   those in `keep`), and lets the test summon, activate, end turns and read hands, zones and LP. Check it fails with
   the override moved aside.
   ProjectIgnis' `pre-errata/` scripts and the `(Pre-Errata)` entries of `vendor/BabelCDB/cards-unofficial.cdb`
   show many old rulings, under other passcodes: compare them with the client's text, and copy only what it says.
3. Copy ProjectIgnis' script from `engine/vendor/CardScripts/official/`, change it with a `yo3-revival override:`
   comment saying what and why, and add a row to the README.
4. Rebuild the engine and sweep the card: `YO3_CARD_SWEEP=<cid>, YO3_CARD_SWEEP_SEEDS=10 cargo nextest run --release -p
   yo3-server --test card_sweep --no-capture` (the report counts the duels it was used in).

Both release editions ship the folder as `engine/overrides/` (`tools/engine-files.sh`, checked by
`tools/test_package_editions.py`). The engine's `identity` fingerprints the overrides with the scripts, so a recorded
duel replays only with the same overrides. The old sidecar (`engine/duelcore.ts`) reads them too, so the parity check
compares like with like. After `engine/fetch.sh --update`, compare each overridden card's new upstream script with the one it was copied
from: carry an upstream change over, or delete an override upstream has fixed.

The card sweep (`tests/card_sweep.rs`) plays each card in a self-played duel through the duel check: three copies in
both decks, two in the opening hand (twice in each Extra Deck for Fusion and Synchro Monsters). Every duel check also
runs the round trip: each answer the engine offers must be reachable through the client's windows, and choices the
server makes for the player are counted. By default it sweeps 24 cards; before a release or after engine changes run
all of them (about 15 minutes on 12 cores):

```bash
YO3_CARD_SWEEP=all cargo nextest run --release -p yo3-server --test card_sweep --no-capture
YO3_CARD_SWEEP=7986,7687 cargo nextest run --release -p yo3-server --test card_sweep --no-capture   # some cards
```

Its report, one line per card (activations, summons, choices made for the player), is `target/tmp/card-sweep.tsv`.

The real-deck sweep (`tests/original_decks.rs`) self-plays, the same way, the decks players used on Konami's own
server, rebuilt from 2010–2012 duel logs: each side's cards as its log showed them, filled to 40. The logs are private
and stay out of git (`YO3_ORIGINAL_LOGS`, default `re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players`); without them it skips.
By default it plays 8 deck pairs; before a release, all of them with several seeds:

```bash
YO3_ORIGINAL_DECKS=all YO3_ORIGINAL_DECKS_SEEDS=10 cargo nextest run --release -p yo3-server --test original_decks --no-capture
```

The NPCs' duel bot (`crates/yo3-engine/src/bot`, the engine's `think` request) is measured against the old placeholder
CPU on the client's own NPC decks (`tests/npc_bot.rs`, 6 duels in the normal run). After changing it, play more, with
the same deck on both sides too (skill alone), against the bot's first version (`YO3_BOT_RIVAL=action`) or itself
(`bot`), with a style of its own (`YO3_BOT_STYLE='{"sees":true}'`) or the heroes' Heart of the Cards (`YO3_BOT_SKILL=1`;
`YO3_CHECK_SKILLS=1` gives it to the duel check's opponent), and read a duel's look-ahead decisions with
`YO3_BOT_TRACE`:

```bash
YO3_BOT_DUELS=40 cargo nextest run --release -p yo3-server --test npc_bot --no-capture
YO3_BOT_DUELS=40 YO3_BOT_MIRROR=1 cargo nextest run --release -p yo3-server --test npc_bot --no-capture
YO3_BOT_ONLY=21 YO3_BOT_MIRROR=1 YO3_BOT_DUELS=40 YO3_BOT_TRACE=/tmp/bot.trace cargo nextest run --release -p yo3-server --test npc_bot
```

Each duel's winner and the bot's thinking time go to `target/tmp/npc-bot.tsv`; a duel that doesn't end prints its last
prompts. The trace gives each decision's time. A decision has a budget (`bot::BUDGET`, 700 ms, and 300 ms more for a
try already under way): late in a duel with a full field each engine step costs up to 80 ms (every idle prompt checks
the Extra Deck's summons), so copies rebuilt from the start are slow, and the bot keeps one ready copy per thread.

Protocol and UI monkeys are documented in the
[working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md#commands).

### The rules engine

`yo3-engine` (`crates/yo3-engine`, AGPL-3.0-or-later like the core it links) is EDOPro's core and its Lua, compiled from
the sources `engine/fetch.sh` pins (`CORE_COMMIT`, `LUA_COMMIT`), behind the JSON-lines protocol the server speaks
(`crates/yo3-server/src/engine.rs`): one process per duel, Konami ids in and out. The server finds it beside itself,
or where `YO3_ENGINE` points. It answers exactly as the old Node sidecar (`engine/duelcore.ts`, EDOPro's core built for
WebAssembly by ocgcore-wasm 0.1.6, from the same commits) did, message for message, so the translator was unchanged by
the switch.

`tools/engine/parity.py` proves it: started in the engine's place, it sends every request to both engines and fails the
duel on any difference. It needs the old sidecar's packages (`engine/fetch.sh --sidecar`, which needs Node.js):

```bash
engine/fetch.sh --sidecar
cargo build --release -p yo3-engine
YO3_ENGINE=$PWD/tools/engine/parity.py cargo nextest run --release -p yo3-server --test duel_check
YO3_ENGINE=$PWD/tools/engine/parity.py YO3_CARD_SWEEP=all cargo nextest run --release -p yo3-server --test card_sweep
```

Differences go to `parity-failures.log` (`YO3_PARITY_LOG`) with the whole duel's requests. The one difference
allowed: the order within a run of card and player hints (MSG_CARD_HINT, MSG_PLAYER_HINT), which the core emits while
walking effects ordered by their addresses, so it follows the memory allocator (wasm's or the system's), not the game.
On one platform the order repeats run to run.

## Build release bundles

The Windows target also needs 64-bit MinGW (`x86_64-w64-mingw32-gcc`). Install `tar`, `xz`, `sha256sum` and system
`python3`, then run:

```bash
rustup target add x86_64-pc-windows-gnu
tools/release.sh
```

Set `YO3_VERSION` when building a specific unreleased version; otherwise the script uses Git's description.
The rules engine (`yo3-engine`, built for each platform), its card scripts, card database and third-party notices are
included. Game files, account profiles and server keys are not. Desktop 0.0.9 and older look for the old Node
sidecar's files before installing an update, so each bundle keeps a short note at those paths (`engine/duelcore.ts`,
`engine/package.json`, `engine/node_modules/ocgcore-wasm/package.json`, `engine/node/node[.exe]`); nothing reads them.

Five archives are produced under `dist/`:

| Edition | Linux                                             | Windows                                          | Stable inner folder    |
| ------- | ------------------------------------------------- | ------------------------------------------------ | ---------------------- |
| Desktop | `yo3-revival-desktop-VERSION-linux-x86_64.tar.gz` | `yo3-revival-desktop-VERSION-windows-x86_64.zip` | `yo3-revival-desktop/` |
| Scripts | `yo3-revival-scripts-VERSION-linux-x86_64.tar.gz` | `yo3-revival-scripts-VERSION-windows-x86_64.zip` | `yo3-revival-scripts/` |
| Server  | `yo3-revival-server-VERSION-linux-x86_64.tar.gz`  | (none)                                           | `yo3-revival-server/`  |

All contain the shared `docs/` and root `screenshots/` tree. Desktop opens with the short project README; Scripts
opens with its terminal guide; Server, for a Linux server or VPS (#37), opens with the [hosting guide](hosting.md) and
has only the server and the rules engine, plus `server.toml.example` and `systemd/` (the unit, the monitor and the
off-machine backup, from `tools/host/`; `tools/test_host_scripts.py` fires each alert against a fake webhook).
Packaging validates copied links and anchors before making each archive:

```bash
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-desktop
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-scripts
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-server
```

Check the real app and scripts from extracted archives, including paths with spaces and non-default ports/data
folders. Verify Linux/Wine and native Windows separately. Native Windows 11 checks cover Desktop/Scripts hosting
and play, starter setup, a complete NPC duel, log export and updater activation. Windows 10, protected-folder
direct-start installation and runtime reinstallation remain open; see the [current evidence](history.md#status).

## Verify the packaged updater

The release workflow runs the real Desktop archive through the updater **after bundling and before publishing**.
The test verifies archive checksums/contents, stages the update, opens its actual launcher on private Xvfb and
requires the process's window-readiness acknowledgement before activation. A failed check blocks publication.
CI setup, lints, tests, bundling and this archive check run only before publishing a new release version.
When the changelog version is already released, the workflow stops after checking its version and tag.

Run the same check locally on Linux with `Xvfb`, `xvfb-run` and `xauth` installed:

```bash
YO3_VERSION=v0.0.5 tools/release.sh
tools/test-packaged-update.sh v0.0.5 dist/yo3-revival-desktop-v0.0.5-linux-x86_64.tar.gz
```

Use the version being tested. Building with a stable version label does not publish or create a Git tag; `-dev`
archives are rejected by the production updater. The helper owns its Xvfb display and creates/removes fresh
application storage. It does not start the game or a server. The integration tests stay ignored in ordinary
workspace runs because they need the final archive; the release gate selects them explicitly and fails if none matches.
This checks Linux package activation; native Windows verification remains separate. What it checks:

- **The signature** (#32): it makes a throwaway Ed25519 key, writes `SHA256SUMS` over the archive and signs it with the
  Release workflow's own `.github/scripts/sign_release.sh`. That script refuses this key as Desktop's. The launcher's
  check (`signature::vouch`) then accepts the signed manifest and refuses a changed signature, the project's key, a
  missing `SHA256SUMS.sig` and a download the manifest doesn't list.
- **Activation:** the archive is staged, marked pending and installed at the next start once its window is up.
- **From a released version**, with `--from` and that release's Linux Desktop archive. The release gate runs it from
  v0.0.21, the last release before the beta:

  ```bash
  gh release download v0.0.21 --dir /tmp/released -p 'yo3-revival-desktop-v0.0.21-linux-x86_64.tar.gz'
  tools/test-packaged-update.sh v0.1.0 dist/yo3-revival-desktop-v0.1.0-linux-x86_64.tar.gz \
    --from /tmp/released/yo3-revival-desktop-v0.0.21-linux-x86_64.tar.gz
  ```

  First the released launcher opens and makes its `launcher.db`. Then this release opens on that data and is
  promoted. Finally the released binary is started again, as the player's original shortcut is. It must hand over to
  the installed update and exit. That step runs the old version's own code: its `launcher.db` check, and its bundle
  checks (components, the old engine paths) on the new archive.

### Signing releases

The Release workflow signs `SHA256SUMS` with the repository secret `UPDATE_SIGNING_KEY`, an Ed25519 private key in
PEM. The step writes the secret to a private temporary file, removes it afterwards and never prints it. It then runs
`.github/scripts/sign_release.sh assets KEY`, which:

1. checks that the key's public half is the one built into Desktop (`updates/signature.rs` `PUBLIC_KEY`);
2. writes `SHA256SUMS.sig` (64 raw bytes, `openssl pkeyutl -sign -rawin`);
3. verifies the signature with the public key alone.

Both files are attached to the release, and the notes don't list the signature among the downloads. A release
Desktop would refuse is never published. To replace the key, generate a new one
(`openssl genpkey -algorithm ed25519`). Put its public half
(`openssl pkey -pubout -outform DER | tail -c 32 | od -An -tx1`) in `PUBLIC_KEY`, release that version signed with the
**old** key, and only then change the secret: launchers check with the key they were built with.

Desktop refuses a release without `SHA256SUMS.sig`. A launcher is only offered releases newer than itself, and every
release since the key was built in is signed, so an unsigned one is broken or substituted. v0.0.21 and older don't check
signatures, so they still update to a signed release.

### launcher.db and released launchers

Every released launcher (v0.0.21 and older) refuses a `launcher.db` whose `PRAGMA user_version` is above 1. It
opens the database on every start, to hand its original shortcut over to the installed update, and when it promotes
an update it launched. So `user_version` is a compatibility marker that stays `COMPATIBLE` (1). The schema revision
lives in the `launcher_schema` table, and its steps are `MIGRATIONS` in `crates/yo3-desktop/src/lib.rs`, append only.
A step may drop or reshape the beta's own tables. Released launchers recreate the tables they use (`settings`,
`servers`, `update_state`) when missing, so dropping the unused `servers` table is fine. A database at user_version 2
(beta development builds before this) is accepted and set back to 1. Raising `user_version` locks out every older
launcher and its shortcut. The test
`released_launchers_still_open_the_database_so_their_shortcuts_reach_the_update` replays v0.0.21's statements.

## Publish a release

Windows releases remain unsigned with Authenticode. SignPath Foundation declined the application; certificate
purchase/signing is deferred until funded (#48). The [code-signing policy](code-signing.md) is still a proposal.
The separate Ed25519 update signatures remain required.

1. Update the current **Unreleased** changelog entry with player-visible behavior, then give it the next unused
   `## vX.Y.Z (YYYY-MM-DD)` heading and set that same version in the workspace `Cargo.toml` (without the leading v).
   Update `Cargo.lock` too. Never move or reuse a published tag. In
   [features](features.md), replace each "Unreleased" in the Since column with that version and date.
2. Keep documentation/screenshots current, run required checks and live-test the extracted bundles.
3. Commit in logical slices on a feature branch. Follow the working guide's local merge policy and ask before pushing.
4. A push to `main` with an untagged top changelog version runs the release workflow: tests/lints, the Desktop/Scripts/Server bundles,
   the mandatory packaged-updater check (also from v0.0.21), a `SHA256SUMS` file signed as `SHA256SUMS.sig` with the
   `UPDATE_SIGNING_KEY` secret ([signing releases](#signing-releases)), the same checksums in the release notes (with how
   to check them; players read [antivirus warnings](antivirus.md)), and a GitHub release. An already published top
   version skips verification, bundling and publishing after the changelog/Cargo version check.
5. Successful publication builds/pushes both GHCR images, then deploys their exact digests to the configured VPS.
   There is no approval button. Configure the restricted key/secrets once using the
   [deployment guide](deployment.md#enable-automatic-updates-once); startup failure restores previous images/data.

Desktop checks select only the matching Desktop archive; Scripts updates stay manual. Maintainer architecture and
protocol evidence live in [authentication](authentication.md), [protocol](protocol.md), [client internals](client-internals.md)
and the [roadmap](roadmap.md).
