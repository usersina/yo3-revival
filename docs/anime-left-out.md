# Anime cards left out

The anime cards of Duel Monsters, GX and 5D's that are not in the game yet, each with why and what adding it would take.
Everything after 5D's (ZEXAL on: Xyz, Pendulum and Link Monsters) is out of scope by the owner's decision of 2026-11-27,
and is not listed. The list comes from the anime-missing report of 2026-11-25 (every card the three series' galleries
show, matched against ProjectIgnis' databases), less what [added cards](added-cards.md) and
[anime versions](anime-cards.md) now cover; `tools/engine/added-cards.py --find CARD` gives the reason for one card.

Not left out: the video game's copies of printed cards ("Afterglow (VG)" and about a hundred more, a printed card's alias
in ProjectIgnis' database), because the printed card plays them; and the anime cards ProjectIgnis marks Video Game rather
than Anime (their texts are the Tag Force games'), added in batches 30-34 as anime-only cards.

| Kind | Cards | Why | What adding them would take |
| --- | --- | --- | --- |
| [Riding Duels](#riding-duels) | 47 | Speed Spells and the cards around them count Speed Counters, which only Speed World puts on a card: a Riding Duel's Field Spell, a duel the game never had | A Riding Duel option (Speed World in both Field Zones from the start, as `--duel-field` does), the client showing Speed Counters (it shows Spell Counters; whether it can show another kind is open), and Speed World's rules for other Spells |
| [Placeholder scripts](#placeholder-scripts) | 14 | ProjectIgnis has the card but its script registers no effect | A script of our own per card in `engine/overrides/`, from the episodes |
| [Not in ProjectIgnis](#not-in-projectignis) | 42 | No database row and no script | A row (stats, text) and a script of our own per card: a source for added cards the tool does not have yet |
| [Other](#other) | 4 | A kind or stats the client can't show, or a data gap | Per card, below |

Each card here would need a stand-in id. On 2026-11-27 the added cards hold 1,269 and 1,008 are free (100 of them kept
back), counting the Konami ids of the Xyz, Link and Pendulum Monsters the client never shows: enough for all of them.

## Riding Duels

The Speed Spells and Speed World; the last six are video game cards that read the Speed Counters on the Field Zone's card
(with no Field Spell there, their scripts stop on a Lua error). `added-cards.py --find` refuses a script that counts
Speed Counters (counter 0x91).

| Card | Series | Deck | Passcodes |
| --- | --- | --- | --- |
| Speed Spell - Accelerator Draw | 5Ds | Hunter Pace | 100100112 |
| Speed Spell - Angel Baton | 5Ds | Crow Hogan | 100100102, 513000145 |
| Speed Spell - Count Up | 5Ds | Tetsu Trudge | 100100103 |
| Speed Spell - Curse of Fate | 5Ds | Carly Carmine | 100100125, 513000146 |
| Speed Spell - Dash Pilfer | 5Ds | Yusei Fudo | 100100118 |
| Speed Spell - Deceased Synchron | 5Ds | Yusei Fudo | 100100133, 513000149 |
| Speed Spell - Defense Buster | 5Ds | Yusei Fudo | 511000093 |
| Speed Spell - Final Attack | 5Ds | Yusei Fudo | 100100120, 513000148 |
| Speed Spell - Gap Storm | 5Ds | Yusei Fudo | 100100117 |
| Speed Spell - Half Seize | 5Ds | Jack Atlas | 100100124, 513000150 |
| Speed Spell - High Speed Crash | 5Ds | Yusei Fudo | 100100123 |
| Speed Spell - Ignition | 5Ds | Taro Yamashita | 100100129 |
| Speed Spell - Lightning Rod | 5Ds | Primo | 511000972 |
| Speed Spell - Overboost | 5Ds | Jack Atlas | 100100122 |
| Speed Spell - Overspeed | 5Ds | Yusei Fudo | 511000780 |
| Speed Spell - Overtune | 5Ds | Yusei Fudo | 100100114, 511600294 |
| Speed Spell - Power Baton | 5Ds | Kalin Kessler | 100100121 |
| Speed Spell - Rapid Shotwing | 5Ds | Tetsu Trudge | 100100101 |
| Speed Spell - Reactor Pod | 5Ds | Yusei Fudo | 100100127 |
| Speed Spell - Shift Down | 5Ds | Yusei Fudo | 100100107 |
| Speed Spell - Silent Burn | 5Ds | Jakob | 511000977 |
| Speed Spell - Silver Contrails | 5Ds | Yusei Fudo | 100100110, 513000152 |
| Speed Spell - Sonic Buster | 5Ds | Tetsu Trudge | 100100111, 513000153 |
| Speed Spell - Speed Energy | 5Ds | Yusei Fudo | 100100106, 513000154 |
| Speed Spell - Speed Force | 5Ds | Crow Hogan | 100100109, 513000155 |
| Speed Spell - Speed Fusion | 5Ds | Jack Atlas | 100100100 |
| Speed Spell - Speed Storm | 5Ds | Tetsu Trudge | 100100132 |
| Speed Spell - Star Force | 5Ds | Yusei Fudo | 100100115 |
| Speed Spell - Summon Close | 5Ds | Lester | 100100105, 513000156 |
| Speed Spell - Summon Speeder | 5Ds | Hunter Pace | 100100131 |
| Speed Spell - Synchro Defuse | 5Ds | Yusei Fudo | 100100119 |
| Speed Spell - Synchro Panic | 5Ds | Yusei Fudo | 100100525 |
| Speed Spell - Synchro Return | 5Ds | Yusei Fudo | 100100130 |
| Speed Spell - Takeover | 5Ds | Jakob | 511000978 |
| Speed Spell - The End of the Storm | 5Ds | Jack Atlas | 100100104, 513000147 |
| Speed Spell - Tune Up 123 | 5Ds | Crow Hogan | 511000944 |
| Speed Spell - Tyrant Force | 5Ds | Jean | 511000779, 513000157 |
| Speed Spell - Vision Wind | 5Ds | Yusei Fudo | 100100128, 513000158 |
| Speed Spell - Zero Reverse | 5Ds | Yusei Fudo | 100100116, 513000159 |
| Speed World | 5Ds | Yusei Fudo | 511600371, 511600373 |
| Speed World 2 | 5Ds | Yusei Fudo | 511600372, 511600374 |
| Acceleration Zone | 5Ds | Hunter Pace | 100100150 |
| Des Accelerator | 5Ds | Hunter Pace | 100100156 |
| Full Throttle | 5Ds | Sector Security | 100100153 |
| Slip Stream | 5Ds | Yusei Fudo | 100100154 |
| Speed Booster | 5Ds | Hunter Pace | 100100151 |
| Speed Edge | 5Ds | Sector Security | 100100152 |

## Placeholder scripts

ProjectIgnis' file for each is a placeholder ("TBR", to be reworked), or its anime effect is reconstructed and not
scripted (Globerman).

| Card | Series | Deck | Passcodes |
| --- | --- | --- | --- |
| Question Change | GX | Bob Banter | 511002792 |
| Quiz Hour | GX | Bob Banter | 511002885 |
| Quiz Panel - Obelisk 10 | GX | Bob Banter | 511002781 |
| Quiz Panel - Obelisk 20 | GX | Bob Banter | 511002782 |
| Quiz Panel - Obelisk 30 | GX | Bob Banter | 511001940 |
| Quiz Panel - Ra 10 | GX | Bob Banter | 511002786 |
| Quiz Panel - Ra 20 | GX | Bob Banter | 511002787 |
| Quiz Panel - Ra 30 | GX | Bob Banter | 511002788 |
| Quiz Panel - Slifer 10 | GX | Bob Banter | 511002783 |
| Quiz Panel - Slifer 20 | GX | Bob Banter | 511002784 |
| Quiz Panel - Slifer 30 | GX | Bob Banter | 511002785 |
| Quizzer Panel 9 | GX | Bob Banter | 511002791 |
| Search Shock | GX | Bob Banter | 511009202 |
| Globerman | DM | Leon von Schroeder | 511001414 |

## Not in ProjectIgnis

Neither database has them, under any name.

| Card | Series | Deck | Passcodes |
| --- | --- | --- | --- |
| Abyss Guardian | 5Ds | Familiar of Red Nova | - |
| Abyss Ruler Mictlancoatl | 5Ds | Familiar of Red Nova | - |
| Granel Attack 3 | 5Ds | Jakob | - |
| Granel Guard 3 | 5Ds | Jakob | - |
| Infinite Feast | 5Ds | Jakob | - |
| Monster Baton | 5Ds | Jack Atlas | - |
| Overpower | 5Ds | Jack Atlas | - |
| Smile Angel | 5Ds | Marco | - |
| Cursed Twin Dolls | DM | Yami Bakura | - |
| Death Hand | DM | Seeker | - |
| Glassman | DM | Joey Wheeler | - |
| Impact Revive | DM | Dartz | - |
| Joyful Doom | DM | Yami Marik | - |
| Landstar Shot | DM | Joey Wheeler | - |
| Magical Neutralizing Force Field | DM | Maximillion Pegasus | - |
| Matter Leveler | DM | Noah Kaiba | - |
| Mirage Spell | DM | Yugi Muto | - |
| Surprise Attack from Beyond | DM | Yami Marik | - |
| Alice in Despairland | GX | - | - |
| Bit Shoot | GX | Frost | - |
| Camouflage Gardna | GX | Trapper | - |
| Code Change | GX | Jaden Yuki | - |
| Combat Scissor Beetle | GX | Chazz Princeton | - |
| Counterattack! | GX | Tyranno Hassleberry | - |
| Cut Jewel | GX | Jesse Anderson | - |
| Drain Time | GX | The D | - |
| Gachi Battle | GX | Howard X Miller | - |
| Hunter Channel | GX | Bob Banter | - |
| Immortal Class | GX | Yubel (character) | - |
| Lord of Darkness | GX | Sarina | - |
| Mecha Ojama King | GX | Chazz Princeton | - |
| Mecha Ojama King Transformation | GX | Chazz Princeton | - |
| Memory Snatcher | GX | Yusuke Fujiwara | - |
| Mirror Trap | GX | Sarina | - |
| Nightmare Shuffle | GX | Yubel (character) | - |
| Partner Change | GX | Blair Flannigan | - |
| Reflector Bit | GX | Frost | - |
| Revolving Stage | GX | Frost | - |
| Short Circuit (GX) | GX | - | - |
| Side Attack | GX | Frost | - |
| Spike Egg | GX | Blair Flannigan | - |
| Trap Sluzer | GX | Howard X Miller | - |

## Other

| Card | Series | Passcodes | Why | What it would take |
| --- | --- | --- | --- | --- |
| Big Bang Dragon Blow | DM | 170000196 | A Fusion that is an Equip Spell (Hermos' Fusion): no game card has that kind. The five printed Hermos cards (Time Magic Hammer and the others) play their anime Spells through their printed rows ([anime versions](anime-cards.md)) | A row of a kind the client has (a Spell in the Extra Deck, as those five are shown), with the tool allowing it for an anime-only card |
| Seven-Armed Fiend | DM | 511001076 | ATK and DEF 666: the client keeps both in tens, so it would show 660 or 670 | A stand-in shown with the nearest value and the text saying 666, if the owner accepts that |
| Card of Last Will | DM | 450000130, 513000037 | ProjectIgnis marks the card Illegal (scope 0x8) and the anime one is its alias, so the tool finds neither | The tool accepting an anime card whose base is Illegal |
| Chaos Form (GX) | GX | 100000258 | The anime version of the printed Chaos Form (21082832, printed after 5D's), which the game doesn't have | Adding the printed Chaos Form, then this as its anime version |
