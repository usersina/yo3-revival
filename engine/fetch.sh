#!/bin/sh
# Fetches what the rules engine needs and is not kept in git: the npm packages (ocgcore-wasm, EDOPro's core
# built for WebAssembly), ProjectIgnis' card scripts and card database. Everything goes under engine/ (ignored).
# `fetch.sh --check` only checks that the tools are installed (Node.js 22.18 or newer, npm, Git, curl).
cd "$(dirname "$0")"

missing=""
need() {
  command -v "$1" >/dev/null 2>&1 || missing="$missing
  - $1: $2"
}
need node "install Node.js (version 22.18 or newer, the LTS one) from https://nodejs.org/"
need npm "it comes with Node.js: https://nodejs.org/"
need git "install Git from https://git-scm.com/downloads"
need curl "install curl with your package manager (for example: sudo apt install curl)"
if [ -n "$missing" ]; then
  echo "Cannot set up the rules engine: these are missing:$missing"
  echo "Without them duels cannot be played. Install them, then run this again."
  exit 1
fi
v=$(node --version 2>/dev/null)
v=${v#v}
major=${v%%.*}
minor=${v#*.}
minor=${minor%%.*}
case "$major$minor" in
  '' | *[!0-9]*)
    major=0
    minor=0
    ;;
esac
if [ "$major" -lt 22 ] || { [ "$major" -eq 22 ] && [ "$minor" -lt 18 ]; }; then
  echo "Node.js $v is too old: the rules engine needs 22.18 or newer. Without it every duel ends at once as a draw."
  echo "Install the current LTS from https://nodejs.org/ and run this again."
  exit 1
fi
[ "$1" = "--check" ] && exit 0

set -e
npm install --no-audit --no-fund
mkdir -p vendor
if [ ! -d vendor/CardScripts ]; then
  git clone --depth 1 https://github.com/ProjectIgnis/CardScripts vendor/CardScripts
fi
if [ ! -d vendor/BabelCDB ]; then
  git clone --depth 1 https://github.com/ProjectIgnis/BabelCDB vendor/BabelCDB
fi
# Konami card ids for every passcode (misc_info.konami_id), to map the client's cards to the engine's.
if [ ! -s vendor/cardinfo.json ]; then
  curl -fsS --retry 3 -o vendor/cardinfo.json 'https://db.ygoprodeck.com/api/v7/cardinfo.php?misc=yes'
fi
