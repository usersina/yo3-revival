# Development guide

These commands run in a **source checkout**, not an extracted release. For playing, use
[Desktop](launcher.md) or [Scripts](scripts.md). Contributors should first read the
[working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md).

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
dependencies use workspace paths and all eight crates advance together. Published bundle versions still come from
`CHANGELOG.md` through `YO3_BUILD_VERSION`; reorganizing package metadata does not publish or bump a release.

```bash
git clone https://github.com/usersina/yo3-revival.git
cd yo3-revival
engine/fetch.sh
cargo build --release -p yo3-server -p yo3-launcher -p yo3-desktop -p yo3-engine
tools/mod/build.sh
```

For an existing checkout, skip cloning. Open `./target/release/yo3-revival` for the Desktop app. For CLI setup, follow
[Scripts setup](scripts.md#play-on-your-own-pc), replacing `./yo3-server` with `./target/release/yo3-server` and staying
in the repository root. The play equivalent is:

```bash
./target/release/yo3-launcher --profile /absolute/path/client.json \
  --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3" \
  --injector tools/mod/build/yo3-auth-launch.exe --dll tools/mod/build/yo3-auth.dll
```

Source builds use the checkout's `engine/` data and the `yo3-engine` program beside the server in `target/release/`.
The release bundles carry both; no Node.js is needed anywhere.
Game content comes from the installed client; never commit game files, extracted data or reverse-engineering exports.
The [repo map and exact scratch commands](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md#commands)
cover server setup beside an existing live session.

## Checks

```bash
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 tools/mod/test_fixture_layout.py
python3 tools/client/test_vclient.py
python3 tools/monkey/test_duel_report.py
python3 tools/monkey/test_duel_replay.py
python3 tools/test_package_editions.py
python3 tools/test_docs_check.py
tools/docs-check.py
```

Fetch engine dependencies before engine tests; game-data tests can skip without an original installation.
DLL/loader changes also need `tools/mod/test.sh` and `tools/mod/test-loader.sh`. See the working guide for the
additional direct-launch fixture and prerequisites.

Keep the user's screen, default ports, live data and `:77` client untouched. Use private Xvfb displays, separate
Wine prefixes/documents, scratch server data and free alternate ports through `tools/client/vclient.sh`.
Never run `cargo fmt` across this repository; match its existing style.

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
| Duel check (`--duel-check`), native checkpoints, `duel-report.py` | The server's model of each client, the engine, and the client's own card manager, compared after every step | State drift between engine, server and client | Rendering, menus, animations |
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

For state validation and exact engine replay, enable **Advanced → Hosting → Advanced → All server flags → --duel-check**,
save and restart the managed server between games. For Scripts, pass `--duel-check` to the server; each player's
play command already captures client output. This extra engine work is optional and separate from automatic logging.

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
engine state after every step. A failure reports the seed, step and preceding engine messages.

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
card that should play by the client's 2010–2012 text rather than today's (Goyo Guardian takes any Tuner), or for an
upstream bug (Double Spell). The folder's
[README](https://github.com/usersina/yo3-revival/blob/main/engine/overrides/README.md) lists each one and why. To add
one:

1. Read the client's text: `yo3_data::Card::desc` (CARD_Desc_E), against the database's (`texts.desc` in
   `engine/vendor/BabelCDB/cards.cdb`).
2. Write a failing engine test (`crates/yo3-engine/src/session.rs`: `first_idle` sets cards out and runs to the first
   idle prompt).
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

Four archives are produced under `dist/`:

| Edition | Linux                                             | Windows                                          | Stable inner folder    |
| ------- | ------------------------------------------------- | ------------------------------------------------ | ---------------------- |
| Desktop | `yo3-revival-desktop-VERSION-linux-x86_64.tar.gz` | `yo3-revival-desktop-VERSION-windows-x86_64.zip` | `yo3-revival-desktop/` |
| Scripts | `yo3-revival-scripts-VERSION-linux-x86_64.tar.gz` | `yo3-revival-scripts-VERSION-windows-x86_64.zip` | `yo3-revival-scripts/` |

Both contain the shared `docs/` and root `screenshots/` tree. Desktop opens with the short project README; Scripts
opens with its terminal guide. Packaging validates copied links and anchors before making each archive:

```bash
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-desktop
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-scripts
```

Check the real app and scripts from extracted archives, including paths with spaces and non-default ports/data
folders. Verify Linux/Wine and native Windows separately. Native Windows 11 checks cover Desktop/Scripts hosting
and play, starter setup, a complete NPC duel, log export and updater activation. Windows 10, protected-folder
direct-start installation and runtime reinstallation remain open; see the [current evidence](roadmap.md#status).

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
application storage. It does not start the game or a server. The integration test stays ignored in ordinary
workspace runs because it needs the final archive; the release gate selects it explicitly and fails if no test matches.
This checks Linux package activation; native Windows verification remains separate.

## Publish a release

1. Update the current **Unreleased** changelog entry with player-visible behavior, then give it the next unused
   `## vX.Y.Z (YYYY-MM-DD)` heading. Never move or reuse a published tag.
2. Keep documentation/screenshots current, run required checks and live-test the extracted bundles.
3. Commit in logical slices on a feature branch. Follow the working guide's local merge policy and ask before pushing.
4. A push to `main` with an untagged top changelog version runs the release workflow: tests/lints, four bundles,
   the mandatory packaged-updater check, checksums, release notes and a GitHub release. An already published top
   version skips verification, bundling and publishing after the changelog check.

Desktop checks select only the matching Desktop archive; Scripts updates stay manual. Maintainer architecture and
protocol evidence live in [authentication](authentication.md), [protocol](protocol.md), [client internals](client-internals.md)
and the [roadmap](roadmap.md).
