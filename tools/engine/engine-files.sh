#!/bin/sh
# tools/engine/engine-files.sh DEST: what the engine program reads, into DEST/engine (tools/release/release.sh, for each platform; run
# from the repository root). The root utility scripts and, of the card scripts (official/, pre-errata/ and pre-release/,
# and unofficial/ for the anime versions), only those the engine can ever load (tools/engine/engine-scripts.py: the
# game's cards, the added cards, their anime versions and every script those reference); our own scripts that take the
# place of some (engine/overrides/); cards.cdb and cards-unofficial.cdb; the pinned Konami card ids; the reviewed map
# anime-cards.json and the added cards. The program itself goes beside yo3-server.
set -e
e="$1/engine"
mkdir -p "$e/vendor/CardScripts" "$e/vendor/BabelCDB"
cp engine/vendor/CardScripts/*.lua engine/vendor/CardScripts/COPYING engine/vendor/CardScripts/README.md "$e/vendor/CardScripts/"
scripts=$(python3 "$(dirname "$0")/engine-scripts.py" --root .)
printf '%s\n' "$scripts" | (cd engine/vendor/CardScripts && tar cf - -T -) | (cd "$e/vendor/CardScripts" && tar xf -)
cp engine/vendor/BabelCDB/cards.cdb engine/vendor/BabelCDB/cards-unofficial.cdb engine/vendor/BabelCDB/README.md "$e/vendor/BabelCDB/"
cp engine/anime-cards.json "$e/anime-cards.json"
# The cards the game never had (#75): the server adds them after the game's rows and serves them to the launcher.
cp engine/added-cards.json "$e/added-cards.json"
cp engine/vendor/cardinfo.json "$e/vendor/cardinfo.json"
cp -r engine/overrides "$e/overrides"
cp engine/vendor/ygopro-core/LICENSE "$e/LICENSE-AGPL.txt"
core_commit=$(sed -n 's/^CORE_COMMIT=//p' engine/fetch.sh)
lua_commit=$(sed -n 's/^LUA_COMMIT=//p' engine/fetch.sh)
cat > "$e/THIRD-PARTY.md" <<EOF
# What the rules engine is made of

yo3-revival's own code is MIT (../LICENSE). The rules engine is built from other people's work, shipped unchanged
except for the few card scripts in \`overrides/\` and the map \`anime-cards.json\`:

| Part | Where | License | Source |
|---|---|---|---|
| yo3-engine: EDOPro's core and Lua, built into the program with yo3-revival's engine glue (crates/yo3-engine) | \`../yo3-engine\` | AGPL-3.0-or-later (\`LICENSE-AGPL.txt\`): the core's; Lua's own code is MIT | core $core_commit: https://github.com/edo9300/ygopro-core/tree/$core_commit , Lua $lua_commit: https://github.com/lua/lua/tree/$lua_commit , glue: this release's source at https://github.com/usersina/yo3-revival |
| ProjectIgnis card scripts (\`official/\` and the root scripts) | \`vendor/CardScripts/\` | AGPL-3.0 (\`COPYING\`) | https://github.com/ProjectIgnis/CardScripts |
| ProjectIgnis anime card scripts (\`unofficial/\`, read only with the \`anime-cards\` setting on) | \`vendor/CardScripts/unofficial/\` | AGPL-3.0 (\`COPYING\`) | https://github.com/ProjectIgnis/CardScripts/tree/master/unofficial |
| yo3-revival's reviewed map from the game's cards to their anime versions (which card plays as which, and the cards held back and why) | \`anime-cards.json\` | MIT, like yo3-revival | this release's source at https://github.com/usersina/yo3-revival (\`engine/anime-cards.json\`, made by \`tools/engine/anime-map.py\`) |
| yo3-revival's card script overrides, each from ProjectIgnis' script of the same name (\`overrides/README.md\` says what each changes) | \`overrides/\` | AGPL-3.0, like the scripts they come from | this release's source at https://github.com/usersina/yo3-revival (\`engine/overrides/\`) |
| ProjectIgnis card database (\`cards.cdb\`, and \`cards-unofficial.cdb\` for the anime cards) | \`vendor/BabelCDB/\` | see its README | https://github.com/ProjectIgnis/BabelCDB |
| Konami card ids (each card's ids and artworks only, from YGOPRODeck's card info) | \`vendor/cardinfo.json\` | | https://ygoprodeck.com/api-guide/ |

Only the card scripts the engine can load ship (the game's cards, the added cards, their anime versions and what
those reference). To get newer card scripts before the next release, replace \`vendor/CardScripts/\` and
\`vendor/BabelCDB/cards.cdb\` with fresh copies from the sources above. A script in \`overrides/\` still takes the place of the one of the same name.
EOF
