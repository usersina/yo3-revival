//! Real-core regressions for Yusei's battle protection and Synchro material restrictions.
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::json;
use yo3_proto::{Message, Value};
use yo3_server::{duel::command, duelcheck::ClientModel, engine::Engine, translate::{CardStats, Translator, bot_answer}};

fn engine() -> Option<Engine> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !dir.join("vendor/cardinfo.json").is_file() { eprintln!("engine not fetched; skipping"); return None }
    Some(Engine::start(&dir).expect("node runs the engine"))
}
fn number(m: &Message, key: &str) -> u32 { m.get(key).and_then(Value::as_u32).unwrap_or(0) }

fn protected_battles(cid: u16, protections: usize, set: bool) {
    let Some(mut engine) = engine() else { return };
    let defender = vec![cid; 40]; let attackers = vec![6960; 40]; // Gene-Warped Warwolf, 2000 ATK.
    let decks = [&defender[..], &attackers[..]];
    let cards: HashMap<_, _> = [cid, 6960].map(|c| (c, CardStats { cid: c, ..CardStats::default() })).into();
    let mut views: Vec<_> = (0..2).map(|seat| {
        let (translator, table) = Translator::against_player(seat as u32, [decks[seat], decks[1-seat]], cards.clone());
        let mut model = ClientModel::new(seat as u32, cards.clone());
        let setup = ["MainDeckNum001", "MainDeckNum002"].into_iter().fold(
            Message::reply(0x200D).with("duelRequestName", Value::U32(0x15)), |m, k| m.with(k, Value::U32(40)));
        model.apply(&setup); model.apply_all(&table);
        model.apply(&command(0x16, [0, 0xD, 0, 0])); model.apply(&command(0x16, [1, 0xD, 0, 0]));
        (translator, model)
    }).collect();
    let mut answer = engine.new_duel(7, 50000, [(&defender, &[]), (&attackers, &[])]).unwrap();
    let (mut summoned, mut survived, mut battles) = (false, 0, 0);
    for step in 0..500 {
        assert!(answer.errors.is_empty(), "{:?}", answer.errors);
        let state = engine.state().unwrap();
        let battle = answer.messages.iter().any(|m| m["type"] == 111);
        let alive = state[0]["mzone"].as_array().unwrap().iter().any(|c| c["cid"] == cid);
        if battle { battles += 1; if alive { survived += 1; } }
        for (seat, (translator, model)) in views.iter_mut().enumerate() {
            let out = translator.events(&answer.messages);
            // Native BattleRun's destroy bit hides the card (0x4F8920 -> 0x4FB220), without removing its slot.
            if battle && alive {
                for m in out.iter().filter(|m| number(m,"id") == 0xD) {
                    let mask = 2 << (seat * 8);
                    assert_eq!(number(m,"param3") & mask, 0,
                        "seat {seat}, step {step}: core kept card {cid} on the field, but BattleRun hides it: {m:?}");
                }
            }
            model.apply_all(&out);
            assert!(model.check(&state).is_empty(), "seat {seat}, step {step}: {:?}", answer.messages);
        }
        if summoned && !alive && battle && state[0]["grave"].as_array().unwrap().iter().any(|c| c["cid"] == cid) {
            assert!(survived >= protections, "no protected battle was reached");
            assert!(battles > survived, "the exhausted protection must allow a later destruction");
            return;
        }
        assert_eq!(answer.status, "await");
        let prompt = answer.messages.last().unwrap();
        let response = match prompt["type"].as_u64() {
            Some(11) => {
                let player = prompt["player"].as_u64().unwrap();
                let list = if player == 0 && set { "monster_sets" } else { "summons" };
                if (player == 1 || !summoned) && prompt[list].as_array().is_some_and(|v| !v.is_empty()) {
                    if player == 0 { summoned = true; }
                    json!({"type":1,"action":if player == 0 && set {3} else {0},"index":0})
                } else { json!({"type":1,"action":if player == 1 && prompt["to_bp"] == true {6} else {7},"index":null}) }
            },
            _ => bot_answer(prompt),
        };
        answer = engine.respond(response).unwrap();
    }
    panic!("card {cid}: no exhausted protection after {battles} battles ({survived} survived)");
}

#[test]
fn fortress_warrior_survives_visibly_and_later_dies_in_both_views() { protected_battles(7985, 1, false); }
#[test]
fn a_set_fortress_warrior_flips_and_survives_visibly() { protected_battles(7985, 1, true); }
#[test]
fn shield_wing_survives_visibly_until_its_protection_is_exhausted() { protected_battles(7990, 2, false); }

fn stardust_materials(materials: &[u16], legal: bool) {
    let Some(mut engine) = engine() else { return };
    let deck: Vec<_> = materials.iter().copied().cycle().take(40).collect();
    let mut answer = engine.new_duel(7, 8000, [(&deck, &[7734]), (&[6960; 40], &[])]).unwrap();
    let (mut next, mut summoning) = (0, false);
    for _ in 0..200 {
        assert!(answer.errors.is_empty(), "{:?}", answer.errors);
        let state = engine.state().unwrap();
        if state[0]["mzone"].as_array().unwrap().iter().any(|c| c["cid"] == 7734) {
            assert!(legal && summoning);
            for cid in materials {
                assert!(state[0]["grave"].as_array().unwrap().iter().any(|c| c["cid"] == *cid), "material {cid} wasn't used");
            }
            return;
        }
        assert_eq!(answer.status, "await");
        let prompt = answer.messages.last().unwrap();
        let response = if prompt["type"] == 11 && prompt["player"] == 0 {
            if next < materials.len() {
                let index = prompt["summons"].as_array().unwrap().iter().position(|c| c["cid"] == materials[next]);
                if let Some(index) = index { next += 1; json!({"type":1,"action":0,"index":index}) }
                else { json!({"type":1,"action":7,"index":null}) }
            } else {
                let index = prompt["special_summons"].as_array().unwrap().iter().position(|c| c["cid"] == 7734);
                assert_eq!(index.is_some(), legal, "Stardust with {materials:?}: {prompt}");
                if !legal { return }
                summoning = true; json!({"type":1,"action":1,"index":index.unwrap()})
            }
        } else if prompt["type"] == 11 { json!({"type":1,"action":7,"index":null}) }
        else { bot_answer(prompt) };
        answer = engine.respond(response).unwrap();
    }
    panic!("Stardust material scenario did not finish");
}

#[test]
fn debris_dragon_and_max_warrior_cannot_synchro_summon_stardust() { stardust_materials(&[8005, 8438], false); }
#[test]
fn debris_dragon_can_use_fortress_warrior_and_quillbolt_for_stardust() { stardust_materials(&[8005, 7985, 7701], true); }

#[test]
fn original_yusei_decks_stay_in_sync_for_both_seats() {
    let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    if !game.join("yo3.exe").is_file() { eprintln!("game not installed; skipping original deck fixtures"); return }
    let data = yo3_data::GameData::load(&game).unwrap();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !dir.join("vendor/cardinfo.json").is_file() { eprintln!("engine not fetched; skipping"); return }
    let mut battles = 0;
    for npc in [171, 172, 173] {
        let deck = data.npc_deck(npc).unwrap();
        let cids = |rows: &[u16]| rows.iter().map(|r| data.cards.by_row(*r).unwrap().cid).collect::<Vec<_>>();
        let (main, extra) = (cids(&deck.main), cids(&deck.extra));
        for seed in 7..11 {
            for seat in 0..2 {
                let played = yo3_server::duelcheck::selfplay::play(&dir, seed, [(&main, &extra), (&main, &extra)], seat, true)
                    .unwrap_or_else(|e| panic!("Yusei NPC {npc}: {e}"));
                battles += played.kinds.get(&111).copied().unwrap_or(0);
            }
        }
    }
    assert!(battles > 0, "the original Yusei decks must reach battle");
    eprintln!("24 original-Yusei self-play checks, {battles} battles across both views");
}
