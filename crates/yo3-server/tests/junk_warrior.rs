//! Junk Warrior (7696): "When this card is Synchro Summoned: it gains ATK equal to the total ATK of all Level 2 or lower
//! monsters you currently control." The materials have left the field by then, so only other Level 2 or lower
//! monsters count. Real core, both seats' client models checked after every step.
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::{Value as Json, json};
use yo3_proto::Message;
use yo3_server::{duel::command, duelcheck::ClientModel, engine::Engine, translate::{CardStats, Translator, bot_answer}};

const JUNK_WARRIOR: u16 = 7696;
const JUNK_SYNCHRON: u16 = 7687;
const HEDGEHOG: u16 = 7701; // Quillbolt Hedgehog: Level 2, 800 ATK
const WARWOLF: u16 = 6960;

fn engine() -> Option<Engine> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !dir.join("vendor/cardinfo.json").is_file() { eprintln!("engine not fetched; skipping"); return None }
    Some(Engine::start(&dir).expect("the engine program runs"))
}

fn on_field(state: &Json, cid: u16) -> Vec<Json> {
    state[0]["mzone"].as_array().unwrap().iter().filter(|c| c["cid"] == cid).cloned().collect()
}

/// Normal Summons `hedgehogs` Quillbolt Hedgehogs and a Junk Synchron over the turns, then Synchro Summons Junk Warrior
/// from Junk Synchron and one Hedgehog. Returns Junk Warrior's ATK as the core and both clients' models have it.
fn junk_warrior_after(hedgehogs: usize) -> Option<(i64, Vec<String>)> {
    let mut engine = engine()?;
    let deck: Vec<u16> = [HEDGEHOG, HEDGEHOG, JUNK_SYNCHRON].iter().copied().cycle().take(40).collect();
    let extra = [JUNK_WARRIOR];
    let other = vec![WARWOLF; 40];
    let cards: HashMap<_, _> = [JUNK_WARRIOR, JUNK_SYNCHRON, HEDGEHOG, WARWOLF].map(|c| (c, CardStats { cid: c, ..CardStats::default() })).into();
    let mut views: Vec<_> = (0..2).map(|seat| {
        let decks = [&deck[..], &other[..]];
        let extras: [&[u16]; 2] = [&extra, &[]];
        let (mut translator, mut table) = Translator::against_player(seat as u32, [decks[seat], decks[1 - seat]], cards.clone());
        translator.tuners([JUNK_SYNCHRON].into());
        table.extend(translator.extra_decks([extras[seat], extras[1 - seat]]));
        let mut model = ClientModel::new(seat as u32, cards.clone());
        let n = |v: usize| yo3_proto::Value::U32(v as u32);
        let setup = Message::reply(0x200D).with("duelRequestName", n(0x15)).with("MainDeckNum001", n(40)).with("MainDeckNum002", n(40))
            .with("ExtraDeckNum001", n(extras[seat].len())).with("ExtraDeckNum002", n(extras[1 - seat].len()));
        model.apply(&setup); model.apply_all(&table);
        model.apply(&command(0xF, [0, 8000, 0, 0])); model.apply(&command(0xF, [1, 8000, 0, 0]));
        for zone in [0xD, 0xC] { model.apply(&command(0x16, [0, zone, 0, 0])); model.apply(&command(0x16, [1, zone, 0, 0])); }
        (translator, model)
    }).collect();
    let mut answer = engine.new_duel(7, 8000, [(&deck, &extra), (&other, &[])]).unwrap();
    let (mut hedgehogs_out, mut synchron_out) = (0, false);
    let mut mismatches = vec![];
    for step in 0..400 {
        assert!(answer.errors.is_empty(), "{:?}", answer.errors);
        let state = engine.state().unwrap();
        for (seat, (translator, model)) in views.iter_mut().enumerate() {
            model.apply_all(&translator.events(&answer.messages));
            for problem in model.check(&state) { mismatches.push(format!("seat {seat}, step {step}: {problem}")); }
        }
        if let Some(warrior) = on_field(&state, JUNK_WARRIOR).first()
            && !answer.messages.iter().any(|m| m["type"] == 72 || m["type"] == 160) && answer.status == "await"
            && answer.messages.last().is_some_and(|m| m["type"] == 11 && m["player"] == 0) {
            return Some((warrior["attack"].as_i64().unwrap(), mismatches));
        }
        assert_eq!(answer.status, "await", "the duel ended before the summon");
        let prompt = answer.messages.last().unwrap();
        let response = if prompt["type"] == 11 && prompt["player"] == 0 {
            let find = |list: &str, cid: u16| prompt[list].as_array().unwrap().iter().position(|c| c["cid"] == cid);
            if hedgehogs_out < hedgehogs && let Some(i) = find("summons", HEDGEHOG) {
                hedgehogs_out += 1; json!({"type":1,"action":0,"index":i})
            } else if hedgehogs_out == hedgehogs && !synchron_out && let Some(i) = find("summons", JUNK_SYNCHRON) {
                synchron_out = true; json!({"type":1,"action":0,"index":i})
            } else if synchron_out && let Some(i) = find("special_summons", JUNK_WARRIOR) {
                json!({"type":1,"action":1,"index":i})
            } else { json!({"type":1,"action":7,"index":null}) }
        } else if prompt["type"] == 11 { json!({"type":1,"action":7,"index":null}) }
        else { bot_answer(prompt) };
        answer = engine.respond(response).unwrap();
    }
    panic!("Junk Warrior was not Synchro Summoned with {hedgehogs} Hedgehogs");
}

#[test]
fn junk_warrior_alone_keeps_its_attack() {
    let Some((atk, mismatches)) = junk_warrior_after(1) else { return };
    assert_eq!(atk, 2300, "the materials have left the field");
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn junk_warrior_gains_a_level_2_monster_left_on_the_field() {
    let Some((atk, mismatches)) = junk_warrior_after(2) else { return };
    assert_eq!(atk, 2300 + 800, "one Hedgehog stayed on the field");
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}
