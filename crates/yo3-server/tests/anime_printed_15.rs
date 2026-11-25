//! Printed anime cards, group 15 (rows 4871-5160 of `engine/added/cards.tsv`): Toy Soldier, Trap Hole of Spikes, Triangle O,
//! Tribute Burial, Tyrant Burst Dragon, Tyrant Wing, Unacceptable Result, Unknown Synchron, the Valkyries, Valkyrie's
//! Embrace, Water of Life, Weathering Soldier, White Knight Lord, White Veil, Wing Requital, Wiretap, Yowie, Zaphion and
//! Zushin. Each effect is played through the real core with both seats' client models checked after every step
//! (`duelcheck::scenario`): the printed effect with anime cards off, and one test of the anime version with them on where
//! it differs.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// Added cards of this group (client ids from engine/added-cards.json).
const TOY_SOLDIER: u16 = 6791;
const TRAP_HOLE_OF_SPIKES: u16 = 12240;
const TRIANGLE_O: u16 = 4997;
const TRIBUTE_BURIAL: u16 = 12571;
const TYRANT_BURST_DRAGON: u16 = 11883;
const TYRANT_WING: u16 = 11886;
const UNACCEPTABLE_RESULT: u16 = 14556;
const UNKNOWN_SYNCHRON: u16 = 9473;
const BRUNHILDE: u16 = 13995; // Valkyrie Brunhilde
const DRITTE: u16 = 13992; // Valkyrie Dritte
const ERSTE: u16 = 13994; // Valkyrie Erste
const ZWEITE: u16 = 13993; // Valkyrie Zweite
const EMBRACE: u16 = 14539; // Valkyrie's Embrace
const WATER_OF_LIFE: u16 = 14547;
const WEATHERING_SOLDIER: u16 = 15365;
const WHITE_KNIGHT_LORD: u16 = 6829;
const WHITE_VEIL: u16 = 12905;
const WHITE_NIGHT_DRAGON: u16 = 7343; // Level 8 Dragon
const WING_REQUITAL: u16 = 4661;
const WIRETAP: u16 = 11180;
const YOWIE: u16 = 4000;
const ZAPHION: u16 = 13158;
const ZUSHIN: u16 = 12541;
// Other added cards they play with.
const CRYSTAL_SKULL: u16 = 4793;
const ASHOKA_PILLAR: u16 = 4995;
const CABRERA_STONE: u16 = 4996;
const FANG_OF_CRITIAS: u16 = 11885;
const RIDE_OF_THE_VALKYRIES: u16 = 13997;
const MISCHIEF: u16 = 13998; // Mischief of the Time Goddess
const FORTUNE_FAIRY_HU: u16 = 14552;
const FOSSIL_FUSION: u16 = 15369;
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const DARK_MAGICIAN: u16 = 4041; // a Spellcaster
const BLUE_EYES: u16 = 4007;
const LUSTER_DRAGON: u16 = 5644;
const HARPIE_LADY: u16 = 4068; // Winged Beast
const SONIC_BIRD: u16 = 4929; // Winged Beast
const CALL_OF_THE_HAUNTED: u16 = 4989; // a Continuous Trap
const SKULL_SERVANT: u16 = 4030; // a Level 1 Normal Monster
const THE_A_FORCES: u16 = 5327; // a Continuous Spell

const PLAIN: [u16; 40] = [KURIBOH; 40];

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn field(anime: bool, lp: u32, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(
        if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }
            .unwrap(),
    )
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn pile(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap_or(-1)
}

fn count_all(s: &Scenario, team: usize, place: &str) -> usize {
    s.state()[team][place].as_array().map_or(0, |cards| cards.iter().filter(|c| !c.is_null()).count())
}

fn listed(s: &Scenario, list: &str, cid: u16) -> bool {
    s.prompt()[list].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
}

fn can_activate(s: &Scenario, cid: u16) -> bool {
    listed(s, "activates", cid)
}

/// A deck of Kuriboh with these cards in it many times.
fn deck_with(cards: &[u16], copies: usize) -> Vec<u16> {
    let mut d: Vec<u16> = cards.iter().flat_map(|&c| std::iter::repeat_n(c, copies)).collect();
    d.resize(40, KURIBOH);
    d
}

// ---- Toy Soldier ----

/// Toy Soldier: "You can Set this card from your hand to your Spell & Trap Zone as a Spell." It sits in the Spell & Trap Zone.
#[test]
fn toy_soldier_sets_itself_as_a_spell() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, TOY_SOLDIER)]) else { return };
    s.run(&[SetSpell(TOY_SOLDIER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", TOY_SOLDIER), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TOY_SOLDIER), 0, "{}", s.context());
}

/// "If this Set card in the Spell & Trap Zone is sent to the GY: You can Special Summon this card." The opponent's
/// Mystical Space Typhoon destroys it and it comes out as a monster instead of staying in the Graveyard.
#[test]
fn toy_soldier_special_summons_itself_when_the_set_card_is_sent_to_the_grave() {
    let spawns = [at(0, Hand, TOY_SOLDIER), at(1, Hand, MST)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, TOY_SOLDIER)]);
    s.run_both([&[SetSpell(TOY_SOLDIER), EndTurn, Chain(TOY_SOLDIER)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TOY_SOLDIER), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", TOY_SOLDIER), 0, "{}", s.context());
    assert_eq!(s.count(1, "grave", MST), 1, "{}", s.context());
}

// ---- Trap Hole of Spikes ----

/// Trap Hole of Spikes: "When an opponent's monster that was Normal or Special Summoned this turn declares an attack:
/// Destroy that attacking monster, and if you do, inflict damage to your opponent equal to half the original ATK of that
/// monster." Warwolf (2000) is Normal Summoned and attacks: it is destroyed and its controller takes 1000.
#[test]
fn trap_hole_of_spikes_destroys_a_summoned_attacker_and_burns_half_its_attack() {
    let spawns = [at(0, SpellSet, TRAP_HOLE_OF_SPIKES), at(1, Hand, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(TRAP_HOLE_OF_SPIKES)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// "...that was Normal or Special Summoned this turn": a Warwolf already on the field is not a target, the trap cannot
/// be chained and the attack hits.
#[test]
fn trap_hole_of_spikes_ignores_a_monster_not_summoned_this_turn() {
    let spawns = [at(0, SpellSet, TRAP_HOLE_OF_SPIKES), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    // The trap's chain never comes up, so the planned Chain is left over and the run reports it: the attack has happened.
    let _ = s.run_both([&[EndTurn, ChainOnAttack(TRAP_HOLE_OF_SPIKES)], &[Attack(WARWOLF)]]);
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "the direct attack went through: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Trap Hole of Spikes, anime version: "When an opponent's monster declares an attack: Destroy that attacking monster, and
/// if you do, inflict damage to your opponent equal to 1/4 the destroyed monster's ATK". Any attacker, a quarter of 2000.
#[test]
fn trap_hole_of_spikes_anime_burns_a_quarter_of_any_attacker() {
    let spawns = [at(0, SpellSet, TRAP_HOLE_OF_SPIKES), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(TRAP_HOLE_OF_SPIKES)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7500, "{}", s.context());
}

// ---- Triangle O ----

fn triangle_board() -> Vec<Spawn> {
    vec![
        at(0, Hand, TRIANGLE_O),
        at(0, Monster, CRYSTAL_SKULL),
        at(0, Monster, ASHOKA_PILLAR),
        at(0, Monster, CABRERA_STONE),
        at(1, Monster, WARWOLF),
        at(1, SpellSet, CALL_OF_THE_HAUNTED),
    ]
}

/// Triangle O: "If you control "Crystal Skull", "Ashoka Pillar", and "Cabrera Stone": Destroy all cards on the field, also
/// for the rest of this turn, your opponent takes any effect damage you would have taken instead." Everything on both
/// sides is destroyed, and the 2000 + 1000 the Pillar and the Stone would cost their controller land on the opponent.
#[test]
fn triangle_o_destroys_everything_and_sends_the_damage_to_the_opponent() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &triangle_board()) else { return };
    s.run(&[Activate(TRIANGLE_O)]).unwrap();
    clean(&s);
    for team in 0..2 {
        assert_eq!(count_all(&s, team, "mzone"), 0, "monsters of team {team}: {}", s.context());
        assert_eq!(count_all(&s, team, "szone"), 0, "Spells and Traps of team {team}: {}", s.context());
    }
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 5000, "{}", s.context());
}

/// "If you control Crystal Skull, Ashoka Pillar, and Cabrera Stone": without the Pillar, Triangle O cannot be activated.
#[test]
fn triangle_o_needs_all_three_stones() {
    let spawns = [at(0, Hand, TRIANGLE_O), at(0, Monster, CRYSTAL_SKULL), at(0, Monster, CABRERA_STONE)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, TRIANGLE_O), "{}", s.context());
}

/// "You can banish this card from your GY, then target 1 each of "Crystal Skull", "Ashoka Pillar", and "Cabrera Stone"
/// in your GY; shuffle them into the Deck, then draw 3 cards." The three go back to the Deck and 3 cards are drawn.
#[test]
fn triangle_o_shuffles_the_three_back_and_draws_three() {
    let spawns = [at(0, Grave, TRIANGLE_O), at(0, Grave, CRYSTAL_SKULL), at(0, Grave, ASHOKA_PILLAR), at(0, Grave, CABRERA_STONE)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    let (hand_before, deck_before) = (hand(&s, 0), pile(&s, 0));
    s.run(&[Activate(TRIANGLE_O)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", TRIANGLE_O), 1, "{}", s.context());
    for cid in [CRYSTAL_SKULL, ASHOKA_PILLAR, CABRERA_STONE] {
        assert_eq!(s.count(0, "grave", cid), 0, "{cid} back in the Deck: {}", s.context());
    }
    assert_eq!(hand(&s, 0), hand_before + 3, "{}", s.context());
    // Three cards went into the Deck and three came out of it.
    assert_eq!(pile(&s, 0), deck_before, "{}", s.context());
}

// ---- Tribute Burial ----

/// Tribute Burial: "Once during this turn, you can Tribute Summon a monster that requires 2 Tributes by banishing 1
/// monster from each GY instead, also you cannot Special Summon monsters for the rest of this turn." Blue-Eyes
/// (Level 8) comes without a Tribute; a monster of each Graveyard is banished, and Monster Reborn is no longer allowed.
#[test]
fn tribute_burial_summons_a_two_tribute_monster_by_banishing_one_from_each_grave() {
    let spawns = [at(0, Hand, TRIBUTE_BURIAL), at(0, Hand, BLUE_EYES), at(0, Hand, MONSTER_REBORN), at(0, Grave, KURIBOH),
        at(0, Grave, WARWOLF), at(1, Grave, KURIBOH), at(1, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, KURIBOH)]);
    s.run(&[Activate(TRIBUTE_BURIAL), Summon(BLUE_EYES)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(count_all(&s, 0, "removed"), 1, "{}", s.context());
    assert_eq!(count_all(&s, 1, "removed"), 1, "one monster of each Graveyard: {}", s.context());
    assert!(!can_activate(&s, MONSTER_REBORN), "no Special Summon the rest of the turn: {}", s.context());
}

/// "...a monster that requires 2 Tributes": a Level 5 monster (Valkyrie Zweite) needs only one, so the card does not let
/// it be Summoned without a Tribute.
#[test]
fn tribute_burial_does_not_help_a_one_tribute_monster() {
    let spawns = [at(0, Hand, TRIBUTE_BURIAL), at(0, Hand, ZWEITE), at(0, Grave, KURIBOH), at(1, Grave, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(TRIBUTE_BURIAL)]).unwrap();
    clean(&s);
    assert!(!listed(&s, "summons", ZWEITE), "{}", s.context());
    assert_eq!(s.count(0, "szone", TRIBUTE_BURIAL) + s.count(0, "grave", TRIBUTE_BURIAL), 1, "{}", s.context());
}

/// Tribute Burial, anime version: "you can conduct a Tribute Summon by banishing 1 monster from each Graveyard instead of
/// Tributing": a Level 5 monster comes without its Tribute too.
#[test]
fn tribute_burial_anime_replaces_any_tribute_summons_tribute() {
    let spawns = [at(0, Hand, TRIBUTE_BURIAL), at(0, Hand, ZWEITE), at(0, Grave, KURIBOH), at(1, Grave, KURIBOH)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(TRIBUTE_BURIAL), Summon(ZWEITE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZWEITE), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(1, "removed", KURIBOH), 1, "{}", s.context());
}

// ---- Tyrant Wing and Tyrant Burst Dragon ----

/// Tyrant Wing: "Target 1 Dragon monster on the field; equip this card to it. It gains 400 ATK/DEF, also it can make up to
/// 2 attacks on monsters during each Battle Phase. Once per turn, during the End Phase, if the monster equipped with this
/// card by this effect attacked an opponent's monster this turn: Destroy this card." Luster Dragon (1900) goes to 2300,
/// attacks both of the opponent's monsters, and the Wing is destroyed at the End Phase.
#[test]
fn tyrant_wing_equips_a_dragon_gives_two_attacks_and_destroys_itself() {
    let spawns = [at(0, SpellSet, TYRANT_WING), at(0, Monster, LUSTER_DRAGON), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(TYRANT_WING)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, LUSTER_DRAGON), Some(2300), "{}", s.context());
    assert_eq!(s.count(0, "szone", TYRANT_WING), 1, "{}", s.context());
    let spawns = [at(0, SpellSet, TYRANT_WING), at(0, Monster, LUSTER_DRAGON), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH)];
    let mut s = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns).unwrap();
    s.run_both([&[Activate(TYRANT_WING), Attack(LUSTER_DRAGON), Attack(LUSTER_DRAGON), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 2, "two attacks on monsters: {}", s.context());
    assert_eq!(s.count(0, "mzone", LUSTER_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", TYRANT_WING), 1, "destroyed at the End Phase: {}", s.context());
}

/// Tyrant Wing, anime version: "When an opponent's monster declares an attack: Equip this card to a monster you control.
/// It gains 400 ATK". Chained to Warwolf's attack it equips Luster Dragon, which has 2300 ATK from then on.
#[test]
fn tyrant_wing_anime_equips_at_an_attack() {
    let spawns = [at(0, SpellSet, TYRANT_WING), at(0, Monster, LUSTER_DRAGON), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(TYRANT_WING)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, LUSTER_DRAGON), Some(2300), "{}", s.context());
    assert_eq!(s.count(0, "szone", TYRANT_WING), 1, "equipped: {}", s.context());
}

fn burst_board() -> Vec<Spawn> {
    vec![at(0, Hand, FANG_OF_CRITIAS), at(0, SpellSet, TYRANT_WING), at(0, Monster, WARWOLF), at(1, Monster, KURIBOH),
        at(1, Monster, KURIBOH), at(1, Monster, SKULL_SERVANT)]
}

/// Tyrant Burst Dragon: "Must be Special Summoned with "The Fang of Critias", using "Tyrant Wing"." The Fang sends the Set
/// Tyrant Wing and the Dragon comes out of the Extra Deck.
#[test]
fn tyrant_burst_dragon_comes_with_the_fang_using_tyrant_wing() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[TYRANT_BURST_DRAGON]), (&PLAIN, &[])], &burst_board()) else { return };
    s.run(&[Activate(FANG_OF_CRITIAS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TYRANT_BURST_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", TYRANT_WING), 1, "{}", s.context());
}

/// "This card can attack all your opponent's monsters, once each": it attacks the opponent's three monsters in one Battle
/// Phase and destroys them all.
#[test]
fn tyrant_burst_dragon_attacks_all_the_opponents_monsters() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[TYRANT_BURST_DRAGON]), (&PLAIN, &[])], &burst_board()) else { return };
    s.run(&[Activate(FANG_OF_CRITIAS), Attack(TYRANT_BURST_DRAGON), Attack(TYRANT_BURST_DRAGON), Attack(TYRANT_BURST_DRAGON)])
        .unwrap();
    clean(&s);
    assert_eq!(count_all(&s, 1, "mzone"), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TYRANT_BURST_DRAGON), 1, "{}", s.context());
}

/// "You can target 1 face-up monster you control; equip this monster to that target. It gains 400 ATK/DEF": the Dragon
/// equips itself to Warwolf (2000 to 2400).
#[test]
fn tyrant_burst_dragon_equips_itself_for_400() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[TYRANT_BURST_DRAGON]), (&PLAIN, &[])], &burst_board()) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(FANG_OF_CRITIAS), Activate(TYRANT_BURST_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TYRANT_BURST_DRAGON), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", TYRANT_BURST_DRAGON), 1, "equipped: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2400), "{}", s.context());
}

/// Tyrant Burst Dragon, anime version: "You can banish this card, then target 1 Dragon-Type monster you control; it gains
/// 400 ATK, and can attack all monsters your opponent controls once each." Luster Dragon goes to 2300 and attacks two
/// monsters.
#[test]
fn tyrant_burst_dragon_anime_banishes_itself_for_a_dragon() {
    let spawns = [at(0, Monster, TYRANT_BURST_DRAGON), at(0, Monster, LUSTER_DRAGON), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[TYRANT_BURST_DRAGON]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, LUSTER_DRAGON)]);
    s.run(&[Activate(TYRANT_BURST_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", TYRANT_BURST_DRAGON), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, LUSTER_DRAGON), Some(2300), "{}", s.context());
    s.run(&[Attack(LUSTER_DRAGON), Attack(LUSTER_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(count_all(&s, 1, "mzone"), 0, "attacks all once each: {}", s.context());
}

// ---- Unacceptable Result ----

/// Unacceptable Result: "If you control a Spellcaster monster: Special Summon 1 "Fortune Fairy" monster from your hand."
#[test]
fn unacceptable_result_summons_a_fortune_fairy() {
    let spawns = [at(0, Hand, UNACCEPTABLE_RESULT), at(0, Hand, FORTUNE_FAIRY_HU), at(0, Monster, DARK_MAGICIAN)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(UNACCEPTABLE_RESULT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", FORTUNE_FAIRY_HU), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", UNACCEPTABLE_RESULT), 1, "{}", s.context());
}

/// Without a Spellcaster (Warwolf is a Beast-Warrior) it cannot be activated.
#[test]
fn unacceptable_result_needs_a_spellcaster() {
    let spawns = [at(0, Hand, UNACCEPTABLE_RESULT), at(0, Hand, FORTUNE_FAIRY_HU), at(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, UNACCEPTABLE_RESULT), "{}", s.context());
}

// ---- Unknown Synchron ----

/// Unknown Synchron: "If only your opponent controls a monster, you can Special Summon this card (from your hand)."
#[test]
fn unknown_synchron_comes_when_only_the_opponent_has_a_monster() {
    let spawns = [at(0, Hand, UNKNOWN_SYNCHRON), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[SpSummon(UNKNOWN_SYNCHRON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", UNKNOWN_SYNCHRON), 1, "{}", s.context());
}

/// With no monster on the opponent's side, or one on its own, it cannot be Special Summoned.
#[test]
fn unknown_synchron_needs_only_the_opponent_to_control_a_monster() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, UNKNOWN_SYNCHRON)]) else { return };
    s.run(&[]).unwrap();
    assert!(!listed(&s, "special_summons", UNKNOWN_SYNCHRON), "an empty field: {}", s.context());
    let spawns = [at(0, Hand, UNKNOWN_SYNCHRON), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let mut s = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns).unwrap();
    s.run(&[]).unwrap();
    assert!(!listed(&s, "special_summons", UNKNOWN_SYNCHRON), "both control one: {}", s.context());
}

/// "You can only Special Summon "Unknown Synchron" once per Duel this way.": the first copy is Special Summoned and
/// destroyed in battle, and the second copy in the hand is not offered although the situation is the same again.
#[test]
fn unknown_synchron_comes_once_per_duel() {
    let spawns = [at(0, Hand, UNKNOWN_SYNCHRON), at(0, Hand, UNKNOWN_SYNCHRON), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the end-of-turn discard
    s.run_both([&[SpSummon(UNKNOWN_SYNCHRON), EndTurn], &[Attack(WARWOLF), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", UNKNOWN_SYNCHRON), 1, "the first one died: {}", s.context());
    assert_eq!(count_all(&s, 0, "mzone"), 0, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", UNKNOWN_SYNCHRON), 1, "{}", s.context());
    assert!(!listed(&s, "special_summons", UNKNOWN_SYNCHRON), "{}", s.context());
}

/// Unknown Synchron, anime version has no once-per-Duel limit: the same board offers the second copy.
#[test]
fn unknown_synchron_anime_can_come_again() {
    let spawns = [at(0, Hand, UNKNOWN_SYNCHRON), at(0, Hand, UNKNOWN_SYNCHRON), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the end-of-turn discard
    s.run_both([&[SpSummon(UNKNOWN_SYNCHRON), EndTurn], &[Attack(WARWOLF), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", UNKNOWN_SYNCHRON), 1, "{}", s.context());
    assert!(listed(&s, "special_summons", UNKNOWN_SYNCHRON), "{}", s.context());
}

// ---- Valkyrie Brunhilde ----

/// Valkyrie Brunhilde: "Gains 500 ATK for each monster your opponent controls." Two monsters: 1800 + 1000.
#[test]
fn brunhilde_gains_500_for_each_opposing_monster() {
    let spawns = [at(0, Monster, BRUNHILDE), at(1, Monster, WARWOLF), at(1, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, BRUNHILDE), Some(2800), "{}", s.context());
}

/// "Unaffected by your opponent's Spell effects.": the opponent's Dark Hole destroys their own Warwolf and not her.
#[test]
fn brunhilde_is_unaffected_by_the_opponents_spells() {
    let spawns = [at(0, Monster, BRUNHILDE), at(1, Monster, WARWOLF), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BRUNHILDE), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// "When an opponent's monster declares an attack: You can have this card lose exactly 1000 DEF, and if you do, your
/// "Valkyrie" monsters cannot be destroyed by battle this turn." In Defense Position (2000 DEF) she would fall to
/// Warwolf's 2000 ATK at 1000 DEF; with the effect she stays, at 1000 DEF.
#[test]
fn brunhilde_pays_1000_defense_so_that_no_valkyrie_is_destroyed_by_battle() {
    let spawns = [at(0, MonsterDefense, BRUNHILDE), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(BRUNHILDE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BRUNHILDE), 1, "{}", s.context());
    assert_eq!(s.monster(0, BRUNHILDE).map(|m| m["defense"].as_i64()), Some(Some(1000)), "{}", s.context());
}

/// Valkyrie Brunhilde, anime version: "Gains 300 ATK for each Dragon-Type and Warrior-Type monster on the field." A
/// Blue-Eyes (Dragon) of the opponent's and a Luster Dragon of hers: 1800 + 600, and Warwolf (Beast-Warrior) adds nothing.
#[test]
fn brunhilde_anime_gains_300_for_each_dragon_and_warrior() {
    let spawns = [at(0, Monster, BRUNHILDE), at(0, Monster, LUSTER_DRAGON), at(1, Monster, BLUE_EYES), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, BRUNHILDE), Some(2400), "{}", s.context());
}

// ---- Valkyrie Dritte ----

/// Valkyrie Dritte: "If this card is Normal or Special Summoned: You can add 1 "Valkyrie" card from your Deck to your hand,
/// except "Valkyrie Dritte"." A Valkyrie card leaves the Deck for the hand.
#[test]
fn dritte_adds_a_valkyrie_card_from_the_deck() {
    let deck = deck_with(&[ZWEITE], 20);
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &[at(0, Hand, DRITTE)]) else { return };
    let (hand_before, deck_before) = (hand(&s, 0), deck_count(&s));
    s.run(&[Summon(DRITTE), Chain(DRITTE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DRITTE), 1, "{}", s.context());
    assert_eq!(deck_count(&s), deck_before - 1, "{}", s.context());
    assert_eq!(hand(&s, 0), hand_before, "-1 for the Summon, +1 for the card: {}", s.context());
}

fn deck_count(s: &Scenario) -> i64 {
    pile(s, 0)
}

/// "Gains 200 ATK for each of your opponent's banished monsters.": two banished, 1000 + 400.
#[test]
fn dritte_gains_200_for_each_banished_opposing_monster() {
    let spawns = [at(0, Monster, DRITTE), at(1, Banished, KURIBOH), at(1, Banished, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DRITTE), Some(1400), "{}", s.context());
}

/// Valkyrie Dritte, anime version: "Gains 100 ATK for each of your opponent's banished monsters." Two: 1000 + 200.
#[test]
fn dritte_anime_gains_100_for_each_banished_opposing_monster() {
    let spawns = [at(0, Monster, DRITTE), at(1, Banished, KURIBOH), at(1, Banished, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DRITTE), Some(1200), "{}", s.context());
}

// ---- Valkyrie Erste ----

/// Valkyrie Erste: "If this card is Special Summoned from your hand by a Spell effect: You can target 1 "Mischief of the
/// Time Goddess" in your GY; add it to your hand." Ride of the Valkyries brings her from the hand.
#[test]
fn erste_gets_mischief_back_when_a_spell_summons_her() {
    let spawns = [at(0, Hand, RIDE_OF_THE_VALKYRIES), at(0, Hand, ERSTE), at(0, Grave, MISCHIEF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(RIDE_OF_THE_VALKYRIES), Chain(ERSTE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ERSTE), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", MISCHIEF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", MISCHIEF), 0, "{}", s.context());
}

/// "If you control a "Valkyrie" monster other than "Valkyrie Erste" (Quick Effect): You can banish 1 monster from your
/// opponent's GY, then this card's ATK becomes equal to the banished monster's original ATK until the end of this turn."
/// Warwolf in the opponent's Graveyard: banished, Erste (1600) becomes 2000.
#[test]
fn erste_banishes_a_monster_and_takes_its_attack() {
    let spawns = [at(0, Monster, ERSTE), at(0, Monster, ZWEITE), at(1, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(ERSTE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, ERSTE), Some(2000), "{}", s.context());
}

/// Without another "Valkyrie" monster the effect is not available.
#[test]
fn erste_needs_another_valkyrie() {
    let spawns = [at(0, Monster, ERSTE), at(0, Monster, KURIBOH), at(1, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, ERSTE), "{}", s.context());
}

/// Valkyrie Erste, anime version: "Once per turn: You can target 1 monster in your opponent's Graveyard; banish it." No
/// other Valkyrie is needed.
#[test]
fn erste_anime_needs_no_other_valkyrie() {
    let spawns = [at(0, Monster, ERSTE), at(1, Grave, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(ERSTE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, ERSTE), Some(2000), "{}", s.context());
}

// ---- Valkyrie Zweite ----

/// Valkyrie Zweite: "If this card is Summoned: You can target 1 monster your opponent controls; destroy it." Tribute
/// Summoned over a Kuriboh, she destroys Warwolf.
#[test]
fn zweite_destroys_an_opposing_monster_when_summoned() {
    let spawns = [at(0, Hand, ZWEITE), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Summon(ZWEITE), Chain(ZWEITE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZWEITE), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// "After damage calculation, if this card battled: You can target 1 Continuous Spell in your GY; add it to your hand."
/// She attacks a Kuriboh and gets The A. Forces back.
#[test]
fn zweite_adds_a_continuous_spell_from_the_grave_after_battle() {
    let spawns = [at(0, Monster, ZWEITE), at(0, Grave, THE_A_FORCES), at(1, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Attack(ZWEITE), Chain(ZWEITE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", THE_A_FORCES), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THE_A_FORCES), 0, "{}", s.context());
}

/// Valkyrie Zweite, anime version: "If this card is Summoned: Target 1 monster your opponent controls; destroy it, if
/// any." No battle effect: the Continuous Spell stays in the Graveyard.
#[test]
fn zweite_anime_has_no_battle_effect() {
    let spawns = [at(0, Monster, ZWEITE), at(0, Grave, THE_A_FORCES), at(1, Monster, KURIBOH)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Attack(ZWEITE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THE_A_FORCES), 1, "{}", s.context());
}

// ---- Valkyrie's Embrace ----

/// Valkyrie's Embrace: "If all monsters you control are "Valkyrie" monsters: Target 1 Attack Position "Valkyrie" monster
/// you control and 1 face-up monster your opponent controls; change your monster to Defense Position, and if you do,
/// banish that opponent's monster. You cannot Special Summon monsters the turn you activate this card, except "Valkyrie"
/// monsters."
#[test]
fn embrace_banishes_the_opponents_monster_and_turns_the_valkyrie_to_defense() {
    let spawns = [at(0, Hand, EMBRACE), at(0, Hand, MONSTER_REBORN), at(0, Monster, BRUNHILDE), at(0, Grave, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(EMBRACE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.position_of(0, BRUNHILDE), Some(4), "{}", s.context());
    assert!(!can_activate(&s, MONSTER_REBORN), "no non-Valkyrie Special Summon this turn: {}", s.context());
}

/// With a monster that is not a Valkyrie too, it cannot be activated.
#[test]
fn embrace_needs_all_your_monsters_to_be_valkyries() {
    let spawns = [at(0, Hand, EMBRACE), at(0, Monster, BRUNHILDE), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, EMBRACE), "{}", s.context());
}

/// Valkyrie's Embrace, anime version: no such condition, and no Special Summon restriction.
#[test]
fn embrace_anime_works_next_to_other_monsters() {
    let spawns = [at(0, Hand, EMBRACE), at(0, Monster, BRUNHILDE), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(EMBRACE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.position_of(0, BRUNHILDE), Some(4), "{}", s.context());
}

// ---- Water of Life ----

/// Water of Life: "If you control no monsters: Target 1 monster in your GY; Special Summon that target in Attack
/// Position".
#[test]
fn water_of_life_revives_a_monster_in_attack_position() {
    let spawns = [at(0, Hand, WATER_OF_LIFE), at(0, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(WATER_OF_LIFE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.position_of(0, WARWOLF), Some(1), "{}", s.context());
}

/// "If you control no monsters": with a Kuriboh on the field it cannot be activated.
#[test]
fn water_of_life_needs_an_empty_field() {
    let spawns = [at(0, Hand, WATER_OF_LIFE), at(0, Grave, WARWOLF), at(0, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, WATER_OF_LIFE), "{}", s.context());
}

/// Water of Life, anime version has no such condition: with a Kuriboh on the field it revives Warwolf next to it.
#[test]
fn water_of_life_anime_works_with_monsters_on_the_field() {
    let spawns = [at(0, Hand, WATER_OF_LIFE), at(0, Grave, WARWOLF), at(0, Monster, KURIBOH)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(WATER_OF_LIFE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
}

// ---- Weathering Soldier ----

/// Weathering Soldier: "If this card is destroyed by battle or sent to the GY by a card effect: You can add 1 "Fossil
/// Fusion", or 1 card that specifically lists "Fossil Fusion" in its text except "Weathering Soldier", from your Deck to
/// your hand." Her controller's own Dark Hole destroys her, and Fossil Fusion leaves the Deck.
#[test]
fn weathering_soldier_adds_fossil_fusion_when_destroyed_by_an_effect() {
    let deck = deck_with(&[FOSSIL_FUSION], 20);
    let spawns = [at(0, Monster, WEATHERING_SOLDIER), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    let (hand_before, deck_before) = (hand(&s, 0), pile(&s, 0));
    s.run(&[Activate(DARK_HOLE), Chain(WEATHERING_SOLDIER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WEATHERING_SOLDIER), 1, "{}", s.context());
    assert_eq!(pile(&s, 0), deck_before - 1, "{}", s.context());
    assert_eq!(hand(&s, 0), hand_before, "-1 for Dark Hole, +1 for the card: {}", s.context());
}

/// Destroyed by battle too: the opponent's Warwolf (2000) attacks her at 2000 ATK... in Defense Position (1200 DEF).
#[test]
fn weathering_soldier_adds_fossil_fusion_when_destroyed_by_battle() {
    let deck = deck_with(&[FOSSIL_FUSION], 20);
    let spawns = [at(0, MonsterDefense, WEATHERING_SOLDIER), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    let deck_before = pile(&s, 0);
    s.run_both([&[EndTurn, Chain(WEATHERING_SOLDIER)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WEATHERING_SOLDIER), 1, "{}", s.context());
    assert_eq!(pile(&s, 0), deck_before - 1, "{}", s.context());
}

/// "Once per turn, during your End Phase: This card loses 600 ATK."
#[test]
fn weathering_soldier_loses_600_attack_at_the_end_phase() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, WEATHERING_SOLDIER)]) else { return };
    s.run(&[Summon(WEATHERING_SOLDIER)]).unwrap();
    assert_eq!(s.attack_of(0, WEATHERING_SOLDIER), Some(2000), "{}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WEATHERING_SOLDIER), Some(1400), "{}", s.context());
}

// ---- White Knight Lord ----

/// White Knight Lord: "You can banish 3 monsters from your GY; Special Summon this card from your hand, then you can apply
/// 1 of these effects. ... Add 1 "White Veil" from your Deck to your hand." (No "White Knight" or "White Night" monster
/// exists in the game to play the first branch with.) Three monsters leave the Graveyard for the banished zone.
#[test]
fn white_knight_lord_banishes_three_and_adds_white_veil() {
    let deck = deck_with(&[WHITE_VEIL], 20);
    let spawns = [at(0, Hand, WHITE_KNIGHT_LORD), at(0, Grave, KURIBOH), at(0, Grave, KURIBOH), at(0, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    let (hand_before, deck_before) = (hand(&s, 0), pile(&s, 0));
    s.run(&[Activate(WHITE_KNIGHT_LORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WHITE_KNIGHT_LORD), 1, "{}", s.context());
    assert_eq!(count_all(&s, 0, "removed"), 3, "{}", s.context());
    assert_eq!(count_all(&s, 0, "grave"), 0, "{}", s.context());
    assert_eq!(pile(&s, 0), deck_before - 1, "White Veil: {}", s.context());
    assert_eq!(hand(&s, 0), hand_before, "-1 for the Summon, +1 for the Veil: {}", s.context());
}

/// White Knight Lord, first branch: "Special Summon 1 "White Knight" or "White Night" monster from your hand or Deck."
/// White Night Dragon comes from the Deck (the first option is chosen).
#[test]
fn white_knight_lord_special_summons_white_night_dragon() {
    let deck = deck_with(&[WHITE_NIGHT_DRAGON], 20);
    let spawns = [at(0, Hand, WHITE_KNIGHT_LORD), at(0, Grave, KURIBOH), at(0, Grave, KURIBOH), at(0, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    let before = s.count(0, "mzone", WHITE_NIGHT_DRAGON) + pile(&s, 0) as usize;
    s.choose_options(&[0]);
    s.run(&[Activate(WHITE_KNIGHT_LORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WHITE_KNIGHT_LORD), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WHITE_NIGHT_DRAGON), 1, "{}", s.context());
    assert_eq!(count_all(&s, 0, "removed"), 3, "{}", s.context());
    assert_eq!(s.count(0, "hand", WHITE_VEIL), 0, "the other branch was not taken: {}", s.context());
    assert!(before > 0, "{}", s.context());
}

/// "When an attack is declared involving this card and an opponent's monster: Change that opponent's monster's ATK to 0."
/// Attacking a 2000 ATK Warwolf, she deals her full 2000.
#[test]
fn white_knight_lord_turns_the_opposing_monsters_attack_to_zero() {
    let spawns = [at(0, Monster, WHITE_KNIGHT_LORD), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Attack(WHITE_KNIGHT_LORD)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WHITE_KNIGHT_LORD), 1, "{}", s.context());
    assert_eq!(s.lp(1), 6000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// "If this card in its owner's possession is destroyed by an opponent's card: Inflict 1000 damage to your opponent."
#[test]
fn white_knight_lord_burns_the_opponent_for_1000_when_their_card_destroys_her() {
    let spawns = [at(0, Monster, WHITE_KNIGHT_LORD), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WHITE_KNIGHT_LORD), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7000, "{}", s.context());
}

/// White Knight Lord, anime version: "During each End Phase, if this card did not battle this turn: Take 800 damage.
/// Cannot be destroyed by battle, also you take no battle damage from attacks involving this card." She takes 800 at the
/// first End Phase, then survives Warwolf's attack without damage.
#[test]
fn white_knight_lord_anime_costs_800_and_cannot_be_hurt_in_battle() {
    let spawns = [at(0, Monster, WHITE_KNIGHT_LORD), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WHITE_KNIGHT_LORD), 1, "{}", s.context());
    assert_eq!(s.lp(0), 7200, "{}", s.context());
}

// ---- White Veil ----

/// White Veil: an Equip Spell for the monster it is put on.
#[test]
fn white_veil_equips_a_monster() {
    let spawns = [at(0, Hand, WHITE_VEIL), at(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(WHITE_VEIL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", WHITE_VEIL), 1, "{}", s.context());
}

/// "When this face-up card in a player's Spell & Trap Zone leaves the field, that player takes 3000 damage." The
/// opponent's Mystical Space Typhoon destroys it: 8000 - 3000.
#[test]
fn white_veil_costs_its_controller_3000_when_it_leaves_the_field() {
    let spawns = [at(0, Hand, WHITE_VEIL), at(0, Monster, WARWOLF), at(1, Hand, MST)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, WHITE_VEIL)]);
    s.run_both([&[Activate(WHITE_VEIL), EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WHITE_VEIL), 1, "{}", s.context());
    assert_eq!(s.lp(0), 5000, "{}", s.context());
}

/// "When the equipped monster destroys an opponent's monster by battle: You can destroy all Spell and Trap Cards your
/// opponent controls." Warwolf kills Kuriboh and the opponent's Set Trap goes (it could not have been activated: "your
/// opponent cannot activate any Spell/Trap Cards" while the equipped monster battles).
#[test]
fn white_veil_destroys_the_opponents_spells_and_traps_after_a_kill() {
    let spawns = [at(0, Hand, WHITE_VEIL), at(0, Monster, WARWOLF), at(1, Monster, KURIBOH), at(1, SpellSet, CALL_OF_THE_HAUNTED)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(WHITE_VEIL), Attack(WARWOLF), Chain(WHITE_VEIL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", CALL_OF_THE_HAUNTED), 1, "{}", s.context());
    assert_eq!(count_all(&s, 1, "szone"), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", WHITE_VEIL), 1, "{}", s.context());
}

/// White Veil, anime version: "If the equipped monster attacks, destroy all Spell and Trap Cards your opponent controls
/// and negate their effect(s). When the equipped monster is destroyed: its controller takes damage equal to the equipped
/// monster's ATK." The attack alone clears the opponent's Set Trap.
#[test]
fn white_veil_anime_clears_the_opponents_spells_and_traps_on_attack() {
    let spawns = [at(0, Hand, WHITE_VEIL), at(0, Monster, WARWOLF), at(1, Monster, KURIBOH), at(1, SpellSet, CALL_OF_THE_HAUNTED)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(WHITE_VEIL), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", CALL_OF_THE_HAUNTED), 1, "{}", s.context());
    assert_eq!(count_all(&s, 1, "szone"), 0, "{}", s.context());
}

/// "When the equipped monster is destroyed: its controller takes damage equal to the equipped monster's ATK." Dark Hole
/// destroys the equipped Warwolf (2000).
#[test]
fn white_veil_anime_burns_its_controller_when_the_monster_is_destroyed() {
    let spawns = [at(0, Hand, WHITE_VEIL), at(0, Monster, WARWOLF), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[Activate(WHITE_VEIL), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "{}", s.context());
}

// ---- Wing Requital ----

/// Wing Requital: "If all monsters you control are Winged Beast monsters and you control at least 2 monsters with different
/// original names: Pay 600 LP; draw 2 cards."
#[test]
fn wing_requital_pays_600_and_draws_two() {
    let spawns = [at(0, Hand, WING_REQUITAL), at(0, Monster, HARPIE_LADY), at(0, Monster, SONIC_BIRD)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    let (hand_before, deck_before) = (hand(&s, 0), pile(&s, 0));
    s.run(&[Activate(WING_REQUITAL)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7400, "{}", s.context());
    assert_eq!(hand(&s, 0), hand_before + 1, "-1 for the card, +2 drawn: {}", s.context());
    assert_eq!(pile(&s, 0), deck_before - 2, "{}", s.context());
}

/// Two copies of the same Winged Beast, or a monster of another race, do not meet the condition.
#[test]
fn wing_requital_needs_winged_beasts_with_different_names() {
    let spawns = [at(0, Hand, WING_REQUITAL), at(0, Monster, HARPIE_LADY), at(0, Monster, HARPIE_LADY)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, WING_REQUITAL), "same names: {}", s.context());
    let spawns = [at(0, Hand, WING_REQUITAL), at(0, Monster, HARPIE_LADY), at(0, Monster, SONIC_BIRD), at(0, Monster, WARWOLF)];
    let mut s = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns).unwrap();
    s.run(&[]).unwrap();
    assert!(!can_activate(&s, WING_REQUITAL), "a Beast-Warrior among them: {}", s.context());
}

/// Wing Requital, anime version: "Pay 600 LP; count the number of Winged-Beast-Type monsters you control. During the next
/// Standby Phase, draw an equal number of cards." Nothing is drawn at once; the two cards come at the Standby Phase.
#[test]
fn wing_requital_anime_draws_at_the_next_standby_phase() {
    let spawns = [at(0, Hand, WING_REQUITAL), at(0, Monster, HARPIE_LADY), at(0, Monster, SONIC_BIRD)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    let deck_before = pile(&s, 0);
    s.run(&[Activate(WING_REQUITAL)]).unwrap();
    assert_eq!(s.lp(0), 7400, "{}", s.context());
    assert_eq!(pile(&s, 0), deck_before, "nothing drawn yet: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    // The opponent's Standby Phase comes first: the draw belongs to the card's controller.
    assert!(pile(&s, 0) <= deck_before - 2, "two cards drawn by the Standby Phase: {}", s.context());
}

// ---- Wiretap ----

/// Wiretap: "When a Trap Card is activated: Negate the activation, and if you do, shuffle that card into the Deck." The
/// opponent's Call of the Haunted is negated and does not reach the Graveyard.
#[test]
fn wiretap_negates_a_trap_and_shuffles_it_into_the_deck() {
    let spawns = [at(0, SpellSet, WIRETAP), at(1, SpellSet, CALL_OF_THE_HAUNTED), at(1, Grave, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    let deck_before = pile(&s, 1);
    s.run_both([&[Chain(WIRETAP)], &[Activate(CALL_OF_THE_HAUNTED)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "the revival was negated: {}", s.context());
    assert_eq!(s.count(1, "grave", CALL_OF_THE_HAUNTED), 0, "{}", s.context());
    assert_eq!(s.count(1, "szone", CALL_OF_THE_HAUNTED), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", WIRETAP), 1, "{}", s.context());
    // One card drawn for the turn, the Trap shuffled in.
    assert_eq!(pile(&s, 1), deck_before, "{}", s.context());
}

// ---- Yowie ----

/// Yowie: "If this card is Summoned (and no other cards): You can activate this effect; your opponent skips their next
/// Draw Phase." The opponent's hand does not grow on their turn.
#[test]
fn yowie_makes_the_opponent_skip_their_next_draw() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, YOWIE)]) else { return };
    let before = hand(&s, 1);
    s.run_both([&[Summon(YOWIE), Chain(YOWIE), EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 1), before, "no draw: {}", s.context());
}

/// Yowie, anime version: "When this card is Summoned, your opponent skips their next Draw Phase." Automatic.
#[test]
fn yowie_anime_skips_the_draw_without_asking() {
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, YOWIE)]) else { return };
    let before = hand(&s, 1);
    s.run_both([&[Summon(YOWIE), EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 1), before, "{}", s.context());
}

// ---- Zaphion, the Timelord ----

/// Zaphion, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing." A Level 10
/// monster comes with no Tribute on an empty field, and not next to another monster.
#[test]
fn zaphion_is_summoned_without_tribute_only_on_an_empty_field() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, ZAPHION)]) else { return };
    s.run(&[Summon(ZAPHION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZAPHION), 1, "{}", s.context());
    let spawns = [at(0, Hand, ZAPHION), at(0, Monster, KURIBOH)];
    let mut s = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns).unwrap();
    s.run(&[]).unwrap();
    assert!(!listed(&s, "summons", ZAPHION), "{}", s.context());
}

/// "Cannot be destroyed by battle or card effects.": the opponent's Dark Hole destroys their own Warwolf and not her.
#[test]
fn zaphion_cannot_be_destroyed_by_card_effects() {
    let spawns = [at(0, Hand, ZAPHION), at(1, Monster, WARWOLF), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[Summon(ZAPHION), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZAPHION), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// "You take no battle damage from attacks involving this card." Warwolf (2000) attacks her (0 ATK): she stays and
/// her controller loses nothing.
#[test]
fn zaphion_cannot_be_destroyed_by_battle_and_takes_no_battle_damage() {
    let spawns = [at(0, Hand, ZAPHION), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[Summon(ZAPHION), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZAPHION), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// "At the end of the Battle Phase, if this card battled: Shuffle all Spells and Traps your opponent controls into the
/// Deck." Zaphion (0 ATK), Normal Summoned on the second player's turn (she would be shuffled away at their next Standby
/// Phase), attacks Kuriboh; the other side's Set Trap is gone after the Battle Phase and is in the Deck.
#[test]
fn zaphion_shuffles_the_opponents_spells_and_traps_into_the_deck_after_battle() {
    let spawns = [at(0, Monster, KURIBOH), at(0, SpellSet, CALL_OF_THE_HAUNTED), at(1, Hand, ZAPHION)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    let deck_before = pile(&s, 0);
    s.run_both([&[], &[Summon(ZAPHION), Attack(ZAPHION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(count_all(&s, 0, "szone"), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", CALL_OF_THE_HAUNTED), 0, "shuffled, not destroyed: {}", s.context());
    // The Trap went in and the player drew one card for the next turn.
    assert_eq!(pile(&s, 0), deck_before, "{}", s.context());
    assert_eq!(s.count(1, "mzone", ZAPHION), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn zaphion_goes_back_to_the_deck_at_the_next_standby_phase() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, ZAPHION)]) else { return };
    s.run_both([&[Summon(ZAPHION), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZAPHION), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", ZAPHION), 0, "{}", s.context());
}

/// Zaphion, anime version: "At the end of the Damage Step, if this card battled: Shuffle all Spell and Trap Cards your
/// opponent controls into the Deck." It happens right after the attack, with no End Phase wanted.
#[test]
fn zaphion_anime_shuffles_at_the_end_of_the_damage_step() {
    // The defender is Warwolf: the anime Kuriboh negates an attack on itself and is destroyed for its own ATK.
    let spawns = [at(0, Monster, WARWOLF), at(0, SpellSet, CALL_OF_THE_HAUNTED), at(1, Hand, ZAPHION), at(1, Monster, KURIBOH),
        at(1, Monster, KURIBOH)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[], &[Summon(ZAPHION), Attack(ZAPHION)]]).unwrap();
    clean(&s);
    assert_eq!(count_all(&s, 0, "szone"), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", CALL_OF_THE_HAUNTED), 0, "{}", s.context());
}

// ---- Zushin the Sleeping Giant ----

fn counters_on(s: &Scenario, team: usize, cid: u16) -> u64 {
    s.monster(team, cid).and_then(|m| m["counters"].as_object().map(|o| o.values().filter_map(|v| v.as_u64()).sum())).unwrap_or(0)
}

/// Zushin: "Once per turn: You can reveal this card in your hand, then target 1 Level 1 Normal Monster you control (keep
/// this card revealed until the end of this turn); place 1 Zushin Counter on it."
#[test]
fn zushin_places_a_counter_on_a_level_1_normal_monster() {
    let spawns = [at(0, Hand, ZUSHIN), at(0, Monster, SKULL_SERVANT)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(ZUSHIN)]).unwrap();
    clean(&s);
    assert_eq!(counters_on(&s, 0, SKULL_SERVANT), 1, "{}", s.context());
    assert!(!can_activate(&s, ZUSHIN), "once per turn: {}", s.context());
}

/// "Must be Special Summoned (from your hand) by Tributing a monster you control with 10 Zushin Counters.": ten turns of
/// counters on a Skull Servant, then Zushin comes by Tributing it; before that nothing offers the Summon.
#[test]
fn zushin_is_summoned_by_tributing_a_monster_with_ten_counters() {
    let spawns = [at(0, Hand, ZUSHIN), at(0, Monster, SKULL_SERVANT)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, KURIBOH)]); // the end-of-turn discards
    let nine = [Activate(ZUSHIN); 9];
    s.run(&nine).unwrap();
    clean(&s);
    assert_eq!(counters_on(&s, 0, SKULL_SERVANT), 9, "{}", s.context());
    assert!(!listed(&s, "special_summons", ZUSHIN), "nine are not enough: {}", s.context());
    s.run(&[Activate(ZUSHIN)]).unwrap();
    assert_eq!(counters_on(&s, 0, SKULL_SERVANT), 10, "{}", s.context());
    s.run(&[SpSummon(ZUSHIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ZUSHIN), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", SKULL_SERVANT), 1, "the Tribute: {}", s.context());
}

/// "Cannot be Normal Summoned/Set.": it is not offered as a Summon or a Set.
#[test]
fn zushin_cannot_be_normal_summoned_or_set() {
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &[at(0, Hand, ZUSHIN)]) else { return };
    s.run(&[]).unwrap();
    assert!(!listed(&s, "summons", ZUSHIN), "{}", s.context());
    assert!(!listed(&s, "monster_sets", ZUSHIN), "{}", s.context());
}

/// "Unaffected by other cards' effects." and "If this card battles a monster, during damage calculation: ATK/DEF of this
/// card become equal to the current ATK of the monster it is battling +1000": Dark Hole leaves it alone, and attacking
/// Warwolf (2000) it has 3000 ATK and destroys it.
#[test]
fn zushin_is_unaffected_and_takes_the_attack_of_its_opponent_plus_1000() {
    let spawns = [at(0, Monster, ZUSHIN), at(1, Monster, WARWOLF), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(s.count(0, "mzone", ZUSHIN), 1, "Dark Hole: {}", s.context());
    let spawns = [at(0, Monster, ZUSHIN), at(1, Monster, WARWOLF)];
    let mut s = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns).unwrap();
    s.run(&[Attack(ZUSHIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7000, "3000 against 2000: {}", s.context());
}

/// Zushin, anime version: "If this card battles with an opponent's monster, until the end of the Damage Step, its ATK
/// becomes the ATK of the opponent's monster + 1000". The same battle against Warwolf.
#[test]
fn zushin_anime_takes_the_attack_of_its_opponent_plus_1000() {
    let spawns = [at(0, Monster, ZUSHIN), at(1, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Attack(ZUSHIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7000, "{}", s.context());
}
