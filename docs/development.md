# Development guide

These commands run in a **source checkout**, not an extracted release. For playing, use
[Desktop](launcher.md) or [Scripts](scripts.md). Contributors should first read the
[working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md).

- [Build from source](#build-from-source)
- [Checks](#checks)
- [Duel diagnostics](#duel-diagnostics)
- [Build release bundles](#build-release-bundles)
- [Verify the packaged updater](#verify-the-packaged-updater)
- [Publish a release](#publish-a-release)

## Build from source

On Linux, install Rust/Cargo **1.98.1 or newer**, Node.js **22.18 or newer** with npm, Git, curl, and 32-bit MinGW GCC/binutils
(`i686-w64-mingw32-gcc`, `i686-w64-mingw32-dlltool`). Linux game testing needs Wine.
The GUI also needs pkg-config and X11/Wayland/xkbcommon development libraries; on Debian/Ubuntu these are
`pkg-config libx11-dev libwayland-dev libxkbcommon-dev`.

The root `Cargo.toml` owns the shared crate version, edition, license, Rust baseline, dependency versions and lint
settings. Every crate inherits them; crate-specific features such as Iced's renderer and Windows APIs stay in that
crate's manifest. Change shared versions there and review `Cargo.lock` when updating dependencies. Internal crate
dependencies use workspace paths and all seven crates advance together. Published bundle versions still come from
`CHANGELOG.md` through `YO3_BUILD_VERSION`; reorganizing package metadata does not publish or bump a release.

```bash
git clone https://github.com/usersina/yo3-revival.git
cd yo3-revival
engine/fetch.sh
cargo build --release -p yo3-server -p yo3-launcher -p yo3-desktop
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

Source builds use the checkout's `engine/` and your Node installation. Release bundles carry their own Node runtime.
Game content comes from the installed client; never commit game files, extracted data or reverse-engineering exports.
The [repo map and exact scratch commands](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md#commands)
cover server setup beside an existing live session.

## Checks

```bash
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
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

The report fails on missing/failed checkpoints and reports coverage gaps. Replay requires the same Node version,
sidecar, core, card scripts, database and card-ID mapping; it compares exact recorded answers and state queries.
Replay exercises the engine, not mouse input or pixels. Use focused live checks for remaining UI behavior.

After translator or sidecar changes, run the full duel check: its 240 self-played duels compare modeled client and
engine state after every step. A failure reports the seed, step and preceding engine messages. Protocol and UI
monkeys are documented in the [working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md#commands).

## Build release bundles

The Windows target also needs 64-bit MinGW (`x86_64-w64-mingw32-gcc`). Install `tar`, `xz`, `sha256sum` and system
`python3`, then run:

```bash
rustup target add x86_64-pc-windows-gnu
tools/release.sh
```

Set `YO3_VERSION` when building a specific unreleased version; otherwise the script uses Git's description.
Node downloads are cached in `dist/.cache` and checked against upstream checksums. Engine scripts, card database,
Node and third-party notices are included. Game files, account profiles and server keys are not.

Four archives are produced under `dist/`:

| Edition | Linux | Windows | Stable inner folder |
|---|---|---|---|
| Desktop | `yo3-revival-desktop-VERSION-linux-x86_64.tar.gz` | `yo3-revival-desktop-VERSION-windows-x86_64.zip` | `yo3-revival-desktop/` |
| Scripts | `yo3-revival-scripts-VERSION-linux-x86_64.tar.gz` | `yo3-revival-scripts-VERSION-windows-x86_64.zip` | `yo3-revival-scripts/` |

Both contain the shared `docs/` and root `screenshots/` tree. Desktop opens with the short project README; Scripts
opens with its terminal guide. Packaging validates copied links and anchors before making each archive:

```bash
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-desktop
tools/docs-check.py --root /absolute/path/to/extracted/yo3-revival-scripts
```

Check the real app and scripts from extracted archives, including paths with spaces and non-default ports/data
folders. Verify Linux/Wine and native Windows separately. The current Desktop changes still need a native Windows
check; Windows Scripts hosting/login and bundled-engine initialization have been exercised under Wine.

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
