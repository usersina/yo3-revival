#!/bin/sh
# tools/engine-files.sh DEST: what the engine program reads, into DEST/engine (tools/release.sh, for each platform; run
# from the repository root). Only the scripts it loads (the root utilities, official/, and pre-errata/ and
# pre-release/, where a few current scripts live), our own scripts that take the place of some (engine/overrides/),
# only cards.cdb, and the pinned Konami card ids (each card's passcode, artworks and Konami ids). The program itself
# goes beside yo3-server. Desktop 0.0.9 and older check an update for the old Node sidecar's files before installing
# it; small notes stand at those paths so that they still update to this version (nothing reads them).
set -e
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
cp -r engine/overrides "$e/overrides"
cp engine/vendor/ygopro-core/LICENSE "$e/LICENSE-AGPL.txt"
core_commit=$(sed -n 's/^CORE_COMMIT=//p' engine/fetch.sh)
lua_commit=$(sed -n 's/^LUA_COMMIT=//p' engine/fetch.sh)
cat > "$e/THIRD-PARTY.md" <<EOF
# What the rules engine is made of

yo3-revival's own code is MIT (../LICENSE). The rules engine is built from other people's work, shipped unchanged
except for the few card scripts in \`overrides/\`:

| Part | Where | License | Source |
|---|---|---|---|
| yo3-engine: EDOPro's core and Lua, built into the program with yo3-revival's engine glue (crates/yo3-engine) | \`../yo3-engine\` | AGPL-3.0-or-later (\`LICENSE-AGPL.txt\`): the core's; Lua's own code is MIT | core $core_commit: https://github.com/edo9300/ygopro-core/tree/$core_commit , Lua $lua_commit: https://github.com/lua/lua/tree/$lua_commit , glue: this release's source at https://github.com/usersina/yo3-revival |
| ProjectIgnis card scripts (\`official/\` and the root scripts) | \`vendor/CardScripts/\` | AGPL-3.0 (\`COPYING\`) | https://github.com/ProjectIgnis/CardScripts |
| yo3-revival's card script overrides, each from ProjectIgnis' script of the same name (\`overrides/README.md\` says what each changes) | \`overrides/\` | AGPL-3.0, like the scripts they come from | this release's source at https://github.com/usersina/yo3-revival (\`engine/overrides/\`) |
| ProjectIgnis card database (\`cards.cdb\`) | \`vendor/BabelCDB/\` | see its README | https://github.com/ProjectIgnis/BabelCDB |
| Konami card ids (each card's ids and artworks only, from YGOPRODeck's card info) | \`vendor/cardinfo.json\` | | https://ygoprodeck.com/api-guide/ |

To get newer card scripts before the next release, replace \`vendor/CardScripts/\` and \`vendor/BabelCDB/cards.cdb\` with
fresh copies from the sources above. A script in \`overrides/\` still takes the place of the one of the same name.
EOF
