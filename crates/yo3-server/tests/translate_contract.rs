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
    assert_eq!(t.dialog_result(1), Some(json!({ "type": 5, "indicies": [1] })));
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
    assert_eq!(t.dialog_result(2), Some(json!({ "type": 12, "indicies": [1, 0] })));
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
    assert_eq!(t.dialog_result(0), Some(json!({ "type": 5, "indicies": [] })));
    let cancelable = json!({ "type": 15, "player": 0, "can_cancel": true, "min": 1, "max": 1,
        "selects": [card_at(0, 2, 0, 4100)] });
    t.prompt(&cancelable);
    assert_eq!(t.dialog_result(0), Some(json!({ "type": 5, "indicies": null })));
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
