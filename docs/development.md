# Development guide

These commands run in a **source checkout**, not an extracted release. For playing, use
[Desktop](launcher.md) or [Scripts](scripts.md). Contributors should first read the
[working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md).

- [Build from source](#build-from-source)
- [Checks](#checks)
- [Duel diagnostics](#duel-diagnostics)
- [Build release bundles](#build-release-bundles)
- [Publish a release](#publish-a-release)

## Build from source

On Linux, install Rust/Cargo, Node.js **22.18 or newer** with npm, Git, curl, and 32-bit MinGW GCC/binutils
(`i686-w64-mingw32-gcc`, `i686-w64-mingw32-dlltool`). Linux game testing needs Wine.
The GUI also needs pkg-config and X11/Wayland/xkbcommon development libraries; on Debian/Ubuntu these are
`pkg-config libx11-dev libwayland-dev libxkbcommon-dev`.

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

In Desktop, enable **Advanced → Activity → Diagnostics options → Detailed tracing and duel validation**, save,
then restart the managed server between games and restart each game. For Scripts, use `--duel-check` on the server
and `--detailed-logs` on each player's play command. Explicit Desktop flag overrides take priority over defaults.

The server compares its model of both clients with the rules engine after each step. The required DLL independently
reports native slots, card visibility, hands/piles, LP, stats, counters, turns and phases. Exact engine inputs, shuffled
deck order and dependency fingerprints are recorded for replay. This catches model/native drift; it does not prove
that every rendered animation or legal menu interaction is correct.

Collect the server and both players' matching session logs, including **every rotated part**. GUI users can export
[diagnostic ZIPs](launcher.md#logs-and-support). Detailed logs can reveal deck contents, player names and chat;
review them before sharing and turn tracing off afterwards.

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

## Publish a release

1. Update the current **Unreleased** changelog entry with player-visible behavior, then give it the next unused
   `## vX.Y.Z (YYYY-MM-DD)` heading. Never move or reuse a published tag.
2. Keep documentation/screenshots current, run required checks and live-test the extracted bundles.
3. Commit in logical slices on a feature branch. Follow the working guide's local merge policy and ask before pushing.
4. A push to `main` with an untagged top changelog version runs the release workflow: tests/lints, four bundles,
   checksums, release notes and a GitHub release. An already published top version releases nothing.

Desktop checks select only the matching Desktop archive; Scripts updates stay manual. Maintainer architecture and
protocol evidence live in [authentication](authentication.md), [protocol](protocol.md), [client internals](client-internals.md)
and the [roadmap](roadmap.md).
