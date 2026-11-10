//! The duel check's client model (`duelcheck.rs`) on its own: what the client's card manager does with each card
//! command, and the comparison with an engine state. No engine needed.
use std::collections::HashMap;

use serde_json::json;
use yo3_proto::{Message, Value};
use yo3_server::duel::command;
use yo3_server::duelcheck::ClientModel;
use yo3_server::translate::CardStats;

/// Rows 100.. are cards 4100.. (row 100 is card 4100).
fn model() -> ClientModel {
    let cards = (100..120).map(|r| (r, CardStats { cid: 4000 + r, ..CardStats::default() })).collect::<HashMap<_, _>>();
    let mut m = ClientModel::new(0, cards);
    let setup = [("MainDeckNum001", 3), ("MainDeckNum002", 0), ("ExtraDeckNum001", 0), ("ExtraDeckNum002", 0)]
        .into_iter()
        .fold(Message::reply(0x200D).with("duelRequestName", Value::U32(0x15)), |m, (k, v)| m.with(k, Value::U32(v)));
    m.apply(&setup);
    for uid in 1..=3 {
        m.apply(&command(1000, [uid, 99 + uid, 0, 0xD << 8 | (uid - 1) << 16]));
    }
    m.apply(&command(0x16, [0, 0xD, 0, 0]));
    m
}

/// Move places: player | zone << 1 | index << 6, face up bit 14, defense bit 15.
fn at(zone: u32, index: u32) -> u32 {
    zone << 1 | index << 6
}

/// Draws uids 1-3 (rows 100-102) into the hand, with their table entries.
fn drawn() -> ClientModel {
    let mut m = model();
    for uid in 1..=3 {
        m.apply(&command(1000, [uid, 99 + uid, 0, 0xB << 8 | (uid - 1) << 16]));
        m.apply(&command(0x1A, [uid | (99 + uid) << 16, at(0xD, 0), at(0xB, uid - 1), 0]));
    }
    m
}

/// An engine state with team 0's hand and monster zones as given, team 1 empty.
fn state(hand: &[u32], mzone: [Option<(u32, u32)>; 5], deck: u32) -> serde_json::Value {
    let card = |cid: u32, position: u32| json!({ "cid": cid, "code": cid, "position": position });
    let side = |hand: Vec<serde_json::Value>, mzone: Vec<serde_json::Value>, deck: u32| {
        json!({ "hand": hand, "mzone": mzone, "szone": [null, null, null, null, null, null], "grave": [], "removed": [], "extra": [], "deck": deck })
    };
    let hand = hand.iter().map(|&c| card(c, 10)).collect();
    let mzone = mzone.iter().map(|z| z.map_or(json!(null), |(c, p)| card(c, p))).collect();
    json!([side(hand, mzone, deck), side(vec![], vec![json!(null); 5], 0)])
}

#[test]
fn a_battle_destroy_flag_hides_a_card_even_when_its_slot_stays_occupied() {
    // 0x4F8920's destroy effect ends at 0x4FB220 mode 0. BattleEnd does not restore that card's visibility.
    let mut m = drawn();
    m.apply(&command(0x1A, [1 | 100 << 16, at(0xB, 0), at(0, 0) | 1 << 14, 0]));
    m.apply(&command(0x1A, [2 | 101 << 16, at(0xB, 0), 1 | at(1, 0) | 1 << 14, 0]));
    m.apply(&command(1000, [1, 100, 0, 0]));
    m.apply(&command(1000, [2, 101, 0, 1 | 1 << 8]));
    m.apply(&command(1000, [3, 102, 0, 0xB << 8]));
    let mut engine = state(&[4102], [Some((4100, 1)), None, None, None, None], 0);
    engine[1]["mzone"][1] = json!({"cid":4101,"position":1});
    let initial = m.check(&engine); assert!(initial.is_empty(), "{initial:?}");
    m.apply(&command(0xA, [0; 4]));
    m.apply(&command(0xC, [1 | 1 << 8, 0, 0, 0]));
    m.apply(&command(0xD, [1400, 0, 2, 0]));
    m.apply(&command(0xE, [0; 4]));
    assert!(!m.check(&engine).is_empty(), "the battle hid a surviving monster: occupied slots alone are insufficient");
    m.apply(&command(1000, [1, 100, 0, 0xE << 8]));
    m.apply(&command(0x1A, [1 | 100 << 16, at(0, 0) | 1 << 14, at(0xE, 0) | 1 << 14, 0]));
    engine[0]["mzone"][0] = json!(null);
    engine[0]["grave"] = json!([{"cid":4100,"position":1}]);
    let settled = m.check(&engine); assert!(settled.is_empty(), "a subsequent real move settles the hidden card normally: {settled:?}");
}

#[test]
fn a_hand_and_the_deck_pile_follow_the_moves() {
    // Drawn cards go to the end of the hand (0x52A400), the deck pile counts down from the setup's 3 (DeckReset 0x16,
    // then a card leaving the pile: 0x4FE4D0).
    let mut m = drawn();
    assert_eq!(m.check(&state(&[4100, 4101, 4102], [None; 5], 0)), Vec::<String>::new());
    let wrong = m.check(&state(&[4101, 4100, 4102], [None; 5], 1));
    assert!(wrong.iter().any(|p| p.contains("hand index 0")), "{wrong:?}");
    assert!(wrong.iter().any(|p| p.contains("deck")), "{wrong:?}");
}

#[test]
fn a_hand_card_leaving_by_another_cards_index_is_the_crash() {
    // The client takes a hand card out by the move's index (0x5005D0), not by uid: uid 1 leaving by index 1 drops uid 2
    // and keeps uid 1 in the hand list, which crashed the client at 0x516185 on its next renumbering.
    let mut m = drawn();
    m.apply(&command(0x1A, [1 | 100 << 16, at(0xB, 1), at(2, 0) | 1 << 14, 0]));
    let problems = m.take_problems();
    assert!(problems.iter().any(|p| p.contains("drops uid 2 and keeps uid 1")), "{problems:?}");
}

#[test]
fn a_card_reaching_the_hand_goes_to_its_end_whatever_the_index() {
    // 0x5000E0 pushes a card that lands in a hand to the back of the list (0x52A400), not at the move's index.
    let mut m = drawn();
    m.apply(&command(0x1A, [1 | 100 << 16, at(0xB, 0), at(2, 0) | 1 << 14, 0]));
    m.apply(&command(1000, [2, 101, 0, 0xB << 8]));
    m.apply(&command(1000, [3, 102, 0, 0xB << 8 | 1 << 16]));
    m.apply(&command(1000, [1, 100, 0, 0xB << 8 | 2 << 16]));
    m.apply(&command(0x1A, [1 | 100 << 16, at(2, 0), at(0xB, 0), 0]));
    assert_eq!(m.check(&state(&[4101, 4102, 4100], [None; 5], 0)), Vec::<String>::new());
}

#[test]
fn a_hand_shuffle_rebuilds_the_hand_from_the_table() {
    // HandShuffle 0x12 {player} takes the card at each hand index from the table (0x523DF0); an index without an entry
    // stops the client (error state 0x1200).
    let mut m = drawn();
    m.apply(&command(1000, [3, 102, 0, 0xB << 8]));
    m.apply(&command(1000, [1, 100, 0, 0xB << 8 | 1 << 16]));
    m.apply(&command(1000, [2, 101, 0, 0xB << 8 | 2 << 16]));
    m.apply(&command(0x12, [0, 0, 0, 0]));
    assert_eq!(m.check(&state(&[4102, 4100, 4101], [None; 5], 0)), Vec::<String>::new());
    m.apply(&command(1000, [2, 101, 0, 0xE << 8]));
    m.apply(&command(0x12, [0, 0, 0, 0]));
    let problems = m.take_problems();
    assert!(problems.iter().any(|p| p.contains("no table entry at hand index 2")), "{problems:?}");
}

#[test]
fn a_swap_trades_two_slots_and_a_vanished_card_is_off_the_field() {
    // CardSwap 0x1B {_, A, B}: the slots trade cards, each taking the word it lands on (case 0x1B). CardVanish 0x1F marks
    // the card off the field (+0xD7) until it lands again.
    let mut m = drawn();
    m.apply(&command(0x1A, [1 | 100 << 16, at(0xB, 0), at(0, 0) | 1 << 14, 0]));
    m.apply(&command(0x1A, [2 | 101 << 16, at(0xB, 0), at(1, 0) | 1 << 15, 0]));
    m.apply(&command(0x1B, [0, at(0, 0) | 1 << 15, at(1, 0) | 1 << 14, 0]));
    m.apply(&command(1000, [2, 101, 0, 0]));
    m.apply(&command(1000, [1, 100, 0, 1 << 8]));
    m.apply(&command(1000, [3, 102, 0, 0xB << 8]));
    let swapped = [Some((4101, 8)), Some((4100, 1)), None, None, None];
    assert_eq!(m.check(&state(&[4102], swapped, 0)), Vec::<String>::new());
    // The engine's positions differ from the words: the check says so.
    let turned = m.check(&state(&[4102], [Some((4101, 1)), Some((4100, 1)), None, None, None], 0));
    assert!(turned.iter().any(|p| p.contains("zone 0") && p.contains("face-up attack in the engine")), "{turned:?}");
    m.apply(&command(0x1F, [0, 1, 0, 0]));
    assert_eq!(m.check(&state(&[4102], [Some((4101, 8)), None, None, None, None], 0)), Vec::<String>::new());
}
