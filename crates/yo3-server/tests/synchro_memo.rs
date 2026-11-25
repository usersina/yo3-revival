//! `engine/overrides/proc_synchro.lua` is ProjectIgnis' Synchro procedure with its material checks remembered and cut
//! short once the picked materials' Levels sum past the Synchro's: the anime T.G. Cyber Magician's hand materials made the
//! unchanged procedure take minutes. The same boards are played with the unchanged procedure (an engine folder without
//! the override) and with ours: the Synchro Summons the first Main Phase offers, and the first material choice of one of
//! them, must be the same. Random boards of Tuners, non-Tuners, the cards that take materials from the hand or decide the
//! Levels (Eccentric Boy, Malefic Parallel Gear, T.G. Cyber Magician printed and anime, Mono Synchron) and an Extra Deck
//! of Synchro Monsters with their own requirements (two Tuners, a named Tuner or material), anime cards off and on.
//! Boards stay small enough for the unchanged procedure to answer.
use std::path::{Path, PathBuf};

use serde_json::Value as Json;
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

const DARK_HOLE: u16 = 4342; // the Deck's filler: no monster drawn into the hand
const KURIBOH: u16 = 4064;
/// Tuners, non-Tuners and the material rule cards, for the field and the hand.
const MAIN: [u16; 22] = [
    7687, 7870, 8005, 7535, 8933, 9118, // Junk Synchron, Turbo Synchron, Debris Dragon, Flamvell Guard, Effect Veiler, Formula Synchron
    7701, 7688, 7686, 6960, 4054, 4764, 8440, // Quillbolt Hedgehog, Speed Warrior, Sonic Chick, Warwolf, Sangan, Big Shield Gardna, Level Eater
    8580, 9043, 9487, 10309, // Eccentric Boy, Malefic Parallel Gear, T.G. Cyber Magician, Mono Synchron
    9488, 9491, 9492, // T.G. Striker, T.G. Warwolf, T.G. Rush Rhino
    7734, 8137, // Stardust Dragon, Power Tool Dragon (materials of Shooting Star and Life Stream Dragon)
];
const EXTRA: [u16; 22] = [
    7734, 7735, 7898, 8137, 7696, 7733, 8860, 8090, 7697, 7736, 7737, 7738, 7549, 7987, 8652, 7899, 9117, 7552,
    9511, 9512, 9554, 9509, // the T.G. Synchros, Life Stream Dragon
];

/// A copy of the engine folder without our Synchro procedure: the unchanged one from ProjectIgnis. Links, not copies.
fn upstream_dir(dir: &Path) -> PathBuf {
    let out = std::env::temp_dir().join(format!("yo3-synchro-upstream-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("overrides")).unwrap();
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != "overrides" {
            std::os::unix::fs::symlink(std::fs::canonicalize(entry.path()).unwrap(), out.join(entry.file_name())).unwrap();
        }
    }
    for entry in std::fs::read_dir(dir.join("overrides")).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != "proc_synchro.lua" {
            std::os::unix::fs::symlink(std::fs::canonicalize(entry.path()).unwrap(), out.join("overrides").join(entry.file_name())).unwrap();
        }
    }
    out
}

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) % n as u64) as usize
    }
}

/// The board of `seed`: 2 to 4 monsters on the field and up to 4 in the hand.
fn board(seed: u64) -> Vec<Spawn> {
    let mut rng = Rng(seed * 7919 + 17);
    let mut out = vec![];
    for _ in 0..2 + rng.below(3) {
        out.push(Spawn { team: 0, place: Monster, cid: MAIN[rng.below(MAIN.len())] });
    }
    for _ in 0..rng.below(5) {
        out.push(Spawn { team: 0, place: Hand, cid: MAIN[rng.below(MAIN.len())] });
    }
    out
}

fn cids(list: &Json) -> Vec<u64> {
    let mut v: Vec<u64> = list.as_array().map(|a| a.iter().filter_map(|c| c["cid"].as_u64()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// What the board offers: the Synchro Summons of the first Main Phase, and for the `pick`th of them, the first material
/// choice (its kind and the cards it offers). Err: the engine failed (the unchanged procedure taking too long).
/// The Synchro Summons on offer, and the first material choice of one: its message type and the cards it offers.
type Offers = (Vec<u64>, Option<(u64, Vec<u64>)>);

fn offers(dir: &Path, anime: bool, seed: u64, spawns: &[Spawn], pick: usize) -> Result<Offers, String> {
    let decks = [(&[DARK_HOLE; 40][..], &EXTRA[..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = if anime { Scenario::start_anime_field(dir, seed, 8000, decks, spawns) } else { Scenario::start_field(dir, seed, 8000, decks, spawns) }?;
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD && p["player"] == 0)?;
    let summons = cids(&s.prompt()["special_summons"]);
    let synchros: Vec<u64> = summons.iter().copied().filter(|c| EXTRA.contains(&(*c as u16))).collect();
    let Some(&target) = synchros.get(pick % synchros.len().max(1)) else { return Ok((summons, None)) };
    s.run_to(&[SpSummon(target as u16)], |p| p["type"] != msg::SELECT_IDLECMD)?;
    let p = s.prompt().clone();
    let offered = if p["select_cards"].is_array() { cids(&p["select_cards"]) } else { cids(&p["selects"]) };
    Ok((summons, Some((p["type"].as_u64().unwrap_or(0), offered))))
}

#[test]
fn our_synchro_procedure_offers_what_projectignis_offers() {
    let Some(dir) = engine_dir() else { return };
    let upstream = upstream_dir(&dir);
    let boards: u64 = std::env::var("YO3_SYNCHRO_BOARDS").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let (mut compared, mut offered, mut skipped) = (0, 0, vec![]);
    let mut differences = vec![];
    for anime in [false, true] {
        for seed in 1..=boards {
            let spawns = board(seed);
            let pick = seed as usize;
            let theirs = match offers(&upstream, anime, seed, &spawns, pick) {
                Ok(o) => o,
                Err(e) => { skipped.push(format!("anime {anime}, seed {seed}: {e}")); continue }
            };
            let ours = offers(&dir, anime, seed, &spawns, pick).unwrap_or_else(|e| panic!("anime {anime}, seed {seed}: ours failed where ProjectIgnis' answered: {e}"));
            compared += 1;
            if theirs.0.iter().any(|c| EXTRA.contains(&(*c as u16))) {
                offered += 1;
            }
            if ours != theirs {
                differences.push(format!("anime {anime}, seed {seed}, board {:?}:\n  ProjectIgnis {theirs:?}\n  ours         {ours:?}",
                    spawns.iter().map(|s| (s.place, s.cid)).collect::<Vec<_>>()));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&upstream);
    assert!(differences.is_empty(), "{} board(s) differ:\n{}", differences.len(), differences.join("\n"));
    eprintln!("{compared} boards compared, {offered} with a Synchro Summon on offer; skipped (ProjectIgnis' failed): {skipped:?}");
    assert!(offered * 3 >= compared, "most boards should offer a Synchro Summon ({offered} of {compared})");
}
