#!/bin/sh
# Builds Desktop and Scripts editions for Linux and Windows, with versioned archives and stable inner folders.
# Each edition is ready to
# run with nothing to install: the server, the rules engine (yo3-engine: EDOPro's core built in), ProjectIgnis' card
# scripts and card database, a launcher and the setup guide. No game files go in: the server reads the host's own game
# install. The Windows build needs the x86_64-pc-windows-gnu target and mingw-w64. Building needs the network once: the
# engine's sources and data (engine/fetch.sh, when not fetched yet).
set -e
cd "$(dirname "$0")/.."
# The version in the bundles' names: YO3_VERSION (the release workflow passes the CHANGELOG.md version), else git's.
version=${YO3_VERSION:-$(git describe --tags --always --dirty 2>/dev/null || echo dev)}
export YO3_BUILD_VERSION="$version"
export YO3_BUILD_COMMIT="${YO3_BUILD_COMMIT:-$(git rev-parse --short=12 HEAD)}"
# The engine's sources and data at the commits pinned in fetch.sh (quick when they are there already): before the
# builds, since yo3-engine compiles EDOPro's core from them.
engine/fetch.sh
cargo build --release -p yo3-server -p yo3-launcher -p yo3-desktop -p yo3-engine
cargo build --release -p yo3-server -p yo3-launcher -p yo3-desktop -p yo3-engine --target x86_64-pc-windows-gnu
tools/mod/build.sh

# What the engine program reads, into $1/engine: only the scripts it loads (the root utilities, official/, and
# pre-errata/ and pre-release/, where a few current scripts live), only cards.cdb, and the pinned Konami card ids (each
# card's passcode, artworks and Konami ids). The program itself goes beside yo3-server. Desktop 0.0.9 and older check
# an update for the old Node sidecar's files before installing it; small notes stand at those paths so that they still
# update to this version (nothing reads them).
engine_files() {
  e="$1/engine"
  mkdir -p "$e/vendor/CardScripts" "$e/vendor/BabelCDB" "$e/node" "$e/node_modules/ocgcore-wasm"
  note="Kept so that yo3-revival Desktop 0.0.9 and older can install this update. Nothing reads it: the rules engine is yo3-engine, beside yo3-server."
  for f in duelcore.ts package.json node_modules/ocgcore-wasm/package.json node/node node/node.exe; do
    case "$f" in *.json) printf '{"description": "%s"}\n' "$note" ;; *) printf '%s\n' "$note" ;; esac > "$e/$f"
  done
  cp engine/vendor/CardScripts/*.lua engine/vendor/CardScripts/COPYING engine/vendor/CardScripts/README.md "$e/vendor/CardScripts/"
  cp -r engine/vendor/CardScripts/official engine/vendor/CardScripts/pre-errata engine/vendor/CardScripts/pre-release "$e/vendor/CardScripts/"
  cp engine/vendor/BabelCDB/cards.cdb engine/vendor/BabelCDB/README.md "$e/vendor/BabelCDB/"
  cp engine/vendor/cardinfo.json "$e/vendor/cardinfo.json"
  cp engine/vendor/ygopro-core/LICENSE "$e/LICENSE-AGPL.txt"
  core_commit=$(sed -n 's/^CORE_COMMIT=//p' engine/fetch.sh)
  lua_commit=$(sed -n 's/^LUA_COMMIT=//p' engine/fetch.sh)
  cat > "$e/THIRD-PARTY.md" <<EOF
# What the rules engine is made of

yo3-revival's own code is MIT (../LICENSE). The rules engine is built from other people's work, shipped unchanged:

| Part | Where | License | Source |
|---|---|---|---|
| yo3-engine: EDOPro's core and Lua, built into the program with yo3-revival's engine glue (crates/yo3-engine) | \`../yo3-engine\` | AGPL-3.0-or-later (\`LICENSE-AGPL.txt\`): the core's; Lua's own code is MIT | core $core_commit: https://github.com/edo9300/ygopro-core/tree/$core_commit , Lua $lua_commit: https://github.com/lua/lua/tree/$lua_commit , glue: this release's source at https://github.com/usersina/yo3-revival |
| ProjectIgnis card scripts (\`official/\` and the root scripts) | \`vendor/CardScripts/\` | AGPL-3.0 (\`COPYING\`) | https://github.com/ProjectIgnis/CardScripts |
| ProjectIgnis card database (\`cards.cdb\`) | \`vendor/BabelCDB/\` | see its README | https://github.com/ProjectIgnis/BabelCDB |
| Konami card ids (each card's ids and artworks only, from YGOPRODeck's card info) | \`vendor/cardinfo.json\` | | https://ygoprodeck.com/api-guide/ |

To get newer card scripts before the next release, replace \`vendor/CardScripts/\` and \`vendor/BabelCDB/cards.cdb\` with
fresh copies from the sources above.
EOF
}

# Keep repository and offline documentation links identical: README, docs/, screenshots/.
docs_files() {
  cp README.md "$1/README.md"
  mkdir -p "$1/docs"
  cp docs/*.md "$1/docs/"
  cp -r screenshots "$1/"
  cp LICENSE "$1/LICENSE"

}

out="dist/.staging/linux-x86_64"
rm -rf "$out"
mkdir -p "$out"
cp target/release/yo3-server target/release/yo3-revival "$out/"
cp target/release/yo3-launcher tools/mod/build/yo3-auth.dll tools/mod/build/yo3-auth-launch.exe "$out/"
cp tools/mod/build/winmm.dll "$out/yo3-bootstrap.dll"
cp target/x86_64-pc-windows-gnu/release/yo3-launcher.exe "$out/yo3-launcher-windows.exe"
cp target/release/yo3-engine "$out/"
engine_files "$out"
docs_files "$out"
chmod +x "$out/yo3-server" "$out/yo3-revival" "$out/yo3-launcher" "$out/yo3-engine"
python3 tools/package-editions.py "$out" linux-x86_64 "$version" dist

# Windows: the same with a .cmd launcher (CRLF line ends).
out="dist/.staging/windows-x86_64"
rm -rf "$out"
mkdir -p "$out"
cp target/x86_64-pc-windows-gnu/release/yo3-server.exe target/x86_64-pc-windows-gnu/release/yo3-revival.exe "$out/"
cp target/x86_64-pc-windows-gnu/release/yo3-launcher.exe tools/mod/build/yo3-auth.dll tools/mod/build/yo3-auth-launch.exe "$out/"
cp target/x86_64-pc-windows-gnu/release/yo3-engine.exe "$out/"
cp tools/mod/build/winmm.dll "$out/yo3-bootstrap.dll"
engine_files "$out"
docs_files "$out"
python3 tools/package-editions.py "$out" windows-x86_64 "$version" dist
