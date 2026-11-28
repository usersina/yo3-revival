//! The custom NPCs (`engine/added/npcs.json`) in self-played duels: each level's deck, as the server builds it, played by
//! the duel bot with the character's style (`npc.rs`) against a hero of its series (Seto Kaiba, Jaden Yuki or Yusei Fudo,
//! their middle level, the default style), both decks placed as the NPC duel places them (`World::place_deck`: a card the
//! anime profile makes a Main Deck card leaves the Extra Deck), with anime cards off and on, under the duel check (the client's view compared
//! with the engine after every step, through the translator, as live). A duel fails on a duel check problem, an engine
//! (Lua) error or the bot finding no answer; one that reaches the duel check's turn limit is reported as a stall.
//!
//! The game's own NPCs' anime decks (`game_npcs`) play too, anime cards on only, in the full run or when named.
//!
//! `YO3_NPC_SELFPLAY=n`: n duels per level and mode for every custom NPC (the full run, `task check:npcs`); unset, one duel
//! each for the last level of the first custom NPC of each series, anime cards on. `YO3_NPC_ONLY=Name,Name` only those;
//! `YO3_NPC_JOBS=n` duels at once (4). Each duel's line (name, level, mode, seed, result, turns, the deck's bosses summoned
//! and which, the bot's longest answer in ms) goes to `custom-npc-selfplay.tsv` in Cargo's test temporary directory. Skipped without the game or the engine.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::Value as Json;
use yo3_server::duelcheck::selfplay;

struct Duel {
    name: String,
    slot: u32,
    level: usize,
    anime: bool,
    seed: u64,
    npc: [Vec<u16>; 2],
    rival: [Vec<u16>; 2],
    style: Json,
    bosses: Vec<u16>,
}

#[test]
fn every_custom_npcs_decks_play_self_played_duels_through_the_duel_check() {
    let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    let engine = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !game.join("package/data002.ypk").is_file() || !engine.join("vendor/cardinfo.json").is_file() {
        eprintln!("game not installed or engine not fetched; skipping");
        return;
    }
    // nextest runs each test in its own process
    unsafe { std::env::set_var("YO3_ENGINE_DIR", &engine) };
    let root = std::env::temp_dir().join(format!("yo3-custom-npc-selfplay-{}", std::process::id()));
    let mut cfg = yo3_server::Config::ephemeral(root.clone());
    cfg.game_dir = game;
    let world = yo3_server::game_world(&cfg).unwrap();
    let full: Option<u64> = std::env::var("YO3_NPC_SELFPLAY").ok().and_then(|v| v.parse().ok());
    let only: Vec<String> = std::env::var("YO3_NPC_ONLY").map(|v| v.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
    let cid = |row: &u16| world.cards.get(row).map_or(0, |c| c.cid);
    let ids = |rows: &[u16]| rows.iter().map(cid).collect::<Vec<u16>>();
    // A hero of each series, the middle of his three levels, as the rival.
    let rival_of = |series: u32| {
        let hero = ["Seto Kaiba", "Jaden Yuki", "Yusei Fudo"][series.min(2) as usize];
        let mut levels: Vec<u32> = world.npc_names.iter().filter(|(_, n)| n.as_str() == hero).map(|(id, _)| *id).collect();
        levels.sort();
        levels[1]
    };
    let mut duels = vec![];
    let mut first_of_series = [true; 3];
    for npc in world.added.npcs.iter() {
        let smoke_pick = npc.slot > 25 && std::mem::take(&mut first_of_series[npc.series.min(2) as usize]);
        if !only.is_empty() && !only.contains(&npc.name) || only.is_empty() && full.is_none() && !smoke_pick {
            continue;
        }
        let rival = rival_of(npc.series);
        for (level, id) in npc.ids().into_iter().enumerate() {
            if full.is_none() && only.is_empty() && level != 2 {
                continue;
            }
            for anime in [false, true] {
                if full.is_none() && !anime {
                    continue;
                }
                world.settings.set("anime-cards", if anime { "on" } else { "off" }).unwrap();
                // As the NPC duel's opening (duel.rs) places them.
                let deck = world.place_deck(world.npc_deck(id).unwrap());
                let rival_deck = world.place_deck(world.npc_deck(rival).unwrap());
                // The deck's bosses: Level 7 or more, or an Extra Deck monster.
                let mut bosses: Vec<u16> = deck.main.iter().filter(|r| world.cards.get(r).is_some_and(|c| c.level >= 7))
                    .chain(deck.extra.iter()).map(cid).collect();
                bosses.sort();
                bosses.dedup();
                for n in 0..full.unwrap_or(1) {
                    duels.push(Duel {
                        name: npc.name.clone(), slot: npc.slot, level: level + 1, anime,
                        seed: 7000 + 97 * u64::from(npc.slot) + 13 * level as u64 + 3 * n + u64::from(anime),
                        npc: [ids(&deck.main), ids(&deck.extra)], rival: [ids(&rival_deck.main), ids(&rival_deck.extra)],
                        style: world.npc_styles.get(&id).cloned().unwrap_or(Json::Null), bosses: bosses.clone(),
                    });
                }
            }
        }
    }
    // The game's own NPCs' anime decks (`game_npcs`), anime cards on only: their printed decks are the client's presets.
    // Their series by slot as the roster's tabs (0-8 Yu-Gi-Oh!, 9-16 GX, 17-24 5D's, docs/protocol.md).
    if full.is_some() || !only.is_empty() {
        world.settings.set("anime-cards", "on").unwrap();
        for npc in world.added.game_npcs.iter() {
            if !only.is_empty() && !only.contains(&npc.name) {
                continue;
            }
            let rival = rival_of(match npc.slot { 0..=8 => 0, 9..=16 => 1, _ => 2 });
            for ((level, id), anime_level) in npc.ids().into_iter().enumerate().zip(&npc.levels) {
                if anime_level.anime_deck.is_empty() || !world.npc_anime_decks.contains_key(&id) {
                    continue;
                }
                let deck = world.place_deck(world.npc_deck(id).unwrap());
                let rival_deck = world.place_deck(world.npc_deck(rival).unwrap());
                let mut bosses: Vec<u16> = deck.main.iter().filter(|r| world.cards.get(r).is_some_and(|c| c.level >= 7))
                    .chain(deck.extra.iter()).map(cid).collect();
                bosses.sort();
                bosses.dedup();
                for n in 0..full.unwrap_or(1) {
                    duels.push(Duel {
                        name: npc.name.clone(), slot: npc.slot, level: level + 1, anime: true,
                        seed: 5000 + 97 * u64::from(npc.slot) + 13 * level as u64 + 3 * n,
                        npc: [ids(&deck.main), ids(&deck.extra)], rival: [ids(&rival_deck.main), ids(&rival_deck.extra)],
                        style: world.npc_styles.get(&id).cloned().unwrap_or(Json::Null), bosses: bosses.clone(),
                    });
                }
            }
        }
    }
    let jobs: usize = std::env::var("YO3_NPC_JOBS").ok().and_then(|v| v.parse().ok()).unwrap_or(4).max(1);
    let queue = Mutex::new(duels.iter());
    let results: Mutex<Vec<(usize, String, bool)>> = Mutex::new(vec![]);
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let Some(d) = queue.lock().unwrap().next() else { break };
                let index = duels.iter().position(|x| std::ptr::eq(x, d)).unwrap();
                // The NPC goes first in even seeds; it is the CPU's side, the rival the client's, as live.
                let npc_team = (d.seed % 2) as usize;
                let mut decks: [(&[u16], &[u16]); 2] = [(&d.rival[0], &d.rival[1]), (&d.rival[0], &d.rival[1])];
                decks[npc_team] = (&d.npc[0], &d.npc[1]);
                let mut styles = [Json::Null, Json::Null];
                styles[npc_team] = d.style.clone();
                let head = format!("{}\t{}\t{}\t{}\t{}", d.name, d.slot, d.level, if d.anime { "anime" } else { "printed" }, d.seed);
                let (line, failed) = match selfplay::play_bots(&engine, d.seed, decks, d.anime, styles, 1 - npc_team) {
                    Ok(p) => {
                        let result = match p.winner {
                            Some(w) if w as usize == npc_team => "win",
                            Some(_) => "loss",
                            None if p.ended => "draw",
                            None => "stall",
                        };
                        let summoned: Vec<String> = d.bosses.iter().filter(|b| p.summoned.contains_key(b)).map(|b| b.to_string()).collect();
                        (format!("{head}\t{result}\t{}\t{}/{}\t{}\t{}", p.turns, summoned.len(), d.bosses.len(), summoned.join(","), p.longest_thought_ms), false)
                    }
                    Err(e) => (format!("{head}\terror\t\t\t{}", e.replace(['\n', '\t'], " ")), true),
                };
                eprintln!("{line}");
                results.lock().unwrap().push((index, line, failed));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| r.0);
    let report = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("custom-npc-selfplay.tsv");
    let table: String = results.iter().map(|r| format!("{}\n", r.1)).collect();
    std::fs::write(&report, format!("name\tslot\tlevel\tmode\tseed\tresult\tturns\tbosses\tsummoned\tthink_ms\n{table}")).unwrap();
    let mut tally: BTreeMap<&str, u32> = BTreeMap::new();
    for r in &results {
        *tally.entry(r.1.split('\t').nth(5).unwrap_or("?")).or_default() += 1;
    }
    eprintln!("{} duels: {tally:?}; report {}", results.len(), report.display());
    let _ = std::fs::remove_dir_all(root);
    let failed: Vec<&str> = results.iter().filter(|r| r.2).map(|r| r.1.as_str()).collect();
    assert!(failed.is_empty(), "{} failed:\n{}", failed.len(), failed.join("\n"));
}
