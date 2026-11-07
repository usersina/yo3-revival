#!/bin/sh
# Fetches what the rules engine needs and is not kept in git: the npm packages (ocgcore-wasm, EDOPro's core
# built for WebAssembly), ProjectIgnis' card scripts and card database. Everything goes under engine/ (ignored).
set -e
cd "$(dirname "$0")"
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
