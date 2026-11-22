//! Cards the game never had (#75), in duels: the first, Shooting Quasar Dragon (Konami id 9709), Synchro Summoned from
//! the game's own cards through the real core, with both seats' client models checked after every step
//! (`duelcheck::scenario`). Its materials, all in the client: Ally of Justice Catastor twice (Junk Synchron + Quillbolt
//! Hedgehog, Level 5 each) and Formula Synchron (Effect Veiler + Kuriboh, a Level 2 Synchro Tuner): 5 + 5 + 2 = 12.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};

const QUASAR: u16 = 9709;
const CATASTOR: u16 = 7552;
const FORMULA: u16 = 9118;
const JUNK_SYNCHRON: u16 = 7687;
const HEDGEHOG: u16 = 7701;
const VEILER: u16 = 8933; // Effect Veiler: a Level 1 Tuner
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const SHOOTING_STAR: u16 = 9117;

/// Quasar Synchro Summoned on the planned side's field (each side with 10000 LP), against a passive opponent holding only
/// Dark Holes; the scenario to go on from, in the planned side's Main Phase 1.
fn quasar_on_the_field() -> Option<Scenario> {
    let dir = engine_dir()?;
    let deck: Vec<u16> = [KURIBOH, VEILER, JUNK_SYNCHRON, HEDGEHOG].iter().copied().cycle().take(40).collect();
    let extra = [CATASTOR, CATASTOR, FORMULA, QUASAR, SHOOTING_STAR];
    let mut s = Scenario::start(&dir, 11, 10000, [(&deck, &extra), (&[DARK_HOLE; 40], &[])]).unwrap();
    s.run(&[
        Summon(HEDGEHOG), EndTurn,
        Summon(JUNK_SYNCHRON), SpSummon(CATASTOR), EndTurn,
        Summon(HEDGEHOG), EndTurn,
        Summon(JUNK_SYNCHRON), SpSummon(CATASTOR), EndTurn,
        Summon(KURIBOH), EndTurn,
        Summon(VEILER), SpSummon(FORMULA), SpSummon(QUASAR),
    ]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    Some(s)
}

#[test]
fn shooting_quasar_dragon_is_synchro_summoned_from_the_games_cards() {
    let Some(s) = quasar_on_the_field() else { return };
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "Shooting Quasar Dragon is on the field with 4000 ATK");
    assert_eq!(s.attack_of(0, CATASTOR), None, "both Catastors were its materials");
}

#[test]
fn it_attacks_once_for_each_non_tuner_used_for_it() {
    // "can attack a number of times each Battle Phase, up to the number of non-Tuners used for its Synchro Summon": the two
    // Catastors, so two direct attacks of 4000.
    let Some(mut s) = quasar_on_the_field() else { return };
    s.run(&[Attack(QUASAR), Attack(QUASAR)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.lp(1), 10000 - 2 * 4000, "two attacks");
}

#[test]
fn it_negates_and_destroys_an_activation_then_brings_shooting_star_dragon_when_it_leaves() {
    // "Once per turn, when a card or effect is activated: negate the activation, and if you do, destroy it": the first Dark
    // Hole. The second, the same turn, destroys it: "When this face-up card leaves the field: Special Summon 1 Shooting Star
    // Dragon from your Extra Deck".
    let Some(mut s) = quasar_on_the_field() else { return };
    s.run_both([&[EndTurn, Chain(QUASAR)], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "the first Dark Hole was negated");
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), None, "the second destroyed it");
    assert_eq!(s.attack_of(0, SHOOTING_STAR), Some(3300), "and Shooting Star Dragon came from the Extra Deck");
}

/// The spawn testing aid (`Scenario::start_field`, the server's `--duel-field`): Quasar starts on the field and the opponent
/// holds Dark Hole, so the negation and Shooting Star Dragon need no nine turns of summoning. Both seats' client models
/// follow the moves from the first step.
#[test]
fn a_spawned_quasar_negates_and_brings_shooting_star_dragon() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: QUASAR }, Spawn { team: 1, place: Hand, cid: DARK_HOLE }, Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
    let mut s = Scenario::start_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[SHOOTING_STAR]), (&[KURIBOH; 40], &[])], &field).unwrap();
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "on the field from the start");
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    s.run_both([&[EndTurn, Chain(QUASAR)], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "the first Dark Hole was negated");
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!((s.attack_of(0, QUASAR), s.attack_of(0, SHOOTING_STAR)), (None, Some(3300)));
}
