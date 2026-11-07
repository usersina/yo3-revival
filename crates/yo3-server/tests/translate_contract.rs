//! The translator: ocgcore messages (as the engine sidecar reports them) to the client's duel stream, and the
//! client's card commands back to ocgcore responses.
use std::collections::HashMap;

use serde_json::json;
use yo3_proto::{Message, Value};
use yo3_server::duel::CardCommand;
use yo3_server::translate::{Prompt, Translator};

/// Rows 100.. (cid 4000 + row) for the client, 200.. for the CPU; the client goes first.
fn translator() -> (Translator, Vec<Message>) {
    let cids = (100..110).chain(200..210).map(|r| (r, 4000 + r)).collect::<HashMap<_, _>>();
    let client: Vec<u16> = (100..110).collect();
    let cpu: Vec<u16> = (200..210).collect();
    Translator::new(0, [&client, &cpu], cids)
}

fn cmd(m: &Message) -> (u32, [u32; 4]) {
    let int = |k| m.get(k).and_then(Value::as_u32).unwrap();
    (int("id"), [int("param1"), int("param2"), int("param3"), int("param4")])
}

#[test]
fn both_decks_go_into_the_card_table() {
    let (_, table) = translator();
    assert_eq!(table.len(), 20);
    assert_eq!(cmd(&table[0]), (1000, [1, 100, 0, 0xD << 8]));
    assert_eq!(cmd(&table[10]), (1000, [11, 200, 0, 1 | 0xD << 8]));
}

#[test]
fn a_draw_moves_the_card_with_that_id_to_the_hand() {
    // MSG_DRAW (90) {player: team, drawn: [{code, cid}]}: team 0 is the first player (here the client).
    let (mut t, _) = translator();
    let out = t.events(&[json!({ "type": 90, "player": 0, "drawn": [{ "code": 1, "cid": 4103 }] })]);
    let out: Vec<_> = out.iter().map(cmd).collect();
    // uid 4 holds row 103; it goes to hand index 0.
    assert_eq!(out, [(1000, [4, 103, 0, 0xB << 8]), (0x1A, [4 | 103 << 16, 0xD << 1, 0xB << 1, 0])]);
    let out = t.events(&[json!({ "type": 90, "player": 1, "drawn": [{ "code": 2, "cid": 4205 }] })]);
    assert_eq!(cmd(&out[0]), (1000, [16, 205, 0, 1 | 0xB << 8]));
}

#[test]
fn turns_and_phases_follow_the_engine() {
    // MSG_NEW_TURN (40) {player}: the client starts on turn 1, so only later turns send 6. MSG_NEW_PHASE (41):
    // ocgcore phases 1, 2, 4, 8..0x80, 0x100, 0x200 are the panel's 0-5.
    let (mut t, _) = translator();
    let out = t.events(&[
        json!({ "type": 40, "player": 0 }),
        json!({ "type": 41, "phase": 1 }),
        json!({ "type": 41, "phase": 4 }),
        json!({ "type": 40, "player": 1 }),
        json!({ "type": 41, "phase": 0x200 }),
    ]);
    let out: Vec<_> = out.iter().map(cmd).collect();
    assert_eq!(out, [(5, [0, 0, 0, 0]), (5, [0, 2, 0, 0]), (6, [1, 0, 0, 0]), (5, [1, 5, 0, 0])]);
}

#[test]
fn the_clients_idle_prompt_becomes_a_command_map_and_an_input_wait() {
    // MSG_SELECT_IDLECMD (11): each listed card gets its menu bit (summons 4, monster_sets 6, spell_sets 7,
    // activates 3...) at its card-table place.
    let (mut t, _) = translator();
    t.events(&[json!({ "type": 90, "player": 0, "drawn": [{ "cid": 4100 }, { "cid": 4101 }] })]);
    let hand = |seq| json!({ "controller": 0, "location": 2, "sequence": seq });
    let idle = json!({ "type": 11, "player": 0, "summons": [hand(0)], "special_summons": [], "pos_changes": [],
        "monster_sets": [hand(0)], "spell_sets": [hand(1)], "activates": [hand(1)], "to_bp": true, "to_ep": true });
    let Prompt::Client(out) = t.prompt(&idle) else { panic!("the client chooses") };
    // Then the other flags: phase buttons 3 (BP) and 5 (EP) enabled, as the engine allows (0x501C00 reads them).
    let data = [0x00, 0x00, 0x0B, 0x00, 0, 0, 0, 0x50, 0x00, 0x01, 0x0B, 0x00, 0, 0, 0, 0x88, 3, 1, 5, 1];
    assert_eq!(out[0].get("COMIDDATA"), Some(&Value::Bin(data.to_vec())));
    assert_eq!(out[0].get("otherFlagSize"), Some(&Value::U32(2)));
    // Command 4 {mode, player}: mode 2 or 3 turns the phase panel on (0x504890).
    assert_eq!(cmd(&out[1]), (4, [2, 0, 0, 0]));
    // Summon on hand card 0 is summons[0]; Set (bit 7) on hand card 1 is spell_sets[0].
    let summon = CardCommand { command: 4, player: 0, zone: 0xB, index: 0 };
    assert_eq!(t.answer(summon), Some(json!({ "type": 1, "action": 0, "index": 0 })));
    t.prompt(&idle);
    let set = CardCommand { command: 7, player: 0, zone: 0xB, index: 1 };
    assert_eq!(t.answer(set), Some(json!({ "type": 1, "action": 4, "index": 0 })));
    // A command the prompt doesn't list is not an answer.
    t.prompt(&idle);
    assert_eq!(t.answer(CardCommand { command: 0, player: 0, zone: 0xB, index: 0 }), None);
}

#[test]
fn the_cpu_ends_its_turn_and_never_chains() {
    let (mut t, _) = translator();
    let idle = json!({ "type": 11, "player": 1, "summons": [], "to_bp": true, "to_ep": true });
    assert_eq!(t.prompt(&idle), Prompt::Answer(json!({ "type": 1, "action": 7, "index": null })));
    let chain = json!({ "type": 16, "player": 1, "forced": false, "selects": [] });
    assert_eq!(t.prompt(&chain), Prompt::Answer(json!({ "type": 8, "index": null })));
}

#[test]
fn a_summon_moves_the_card_from_the_hand_to_its_zone() {
    // MSG_MOVE (50) {card, cid, from, to: {controller, location, sequence, position}}. Hand 2 → monster zone 4
    // (sequence = slot), face-up attack (position 1). Move places: player | zone << 1 | index << 6, face up bit
    // 14, defense bit 15 (0x5000E0); field slots are zones 0-10 with index 0. The card left behind in the hand
    // moves up and gets its table entry again.
    let (mut t, _) = translator();
    t.events(&[json!({ "type": 90, "player": 0, "drawn": [{ "cid": 4100 }, { "cid": 4101 }] })]);
    let out = t.events(&[json!({ "type": 50, "cid": 4100,
        "from": { "controller": 0, "location": 2, "sequence": 0, "position": 10 },
        "to": { "controller": 0, "location": 4, "sequence": 2, "position": 1 } })]);
    let out: Vec<_> = out.iter().map(cmd).collect();
    assert_eq!(
        out,
        [(1000, [1, 100, 0, 2 << 8]), (0x1A, [1 | 100 << 16, 0xB << 1, 2 << 1 | 1 << 14, 0]), (1000, [2, 101, 0, 0xB << 8])]
    );
    // A face-down defense Set to the Spell/Trap zone 1 (zone 6).
    let out = t.events(&[json!({ "type": 50, "cid": 4101,
        "from": { "controller": 0, "location": 2, "sequence": 0, "position": 10 },
        "to": { "controller": 0, "location": 8, "sequence": 1, "position": 8 } })]);
    assert_eq!(cmd(&out[1]), (0x1A, [2 | 101 << 16, 0xB << 1, 6 << 1 | 1 << 15, 0]));
}

#[test]
fn place_and_position_prompts_are_answered_for_the_player() {
    // MSG_SELECT_PLACE (18): the client has no window for it; the first free own zone is taken (field_mask bits
    // set = unavailable: monster zones 0-4, Spell/Trap 8-12). MSG_SELECT_POSITION (19): face-up attack if allowed.
    let (mut t, _) = translator();
    let place = json!({ "type": 18, "player": 0, "count": 1, "field_mask": 0xFFFF_E0FFu32 });
    let spell = json!({ "type": 10, "places": [{ "player": 0, "location": 8, "sequence": 0 }] });
    assert_eq!(t.prompt(&place), Prompt::Answer(spell));
    let place = json!({ "type": 18, "player": 0, "count": 1, "field_mask": 0xFFFF_FFE2u32 });
    let monster = json!({ "type": 10, "places": [{ "player": 0, "location": 4, "sequence": 0 }] });
    assert_eq!(t.prompt(&place), Prompt::Answer(monster));
    let position = json!({ "type": 19, "player": 0, "code": 1, "positions": 5 });
    assert_eq!(t.prompt(&position), Prompt::Answer(json!({ "type": 11, "position": 1 })));
}

#[test]
fn the_phase_buttons_end_the_main_phase() {
    // A phase button sends duelRequestName 3 {phase: the button, 0 DP .. 5 EP} (0x521180). In Main Phase, BP
    // answers TO_BP (6) and EP TO_EP (7), if the prompt allows them.
    let (mut t, _) = translator();
    let idle = json!({ "type": 11, "player": 0, "summons": [], "special_summons": [], "pos_changes": [],
        "monster_sets": [], "spell_sets": [], "activates": [], "to_bp": false, "to_ep": true });
    t.prompt(&idle);
    assert_eq!(t.phase(3), None, "no Battle Phase on this prompt");
    assert_eq!(t.phase(5), Some(json!({ "type": 1, "action": 7, "index": null })));
}
