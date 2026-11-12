//! The card sweep: every card the client knows played in a self-played duel through the rules engine and the
//! translator, with the duel check's client model compared with the engine after every step (`duelcheck::selfplay`).
//! Each card goes into both decks (three copies, two of them in the opening hand; an Extra Deck monster twice in each
//! Extra Deck) beside the duel check's pool, so both the client's side and the opponent's play it. A duel fails on a
//! difference between the client's view and the engine, an answer the engine keeps refusing, or a script error.
//!
//! `YO3_CARD_SWEEP` picks the cards: unset, 24 spread over the table; `all`, every card (about ten minutes); a number
//! n, n spread over the table; or Konami ids (`7986,7687`). `YO3_CARD_SWEEP_SEEDS=n` plays n duels per card (1).
//! The report, one line per card (id, name, result, activations, summons), goes to `card-sweep.tsv` in Cargo's test
//! temporary directory. Skipped unless the game is installed and engine/fetch.sh has run.
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

use yo3_data::{GameData, default_game_dir};
use yo3_server::duelcheck::selfplay::{self, CardData};
use yo3_server::engine::{Engine, NOT_IN_THE_ENGINE};
use yo3_server::translate::CardStats;

const TYPE_TOKEN: u64 = 0x4000;
const EXTRA_DECK: u64 = 0x40 | 0x2000 | 0x80_0000; // Fusion, Synchro, Xyz

fn dirs() -> Option<(PathBuf, PathBuf)> {
    let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let engine = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !game.join("package/data002.ypk").is_file() || !engine.join("vendor/cardinfo.json").is_file() {
        eprintln!("game not installed or engine not fetched; skipping");
        return None;
    }
    Some((game, engine))
}

/// The card data a server builds from the client's (`lib.rs`).
fn card_data(game: &GameData) -> CardData {
    let mut data = CardData::default();
    for c in game.cards.iter() {
        data.cards.insert(c.row, CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race });
        data.names.entry(c.cid).or_insert_with(|| c.name.clone());
        if (3900..4000).contains(&c.cid) {
            data.tokens.entry(c.name.clone()).or_default().push(c.row);
        }
    }
    data
}

/// The cards to sweep, by `YO3_CARD_SWEEP`: (Konami id, first row).
fn chosen(all: &[(u16, u16)]) -> Vec<(u16, u16)> {
    let spread = |n: usize| -> Vec<(u16, u16)> {
        let n = n.clamp(1, all.len());
        (0..n).map(|i| all[i * all.len() / n]).collect()
    };
    match std::env::var("YO3_CARD_SWEEP").ok().as_deref() {
        None | Some("") => spread(24),
        Some("all") => all.to_vec(),
        Some(v) if !v.contains(',') && v.parse::<usize>().is_ok_and(|n| n > 0 && n < 3900) && all.len() > 3900 => spread(v.parse().unwrap()),
        Some(v) => {
            let ids: Vec<u16> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            all.iter().copied().filter(|(cid, _)| ids.contains(cid)).collect()
        }
    }
}

#[test]
fn every_card_plays_with_the_client_view_matching_the_engine() {
    let Some((game_dir, engine_dir)) = dirs() else { return };
    let game = GameData::load(&game_dir).expect("the client's data loads");
    let data = card_data(&game);
    let row_of = |cid: u16| game.cards.by_cid(cid).map(|c| c.row);

    // Every card the engine can play (not tokens, which effects make), with its type from the engine.
    let mut firsts: BTreeMap<u16, u16> = BTreeMap::new();
    for c in game.cards.iter().filter(|c| !(3900..4000).contains(&c.cid) && !NOT_IN_THE_ENGINE.contains(&c.cid)) {
        firsts.entry(c.cid).or_insert(c.row);
    }
    let mut engine = Engine::start(&engine_dir).expect("node runs the engine");
    let list: Vec<(u16, &str)> = firsts.keys().map(|cid| (*cid, data.names[cid].as_str())).collect();
    let types: HashMap<u16, u64> = list
        .chunks(500)
        .flat_map(|chunk| engine.cards(chunk).expect("the engine checks cards"))
        .map(|a| (a["cid"].as_u64().unwrap() as u16, a["data"]["type"].as_u64().unwrap_or(0)))
        .collect();
    drop(engine);
    let all: Vec<(u16, u16)> = firsts.into_iter().filter(|(cid, _)| types.get(cid).is_some_and(|t| t & TYPE_TOKEN == 0)).collect();
    let cards = chosen(&all);
    let seeds: u64 = std::env::var("YO3_CARD_SWEEP_SEEDS").ok().and_then(|n| n.parse().ok()).unwrap_or(1);

    // The duel check's pool, as rows: the filler around the swept card (without its Xyz Monsters, which the client
    // lacks).
    let pool = |seed: u64| -> (Vec<u16>, Vec<u16>) {
        let (main, extra) = selfplay::deck(seed);
        (main.into_iter().filter_map(row_of).collect(), extra.into_iter().filter_map(row_of).collect())
    };

    let jobs: Mutex<Vec<(usize, u16, u16, u64)>> =
        Mutex::new(cards.iter().enumerate().flat_map(|(i, &(cid, row))| (0..seeds).map(move |s| (i, cid, row, s))).collect());
    type Outcome = (u16, u64, Result<selfplay::Played, String>);
    let results: Mutex<Vec<Outcome>> = Mutex::new(vec![]);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let Some((i, cid, row, s)) = jobs.lock().unwrap().pop() else { break };
                let seed = 1000 + i as u64 * 7 + s * 100_003;
                let extra_deck = types[&cid] & EXTRA_DECK != 0;
                let mut decks = [pool(seed * 2), pool(seed * 2 + 1)];
                for (main, extra) in decks.iter_mut() {
                    if extra_deck {
                        extra.truncate(11);
                        extra.extend([row, row]);
                    } else {
                        main.truncate(37);
                        main.extend([row, row, row]);
                    }
                }
                let top: &[u16] = if extra_deck { &[] } else { &[row, row] };
                let decks_ref = [(&decks[0].0[..], &decks[0].1[..]), (&decks[1].0[..], &decks[1].1[..])];
                let played = selfplay::play_with(&engine_dir, seed, decks_ref, &data, [top, top], i % 2, i % 3 == 0);
                results.lock().unwrap().push((cid, seed, played));
            });
        }
    });

    let results = results.into_inner().unwrap();
    let mut report = String::from("cid\tname\tseed\tresult\tactivated\tsummoned\tturns\tdecided for the player\n");
    let mut decided: BTreeMap<u64, u32> = BTreeMap::new();
    let mut failures = vec![];
    let (mut used, mut total) = (0, 0);
    for (cid, seed, result) in &results {
        let name = &data.names[cid];
        match result {
            Ok(p) => {
                let (a, s) = (p.activated.get(cid).copied().unwrap_or(0), p.summoned.get(cid).copied().unwrap_or(0));
                total += 1;
                used += usize::from(a + s > 0);
                for (kind, n) in &p.decided {
                    *decided.entry(*kind).or_default() += n;
                }
                let d: Vec<String> = p.decided.iter().map(|(k, n)| format!("{k}x{n}")).collect();
                report.push_str(&format!("{cid}\t{name}\t{seed}\tok\t{a}\t{s}\t{}\t{}\n", p.turns, d.join(" ")));
            }
            Err(e) => {
                let first = e.lines().next().unwrap_or_default();
                report.push_str(&format!("{cid}\t{name}\t{seed}\tFAIL {first}\t\t\t\n"));
                failures.push(format!("{cid} {name}: {e}"));
            }
        }
    }
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("card-sweep.tsv");
    std::fs::write(&out, report).expect("the report is written");
    eprintln!(
        "card sweep: {} duel(s) of {} card(s), {} failed; the swept card was used (activated or summoned) in {used} of {total}; \
         prompts the server answered for the player, by engine message type: {decided:?}; report: {}",
        results.len(),
        cards.len(),
        failures.len(),
        out.display()
    );
    assert!(failures.is_empty(), "{} duel(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}
