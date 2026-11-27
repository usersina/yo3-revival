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
| `c511000138.lua` | Dark Element (anime, passcode 511000138) | "You cannot Normal or Special Summon other monsters": upstream's lock had no exception and stopped the card's own Special Summon of Dark Guardian. |
| `c511002048.lua` | Call of the Haunted (anime, passcode 511002048) | A monster destroyed by battle comes back with 10% more than its own ATK: its printed ATK, or what an earlier revival by this card gave it (Warwolf 2000 with Axe of Despair: 2200; with a new Axe 3200; destroyed again: 2420). Another card's boost falls off with that card and one given later applies on top; a monster back another way starts from its printed ATK again. Upstream took its ATK in the Graveyard and never reset the effect, so the revivals never grew (`tests/anime_call_of_the_haunted.rs`). |
| `c511000428.lua` | Limit Tribute (anime, passcode 511000428) | "Both players can only Tribute 1 monster per turn" also stops a Summon or Set that needs 2 or more Tributes; upstream counted a Tribute only after it happened, so a two-Tribute Summon went through. |
| `c511000438.lua` | Seven Kid Goats (anime, passcode 511000438) | Destroys the face-up Beast it picks and inflicts its ATK; upstream picked it as the effect resolved and then required a relation to the effect, which only a targeted card has, so nothing was ever destroyed (found by its scenario test). |
| `c511001499.lua` | Take One Chance (anime, passcode 511001499) | "Activate that card immediately, if able": the chosen card's condition, cost and target are checked as Take One Chance resolves, and a card that can't activate stays in the GY; upstream checked them only outside a chain, so Battle Fusion (Anime) ran outside a battle and stopped on a Lua error. A check that errors counts as failing (Dark Magic Retribution's condition reads the attacker, and outside an attack there is none: batch 4's fuzz). |
| `c511000271.lua` | Nibelung's Ring (anime, passcode 511000271) | "If the monster equipped with this card is destroyed by battle with a monster, return this card to the Deck. Then shuffle the Deck": upstream never scripted that sentence, so the Ring went to the Graveyard (found by its scenario test, `tests/anime_only_04.rs`). |
| `c810000014.lua` | The Legendary Gambler (anime, passcode 810000014) | Answers only the opponent's die roll for an effect in a chain, once per chain link; upstream answered every roll, its own too, so its own roll started it again (the core's "recursive event trigger") and a roll outside a chain had no effect to negate (found by batch 4's fuzz, `tests/anime_only_04.rs`). |
| `c511000086.lua` | Bounce Spell (anime, passcode 511000086) | Resolves with nothing when its target is gone; upstream used the missing target and raised a Lua error (the per-card fuzz, seed 59357). |
| `c511000116.lua` | Spirit Shield (anime, passcode 511000116) | "Banish 1 Fiend or Zombie-Type monster from your Graveyard; negate that attack": negated even when banishing the last one destroyed the card first (upstream then negated nothing). |
| `c511004015.lua` | Catapult Turtle (anime, passcode 511004015) | Its second effect resolves with nothing when the Tributed monster took the only face-up Spell or Trap with it (an Equip Spell); upstream raised a Lua error partway through the effect (self-play, seed 509). |
| `c511001134.lua` | Advanced Crystal Beast Emerald Tortoise (anime, passcode 511001134) | "1 monster you control that attacked this turn": marked when it attacks; upstream read its attack count, which the core resets when the Battle Phase ends, so the effect was never offered in Main Phase 2. |
| `c511001419.lua` | Iron Cage (anime, passcode 511001419) | The caged monster goes to its owner's Graveyard (upstream passed its reason as the player, so an opponent's monster went to the activating player's Graveyard). |
| `proc_synchro.lua` | Every Synchro Summon's procedure (`Synchro.AddProcedure`) | The same material checks, each answer remembered for the card and groups it was asked with during one Synchro Summon's check, and a choice cut short once the picked materials' lowest Levels sum past the Synchro's (not with a card that decides the Levels itself). The unchanged checks try every order of the materials: with the anime T.G. Cyber Magician's hand materials, a hand of eight monsters took 19 s and ten more than 30. `tests/synchro_memo.rs` holds the answers equal to ProjectIgnis' on random boards (300 compared on 2026-11-25, none differing). |
| `proc_equip.lua` | Every Equip Spell's procedure (`aux.AddEquipProcedure`) | An Equip Spell that resolves with no target does nothing (upstream raised a Lua error mid-effect: a live duel logs it and goes on, the self-play fails): the anime Orichalcos Sword of Sealing's discard cost can wipe the field before the target is chosen (self-play, seed 568). |
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
