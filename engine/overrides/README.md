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
| `c26202165.lua` | Sangan | No once-per-turn limit, and the added card's name isn't locked for the turn. |
| `c78010363.lua` | Witch of the Black Forest | No once-per-turn limit, and the added card's name isn't locked for the turn. |
| `c8131171.lua` | Sinister Serpent | No once-per-turn limit, and no "Sinister Serpent" banished afterwards. |
| `c14878871.lua` | Rescue Cat | No once-per-turn limit, and the summoned Beasts keep their effects. |
| `c40737112.lua` | Dark Magician of Chaos | "When this card is Normal or Special Summoned" it adds a Spell (today: in the End Phase, once per turn); monsters it destroys by battle are removed from play instead of going to the Graveyard. |
| `c50321796.lua` | Brionac, Dragon of the Ice Barrier | Returns cards "from the field", either player's; no once-per-turn limit. |
| `c32646477.lua` | Dark Strike Fighter | No Main Phase 1 condition and no once-per-turn limit. |
| `c25862681.lua` | Ancient Fairy Dragon | "Once per turn" for each copy (today: each effect once per turn by name); the Field Spell it adds may have any name. |
| `c17484499.lua` | Exchange of the Spirit | 15 or more cards in *your* Graveyard (today: in both); no once-per-Duel limit. |
| `c56570271.lua` | Destiny Hero - Disk Commander | Draws 2 every time it is Special Summoned from the Graveyard (today: optional, once per Duel, and not the turn it was sent there). |
| `c36562627.lua` | Second Coin Toss | "Once per turn" for each copy (today: once per turn by name). |
| `c67959180.lua` | Goddess of Whim | No once-per-turn limit. |
| `c21593977.lua` | Makyura the Destructor | "During the turn this card is sent to the Graveyard" (from anywhere), Traps from the hand, any number; not an effect that activates. |
| `c95727991.lua` | Catapult Turtle | No once-per-turn limit; any monster can be Tributed. |
| `c82301904.lua` | Chaos Emperor Dragon - Envoy of the End | No once-per-turn limit or lock on your other cards; 300 damage for every card sent, either player's. |
| `c29762407.lua` | Temple of the Kings | Every Trap can be activated the turn it is Set (today: 1); any monster from the Extra Deck (today: a Fusion Monster); no once-per-turn limits. |
| `c83555666.lua` | Ring of Destruction | Any face-up monster, in either player's turn, any number per turn; both players take its ATK at once. |
| `c70583986.lua` | Dewloren, Tiger King of the Ice Barrier | "Once per turn" for each copy (today: once per turn by name). |
| `c96782886.lua` | Mind Master | Can't Tribute a "Mind Master"; no once-per-turn limit. |
| `c9995766.lua` | Imperial Custom | Continuous Traps "cannot be destroyed", by anything (today: by battle or card effect). No engine test: it differs only for a Continuous Trap destroyed neither by battle nor by an effect (by a rule or as a cost). |
| `c80921533.lua` | Mausoleum of the Emperor | Paying the LP is part of the Normal Summon or Set (today: an effect the turn player activates first). |

When `engine/fetch.sh --update` moves the scripts' pin, compare each upstream script with the one it was copied from: an
upstream fix may make an override unneeded, or need carrying over.
