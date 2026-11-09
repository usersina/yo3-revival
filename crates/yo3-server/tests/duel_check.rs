//! The duel check (`duelcheck.rs`): self-played duels through the rules engine, two seeded random players with seeded
//! decks from a pool of searches, discards, control changes, swaps, flips, tokens, Synchro and Xyz Summons, the client's
//! view (a model of its card manager, fed the commands the translator sends) compared with the engine after every engine
//! step. A difference fails with the seed, the step and the engine messages before it. Skipped unless engine/fetch.sh
//! has run. `YO3_CHECK_DUELS=n` plays n duels per test instead of 30 (8 tests).
use std::collections::BTreeMap;
use std::path::PathBuf;

use yo3_server::duelcheck::selfplay;

fn engine_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    Some(dir)
}

/// Plays one seed: two decks of its own, the client on alternating sides, and as against a player (the opponent's hidden
/// cards without rows) every third seed. Returns what the duel did, or its first difference.
fn play(dir: &std::path::Path, seed: u64) -> Result<selfplay::Played, String> {
    let (a, b) = (selfplay::deck(seed * 2), selfplay::deck(seed * 2 + 1));
    let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
    selfplay::play(dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3))
}

/// The `part`th share of the seeds, all played; fails listing every duel whose client view left the engine's.
fn run(part: u64) {
    let Some(dir) = engine_dir() else { return };
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let failures: Vec<String> = (part * per..(part + 1) * per).filter_map(|seed| play(&dir, seed).err()).collect();
    assert!(failures.is_empty(), "{} of {per} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn duels_0() {
    run(0);
}

#[test]
fn duels_1() {
    run(1);
}

#[test]
fn duels_2() {
    run(2);
}

#[test]
fn duels_3() {
    run(3);
}

#[test]
fn duels_4() {
    run(4);
}

#[test]
fn duels_5() {
    run(5);
}

#[test]
fn duels_6() {
    run(6);
}

#[test]
fn duels_7() {
    run(7);
}

#[test]
fn the_seeds_reach_the_cases_that_drifted() {
    // Seeds that reach the cases where the duel check first found the client's view leaving the engine's, kept so they
    // stay covered: a hand shuffle after a search (33; the client crash at 0x516185) and Magical Hats' Set card shuffle
    // (36) in seed 1, Creature Swap (55) and an Xyz Summon's materials in seed 36, Exchange of the Spirit (35) in seed 152;
    // and the links shown as hover markers, which must never join an empty slot: a lasting target (96) in seed 4, an
    // equip (93) in seed 8.
    let Some(dir) = engine_dir() else { return };
    let mut kinds: BTreeMap<u64, u32> = BTreeMap::new();
    let mut materials = 0;
    for seed in [1, 4, 8, 36, 152] {
        let played = play(&dir, seed).unwrap_or_else(|e| panic!("{e}"));
        materials += played.materials;
        for (k, n) in played.kinds {
            *kinds.entry(k).or_default() += n;
        }
    }
    for (kind, what) in [(33, "a hand shuffle"), (35, "a Graveyard and Deck swap"), (36, "a Set card shuffle"), (55, "a control swap"), (93, "an equip"), (96, "a lasting target")] {
        assert!(kinds.contains_key(&kind), "no {what} ({kind}) in the seeds: {kinds:?}");
    }
    assert!(materials > 0, "no Xyz material moved in the seeds");
}
