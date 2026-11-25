//! Printed cards from the anime, group 07 (Goddess Urd's Verdict to Infinite Machine), each effect through the real core
//! with both seats' client models checked after every step (`duelcheck::scenario`). Printed effects with the anime cards
//! off; a card whose anime version does something else gets one test of it with the anime cards on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// Cards of this group (client ids).
const GOLD_MOON_COIN: u16 = 14548;
const GUARDED_TREASURE: u16 = 11158;
const HIGH_RATE_DRAW: u16 = 15376;
const HAND_HOLDING_GENIE: u16 = 12542;
const HOLDING_ARMS: u16 = 12236;
const HOLDING_LEGS: u16 = 12237;
const ICE_KNIGHT: u16 = 6861;
const ICE_MIRROR: u16 = 4686;
const HEROS_RULE_1: u16 = 5084;
const HAILON: u16 = 13816; // Hailon, the Timelord
const HEXE_TRUDE: u16 = 13807;
const STROMBERG: u16 = 13808; // Golden Castle of Stromberg
const DREADSCYTHE: u16 = 11155; // Guardian Dreadscythe
const REAPER_SCYTHE: u16 = 11157; // Reaper Scythe - Dreadscythe
const INFINITE_MACHINE: u16 = 13821;
const INFINITE_LIGHT: u16 = 13822;
const EMPTY_MACHINE: u16 = 13820;
const METAION: u16 = 9698; // Metaion, the Timelord
const LAZION: u16 = 13157; // Lazion, the Timelord
const PHOENIX_FORMATION: u16 = 10865; // Harpie Lady Phoenix Formation
const FEATHER_STORM: u16 = 12900; // Harpie's Feather Storm
const GOLDEN_RULE: u16 = 4807;
const GRIEF_TABLET: u16 = 5050;
const HORSE: u16 = 4671; // Horse of the Floral Knights
const HORNED_SAURUS: u16 = 4829;
const KAITOPTERA: u16 = 4827;
const VERDANDE: u16 = 14000; // Goddess Verdande's Guidance
const VERDICT: u16 = 14001; // Goddess Urd's Verdict
const VALKYRIE_ERSTE: u16 = 13994;
const SLINGER: u16 = 5157; // Infernity Doom Slinger
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const BLUE_EYES: u16 = 4007;
const DARK_HOLE: u16 = 4342;
const JELLYFISH: u16 = 4077; // Level 4 WATER Aqua, 1200/1500, no effect
const MONSTURTLE: u16 = 4183; // Level 3 WATER Aqua, 800/1000, no effect
const AVIAN: u16 = 6310; // Elemental HERO Avian
const EATOS: u16 = 8536; // Guardian Eatos
const MST: u16 = 4909; // Mystical Space Typhoon
const POLYMERIZATION: u16 = 4837;
const BOOK_OF_MOON: u16 = 5432;
const URABY: u16 = 4086; // a Level 4 Dinosaur
const BABY_DRAGON: u16 = 4010; // a Level 3 Dragon
const YAMI: u16 = 4341; // a Field Spell
const FEATHER_DUSTER: u16 = 4678; // Harpie's Feather Duster
const HARPIE_LADY: u16 = 4068;
const HARPIE_LADY_1: u16 = 6186;
const HARPIE_LADY_2: u16 = 6187;
const ARCHFIEND: u16 = 8272; // Infernity Archfiend, a Fiend
const AMETHYST_CAT: u16 = 7071; // Crystal Beast Amethyst Cat, Level 3
const RUBY_CARBUNCLE: u16 = 7074; // Crystal Beast Ruby Carbuncle, Level 3
const TOPAZ_TIGER: u16 = 7076; // Crystal Beast Topaz Tiger, Level 4
const JOAN: u16 = 9201; // Noble Knight Joan, a LIGHT Noble Knight

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand_card(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }

/// Both teams with a Kuriboh deck, `spawns` in place; anime cards on with `anime`.
fn field(anime: bool, spawns: &[Spawn]) -> Option<Scenario> {
    field_decks(anime, [&[KURIBOH; 40], &[KURIBOH; 40]], spawns)
}

fn field_decks(anime: bool, decks: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    field_extra(anime, decks, [&[], &[]], spawns)
}

/// `field_decks` with each team's Extra Deck.
fn field_extra(anime: bool, decks: [&[u16]; 2], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(decks[0], extra[0]), (decks[1], extra[1])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// A Kuriboh deck with these cards at its bottom (never drawn in the first turns).
fn deck_with(cards: &[u16]) -> Vec<u16> {
    cards.iter().copied().chain(std::iter::repeat_n(KURIBOH, 40 - cards.len())).collect()
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn defense_of(s: &Scenario, team: usize, cid: u16) -> Option<i64> {
    s.monster(team, cid)?["defense"].as_i64()
}

/// Gold Moon Coin: "Add 2 cards from your hand to your opponent's hand, then draw 2 cards."
#[test]
fn gold_moon_coin_gives_two_cards_and_draws_two() {
    let Some(mut s) = field(false, &[hand_card(0, GOLD_MOON_COIN), hand_card(0, WARWOLF), hand_card(0, WARWOLF)]) else { return };
    let (mine, theirs) = (hand(&s, 0), hand(&s, 1));
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(GOLD_MOON_COIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", WARWOLF), 2, "the two Warwolves went to the opponent: {}", s.context());
    assert_eq!(hand(&s, 1), theirs + 2, "{}", s.context());
    assert_eq!(hand(&s, 0), mine - 1, "the coin, then two given, two drawn: {}", s.context());
    assert_eq!(s.count(0, "grave", GOLD_MOON_COIN), 1);
}

/// Gold Moon Coin (anime): "Add all cards in your hand to your opponent's hand, then, draw the same number of cards that
/// you gave to your opponent."
#[test]
fn gold_moon_coin_anime_gives_the_whole_hand() {
    let Some(mut s) = field(true, &[hand_card(0, GOLD_MOON_COIN), hand_card(0, WARWOLF), hand_card(0, WARWOLF)]) else { return };
    let (mine, theirs) = (hand(&s, 0), hand(&s, 1));
    s.run(&[Activate(GOLD_MOON_COIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", WARWOLF), 2, "{}", s.context());
    assert_eq!(hand(&s, 1), theirs + mine - 1, "the whole hand but the coin went over: {}", s.context());
    assert_eq!(hand(&s, 0), mine - 1, "and as many were drawn: {}", s.context());
}

/// Guarded Treasure: "Activate this card by discarding 5 cards; draw 2 cards. While this card is on the field, draw 2 cards
/// instead of 1 for your normal draw during your Draw Phase."
#[test]
fn guarded_treasure_discards_five_and_draws_two_then_draws_two_each_turn() {
    let Some(mut s) = field(false, &[hand_card(0, GUARDED_TREASURE)]) else { return };
    let before = hand(&s, 0);
    let graves = s.state()[0]["grave"].as_array().unwrap().len();
    s.run(&[Activate(GUARDED_TREASURE)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before - 1 - 5 + 2, "the card played, five discarded, two drawn: {}", s.context());
    assert_eq!(s.state()[0]["grave"].as_array().unwrap().len(), graves + 5, "five discards: {}", s.context());
    assert_eq!(s.count(0, "szone", GUARDED_TREASURE), 1, "it stays on the field");
    s.run(&[EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before - 1 - 5 + 2 + 2, "two cards for the next Draw Phase: {}", s.context());
}

/// High Rate Draw: "Destroy 2 or more monsters you control, and if you do, draw 1 card for every 2 monsters destroyed."
#[test]
fn high_rate_draw_destroys_two_monsters_and_draws_one() {
    let Some(mut s) = field(false, &[set(0, HIGH_RATE_DRAW), mon(0, WARWOLF), mon(0, JELLYFISH)]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(HIGH_RATE_DRAW)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", JELLYFISH)), (1, 1), "both destroyed: {}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "one drawn: {}", s.context());
    assert_eq!(s.count(0, "grave", HIGH_RATE_DRAW), 1);
}

/// High Rate Draw: "During your opponent's Main Phase, if this card is in your GY: You can target 1 monster you control;
/// destroy it, and if you do, Set this card, but banish it when it leaves the field."
#[test]
fn high_rate_draw_sets_itself_from_the_graveyard() {
    let Some(mut s) = field(false, &[grave(0, HIGH_RATE_DRAW), mon(0, WARWOLF), hand_card(1, JELLYFISH)]) else { return };
    s.run_both([&[EndTurn, Chain(HIGH_RATE_DRAW)], &[Summon(JELLYFISH)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the monster was destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", HIGH_RATE_DRAW), 1, "High Rate Draw is Set: {}", s.context());
    assert_eq!(s.count(0, "grave", HIGH_RATE_DRAW), 0);
}

/// Hand-Holding Genie: "Monsters your opponent controls cannot target monsters for attacks, except this one."
#[test]
fn hand_holding_genie_is_the_only_target_for_attacks() {
    let Some(mut s) = field(false, &[mon(0, HAND_HOLDING_GENIE), mon(0, WARWOLF), mon(1, BLUE_EYES)]) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", HAND_HOLDING_GENIE), 1, "Blue-Eyes had to attack the Genie: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf was left alone: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "3000 - 1000 battle damage");
}

/// Hand-Holding Genie: "Gains DEF equal to the combined original DEF of all other face-up Defense Position monsters you
/// control." Warwolf (100) and Kuriboh (200) in Defense Position, and Jellyfish in Attack Position that does not count.
#[test]
fn hand_holding_genie_gains_def_from_the_defense_position_monsters() {
    let spawns = [mon(0, HAND_HOLDING_GENIE), Spawn { team: 0, place: MonsterDefense, cid: WARWOLF },
        Spawn { team: 0, place: MonsterDefense, cid: KURIBOH }, mon(0, JELLYFISH)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(defense_of(&s, 0, HAND_HOLDING_GENIE), Some(1600 + 100 + 200), "{}", s.state());
}

/// Hand-Holding Genie (anime): "Your opponent cannot target another face-up monster you control for an attack. While
/// this card is in face-up Defense Position, it gains DEF equal to the total DEF of all other face-up monsters you
/// control." Warwolf (100) and Jellyfish (1500), both in Attack Position, count with the anime cards on.
#[test]
fn hand_holding_genie_anime_counts_every_face_up_monster() {
    let spawns = [Spawn { team: 0, place: MonsterDefense, cid: HAND_HOLDING_GENIE }, mon(0, WARWOLF), mon(0, JELLYFISH)];
    let Some(mut s) = field(true, &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(defense_of(&s, 0, HAND_HOLDING_GENIE), Some(1600 + 100 + 1500), "{}", s.state());
}

/// Holding Arms: "If this card is Normal or Special Summoned: Target 1 face-up monster your opponent controls; while this
/// card is face-up on the field, that face-up monster cannot attack, also its effects are negated. While that face-up
/// monster is on the field, this card cannot be destroyed by battle or card effects."
#[test]
fn holding_arms_stops_an_attacker_and_survives_dark_hole() {
    let Some(mut s) = field(false, &[hand_card(0, HOLDING_ARMS), mon(1, WARWOLF), hand_card(1, DARK_HOLE)]) else { return };
    s.run(&[Summon(HOLDING_ARMS)]).unwrap();
    clean(&s);
    // The held Warwolf can't attack on its own turn: its attack is not on offer.
    let err = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap_err();
    assert!(err.contains("can't attack"), "{err}");
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HOLDING_ARMS), 1, "Holding Arms survived Dark Hole: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf did not: {}", s.context());
}

/// Holding Legs: "If this card is Summoned: Return all Set Spells/Traps on the field to the hand."
#[test]
fn holding_legs_returns_every_set_spell_and_trap() {
    let Some(mut s) = field(false, &[hand_card(0, HOLDING_LEGS), set(0, DARK_HOLE), set(1, DARK_HOLE)]) else { return };
    let (mine, theirs) = (hand(&s, 0), hand(&s, 1));
    s.run(&[Summon(HOLDING_LEGS)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", DARK_HOLE), s.count(1, "szone", DARK_HOLE)), (0, 0), "{}", s.context());
    assert_eq!((s.count(0, "hand", DARK_HOLE), s.count(1, "hand", DARK_HOLE)), (1, 1), "{}", s.context());
    assert_eq!((hand(&s, 0), hand(&s, 1)), (mine, theirs + 1), "{}", s.context());
}

/// Holding Legs: "You can banish this card from your GY, then target 1 Set Spell/Trap your opponent controls; it cannot be
/// activated until the end of the next turn." The opponent's Set Dark Hole is not on offer in its own turn.
#[test]
fn holding_legs_stops_a_set_card_from_the_graveyard() {
    let Some(mut s) = field(false, &[grave(0, HOLDING_LEGS), set(1, DARK_HOLE), mon(1, WARWOLF)]) else { return };
    s.run(&[Activate(HOLDING_LEGS), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", HOLDING_LEGS), 1, "banished as the cost: {}", s.context());
    let can = s.prompt()["activates"].as_array().unwrap().iter().any(|c| c["cid"] == DARK_HOLE);
    assert!(!can, "Dark Hole can't be activated: {}", s.prompt());
}

/// Ice Knight: "Gains 400 ATK for each Aqua monster you control."
#[test]
fn ice_knight_gains_400_atk_for_each_aqua_monster_you_control() {
    let Some(mut s) = field(false, &[mon(0, ICE_KNIGHT), mon(0, JELLYFISH), mon(1, JELLYFISH)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, ICE_KNIGHT), Some(1300 + 2 * 400), "itself and Jellyfish, not the opponent's: {}", s.state());
}

/// Ice Knight (anime): "This card gains 400 ATK for each face-up Aqua-Type monster on the field." The opponent's too.
#[test]
fn ice_knight_anime_counts_both_fields() {
    let Some(mut s) = field(true, &[mon(0, ICE_KNIGHT), mon(0, JELLYFISH), mon(1, JELLYFISH)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, ICE_KNIGHT), Some(1300 + 3 * 400), "{}", s.state());
}

/// Ice Knight: "Once per turn: You can activate this effect; you can Normal Summon 1 WATER monster during your Main
/// Phase this turn, in addition to your Normal Summon/Set, also you cannot Normal or Special Summon monsters for the
/// rest of this turn, except WATER monsters."
#[test]
fn ice_knight_gives_an_extra_normal_summon_for_water_monsters_only() {
    let spawns = [mon(0, ICE_KNIGHT), hand_card(0, JELLYFISH), hand_card(0, MONSTURTLE), hand_card(0, WARWOLF)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.run(&[Activate(ICE_KNIGHT), Summon(JELLYFISH), Summon(MONSTURTLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", JELLYFISH), s.count(0, "mzone", MONSTURTLE)), (1, 1), "two Normal Summons: {}", s.context());
    let summons = s.prompt()["summons"].as_array().unwrap();
    assert!(!summons.iter().any(|c| c["cid"] == WARWOLF), "no more Summons, and not Warwolf: {}", s.prompt());
}

/// Ice Mirror: "Target 1 Level 3 or lower WATER monster you control; Special Summon 1 monster with the same name from
/// your Deck, but you cannot Special Summon from the Extra Deck while you control that face-up Special Summoned monster."
#[test]
fn ice_mirror_calls_a_copy_from_the_deck() {
    let deck: Vec<u16> = [MONSTURTLE].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &[hand_card(0, ICE_MIRROR), mon(0, MONSTURTLE)]) else { return };
    s.run(&[Activate(ICE_MIRROR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MONSTURTLE), 2, "the copy came: {}", s.context());
}

/// Hero's Rule 1: Five Freedoms: "Target up to 5 cards in any GY(s); banish them."
#[test]
fn heros_rule_1_banishes_from_the_graveyards() {
    let Some(mut s) = field(false, &[set(0, HEROS_RULE_1), grave(1, WARWOLF), grave(0, JELLYFISH)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(HEROS_RULE_1)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "{}", s.context());
}

/// Hero's Rule 1: Five Freedoms: "If this card is in your GY, except the turn it was sent there: You can banish it, then
/// target 1 of your banished "Elemental HERO" monsters; Special Summon it, ignoring its Summoning conditions."
#[test]
fn heros_rule_1_brings_back_a_banished_hero_from_the_graveyard() {
    let spawns = [grave(0, HEROS_RULE_1), Spawn { team: 0, place: Banished, cid: AVIAN }];
    let Some(mut s) = field(false, &spawns) else { return };
    s.run(&[Activate(HEROS_RULE_1)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", AVIAN), 1, "Avian came back: {}", s.context());
    assert_eq!(s.count(0, "removed", HEROS_RULE_1), 1, "the rule banished itself: {}", s.context());
}

/// Hailon, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing. Cannot be
/// destroyed by battle or card effects."
#[test]
fn hailon_is_summoned_without_tributes_and_survives_dark_hole() {
    let Some(mut s) = field(false, &[hand_card(0, HAILON), hand_card(0, DARK_HOLE), mon(1, WARWOLF)]) else { return };
    s.run(&[Summon(HAILON), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HAILON), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole took the Warwolf only: {}", s.context());
}

/// Hailon, the Timelord: "You take no battle damage from attacks involving this card. At the end of the Battle Phase, if
/// this card battled: If your LP is lower than your opponent's, inflict damage to your opponent equal to the difference."
/// Warwolf hits the empty field for 2000; Hailon (0 ATK) then battles it and the 2000 LP difference comes back.
#[test]
fn hailon_inflicts_the_lp_difference_after_battling() {
    let Some(mut s) = field(false, &[hand_card(0, HAILON), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the card under test
    s.run_both([&[EndTurn, Summon(HAILON), Attack(HAILON), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HAILON), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "no battle damage from Hailon's battle: {}", s.context());
    assert_eq!(s.lp(1), 6000, "the 2000 difference: {}", s.context());
}

/// Hailon, the Timelord: "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn hailon_shuffles_itself_into_the_deck_in_your_standby_phase() {
    let Some(mut s) = field(false, &[hand_card(0, HAILON)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Summon(HAILON), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HAILON), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", HAILON), 0, "shuffled, not sent: {}", s.context());
}

/// Golden Castle of Stromberg: "When an opponent's monster declares an attack: Destroy the attacking monster, and if you
/// do, inflict damage to your opponent equal to half the ATK that monster had on the field."
#[test]
fn stromberg_destroys_an_attacker_and_burns_half_its_atk() {
    let Some(mut s) = field(false, &[hand_card(0, STROMBERG), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Activate(STROMBERG), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "half of 2000: {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack never landed");
}

/// Golden Castle of Stromberg: "Once per turn, during your Standby Phase, you must banish 10 cards from the top of your
/// Deck face-down."
#[test]
fn stromberg_banishes_ten_cards_each_standby_phase() {
    let Some(mut s) = field(false, &[hand_card(0, STROMBERG)]) else { return };
    s.run(&[Activate(STROMBERG), EndTurn, EndTurn]).unwrap();
    clean(&s);
    let removed = s.state()[0]["removed"].as_array().map_or(0, Vec::len);
    assert_eq!(removed, 10, "{}", s.context());
    assert_eq!(s.count(0, "szone", STROMBERG), 1, "it stays: {}", s.context());
}

/// Golden Castle of Stromberg: "During your Main Phase: You can Special Summon 1 monster from your Deck that specifically
/// lists the card "Golden Castle of Stromberg" in its text (Hexe Trude). You cannot Normal Summon/Set the turn you
/// activate this effect."
#[test]
fn stromberg_special_summons_hexe_trude_and_stops_normal_summons() {
    let deck = deck_with(&[HEXE_TRUDE]);
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &[hand_card(0, STROMBERG), hand_card(0, WARWOLF)]) else { return };
    s.run(&[Activate(STROMBERG), Activate(STROMBERG)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HEXE_TRUDE), 1, "{}", s.context());
    let none = s.prompt()["summons"].as_array().unwrap().is_empty();
    assert!(none, "no Normal Summon this turn: {}", s.prompt());
}

/// Golden Castle of Stromberg (anime): "You cannot Normal Summon/Set monsters." as long as it is on the field.
#[test]
fn stromberg_anime_stops_normal_summons_while_it_stays() {
    let Some(mut s) = field(true, &[hand_card(0, STROMBERG), hand_card(0, WARWOLF)]) else { return };
    s.run(&[Activate(STROMBERG)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", STROMBERG), 1, "{}", s.context());
    let p = s.prompt();
    assert!(p["summons"].as_array().unwrap().is_empty() && p["monster_sets"].as_array().unwrap().is_empty(), "{p}");
}

/// Hexe Trude: "If "Golden Castle of Stromberg" is in a Field Zone, you can Normal Summon this card without Tributing."
#[test]
fn hexe_trude_is_summoned_without_tributes_under_stromberg() {
    let Some(mut s) = field(false, &[hand_card(0, STROMBERG), hand_card(0, HEXE_TRUDE)]) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the card under test
    s.run(&[Activate(STROMBERG), Summon(HEXE_TRUDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HEXE_TRUDE), 1, "{}", s.context());
}

/// Hexe Trude: "Once per turn, if "Golden Castle of Stromberg" is in a Field Zone: You can target 1 card on the field,
/// except this one; destroy it, and if you do, this card can make up to 2 attacks on monsters during each Battle Phase
/// this turn. When this card destroys a monster by battle: You can target 1 face-up monster you control; it gains 400 ATK."
#[test]
fn hexe_trude_destroys_a_card_attacks_twice_and_grows() {
    let spawns = [hand_card(0, STROMBERG), hand_card(0, HEXE_TRUDE), mon(1, JELLYFISH), mon(1, JELLYFISH), mon(1, JELLYFISH)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, JELLYFISH)]); // the End Phase discard keeps the card under test; the effect's target
    let plan = [Activate(STROMBERG), EndTurn, Summon(HEXE_TRUDE), Activate(HEXE_TRUDE), Attack(HEXE_TRUDE), Chain(HEXE_TRUDE),
        Attack(HEXE_TRUDE), Chain(HEXE_TRUDE)];
    s.run(&plan).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", JELLYFISH), 3, "one by the effect, two in battle: {}", s.context());
    assert_eq!(s.attack_of(0, HEXE_TRUDE), Some(2600 + 800), "400 for each destroyed monster: {}", s.context());
}

/// Guardian Dreadscythe: "If a "Guardian Eatos" is destroyed by battle or card effect and sent to your GY: You can Special
/// Summon this card from your hand. When this card is Special Summoned: You can equip 1 "Reaper Scythe - Dreadscythe"
/// from your Deck to this card."
#[test]
fn dreadscythe_comes_when_eatos_is_destroyed_and_equips_its_scythe() {
    let deck = deck_with(&[REAPER_SCYTHE]);
    let spawns = [hand_card(0, DREADSCYTHE), mon(0, EATOS), mon(1, BLUE_EYES)];
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the card under test
    s.run_both([&[EndTurn, Chain(DREADSCYTHE), Chain(DREADSCYTHE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", EATOS), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DREADSCYTHE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", REAPER_SCYTHE), 1, "the scythe is equipped: {}", s.context());
}

/// Guardian Dreadscythe: "If this card is sent from the field to the GY: Send 1 card from your hand to the GY, and if you
/// do, Special Summon this card from the GY."
#[test]
fn dreadscythe_comes_back_after_a_card_from_the_hand_is_sent() {
    let spawns = [hand_card(0, DREADSCYTHE), mon(0, EATOS), mon(1, BLUE_EYES), hand_card(0, DARK_HOLE)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the card under test
    s.run_both([&[EndTurn, Chain(DREADSCYTHE), Activate(DARK_HOLE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "Dark Hole went off: {}", s.context());
    assert_eq!(s.count(0, "mzone", DREADSCYTHE), 1, "it came back: {}", s.context());
}

fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }

/// Infinite Machine: "Activate this card by sending 1 face-up "Empty Machine" from your Spell & Trap Zone to the GY."
#[test]
fn infinite_machine_is_activated_by_sending_empty_machine_to_the_graveyard() {
    let Some(mut s) = field(false, &[set(0, INFINITE_MACHINE), spell(0, EMPTY_MACHINE)]) else { return };
    s.run(&[Activate(INFINITE_MACHINE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", EMPTY_MACHINE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", INFINITE_MACHINE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", EMPTY_MACHINE), 0);
}

/// Infinite Machine: "Once per turn, you can activate 1 of these effects. During the Main Phase: Special Summon 1
/// "Timelord" monster from your hand."
#[test]
fn infinite_machine_special_summons_a_timelord_from_the_hand() {
    let Some(mut s) = field(false, &[spell(0, INFINITE_MACHINE), hand_card(0, HAILON)]) else { return };
    s.run(&[Activate(INFINITE_MACHINE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HAILON), 1, "{}", s.context());
}

/// Infinite Machine: "You can target 1 "Timelord" monster in your GY; shuffle it into the Deck, then, you can Set 1
/// "Infinite Light" directly from your hand or Deck."
#[test]
fn infinite_machine_shuffles_a_timelord_back_and_sets_infinite_light() {
    let spawns = [spell(0, INFINITE_MACHINE), grave(0, HAILON), hand_card(0, INFINITE_LIGHT)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.run(&[Activate(INFINITE_MACHINE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", HAILON), 0, "shuffled into the Deck: {}", s.context());
    assert_eq!(s.count(0, "szone", INFINITE_LIGHT), 1, "Infinite Light is Set: {}", s.context());
}

/// Infinite Machine: "Once per turn, this face-up card cannot be destroyed by your opponent's card effects." The first
/// Mystical Space Typhoon of the turn fails, the second destroys it.
#[test]
fn infinite_machine_survives_one_destruction_a_turn() {
    let spawns = [spell(0, INFINITE_MACHINE), hand_card(1, MST), hand_card(1, MST)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(0, INFINITE_MACHINE)]);
    s.run_both([&[EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", INFINITE_MACHINE), 1, "the first destruction was replaced: {}", s.context());
    s.run_both([&[], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", INFINITE_MACHINE), 0, "the second one destroys it: {}", s.context());
}

/// Infinite Light: "Activate this card by sending 1 face-up "Infinite Machine" from your Spell & Trap Zone to the GY."
#[test]
fn infinite_light_is_activated_by_sending_infinite_machine_to_the_graveyard() {
    let Some(mut s) = field(false, &[set(0, INFINITE_LIGHT), spell(0, INFINITE_MACHINE)]) else { return };
    s.run(&[Activate(INFINITE_LIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", INFINITE_MACHINE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", INFINITE_LIGHT), 1, "{}", s.context());
}

/// Infinite Light: "Once per turn, if you control no monsters: You can Special Summon up to 1 "Timelord" monster each from
/// your hand, Deck, and GY, with different names, ignoring their Summoning conditions." The choice takes one Timelord
/// here; each source is tried alone (Hailon and Metaion can't be Special Summoned from the Deck otherwise).
#[test]
fn infinite_light_special_summons_timelords_from_hand_deck_and_graveyard() {
    let deck = deck_with(&[METAION]);
    let cases: [(&[u16], &[Spawn], u16); 3] = [
        (&deck, &[], METAION),
        (&[KURIBOH; 40], &[hand_card(0, HAILON)], HAILON),
        (&[KURIBOH; 40], &[grave(0, LAZION)], LAZION),
    ];
    for (deck, extra, timelord) in cases {
        let mut spawns = vec![spell(0, INFINITE_LIGHT)];
        spawns.extend_from_slice(extra);
        let Some(mut s) = field_decks(false, [deck, &[KURIBOH; 40]], &spawns) else { return };
        s.run(&[Activate(INFINITE_LIGHT)]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "mzone", timelord), 1, "{timelord} came: {}", s.context());
    }
}

/// Infinite Light: "Neither player can target "Timelord" monsters you control with card effects, or return "Timelord"
/// monsters from the field to the Deck." Book of Moon is on offer against Hailon alone, not with Infinite Light out;
/// and Hailon's own "shuffle this card into the Deck" in the Standby Phase does nothing.
#[test]
fn infinite_light_protects_timelords_from_targets_and_from_the_deck() {
    let on_offer = |light: bool| -> Option<bool> {
        let mut spawns = vec![hand_card(0, HAILON), hand_card(1, BOOK_OF_MOON)];
        if light {
            spawns.push(spell(0, INFINITE_LIGHT));
        }
        let mut s = field(false, &spawns)?;
        s.prefer(&[(0, KURIBOH)]);
        s.run(&[Summon(HAILON), EndTurn]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "mzone", HAILON), 1, "{}", s.context());
        Some(s.prompt()["activates"].as_array().unwrap().iter().any(|c| c["cid"] == BOOK_OF_MOON))
    };
    let Some(without) = on_offer(false) else { return };
    assert!(without, "Book of Moon can target Hailon without the card");
    assert_eq!(on_offer(true), Some(false), "Hailon can't be targeted");
    let Some(mut s) = field(false, &[hand_card(0, HAILON), spell(0, INFINITE_LIGHT)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Summon(HAILON), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HAILON), 1, "not shuffled into the Deck: {}", s.context());
}

/// Harpie Lady Phoenix Formation: "If you control 3 or more "Harpie Lady" and/or "Harpie Lady Sisters": Target as many
/// monsters your opponent controls as possible, but not more than the total number of "Harpie Lady" and "Harpie Lady
/// Sisters" you control; destroy those targets, and if you do, inflict damage to your opponent equal to the highest
/// original ATK among those destroyed monsters."
#[test]
fn phoenix_formation_destroys_three_monsters_and_burns_the_highest_atk() {
    let spawns = [hand_card(0, PHOENIX_FORMATION), mon(0, HARPIE_LADY), mon(0, HARPIE_LADY_1), mon(0, HARPIE_LADY_2),
        mon(1, WARWOLF), mon(1, JELLYFISH), mon(1, MONSTURTLE), mon(1, KURIBOH)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(1, JELLYFISH), (1, MONSTURTLE), (1, KURIBOH)]);
    s.run(&[Activate(PHOENIX_FORMATION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf was not a target: {}", s.context());
    assert_eq!((s.count(1, "grave", JELLYFISH), s.count(1, "grave", MONSTURTLE)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1200, "the highest original ATK among the destroyed, not their total: {}", s.context());
}

/// Harpie Lady Phoenix Formation: "You cannot ... conduct your Battle Phase the turn you activate this card."
#[test]
fn phoenix_formation_ends_your_battle_phase_for_the_turn() {
    let spawns = [hand_card(0, PHOENIX_FORMATION), mon(0, HARPIE_LADY), mon(0, HARPIE_LADY_1), mon(0, HARPIE_LADY_2), mon(1, WARWOLF)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[EndTurn, Activate(PHOENIX_FORMATION)]).unwrap();
    clean(&s);
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase: {}", s.prompt());
}

/// Harpie Lady Phoenix Formation (anime): "Destroy monsters your opponent controls, up to the number of "Harpie Lady" you
/// control. Then your opponent loses Life Points equal to the total ATK of the destroyed monster(s)." One Harpie Lady is
/// enough (the printed card needs three) and the monster destroyed is paid for in LP.
#[test]
fn phoenix_formation_anime_needs_one_harpie_lady_and_takes_the_atk() {
    let spawns = [hand_card(0, PHOENIX_FORMATION), mon(0, HARPIE_LADY), mon(1, WARWOLF), mon(1, JELLYFISH)];
    let Some(mut s) = field(true, &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(PHOENIX_FORMATION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", JELLYFISH), 1, "one Harpie Lady, one monster: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
}

/// Harpie's Feather Storm: "If you control a WIND Winged Beast monster: Until the end of this turn, negate any monster
/// effects your opponent activates. If you control a "Harpie" monster, you can activate this card from your hand."
/// Ice Knight's effect, activated by the opponent after the Storm, is negated: no extra Normal Summon.
#[test]
fn feather_storm_negates_the_opponents_monster_effects_from_the_hand() {
    for storm in [false, true] {
        let spawns = [mon(1, HARPIE_LADY), hand_card(1, FEATHER_STORM), hand_card(0, ICE_KNIGHT), hand_card(0, JELLYFISH)];
        let Some(mut s) = field(false, &spawns) else { return };
        let theirs: &[yo3_server::duelcheck::scenario::Action] = if storm { &[Chain(FEATHER_STORM)] } else { &[] };
        s.run_both([&[Summon(ICE_KNIGHT), Activate(ICE_KNIGHT)], theirs]).unwrap();
        clean(&s);
        let summons = s.prompt()["summons"].as_array().unwrap();
        let extra = summons.iter().any(|c| c["cid"] == JELLYFISH);
        assert_eq!(extra, !storm, "the extra Normal Summon is there unless the Storm negated the effect: {}", s.context());
        assert_eq!(s.count(1, "grave", FEATHER_STORM), usize::from(storm), "played from the hand: {}", s.context());
    }
}

/// Harpie's Feather Storm: "If this card in its owner's Spell & Trap Zone is destroyed by an opponent's card effect: You
/// can add 1 "Harpie's Feather Duster" from your Deck or GY to your hand."
#[test]
fn feather_storm_searches_a_feather_duster_when_destroyed() {
    let deck = deck_with(&[FEATHER_DUSTER]);
    let spawns = [set(0, FEATHER_STORM), hand_card(1, MST)];
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, FEATHER_STORM)]);
    s.run_both([&[EndTurn, Chain(FEATHER_STORM)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FEATHER_STORM), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", FEATHER_DUSTER), 1, "{}", s.context());
}

/// Golden Rule: "Place 2 "Crystal Beast" monsters from your Deck face-up in your Spell & Trap Zone as Continuous Spells,
/// then Special Summon 1 "Crystal Beast" monster from your hand or GY with a different name than those cards, and if you
/// do, equip it with this card. When this card leaves the field, destroy that monster."
#[test]
fn golden_rule_places_two_crystal_beasts_and_summons_a_third() {
    let deck = deck_with(&[AMETHYST_CAT, RUBY_CARBUNCLE]);
    let spawns = [hand_card(0, GOLDEN_RULE), hand_card(0, TOPAZ_TIGER)];
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(GOLDEN_RULE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", AMETHYST_CAT), s.count(0, "szone", RUBY_CARBUNCLE)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "mzone", TOPAZ_TIGER), 1, "the third came from the hand: {}", s.context());
    assert_eq!(s.count(0, "szone", GOLDEN_RULE), 1, "equipped to it: {}", s.context());
}

/// Golden Rule: "When this card leaves the field, destroy that monster."
#[test]
fn golden_rule_destroys_its_monster_when_it_leaves() {
    let deck = deck_with(&[AMETHYST_CAT, RUBY_CARBUNCLE]);
    let spawns = [hand_card(0, GOLDEN_RULE), hand_card(0, TOPAZ_TIGER), hand_card(1, MST)];
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, GOLDEN_RULE), (0, KURIBOH)]);
    s.run_both([&[Activate(GOLDEN_RULE), EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GOLDEN_RULE), 1, "{}", s.context());
    // Destroyed, a Crystal Beast places itself in the Spell & Trap Zone as a Continuous Spell.
    assert_eq!((s.count(0, "mzone", TOPAZ_TIGER), s.count(0, "szone", TOPAZ_TIGER)), (0, 1), "Topaz Tiger was destroyed: {}", s.context());
}

/// Grief Tablet: "The equipped monster cannot attack or be Tributed, also its effects are negated. ... If this card is sent
/// to the GY because the equipped monster is destroyed: You can inflict 500 damage to your opponent."
#[test]
fn grief_tablet_stops_attacks_and_burns_for_500_when_the_monster_is_destroyed() {
    let spawns = [hand_card(0, GRIEF_TABLET), mon(0, WARWOLF), hand_card(0, DARK_HOLE)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    let err = s.run(&[Activate(GRIEF_TABLET), EndTurn, Attack(WARWOLF)]).unwrap_err();
    assert!(err.contains("can't attack") || err.contains("never became possible"), "the equipped Warwolf never attacks: {err}");
    let Some(mut s) = field(false, &[mon(0, WARWOLF)]) else { return };
    s.run(&[EndTurn, Attack(WARWOLF)]).unwrap();
    assert_eq!(s.lp(1), 6000, "without the Tablet it attacks");
    let Some(mut s) = field(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(GRIEF_TABLET), Activate(DARK_HOLE), Chain(GRIEF_TABLET)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", GRIEF_TABLET), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7500, "500 damage: {}", s.context());
}

/// Horse of the Floral Knights: "If this card is Normal or Special Summoned: You can add 1 LIGHT "Noble Knight" monster
/// from your Deck to your hand."
#[test]
fn horse_of_the_floral_knights_adds_a_noble_knight() {
    let deck = deck_with(&[JOAN]);
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &[hand_card(0, HORSE)]) else { return };
    s.run(&[Summon(HORSE), Chain(HORSE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", JOAN), 1, "{}", s.context());
}

/// Horse of the Floral Knights (anime): "When this card is Normal Summoned: You can draw 1 card."
#[test]
fn horse_of_the_floral_knights_anime_draws_a_card() {
    let deck = deck_with(&[JOAN]);
    let Some(mut s) = field_decks(true, [&deck, &[KURIBOH; 40]], &[hand_card(0, HORSE)]) else { return };
    let before = hand(&s, 0);
    s.run(&[Summon(HORSE), Chain(HORSE)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before, "the Horse left the hand, a card was drawn: {}", s.context());
    assert_eq!(s.count(0, "hand", JOAN), 0, "no search with the anime version: {}", s.context());
}

/// Infernity Doom Slinger: "During your Main Phase: You can Special Summon 1 Fiend monster from your hand."
#[test]
fn doom_slinger_special_summons_a_fiend_from_the_hand() {
    let Some(mut s) = field(false, &[mon(0, SLINGER), hand_card(0, ARCHFIEND)]) else { return };
    s.prefer(&[(0, ARCHFIEND)]);
    s.run(&[Activate(SLINGER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARCHFIEND), 1, "{}", s.context());
}

/// Holding Legs (anime): "If this card is Summoned: Return all Set cards on the field to the hand." A Set monster goes
/// too, which the printed card leaves.
#[test]
fn holding_legs_anime_returns_set_monsters_too() {
    for anime in [false, true] {
        let spawns = [hand_card(0, HOLDING_LEGS), Spawn { team: 1, place: MonsterSet, cid: WARWOLF }, set(1, DARK_HOLE)];
        let Some(mut s) = field(anime, &spawns) else { return };
        s.run(&[Summon(HOLDING_LEGS)]).unwrap();
        clean(&s);
        assert_eq!(s.count(1, "hand", DARK_HOLE), 1, "the Set Spell returns either way: {}", s.context());
        assert_eq!(s.count(1, "hand", WARWOLF), usize::from(anime), "the Set monster only in the anime: {}", s.context());
    }
}

/// Hexe Trude (anime): "If this card destroys an opponent's monster by battle: It gains 400 ATK." Without the castle.
#[test]
fn hexe_trude_anime_gains_400_atk_after_destroying_a_monster() {
    let Some(mut s) = field(true, &[mon(0, HEXE_TRUDE), mon(1, JELLYFISH)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[EndTurn, Attack(HEXE_TRUDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", JELLYFISH), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, HEXE_TRUDE), Some(2600 + 400), "{}", s.context());
}

/// Holding Arms (anime): "When this card attacks a monster your opponent controls: The attacked monster cannot attack for
/// 3 of your opponent's turns."
#[test]
fn holding_arms_anime_stops_the_monster_it_attacks() {
    let Some(mut s) = field(true, &[mon(0, HOLDING_ARMS), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    let err = s.run_both([&[EndTurn, Attack(HOLDING_ARMS), EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap_err();
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
}

/// Horned Saurus: "If this card is Fusion Summoned: You can place 1 Field Spell from your Deck or GY face-up in your Field
/// Zone." and "This card can attack directly the turn it is Special Summoned." Fusion Summoned with Polymerization from
/// Kaitoptera and a Dinosaur, in the opponent's first turn.
#[test]
fn horned_saurus_places_a_field_spell_and_attacks_directly() {
    let deck = deck_with(&[YAMI]);
    let spawns = [hand_card(1, POLYMERIZATION), hand_card(1, KAITOPTERA), hand_card(1, URABY), mon(0, WARWOLF)];
    let Some(mut s) = field_extra(false, [&[KURIBOH; 40], &deck], [&[], &[HORNED_SAURUS]], &spawns) else { return };
    s.run_both([&[], &[Activate(POLYMERIZATION), Chain(HORNED_SAURUS), Attack(HORNED_SAURUS)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", HORNED_SAURUS), 1, "{}", s.context());
    assert_eq!(s.count(1, "szone", YAMI), 1, "Yami in the Field Zone: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "a direct attack past Warwolf: {}", s.context());
}

/// Horned Saurus: "During your Main Phase, you can: Immediately after this effect resolves, Normal Summon 1 Dinosaur or
/// Dragon monster." Baby Dragon, with the turn's Normal Summon already spent on Kaitoptera.
#[test]
fn horned_saurus_normal_summons_a_dragon_in_addition() {
    let spawns = [hand_card(1, POLYMERIZATION), hand_card(1, KAITOPTERA), hand_card(1, URABY), hand_card(1, BABY_DRAGON)];
    let Some(mut s) = field_extra(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[HORNED_SAURUS]], &spawns) else { return };
    s.run_both([&[], &[Summon(KAITOPTERA), Activate(POLYMERIZATION), Activate(HORNED_SAURUS)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", HORNED_SAURUS), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", BABY_DRAGON), 1, "Baby Dragon was Normal Summoned: {}", s.context());
}

/// Goddess Verdande's Guidance: "When this card is activated: If all monsters you control are "Valkyrie" monsters (min.
/// 1), you can add 1 "Goddess Urd's Verdict" from your Deck to your hand."
#[test]
fn verdande_adds_urds_verdict_when_all_your_monsters_are_valkyries() {
    let deck = deck_with(&[VERDICT]);
    let Some(mut s) = field_decks(false, [&deck, &[KURIBOH; 40]], &[hand_card(0, VERDANDE), mon(0, VALKYRIE_ERSTE)]) else { return };
    s.run(&[Activate(VERDANDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", VERDICT), 1, "{}", s.context());
}

/// A Kuriboh deck whose top card (the first draw after the opening hand) is `card`.
fn deck_topped_by(card: u16) -> Vec<u16> {
    std::iter::repeat_n(KURIBOH, 34).chain([card]).chain(std::iter::repeat_n(KURIBOH, 5)).collect()
}

/// Goddess Verdande's Guidance: "Once per turn: You can declare 1 type of card (Monster, Spell, or Trap); reveal the top
/// card of your opponent's Deck and if it was the declared type, they Set it to their field. Otherwise, or if it cannot
/// be Set, they add it to their hand." Monster is declared (the first choice) and the top card is a monster.
#[test]
fn verdande_sets_the_monster_it_revealed_on_the_opponents_field() {
    let deck = deck_topped_by(WARWOLF);
    let Some(mut s) = field_decks(false, [&[KURIBOH; 40], &deck], &[spell(0, VERDANDE)]) else { return };
    s.run(&[Activate(VERDANDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "the top card was Set: {}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(8), "face-down Defense: {}", s.context());
}

/// Goddess Verdande's Guidance: when the revealed card is not the declared type, "they add it to their hand."
#[test]
fn verdande_gives_the_opponent_the_card_when_the_type_was_wrong() {
    let deck = deck_topped_by(DARK_HOLE);
    let Some(mut s) = field_decks(false, [&[KURIBOH; 40], &deck], &[spell(0, VERDANDE)]) else { return };
    let before = hand(&s, 1);
    s.run(&[Activate(VERDANDE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(hand(&s, 1), before + 1, "{}", s.context());
}

/// Goddess Urd's Verdict: "Your opponent cannot target "Valkyrie" monsters you control with card effects, also they cannot
/// be destroyed by your opponent's card effects." Dark Hole leaves the Valkyrie, Mystical Space Typhoon has nothing to hit.
#[test]
fn urds_verdict_protects_valkyries_from_the_opponents_effects() {
    let spawns = [spell(0, VERDICT), mon(0, VALKYRIE_ERSTE), mon(1, WARWOLF), hand_card(1, DARK_HOLE)];
    let Some(mut s) = field(false, &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", VALKYRIE_ERSTE), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole did destroy the Warwolf: {}", s.context());
}

/// Goddess Urd's Verdict: "Once per turn: You can declare 1 card name, then target 1 Set card your opponent controls;
/// reveal the Set card, and if it was the declared card, banish it. Otherwise, banish 1 card you control." The harness
/// can't choose the declared name (the CPU's pick is not Dark Hole), so this is the wrong guess: the Set card stays, and
/// the card banished is one of ours (Warwolf).
#[test]
fn urds_verdict_banishes_a_card_of_ours_after_a_wrong_guess() {
    let Some(mut s) = field(false, &[spell(0, VERDICT), set(1, DARK_HOLE), mon(0, WARWOLF)]) else { return };
    s.prefer(&[(1, DARK_HOLE), (0, WARWOLF)]);
    s.run(&[Activate(VERDICT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", DARK_HOLE), 1, "their Set card stays: {}", s.context());
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "ours is banished: {}", s.context());
}
