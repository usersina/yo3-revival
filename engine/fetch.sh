#!/bin/sh
# Fetches what the rules engine needs and is not kept in git: the npm packages (ocgcore-wasm, EDOPro's core
# built for WebAssembly), ProjectIgnis' card scripts and card database, at the commits pinned below, and copies the
# pinned Konami card ids (card-ids.json). Everything goes under engine/vendor/ (ignored).
# `fetch.sh --check` only checks that the tools are installed (Node.js 22.18 or newer, npm, Git, curl).
# `fetch.sh --update` moves to the newest scripts, database and card ids and rewrites the pins: then run the card
# check and the full card sweep (docs/development.md) before committing them. Releases use the pins.
SCRIPTS_COMMIT=915c6ff63518a04e18d16ee45962d75a8c8be264
DATABASE_COMMIT=5b6b23d9fed8d11983dc8272bff7a21a12cfbf5e
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
# A repository at a commit: a fresh shallow fetch of just that commit (GitHub serves commits by id).
at_commit() {
  dir=$1 url=$2 commit=$3
  [ -d "$dir/.git" ] || git init -q "$dir"
  [ "$(git -C "$dir" rev-parse -q --verify HEAD 2>/dev/null)" = "$commit" ] && return
  git -C "$dir" fetch -q --depth 1 "$url" "$commit"
  git -C "$dir" checkout -q --force FETCH_HEAD
  git -C "$dir" clean -q -fdx
}
if [ "$1" = "--update" ]; then
  SCRIPTS_COMMIT=$(git ls-remote https://github.com/ProjectIgnis/CardScripts HEAD | cut -f1)
  DATABASE_COMMIT=$(git ls-remote https://github.com/ProjectIgnis/BabelCDB HEAD | cut -f1)
  curl -fsS --retry 3 -o vendor/cardinfo.full.json 'https://db.ygoprodeck.com/api/v7/cardinfo.php?misc=yes'
  node card-ids.mjs vendor/cardinfo.full.json card-ids.json
  rm -f vendor/cardinfo.full.json
  sed -i.bak -e "s/^SCRIPTS_COMMIT=.*/SCRIPTS_COMMIT=$SCRIPTS_COMMIT/" -e "s/^DATABASE_COMMIT=.*/DATABASE_COMMIT=$DATABASE_COMMIT/" fetch.sh
  rm -f fetch.sh.bak
  echo "Pinned CardScripts $SCRIPTS_COMMIT, BabelCDB $DATABASE_COMMIT and new card ids. Run the card check and the full card sweep before committing."
fi
at_commit vendor/CardScripts https://github.com/ProjectIgnis/CardScripts "$SCRIPTS_COMMIT"
at_commit vendor/BabelCDB https://github.com/ProjectIgnis/BabelCDB "$DATABASE_COMMIT"
# Konami card ids for every passcode (YGOPRODeck's misc_info.konami_id) and its artworks, to map the client's cards
# to the engine's: pinned in git, since the live list changes.
cp card-ids.json vendor/cardinfo.json
