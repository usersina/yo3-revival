//! The duel check (`duelcheck.rs`): self-played duels through the rules engine, two seeded random players with seeded
//! decks from a pool of searches, discards, control changes, swaps, flips, tokens, Fusion, Synchro and Xyz Summons, the client's
//! view (a model of its card manager, fed the commands the translator sends) compared with the engine after every engine
//! step. A difference fails with the seed, the step and the engine messages before it. Skipped unless engine/fetch.sh
//! has run. `YO3_CHECK_DUELS=n` plays n duels per test instead of 30 (8 tests); `YO3_CHECK_SEED=s` replays seed s alone.
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

/// Writes this part's effect coverage (#67) for `tools/duel/coverage.py`: per card, its chain links activated,
/// resolved, resolved with a visible result, negated, and its summons, with the deck pool and the number of duels.
fn coverage(part: u64, duels: u64, cards: &BTreeMap<u16, [u32; 5]>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("duel-coverage");
    let cards: serde_json::Map<String, serde_json::Value> = cards
        .iter()
        .map(|(cid, [a, r, v, n, s])| (cid.to_string(), serde_json::json!({"activated": a, "resolved": r, "visible": v, "negated": n, "summoned": s})))
        .collect();
    let report = serde_json::json!({"duels": duels, "pool": selfplay::pool(), "cards": cards});
    let written = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(format!("part-{part}.json")), report.to_string()));
    if let Err(e) = written {
        eprintln!("effect coverage not written to {}: {e}", dir.display());
    }
}

/// The `part`th share of the seeds, all played; fails listing every duel whose client view left the engine's.
fn run(part: u64) {
    let Some(dir) = engine_dir() else { return };
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let mut failures = vec![];
    let mut cards: BTreeMap<u16, [u32; 5]> = BTreeMap::new();
    let mut duels = 0;
    // `YO3_CHECK_SEED=s` replays seed s alone (in the first part).
    let seeds = match std::env::var("YO3_CHECK_SEED").ok().and_then(|n| n.parse::<u64>().ok()) {
        Some(seed) if part == 0 => seed..seed + 1,
        Some(_) => return,
        None => part * per..(part + 1) * per,
    };
    for seed in seeds {
        match play(&dir, seed) {
            Ok(played) => {
                duels += 1;
                for (i, counts) in [&played.activated, &played.resolved, &played.visible, &played.negated, &played.summoned].into_iter().enumerate() {
                    for (&cid, &n) in counts {
                        cards.entry(cid).or_default()[i] += n;
                    }
                }
            }
            Err(e) => failures.push(e),
        }
    }
    coverage(part, duels, &cards);
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
    // equip (93) in seed 8. A Fusion Summon with both materials from the hand (issue #40: the client crashed at
    // 0x4FC4A9 when both materials' effects came before their moves) in seed 69, and a Synchro Summon in seed 152. The
    // position of a token about to appear, asked of the client (Gorz the Emissary of Darkness's froze the duel: a dialog
    // with no card to click), twice in seed 28. A phase's own window with something to activate, shown on the default
    // Activation Message (#53: its optional triggers come in that chain question).
    let Some(dir) = engine_dir() else { return };
    let mut kinds: BTreeMap<u64, u32> = BTreeMap::new();
    let mut animations: BTreeMap<u32, u32> = BTreeMap::new();
    let (mut materials, mut hand_materials, mut token_positions, mut phase_windows, mut field_targets) = (0, 0, 0, 0, 0);
    for seed in [1, 4, 8, 28, 36, 69, 152] {
        let played = play(&dir, seed).unwrap_or_else(|e| panic!("{e}"));
        materials += played.materials;
        hand_materials += played.hand_materials;
        token_positions += played.token_positions;
        phase_windows += played.phase_windows;
        field_targets += played.field_targets;
        for (k, n) in played.kinds {
            *kinds.entry(k).or_default() += n;
        }
        for (k, n) in played.animations {
            *animations.entry(k).or_default() += n;
        }
    }
    for (kind, what) in [(33, "a hand shuffle"), (35, "a Graveyard and Deck swap"), (36, "a Set card shuffle"), (55, "a control swap"), (93, "an equip"), (96, "a lasting target")] {
        assert!(kinds.contains_key(&kind), "no {what} ({kind}) in the seeds: {kinds:?}");
    }
    assert!(materials > 0, "no Xyz material moved in the seeds");
    assert!(animations.contains_key(&0) && hand_materials >= 2, "no Fusion from the hand in the seeds: {animations:?}, {hand_materials}");
    assert!(animations.contains_key(&2), "no Synchro Summon in the seeds: {animations:?}");
    assert!(token_positions > 0, "no token's position asked of the client in the seeds");
    assert!(phase_windows > 0, "no phase window with something to activate reached the client in the seeds");
    assert!(field_targets > 0, "no attack target was picked on the field in the seeds");
}

/// The same self-play with the anime card versions on (#60, the `anime-cards` setting): decks of the cards the reviewed
/// map approves, so that their anime scripts run through every kind of prompt and the client's view must still follow the
/// engine. `YO3_CHECK_DUELS=n` plays n duels instead of 30; `YO3_CHECK_ANIME_SEED=s` replays seed s alone.
/// Self-play with decks full of the cards the game never had (#75, `engine/added-cards.json`): every added card the
/// engine plays, topped up with the ordinary pool's, so random duels exercise their effects with the client's view checked
/// after every step. The anime versions off (the printed added cards), then on (every added card, the anime-only ones
/// too). `YO3_CHECK_DUELS=n` plays n duels each instead of 20.
#[test]
fn duels_with_the_added_cards() {
    let Some(dir) = engine_dir() else { return };
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(20);
    let mut failures = vec![];
    let added: std::collections::BTreeSet<u16> = yo3_data::added::Added::load(&dir).unwrap().cards.iter().map(|c| c.cid).collect();
    let mut activated: BTreeMap<u16, u32> = BTreeMap::new();
    for anime in [false, true] {
        if anime && yo3_server::engine::anime_problem(&dir).is_some() {
            continue;
        }
        let pool = selfplay::added_pool(&dir, anime, 8).expect("the added cards");
        for seed in 900..900 + per {
            let (a, b) = (selfplay::anime_deck(&pool, seed * 2), selfplay::anime_deck(&pool, seed * 2 + 1));
            let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
            let played = if anime { selfplay::play_anime(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3)) }
                else { selfplay::play(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3)) };
            match played {
                Ok(p) => for (cid, n) in p.activated.iter().chain(&p.summoned) {
                    if added.contains(cid) { *activated.entry(*cid).or_default() += n }
                },
                Err(e) => failures.push(format!("anime {anime}: {e}")),
            }
        }
    }
    assert!(failures.is_empty(), "{} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
    eprintln!("added cards activated or summoned: {activated:?}");
    assert!(activated.len() >= 3, "the added cards were played: {activated:?}");
}

/// The Gods with anime cards on, among the cards that test them most: the six manga Gods, Sphere Mode, Ra's Disciple and
/// the Orichalcos line that brings the anime Divine Serpent Geh, cheap Tributes, and removal, control and LP cards
/// (Book of Moon, Shrink, Change of Heart, Creature Swap, Enemy Controller, Mystik Wok, Ring of Destruction, Skill
/// Drain...). Each duel two random decks of them, the client's view checked against the engine after every step.
/// `YO3_CHECK_DUELS=n` plays n duels instead of 20.
#[test]
fn duels_with_the_gods() {
    let Some(dir) = engine_dir() else { return };
    if yo3_server::engine::anime_problem(&dir).is_some() {
        return;
    }
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(20);
    let pool = selfplay::AnimePool {
        // The Gods, Sphere Mode, Ra's Disciple, Orichalcos Kyutora (Shunoros, then Geh), and Tributes.
        monsters: vec![4998, 4999, 5000, 6405, 7091, 7092, 11927, 11169, 3984, 6960, 5586, 4064, 4054, 6390, 6000, 5526],
        // Book of Moon, Shrink, Change of Heart, Creature Swap, Enemy Controller, Monster Reborn, Dark Hole, Raigeki,
        // Mystik Wok, Snatch Steal, Brain Control, Soul Exchange, Premature Burial, Scapegoat, Smashing Ground, Fissure,
        // Lightning Vortex, Nobleman of Crossout, Swords of Revealing Light, The Seal of Orichalcos, Illusion Gate,
        // Underworld Circle.
        spells: vec![5432, 5355, 4860, 5162, 5505, 4842, 4342, 4343, 5978, 4898, 4803, 5099, 4966, 4818, 5905, 4835, 5217,
            4963, 4354, 10327, 3976, 3978],
        // Mirror Force, Raigeki Break, Ring of Destruction, Skill Drain, Torrential Tribute, Call of the Haunted,
        // Bottomless Trap Hole, Compulsory Evacuation Device, Dimensional Prison, Solemn Judgment, Arduous Decision.
        traps: vec![4887, 5544, 5005, 5740, 5114, 4989, 5400, 5914, 7400, 4861, 3980],
        extra: vec![],
    };
    let mut failures = vec![];
    let mut played: BTreeMap<u16, u32> = BTreeMap::new();
    for seed in 1300..1300 + per {
        let (a, b) = (selfplay::anime_deck(&pool, seed * 2), selfplay::anime_deck(&pool, seed * 2 + 1));
        let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
        match selfplay::play_anime(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3)) {
            Ok(p) => for (cid, n) in p.activated.iter().chain(&p.summoned) { *played.entry(*cid).or_default() += n },
            Err(e) => failures.push(format!("seed {seed}: {e}")),
        }
    }
    assert!(failures.is_empty(), "{} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
    eprintln!("cards activated or summoned: {played:?}");
    let gods = [4998, 4999, 5000, 6405, 7091, 7092].iter().filter(|g| played.contains_key(g)).count();
    assert!(gods >= 2, "Gods were summoned: {played:?}");
}

/// Both players at ∞ LP (the engine's one ∞ value, `guards.lua`), with anime cards on, among the cards that pay, give,
/// take or set LP (Dian Keto, Premature Burial, Mystik Wok, Yu-Jo Friendship, Reversal Quiz, Solemn Judgment, Life
/// Equalizer, Self-Destruct Button, Wall of Revealing Light, Hope for Escape, Riryoku...), change ATK (Axe of Despair,
/// Megamorph, Shadow Spell) or deal ∞ (the Gods, Ring of Destruction): finite changes leave ∞ alone, and the client's
/// view is checked against the engine after every step. `YO3_CHECK_DUELS=n` plays n duels instead of 12.
#[test]
fn duels_at_infinite_lp() {
    let Some(dir) = engine_dir() else { return };
    if yo3_server::engine::anime_problem(&dir).is_some() {
        return;
    }
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(12);
    let pool = selfplay::AnimePool {
        // Obelisk, Slifer, Ra, Avatar, the Gene-Warped Warwolf, Kuriboh and other cheap Tributes.
        monsters: vec![4998, 4999, 5000, 7091, 6960, 5586, 4064, 4054, 6390, 6000, 5526],
        // Dian Keto, Premature Burial, Mystik Wok, Yu-Jo Friendship, Reversal Quiz, Megamorph, Riryoku, Monster Reborn,
        // Axe of Despair, De-Fusion, Dark Hole.
        spells: vec![4348, 4966, 5978, 5754, 5541, 4663, 5009, 4842, 4310, 5118, 4342],
        // Solemn Judgment, Life Equalizer, Self-Destruct Button, Wall of Revealing Light, Hope for Escape, Ring of
        // Destruction, Shadow Spell, Mirror Force.
        traps: vec![4861, 6708, 5916, 5990, 9354, 5005, 4675, 4887],
        extra: vec![],
    };
    let lp_cards = [4348u16, 4966, 5978, 5754, 5541, 5009, 4861, 6708, 5916, 5990, 9354];
    let mut failures = vec![];
    let mut played: BTreeMap<u16, u32> = BTreeMap::new();
    for seed in 1500..1500 + per {
        let (a, b) = (selfplay::anime_deck(&pool, seed * 2), selfplay::anime_deck(&pool, seed * 2 + 1));
        let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
        match selfplay::play_anime_at(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3), 19_999_998) {
            Ok(p) => for (cid, n) in p.activated.iter().chain(&p.summoned) { *played.entry(*cid).or_default() += n },
            Err(e) => failures.push(format!("seed {seed}: {e}")),
        }
    }
    assert!(failures.is_empty(), "{} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
    eprintln!("cards activated or summoned: {played:?}");
    let used = lp_cards.iter().filter(|c| played.contains_key(c)).count();
    assert!(used >= 4, "LP cards were played at ∞: {played:?}");
}

/// The self-play under the broadest `revival-rule` house rule, `graveyard-is-graveyard` (Monster Reborn and every revival
/// reach monsters never properly Summoned): the client's view must follow the engine through those revivals too.
/// `YO3_CHECK_DUELS=n` plays n duels instead of 30.
#[test]
fn duels_with_graveyard_is_graveyard() {
    let Some(dir) = engine_dir() else { return };
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let mut failures = vec![];
    for seed in 700..700 + per {
        let (a, b) = (selfplay::deck(seed * 2), selfplay::deck(seed * 2 + 1));
        let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
        if let Err(e) = selfplay::play_revival(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3),
            yo3_server::engine::Revival::GraveyardIsGraveyard) {
            failures.push(e);
        }
    }
    assert!(failures.is_empty(), "{} of {per} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn duels_with_the_anime_versions_on() {
    let Some(dir) = engine_dir() else { return };
    if yo3_server::engine::anime_problem(&dir).is_some() {
        eprintln!("anime data not fetched; skipping");
        return;
    }
    let pool = selfplay::anime_pool(&dir).expect("the approved cards");
    assert!(pool.monsters.len() > 50 && pool.spells.len() > 30 && pool.traps.len() > 20 && !pool.extra.is_empty(), "{pool:?}");
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(30);
    let mut failures = vec![];
    let mut activated = 0;
    let only: Option<u64> = std::env::var("YO3_CHECK_ANIME_SEED").ok().and_then(|n| n.parse().ok());
    for seed in only.map_or(500..500 + per, |s| s..s + 1) {
        let (a, b) = (selfplay::anime_deck(&pool, seed * 2), selfplay::anime_deck(&pool, seed * 2 + 1));
        let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
        match selfplay::play_anime(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3)) {
            Ok(played) => activated += played.activated.values().sum::<u32>(),
            Err(e) => failures.push(e),
        }
    }
    assert!(failures.is_empty(), "{} of {per} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
    assert!(activated > 0, "no effect of the anime cards was ever activated");
}

/// The same self-play with decks of the cards the card profile shows differently from the game's (the released categories:
/// flags, subtypes, types, Main/Extra placement, Level, attribute, race, ATK, DEF and kind): the first face-up monster of each is
/// held to the profile's ATK and DEF, each deck card to its Level, race and attribute in the engine's card data, and the client's
/// view must still follow the engine through their anime effects. `YO3_CHECK_DUELS=n` plays n duels instead of 40;
/// `YO3_CHECK_PROFILE_SEED=s` replays seed s alone.
#[test]
fn duels_with_the_cards_the_profile_changes() {
    let Some(dir) = engine_dir() else { return };
    if yo3_server::engine::anime_problem(&dir).is_some() {
        eprintln!("anime data not fetched; skipping");
        return;
    }
    let pool = selfplay::anime_pool_of_profile_cards(&dir, 12).expect("the cards the profile changes");
    let per: u64 = std::env::var("YO3_CHECK_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(40);
    let mut failures = vec![];
    let only: Option<u64> = std::env::var("YO3_CHECK_PROFILE_SEED").ok().and_then(|n| n.parse().ok());
    for seed in only.map_or(900..900 + per, |s| s..s + 1) {
        let (a, b) = (selfplay::anime_deck(&pool, seed * 2), selfplay::anime_deck(&pool, seed * 2 + 1));
        let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
        if let Err(e) = selfplay::play_anime(&dir, seed, decks, (seed % 2) as usize, seed.is_multiple_of(3)) {
            failures.push(e);
        }
    }
    assert!(failures.is_empty(), "{} of {per} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
}

/// The duel check holds the engine's monsters to the card profile (issue #60): with every anime card's ATK in the profile
/// changed, the first face-up monster of a profile card fails the duel and the report names the profile; unchanged, the
/// same duels pass.
#[test]
fn a_profile_that_differs_from_the_engine_fails_the_duel_check() {
    let Some(dir) = engine_dir() else { return };
    if yo3_server::engine::anime_problem(&dir).is_some() {
        eprintln!("anime data not fetched; skipping");
        return;
    }
    let pool = selfplay::anime_pool(&dir).expect("the approved cards");
    let mut caught = 0;
    for seed in 500..506 {
        let (a, b) = (selfplay::anime_deck(&pool, seed * 2), selfplay::anime_deck(&pool, seed * 2 + 1));
        let decks = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
        let wrong = selfplay::play_anime_with(&dir, seed, decks, 0, false, |expect| {
            for e in expect.values_mut() {
                e.atk = yo3_server::profile::Stat::Value(7);
                e.judge.atk = true;
            }
        });
        match wrong {
            Err(e) if e.contains("differs from the card profile") => caught += 1,
            Err(e) => panic!("seed {seed} failed for another reason:\n{e}"),
            Ok(_) => {}
        }
        selfplay::play_anime(&dir, seed, decks, 0, false).unwrap_or_else(|e| panic!("seed {seed} with the right profile:\n{e}"));
    }
    assert!(caught > 0, "no duel put a profile monster on the field, or the check does not look at them");
}
