//! The translator: ocgcore messages (as the engine sidecar reports them) to the client's duel stream, and the
//! client's card commands back to ocgcore responses.
use std::collections::HashMap;

use serde_json::json;
use yo3_proto::{Message, Value};
use yo3_server::duel::CardCommand;
use yo3_server::translate::{CardStats, Prompt, Translator};

/// Rows 100.. (cid 4000 + row, ATK 10 × row, DEF 1000, level 4, attribute 2, race 12) for the client, 200.. for
/// the CPU; the client goes first.
fn translator() -> (Translator, Vec<Message>) {
    let stats = |r: u16| CardStats { cid: 4000 + r, atk: 10 * r as u32, def: 1000, level: 4, attribute: 2, race: 12 };
    let cids = (100..110).chain(200..210).map(|r| (r, stats(r))).collect::<HashMap<_, _>>();
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
    let idle = json!({ "type": 11, "player": 1, "summons": [], "to_bp": false, "to_ep": true });
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
    // The monster's record (shown id, race, attribute, level, ATK/DEF, base ATK/DEF) comes before the move.
    assert_eq!(
        out,
        [
            (1000, [1, 100, 0, 2 << 8]),
            (0x3E9, [0, 4100, 12, 2]),
            (0x3EA, [0, 4, 1000, 1000]),
            (0x3EB, [2 << 8, 1000, 1000, 4100]),
            (0x1A, [1 | 100 << 16, 0xB << 1, 2 << 1 | 1 << 14, 0]),
            (1000, [2, 101, 0, 0xB << 8])
        ]
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
    // set = unavailable: monster zones 0-4, Spell/Trap 8-12). MSG_SELECT_POSITION (19) without an attack position
    // (the position dialog's first button is always attack): the first allowed one, here face-up defense.
    let (mut t, _) = translator();
    let place = json!({ "type": 18, "player": 0, "count": 1, "field_mask": 0xFFFF_E0FFu32 });
    let spell = json!({ "type": 10, "places": [{ "player": 0, "location": 8, "sequence": 0 }] });
    assert_eq!(t.prompt(&place), Prompt::Answer(spell));
    let place = json!({ "type": 18, "player": 0, "count": 1, "field_mask": 0xFFFF_FFE2u32 });
    let monster = json!({ "type": 10, "places": [{ "player": 0, "location": 4, "sequence": 0 }] });
    assert_eq!(t.prompt(&place), Prompt::Answer(monster));
    let position = json!({ "type": 19, "player": 0, "code": 1, "positions": 12 });
    assert_eq!(t.prompt(&position), Prompt::Answer(json!({ "type": 11, "position": 4 })));
    // The CPU takes face-up attack when allowed.
    let position = json!({ "type": 19, "player": 1, "code": 1, "positions": 5 });
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

#[test]
fn the_battle_prompt_offers_the_attackers_and_m2_or_ep() {
    // MSG_SELECT_BATTLECMD (10): attackers get bit 0 (Attack), chainable cards bit 3; buttons M2 (4) and EP (5) as
    // allowed; command 4 waits with mode 3. A monster in slot s is at place (zone 0, index s) in the command map
    // (DuelFieldManager 0x50A690 queries it so).
    let (mut t, _) = translator();
    let attacker = json!({ "controller": 0, "location": 4, "sequence": 1, "can_direct": true });
    let battle = json!({ "type": 10, "player": 0, "chains": [], "attacks": [attacker], "to_m2": true, "to_ep": true });
    let Prompt::Client(out) = t.prompt(&battle) else { panic!("the client chooses") };
    let data = [0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0x01, 4, 1, 5, 1];
    assert_eq!(out[0].get("COMIDDATA"), Some(&Value::Bin(data.to_vec())));
    assert_eq!(cmd(&out[1]), (4, [3, 0, 0, 0]));
    // Attack: SELECT_BATTLE (action 1) with the attacker's index.
    let attack = CardCommand { command: 0, player: 0, zone: 0, index: 1 };
    assert_eq!(t.answer(attack), Some(json!({ "type": 0, "action": 1, "index": 0 })));
    t.prompt(&battle);
    assert_eq!(t.phase(4), Some(json!({ "type": 0, "action": 2, "index": null })));
    t.prompt(&battle);
    assert_eq!(t.phase(5), Some(json!({ "type": 0, "action": 3, "index": null })));
}

#[test]
fn damage_and_lp_changes_reach_the_lp_display() {
    // Command 0x10 {player, change} adds a signed change to the LP shown (DuelCockpit 0x502EB0, clamped to
    // 0..99999); 0xF sets it. MSG_DAMAGE (91) and MSG_PAY_LPCOST (100) lower it, MSG_RECOVER (92) raises it,
    // MSG_LPUPDATE (94) sets it. Team 1 is the CPU here.
    let (mut t, _) = translator();
    let out = t.events(&[
        json!({ "type": 91, "player": 1, "amount": 2300 }),
        json!({ "type": 92, "player": 0, "amount": 500 }),
        json!({ "type": 100, "player": 0, "amount": 1000 }),
        json!({ "type": 94, "player": 1, "lp": 5700 }),
    ]);
    let out: Vec<_> = out.iter().map(cmd).collect();
    assert_eq!(
        out,
        [(0x10, [1, (-2300i32) as u32, 0, 0]), (0x10, [0, 500, 0, 0]), (0x10, [0, (-1000i32) as u32, 0, 0]), (0xF, [1, 5700, 0, 0])]
    );
}

#[test]
fn the_cpu_summons_then_attacks_with_everything_then_ends_its_turn() {
    // A placeholder AI: Normal Summon (or Set) the first monster it may, go to battle, attack with each attacker,
    // end the turn.
    let (mut t, _) = translator();
    let card = json!({ "controller": 1, "location": 2, "sequence": 0 });
    let idle = |summons: &serde_json::Value, bp| json!({ "type": 11, "player": 1, "summons": summons, "special_summons": [],
        "pos_changes": [], "monster_sets": [], "spell_sets": [], "activates": [], "to_bp": bp, "to_ep": true });
    assert_eq!(t.prompt(&idle(&json!([card]), true)), Prompt::Answer(json!({ "type": 1, "action": 0, "index": 0 })));
    assert_eq!(t.prompt(&idle(&json!([]), true)), Prompt::Answer(json!({ "type": 1, "action": 6, "index": null })));
    assert_eq!(t.prompt(&idle(&json!([]), false)), Prompt::Answer(json!({ "type": 1, "action": 7, "index": null })));
    let battle = |attacks: serde_json::Value| json!({ "type": 10, "player": 1, "chains": [], "attacks": attacks, "to_m2": true, "to_ep": true });
    assert_eq!(t.prompt(&battle(json!([card]))), Prompt::Answer(json!({ "type": 0, "action": 1, "index": 0 })));
    assert_eq!(t.prompt(&battle(json!([]))), Prompt::Answer(json!({ "type": 0, "action": 3, "index": null })));
}

#[test]
fn other_prompts_get_simple_answers() {
    // Card selections (15 SELECT_CARD, 20 SELECT_TRIBUTE) take the first `min` cards (at least one), yes/no
    // questions (12, 13) are answered yes, options (14) the first.
    let (mut t, _) = translator();
    let select = json!({ "type": 15, "player": 1, "can_cancel": false, "min": 2, "max": 3, "selects": [{}, {}, {}] });
    assert_eq!(t.prompt(&select), Prompt::Answer(json!({ "type": 5, "indicies": [0, 1] })));
    let tribute = json!({ "type": 20, "player": 1, "can_cancel": false, "min": 1, "max": 1, "selects": [{}, {}] });
    assert_eq!(t.prompt(&tribute), Prompt::Answer(json!({ "type": 12, "indicies": [0] })));
    assert_eq!(t.prompt(&json!({ "type": 13, "player": 1 })), Prompt::Answer(json!({ "type": 3, "yes": true })));
    assert_eq!(t.prompt(&json!({ "type": 12, "player": 1 })), Prompt::Answer(json!({ "type": 2, "yes": true })));
    assert_eq!(t.prompt(&json!({ "type": 14, "player": 1, "options": [5, 6] })), Prompt::Answer(json!({ "type": 4, "index": 0 })));
}

#[test]
fn the_engines_win_ends_the_duel_on_the_client() {
    // MSG_WIN (5) {player: the winning team, reason} → command 2 {result, reason}: DuelScene 0x514290 keeps the
    // result at MainGamen+0x81C and its swap (1 ↔ 2, 0x504D00) as the opponent's, so 1 = this client won, 2 = lost;
    // DuelEndEffect (0x5083A0) plays data/gfx/duel_win / duel_lose / duel_draw.swf for 1 / 2 / 3 and shows text
    // 1999 + reason (0x508650): 1 LP 0, 2 deck out, 5 surrender... EDOPro's reasons: 0 surrender, 1 LP 0, 2 deck.
    let (mut t, _) = translator();
    assert_eq!(cmd(&t.events(&[json!({ "type": 5, "player": 0, "reason": 1 })])[0]), (2, [1, 1, 0, 0]));
    assert_eq!(cmd(&t.events(&[json!({ "type": 5, "player": 1, "reason": 2 })])[0]), (2, [2, 2, 0, 0]));
    assert_eq!(cmd(&t.events(&[json!({ "type": 5, "player": 1, "reason": 0 })])[0]), (2, [2, 5, 0, 0]));
}

#[test]
fn the_translator_remembers_the_result_for_the_front_server() {
    // The result screen follows the front push 0x2009 (see duel_contract), not another duel command: 1504 would
    // open Card Swap for the next duel of a match (DuelManager 0x50FC00 → 0x50FDF0).
    let (mut t, _) = translator();
    assert_eq!(t.result(), None);
    let out: Vec<_> = t.events(&[json!({ "type": 5, "player": 1, "reason": 1 })]).iter().map(cmd).collect();
    assert_eq!(out, [(2, [2, 1, 0, 0])]);
    assert_eq!(t.result(), Some(2));
}

#[test]
fn a_draw_ends_the_duel_as_a_draw() {
    // EDOPro's WIN names player 2 for a draw; the client plays duel_draw.swf for result 3 (0x5084D0) and shows no
    // reason text for it (0x508650).
    let (mut t, _) = translator();
    assert_eq!(cmd(&t.events(&[json!({ "type": 5, "player": 2, "reason": 1 })])[0]), (2, [3, 1, 0, 0]));
    assert_eq!(t.result(), Some(3));
}

/// The client's hand holds rows 100 and 101; the CPU's monster zone 1 holds row 200.
fn with_cards() -> Translator {
    let (mut t, _) = translator();
    t.events(&[json!({ "type": 90, "player": 0, "drawn": [{ "cid": 4100 }, { "cid": 4101 }] })]);
    t.events(&[json!({ "type": 90, "player": 1, "drawn": [{ "cid": 4200 }] })]);
    t.events(&[json!({ "type": 50, "cid": 4200,
        "from": { "controller": 1, "location": 2, "sequence": 0, "position": 10 },
        "to": { "controller": 1, "location": 4, "sequence": 1, "position": 1 } })]);
    t
}

fn card_at(controller: u32, location: u32, sequence: u32, cid: u32) -> serde_json::Value {
    json!({ "code": cid, "cid": cid, "controller": controller, "location": location, "sequence": sequence, "position": 1 })
}

fn client_commands(p: Prompt) -> Vec<(u32, [u32; 4])> {
    match p {
        Prompt::Client(out) => out.iter().map(cmd).collect(),
        other => panic!("expected a client prompt, got {other:?}"),
    }
}

#[test]
fn the_clients_card_choice_opens_the_card_list() {
    // MSG_SELECT_CARD (15) {min, max, selects} becomes the card list dialog: DuelScene command 0x37 {player, kind,
    // message} (0x514CCB, DuelCardListDialog 0x4C72D0), after data commands the pop keeps (0x522A50): 0x3EF {count,
    // max} (0x9B8 = max picks, 0 = view only), 0x3F2 the CARD_Prop rows four at a time (getCardInfo 0x4C78A0 reads
    // +0x9BC), 0x3F3 a mark per card four at a time (+0xE6C; 0x8000 = the opponent's, dlg_cardlist.swf). The kind sets
    // how many picks OK needs (0x4C72D0): 0x27 one. A message comes from 0x3EC {entries} and 0x3ED {kind << 28 |
    // text}: kind 1 is a text of the duel prompt table (0x519870); EDOPro's preceding hint (MSG_HINT 2, HINT_SELECTMSG
    // 3) picks it, here 551 HINTMSG_TARGET → 94 "Select the card(s) to target."
    let mut t = with_cards();
    t.events(&[json!({ "type": 2, "hint_type": 3, "player": 0, "hint": 551 })]);
    let select = json!({ "type": 15, "player": 0, "can_cancel": false, "min": 1, "max": 1,
        "selects": [card_at(0, 2, 1, 4101), card_at(1, 4, 1, 4200)] });
    assert_eq!(
        client_commands(t.prompt(&select)),
        [
            (0x3EF, [2, 1, 0, 0]),
            (0x3F2, [101, 200, 0, 0]),
            (0x3F3, [0, 0x8000, 0, 0]),
            (0x3EC, [1, 0, 0, 0]),
            (0x3ED, [1 << 28 | 94, 0, 0, 0]),
            (0x37, [0, 0x27, 1, 0]),
        ]
    );
}

#[test]
fn the_picks_answer_the_engine_when_the_list_closes() {
    // Closing the list (decide, 0x4C74F0) sends duelRequestName 6 {SetIndex: the first pick}, then 7 {index: list
    // position, data: pick order} per pick (0x521CC0), then 4 {DLGResult: the number of picks} (0x521A70).
    let mut t = with_cards();
    let select = json!({ "type": 15, "player": 0, "can_cancel": false, "min": 1, "max": 1,
        "selects": [card_at(0, 2, 1, 4101), card_at(1, 4, 1, 4200)] });
    t.prompt(&select);
    t.pick(1);
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 5, "indicies": [1] }))));
    // Nothing is pending any more.
    assert_eq!(t.dialog_result(1), None);
}

#[test]
fn tributes_use_the_card_list_too() {
    // MSG_SELECT_TRIBUTE (20): the same list with "Select the monster(s) as Tribute." (18). min = max: kind 6, which
    // needs every pick (accept = max).
    let mut t = with_cards();
    let tribute = json!({ "type": 20, "player": 0, "can_cancel": false, "min": 2, "max": 2,
        "selects": [card_at(0, 2, 0, 4100), card_at(0, 2, 1, 4101)] });
    let out = client_commands(t.prompt(&tribute));
    assert_eq!(out[0], (0x3EF, [2, 2, 0, 0]));
    assert_eq!(out[4], (0x3ED, [1 << 28 | 18, 0, 0, 0]));
    assert_eq!(out[5], (0x37, [0, 6, 1, 0]));
    t.pick(1);
    t.pick(0);
    assert_eq!(t.dialog_result(2), Some(Prompt::Answer(json!({ "type": 12, "indicies": [1, 0] }))));
}

#[test]
fn a_cancelled_or_optional_choice() {
    // Without a known hint the list has no message (param3 0). min 0: kind 1, OK with no pick. A cancelable choice
    // closed with no pick cancels (indicies null).
    let mut t = with_cards();
    let optional = json!({ "type": 15, "player": 0, "can_cancel": false, "min": 0, "max": 1,
        "selects": [card_at(0, 2, 0, 4100)] });
    let out = client_commands(t.prompt(&optional));
    assert_eq!(out.last(), Some(&(0x37, [0, 1, 0, 0])));
    assert!(!out.iter().any(|(id, _)| *id == 0x3ED));
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 5, "indicies": [] }))));
    let cancelable = json!({ "type": 15, "player": 0, "can_cancel": true, "min": 1, "max": 1,
        "selects": [card_at(0, 2, 0, 4100)] });
    t.prompt(&cancelable);
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 5, "indicies": null }))));
}

#[test]
fn an_attack_target_is_a_card_choice_with_its_message() {
    // ocgcore asks for an attack target as MSG_SELECT_CARD after hint 549 (HINTMSG_ATTACKTARGET): "Select an attack
    // target @3monster@0." (222). Live: Goblin Elite's attack listed the CPU's monster; picking it dealt 1200.
    let mut t = with_cards();
    t.events(&[json!({ "type": 2, "hint_type": 3, "player": 0, "hint": 549 })]);
    let select = json!({ "type": 15, "player": 0, "can_cancel": true, "min": 1, "max": 1, "selects": [card_at(1, 4, 1, 4200)] });
    let out = client_commands(t.prompt(&select));
    assert_eq!(out[3..], [(0x3EC, [1, 0, 0, 0]), (0x3ED, [1 << 28 | 222, 0, 0, 0]), (0x37, [0, 0x27, 1, 0])]);
}

#[test]
fn an_effect_question_opens_the_yes_no_dialog() {
    // MSG_SELECT_EFFECTYN (12) {code, cid, ...}: dialog command 0x36 {kind 2 Yes/No, 1 = message from the list}
    // (factory 0x4C8CD0, DuelYesNoDialog 0x4C4FB0: buttons 0 "No", 1 "Yes"). Message entries: kind 1 a duel prompt
    // text, kind 5 a card name by Konami id (0x5374B0) put into its %s: 250 'Activate the effect of "%s"?'. The
    // dialog answers duelRequestName 4 {DLGResult: the button} (0x4C50F0).
    let mut t = with_cards();
    let effect = json!({ "type": 12, "player": 0, "code": 4100, "cid": 4100, "controller": 0, "location": 4,
        "sequence": 0, "position": 1, "description": 0 });
    assert_eq!(
        client_commands(t.prompt(&effect)),
        [(0x3EC, [2, 0, 0, 0]), (0x3ED, [1 << 28 | 250, 5 << 28 | 4100, 0, 0]), (0x36, [2, 1, 0, 0])]
    );
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 2, "yes": true }))));
    t.prompt(&effect);
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 2, "yes": false }))));
}

#[test]
fn a_yes_no_question_names_its_card() {
    // MSG_SELECT_YESNO (13) {description}: the question is a card script string (code << 4 | n), which the client
    // doesn't have; the sidecar adds the card's Konami id (description_cid), so it reads 226 'Use the effect of
    // "%s"?', else 270 "Use the effect now?".
    let mut t = with_cards();
    let named = json!({ "type": 13, "player": 0, "description": 65601, "description_cid": 4101 });
    assert_eq!(client_commands(t.prompt(&named))[1], (0x3ED, [1 << 28 | 226, 5 << 28 | 4101, 0, 0]));
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 3, "yes": true }))));
    let plain = json!({ "type": 13, "player": 0, "description": 30 });
    assert_eq!(client_commands(t.prompt(&plain))[..2], [(0x3EC, [1, 0, 0, 0]), (0x3ED, [1 << 28 | 270, 0, 0, 0])]);
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 3, "yes": false }))));
}

#[test]
fn a_position_choice_opens_the_position_dialog() {
    // MSG_SELECT_POSITION (19) {code, positions: 1 face-up attack, 2 face-down attack, 4 face-up defense, 8 face-down
    // defense}: dialog command 0x36 {kind 8, message, uid | face-up defense << 16} (factory 0x4C8CD0 →
    // DuelSelectStandDialog 0x4C5E50: the uid's CARD_Prop row from the card table shows on both buttons;
    // dlg_selstand.swf: button 0 "head_atk", button 1 "head_def" with the flag, else "tail_def"). Message: 327 'Select
    // position for "%s".'. DLGResult = the button (0x4C29D0, 0x4C6410).
    let mut t = with_cards();
    let position = json!({ "type": 19, "player": 0, "code": 4101, "cid": 4101, "positions": 5 });
    assert_eq!(
        client_commands(t.prompt(&position)),
        [(0x3EC, [2, 0, 0, 0]), (0x3ED, [1 << 28 | 327, 5 << 28 | 4101, 0, 0]), (0x36, [8, 1, 2 | 1 << 16, 0])]
    );
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 11, "position": 4 }))));
    t.prompt(&position);
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 11, "position": 1 }))));
    // Face-up attack or face-down defense: the defense button shows the card's back.
    let set = json!({ "type": 19, "player": 0, "code": 4101, "cid": 4101, "positions": 9 });
    assert_eq!(client_commands(t.prompt(&set))[2], (0x36, [8, 1, 2, 0]));
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 11, "position": 8 }))));
}

#[test]
fn an_option_the_client_has_labels_for_opens_the_select_dialog() {
    // MSG_SELECT_OPTION (14) {options: descriptions}: dialog command 0x36 {kind 4} (DuelSelectDialog 0x4C5750,
    // dlg_select.swf) lists the 0x3F0 values (four per command, 0x522A50) up to 0x3EF's count (0x4C5880): a text of the
    // duel prompt table in the low 14 bits, listed when bit 15 is set, at most four. DLGResult = the chosen value's
    // index (0x4C5B20). EDOPro's SELECT_HEADS/TAILS (60/61) are the client's "Heads"/"Tails" (400/401), titled "Coin
    // Toss Selection:" (56).
    let mut t = with_cards();
    let coin = json!({ "type": 14, "player": 0, "options": [60, 61] });
    assert_eq!(
        client_commands(t.prompt(&coin)),
        [
            (0x3EF, [2, 0, 0, 0]),
            (0x3F0, [0x8000 | 400, 0x8000 | 401, 0, 0]),
            (0x3EC, [1, 0, 0, 0]),
            (0x3ED, [1 << 28 | 56, 0, 0, 0]),
            (0x36, [4, 1, 0, 0]),
        ]
    );
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 4, "index": 1 }))));
    // DECLTYPE_MONSTER/SPELL/TRAP (70-72): "Monster Cards", "Spell Cards", "Trap Cards" (440-442), "Select card type."
    // (260).
    let types = json!({ "type": 14, "player": 0, "options": [70, 71, 72] });
    let out = client_commands(t.prompt(&types));
    assert_eq!(out[1], (0x3F0, [0x8000 | 440, 0x8000 | 441, 0x8000 | 442, 0]));
    assert_eq!(out[3], (0x3ED, [1 << 28 | 260, 0, 0, 0]));
    assert_eq!(t.dialog_result(2), Some(Prompt::Answer(json!({ "type": 4, "index": 2 }))));
    // A card's own strings (code << 4 | n) have no text on the client: the first option, as the CPU would.
    let card = json!({ "type": 14, "player": 0, "options": [65600, 65601] });
    assert_eq!(t.prompt(&card), Prompt::Answer(json!({ "type": 4, "index": 0 })));
}

/// A chain chance for the client: `selects` its cards that may chain.
fn select_chain(spe_count: u32, forced: bool, selects: serde_json::Value) -> serde_json::Value {
    json!({ "type": 16, "player": 0, "spe_count": spe_count, "forced": forced, "hint_timing": 0, "hint_timing_other": 0,
        "selects": selects })
}

#[test]
fn a_timely_chain_chance_asks_with_the_cards_that_may_chain() {
    // MSG_SELECT_CHAIN (16) {spe_count, forced, hint_timing, selects}: the question is dialog command 0x36 {kind 3, message,
    // level} (DuelEffectDialog 0x4C5120): Yes/No (buttons 0 "No", 1 "Yes"), as dlg_yesno_card.swf with the 0x3F2 rows
    // (five at most) when 0x3EF's param3 is 0xFF (0x4C51A0). In a chain the message names the last link (MSG_CHAINING
    // 70): 7 'The effect of "%s" is activated. Chain another card or effect?', or 8 '"%s" is activated. ...' for a
    // Spell/Trap Card; else 269 "Activate the effect of a card?". No cards: no chain, nothing asked.
    let mut t = with_cards();
    let trap = card_at(0, 8, 0, 4101);
    assert_eq!(t.prompt(&select_chain(0, false, json!([]))), Prompt::Answer(json!({ "type": 8, "index": null })));
    t.events(&[json!({ "type": 70, "code": 4200, "cid": 4200, "controller": 1, "location": 4, "sequence": 1,
        "position": 1, "triggering_controller": 1, "triggering_location": 4, "triggering_sequence": 1,
        "description": 0, "chain_size": 1 })]);
    assert_eq!(
        client_commands(t.prompt(&select_chain(1, false, json!([trap])))),
        [
            (0x3EF, [1, 0, 0xFF, 0]),
            (0x3F2, [101, 0, 0, 0]),
            (0x3EC, [2, 0, 0, 0]),
            (0x3ED, [1 << 28 | 7, 5 << 28 | 4200, 0, 0]),
            (0x36, [3, 1, 3, 0]),
        ]
    );
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 8, "index": null }))));
    // One card: Yes chains it.
    t.prompt(&select_chain(1, false, json!([trap])));
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 8, "index": 0 }))));
    // A Spell/Trap Card's activation, then the chain's end.
    t.events(&[json!({ "type": 70, "code": 4101, "cid": 4101, "controller": 0, "location": 8, "sequence": 0,
        "position": 1, "description": 0, "chain_size": 2 })]);
    assert_eq!(client_commands(t.prompt(&select_chain(1, false, json!([trap]))))[3], (0x3ED, [1 << 28 | 8, 5 << 28 | 4101, 0, 0]));
    t.events(&[json!({ "type": 74 })]);
    assert_eq!(client_commands(t.prompt(&select_chain(1, false, json!([trap]))))[3], (0x3ED, [1 << 28 | 269, 0, 0, 0]));
}

#[test]
fn with_several_cards_yes_opens_the_card_list() {
    // After Yes, which card: the card list (one pick) with "Select effect to activate." (276); a forced chain skips the
    // question, and a forced chain of one card needs no choice.
    let mut t = with_cards();
    let cards = json!([card_at(0, 8, 0, 4101), card_at(0, 2, 0, 4100)]);
    t.prompt(&select_chain(1, false, cards.clone()));
    let Some(list) = t.dialog_result(1) else { panic!("no card list") };
    assert_eq!(
        client_commands(list),
        [
            (0x3EF, [2, 1, 0, 0]),
            (0x3F2, [101, 100, 0, 0]),
            (0x3F3, [0, 0, 0, 0]),
            (0x3EC, [1, 0, 0, 0]),
            (0x3ED, [1 << 28 | 276, 0, 0, 0]),
            (0x37, [0, 0x27, 1, 0]),
        ]
    );
    t.pick(1);
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 8, "index": 1 }))));
    // No pick: no chain.
    t.prompt(&select_chain(1, false, cards.clone()));
    t.dialog_result(1);
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 8, "index": null }))));
    // Forced.
    assert_eq!(client_commands(t.prompt(&select_chain(0, true, cards))).last(), Some(&(0x37, [0, 0x27, 1, 0])));
    t.pick(0);
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 8, "index": 0 }))));
    let one = json!([card_at(0, 2, 0, 4100)]);
    assert_eq!(t.prompt(&select_chain(0, true, one)), Prompt::Answer(json!({ "type": 8, "index": 0 })));
}

#[test]
fn against_a_player_the_opponents_hidden_cards_carry_no_row() {
    // Against another player the card table (1000) and the moves (0x1A uid | row << 16) give row 0 for the opponent's
    // deck, hand and face-down cards, and a face-down monster gets no field record; a face-up card shows its row.
    let stats = |r: u16| CardStats { cid: 4000 + r, atk: 10 * r as u32, def: 1000, level: 4, attribute: 2, race: 12 };
    let cids = (100..110).chain(200..210).map(|r| (r, stats(r))).collect::<HashMap<_, _>>();
    let (client, cpu): (Vec<u16>, Vec<u16>) = ((100..110).collect(), (200..210).collect());
    let (mut t, table) = Translator::against_player(0, [&client, &cpu], cids);
    assert_eq!(cmd(&table[10]), (1000, [11, 0, 0, 1 | 0xD << 8]));
    assert_eq!(cmd(&table[0]), (1000, [1, 100, 0, 0xD << 8]));
    let drawn: Vec<_> = t.events(&[json!({ "type": 90, "player": 1, "drawn": [{ "cid": 4200 }, { "cid": 4201 }] })]).iter().map(cmd).collect();
    assert!(drawn.iter().all(|(id, p)| *id != 1000 || p[1] == 0), "{drawn:?}");
    assert!(drawn.iter().all(|(id, p)| *id != 0x1A || p[0] >> 16 == 0), "{drawn:?}");
    let summon = |position| json!({ "type": 50, "cid": 4200,
        "from": { "controller": 1, "location": 2, "sequence": 0, "position": 10 },
        "to": { "controller": 1, "location": 4, "sequence": 1, "position": position } });
    let face_up: Vec<_> = t.events(&[summon(1)]).iter().map(cmd).collect();
    assert_eq!(face_up[0], (1000, [11, 200, 0, 1 | 1 << 8]));
    assert!(face_up.iter().any(|(id, _)| *id == 0x3EB), "the field record");
    let set = json!({ "type": 50, "cid": 4201,
        "from": { "controller": 1, "location": 2, "sequence": 0, "position": 10 },
        "to": { "controller": 1, "location": 4, "sequence": 2, "position": 8 } });
    let face_down: Vec<_> = t.events(&[set]).iter().map(cmd).collect();
    assert_eq!(face_down[0], (1000, [12, 0, 0, 1 | 2 << 8]));
    assert!(!face_down.iter().any(|(id, _)| *id == 0x3EB), "no field record");
}

/// A chain chance's level: the Effect dialog's param3.
fn chain_level(t: &mut Translator, m: serde_json::Value) -> u32 {
    client_commands(t.prompt(&m)).last().map(|&(_, p)| p[2]).expect("the question")
}

#[test]
fn a_chain_chance_says_how_much_it_matters() {
    // The Effect dialog decides by its level and the player's Activation Message option (duelOption invoke_conf,
    // 0x4C5560; the DUEL OPTIONS screen): level 4+ always shows; Manual (0) shows all unless the right mouse button is
    // held; Semi Auto Cancel (1, the default) answers No by itself below level 2 ("low priority"); Auto Cancel (2) only
    // shows while the left button is held. Unshown, it answers DLGResult 0 at once (0x4C5120). The levels sent: 3 when
    // something happened (a summon, an attack, damage... or a chain in progress), 1 at a phase or step, 0 when no card
    // fits the timing (spe_count 0, which EDOPro's own client skips).
    let mut t = with_cards();
    let trap = json!([card_at(0, 8, 0, 4101)]);
    let at = |spe: u32, timing: u32, other: u32| {
        json!({ "type": 16, "player": 0, "spe_count": spe, "forced": false, "hint_timing": timing, "hint_timing_other": other,
            "selects": trap.clone() })
    };
    assert_eq!(chain_level(&mut t, at(0, 0x20, 0)), 0, "nothing fits: Manual only");
    t.dialog_result(0);
    assert_eq!(chain_level(&mut t, at(1, 0x20, 0)), 1, "the End Phase: Semi answers No");
    t.dialog_result(0);
    assert_eq!(chain_level(&mut t, at(1, 0, 0x1000)), 3, "the opponent's attack");
    t.dialog_result(0);
    assert_eq!(chain_level(&mut t, at(1, 0x40, 0)), 3, "a summon");
    t.dialog_result(0);
    t.events(&[json!({ "type": 70, "code": 4200, "cid": 4200, "controller": 1, "location": 4, "sequence": 1, "position": 1,
        "description": 0, "chain_size": 1 })]);
    assert_eq!(chain_level(&mut t, at(1, 0, 0)), 3, "a chain in progress");
}
