//! The rules engine sidecar (engine/duelcore.ts). Skipped unless engine/fetch.sh has run.
use std::path::PathBuf;

use yo3_server::engine::Engine;

fn engine() -> Option<Engine> {
    let dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    Some(Engine::start(&dir).expect("node runs the engine"))
}

#[test]
fn a_duel_deals_five_each_and_asks_the_first_player() {
    let Some(mut engine) = engine() else { return };
    // 4007 = Blue-Eyes White Dragon, a Normal Monster (no script needed).
    let deck = [4007u16; 40];
    let answer = engine.new_duel(7, 8000, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    assert_eq!(answer.status, "await", "{answer:?}");
    assert!(answer.errors.is_empty(), "{:?}", answer.errors);
    // MSG_DRAW (90) for both players, with the client's card ids.
    let draws: Vec<_> = answer.messages.iter().filter(|m| m["type"] == 90).collect();
    assert!(draws.len() >= 2);
    for (player, draw) in draws.iter().take(2).enumerate() {
        assert_eq!(draw["player"], player);
        let drawn = draw["drawn"].as_array().unwrap();
        assert_eq!(drawn.len(), 5);
        assert!(drawn.iter().all(|c| c["cid"] == 4007 && c["code"] == 89631139));
    }
}

#[test]
fn two_placeholder_ais_play_a_duel_to_the_end() {
    // Both players answered by the placeholder AI: the engine and the AI must never stall. 6960 = Gene-Warped
    // Warwolf (level 4, 2000 ATK), 5004 = Vorse Raider (level 4, 1900 ATK), both Normal Monsters.
    let Some(mut engine) = engine() else { return };
    let deck: Vec<u16> = [6960u16; 20].into_iter().chain([5004; 20]).collect();
    let mut answer = engine.new_duel(11, 8000, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    let mut won = false;
    for _ in 0..5_000 {
        won |= answer.messages.iter().any(|m| m["type"] == 5);
        if answer.status != "await" {
            break;
        }
        let prompt = answer.messages.last().expect("a prompt").clone();
        answer = engine.respond(yo3_server::translate::bot_answer(&prompt)).expect("the engine answers");
    }
    if answer.status != "end" {
        for m in answer.messages.iter().rev().take(3) {
            eprintln!("last: {m}");
        }
    }
    assert_eq!(answer.status, "end", "the duel finished");
    assert!(won, "someone won (MSG_WIN)");
}

#[test]
fn a_hand_shuffle_names_the_cards_by_their_client_ids() {
    // After a search the engine shows the card and shuffles the hand: MSG_SHUFFLE_HAND (33) {player, cards} with bare
    // passcodes, which the sidecar gives `cids` beside, so the translator can follow the new order (a missed shuffle
    // put the client's hand out of step and crashed it). 4054 = Sangan (destroyed, it adds a monster with 1500 or less
    // ATK from the Deck: another Sangan), against 6960 = Gene-Warped Warwolf (2000 ATK), both sides played by the AI.
    let Some(mut engine) = engine() else { return };
    let sangans = [4054u16; 40];
    let warwolves = [6960u16; 40];
    let mut answer = engine.new_duel(5, 8000, [(&sangans, &[]), (&warwolves, &[])]).expect("a duel");
    let mut shuffles = vec![];
    for _ in 0..5_000 {
        shuffles.extend(answer.messages.iter().filter(|m| m["type"] == 33).cloned());
        if answer.status != "await" || !shuffles.is_empty() {
            break;
        }
        let prompt = answer.messages.last().expect("a prompt").clone();
        answer = engine.respond(yo3_server::translate::bot_answer(&prompt)).expect("the engine answers");
    }
    let shuffle = shuffles.first().expect("a hand shuffle after Sangan's search");
    let (cards, cids) = (shuffle["cards"].as_array().unwrap(), shuffle["cids"].as_array().expect("cids beside the cards"));
    assert_eq!(cards.len(), cids.len());
    assert!(cids.iter().all(|c| c == 4054), "{shuffle}");
}

#[test]
fn moves_carry_their_reason() {
    // ocgcore-wasm drops MSG_MOVE's reason; the sidecar reads it back with a query at the card's new place. Normal
    // Monsters leave the field only by battle: REASON_DESTROY (1) | REASON_BATTLE (0x20).
    let Some(mut engine) = engine() else { return };
    let deck: Vec<u16> = [6960u16; 20].into_iter().chain([5004; 20]).collect();
    let mut answer = engine.new_duel(11, 8000, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    let mut destroyed = 0;
    for _ in 0..5_000 {
        for m in answer.messages.iter().filter(|m| m["type"] == 50) {
            assert!(m["reason"].is_u64(), "a move without its reason: {m}");
            if m["from"]["location"] == 4 && m["to"]["location"] == 16 {
                assert_eq!(m["reason"].as_u64().unwrap() & 0x21, 0x21, "{m}");
                destroyed += 1;
            }
        }
        if answer.status != "await" {
            break;
        }
        let prompt = answer.messages.last().expect("a prompt").clone();
        answer = engine.respond(yo3_server::translate::bot_answer(&prompt)).expect("the engine answers");
    }
    assert!(destroyed > 0, "no monster was destroyed");
}

#[test]
fn the_field_report_follows_the_monsters_stats() {
    // The engine reports no stat changes: after each step the sidecar reports both monster zones when they changed,
    // {"type": "field", monsters: [{controller, sequence, position, level, attack, defense, base_attack,
    // base_defense}]}, before the step's prompt. 6960 = Gene-Warped Warwolf (level 4, 2000/100).
    let Some(mut engine) = engine() else { return };
    let deck = [6960u16; 40];
    let mut answer = engine.new_duel(11, 8000, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    let mut seen = 0;
    for _ in 0..200 {
        if answer.status == "await" {
            assert!(answer.messages.last().unwrap()["type"].is_u64(), "the prompt stays last");
        }
        for m in answer.messages.iter().filter(|m| m["type"] == "field") {
            for w in m["monsters"].as_array().unwrap() {
                let stats = ["level", "attack", "defense", "base_attack", "base_defense"].map(|k| w[k].as_u64());
                assert_eq!(stats, [Some(4), Some(2000), Some(100), Some(2000), Some(100)], "{w}");
                assert!(w["sequence"].as_u64().unwrap() < 5 && w["position"].is_u64(), "{w}");
                seen += 1;
            }
        }
        if answer.status != "await" || seen >= 2 {
            break;
        }
        let prompt = answer.messages.last().expect("a prompt").clone();
        answer = engine.respond(yo3_server::translate::bot_answer(&prompt)).expect("the engine answers");
    }
    assert!(seen >= 2, "no monster reported");
}

#[test]
fn declarations_reach_the_core_in_its_own_terms() {
    // Abyssal Designator (6132) asks for an Attribute and a Type (MSG_ANNOUNCE_ATTRIB 141, MSG_ANNOUNCE_RACE 140),
    // Prohibition (4968) for a card name (MSG_ANNOUNCE_CARD 142). The answers come as the translator gives them:
    // attributes as numbers, races as numbers (the core's are 64-bit: the sidecar makes them BigInts), a card by its
    // Konami id (the sidecar finds its passcode).
    let Some(mut engine) = engine() else { return };
    // With monsters (Giant Orc, 5586) for Abyssal Designator to name; interleaved, as the core deals the cards last
    // added first.
    let deck: Vec<u16> = [5586u16, 4968, 6132].into_iter().cycle().take(40).collect();
    let mut answer = engine.new_duel(7, 8000, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    let mut asked = std::collections::BTreeSet::new();
    for _ in 0..200 {
        assert!(answer.errors.is_empty(), "{:?}", answer.errors);
        assert_ne!(answer.status, "error", "{answer:?}");
        if answer.status != "await" || asked.len() == 3 {
            break;
        }
        let prompt = answer.messages.last().expect("a prompt").clone();
        let first_bit = |k: &str| 1u64 << prompt[k].as_u64().unwrap_or(1).trailing_zeros();
        let response = match prompt["type"].as_u64() {
            // Abyssal Designator first, then Prohibition.
            Some(11) if prompt["activates"].as_array().is_some_and(|a| !a.is_empty()) => {
                let activates = prompt["activates"].as_array().unwrap();
                let wanted = if asked.contains(&140) { 4968 } else { 6132 };
                let index = activates.iter().position(|a| a["cid"] == wanted).unwrap_or(0);
                serde_json::json!({ "type": 1, "action": 5, "index": index })
            }
            Some(kind @ 140) => {
                asked.insert(kind);
                serde_json::json!({ "type": 16, "races": [first_bit("available")] })
            }
            Some(kind @ 141) => {
                asked.insert(kind);
                serde_json::json!({ "type": 17, "attributes": [first_bit("available")] })
            }
            Some(kind @ 142) => {
                asked.insert(kind);
                serde_json::json!({ "type": 18, "cid": 4007 })
            }
            _ => yo3_server::translate::bot_answer(&prompt),
        };
        answer = engine.respond(response).expect("the engine answers");
    }
    assert!(answer.errors.is_empty(), "{:?}", answer.errors);
    assert_eq!(asked.into_iter().collect::<Vec<_>>(), [140, 141, 142]);
}

#[test]
fn a_cards_own_options_are_its_script_strings() {
    // MSG_SELECT_OPTION's options are aux.Stringid values, code << 20 | n (utility.lua): Card Shuffle (5529, passcode
    // 12183332) asks between its strings 1 and 2 when its effect resolves.
    let Some(mut engine) = engine() else { return };
    let deck = [5529u16; 40];
    let mut answer = engine.new_duel(7, 8000, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    let mut used = 0;
    for _ in 0..40 {
        let prompt = answer.messages.last().expect("a prompt").clone();
        let response = match prompt["type"].as_u64() {
            Some(14) => {
                let options: Vec<u64> = prompt["options"].as_array().unwrap().iter().filter_map(|o| o.as_u64()).collect();
                assert_eq!(options, [12183332 << 20 | 1, 12183332 << 20 | 2]);
                return;
            }
            // Activate the last card offered (the one on the field, once there), twice.
            Some(11) if used < 2 && prompt["activates"].as_array().is_some_and(|a| !a.is_empty()) => {
                used += 1;
                serde_json::json!({ "type": 1, "action": 5, "index": prompt["activates"].as_array().unwrap().len() - 1 })
            }
            _ => yo3_server::translate::bot_answer(&prompt),
        };
        answer = engine.respond(response).expect("the engine answers");
    }
    panic!("Card Shuffle never asked");
}

#[test]
fn a_synchro_summon_is_offered_from_the_extra_deck() {
    // Junk Synchron (7687, a Level 3 Tuner) and Giant Orc (5586, Level 4) on the field: the idle prompt lists Ancient
    // Fairy Dragon (8090, a Level 7 Synchro for any Tuner) among its Special Summons, from the extra deck (location 64).
    let Some(mut engine) = engine() else { return };
    let deck: Vec<u16> = [7687u16, 5586].into_iter().cycle().take(40).collect();
    let mut answer = engine.new_duel(7, 8000, [(&deck, &[8090]), (&deck, &[])]).expect("a duel");
    for _ in 0..60 {
        let prompt = answer.messages.last().expect("a prompt").clone();
        if prompt["type"] == 11 && prompt["player"] == 0 {
            let extra = prompt["special_summons"].as_array().unwrap().iter().find(|c| c["location"] == 64).cloned();
            if let Some(extra) = extra {
                assert_eq!(extra["cid"], 8090);
                // Chosen: the core asks for the materials (MSG_SELECT_UNSELECT_CARD, 26: the Tuner first, then the
                // rest), sends both to the Graveyard and moves the Synchro from the extra deck to a monster zone.
                let index = prompt["special_summons"].as_array().unwrap().iter().position(|c| c["location"] == 64);
                let mut answer = engine.respond(serde_json::json!({ "type": 1, "action": 1, "index": index })).expect("the engine answers");
                let mut seen = vec![];
                for _ in 0..20 {
                    seen.extend(answer.messages.clone());
                    let prompt = answer.messages.last().expect("a prompt").clone();
                    if prompt["type"] == 11 {
                        break;
                    }
                    answer = engine.respond(yo3_server::translate::bot_answer(&prompt)).expect("the engine answers");
                }
                let materials = seen.iter().find(|m| m["type"] == 26).expect("the materials are chosen");
                assert_eq!(materials["select_cards"][0]["cid"], 7687, "the Tuner first");
                let summoned = |m: &&serde_json::Value| m["type"] == 50 && m["from"]["location"] == 64 && m["to"]["location"] == 4;
                assert_eq!(seen.iter().find(summoned).map(|m| m["cid"].clone()), Some(8090.into()));
                return;
            }
        }
        // The first player summons a monster a turn; both end their turns without attacking.
        let response = match prompt["type"].as_u64() {
            Some(11) if prompt["player"] == 0 && prompt["summons"].as_array().is_some_and(|a| !a.is_empty()) => {
                serde_json::json!({ "type": 1, "action": 0, "index": 0 })
            }
            Some(11) => serde_json::json!({ "type": 1, "action": 7, "index": null }),
            _ => yo3_server::translate::bot_answer(&prompt),
        };
        answer = engine.respond(response).expect("the engine answers");
    }
    panic!("Ancient Fairy Dragon was never offered");
}

#[test]
fn a_sidecar_that_never_answers_times_out_and_is_killed() {
    // A call must not block forever: use an installed native sleeper so the timeout is exercised on both platforms.
    let mut sleeper = std::process::Command::new(if cfg!(windows) { "powershell.exe" } else { "sleep" });
    if cfg!(windows) { sleeper.args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 60"]); }
    else { sleeper.arg("60"); }
    let mut engine = Engine::spawn(sleeper, std::time::Duration::from_millis(300)).expect("sleeper runs");
    let started = std::time::Instant::now();
    let err = engine.respond(serde_json::json!({})).unwrap_err();
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert!(matches!(err.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::BrokenPipe), "{err}");
}

#[test]
fn a_sidecar_that_exits_fails_the_call() {
    let mut engine_cmd = std::process::Command::new(if cfg!(windows) { "cmd.exe" } else { "true" });
    if cfg!(windows) { engine_cmd.args(["/D", "/C", "exit", "0"]); }
    let mut engine = Engine::spawn(engine_cmd, std::time::Duration::from_secs(5)).expect("exit fixture runs");
    assert!(engine.respond(serde_json::json!({})).is_err());
}

#[test]
fn diagnostic_identity_covers_the_core_scripts_database_and_sidecar() {
    let Some(mut engine) = engine() else { return };
    let identity = engine.identity().expect("diagnostic identity");
    for key in ["sidecar", "core", "bindings", "scripts", "database", "card_ids"] {
        let hash = identity[key].as_str().expect(key);
        assert_eq!(hash.len(), 64, "SHA-256 for {key}: {identity}");
    }
    assert_eq!(engine.identity().unwrap(), identity, "identity is stable within a duel process");
}

#[test]
fn an_unsupported_dependency_identity_cannot_count_as_a_valid_recording() {
    let mut old = std::process::Command::new(if cfg!(windows) { "powershell.exe" } else { "printf" });
    if cfg!(windows) { old.args(["-NoProfile", "-NonInteractive", "-Command", "[Console]::Out.WriteLine('{\"status\":\"error\",\"error\":\"unknown op identity\"}')"]); }
    else { old.args(["%s\n", "{\"status\":\"error\",\"error\":\"unknown op identity\"}"]); }
    let mut engine = Engine::spawn(old, std::time::Duration::from_secs(5)).unwrap();
    assert!(engine.identity().is_err(), "an older sidecar did not provide the required fingerprints");
}

#[test]
fn the_independent_state_query_includes_the_new_oracle_fields() {
    let Some(mut engine) = engine() else { return };
    let deck = [6960u16; 40];
    engine.new_duel(7, 8000, [(&deck, &[]), (&deck, &[])]).unwrap();
    let state = engine.state().unwrap();
    for team in state.as_array().unwrap() {
        assert_eq!(team["lp"], 8000);
        for card in team["hand"].as_array().unwrap() {
            for key in ["level", "attack", "defense", "base_attack", "base_defense"] {
                assert!(card[key].is_i64(), "missing {key}: {card}");
            }
            assert!(card["counters"].is_object());
        }
    }
}
