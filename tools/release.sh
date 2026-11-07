#!/bin/sh
# Builds the host's bundles: dist/yo3-revival-<version>-linux-x86_64.tar.gz and -windows-x86_64.zip, each with the
# server, the rules engine's sources (its packages, scripts and card database are fetched on the host by
# engine/fetch.sh or fetch.cmd), a launcher and the setup guide. No game files go in: the server reads the host's own
# game install. The Windows build needs the x86_64-pc-windows-gnu target and mingw-w64.
set -e
cd "$(dirname "$0")/.."
version=$(git describe --tags --always --dirty 2>/dev/null || echo dev)
name="yo3-revival-$version-linux-x86_64"
out="dist/$name"
cargo build --release -p yo3-server
cargo build --release -p yo3-server --target x86_64-pc-windows-gnu
rm -rf "$out"
mkdir -p "$out/engine"
cp target/release/yo3-server "$out/"
cp engine/duelcore.ts engine/package.json engine/package-lock.json engine/fetch.sh "$out/engine/"
cp docs/friends.md "$out/README.md"
cat > "$out/run.sh" <<'EOF'
#!/bin/sh
# Starts the server for friends: every network interface, accounts and records in ./data (backed up at each start).
# Extra options go to the server, e.g. ./run.sh --advertise my.host.name   (see ./yo3-server --help)
cd "$(dirname "$0")"
[ -s engine/vendor/cardinfo.json ] || engine/fetch.sh
exec ./yo3-server --bind 0.0.0.0 --data-dir data --log-dir logs "$@"
EOF
chmod +x "$out/run.sh" "$out/engine/fetch.sh"
tar -C dist -czf "dist/$name.tar.gz" "$name"
echo "dist/$name.tar.gz"

# Windows: the same with .cmd launchers (CRLF line ends).
win="yo3-revival-$version-windows-x86_64"
out="dist/$win"
rm -rf "$out"
mkdir -p "$out/engine"
cp target/x86_64-pc-windows-gnu/release/yo3-server.exe "$out/"
cp engine/duelcore.ts engine/package.json engine/package-lock.json "$out/engine/"
cp docs/friends.md "$out/README.md"
crlf() { sed 's/$/\r/' > "$1"; }
crlf "$out/engine/fetch.cmd" <<'EOF'
@echo off
rem Fetches what the rules engine needs and is not shipped: the npm packages (ocgcore-wasm), ProjectIgnis' card
rem scripts and card database, and the Konami card ids. Needs Node.js, Git and curl (Windows 10 and later have curl).
cd /d "%~dp0"
call npm install --no-audit --no-fund || exit /b 1
if not exist vendor mkdir vendor
if not exist vendor\CardScripts git clone --depth 1 https://github.com/ProjectIgnis/CardScripts vendor\CardScripts || exit /b 1
if not exist vendor\BabelCDB git clone --depth 1 https://github.com/ProjectIgnis/BabelCDB vendor\BabelCDB || exit /b 1
if not exist vendor\cardinfo.json curl -fsS --retry 3 -o vendor\cardinfo.json "https://db.ygoprodeck.com/api/v7/cardinfo.php?misc=yes" || exit /b 1
EOF
crlf "$out/run.cmd" <<'EOF'
@echo off
rem Starts the server for friends: every network interface, accounts and records in .\data (backed up at each start).
rem Extra options go to the server, e.g. run.cmd --advertise my.host.name   (see yo3-server.exe --help)
cd /d "%~dp0"
if not exist engine\vendor\cardinfo.json call engine\fetch.cmd || exit /b 1
yo3-server.exe --bind 0.0.0.0 --data-dir data --log-dir logs %*
EOF
rm -f "dist/$win.zip"
(cd dist && python3 -c 'import shutil, sys; shutil.make_archive(sys.argv[1], "zip", ".", sys.argv[1])' "$win")
echo "dist/$win.zip"
