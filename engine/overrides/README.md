# Card script overrides

The rules engine loads a script from this folder instead of ProjectIgnis' script of the same name
(`vendor/CardScripts/`): a card that should play by the client's 2010–2012 text rather than today's, or an upstream
script with a bug. Each one starts from ProjectIgnis' script at the pinned `SCRIPTS_COMMIT` (`engine/fetch.sh`) and
says in a comment what it changes. These scripts are AGPL-3.0, like the ones they come from
(`vendor/CardScripts/COPYING`).

| Script | Card | Why |
|---|---|---|
| `c7391448.lua` | Goyo Guardian | The client's text: "1 Tuner + 1 or more non-Tuner monsters". Today's text wants an EARTH Tuner. |
| `c24096228.lua` | Double Spell | Two Double Spells in the opponent's Graveyard checked each other forever (a Lua stack overflow). |

When `engine/fetch.sh --update` moves the scripts' pin, compare each upstream script with the one it was copied from: an
upstream fix may make an override unneeded, or need carrying over.
