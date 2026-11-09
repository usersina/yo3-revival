#!/bin/sh
# Builds the host's bundles: dist/yo3-revival-<version>-linux-x86_64.tar.gz and -windows-x86_64.zip, each ready to
# run with nothing to install: the server, the rules engine with its own Node.js, ocgcore-wasm, ProjectIgnis' card
# scripts and card database, a launcher and the setup guide. No game files go in: the server reads the host's own game
# install. The Windows build needs the x86_64-pc-windows-gnu target and mingw-w64. Building needs the network once: the
# engine's data (engine/fetch.sh, when not fetched yet) and Node.js (cached in dist/.cache, checked against nodejs.org's
# SHASUMS256.txt).
set -e
cd "$(dirname "$0")/.."
# The version in the bundles' names: YO3_VERSION (the release workflow passes the CHANGELOG.md version), else git's.
version=${YO3_VERSION:-$(git describe --tags --always --dirty 2>/dev/null || echo dev)}
# The Node.js shipped in the bundles (an LTS line; the engine needs 22.18 or newer).
NODE_VERSION=v26.13.0
cache=dist/.cache
cargo build --release -p yo3-server
cargo build --release -p yo3-server --target x86_64-pc-windows-gnu
[ -s engine/vendor/cardinfo.json ] && [ -d engine/node_modules/ocgcore-wasm ] || engine/fetch.sh

# Node.js's archive $1, downloaded once into the cache and checked against the release's SHASUMS256.txt.
node_archive() {
  mkdir -p "$cache"
  [ -s "$cache/SHASUMS256-$NODE_VERSION.txt" ] ||
    curl -fsS --retry 3 -o "$cache/SHASUMS256-$NODE_VERSION.txt" "https://nodejs.org/dist/$NODE_VERSION/SHASUMS256.txt"
  [ -s "$cache/$1" ] || curl -fsS --retry 3 -o "$cache/$1" "https://nodejs.org/dist/$NODE_VERSION/$1"
  (cd "$cache" && grep " $1\$" "SHASUMS256-$NODE_VERSION.txt" | sha256sum -c --quiet -) || {
    echo "$cache/$1 does not match nodejs.org's checksum: delete it and run this again" >&2
    exit 1
  }
}

# Everything the engine reads besides Node.js, into $1/engine: the sidecar, its package, only the scripts it loads
# (the root utilities and official/), only cards.cdb, and of YGOPRODeck's card info only each card's Konami id.
engine_files() {
  e="$1/engine"
  mkdir -p "$e/vendor/CardScripts" "$e/vendor/BabelCDB"
  cp engine/duelcore.ts engine/package.json engine/package-lock.json "$e/"
  cp -r engine/node_modules "$e/"
  cp engine/vendor/CardScripts/*.lua engine/vendor/CardScripts/COPYING engine/vendor/CardScripts/README.md "$e/vendor/CardScripts/"
  cp -r engine/vendor/CardScripts/official "$e/vendor/CardScripts/"
  cp engine/vendor/BabelCDB/cards.cdb engine/vendor/BabelCDB/README.md "$e/vendor/BabelCDB/"
  python3 - engine/vendor/cardinfo.json "$e/vendor/cardinfo.json" <<'PY'
import json, sys
cards = json.load(open(sys.argv[1], encoding="utf-8"))["data"]
ids = [{"id": c["id"], "misc_info": [{"konami_id": m["konami_id"]} for m in c.get("misc_info", []) if m.get("konami_id")]} for c in cards]
json.dump({"data": [c for c in ids if c["misc_info"]]}, open(sys.argv[2], "w"), separators=(",", ":"))
PY
  cat > "$e/THIRD-PARTY.md" <<EOF
# What the rules engine is made of

yo3-revival's own code is MIT (../LICENSE). The engine in this folder is other people's work, shipped unchanged:

| Part | Where | License | Source |
|---|---|---|---|
| Node.js $NODE_VERSION | \`node/\` | MIT and others (\`node/LICENSE\`) | https://nodejs.org/ |
| ocgcore-wasm (EDOPro's core built for WebAssembly) | \`node_modules/ocgcore-wasm/\` | MIT (\`LICENSE.md\`); the core inside is ProjectIgnis' EDOPro core, AGPL-3.0 | https://github.com/edo9300/ygopro-core |
| ProjectIgnis card scripts (\`official/\` and the root scripts) | \`vendor/CardScripts/\` | AGPL-3.0 (\`COPYING\`) | https://github.com/ProjectIgnis/CardScripts |
| ProjectIgnis card database (\`cards.cdb\`) | \`vendor/BabelCDB/\` | see its README | https://github.com/ProjectIgnis/BabelCDB |
| Konami card ids (each card's id only, from YGOPRODeck's card info) | \`vendor/cardinfo.json\` | | https://ygoprodeck.com/api-guide/ |

To get newer card scripts before the next release, replace \`vendor/CardScripts/\` and \`vendor/BabelCDB/cards.cdb\` with
fresh copies from the sources above.
EOF
}

# The guide (as README.md, with its pictures at the same relative paths as in docs/) and the license.
docs_files() {
  cp docs/guide.md "$1/README.md"
  mkdir -p "$1/images"
  cp -r docs/images/guide "$1/images/"
  cp docs/images/duel.png "$1/images/"
  cp LICENSE "$1/LICENSE"
}

name="yo3-revival-$version-linux-x86_64"
out="dist/$name"
rm -rf "$out"
mkdir -p "$out"
cp target/release/yo3-server "$out/"
engine_files "$out"
docs_files "$out"
nodeball="node-$NODE_VERSION-linux-x64.tar.xz"
node_archive "$nodeball"
mkdir -p "$out/engine/node"
tar -xJf "$cache/$nodeball" -C "$out/engine/node" --strip-components=2 "node-$NODE_VERSION-linux-x64/bin/node"
tar -xJf "$cache/$nodeball" -C "$out/engine/node" --strip-components=1 "node-$NODE_VERSION-linux-x64/LICENSE"
cat > "$out/run.sh" <<'EOF'
#!/bin/sh
# Starts the server for friends: every network interface, accounts and records in ./data (backed up at each start).
# Extra options go to the server, e.g. ./run.sh --advertise my.host.name   (see ./yo3-server --help)
# The rules engine comes with it (engine/, with its own Node.js): nothing to install.
cd "$(dirname "$0")"
exec ./yo3-server --bind 0.0.0.0 --data-dir data --log-dir logs "$@"
EOF
chmod +x "$out/run.sh" "$out/yo3-server" "$out/engine/node/node"
tar -C dist -czf "dist/$name.tar.gz" "$name"
echo "dist/$name.tar.gz"

# Windows: the same with node.exe and a .cmd launcher (CRLF line ends).
win="yo3-revival-$version-windows-x86_64"
out="dist/$win"
rm -rf "$out"
mkdir -p "$out"
cp target/x86_64-pc-windows-gnu/release/yo3-server.exe "$out/"
engine_files "$out"
docs_files "$out"
nodezip="node-$NODE_VERSION-win-x64.zip"
node_archive "$nodezip"
mkdir -p "$out/engine/node"
python3 - "$cache/$nodezip" "$out/engine/node" "node-$NODE_VERSION-win-x64" <<'PY'
import sys, zipfile
z, dest, top = zipfile.ZipFile(sys.argv[1]), sys.argv[2], sys.argv[3]
for name in ("node.exe", "LICENSE"):
    with z.open(f"{top}/{name}") as src, open(f"{dest}/{name}", "wb") as dst:
        dst.write(src.read())
PY
crlf() { sed 's/$/\r/' > "$1"; }
crlf "$out/run.cmd" <<'EOF'
@echo off
rem Starts the server for friends: every network interface, accounts and records in .\data (backed up at each start).
rem Extra options go to the server, e.g. run.cmd --advertise my.host.name   (see yo3-server.exe --help)
rem The rules engine comes with it (engine\, with its own Node.js): nothing to install.
cd /d "%~dp0"
yo3-server.exe --bind 0.0.0.0 --data-dir data --log-dir logs %*
if errorlevel 1 pause
exit /b %errorlevel%
EOF
rm -f "dist/$win.zip"
(cd dist && python3 -c 'import shutil, sys; shutil.make_archive(sys.argv[1], "zip", ".", sys.argv[1])' "$win")
echo "dist/$win.zip"
