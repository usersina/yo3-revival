//! Real decks: the decks players used on Konami's own server, rebuilt from its duel logs (a 2010-2012 client folder's
//! `.dlog` files, kept out of git), self-played through the duel check like the card sweep (`tests/card_sweep.rs`),
//! so that the combinations people really played (Junk Synchron with Tuningware, Spell Counter decks...) are tested.
//!
//! A log's card table names every card the duel showed, by uid with its Konami id and first place, so each side's deck
//! is the cards it showed (Fusion and Synchro Monsters in its Extra Deck), matched to this client's rows by Konami id and filled to 40 with
//! the same cards (three copies at most where it can). Cards this client lacks are left out.
//!
//! The logs: `YO3_ORIGINAL_LOGS` (a folder searched for `*.dlog`), else `re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players`.
//! `YO3_ORIGINAL_DECKS` picks the logs: unset, 8; `all`; or a number. `YO3_ORIGINAL_DECKS_SEEDS=n` duels per log (1).
//! Each duel's result goes to `original-decks.tsv` in Cargo's test temporary directory. Skipped without the logs, the
//! game or the engine.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use yo3_data::{GameData, default_game_dir};
use yo3_server::duelcheck::selfplay::{self, CardData};
use yo3_server::engine::NOT_IN_THE_ENGINE;
use yo3_server::translate::CardStats;

const BLOCK: usize = 0x21D0;
const EXTRA: u32 = 0xC;

fn logs_dir() -> PathBuf {
    std::env::var_os("YO3_ORIGINAL_LOGS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players"))
}

fn dlogs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            dlogs(&path, out);
        } else if path.extension().is_some_and(|e| e == "dlog") {
            out.push(path);
        }
    }
}

/// A log's two decks as Konami ids (main, extra), by the side the card table first put each card on; None for a file
/// that isn't a duel log.
fn decks_of(bytes: &[u8]) -> Option<[(Vec<u16>, Vec<u16>); 2]> {
    if bytes.len() < 360 || bytes[..2] != [0xE7, 0x09] {
        return None;
    }
    let body: Vec<u8> = bytes[360..].as_chunks::<2>().0.iter().flat_map(|p| std::iter::repeat_n(p[0], p[1] as usize)).collect();
    let u32_at = |at: usize| body.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let start = 80 + 2 * BLOCK;
    let count = u32_at(start)? as usize;
    // uid -> (side, first zone, Konami id)
    let mut cards: BTreeMap<u32, (usize, u32, u16)> = BTreeMap::new();
    for i in 0..count {
        let at = start + 4 + 24 * i;
        let (id, uid, cid, place) = (u32_at(at)?, u32_at(at + 8)? & 0xFF_FFFF, u32_at(at + 16)?, u32_at(at + 20)?);
        if id != 1000 || cid == 0 || (3900..4000).contains(&cid) {
            continue;
        }
        cards.entry(uid).or_insert(((place & 1) as usize, place >> 8 & 0xFF, cid as u16));
    }
    let mut decks: [(Vec<u16>, Vec<u16>); 2] = Default::default();
    for (side, zone, cid) in cards.into_values() {
        if zone == EXTRA { decks[side].1.push(cid) } else { decks[side].0.push(cid) }
    }
    Some(decks)
}

/// Konami ids as this client's rows, the main deck filled to 40 with its own cards.
fn as_rows(game: &GameData, (main, extra): &(Vec<u16>, Vec<u16>)) -> (Vec<u16>, Vec<u16>) {
    // By the card's kind, not where the log first showed it (a Fusion Monster may first show in the Graveyard).
    let cards: Vec<&yo3_data::Card> =
        main.iter().chain(extra).filter(|cid| !NOT_IN_THE_ENGINE.contains(cid)).filter_map(|cid| game.cards.by_cid(*cid)).collect();
    let mut main: Vec<u16> = cards.iter().filter(|c| !c.extra_deck()).map(|c| c.row).collect();
    let extra: Vec<u16> = cards.iter().filter(|c| c.extra_deck()).map(|c| c.row).take(15).collect();
    let seen = main.clone();
    let mut copies: HashMap<u16, usize> = HashMap::new();
    for r in &main {
        *copies.entry(*r).or_default() += 1;
    }
    let mut i = 0;
    while !seen.is_empty() && main.len() < 40 && i < 40 * 4 {
        let r = seen[i % seen.len()];
        // Three copies at most while some card still has room, then any.
        if copies[&r] < 3 || i >= 40 * 3 {
            main.push(r);
            *copies.get_mut(&r).unwrap() += 1;
        }
        i += 1;
    }
    (main, extra)
}

#[test]
fn the_decks_played_on_konamis_server_play_with_the_client_view_matching_the_engine() {
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let engine_dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    let mut files = vec![];
    dlogs(&logs_dir(), &mut files);
    if files.is_empty() || !game_dir.join("package/data002.ypk").is_file() || !engine_dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("original duel logs, the game or the engine missing; skipping");
        return;
    }
    let game = GameData::load(&game_dir).expect("the client's data loads");
    let mut data = CardData::default();
    for c in game.cards.iter() {
        data.cards.insert(c.row, CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race });
        data.names.entry(c.cid).or_insert_with(|| c.name.clone());
        if (3900..4000).contains(&c.cid) {
            data.tokens.entry(c.name.clone()).or_default().push(c.row);
        }
    }
    let mut decks = vec![];
    for f in &files {
        let Some(pair) = std::fs::read(f).ok().and_then(|b| decks_of(&b)) else { continue };
        let [a, b] = pair.map(|side| as_rows(&game, &side));
        if a.0.len() >= 20 && b.0.len() >= 20 {
            decks.push([a, b]);
        }
    }
    assert!(!decks.is_empty(), "no usable decks in {} logs", files.len());
    let take = match std::env::var("YO3_ORIGINAL_DECKS").ok().as_deref() {
        Some("all") => decks.len(),
        Some(n) => n.parse().unwrap_or(8),
        None => 8,
    };
    let picked: Vec<_> = (0..take.min(decks.len())).map(|i| &decks[i * decks.len() / take.min(decks.len())]).collect();
    let seeds: u64 = std::env::var("YO3_ORIGINAL_DECKS_SEEDS").ok().and_then(|n| n.parse().ok()).unwrap_or(1);

    let jobs: Mutex<Vec<(usize, u64)>> = Mutex::new((0..picked.len()).flat_map(|i| (0..seeds).map(move |s| (i, s))).collect());
    type Outcome = (usize, u64, Result<selfplay::Played, String>);
    let results: Mutex<Vec<Outcome>> = Mutex::new(vec![]);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let Some((i, s)) = jobs.lock().unwrap().pop() else { break };
                let seed = 50_000 + i as u64 * 13 + s * 100_019;
                let [a, b] = picked[i];
                let pair = [(&a.0[..], &a.1[..]), (&b.0[..], &b.1[..])];
                let played = selfplay::play_with(&engine_dir, seed, pair, &data, [&[], &[]], (i + s as usize) % 2, (i + s as usize).is_multiple_of(3));
                results.lock().unwrap().push((i, seed, played));
            });
        }
    });

    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| (r.0, r.1));
    let mut report = String::from("deck pair\tseed\tresult\tturns\tsteps\tactivations\tsummons\tdecided for the player\n");
    let mut failures = vec![];
    for (i, seed, result) in &results {
        match result {
            Ok(p) => {
                let d: Vec<String> = p.decided.iter().map(|(k, n)| format!("{k}x{n}")).collect();
                report.push_str(&format!(
                    "{i}\t{seed}\tok\t{}\t{}\t{}\t{}\t{}\n",
                    p.turns,
                    p.steps,
                    p.activated.values().sum::<u32>(),
                    p.summoned.values().sum::<u32>(),
                    d.join(" ")
                ));
            }
            Err(e) => {
                report.push_str(&format!("{i}\t{seed}\tFAIL {}\n", e.lines().next().unwrap_or_default()));
                failures.push(format!("deck pair {i}: {e}"));
            }
        }
    }
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("original-decks.tsv");
    std::fs::write(&out, report).expect("the report is written");
    eprintln!(
        "original decks: {} duel(s) of {} deck pair(s) from {} log(s), {} failed; report: {}",
        results.len(),
        picked.len(),
        files.len(),
        failures.len(),
        out.display()
    );
    assert!(failures.is_empty(), "{} duel(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}
