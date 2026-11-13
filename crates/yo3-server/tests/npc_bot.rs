//! The duel bot (crates/yo3-engine/src/bot) against the old placeholder CPU, both playing the client's NPC decks: the
//! bot must win clearly more often. `YO3_BOT_DUELS=n` plays n duels (default 6), `YO3_BOT_MIRROR=1` with the same deck on
//! both sides (skill alone, no deck edge), `YO3_BOT_ONLY=i` only duel i, `YO3_BOT_RIVAL=action` against the bot's
//! first version instead (`bot`: the default bot), `YO3_BOT_STYLE={...}` the bot's own style,
//! `YO3_BOT_SKILL=1` the heroes' Heart of the Cards; each duel's result and the bot's
//! thinking time go to `npc-bot.tsv` in Cargo's test temporary directory. Skipped without the game or the engine.
use std::path::PathBuf;
use std::time::Instant;

use yo3_data::{GameData, default_game_dir};
use yo3_server::engine::Engine;

/// The bot's opponent: the old placeholder CPU, or with `YO3_BOT_RIVAL=action` the bot's first version (judging an
/// option right after its action, not through the opponent's turn).
fn rival() -> serde_json::Value {
    match std::env::var("YO3_BOT_RIVAL").as_deref() {
        Ok("action") => serde_json::json!({ "reach": "action" }),
        Ok("bot") => serde_json::Value::Null,
        _ => serde_json::json!({ "policy": "simple" }),
    }
}

/// The bot's own style: `YO3_BOT_STYLE` (JSON, e.g. {"sees":true}), else the default.
fn own_style() -> serde_json::Value {
    std::env::var("YO3_BOT_STYLE").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// One duel: the bot plays engine team `bot` (team 0 goes first). The winner's team (None: a draw or too long), the
/// bot's answers and its total thinking time.
fn duel(engine_dir: &std::path::Path, seed: u64, decks: [(&[u16], &[u16]); 2], bot: u64) -> (Option<u64>, u32, f64, f64) {
    let mut engine = Engine::start(engine_dir).expect("the engine program runs");
    // `YO3_BOT_SKILL=1`: the bot has the heroes' Heart of the Cards.
    let skills: Vec<(u32, String)> =
        if std::env::var_os("YO3_BOT_SKILL").is_some() { vec![(bot as u32, "heart_of_the_cards".into())] } else { vec![] };
    let mut answer = engine.new_duel_with(seed, 8000, decks, &Default::default(), &skills).expect("the duel starts");
    let (mut thoughts, mut ms, mut longest) = (0u32, 0f64, 0f64);
    let mut recent: std::collections::VecDeque<String> = Default::default();
    for _ in 0..6000 {
        if let Some(win) = answer.messages.iter().find(|m| m["type"] == 5) {
            return (win["player"].as_u64(), thoughts, ms, longest);
        }
        if answer.status != "await" {
            break;
        }
        let player = answer.messages.last().and_then(|m| m["player"].as_u64()).unwrap_or(0);
        let started = Instant::now();
        let style = if player == bot { own_style() } else { rival() };
        let response = engine.think(style).expect("an answer");
        if player == bot {
            let took = started.elapsed().as_secs_f64() * 1000.0;
            thoughts += 1;
            ms += took;
            longest = longest.max(took);
        }
        recent.push_back(format!("{} -> {response}", answer.messages.last().map(|m| m.to_string()).unwrap_or_default()));
        if recent.len() > 6 {
            recent.pop_front();
        }
        answer = engine.respond(response).expect("the engine answers");
    }
    eprintln!("duel {seed} (bot team {bot}) did not end; its last prompts:\n{}", recent.iter().map(|r| format!("  {r}")).collect::<Vec<_>>().join("\n"));
    (None, thoughts, ms, longest)
}

#[test]
fn the_bot_beats_the_placeholder_cpu_with_the_npc_decks() {
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let engine_dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !game_dir.join("package/data002.ypk").is_file() || !engine_dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("the game or the engine missing; skipping");
        return;
    }
    let game = GameData::load(&game_dir).expect("the client's data loads");
    let cid = |rows: &[u16]| -> Vec<u16> { rows.iter().filter_map(|r| game.cards.by_row(*r)).map(|c| c.cid).collect() };
    let npcs: Vec<(Vec<u16>, Vec<u16>)> = game.npc_ids().into_iter().filter_map(|id| game.npc_deck(id)).map(|d| (cid(&d.main), cid(&d.extra))).collect();
    let duels: usize = std::env::var("YO3_BOT_DUELS").ok().and_then(|n| n.parse().ok()).unwrap_or(6);
    let mut report = String::from("duel\tseed\tbot team\tdeck a\tdeck b\twinner\tbot answers\tbot ms\tlongest ms\n");
    let (mut won, mut lost, mut ms, mut answers) = (0, 0, 0.0, 0);
    let results: Vec<_> = std::thread::scope(|scope| {
        let only: Option<usize> = std::env::var("YO3_BOT_ONLY").ok().and_then(|n| n.parse().ok());
        let handles: Vec<_> = (0..duels)
            .filter(|i| only.is_none_or(|o| o == *i))
            .map(|i| {
                let (npcs, engine_dir) = (&npcs, &engine_dir);
                scope.spawn(move || {
                    // Deck pairs spread over the NPCs; the bot plays each side of a pair once, going first and second.
                    let (a, b) = ((i / 2 * 7) % npcs.len(), (i / 2 * 7 + 31) % npcs.len());
                    let b = if std::env::var_os("YO3_BOT_MIRROR").is_some() { a } else { b };
                    let seed = 9000 + i as u64 / 2;
                    let bot = i as u64 % 2;
                    let decks = [(&npcs[a].0[..], &npcs[a].1[..]), (&npcs[b].0[..], &npcs[b].1[..])];
                    (i, seed, bot, a, b, duel(engine_dir, seed, decks, bot))
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for (i, seed, bot, a, b, (winner, thoughts, took, longest)) in results {
        match winner {
            Some(w) if w == bot => won += 1,
            Some(_) => lost += 1,
            None => {}
        }
        ms += took;
        answers += thoughts;
        report.push_str(&format!("{i}\t{seed}\t{bot}\t{a}\t{b}\t{winner:?}\t{thoughts}\t{took:.0}\t{longest:.0}\n"));
    }
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("npc-bot.tsv");
    std::fs::write(&out, &report).unwrap();
    eprintln!("bot won {won}, lost {lost} of {duels}; {:.0} ms an answer on average; report {}", ms / answers.max(1) as f64, out.display());
    if std::env::var_os("YO3_BOT_RIVAL").is_none() {
        assert!(won > lost, "the bot should beat the placeholder CPU: won {won}, lost {lost}\n{report}");
    }
}

#[test]
fn every_npc_of_the_client_has_a_character() {
    // Styles and lines go by the client's NPC names (npc.rs); a name that doesn't match plays without them, as
    // "Maximillion Pegasus " (a trailing space in the text table) did until names were trimmed.
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    if !game_dir.join("package/data002.ypk").is_file() {
        return;
    }
    let game = GameData::load(&game_dir).expect("the client's data loads");
    for id in game.npc_ids() {
        let name = game.npc_name(id).unwrap_or_default();
        assert!(yo3_server::npc::style(name).is_object(), "NPC {id} {name:?} has no character in npc.rs");
        assert!(yo3_server::banter::table().contains_key(name), "NPC {id} {name:?} has no lines in npc_lines.txt");
    }
}

#[test]
fn pegasus_reads_a_real_duel_and_says_so() {
    // The NPCs' lines (crate::banter) from a real duel's engine messages: Pegasus (team 1, the bot) against Kaiba's
    // deck (team 0, the bot too). His first turn reads a card of team 0's hand through the engine's state, a Set card is
    // named as it is Set, and the duel's end has its line. Every card named is one of team 0's.
    use yo3_server::banter::{Banter, Live, table};
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let engine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !game_dir.join("package/data002.ypk").is_file() || !engine_dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("the game or the engine missing; skipping");
        return;
    }
    let game = GameData::load(&game_dir).expect("the client's data loads");
    let mut world = yo3_server::front::World::new(vec![]);
    for c in game.cards.iter() {
        world.card_names.entry(c.cid).or_insert_with(|| c.name.clone());
        let stats = yo3_server::translate::CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race };
        world.cards.insert(c.row, stats);
    }
    let deck = |name: &str| {
        let id = game.npc_ids().into_iter().find(|&id| game.npc_name(id) == Some(name)).expect("the NPC");
        let d = game.npc_deck(id).expect("its deck");
        let cid = |rows: &[u16]| -> Vec<u16> { rows.iter().filter_map(|r| game.cards.by_row(*r)).map(|c| c.cid).collect() };
        (cid(&d.main), cid(&d.extra))
    };
    let (kaiba, pegasus) = (deck("Seto Kaiba"), deck("Maximillion Pegasus"));
    let theirs: Vec<&str> = kaiba.0.iter().chain(&kaiba.1).filter_map(|c| world.card_names.get(c)).map(String::as_str).collect();
    let mut engine = Engine::start(&engine_dir).expect("the engine program runs");
    let mut answer = engine.new_duel_with(5, 8000, [(&kaiba.0, &kaiba.1), (&pegasus.0, &pegasus.1)], &world.card_names, &[]).unwrap();
    let mut banter = Banter::new("Maximillion Pegasus", 1, "Tester", 5).expect("his lines");
    let mut said: Vec<String> = banter.greet().into_iter().map(|s| s.text).collect();
    for _ in 0..6000 {
        said.extend(banter.observe(&answer.messages, &mut Live { engine: &mut engine, world: &world }).into_iter().map(|s| s.text));
        if answer.status != "await" {
            break;
        }
        let response = engine.think(serde_json::json!({ "sees": true })).expect("an answer");
        answer = engine.respond(response).expect("the engine answers");
    }
    eprintln!("{}", said.join("\n"));
    let lines = &table()["Maximillion Pegasus"];
    let kind = |text: &str| lines.iter().find(|(_, l)| l.iter().any(|l| {
        let (head, tail) = l.split_once("{card}").unwrap_or((l, ""));
        let (head, tail) = (head.replace("{you}", "Tester"), tail.replace("{you}", "Tester"));
        text.starts_with(&head) && text.ends_with(&tail)
    })).map(|(k, _)| *k);
    assert_eq!(kind(&said[0]), Some("greet"));
    let eye: Vec<&String> = said.iter().filter(|s| matches!(kind(s), Some("eye_hand" | "eye_set"))).collect();
    assert!(!eye.is_empty(), "his Millennium Eye said nothing: {said:?}");
    for line in &eye {
        assert!(theirs.iter().any(|name| line.contains(name)), "not one of Kaiba's cards: {line}");
    }
    assert!(matches!(said.last().and_then(|s| kind(s)), Some("win" | "lose")), "the end: {said:?}");
}
