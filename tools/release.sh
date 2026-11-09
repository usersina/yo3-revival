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
cp docs/guide.md "$out/README.md"
cp LICENSE "$out/LICENSE"
cat > "$out/run.sh" <<'EOF'
#!/bin/sh
# Starts the server for friends: every network interface, accounts and records in ./data (backed up at each start).
# Extra options go to the server, e.g. ./run.sh --advertise my.host.name   (see ./yo3-server --help)
cd "$(dirname "$0")"
# Duels need the rules engine: Node.js 22.18+, npm, Git and curl. Stop here, with a message, rather than start a
# server whose duels all end as draws.
engine/fetch.sh --check || exit 1
if [ ! -s engine/vendor/cardinfo.json ]; then
  engine/fetch.sh || { echo "Fetching the rules engine failed (see above), so duels would not work. Fix it and run ./run.sh again."; exit 1; }
fi
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
cp docs/guide.md "$out/README.md"
cp LICENSE "$out/LICENSE"
crlf() { sed 's/$/\r/' > "$1"; }
crlf "$out/engine/fetch.cmd" <<'EOF'
@echo off
rem Fetches what the rules engine needs and is not shipped: the npm packages (ocgcore-wasm), ProjectIgnis' card
rem scripts and card database, and the Konami card ids. Needs Node.js 22.18 or newer, Git and curl (Windows 10 and
rem later have curl). "fetch.cmd check" only checks that they are installed.
cd /d "%~dp0"
where node >nul 2>nul || set "NO_NODE=1"
where npm >nul 2>nul || set "NO_NPM=1"
where git >nul 2>nul || set "NO_GIT=1"
where curl >nul 2>nul || set "NO_CURL=1"
if not defined NO_NODE if not defined NO_NPM if not defined NO_GIT if not defined NO_CURL goto :havetools
echo Cannot set up the rules engine. Please install:
if defined NO_NODE echo   - Node.js, version 22.18 or newer: https://nodejs.org/
if defined NO_NPM echo   - npm, which comes with Node.js: https://nodejs.org/
if defined NO_GIT echo   - Git: https://git-scm.com/download/win
if defined NO_CURL echo   - curl: Windows 10 and later have it; on older Windows get it from https://curl.se/windows/
echo Without them duels cannot be played. Install them, then run this again.
exit /b 1
:havetools
set "NMAJ=0"
set "NMIN=0"
for /f "tokens=1,2 delims=v." %%a in ('node --version') do (set "NMAJ=%%a" & set "NMIN=%%b")
set "NODEOLD="
if %NMAJ% LSS 22 set "NODEOLD=1"
if %NMAJ% EQU 22 if %NMIN% LSS 18 set "NODEOLD=1"
if defined NODEOLD (
  echo Node.js %NMAJ%.%NMIN% is too old: the rules engine needs 22.18 or newer. Without it every duel ends at once as a draw.
  echo Install the current LTS from https://nodejs.org/ and run this again.
  exit /b 1
)
if /i "%~1"=="check" exit /b 0
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
rem Duels need the rules engine: stop here, with a message, rather than start a server whose duels all end as draws.
call engine\fetch.cmd check || goto :failed
if not exist engine\vendor\cardinfo.json (
  call engine\fetch.cmd || goto :failed
)
yo3-server.exe --bind 0.0.0.0 --data-dir data --log-dir logs %*
exit /b %errorlevel%
:failed
echo.
echo The server was not started. Fix the above and run run.cmd again.
pause
exit /b 1
EOF
rm -f "dist/$win.zip"
(cd dist && python3 -c 'import shutil, sys; shutil.make_archive(sys.argv[1], "zip", ".", sys.argv[1])' "$win")
echo "dist/$win.zip"
