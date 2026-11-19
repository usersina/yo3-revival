//! Junk Warrior (7696): "When this card is Synchro Summoned: it gains ATK equal to the total ATK of all Level 2 or lower
//! monsters you currently control." The materials have left the field by then, so only other Level 2 or lower
//! monsters count. Real core, both seats' client models checked after every step (`duelcheck::scenario`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};

const JUNK_WARRIOR: u16 = 7696;
const JUNK_SYNCHRON: u16 = 7687;
const HEDGEHOG: u16 = 7701; // Quillbolt Hedgehog: Level 2, 800 ATK
const WARWOLF: u16 = 6960;

/// Normal Summons `hedgehogs` Quillbolt Hedgehogs and a Junk Synchron over the turns, then Synchro Summons Junk Warrior
/// from Junk Synchron and one Hedgehog. Returns Junk Warrior's ATK.
fn junk_warrior_after(hedgehogs: usize) -> Option<i64> {
    let dir = engine_dir()?;
    let deck: Vec<u16> = [HEDGEHOG, HEDGEHOG, JUNK_SYNCHRON].iter().copied().cycle().take(40).collect();
    let mut s = Scenario::start(&dir, 7, 8000, [(&deck, &[JUNK_WARRIOR]), (&[WARWOLF; 40], &[])]).unwrap();
    let mut plan = vec![Summon(HEDGEHOG); hedgehogs];
    plan.extend([Summon(JUNK_SYNCHRON), SpSummon(JUNK_WARRIOR)]);
    s.run(&plan).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    Some(s.attack_of(0, JUNK_WARRIOR).expect("Junk Warrior is on the field"))
}

#[test]
fn junk_warrior_alone_keeps_its_attack() {
    let Some(atk) = junk_warrior_after(1) else { return };
    assert_eq!(atk, 2300, "the materials have left the field");
}

#[test]
fn junk_warrior_gains_a_level_2_monster_left_on_the_field() {
    let Some(atk) = junk_warrior_after(2) else { return };
    assert_eq!(atk, 2300 + 800, "one Hedgehog stayed on the field");
}
