//! The rules engine sidecar (engine/duelcore.ts). Skipped unless engine/fetch.sh has run.
use std::path::PathBuf;

use yo3_server::engine::Engine;

fn engine() -> Option<Engine> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
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
