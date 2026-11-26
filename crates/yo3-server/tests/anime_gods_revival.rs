//! The Gods revived from the Graveyard with anime cards on (their manga versions, 513000134-136, through ProjectIgnis'
//! Divine Hierarchy procedure, `engine/overrides/proc_unofficial.lua`): "During the End Phase, if this card was Special
//! Summoned, return it to where it was Special Summoned from." The anime Call of the Haunted ("Special Summon any number
//! of monsters in your Graveyard") brings Obelisk, Slifer and Ra back at once. Both seats' views are checked after every
//! step.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const OBELISK: u16 = 4998;
const SLIFER: u16 = 4999;
const RA: u16 = 5000;
const CALL_OF_THE_HAUNTED: u16 = 4989; // a Spell with anime cards on
const MONSTER_REBORN: u16 = 4842;
const FILLER: u16 = 4012; // Feral Imp

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn field(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    if !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() {
        return None;
    }
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    // The server's default revival rule (graveyard-is-graveyard): a God never Normal Summoned can come back.
    Some(Scenario::start_rule(&dir, yo3_server::engine::Revival::GraveyardIsGraveyard, true, 7, 8000, decks, spawns).unwrap())
}

/// Call of the Haunted brings all three back: Obelisk 4000, Slifer 1000 per card in hand (6000) and also a Dragon, Ra
/// with its "Special Summoned from the Graveyard" choice (the harness takes the first: pay LP down to 1, gain that much
/// ATK, 7999), each a Zombie with DEF 0 by Call of the Haunted's text. At the End Phase each returns to the Graveyard.
#[test]
fn call_of_the_haunted_revives_the_three_gods_until_the_end_phase() {
    let spawns = [at(0, Grave, OBELISK), at(0, Grave, SLIFER), at(0, Grave, RA), at(0, Hand, CALL_OF_THE_HAUNTED)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, OBELISK), (0, SLIFER), (0, RA)]);
    s.prefer_most(true);
    s.run(&[Activate(CALL_OF_THE_HAUNTED)]).unwrap();
    clean(&s);
    let stats = |s: &Scenario, cid| s.monster(0, cid).map(|m| (m["attack"].as_i64(), m["defense"].as_i64(), m["race"].as_i64()))
        .unwrap_or_else(|| panic!("{cid} on the field: {}", s.context()));
    const ZOMBIE: i64 = 0x10;
    const DRAGON: i64 = 0x2000;
    assert_eq!(stats(&s, OBELISK), (Some(4000), Some(0), Some(ZOMBIE)), "{}", s.context());
    assert_eq!(stats(&s, SLIFER), (Some(6000), Some(0), Some(ZOMBIE | DRAGON)), "{}", s.context());
    assert_eq!(stats(&s, RA), (Some(7999), Some(0), Some(ZOMBIE)), "{}", s.context());
    assert_eq!(s.lp(0), 1, "Ra's LP payment: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    for god in [OBELISK, SLIFER, RA] {
        assert_eq!((s.count(0, "mzone", god), s.count(0, "grave", god)), (0, 1), "{god} back in the Graveyard: {}", s.context());
    }
}

/// Monster Reborn brings Obelisk back (the server's revival rule lets a God never Normal Summoned come back); it returns to
/// the Graveyard at the End Phase.
#[test]
fn monster_reborn_revives_obelisk_until_the_end_phase() {
    let spawns = [at(0, Grave, OBELISK), at(0, Hand, MONSTER_REBORN)];
    let Some(mut s) = field(&spawns) else { return };
    s.run(&[Activate(MONSTER_REBORN)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, OBELISK), Some(4000), "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", OBELISK), s.count(0, "grave", OBELISK)), (0, 1), "{}", s.context());
}
