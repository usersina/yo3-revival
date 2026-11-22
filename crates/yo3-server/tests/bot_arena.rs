//! The bot arena: this build's duel bot against another engine program's bot (the one to beat, usually `main`'s), both
//! playing the client's NPC decks with their characters' styles (`npc::style`). Each duel runs in two engine programs
//! in lockstep: both are given the same duel and every answer, and each side's bot thinks in its own program. A bot
//! change lands when it wins clearly more than it loses here (docs/development.md, "The duel bot").
//!
//!     YO3_RIVAL_ENGINE=/path/to/yo3-engine YO3_ARENA_DUELS=40 cargo nextest run --release -p yo3-server --test bot_arena --run-ignored only --no-capture
//!     YO3_ARENA_OURS='{"card":900}' YO3_ARENA_DUELS=40 cargo nextest run ...   (this build on both sides, other numbers on ours)
//!     YO3_ARENA_OURS_STYLE='{"depth":3}' ...   (more of the bot's style for our side; YO3_ARENA_THEIRS_STYLE for theirs)
//!
//! `YO3_ARENA_DUELS` (default 20) duels; deck pairs spread over the NPCs, each pair played twice with the sides swapped
//! (each bot plays each deck, first and second). The report goes to `bot-arena.tsv` in Cargo's test temporary
//! directory. Ignored by default: it takes minutes.
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::Value as Json;
use yo3_data::{GameData, default_game_dir};
use yo3_server::engine::Engine;

/// How one duel went: the winner (0 this build's bot, 1 the rival, None: a draw or too long), turns, and each bot's
/// answers and thinking time.
struct Duel {
    winner: Option<usize>,
    turns: u32,
    answers: [u32; 2],
    ms: [f64; 2],
}

/// One duel between `ours` and `theirs` in lockstep; `ours_team` is the engine team our bot plays.
fn duel(ours: &mut Engine, theirs: &mut Engine, seed: u64, decks: [(&[u16], &[u16]); 2], styles: [&Json; 2], ours_team: u64, weights: &[Json; 2]) -> Duel {
    let names = Default::default();
    let mut a = ours.new_duel_with(seed, 8000, decks, &names, &[]).expect("our duel starts");
    let b = theirs.new_duel_with(seed, 8000, decks, &names, &[]).expect("their duel starts");
    assert_eq!(a.messages, b.messages, "both programs start the same duel");
    let mut out = Duel { winner: None, turns: 0, answers: [0; 2], ms: [0.0; 2] };
    for _ in 0..8000 {
        out.turns += a.messages.iter().filter(|m| m["type"] == 40).count() as u32;
        if let Some(win) = a.messages.iter().find(|m| m["type"] == 5) {
            out.winner = win["player"].as_u64().filter(|&p| p < 2).map(|p| if p == ours_team { 0 } else { 1 });
            return out;
        }
        if a.status != "await" {
            break;
        }
        let team = a.messages.last().and_then(|m| m["player"].as_u64()).unwrap_or(0);
        let side = if team == ours_team { 0 } else { 1 };
        let started = Instant::now();
        let mut style = with_weights(styles[team as usize], &weights[side]);
        // YO3_ARENA_OURS_STYLE / YO3_ARENA_THEIRS_STYLE: more of the bot's style for that side ({"depth": 3}).
        let extra = std::env::var(["YO3_ARENA_OURS_STYLE", "YO3_ARENA_THEIRS_STYLE"][side]).ok().and_then(|e| serde_json::from_str::<Json>(&e).ok());
        if let Some(Json::Object(extra)) = extra {
            for (k, v) in extra {
                style[k] = v;
            }
        }
        let response = if side == 0 { ours.think(style) } else { theirs.think(style) }.expect("an answer");
        out.ms[side] += started.elapsed().as_secs_f64() * 1000.0;
        out.answers[side] += 1;
        a = ours.respond(response.clone()).expect("our program answers");
        let b = theirs.respond(response).expect("their program answers");
        assert_eq!(a.messages.len(), b.messages.len(), "both programs play the same duel");
    }
    out
}

/// A character's style with the bot's numbers `weights` (`bot::Weights`; null: the defaults).
fn with_weights(style: &Json, weights: &Json) -> Json {
    let mut style = if style.is_object() { style.clone() } else { serde_json::json!({}) };
    if !weights.is_null() {
        style["weights"] = weights.clone();
    }
    style
}

#[test]
#[ignore]
fn this_bot_against_the_rival_engines_bot() {
    let engine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    // Without a rival program both sides run this build's: then `YO3_ARENA_OURS` and `YO3_ARENA_THEIRS` (the bot's
    // numbers, JSON) are what differs.
    let rival = std::env::var_os("YO3_RIVAL_ENGINE").map(PathBuf::from).unwrap_or_else(|| yo3_server::engine::program(&engine_dir));
    let weight = |key: &str| std::env::var(key).ok().map(|w| serde_json::from_str::<Json>(&w).expect("weights as JSON")).unwrap_or(Json::Null);
    let weights = [weight("YO3_ARENA_OURS"), weight("YO3_ARENA_THEIRS")];
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let game = GameData::load(&game_dir).expect("the client's data loads");
    let cid = |rows: &[u16]| -> Vec<u16> { rows.iter().filter_map(|r| game.cards.by_row(*r)).map(|c| c.cid).collect() };
    let npcs: Vec<(String, Vec<u16>, Vec<u16>)> = game.npc_ids().into_iter()
        .filter_map(|id| Some((game.npc_name(id)?.to_string(), game.npc_deck(id)?)))
        .map(|(name, d)| (name, cid(&d.main), cid(&d.extra))).collect();
    // `YO3_ARENA_SEED`: the first seed (7000): another base plays other duels, to confirm numbers tuned on these.
    let base: u64 = std::env::var("YO3_ARENA_SEED").ok().and_then(|n| n.parse().ok()).unwrap_or(7000);
    let duels: usize = std::env::var("YO3_ARENA_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(20);
    let results: Vec<_> = std::thread::scope(|scope| {
        // A few duels at once: each bot thinks on threads of its own, so more would slow both and change the play.
        let parallel = 2;
        let chunks: Vec<_> = (0..parallel).map(|t| {
            let (npcs, engine_dir, rival, weights, base) = (&npcs, &engine_dir, &rival, &weights, base);
            scope.spawn(move || {
                let mut out = vec![];
                for i in (t..duels).step_by(parallel) {
                    let k = i / 2 + (base as usize - 7000) / 2;
                    let (a, b) = ((k * 7) % npcs.len(), (k * 7 + 31) % npcs.len());
                    let seed = base + i as u64 / 2;
                    let ours_team = i as u64 % 2;
                    let decks = [(&npcs[a].1[..], &npcs[a].2[..]), (&npcs[b].1[..], &npcs[b].2[..])];
                    let styles = [&yo3_server::npc::style(&npcs[a].0), &yo3_server::npc::style(&npcs[b].0)];
                    let mut ours = Engine::start(engine_dir).expect("this build's engine runs");
                    let mut command = Command::new(rival);
                    command.arg(engine_dir);
                    let mut theirs = Engine::spawn(command, Duration::from_secs(60)).expect("the rival engine runs");
                    let d = duel(&mut ours, &mut theirs, seed, decks, styles, ours_team, weights);
                    out.push((i, seed, ours_team, npcs[a].0.clone(), npcs[b].0.clone(), d));
                }
                out
            })
        }).collect();
        chunks.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let mut report = String::from("duel\tseed\tours team\tdeck a\tdeck b\twinner\tturns\tours ms/answer\ttheirs ms/answer\n");
    let (mut won, mut lost, mut turns_won) = (0, 0, 0);
    let mut results = results;
    results.sort_by_key(|r| r.0);
    for (i, seed, team, a, b, d) in &results {
        match d.winner {
            Some(0) => { won += 1; turns_won += d.turns }
            Some(_) => lost += 1,
            None => {}
        }
        let per = |s: usize| d.ms[s] / d.answers[s].max(1) as f64;
        report.push_str(&format!("{i}\t{seed}\t{team}\t{a}\t{b}\t{:?}\t{}\t{:.0}\t{:.0}\n", d.winner.map(|w| ["ours", "theirs"][w]), d.turns, per(0), per(1)));
    }
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("bot-arena.tsv");
    std::fs::write(&out, &report).unwrap();
    eprintln!("{report}");
    eprintln!("this build won {won}, lost {lost} of {duels} (draws or too long: {}); its wins took {:.1} turns on average; report {}",
        duels - won - lost, turns_won as f64 / won.max(1) as f64, out.display());
}
