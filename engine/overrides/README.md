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
| `c8131171.lua` | Sinister Serpent | No once-per-turn limit by name (each copy still returns once a turn), and no "Sinister Serpent" banished afterwards. |
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
| `c57728570.lua` | Crush Card Virus | Checks the opponent's hand, field and the cards they draw until the end of their 3rd turn (today: hand and field once, then up to 3 from the Deck, and no damage to them for two turns). |
| `c87910978.lua` | Brain Control | Takes any face-up monster (today: one that can be Normal Summoned/Set). |
| `c47355498.lua` | Necrovalley | A card in the Graveyard can still use its own effects ("except for their own effects"). |
| `c80168720.lua` | Darkness Approaches | Flips the monster face-down without changing its battle position (today: face-down Defense Position). |
| `c61740673.lua` | Imperial Order | 700 LP in your own Standby Phases only, and you may choose not to pay (today: every Standby Phase, mandatory). |
| `c77565204.lua` | Future Fusion | The materials go to the Graveyard when it is activated (today: in your 1st Standby Phase). |
| `c513000079.lua` | Dark Strike Fighter (anime, passcode 513000079) | Our own script for the anime card: ProjectIgnis has only a placeholder. A reconstruction (Tribute another monster, Level x 200, no attack the turn it is used); its data record is in `engine/anime/records.json`. |
| `c511247002.lua` | Fortune Lady Fire (anime, passcode 511247002) | Our own script for the anime card: the official card's, with the destroy trigger on any face-up Attack Position Special Summon. Data record in `engine/anime/records.json`. |
| `c513000137.lua` | The Wicked Dreadroot (manga) | Halves every other monster's ATK and DEF but an infinite one (9,999,999 or more): half of infinity is still infinite, so the anime Geh keeps its "100 times your LP" damage and Avatar ties it rather than beating it by 1. |
| `proc_unofficial.lua` | The anime and manga cards' shared procedures | The Divine Hierarchy kept face-down (its protections carry EFFECT_FLAG_SET_AVAILABLE; a God another card's effect turns face-down is face-up in Attack Position again at the end of that turn). The Divine Hierarchy's End Phase return skipped for a God that Sphere Mode Special Summons, which also keeps Sphere Mode's 4000 ATK/DEF past the 1-turn limit (a house rule: it is Ra out of its sphere form, docs/anime-cards.md), and its battle check: a monster without a Hierarchy counts as 0, so a manga God "cannot be destroyed by battle with a monster with lower Divine Hierarchy" (and its controller takes no battle damage) against ordinary monsters too, as the procedure already counts them for monster effects. Upstream gave no protection then. |
| `c82732705.lua` | Skill Drain | Only for the Gods the `anime-cards` setting plays (the manga Egyptian and Wicked Gods, whose "other cards' effects only apply to this card for 1 turn"; the anime Gods' scripts too): negated until the end of the turn Skill Drain is activated, not affected when summoned later. Every other monster as upstream. |
| `c15894048.lua` | Ultimate Tyranno | Must attack first and attack every monster if it is in face-up Attack Position when your Battle Phase begins (today: may attack them all, and locks your other monsters). From ProjectIgnis' `pre-errata/c511003009.lua`. |
| `c88264978.lua` | Red-Eyes Darkness Metal Dragon | Its Special Summon from the hand has no limit, and "Once per turn" is for each copy (today: both once per turn by name). Today's script is ProjectIgnis' `pre-errata/c88264978.lua`. |
| `c65622692.lua` | Y-Dragon Head | The old Union rules (`aux.AddUnionProcedure(..., true)`): Special Summoned in face-up Attack Position, 1 Union per monster, protects from destruction by battle only. |
| `c64500000.lua` | Z-Metal Tank | As Y-Dragon Head. |
| `c96300057.lua` | W-Wing Catapult | As Y-Dragon Head. |
| `c23265594.lua` | Heavy Mech Support Platform | The old Union rules, protecting from any destruction (`..., true, false)`), as the client's "would be destroyed" says. |
| `c67159705.lua` | Armored Cybern | As Heavy Mech Support Platform. |
| `c42940404.lua` | Machina Gearframe | As Heavy Mech Support Platform. |
| `c78349103.lua` | Machina Peacekeeper | As Heavy Mech Support Platform. |
| `c43586926.lua` | Twin-Headed Behemoth | Comes back in the End Phase by itself (today: "You can"). |
| `c16226786.lua` | Night Assailant | Returns any Flip Effect Monster but itself, another Night Assailant too (today: not "Night Assailant"). |

When `engine/fetch.sh --update` moves the scripts' pin, compare each upstream script with the one it was copied from: an
upstream fix may make an override unneeded, or need carrying over.
