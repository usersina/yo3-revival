//! Scenario tests for the sixth group of the anime's printed cards (rows 4848-5205 of engine/added/cards.tsv): the Fortune
//! Fairies, the Fossil Fusion family, and the cards from "Full Moon Mirror" to "Goddess Skuld's Oracle". Each through the
//! real core with both seats' client models checked after every step (`duelcheck::scenario`). Texts: the printed ones
//! (anime cards off) from the pinned official database; an anime version is tested with anime cards on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The added cards of this group (client ids from the group list).
const CHEE: u16 = 14555; // Fortune Fairy Chee
const EN: u16 = 14551; // Fortune Fairy En
const HIKARI: u16 = 14550; // Fortune Fairy Hikari
const HU: u16 = 14552; // Fortune Fairy Hu
const SWEE: u16 = 14553; // Fortune Fairy Swee
const SKULLGAR: u16 = 15368; // Fossil Dragon Skullgar
const SKULLGIOS: u16 = 7263; // Fossil Dragon Skullgios
const FOSSIL_FUSION: u16 = 15369;
const SKULL_BUGGY: u16 = 15883; // Fossil Machine Skull Buggy
const SKULL_CONVOY: u16 = 15881; // Fossil Machine Skull Convoy
const SKULL_WAGON: u16 = 15882; // Fossil Machine Skull Wagon
const SKULL_BONE: u16 = 15367; // Fossil Warrior Skull Bone
const SKULL_KING: u16 = 7253; // Fossil Warrior Skull King
const SKULL_KNIGHT: u16 = 15366; // Fossil Warrior Skull Knight
const TIME_STREAM: u16 = 15370;
const WEATHERING_SOLDIER: u16 = 15365; // a monster that mentions "Fossil Fusion"
const FULL_MOON_MIRROR: u16 = 5158;
const GABRION: u16 = 13818; // Gabrion, the Timelord
const GATE_BLOCKER: u16 = 11179;
const GATLING_OGRE: u16 = 5156;
const GIANT_REX: u16 = 5806;
const GINGERBREAD_HOUSE: u16 = 14549;
const GLASS_SLIPPERS: u16 = 13809;
const GLIFE: u16 = 13806; // Glife the Phantom Bird
const GODDESS_BOW: u16 = 11890;
const SKULDS_ORACLE: u16 = 13999; // Goddess Skuld's Oracle
const PRINZESSIN: u16 = 13802;
const STROMBERG: u16 = 13808; // Golden Castle of Stromberg
const VERDANDE: u16 = 14000; // Goddess Verdande's Guidance
const VALKYRIE_DRITTE: u16 = 13992; // a "Valkyrie" monster
const CLAW_OF_HERMOS: u16 = 11892;
// The game's own.
const EASTER_ISLAND: u16 = 4521; // The Statue of Easter Island: a Level 4 Rock
const GROTTO: u16 = 4638; // Stone Ogre Grotto: a Level 5 Rock
const MOKEY: u16 = 6018; // Mokey Mokey: 300 ATK, 100 DEF
const LABYRINTH_WALL: u16 = 4372; // 0 ATK, 3000 DEF
const STONE_DRAGON: u16 = 4432; // a Level 7 Rock
const PETIT_ANGEL: u16 = 4214; // a Level 3 Fairy
const WICKED_AVATAR: u16 = 7091; // a Level 10 DARK monster
const YAMI: u16 = 4341; // a Field Spell: Spellcasters and Fiends gain 200 ATK
const RAIGEKI: u16 = 4343;
const CHANGE_OF_HEART: u16 = 4860;
const HAMMER_SHOT: u16 = 6130;
const QUEENS_KNIGHT: u16 = 5503; // a Warrior
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const IDOL: u16 = 4739; // Thousand-Eyes Idol, a Level 1 Spellcaster
const DARK_MAGICIAN: u16 = 4041; // a Spellcaster

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn start(anime: bool, main: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main[0], &[][..]), (main[1], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// A duel with Kuriboh decks and `spawns` in place; anime cards off.
fn printed(spawns: &[Spawn]) -> Option<Scenario> {
    start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], spawns)
}

/// A duel with Kuriboh Main Decks, `extra` as team 0's Extra Deck and `spawns` in place; anime cards off.
fn with_extra(extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    duel(8000, &[KURIBOH; 40], extra, spawns)
}

/// A duel at `lp` with team 0's Main and Extra Decks given, Kuriboh for team 1; anime cards off.
fn duel(lp: u32, main: &[u16], extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main, extra), (&[KURIBOH; 40][..], &[][..])];
    Some(Scenario::start_field(&dir, 7, lp, decks, spawns).unwrap())
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// A 40-card deck of Kuriboh whose first draw is `card`. Only for a team with no spawned cards: with them the start
/// shuffles the deck.
fn deck_drawing(card: u16) -> Vec<u16> {
    // Drawn from the end: the last five are the opening hand, the card before them the first draw.
    let mut deck = vec![KURIBOH; 34];
    deck.push(card);
    deck.extend([KURIBOH; 5]);
    deck
}

/// A deck for a team with spawned cards, whose order the start shuffles: `card` 30 times and `others` the rest, so the
/// team's first draw is `card` whatever the order (a card of the same name in the hand is no problem).
fn deck_of(card: u16, others: u16) -> Vec<u16> {
    let mut deck = vec![card; 30];
    deck.extend([others; 10]);
    deck
}

/// Team 1's first draw is `card`: team 0 passes its turn, and team 1 says yes to the card's draw trigger.
fn drawn_by_team1(card: u16, spawns: &[Spawn]) -> Option<Scenario> {
    drawn_by_team1_with(card, spawns, &[Chain(card)])
}

/// [`drawn_by_team1`] with team 1's own plan for its turn. Spawned cards of team 1 shuffle its deck: its tests use `deck_of`.
fn drawn_by_team1_with(card: u16, spawns: &[Spawn], plan: &[yo3_server::duelcheck::scenario::Action]) -> Option<Scenario> {
    let deck = if spawns.iter().any(|s| s.team == 1) { deck_of(card, KURIBOH) } else { deck_drawing(card) };
    let mut s = start(false, [&[KURIBOH; 40], &deck], spawns)?;
    s.run_both([&[EndTurn], plan]).unwrap();
    Some(s)
}

/// Fortune Fairy Chee: "When you draw this card: You can reveal it; Special Summon it from your hand."
#[test]
fn chee_is_special_summoned_when_drawn() {
    let Some(s) = drawn_by_team1(CHEE, &[]) else { return };
    clean(&s);
    assert_eq!(s.count(1, "mzone", CHEE), 1, "{}", s.context());
}

/// Chee: "If this card is Special Summoned from the hand: You can make each player draw 1 card."
#[test]
fn chee_makes_each_player_draw() {
    let Some(s) = drawn_by_team1_with(CHEE, &[], &[Chain(CHEE), Chain(CHEE)]) else { return };
    clean(&s);
    // Team 1: 5 in the opening hand, Chee drawn and Special Summoned, then 1 more; team 0: 5, its turn's draw, 1 more.
    assert_eq!(s.state()[1]["hand"].as_array().map(Vec::len), Some(6), "{}", s.context());
    assert_eq!(s.state()[0]["hand"].as_array().map(Vec::len), Some(7), "{}", s.context());
}

/// Fortune Fairy En: "If this card is Special Summoned from the hand: You can target 1 Set card your opponent controls;
/// destroy it."
#[test]
fn en_destroys_a_set_card() {
    let Some(s) = drawn_by_team1_with(EN, &[at(0, SpellSet, DARK_HOLE)], &[Chain(EN), Chain(EN)]) else { return };
    clean(&s);
    assert_eq!(s.count(1, "mzone", EN), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "the Set card was destroyed: {}", s.context());
    assert_eq!(s.count(0, "szone", DARK_HOLE), 0, "{}", s.context());
}

/// Fortune Fairy Hikari: "If this card is Special Summoned from the hand: You can target 1 monster you control; send it to
/// the GY, and if you do, Special Summon 1 Level 1 Spellcaster monster from your Deck."
#[test]
fn hikari_trades_a_monster_for_a_level_1_spellcaster() {
    let deck = deck_of(HIKARI, IDOL);
    // Hikari is itself a Level 1 Spellcaster, so the choice is told: the Idol.
    let Some(mut s) = start(false, [&[KURIBOH; 40], &deck], &[at(1, Monster, WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF), (1, IDOL)]);
    s.run_both([&[EndTurn], &[Chain(HIKARI), Chain(HIKARI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", IDOL), 1, "the Level 1 Spellcaster came from the Deck: {}", s.context());
    assert_eq!(s.count(1, "mzone", HIKARI), 1, "{}", s.context());
}

/// Fortune Fairy Hu: "If this card is Special Summoned from the hand: You can target 1 of your banished Spellcaster
/// monsters; add it to your hand."
#[test]
fn hu_adds_a_banished_spellcaster() {
    let Some(s) = drawn_by_team1_with(HU, &[at(1, Banished, DARK_MAGICIAN)], &[Chain(HU), Chain(HU)]) else { return };
    clean(&s);
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(1, "removed", DARK_MAGICIAN), 0, "{}", s.context());
}

/// Fortune Fairy Swee: "If this card is Special Summoned from the hand: You can target 1 other face-up monster on the
/// field; banish it until the Standby Phase of your next turn."
#[test]
fn swee_banishes_a_monster_until_the_next_standby_phase() {
    let Some(mut s) = drawn_by_team1_with(SWEE, &[at(0, Monster, WARWOLF)], &[Chain(SWEE), Chain(SWEE)]) else { return };
    clean(&s);
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
    // Through team 0's turn and into team 1's next one: it is back at that Standby Phase.
    s.run_both([&[EndTurn, EndTurn], &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "back at the Standby Phase of team 1's next turn: {}", s.context());
}

/// Fossil Fusion: "Fusion Summon 1 "Fossil" Fusion Monster from your Extra Deck, by banishing Fusion Materials mentioned
/// on it from any GYs." Skull Knight: 1 Rock monster + 1 Level 5 or 6 monster, both in the player's own Graveyard.
#[test]
fn fossil_fusion_summons_by_banishing_from_the_graveyard() {
    let spawns = [at(0, Hand, FOSSIL_FUSION), at(0, Grave, EASTER_ISLAND), at(0, Grave, GROTTO)];
    let Some(mut s) = with_extra(&[SKULL_KNIGHT], &spawns) else { return };
    s.run(&[Activate(FOSSIL_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SKULL_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", EASTER_ISLAND) + s.count(0, "removed", GROTTO), 2, "the materials are banished: {}", s.context());
    assert_eq!(s.count(0, "grave", FOSSIL_FUSION), 1, "{}", s.context());
}

/// Fossil Fusion: "If a face-up "Fossil" Fusion Monster(s) you control is destroyed by battle or card effect, while this
/// card is in your GY: You can add this card from the GY to your hand."
#[test]
fn fossil_fusion_returns_when_a_fossil_monster_is_destroyed() {
    let spawns = [at(0, Grave, FOSSIL_FUSION), at(0, Monster, SKULL_KNIGHT), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(FOSSIL_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SKULL_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", FOSSIL_FUSION), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", FOSSIL_FUSION), 0, "{}", s.context());
}

/// Fossil Warrior Skull Knight: "If this card attacks a Defense Position monster, inflict piercing battle damage."
#[test]
fn skull_knight_pierces() {
    let spawns = [at(0, Monster, SKULL_KNIGHT), at(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULL_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - (2400 - 100), "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Skull Knight: "When this attacking card destroys a monster by battle: You can activate this effect; this card can make
/// a second attack in a row."
#[test]
fn skull_knight_attacks_a_second_time_after_a_destruction() {
    let spawns = [at(0, Monster, SKULL_KNIGHT), at(1, MonsterDefense, WARWOLF), at(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULL_KNIGHT), Chain(SKULL_KNIGHT), Attack(SKULL_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 2, "both were destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * (2400 - 100), "{}", s.context());
}

/// Skull Knight: "You can banish this card from your GY, then target 1 monster on the field; destroy it."
#[test]
fn skull_knight_banishes_itself_to_destroy_a_monster() {
    let spawns = [at(0, Grave, SKULL_KNIGHT), at(1, Monster, WARWOLF)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(SKULL_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SKULL_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Fossil Warrior Skull King: "This card can make a second attack during each Battle Phase. If this card attacks a
/// Defense Position monster, inflict piercing battle damage."
#[test]
fn skull_king_attacks_twice_and_pierces() {
    let spawns = [at(0, Monster, SKULL_KING), at(1, MonsterDefense, WARWOLF), at(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULL_KING), Attack(SKULL_KING)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 2, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * (2800 - 100), "{}", s.context());
}

/// Skull King: "During your opponent's turn (Quick Effect): You can target 1 monster in your opponent's GY; discard 1
/// card, and if you do, Special Summon that monster to your field."
#[test]
fn skull_king_takes_a_monster_from_the_opponents_graveyard() {
    let spawns = [at(0, Monster, SKULL_KING), at(1, Grave, WARWOLF), at(1, Hand, KURIBOH)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn, Chain(SKULL_KING)], &[Summon(KURIBOH)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 0, "{}", s.context());
}

/// Fossil Dragon Skullgar: "If this card attacks a Defense Position monster, inflict piercing battle damage."
#[test]
fn skullgar_pierces() {
    let spawns = [at(0, Monster, SKULLGAR), at(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULLGAR)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - (2500 - 100), "{}", s.context());
}

/// Skullgar: "You can banish this card from your GY; add 1 "Fossil Fusion" from your Deck to your hand."
#[test]
fn skullgar_fetches_fossil_fusion() {
    let main: Vec<u16> = [FOSSIL_FUSION].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = duel(8000, &main, &[], &[at(0, Grave, SKULLGAR)]) else { return };
    s.run(&[Activate(SKULLGAR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SKULLGAR), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", FOSSIL_FUSION), 1, "{}", s.context());
}

/// Fossil Dragon Skullgios: "1 Rock monster + 1 Level 7 or higher monster in your opponent's GY". Fusion Summoned with
/// Fossil Fusion (the Rock from its own, the Level 8 from the opponent's Graveyard), "if this Fusion Summoned card battles
/// an opponent's monster, any battle damage it inflicts to your opponent is doubled". Its switch effect also fires at
/// Warwolf (ATK 2000, DEF 100), which fights with 100 ATK: (3500 - 100) twice.
#[test]
fn skullgios_doubles_its_battle_damage() {
    let spawns = [at(0, Hand, FOSSIL_FUSION), at(0, Grave, EASTER_ISLAND), at(1, Grave, STONE_DRAGON), at(1, Monster, WARWOLF)];
    let Some(mut s) = with_extra(&[SKULLGIOS], &spawns) else { return };
    s.run(&[Activate(FOSSIL_FUSION), EndTurn, Attack(SKULLGIOS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", STONE_DRAGON), 1, "the opponent's Graveyard gave the Level 7 or higher: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * (3500 - 100), "{}", s.context());
}

/// Skullgios: "Before damage calculation, if this card battles an opponent's monster: You can switch the current ATK and
/// DEF of that opponent's monster until the end of that Damage Step." Labyrinth Wall (0 ATK, 3000 DEF) fights as a 3000
/// ATK monster: 500 damage (not Fusion Summoned here, so not doubled).
#[test]
fn skullgios_switches_the_attack_and_defense_of_its_opponent() {
    let spawns = [at(0, Monster, SKULLGIOS), at(1, Monster, LABYRINTH_WALL)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULLGIOS), Chain(SKULLGIOS)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - (3500 - 3000), "{}", s.context());
}

/// Skullgios: "If this card attacks a Defense Position monster, inflict piercing battle damage." Its switch effect makes
/// Warwolf's DEF 2000: 1500 goes through.
#[test]
fn skullgios_pierces() {
    let spawns = [at(0, Monster, SKULLGIOS), at(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULLGIOS)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - (3500 - 2000), "{}", s.context());
}

/// Fossil Machine Skull Buggy: "When this card destroys an opponent's monster by battle: You can inflict 600 damage to
/// your opponent." 1400 against Mokey Mokey's 300 is 1100, and 600 more.
#[test]
fn skull_buggy_burns_for_600() {
    let spawns = [at(0, Monster, SKULL_BUGGY), at(1, Monster, MOKEY)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULL_BUGGY), Chain(SKULL_BUGGY)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1100 - 600, "{}", s.context());
}

/// Skull Buggy: "You can banish this card from your GY; add 1 monster from your Deck to your hand that mentions "Fossil
/// Fusion"." (Weathering Soldier is in the Deck.)
#[test]
fn skull_buggy_fetches_a_monster_that_mentions_fossil_fusion() {
    let main: Vec<u16> = [WEATHERING_SOLDIER, KURIBOH].into_iter().chain([KURIBOH; 38]).collect();
    let Some(mut s) = duel(8000, &main, &[], &[at(0, Grave, SKULL_BUGGY)]) else { return };
    s.run(&[Activate(SKULL_BUGGY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SKULL_BUGGY), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", WEATHERING_SOLDIER), 1, "{}", s.context());
}

/// Fossil Machine Skull Convoy: "While you control this Fusion Summoned card, all monsters your opponent controls lose ATK
/// equal to their original DEF." Warwolf (2000 ATK, 100 DEF) is at 1900.
#[test]
fn skull_convoy_weakens_the_opponents_monsters() {
    let spawns = [at(0, Hand, FOSSIL_FUSION), at(0, Grave, EASTER_ISLAND), at(0, Grave, STONE_DRAGON), at(1, Monster, WARWOLF)];
    let Some(mut s) = with_extra(&[SKULL_CONVOY], &spawns) else { return };
    s.run(&[Activate(FOSSIL_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SKULL_CONVOY), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(1900), "{}", s.context());
}

/// Skull Convoy: "This card can make up to 3 attacks on monsters during each Battle Phase. When this card destroys an
/// opponent's monster by battle: You can inflict 1000 damage to your opponent." Three Mokey Mokey (200 ATK under it):
/// 1900 + 1000 each time.
#[test]
fn skull_convoy_attacks_three_times_for_1000_each() {
    let spawns = [at(0, Hand, FOSSIL_FUSION), at(0, Grave, EASTER_ISLAND), at(0, Grave, STONE_DRAGON), at(1, Monster, MOKEY),
        at(1, Monster, MOKEY), at(1, Monster, MOKEY)];
    let Some(mut s) = duel(30000, &[KURIBOH; 40], &[SKULL_CONVOY], &spawns) else { return };
    let plan = [Activate(FOSSIL_FUSION), EndTurn, Attack(SKULL_CONVOY), Chain(SKULL_CONVOY), Attack(SKULL_CONVOY),
        Chain(SKULL_CONVOY), Attack(SKULL_CONVOY), Chain(SKULL_CONVOY)];
    s.run(&plan).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MOKEY), 3, "{}", s.context());
    assert_eq!(s.lp(1), 30000 - 3 * (1900 + 1000), "{}", s.context());
}

/// Fossil Machine Skull Wagon: "This card can make up to 2 attacks on monsters during each Battle Phase. When this card
/// destroys an opponent's monster by battle: You can inflict 800 damage to your opponent." Two Mokey Mokey: 1400 + 800 each.
#[test]
fn skull_wagon_attacks_twice_for_800_each() {
    let spawns = [at(0, Monster, SKULL_WAGON), at(1, Monster, MOKEY), at(1, Monster, MOKEY)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULL_WAGON), Chain(SKULL_WAGON), Attack(SKULL_WAGON), Chain(SKULL_WAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MOKEY), 2, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * (1400 + 800), "{}", s.context());
}

/// Skull Wagon: "You can banish this card from your GY, then target 1 Spell/Trap on the field; destroy it."
#[test]
fn skull_wagon_banishes_itself_to_destroy_a_spell_or_trap() {
    let spawns = [at(0, Grave, SKULL_WAGON), at(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[Activate(SKULL_WAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SKULL_WAGON), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// Fossil Warrior Skull Bone: "This card can make up to 2 attacks on monsters during each Battle Phase."
#[test]
fn skull_bone_attacks_twice() {
    let spawns = [at(0, Monster, SKULL_BONE), at(1, Monster, MOKEY), at(1, Monster, MOKEY)];
    let Some(mut s) = with_extra(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(SKULL_BONE), Attack(SKULL_BONE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MOKEY), 2, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * (2000 - 300), "{}", s.context());
}

/// Skull Bone: "You can banish this card from your GY; add 1 "Time Stream" from your Deck to your hand."
#[test]
fn skull_bone_fetches_time_stream() {
    let main: Vec<u16> = [TIME_STREAM].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = duel(8000, &main, &[], &[at(0, Grave, SKULL_BONE)]) else { return };
    s.run(&[Activate(SKULL_BONE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SKULL_BONE), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", TIME_STREAM), 1, "{}", s.context());
}

/// A duel with Warwolf Main Decks for team 0 (no Fiend or Fairy in its hand), `later` at the bottom of that deck, and
/// `spawns` in place; anime cards off.
fn warwolves(later: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let main: Vec<u16> = later.iter().copied().chain(std::iter::repeat_n(WARWOLF, 40 - later.len())).collect();
    duel(8000, &main, &[], spawns)
}

/// Full Moon Mirror: "Each time a monster(s) is destroyed by battle or card effect, place 1 Full Moon Counter on this card
/// for each. You can remove Full Moon Counters from this card, then activate 1 of these effects; 1: Special Summon 1
/// Level 6 or lower monster (Fiend or Fairy) from your hand or GY." One destroyed monster: the 1 effect.
#[test]
fn full_moon_mirror_one_counter_summons_a_fairy() {
    let spawns = [at(0, Hand, FULL_MOON_MIRROR), at(0, Hand, DARK_HOLE), at(0, Hand, PETIT_ANGEL), at(1, Monster, WARWOLF)];
    let Some(mut s) = warwolves(&[], &spawns) else { return };
    s.run(&[Activate(FULL_MOON_MIRROR), Activate(DARK_HOLE), Activate(FULL_MOON_MIRROR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", PETIT_ANGEL), 1, "the Fairy came from the hand: {}", s.context());
}

/// Full Moon Mirror: "3: Add 1 Level 10 DARK monster from your Deck to your hand." Three destroyed monsters; The Wicked
/// Avatar is in the Deck.
#[test]
fn full_moon_mirror_three_counters_fetch_a_level_10_dark_monster() {
    let spawns = [at(0, Hand, FULL_MOON_MIRROR), at(0, Hand, DARK_HOLE), at(1, Monster, WARWOLF), at(1, Monster, WARWOLF),
        at(1, Monster, WARWOLF)];
    let Some(mut s) = warwolves(&[WICKED_AVATAR], &spawns) else { return };
    s.run(&[Activate(FULL_MOON_MIRROR), Activate(DARK_HOLE), Activate(FULL_MOON_MIRROR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", WICKED_AVATAR), 1, "{}", s.context());
}

/// Full Moon Mirror: "5: Special Summon 1 Level 10 monster from your hand or GY." Five destroyed monsters; Gabrion, the
/// Timelord (Level 10) is in the hand.
#[test]
fn full_moon_mirror_five_counters_summon_a_level_10_monster() {
    let mut spawns = vec![at(0, Hand, FULL_MOON_MIRROR), at(0, Hand, DARK_HOLE), at(0, Hand, GABRION)];
    spawns.extend([at(1, Monster, WARWOLF); 5]);
    let Some(mut s) = warwolves(&[], &spawns) else { return };
    s.run(&[Activate(FULL_MOON_MIRROR), Activate(DARK_HOLE), Activate(FULL_MOON_MIRROR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 5, "{}", s.context());
    assert_eq!(s.count(0, "mzone", GABRION), 1, "{}", s.context());
}

/// Gabrion, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing."
#[test]
fn gabrion_is_summoned_without_a_tribute() {
    let Some(mut s) = printed(&[at(0, Hand, GABRION)]) else { return };
    s.run(&[Summon(GABRION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GABRION), 1, "{}", s.context());
}

/// Gabrion: "Cannot be destroyed by battle or card effects." Dark Hole destroys the opponent's Warwolf and leaves Gabrion.
#[test]
fn gabrion_is_not_destroyed() {
    let spawns = [at(0, Hand, GABRION), at(0, Hand, DARK_HOLE), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(GABRION), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GABRION), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", GABRION), 0, "{}", s.context());
}

/// Gabrion: "You take no battle damage from attacks involving this card. At the end of the Battle Phase, if this card
/// battled: Shuffle as many cards your opponent controls as possible into the Deck(s), then they draw cards equal to the
/// number shuffled into their Main Deck." Team 1 Summons Gabrion (0 ATK) and attacks Warwolf; Warwolf and the Set card are
/// shuffled away and team 0 draws 2.
#[test]
fn gabrion_shuffles_the_opponents_cards_after_battle() {
    let spawns = [at(1, Hand, GABRION), at(0, Monster, WARWOLF), at(0, SpellSet, DARK_HOLE)];
    let Some(mut s) = printed(&spawns) else { return };
    let before = hand(&s, 0);
    s.run_both([&[EndTurn], &[Summon(GABRION), Attack(GABRION)]]).unwrap();
    // The end of the Battle Phase is reached by leaving it.
    pass_until(&mut s, |s| s.count(0, "mzone", WARWOLF) == 0);
    clean(&s);
    assert_eq!(s.lp(1), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(1, "mzone", GABRION), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(0, "szone", DARK_HOLE), 0, "{}", s.context());
    assert_eq!(hand(&s, 0), before + 2, "the opponent drew the 2 shuffled cards: {}", s.context());
}

/// Passes (leaves the phase, declines every chain) until `done`, at most 20 times: for an effect at a phase's end.
fn pass_until(s: &mut Scenario, done: impl Fn(&Scenario) -> bool) {
    for _ in 0..20 {
        if done(s) {
            return;
        }
        s.pass().unwrap();
    }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Gabrion: "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn gabrion_returns_to_the_deck_at_the_standby_phase() {
    let Some(mut s) = printed(&[at(0, Hand, GABRION)]) else { return };
    s.run(&[Summon(GABRION), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GABRION), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", GABRION), 0, "it was shuffled, not destroyed: {}", s.context());
}

/// Gabrion (anime): "You can reduce the battle damage any player takes from an attack involving this Attack Position
/// card to 0. At the end of the Damage Step, if this card battled: Shuffle all cards your opponent controls into the
/// Deck. You cannot Summon monsters." No draw for the opponent here.
#[test]
fn anime_gabrion_shuffles_the_opponents_cards_at_the_end_of_the_damage_step() {
    // The anime Gabrion is Tribute Summoned (two Warwolves of team 1's own).
    let spawns = [at(1, Hand, GABRION), at(1, Monster, WARWOLF), at(1, Monster, WARWOLF), at(0, Monster, WARWOLF),
        at(0, SpellSet, DARK_HOLE)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    let before = hand(&s, 0);
    s.run_both([&[EndTurn], &[Summon(GABRION), Attack(GABRION)]]).unwrap();
    pass_until(&mut s, |s| s.count(0, "mzone", WARWOLF) == 0);
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(0, "szone", DARK_HOLE), 0, "{}", s.context());
    assert_eq!(hand(&s, 0), before, "the cards went back to the Deck without a draw: {}", s.context());
    assert_eq!(s.count(1, "mzone", GABRION), 1, "{}", s.context());
}

/// Gate Blocker: "Negate the effects of your opponent's Field Spells." Yami, activated by the opponent, gives Dark Magician
/// 200 ATK, unless Gate Blocker negates it.
#[test]
fn gate_blocker_negates_an_opponents_field_spell() {
    let plans: [&[yo3_server::duelcheck::scenario::Action]; 2] = [&[EndTurn], &[Activate(YAMI)]];
    let Some(mut s) = printed(&[at(1, Hand, YAMI), at(1, Monster, DARK_MAGICIAN)]) else { return };
    s.run_both(plans).unwrap();
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(2700), "Yami works without Gate Blocker: {}", s.context());
    let spawns = [at(0, Monster, GATE_BLOCKER), at(1, Hand, YAMI), at(1, Monster, DARK_MAGICIAN)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both(plans).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(2500), "Yami is negated: {}", s.context());
}

/// Gate Blocker: "Your opponent cannot target other monsters you control with card effects." Change of Heart takes
/// Gate Blocker (it can be targeted) and not Warwolf.
#[test]
fn gate_blocker_protects_the_other_monsters() {
    let spawns = [at(0, Monster, GATE_BLOCKER), at(0, Monster, WARWOLF), at(1, Hand, CHANGE_OF_HEART)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[EndTurn], &[Activate(CHANGE_OF_HEART)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf stays: {}", s.context());
    assert_eq!(s.count(1, "mzone", GATE_BLOCKER), 1, "Gate Blocker was the target: {}", s.context());
}

/// Gate Blocker: "Your opponent cannot place counters on cards on the field." Raigeki destroys the monster of the
/// opponent's Full Moon Mirror's owner: the Mirror takes a counter and its effect is on offer, unless Gate Blocker is there.
#[test]
fn gate_blocker_stops_the_opponents_counters() {
    let plans: [&[yo3_server::duelcheck::scenario::Action]; 2] = [&[Activate(RAIGEKI), EndTurn], &[Activate(FULL_MOON_MIRROR)]];
    let spawns = [at(0, Hand, RAIGEKI), at(1, Spell, FULL_MOON_MIRROR), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both(plans).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", KURIBOH), 1, "with no Gate Blocker the Mirror's 1 effect works: {}", s.context());
    let spawns = [at(0, Hand, RAIGEKI), at(0, Monster, GATE_BLOCKER), at(1, Spell, FULL_MOON_MIRROR), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    let result = s.run_both(plans);
    assert!(result.is_err(), "the Mirror has no counter to remove: {}", s.context());
    assert_eq!(s.count(1, "mzone", KURIBOH), 0, "{}", s.context());
}

/// Gatling Ogre: "You can send any number of face-down cards from your Spell & Trap Zone to the GY; inflict 800 damage to
/// your opponent for each, also your opponent takes no damage from non-Fiend monsters' effects for the rest of this turn."
/// Two Set cards go and 1600 is dealt.
#[test]
fn gatling_ogre_burns_800_for_each_set_card() {
    let spawns = [at(0, Monster, GATLING_OGRE), at(0, SpellSet, DARK_HOLE), at(0, SpellSet, DARK_HOLE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(GATLING_OGRE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800 * s.count(0, "grave", DARK_HOLE) as i64, "{}", s.context());
    assert!(s.count(0, "grave", DARK_HOLE) >= 1, "{}", s.context());
}

/// Gatling Ogre: "...also your opponent takes no damage from non-Fiend monsters' effects for the rest of this turn."
/// Skull Buggy (a Rock) destroys Mokey Mokey: the 1100 of battle damage is dealt, its 600 effect damage is not.
#[test]
fn gatling_ogre_shields_the_opponent_from_non_fiend_effect_damage() {
    let spawns = [at(0, Monster, GATLING_OGRE), at(0, SpellSet, DARK_HOLE), at(0, Monster, SKULL_BUGGY), at(1, Monster, MOKEY)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[EndTurn, Activate(GATLING_OGRE), Attack(SKULL_BUGGY), Chain(SKULL_BUGGY)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800 - 1100, "{}", s.context());
    assert_eq!(s.count(1, "grave", MOKEY), 1, "{}", s.context());
}

/// Gatling Ogre: "Once per turn, when this card is targeted for an attack: You can inflict 500 damage to your opponent for
/// each Attack Position monster they control, then end the Battle Phase." Two Warwolves attack Ogre: 1000 damage, and the
/// attack does not go through.
#[test]
fn gatling_ogre_punishes_an_attack_on_it() {
    let spawns = [at(0, Monster, GATLING_OGRE), at(1, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[Chain(GATLING_OGRE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the Battle Phase ended before the damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", GATLING_OGRE), 1, "{}", s.context());
}

/// Giant Rex: "Cannot attack directly." (The opponent has no monster; with one, it attacks.)
#[test]
fn giant_rex_cannot_attack_directly() {
    let Some(mut s) = printed(&[at(0, Monster, GIANT_REX)]) else { return };
    assert!(s.run(&[EndTurn, Attack(GIANT_REX)]).is_err(), "{}", s.context());
    let Some(mut s) = printed(&[at(0, Monster, GIANT_REX), at(1, Monster, MOKEY)]) else { return };
    s.run(&[EndTurn, Attack(GIANT_REX)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MOKEY), 1, "{}", s.context());
}

/// Giant Rex: "If this card is banished: You can Special Summon this card, and if you do, this card gains 200 ATK for each
/// of your banished Dinosaur monsters." Fortune Fairy Swee banishes it; two other Rex are banished: 2000 + 400.
#[test]
fn giant_rex_returns_from_the_banishment() {
    let spawns = [at(0, Monster, GIANT_REX), at(0, Banished, GIANT_REX), at(0, Banished, GIANT_REX)];
    let deck = deck_drawing(SWEE);
    let dir = engine_dir();
    let Some(dir) = dir else { return };
    let mut s = Scenario::start_field(&dir, 7, 8000, [(&[KURIBOH; 40], &[]), (&deck, &[])], &spawns).unwrap();
    s.prefer(&[(0, GIANT_REX)]);
    s.run_both([&[EndTurn, Chain(GIANT_REX)], &[Chain(SWEE), Chain(SWEE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GIANT_REX), 1, "it came back: {}", s.context());
    assert_eq!(s.attack_of(0, GIANT_REX), Some(2400), "{}", s.context());
}

/// Giant Rex (anime): "While your opponent controls no monsters, this card cannot attack."
#[test]
fn anime_giant_rex_cannot_attack_an_empty_field() {
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &[at(0, Monster, GIANT_REX)]) else { return };
    assert!(s.run(&[EndTurn, Attack(GIANT_REX)]).is_err(), "{}", s.context());
    let spawns = [at(0, Monster, GIANT_REX), at(1, Monster, MOKEY)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[EndTurn, Attack(GIANT_REX)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MOKEY), 1, "{}", s.context());
}

/// Gingerbread House: "During your opponent's Standby Phase: You can have as many monsters they control as possible gain
/// 600 ATK, then destroy as many monsters they control as possible with 2500 or more ATK, and if you do, gain LP equal to
/// the number of monsters destroyed x 500. (The ATK gain remains even if this card leaves the field.)" Warwolf (2600) is
/// destroyed and Mokey Mokey stays at 900.
#[test]
fn gingerbread_house_destroys_the_strong_and_heals() {
    let spawns = [at(0, Hand, GINGERBREAD_HOUSE), at(1, Monster, WARWOLF), at(1, Monster, MOKEY)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(GINGERBREAD_HOUSE), EndTurn, Chain(GINGERBREAD_HOUSE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, MOKEY), Some(900), "{}", s.context());
    assert_eq!(s.lp(0), 8500, "{}", s.context());
}

/// Gingerbread House (anime): "Target 1 monster your opponent controls; it gains 600 ATK. If a monster's ATK becomes 2500
/// or higher by this effect, destroy that monster and gain 500 LP." The one targeted: Warwolf, not Mokey Mokey.
#[test]
fn anime_gingerbread_house_targets_one_monster() {
    let spawns = [at(0, Hand, GINGERBREAD_HOUSE), at(1, Monster, MOKEY), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(GINGERBREAD_HOUSE), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, MOKEY), Some(300), "{}", s.context());
    assert_eq!(s.lp(0), 8500, "{}", s.context());
}

/// Glass Slippers: "If this card is equipped to a Fairy monster, it gains 1000 ATK. If this card is equipped to a non-Fairy
/// monster, it cannot attack, also it loses 1000 ATK."
#[test]
fn glass_slippers_help_a_fairy_and_hinder_anyone_else() {
    let spawns = [at(0, Hand, GLASS_SLIPPERS), at(0, Monster, PETIT_ANGEL)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(GLASS_SLIPPERS)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, PETIT_ANGEL), Some(1600), "600 and 1000: {}", s.context());
    let spawns = [at(0, Hand, GLASS_SLIPPERS), at(0, Monster, WARWOLF), at(1, Monster, MOKEY)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(GLASS_SLIPPERS)]).unwrap();
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "{}", s.context());
    assert!(s.run(&[EndTurn, Attack(WARWOLF)]).is_err(), "it cannot attack: {}", s.context());
}

/// Glass Slippers: "If this card is sent to the GY because the equipped monster is destroyed: You can target 1 "Prinzessin"
/// you control; equip this card to that target." Hammer Shot destroys the equipped Warwolf (the Prinzessin has less ATK).
#[test]
fn glass_slippers_move_to_a_prinzessin() {
    let spawns = [at(0, Hand, GLASS_SLIPPERS), at(0, Monster, WARWOLF), at(0, Monster, PRINZESSIN), at(0, Hand, HAMMER_SHOT)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(GLASS_SLIPPERS), Activate(HAMMER_SHOT), Chain(GLASS_SLIPPERS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", GLASS_SLIPPERS), 1, "it is equipped again: {}", s.context());
    assert_eq!(s.attack_of(0, PRINZESSIN), Some(1300), "a Fairy gains 1000: {}", s.context());
}

/// Glife the Phantom Bird: "You can discard this card; add 1 "Golden Castle of Stromberg" from your Deck to your hand."
#[test]
fn glife_discards_to_fetch_stromberg() {
    let main: Vec<u16> = [STROMBERG].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = duel(8000, &main, &[], &[at(0, Hand, GLIFE)]) else { return };
    s.run(&[Activate(GLIFE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GLIFE), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", STROMBERG), 1, "{}", s.context());
}

/// Glife: "If this card is Normal or Special Summoned: You can target 1 card in your opponent's Spell & Trap Zone; destroy it."
#[test]
fn glife_destroys_a_spell_or_trap_when_summoned() {
    let spawns = [at(0, Hand, GLIFE), at(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(GLIFE), Chain(GLIFE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// Glife the Phantom Bird (anime): "When this card is Summoned: You can target 1 Spell/Trap your opponent controls; destroy it."
#[test]
fn anime_glife_destroys_a_spell_or_trap_when_summoned() {
    let spawns = [at(0, Hand, GLIFE), at(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Summon(GLIFE), Chain(GLIFE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// Goddess Bow: "Must be Special Summoned with "The Claw of Hermos", using a Warrior monster. If this card is Special
/// Summoned: Target 1 other face-up monster on the field; equip this card to it." Queen's Knight is sent, and the Bow
/// equips Warwolf.
#[test]
fn goddess_bow_is_summoned_by_the_claw_and_equips() {
    let spawns = [at(0, Hand, CLAW_OF_HERMOS), at(0, Monster, QUEENS_KNIGHT), at(0, Monster, WARWOLF)];
    let Some(mut s) = with_extra(&[GODDESS_BOW], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(CLAW_OF_HERMOS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", QUEENS_KNIGHT), 1, "the Warrior was sent: {}", s.context());
    assert_eq!(s.count(0, "szone", GODDESS_BOW), 1, "the Bow is an equip: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Goddess Skuld's Oracle: "When this card is activated: If all monsters you control are "Valkyrie" monsters (min. 1), you
/// can add 1 "Goddess Verdande's Guidance" from your Deck to your hand."
#[test]
fn skulds_oracle_adds_verdandes_guidance() {
    let main: Vec<u16> = [VERDANDE].into_iter().chain([KURIBOH; 39]).collect();
    let spawns = [at(0, Hand, SKULDS_ORACLE), at(0, Monster, VALKYRIE_DRITTE)];
    let Some(mut s) = duel(8000, &main, &[], &spawns) else { return };
    s.run(&[Activate(SKULDS_ORACLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", VERDANDE), 1, "{}", s.context());
}

/// Skuld's Oracle: "Once per turn, during your Main Phase: You can look at the top 3 cards of your opponent's Deck, and if
/// you do, place them on top of their Deck in any order, but you cannot Normal or Special Summon monsters for the rest of
/// this turn, except Fairy monsters."
#[test]
fn skulds_oracle_forbids_summoning_non_fairies() {
    let spawns = [at(0, Hand, SKULDS_ORACLE), at(0, Hand, WARWOLF), at(0, Hand, PETIT_ANGEL)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(SKULDS_ORACLE), Activate(SKULDS_ORACLE)]).unwrap();
    clean(&s);
    let summons: Vec<i64> = s.prompt()["summons"].as_array().unwrap().iter().filter_map(|c| c["cid"].as_i64()).collect();
    assert!(summons.contains(&(PETIT_ANGEL as i64)), "a Fairy can still be Summoned: {}", s.context());
    assert!(!summons.contains(&(WARWOLF as i64)), "Warwolf cannot: {}", s.context());
}

/// Skuld's Oracle (anime): "Once per turn, during your Main Phase: You can look at the top 3 cards of your opponent's Deck,
/// and if you do, place them on top of their Deck in any order." No Summon restriction.
#[test]
fn anime_skulds_oracle_does_not_forbid_summoning() {
    let spawns = [at(0, Hand, SKULDS_ORACLE), at(0, Hand, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(SKULDS_ORACLE), Activate(SKULDS_ORACLE), Summon(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}
