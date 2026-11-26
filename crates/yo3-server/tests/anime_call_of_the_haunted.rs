//! The anime Call of the Haunted (4989 with anime cards on, ProjectIgnis' 511002048 and our override
//! `engine/overrides/c511002048.lua`): "If a monster you control whose original Type is not Zombie is destroyed by battle
//! and sent to the Graveyard, Special Summon it in face-up Attack Position as a Zombie-Type with DEF 0 and increase its
//! ATK by 10%." The increase is of the ATK the monster had when it was destroyed, boosts included, so each revival grows
//! from the last (upstream took the ATK it had in the Graveyard, its printed one). Both seats' views are checked after
//! every step.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const CALL_OF_THE_HAUNTED: u16 = 4989;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const AXE_OF_DESPAIR: u16 = 4310; // an Equip Spell, +1000 ATK
const BLUE_EYES: u16 = 4007; // 3000 ATK
const OBEDIENT_FIEND: u16 = 5666; // Ultimate Obedient Fiend: 3500 ATK
const FILLER: u16 = 4012; // Feral Imp

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// Warwolf with Axe of Despair (3000) trades with Blue-Eyes (3000): it comes back at 3300, not 2200 (10% of its printed
/// 2000). It then attacks Ultimate Obedient Fiend (3500), is destroyed again, and comes back at 3630: 10% more than the
/// 3300 it had.
#[test]
fn anime_call_of_the_haunted_raises_the_atk_the_monster_had_by_10_percent_each_time() {
    let Some(dir) = engine_dir() else { return };
    if !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() {
        return;
    }
    let spawns = [at(0, Monster, WARWOLF), at(0, Hand, AXE_OF_DESPAIR), at(0, Spell, CALL_OF_THE_HAUNTED), at(1, Monster, BLUE_EYES),
        at(1, Monster, OBEDIENT_FIEND)];
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(AXE_OF_DESPAIR), EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "Blue-Eyes traded with the 3000 Warwolf: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(3300), "back with 10% more than the 3000 it had: {}", s.context());
    let warwolf = s.monster(0, WARWOLF).unwrap().clone();
    assert_eq!(warwolf["defense"].as_i64(), Some(0), "{}", s.context());
    s.prefer(&[(1, OBEDIENT_FIEND)]);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(3630), "back again with 10% more than its 3300: {}", s.context());
}
