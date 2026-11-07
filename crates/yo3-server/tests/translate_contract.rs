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
    // MSG_SELECT_YESNO (13) {description}: the question is a card script string (code << 20 | n), which the client
    // doesn't have; the sidecar adds the card's Konami id (description_cid), so it reads 226 'Use the effect of
    // "%s"?', else 270 "Use the effect now?".
    let mut t = with_cards();
    let named = json!({ "type": 13, "player": 0, "description": 4101u64 << 20 | 1, "description_cid": 4101 });
    assert_eq!(client_commands(t.prompt(&named))[1], (0x3ED, [1 << 28 | 226, 5 << 28 | 4101, 0, 0]));
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 3, "yes": true }))));
    let plain = json!({ "type": 13, "player": 0, "description": 32 });
    assert_eq!(client_commands(t.prompt(&plain))[..2], [(0x3EC, [1, 0, 0, 0]), (0x3ED, [1 << 28 | 270, 0, 0, 0])]);
    assert_eq!(t.dialog_result(0), Some(Prompt::Answer(json!({ "type": 3, "yes": false }))));
}

#[test]
fn the_battles_questions_read_as_the_clients_own() {
    // EDOPro's core asks the attacking player two battle questions by system string (processor.cpp): 31 when the
    // attacker could attack directly or a monster (yes: directly), 30 when a replay happens (yes: attack again). The
    // client's dialog texts have both: 225 "Attack your opponent directly?" and 223 "Replay happens. Continue to
    // attack?" (the target itself is the card list's, hint 549 → 222 "Select an attack target monster.").
    let mut t = with_cards();
    for (description, text) in [(31, 225), (30, 223)] {
        let question = json!({ "type": 13, "player": 0, "description": description });
        assert_eq!(client_commands(t.prompt(&question))[1], (0x3ED, [1 << 28 | text, 0, 0, 0]), "{description}");
        assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 3, "yes": true }))));
    }
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
    // A card's own strings (aux.Stringid: code << 20 | n) take the original server's per-card labels (the prompt table's 400-495, and
    // 369/370 for a side of the field) where one says the same thing, under "Select effect to @3activate@0." (276):
    // Penalty Game! (967928) "Your opponent cannot draw during their next Draw Phase" / "... activate a Spell or Trap
    // Card during this turn" (465/466).
    let penalty = json!({ "type": 14, "player": 0, "options": [967928u64 << 20, 967928u64 << 20 | 1] });
    let out = client_commands(t.prompt(&penalty));
    assert_eq!(out[1], (0x3F0, [0x8000 | 465, 0x8000 | 466, 0, 0]));
    assert_eq!(out[3], (0x3ED, [1 << 28 | 276, 0, 0, 0]));
    assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": 4, "index": 1 }))));
    // Strings without a label: the first option, as the CPU would.
    let card = json!({ "type": 14, "player": 0, "options": [4100u64 << 20, 4100u64 << 20 | 1] });
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

/// The cards' place words of the 0x1C commands among `out`.
fn flips(out: &[Message]) -> Vec<u32> {
    out.iter().map(cmd).filter(|(id, _)| *id == 0x1C).map(|(_, p)| p[0]).collect()
}

#[test]
fn a_position_change_turns_the_card_in_place() {
    // MSG_POS_CHANGE (53) {controller, location, sequence, position}: command 0x1C (CardFlipTurn, DuelCardManager
    // 0x4FF000; the 28th view type in Konami's engine vocabulary) {the place as moves give it, with face up bit 14 and
    // defense bit 15}, which sets the field card's state (+0x82) and redraws it. Here the CPU's monster in zone 1
    // goes to face-up defense (position 4).
    let mut t = with_cards();
    let change = json!({ "type": 53, "code": 4200, "cid": 4200, "controller": 1, "location": 4, "sequence": 1,
        "prev_position": 1, "position": 4 });
    assert_eq!(flips(&t.events(&[change])), [1 | 1 << 1 | 1 << 14 | 1 << 15]);
}

#[test]
fn a_flipped_card_of_the_opponent_shows_its_row_first() {
    // Against a player, the opponent's face-down card has row 0 on the client; flipping it face-up in place sends its
    // table entry (1000) with the real row and its field record before the flip. A flip summon (64) is the same.
    let stats = |r: u16| CardStats { cid: 4000 + r, atk: 10 * r as u32, def: 1000, level: 4, attribute: 2, race: 12 };
    let cids = (100..110).chain(200..210).map(|r| (r, stats(r))).collect::<HashMap<_, _>>();
    let (client, cpu): (Vec<u16>, Vec<u16>) = ((100..110).collect(), (200..210).collect());
    let (mut t, _) = Translator::against_player(0, [&client, &cpu], cids);
    t.events(&[json!({ "type": 90, "player": 1, "drawn": [{ "cid": 4200 }] })]);
    t.events(&[json!({ "type": 50, "cid": 4200,
        "from": { "controller": 1, "location": 2, "sequence": 0, "position": 10 },
        "to": { "controller": 1, "location": 4, "sequence": 2, "position": 8 } })]);
    let flip = json!({ "type": 64, "code": 4200, "cid": 4200, "controller": 1, "location": 4, "sequence": 2, "position": 1 });
    let out: Vec<_> = t.events(&[flip]).iter().map(cmd).collect();
    assert_eq!(out.first(), Some(&(1000, [11, 200, 0, 1 | 2 << 8])), "{out:?}");
    assert!(out.iter().any(|(id, _)| *id == 0x3EB), "its field record");
    assert_eq!(out.last(), Some(&(0x1C, [1 | 2 << 1 | 1 << 14, 0, 0, 0])));
}

#[test]
fn an_activated_set_card_turns_face_up() {
    // A Set Spell/Trap turns face-up as it is activated: the engine sends MSG_POS_CHANGE to POS_FACEUP (5). A
    // Spell/Trap has no battle position, so no defense bit. Live: Trap Hole flipped (0x1C 0x400A) as the CPU summoned.
    let mut t = with_cards();
    t.events(&[json!({ "type": 50, "cid": 4101,
        "from": { "controller": 0, "location": 2, "sequence": 1, "position": 10 },
        "to": { "controller": 0, "location": 8, "sequence": 0, "position": 10 } })]);
    let flip = json!({ "type": 53, "code": 4101, "cid": 4101, "controller": 0, "location": 8, "sequence": 0,
        "prev_position": 10, "position": 5 });
    assert_eq!(flips(&t.events(&[flip])), [5 << 1 | 1 << 14]);
}

/// The battle and summon commands among `out` (0xA-0xE, 0x38, 0x39).
fn shows(out: &[Message]) -> Vec<(u32, [u32; 4])> {
    out.iter().map(cmd).filter(|(id, _)| matches!(id, 0xA..=0xE | 0x38 | 0x39)).collect()
}

fn loc(controller: u32, location: u32, sequence: u32, position: u32) -> serde_json::Value {
    json!({ "controller": controller, "location": location, "sequence": sequence, "position": position })
}

#[test]
fn an_attack_plays_on_the_field() {
    // DuelBattle (0x4F8A90; the call at 0x4F840E passes param1 in ECX, param3 in EDX, param2 on the stack):
    // BattleInit 0xA, BattleAttack 0xC {attacker's player | its zone << 8, the target's player, the target's zone or
    // 0xFFFFFFFF for a direct attack}, BattleRun 0xD {per player: the damage figure on the client's card, on the
    // opponent's, flags: bit 1 the client's card destroyed, bit 9 the opponent's}, which shows the figures at the cards
    // (0x4F8920; states 5 and 6 pair the values with the cards by player, and a destroyed card's effect ends hiding
    // it), and BattleEnd 0xE once the damage step ends or the attack is stopped. A direct attack shows no figures.
    // Live: with the values by attacker and target instead, the CPU's attack hid the client's card.
    let mut t = with_cards();
    t.events(&[json!({ "type": 50, "cid": 4101,
        "from": { "controller": 0, "location": 2, "sequence": 1, "position": 10 },
        "to": { "controller": 0, "location": 4, "sequence": 0, "position": 1 } })]);
    let attack = json!({ "type": 110, "card": loc(0, 4, 0, 1), "target": loc(1, 4, 1, 1) });
    assert_eq!(shows(&t.events(&[attack])), [(0xA, [0; 4]), (0xC, [0, 1, 1, 0])]);
    let battle = json!({ "type": 111,
        "card": { "controller": 0, "location": 4, "sequence": 0, "position": 1, "attack": 1700, "defense": 1000, "destroyed": false },
        "target": { "controller": 1, "location": 4, "sequence": 1, "position": 1, "attack": 1400, "defense": 1200, "destroyed": true } });
    assert_eq!(shows(&t.events(&[battle])), [(0xD, [0, 300, 2 << 8, 0])]);
    assert_eq!(shows(&t.events(&[json!({ "type": 114 })])), [(0xE, [0; 4])]);
    // The CPU's direct attack from its zone 1.
    let direct = json!({ "type": 110, "card": loc(1, 4, 1, 1), "target": null });
    assert_eq!(shows(&t.events(&[direct]))[1], (0xC, [1 | 1 << 8, 0, 0xFFFF_FFFF, 0]));
    let battle = json!({ "type": 111,
        "card": { "controller": 1, "location": 4, "sequence": 1, "position": 1, "attack": 1400, "defense": 1200, "destroyed": false },
        "target": { "controller": 0, "location": 0, "sequence": 0, "position": 0, "attack": 0, "defense": 0, "destroyed": false } });
    assert_eq!(shows(&t.events(&[battle])), [(0xD, [0; 4])]);
    // A stopped attack ends its battle too.
    assert_eq!(shows(&t.events(&[json!({ "type": 112 })])), [(0xE, [0; 4])]);
    // A defending monster: damage only when its DEF is higher.
    t.events(&[json!({ "type": 110, "card": loc(0, 4, 0, 1), "target": loc(1, 4, 1, 4) })]);
    let battle = json!({ "type": 111,
        "card": { "controller": 0, "location": 4, "sequence": 0, "position": 1, "attack": 1000, "defense": 1000, "destroyed": false },
        "target": { "controller": 1, "location": 4, "sequence": 1, "position": 4, "attack": 0, "defense": 1500, "destroyed": false } });
    assert_eq!(shows(&t.events(&[battle])), [(0xD, [500, 0, 0, 0])]);
    // The CPU's 1400 attacks the client's 1700: the CPU's card is destroyed and the CPU takes 300.
    t.events(&[json!({ "type": 114 }), json!({ "type": 110, "card": loc(1, 4, 1, 1), "target": loc(0, 4, 0, 1) })]);
    let battle = json!({ "type": 111,
        "card": { "controller": 1, "location": 4, "sequence": 1, "position": 1, "attack": 1400, "defense": 1200, "destroyed": true },
        "target": { "controller": 0, "location": 4, "sequence": 0, "position": 1, "attack": 1700, "defense": 1000, "destroyed": false } });
    assert_eq!(shows(&t.events(&[battle])), [(0xD, [0, 300, 2 << 8, 0])]);
}

#[test]
fn a_summon_plays_its_effect() {
    // DuelScene (0x514290) RunSummon 0x38 {the player, the summoned card's uid} plays the summon effect at the card
    // (0x21, or 0x23 when it is the player's "ace_monster"); RunSpSummon 0x39 {player, uid} the Special Summon effect
    // (0x19). The engine reports MSG_SUMMONING (60) / MSG_SPSUMMONING (62) once the card is in its zone.
    let mut t = with_cards();
    t.events(&[json!({ "type": 50, "cid": 4101,
        "from": { "controller": 0, "location": 2, "sequence": 1, "position": 10 },
        "to": { "controller": 0, "location": 4, "sequence": 2, "position": 1 } })]);
    let summoning = json!({ "type": 60, "code": 4101, "cid": 4101, "controller": 0, "location": 4, "sequence": 2, "position": 1 });
    assert_eq!(shows(&t.events(&[summoning])), [(0x38, [0, 2, 0, 0])]);
    let special = json!({ "type": 62, "code": 4200, "cid": 4200, "controller": 1, "location": 4, "sequence": 1, "position": 1 });
    assert_eq!(shows(&t.events(&[special])), [(0x39, [1, 11, 0, 0])]);
}

/// The chain commands among `out` (0x33, 0x34).
fn links(out: &[Message]) -> Vec<(u32, [u32; 4])> {
    out.iter().map(cmd).filter(|(id, _)| matches!(id, 0x33 | 0x34)).collect()
}

fn chaining(controller: u32, location: u32, sequence: u32, link: u32) -> serde_json::Value {
    json!({ "type": 70, "code": 4200, "cid": 4200, "controller": controller, "location": location, "sequence": sequence,
        "position": 5, "triggering_controller": controller, "triggering_location": location,
        "triggering_sequence": sequence, "description": 0, "chain_size": link })
}

#[test]
fn a_chain_shows_its_links_and_resolves_them_last_first() {
    // DuelChain (0x501560): ChainSet 0x33 {player, zone, link number} adds a link over the card's zone (0x4E7990's
    // place table 0x81AE38: zones as moves number them, 16 the field's centre); link 1 starts a new list, and the new
    // link is read at index link - 1, so every link needs its ChainSet. ChainRun 0x34 {_, _, link number} resolves the
    // list's last link (states 5-6 pop the end), so the engine's MSG_CHAIN_SOLVING (72) comes last link first. Both
    // read the link from param3; live, ChainRun's link in param1 made the client look up link -1 and stop (0xC0000417
    // from the bounds check 0x501620, called by state 5 at 0x5014B2).
    let mut t = with_cards();
    // The CPU's monster effect in its zone 1, the client's Set card in Spell/Trap zone 0, a hand effect, an Xyz
    // material's (location MZONE | OVERLAY, its monster's zone 2), and a location the client has no place for.
    let out = t.events(&[
        chaining(1, 4, 1, 1),
        chaining(0, 8, 0, 2),
        chaining(0, 2, 3, 3),
        chaining(1, 0x84, 2, 4),
        chaining(0, 0x400, 0, 5),
    ]);
    assert_eq!(
        links(&out),
        [(0x33, [1, 1, 1, 0]), (0x33, [0, 5, 2, 0]), (0x33, [0, 0xB, 3, 0]), (0x33, [1, 2, 4, 0]), (0x33, [0, 16, 5, 0])]
    );
    let solving = |link: u32| json!({ "type": 72, "chain_size": link });
    let solved = |link: u32| json!({ "type": 73, "chain_size": link });
    let out = t.events(&[solving(5), solved(5), solving(4), solved(4), solving(3)]);
    assert_eq!(links(&out), [(0x34, [0, 0, 5, 0]), (0x34, [0, 0, 4, 0]), (0x34, [0, 0, 3, 0])]);
}

/// The CardVanish commands among `out` (0x1F).
fn vanishes(out: &[Message]) -> Vec<(u32, [u32; 4])> {
    out.iter().map(cmd).filter(|(id, _)| *id == 0x1F).collect()
}

fn destroyed(from: serde_json::Value, reason: u32) -> serde_json::Value {
    json!({ "type": 50, "card": 0, "cid": 4101, "from": from, "to": loc(0, 16, 0, 5), "reason": reason })
}

#[test]
fn a_card_destroyed_by_an_effect_shatters_before_it_moves() {
    // DuelCardManager (0x4FF000) CardVanish 0x1F {player, zone | index << 8} (zone at most 0xB) hides the card's label,
    // plays effect 0x33 at its place (the shatter) and hides the card (state 0x4FC520); the move then flies it on.
    // It must come before the card table entry (1000) moves the card away. Only for MSG_MOVE's reason DESTROY (1)
    // without BATTLE (0x20): battles show their own destruction (BattleRun).
    let mut t = with_cards();
    t.events(&[json!({ "type": 90, "player": 0, "drawn": [{ "cid": 4101 }, { "cid": 4101 }, { "cid": 4101 }, { "cid": 4101 }] })]);
    t.events(&[json!({ "type": 50, "cid": 4101,
        "from": { "controller": 0, "location": 2, "sequence": 1, "position": 10 },
        "to": { "controller": 0, "location": 4, "sequence": 2, "position": 1 } })]);
    // REASON_DESTROY | REASON_EFFECT: the shatter, then the card table entry and the move.
    let out = t.events(&[destroyed(loc(0, 4, 2, 1), 0x41)]);
    let ids: Vec<u32> = out.iter().map(cmd).map(|(id, _)| id).collect();
    assert_eq!(vanishes(&out), [(0x1F, [0, 2, 0, 0])]);
    assert_eq!(ids.iter().position(|&id| id == 0x1F), Some(0), "{ids:?}");
    // A Spell/Trap Card in Spell/Trap zone 1, and a card in the hand at index 2.
    t.events(&[json!({ "type": 50, "cid": 4101, "from": loc(0, 2, 0, 10), "to": loc(0, 8, 1, 10) })]);
    assert_eq!(vanishes(&t.events(&[destroyed(loc(0, 8, 1, 10), 0x41)])), [(0x1F, [0, 6, 0, 0])]);
    assert_eq!(vanishes(&t.events(&[destroyed(loc(0, 2, 2, 10), 0x41)])), [(0x1F, [0, 0xB | 2 << 8, 0, 0])]);
    // Destroyed by battle, or not destroyed at all (a resolved Trap going to the graveyard): no shatter.
    t.events(&[json!({ "type": 50, "cid": 4101, "from": loc(0, 2, 0, 10), "to": loc(0, 4, 3, 1) })]);
    assert_eq!(vanishes(&t.events(&[destroyed(loc(0, 4, 3, 1), 0x21)])), []);
    t.events(&[json!({ "type": 50, "cid": 4101, "from": loc(0, 2, 0, 10), "to": loc(0, 8, 2, 5) })]);
    assert_eq!(vanishes(&t.events(&[destroyed(loc(0, 8, 2, 5), 0x400)])), []);
}

/// The monster record commands among `out` (0x3E9-0x3EB).
fn records(out: &[Message]) -> Vec<(u32, [u32; 4])> {
    out.iter().map(cmd).filter(|(id, _)| matches!(id, 0x3E9..=0x3EB)).collect()
}

fn monster(controller: u32, sequence: u32, position: u32, level: u32, attack: u32) -> serde_json::Value {
    json!({ "controller": controller, "sequence": sequence, "position": position, "level": level, "attack": attack,
        "defense": 1000, "base_attack": 2000, "base_defense": 1000 })
}

#[test]
fn a_monsters_changed_stats_go_to_the_client_as_its_record() {
    // DuelCardManager re-reads the monster-zone records after every command (0x4FFF14) and redraws the label of a
    // face-up monster whose record changed. The sidecar's {"type": "field"} report after an engine step gives the
    // engine's stats; a face-up monster whose level, ATK, DEF, base ATK or base DEF differ from those last sent gets
    // its record again: 0x3E9 {_, cid, race, attribute}, 0x3EA {_, level, ATK, DEF}, 0x3EB {player | zone << 8,
    // base ATK, base DEF, cid}.
    let mut t = with_cards();
    let field = |monsters: Vec<serde_json::Value>| json!({ "type": "field", "monsters": monsters });
    // The CPU's monster (row 200: 2000/1000, level 4) as it entered: nothing to send.
    assert_eq!(records(&t.events(&[field(vec![monster(1, 1, 1, 4, 2000)])])), []);
    // An effect raised its ATK by 500.
    assert_eq!(
        records(&t.events(&[field(vec![monster(1, 1, 1, 4, 2500)])])),
        [(0x3E9, [0, 4200, 12, 2]), (0x3EA, [0, 4, 2500, 1000]), (0x3EB, [1 | 1 << 8, 2000, 1000, 4200])]
    );
    // The same again, or a level of 0 (an Xyz monster's: CARD_Prop's level stays): nothing new.
    assert_eq!(records(&t.events(&[field(vec![monster(1, 1, 1, 4, 2500)])])), []);
    assert_eq!(records(&t.events(&[field(vec![monster(1, 1, 1, 0, 2500)])])), []);
    // The client's face-down monster: no record (its label isn't shown).
    t.events(&[json!({ "type": 50, "cid": 4100, "from": loc(0, 2, 0, 10), "to": loc(0, 4, 0, 8) })]);
    assert_eq!(records(&t.events(&[field(vec![monster(0, 0, 8, 4, 3000), monster(1, 1, 1, 4, 2500)])])), []);
    // Its zone emptied and filled again: the new monster's record comes with its move, and a report of the same
    // values sends nothing more.
    t.events(&[json!({ "type": 50, "cid": 4200, "from": loc(1, 4, 1, 1), "to": loc(1, 16, 0, 5) })]);
    t.events(&[json!({ "type": 90, "player": 1, "drawn": [{ "cid": 4201 }] })]);
    let entered = t.events(&[json!({ "type": 50, "cid": 4201, "from": loc(1, 2, 0, 10), "to": loc(1, 4, 1, 1) })]);
    assert_eq!(records(&entered)[1], (0x3EA, [0, 4, 2010, 1000]));
    let same = json!({ "controller": 1, "sequence": 1, "position": 1, "level": 4, "attack": 2010, "defense": 1000,
        "base_attack": 2010, "base_defense": 1000 });
    assert_eq!(records(&t.events(&[field(vec![same])])), []);
}

/// MSG_ADD_COUNTER (101) / MSG_REMOVE_COUNTER (102) {counter_type, controller, location, sequence, count}.
fn counter(add: bool, counter_type: u32, location: u32, sequence: u32, count: u32) -> serde_json::Value {
    json!({ "type": if add { 101 } else { 102 }, "counter_type": counter_type, "controller": 0, "location": location,
        "sequence": sequence, "count": count })
}

fn counters(out: &[Message]) -> Vec<(u32, [u32; 4])> {
    out.iter().map(cmd).filter(|(id, _)| *id == 0x27).collect()
}

/// A translator with a monster in monster zone 2 and a Spell/Trap in Spell/Trap zone 1 (client zone 6).
fn with_field() -> Translator {
    let mut t = with_cards();
    t.events(&[json!({ "type": 90, "player": 0, "drawn": [{ "cid": 4101 }, { "cid": 4101 }, { "cid": 4101 }] })]);
    t.events(&[json!({ "type": 50, "cid": 4101, "from": loc(0, 2, 0, 10), "to": loc(0, 4, 2, 1) })]);
    t.events(&[json!({ "type": 50, "cid": 4101, "from": loc(0, 2, 0, 10), "to": loc(0, 8, 1, 1) })]);
    t
}

#[test]
fn counters_show_on_the_card_as_the_clients_own_kinds() {
    // DuelCardManager's 0x27 {player | zone << 8, change, kind} (0x4FFD8E) adds the change to the card's count of that
    // kind (a map at +0x130; 0 erases it) and shows it on the card's field label with the kind's icon (table 0x789020,
    // setCounter); the card info lists it as UI text 200 + kind ("Spell Counters" 0, "Wedge Counter" 42, "Counter"
    // 38, 0x4551DB). EDOPro's counter ids (strings.conf) map by name; one the client has no name for is "Counter".
    let mut t = with_field();
    assert_eq!(counters(&t.events(&[counter(true, 0x1, 4, 2, 1)])), [(0x27, [2 << 8, 1, 0, 0])], "Spell Counter");
    assert_eq!(counters(&t.events(&[counter(true, 0x1002, 8, 1, 2)])), [(0x27, [6 << 8, 2, 42, 0])], "Wedge Counter");
    assert_eq!(counters(&t.events(&[counter(true, 0x7, 4, 2, 1)])), [(0x27, [2 << 8, 1, 38, 0])], "an unnamed one");
    // Removing takes the change away; never below what the client holds (it keeps a negative count's entry as it was).
    assert_eq!(counters(&t.events(&[counter(false, 0x1002, 8, 1, 1)])), [(0x27, [6 << 8, -1i32 as u32, 42, 0])]);
    assert_eq!(counters(&t.events(&[counter(false, 0x1002, 8, 1, 5)])), [(0x27, [6 << 8, -1i32 as u32, 42, 0])]);
    assert_eq!(counters(&t.events(&[counter(false, 0x1002, 8, 1, 1)])), []);
}

#[test]
fn a_card_leaving_the_field_drops_its_counters_first() {
    // EDOPro clears a card's counters without a message when it leaves the field (card::reset), but the client keeps
    // them on the card (and shows them if it comes back face up); only landing or turning face down clears them
    // (0x4FBF30, 0x4FBE50, 0x4FB770). So before the card moves, its counts go to 0, while it is still at its place.
    let mut t = with_field();
    t.events(&[counter(true, 0x1, 4, 2, 2)]);
    let out = t.events(&[destroyed(loc(0, 4, 2, 1), 0x41)]);
    assert_eq!(counters(&out), [(0x27, [2 << 8, -2i32 as u32, 0, 0])]);
    let ids: Vec<u32> = out.iter().map(cmd).map(|(id, _)| id).collect();
    assert_eq!(ids[0], 0x27, "before the shatter and the move: {ids:?}");
    // Back on the field later: its counts start again from 0.
    t.events(&[json!({ "type": 50, "cid": 4101, "from": loc(0, 16, 0, 5), "to": loc(0, 4, 2, 1) })]);
    assert_eq!(counters(&t.events(&[counter(true, 0x1, 4, 2, 1)])), [(0x27, [2 << 8, 1, 0, 0])]);
    // Turned face down, the client empties the card's counters itself: nothing to drop when it leaves.
    t.events(&[json!({ "type": 53, "code": 4101, "controller": 0, "location": 4, "sequence": 2, "prev_position": 1,
        "position": 8 })]);
    assert_eq!(counters(&t.events(&[destroyed(loc(0, 4, 2, 8), 0x41)])), []);
}

#[test]
fn extra_decks_go_into_the_card_table_and_move_from_there() {
    // Each side's extra deck joins the card table at zone 0xC (uids after both main decks), the opponent's with row
    // 0 against a player. A card leaving the extra deck (a Fusion or Synchro Summon) is found by its id, wherever the
    // engine keeps it, and moves from zone 0xC: the client's extra deck pile counts it down (0x5000E0).
    let stats = |r: u16| CardStats { cid: 4000 + r, atk: 10 * r as u32, def: 1000, level: 4, attribute: 2, race: 12 };
    let cids = (100..110).chain(200..210).chain([150, 151, 250]).map(|r| (r, stats(r))).collect::<HashMap<_, _>>();
    let (client, cpu): (Vec<u16>, Vec<u16>) = ((100..110).collect(), (200..210).collect());
    let (mut t, _) = Translator::against_player(0, [&client, &cpu], cids);
    let table: Vec<_> = t.extra_decks([&[150, 151][..], &[250][..]]).iter().map(cmd).collect();
    assert_eq!(table, [(1000, [21, 150, 0, 0xC << 8]), (1000, [22, 151, 0, 0xC << 8 | 1 << 16]), (1000, [23, 0, 0, 1 | 0xC << 8])]);
    let out: Vec<_> = t
        .events(&[json!({ "type": 50, "cid": 4151,
            "from": { "controller": 0, "location": 64, "sequence": 0, "position": 10 },
            "to": { "controller": 0, "location": 4, "sequence": 2, "position": 1 } })])
        .iter()
        .map(cmd)
        .collect();
    let moved = out.iter().find(|(id, _)| *id == 0x1A).expect("the move");
    assert_eq!(moved.1[0], 22 | 151 << 16, "uid 22, the card asked for");
    assert_eq!((moved.1[1] >> 1) & 0x1F, 0xC, "from the extra deck");
}

#[test]
fn type_attribute_and_number_declarations_open_the_clients_dialogs() {
    // Dialog command 0x36 {kind, message, param3} (factory 0x4C8CD0, kinds by the table 0x4C9178):
    // - kind 5, DuelSelectTypeDialog (dlg_seltype.swf): param3 the allowed types as bits 1-22 in the client's order (UI
    //   texts 1031-1052: Dragon, Zombie, Fiend, Pyro, Sea Serpent, Rock, Machine, Fish, Dinosaur, Insect, Beast,
    //   Beast-Warrior, Plant, Aqua, Warrior, Winged Beast, Fairy, Spellcaster, Thunder, Reptile, Psychic, Divine-Beast),
    //   one pick; DLGResult the type's number.
    // - kind 7, DuelSelectAttrDialog (dlg_selattr.swf): param3 the allowed attributes (bits 1-7: LIGHT, DARK, WATER,
    //   FIRE, EARTH, WIND, DIVINE, texts 1061-1067) << 16 | how many to pick; DLGResult the picks as bits.
    // - kind 11, DuelSelectNumberDialog (dlg_selnum.swf, 0x4C6430): param3 max << 24 | step << 20 | min << 16 | its
    //   format text (302 "%d000LP" counts in thousands, 303 "%d00LP" in hundreds, others in ones: 283 "%d"); DLGResult the
    //   number in those units (0x4C66D0).
    // The answers carry EDOPro's race and attribute bits, and the number's index among the options.
    let mut t = with_cards();
    let race = json!({ "type": 140, "player": 0, "count": 1, "available": 0x2001 });
    let out = client_commands(t.prompt(&race));
    assert_eq!(out.last(), Some(&(0x36, [5, 1, 1 << 1 | 1 << 15, 0])), "Dragon and Warrior");
    assert!(out.contains(&(0x3ED, [1 << 28 | 241, 0, 0, 0])), "Select Type(s).");
    assert_eq!(t.dialog_result(15), Some(Prompt::Answer(json!({ "type": 16, "races": [1] }))), "Warrior");
    let attribute = json!({ "type": 141, "player": 0, "count": 1, "available": 0x30 });
    assert_eq!(client_commands(t.prompt(&attribute)).last(), Some(&(0x36, [7, 1, 0b110 << 16 | 1, 0])), "LIGHT and DARK");
    assert_eq!(t.dialog_result(1 << 2), Some(Prompt::Answer(json!({ "type": 17, "attributes": [0x20] }))), "DARK");
    let levels = json!({ "type": 143, "player": 0, "options": (1..=12).collect::<Vec<u32>>() });
    let out = client_commands(t.prompt(&levels));
    assert_eq!(out.last(), Some(&(0x36, [11, 1, 12 << 24 | 1 << 20 | 1 << 16 | 283, 0])));
    assert!(out.contains(&(0x3ED, [1 << 28 | 280, 0, 0, 0])), "Declare monster level.");
    assert_eq!(t.dialog_result(7), Some(Prompt::Answer(json!({ "type": 19, "value": 6 }))));
    let lp = json!({ "type": 143, "player": 0, "options": [500, 1000, 1500, 2000] });
    assert_eq!(client_commands(t.prompt(&lp)).last(), Some(&(0x36, [11, 0, 20 << 24 | 5 << 20 | 5 << 16 | 303, 0])));
    assert_eq!(t.dialog_result(15), Some(Prompt::Answer(json!({ "type": 19, "value": 2 }))), "1500 LP");
}

#[test]
fn a_card_name_declaration_searches_the_clients_own_card_database() {
    // The card list's extended kinds (0x4C-0x4F, DuelCardListDialogEx 0x5131E0, dlg_cardlistex.swf) list the client's
    // own card database with a name search (CardDuelFilter 0x4976A0: 0x4C any card), not the server's rows; DLGResult is
    // the picked card's Konami id (0x4C8290, id & 0x3FFF). The answer names it by Konami id (the sidecar turns it into
    // a passcode); the core asks again if the card doesn't fit.
    let mut t = with_cards();
    let declare = json!({ "type": 142, "player": 0, "opcodes": [0x40000001_u64] });
    assert_eq!(client_commands(t.prompt(&declare)).last(), Some(&(0x37, [0, 0x4C, 0, 0])));
    assert_eq!(t.dialog_result(4007), Some(Prompt::Answer(json!({ "type": 18, "cid": 4007 }))));
}
