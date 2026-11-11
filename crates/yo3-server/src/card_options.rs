//! Labels for options that are a card's own strings (EDOPro's descriptions, `aux.Stringid`: `code << 20 | n`): the
//! duel prompt table's texts the original server listed per card (400-495, and 369/370 for a side of the field),
//! paired by hand with EDOPro's strings where both say the same thing (**guess**: the original pairing per card is
//! unknown). A card's options get the select dialog only when every one of them has a label here.
// `code << 20 | 0` keeps every entry in the same shape.
#![allow(clippy::identity_op, reason = "Keep zero-valued option indices visible in wire option constants.")]

pub const CARD_OPTION_LABELS: [(u64, u32); 110] = [
    (967928 << 20 | 0, 465), // Penalty Game!: Your opponent cannot draw during their next Draw Phase
    (967928 << 20 | 1, 466), // Penalty Game!: Your opponent cannot activate any Spell or Trap Cards this t
    (1804528 << 20 | 1, 426), // Dark Coffin: Discard 1 random card
    (1804528 << 20 | 2, 427), // Dark Coffin: Select 1 monster on your field and destroy it
    (5817857 << 20 | 0, 492), // Koa'ki Meiru Ghoulungulate: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (5817857 << 20 | 1, 493), // Koa'ki Meiru Ghoulungulate: Reveal 1 Zombie-Type monster in your hand
    (5817857 << 20 | 2, 427), // Koa'ki Meiru Ghoulungulate: Destroy "Koa'ki Meiru Ghoulungulate"
    (8842266 << 20 | 0, 406), // Poison of the Old Man: Gain 1200 Life Points.
    (8842266 << 20 | 1, 420), // Poison of the Old Man: Inflict 800 damage to your opponent.
    (10032958 << 20 | 1, 462), // Divine Dragon - Excelion: This card gains 1000 ATK
    (10032958 << 20 | 2, 463), // Divine Dragon - Excelion: If this card destroys your opponent's monster by battle, it 
    (10032958 << 20 | 3, 464), // Divine Dragon - Excelion: When this card destroys a monster by battle and sends it to 
    (10060427 << 20 | 0, 492), // Koa'ki Meiru Rooklord: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (10060427 << 20 | 1, 493), // Koa'ki Meiru Rooklord: Reveal 1 Warrior-Type monster in your hand
    (10060427 << 20 | 2, 427), // Koa'ki Meiru Rooklord: Destroy "Koa'ki Meiru Rooklord"
    (12183332 << 20 | 1, 424), // Card Shuffle: Shuffle my Deck
    (12183332 << 20 | 2, 425), // Card Shuffle: Shuffle opponen't Deck
    (12435193 << 20 | 0, 492), // Koa'ki Meiru Drago: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (12435193 << 20 | 1, 493), // Koa'ki Meiru Drago: Reveal 1 Dragon-Type monster in your hand
    (12435193 << 20 | 2, 427), // Koa'ki Meiru Drago: Destroy "Koa'ki Meiru Drago"
    (15552258 << 20 | 0, 489), // Half or Nothing: Halve the ATK of all monsters you control until the end of t
    (15552258 << 20 | 1, 490), // Half or Nothing: End the Battle Phase
    (19642889 << 20 | 0, 492), // Koa'ki Meiru Powerhand: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (19642889 << 20 | 1, 493), // Koa'ki Meiru Powerhand: Reveal 1 Normal Trap Card from your hand
    (19642889 << 20 | 2, 427), // Koa'ki Meiru Powerhand: Destroy "Koa'ki Meiru Powerhand"
    (20065322 << 20 | 0, 471), // The Flute of Summoning Kuriboh: Add card
    (20065322 << 20 | 1, 478), // The Flute of Summoning Kuriboh: Special Summon
    (21496848 << 20 | 1, 440), // Evigishki Tetrogre: Monster card
    (21496848 << 20 | 2, 441), // Evigishki Tetrogre: Spell card
    (21496848 << 20 | 3, 442), // Evigishki Tetrogre: Trap card
    (24673894 << 20 | 0, 406), // Changing Destiny: Gain Life Points equal to half the ATK of the monster whose 
    (24673894 << 20 | 1, 420), // Changing Destiny: Inflict damage to your opponent equal to half the ATK of the
    (29424328 << 20 | 1, 438), // Diabolos, King of the Abyss: Put card at top of deck
    (29424328 << 20 | 2, 439), // Diabolos, King of the Abyss: Put card at bottom of deck
    (30461781 << 20 | 0, 448), // Legacy of Yata-Garasu: Draw 1 card
    (30461781 << 20 | 1, 449), // Legacy of Yata-Garasu: Draw 2 cards
    (30936186 << 20 | 0, 492), // Koa'ki Meiru Urnight: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (30936186 << 20 | 1, 493), // Koa'ki Meiru Urnight: Reveal 1 Beast-Warrior-Type monster in your hand
    (30936186 << 20 | 2, 427), // Koa'ki Meiru Urnight: Destroy "Koa'ki Meiru Urnight"
    (32314730 << 20 | 0, 492), // Koa'ki Meiru Crusader: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (32314730 << 20 | 1, 493), // Koa'ki Meiru Crusader: Reveal 1 Beast-Warrior-Type monster in your hand
    (32314730 << 20 | 2, 427), // Koa'ki Meiru Crusader: Destroy "Koa'ki Meiru Crusader"
    (32588805 << 20 | 1, 441), // The Despair Uranus: Continuous Spell
    (32588805 << 20 | 2, 442), // The Despair Uranus: Continuous Trap
    (33970665 << 20 | 0, 369), // Guts of Steel: Special Summon to your field
    (33970665 << 20 | 1, 370), // Guts of Steel: Special Summon to opponent's field
    (39037517 << 20 | 0, 492), // Koa'ki Meiru Beetle: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (39037517 << 20 | 1, 493), // Koa'ki Meiru Beetle: Reveal 1 Insect-Type Monster in your hand
    (39037517 << 20 | 2, 427), // Koa'ki Meiru Beetle: Destroy "Koa'ki Meiru Beetle"
    (39238953 << 20 | 0, 471), // Lullaby of Obedience: Add to Hand
    (39238953 << 20 | 1, 478), // Lullaby of Obedience: Special Summon
    (40695128 << 20 | 2, 438), // Maharaghi: Return to top of deck
    (40695128 << 20 | 3, 439), // Maharaghi: Return to bottom of deck
    (41201555 << 20 | 0, 492), // Koa'ki Meiru Gravirose: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (41201555 << 20 | 1, 493), // Koa'ki Meiru Gravirose: Reveal 1 Plant-Type monster in your hand
    (41201555 << 20 | 2, 427), // Koa'ki Meiru Gravirose: Destroy "Koa'ki Meiru Gravirose"
    (41858121 << 20 | 0, 448), // Vylon Matter: Draw card
    (41858121 << 20 | 1, 486), // Vylon Matter: Destroy 1 card your opponent controls
    (45041488 << 20 | 0, 492), // Koa'ki Meiru Guardian: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (45041488 << 20 | 1, 493), // Koa'ki Meiru Guardian: Reveal 1 Rock-Type monster in your hand
    (45041488 << 20 | 2, 427), // Koa'ki Meiru Guardian: Destroy "Koa'ki Meiru Guardian"
    (48576971 << 20 | 1, 448), // Necklace of Command: Draw 1 card
    (48576971 << 20 | 2, 422), // Necklace of Command: Discard 1 random card from your opponent's hand
    (49680980 << 20 | 0, 492), // Koa'ki Meiru Sandman: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (49680980 << 20 | 1, 493), // Koa'ki Meiru Sandman: Reveal 1 Rock-Type monster in your hand
    (49680980 << 20 | 2, 427), // Koa'ki Meiru Sandman: Destroy "Koa'ki Meiru Sandman"
    (53839837 << 20 | 1, 440), // Vampire Lord: Monster card
    (53839837 << 20 | 2, 441), // Vampire Lord: Spell card
    (53839837 << 20 | 3, 442), // Vampire Lord: Trap card
    (54289683 << 20 | 1, 467), // Power Capsule: Gain 400 ATK
    (54289683 << 20 | 2, 468), // Power Capsule: Destroy 1 face-up Spell or Trap card
    (54289683 << 20 | 3, 469), // Power Capsule: Special Summon 1 "Option Token"
    (54520292 << 20 | 0, 492), // Koa'ki Meiru Ice: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (54520292 << 20 | 1, 493), // Koa'ki Meiru Ice: Reveal 1 Continuous Spell Card in your hand
    (54520292 << 20 | 2, 427), // Koa'ki Meiru Ice: Destroy "Koa'ki Meiru Ice"
    (58074572 << 20 | 0, 406), // Mooyan Curry: Increase your LP
    (58074572 << 20 | 1, 407), // Mooyan Curry: Increase your opponent's LP
    (61587183 << 20 | 3, 438), // Dark Scorpion - Chick the Yellow: Place it on the top of the Deck
    (61587183 << 20 | 4, 439), // Dark Scorpion - Chick the Yellow: Place it on the bottom of the Deck
    (65026212 << 20 | 0, 492), // Koa'ki Meiru Maximus: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (65026212 << 20 | 2, 427), // Koa'ki Meiru Maximus: Destroy "Koa'ki Meiru Maximus"
    (66816282 << 20 | 0, 492), // Koa'ki Meiru Wall: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (66816282 << 20 | 1, 493), // Koa'ki Meiru Wall: Reveal 1 Rock-Type monster in your hand
    (66816282 << 20 | 2, 427), // Koa'ki Meiru Wall: Destroy "Koa'ki Meiru Wall"
    (68809475 << 20 | 0, 492), // Koa'ki Meiru Speeder: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (68809475 << 20 | 1, 493), // Koa'ki Meiru Speeder: Reveal 1 Machine-Type monster in your hand
    (68809475 << 20 | 2, 427), // Koa'ki Meiru Speeder: Destroy "Koa'ki Meiru Speeder"
    (74576482 << 20 | 0, 492), // Koa'ki Meiru Sea Panther: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (74576482 << 20 | 1, 493), // Koa'ki Meiru Sea Panther: Reveal 1 WATER monster in your hand
    (74576482 << 20 | 2, 427), // Koa'ki Meiru Sea Panther: Destroy "Koa'ki Meiru Sea Panther"
    (76614003 << 20 | 1, 438), // Dark Tinker: Return to the top of the deck
    (76614003 << 20 | 2, 439), // Dark Tinker: Return to the bottom of the deck
    (80367387 << 20 | 0, 492), // Koa'ki Meiru Bergzak: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (80367387 << 20 | 1, 493), // Koa'ki Meiru Bergzak: Reveal 1 Warrior-Type monster in your hand
    (80367387 << 20 | 2, 427), // Koa'ki Meiru Bergzak: Destroy "Koa'ki Meiru Bergzak"
    (80925836 << 20 | 0, 492), // Koa'ki Meiru Doom: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (80925836 << 20 | 1, 493), // Koa'ki Meiru Doom: Reveal 1 Fiend-Type monster in your hand
    (80925836 << 20 | 2, 427), // Koa'ki Meiru Doom: Destroy "Koa'ki Meiru Doom"
    (82012319 << 20 | 1, 369), // Scrap Golem: Special Summon to your field
    (82012319 << 20 | 2, 370), // Scrap Golem: Special Summon to the opponent's field
    (86198326 << 20 | 0, 408), // 7 Completed: Gain 700 ATK
    (86198326 << 20 | 1, 409), // 7 Completed: Gain 700 DEF
    (93130021 << 20 | 1, 467), // Victory Viper XX03: This card gains 400 ATK.
    (93130021 << 20 | 2, 468), // Victory Viper XX03: Destroy 1 face-up Spell or Trap card
    (93130021 << 20 | 3, 469), // Victory Viper XX03: Special Summon 1 "Option Token"
    (95090813 << 20 | 0, 492), // Koa'ki Meiru War Arms: Send "Iron Core of Koa'ki Meiru" to the Graveyard
    (95090813 << 20 | 1, 493), // Koa'ki Meiru War Arms: Reveal 1 Warrior-Type monster in your hand
    (95090813 << 20 | 2, 427), // Koa'ki Meiru War Arms: Destroy "Koa'ki Meiru War Arms"
    (96565487 << 20 | 1, 438), // Blizzard Warrior: Place it on the top of the Deck
    (96565487 << 20 | 2, 439), // Blizzard Warrior: Place it on the bottom of the Deck
];
