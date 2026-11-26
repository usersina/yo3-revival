//! The anime Call of the Haunted (4989 with anime cards on, ProjectIgnis' 511002048 and our override
//! `engine/overrides/c511002048.lua`): "If a monster you control whose original Type is not Zombie is destroyed by battle
//! and sent to the Graveyard, Special Summon it in face-up Attack Position as a Zombie-Type with DEF 0 and increase its
//! ATK by 10%." The 10% grows the monster's own ATK: its printed ATK and the increases Call of the Haunted gave it before.
//! A boost from another card (an Equip Spell) falls off with that card when the monster is destroyed, and one given after
//! the revival applies on top. Both seats' views are checked after every step.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const CALL_OF_THE_HAUNTED: u16 = 4989;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const AXE_OF_DESPAIR: u16 = 4310; // an Equip Spell, +1000 ATK
const BLUE_EYES: u16 = 4007; // 3000 ATK
const OBEDIENT_FIEND: u16 = 5666; // Ultimate Obedient Fiend: 3500 ATK
const FILLER: u16 = 4012; // Feral Imp
const MYSTIK_WOK: u16 = 5978; // Tribute 1 monster: gain LP
const MONSTER_REBORN: u16 = 4842;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// Warwolf (2000) with Axe of Despair (3000) trades with Blue-Eyes (3000): it comes back at 2200, 10% over its own 2000,
/// the Axe gone. A second Axe gives it 3200 on top; it attacks Ultimate Obedient Fiend (3500), is destroyed again, and comes
/// back at 2420: 10% over its own 2200, not over the 3200 the Axe made.
#[test]
fn anime_call_of_the_haunted_grows_the_monsters_own_atk_by_10_percent_each_time() {
    let Some(dir) = engine_dir() else { return };
    if !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() {
        return;
    }
    let spawns = [at(0, Monster, WARWOLF), at(0, Hand, AXE_OF_DESPAIR), at(0, SpellSet, AXE_OF_DESPAIR), at(0, Spell, CALL_OF_THE_HAUNTED),
        at(1, Monster, BLUE_EYES), at(1, Monster, OBEDIENT_FIEND)];
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(AXE_OF_DESPAIR), EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "Blue-Eyes traded with the 3000 Warwolf: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2200), "back with 10% over its own 2000, the Axe gone: {}", s.context());
    let warwolf = s.monster(0, WARWOLF).unwrap().clone();
    assert_eq!(warwolf["defense"].as_i64(), Some(0), "{}", s.context());
    s.run_both([&[EndTurn, Activate(AXE_OF_DESPAIR)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(3200), "a new Axe applies on top: {}", s.context());
    s.prefer(&[(1, OBEDIENT_FIEND)]);
    s.run_both([&[Attack(WARWOLF)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", AXE_OF_DESPAIR), 2, "both Axes gone: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2420), "back again with 10% over its own 2200: {}", s.context());
}

/// A monster brought back another way starts from its printed ATK again: the 2200 Warwolf Call of the Haunted revived is
/// Tributed for Mystik Wok, Monster Reborn brings it back at 2000, and when Blue-Eyes destroys it in battle it comes back at
/// 2200, not 2420.
#[test]
fn anime_call_of_the_haunted_forgets_a_revival_after_another_way_back() {
    let Some(dir) = engine_dir() else { return };
    if !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() {
        return;
    }
    let spawns = [at(0, Monster, WARWOLF), at(0, Spell, CALL_OF_THE_HAUNTED), at(0, SpellSet, MYSTIK_WOK), at(0, SpellSet, MONSTER_REBORN),
        at(1, Monster, BLUE_EYES), at(1, Monster, BLUE_EYES)];
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2200), "{}", s.context());
    s.run_both([&[EndTurn, Activate(MYSTIK_WOK), Activate(MONSTER_REBORN)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "Monster Reborn brings back the printed Warwolf: {}", s.context());
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2200), "10% over its printed 2000 again: {}", s.context());
}
