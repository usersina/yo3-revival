//! The Dark Signers' summons with anime cards on, through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`). The game's Dark Synchro Monsters (Underground Arachnid, Hundred Eyes Dragon, Dark
//! Diviner...) play as their anime versions then, which have only the Dark Synchro procedure
//! (`engine/overrides/proc_synchro.lua`, `Synchro.AddDarkSynchroProcedure`): a Dark Tuner of the anime (setcode 0x600)
//! and a non-Tuner whose Levels' difference is the monster's. The printed Dark Tuners (Catastrogue, Nightmare Hand) are
//! another archetype (0x1d5), so their way to a Dark Synchro Monster is their own effect: Tribute both, Special Summon a
//! Synchro Monster of the Levels' difference from the Extra Deck. The anime Dark Tuners (Doom Submarine, Spider Cocoon,
//! Chaos Rogue, Dark Goddess Witaka, Dark Ape) come with the anime-only batches 32-34.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const NIGHTMARE_HAND: u16 = 5155; // Dark Tuner Nightmare Hand (Level 10)
const CATASTROGUE: u16 = 5154; // Dark Tuner Catastrogue (Level 8)
const UNDERGROUND_ARACHNID: u16 = 8603; // Level 6; anime: a Dark Synchro Monster
const DARK_DIVINER: u16 = 9217; // Level 4; anime: a Dark Synchro Monster
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, no effect
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;

fn duel(decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// Nightmare Hand (Level 10) and a Level 4 non-Tuner, anime cards on: its effect brings the Level 6 Underground
/// Arachnid, an anime Dark Synchro Monster, from the Extra Deck, and the client sees it.
#[test]
fn nightmare_hand_brings_an_anime_dark_synchro_monster() {
    let spawns = [Spawn { team: 0, place: Hand, cid: NIGHTMARE_HAND }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel([(&[DARK_HOLE; 40], &[UNDERGROUND_ARACHNID]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.run(&[Summon(NIGHTMARE_HAND), Activate(NIGHTMARE_HAND)]).unwrap();
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
    assert!(s.monster(0, UNDERGROUND_ARACHNID).is_some(), "Underground Arachnid came: {}", s.context());
    assert_eq!(s.count(0, "grave", NIGHTMARE_HAND), 1, "{}", s.context());
}

/// Catastrogue (Level 8) and a Level 4 non-Tuner, anime cards on: the Level 4 Dark Diviner, an anime Dark Synchro
/// Monster.
#[test]
fn catastrogue_brings_an_anime_dark_synchro_monster() {
    let spawns = [Spawn { team: 0, place: Hand, cid: CATASTROGUE }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Monster, cid: KURIBOH }];
    let Some(mut s) = duel([(&[DARK_HOLE; 40], &[DARK_DIVINER]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.prefer(&[(1, KURIBOH)]);
    s.run(&[Summon(CATASTROGUE), Activate(CATASTROGUE)]).unwrap();
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
    assert!(s.monster(0, DARK_DIVINER).is_some(), "Dark Diviner came: {}", s.context());
}

/// Without a Dark Tuner of the anime, no Dark Synchro Summon is offered: the printed Nightmare Hand is a Tuner of
/// another archetype, so Underground Arachnid can't be Synchro Summoned with it (the anime Dark Tuners, batches 32-34,
/// will).
#[test]
fn a_printed_dark_tuner_is_no_material_for_a_dark_synchro_summon() {
    let spawns = [Spawn { team: 0, place: Monster, cid: NIGHTMARE_HAND }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel([(&[DARK_HOLE; 40], &[UNDERGROUND_ARACHNID]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    assert!(s.run(&[SpSummon(UNDERGROUND_ARACHNID)]).is_err(), "no Dark Synchro Summon without an anime Dark Tuner: {}", s.context());
    assert!(s.monster(0, UNDERGROUND_ARACHNID).is_none());
}
