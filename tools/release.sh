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
tools/engine-files.sh "$out"
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
tools/engine-files.sh "$out"
docs_files "$out"
python3 tools/package-editions.py "$out" windows-x86_64 "$version" dist
