//! Effect coverage for the cards players actually meet (#67): every preset deck the client ships (the starter and
//! character/structure decks, and each NPC's) self-played through the duel check (`duelcheck::selfplay`, the client's
//! view compared with the engine after every step), against another preset deck, and the effects that resolved counted
//! per card as the duel check's parts are. `tools/engine/coverage.py --decks` reads the result: the effect cards of
//! those decks that no duel (these, or the duel check's) ever exercised, by the decks that hold them.
//!
//! `YO3_DECK_COVERAGE_SEEDS=n` plays each deck n times (2: once from each seat). The report goes to
//! `duel-coverage/decks.json` in Cargo's test temporary directory. Skipped without the game or the engine. A duel that
//! fails (a difference between the client's view and the engine) fails the test and is listed, as in the card sweep.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Mutex;

use yo3_data::{GameData, default_game_dir};
use yo3_server::duelcheck::selfplay::{self, CardData};
use yo3_server::engine::NOT_IN_THE_ENGINE;
use yo3_server::translate::CardStats;

fn card_data(game: &GameData) -> CardData {
    let mut data = CardData { tokens: yo3_server::token_rows(&game.cards), ..CardData::default() };
    for c in game.cards.iter() {
        data.cards.insert(c.row, CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race });
        data.names.entry(c.cid).or_insert_with(|| c.name.clone());
        if c.tuner() {
            data.tuners.insert(c.cid);
        }
    }
    data
}

/// A deck the players meet, for the report: where it comes from and its rows (main, extra) without cards the engine
/// can't play.
struct Preset {
    label: String,
    main: Vec<u16>,
    extra: Vec<u16>,
}

fn presets(game: &GameData) -> Vec<Preset> {
    let playable = |rows: &[u16]| -> Vec<u16> {
        rows.iter().copied().filter(|r| game.cards.by_row(*r).is_some_and(|c| !NOT_IN_THE_ENGINE.contains(&c.cid))).collect()
    };
    let npcs = game.npc_ids();
    let mut out = vec![];
    for d in &game.decks {
        let label = match npcs.iter().find(|&&n| game.npc_deck(n).is_some_and(|nd| nd.id == d.id)) {
            Some(&n) => format!("NPC {n} {} ({})", game.npc_name(n).unwrap_or("?"), game.npc_deck_title(n).unwrap_or("?")),
            None => format!("preset deck {} (type {})", d.id, d.kind),
        };
        let (main, extra) = (playable(&d.main), playable(&d.extra));
        if main.len() >= 10 {
            out.push(Preset { label, main, extra: extra.into_iter().take(15).collect() });
        }
    }
    out
}

#[test]
fn preset_decks_play_with_the_client_view_matching_the_engine() {
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let engine_dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !game_dir.join("package/data002.ypk").is_file() || !engine_dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("game not installed or engine not fetched; skipping");
        return;
    }
    let game = GameData::load(&game_dir).expect("the client's data loads");
    let data = card_data(&game);
    let decks = presets(&game);
    assert!(decks.len() > 10, "only {} preset decks", decks.len());
    let seeds: u64 = std::env::var("YO3_DECK_COVERAGE_SEEDS").ok().and_then(|n| n.parse().ok()).unwrap_or(2);
    let cid = |row: &u16| data.cards.get(row).map_or(0, |c| c.cid);

    let jobs: Mutex<Vec<(usize, u64)>> = Mutex::new((0..decks.len()).flat_map(|i| (0..seeds).map(move |s| (i, s))).collect());
    let results: Mutex<Vec<(usize, Result<selfplay::Played, String>)>> = Mutex::new(vec![]);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let Some((i, s)) = jobs.lock().unwrap().pop() else { break };
                // Against the next deck in the list, so that every deck faces others over the seeds.
                let (a, b) = (&decks[i], &decks[(i + 1 + s as usize) % decks.len()]);
                let seed = 5000 + i as u64 * 31 + s * 100_003;
                let pair = [(&a.main[..], &a.extra[..]), (&b.main[..], &b.extra[..])];
                let top: [&[u16]; 2] = [&[], &[]];
                let played = selfplay::play_with(&engine_dir, seed, pair, &data, top, (s % 2) as usize, seed.is_multiple_of(3));
                results.lock().unwrap().push((i, played));
            });
        }
    });

    let mut cards: BTreeMap<u16, [u32; 5]> = BTreeMap::new();
    let (mut failures, mut unanswerable) = (vec![], vec![]);
    let mut duels = 0;
    for (i, result) in results.into_inner().unwrap() {
        match result {
            Ok(p) => {
                duels += 1;
                for (k, counts) in [&p.activated, &p.resolved, &p.visible, &p.negated, &p.summoned].into_iter().enumerate() {
                    for (&c, &n) in counts {
                        cards.entry(c).or_default()[k] += n;
                    }
                }
            }
            // A prompt whose answer the client's windows can't give (a Special Summon from the Deck, a Tribute list the
            // client can't show) is a finding to list (docs/roadmap.md), not a drift of the client's view: the duel
            // stops there. `YO3_DECK_COVERAGE_STRICT=1` fails on those too.
            Err(e) if e.contains("the client can't answer") && std::env::var_os("YO3_DECK_COVERAGE_STRICT").is_none() => {
                unanswerable.push(format!("{}: {}", decks[i].label, e.lines().take(2).collect::<Vec<_>>().join(" | ")))
            }
            Err(e) => failures.push(format!("{}: {e}", decks[i].label)),
        }
    }
    let by_deck: BTreeMap<&str, BTreeSet<u16>> =
        decks.iter().map(|d| (d.label.as_str(), d.main.iter().chain(&d.extra).map(cid).filter(|&c| c != 0).collect())).collect();
    let pool: BTreeSet<u16> = by_deck.values().flatten().copied().collect();
    let report = serde_json::json!({
        "duels": duels,
        "pool": pool,
        "decks": by_deck,
        "unanswerable": unanswerable,
        "cards": cards.iter().map(|(c, [a, r, v, n, s])| (c.to_string(), serde_json::json!({"activated": a, "resolved": r, "visible": v, "negated": n, "summoned": s}))).collect::<serde_json::Map<_, _>>(),
    });
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("duel-coverage");
    std::fs::create_dir_all(&dir).expect("the report folder");
    std::fs::write(dir.join("decks.json"), report.to_string()).expect("the report is written");
    eprintln!("deck coverage: {duels} duel(s) of {} preset deck(s), {} distinct card(s), {} failed, {} stopped at a prompt the client can't answer:\n{}", decks.len(), pool.len(), failures.len(), unanswerable.len(), unanswerable.join("\n"));
    assert!(failures.is_empty(), "{} duel(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}
