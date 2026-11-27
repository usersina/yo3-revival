//! The Earthbound Immortals' cards the Dark Signers play (Rex Goodwin's and Greiger's decks), with anime cards on,
//! through the real core with both seats' client models checked after every step (`duelcheck::scenario`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const WASTELAND: u16 = 4337;
const URU: u16 = 8460; // Earthbound Immortal Uru (Level 10)
const EARTHBOUND_RELEASE: u16 = 5014;
const SUMMONED_SKULL: u16 = 4028; // Level 6
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, no effect
const DARK_HOLE: u16 = 4342;

/// Earthbound Release (anime): "When your opponent Summons a Level 6 or higher monster: Tribute 1 "Earthbound
/// Immortal" monster; destroy all monsters your opponent controls, and inflict damage equal to their total ATK."
/// The opponent Tribute Summons Summoned Skull; Uru is Tributed, the Skull destroyed, its ATK dealt. Upstream's script
/// raised a Lua error checking the cost (`engine/overrides/c100000430.lua`).
#[test]
fn earthbound_release_tributes_an_immortal_against_a_level_6_summon() {
    let Some(dir) = engine_dir() else { return };
    let spawns = [Spawn { team: 0, place: Hand, cid: WASTELAND }, Spawn { team: 0, place: Hand, cid: URU },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: SpellSet, cid: EARTHBOUND_RELEASE },
        Spawn { team: 1, place: Hand, cid: SUMMONED_SKULL }, Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, [(&[DARK_HOLE; 40], &[]), (&[DARK_HOLE; 40], &[])], &spawns).unwrap();
    s.run_both([&[Activate(WASTELAND), Summon(URU), EndTurn, Chain(EARTHBOUND_RELEASE)], &[Summon(SUMMONED_SKULL)]]).unwrap();
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
    assert_eq!(s.count(1, "grave", SUMMONED_SKULL), 1, "the Skull was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", URU), 1, "Uru was Tributed for the cost: {}", s.context());
    assert!(s.lp(1) < 8000, "its ATK was dealt: {}", s.context());
}
