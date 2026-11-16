//! The position dialog draws its two buttons from one card's row in the client's card table (DuelSelectStandDialog
//! 0x4C5E50). The engine's MSG_SELECT_POSITION names only the card's id, so the dialog shows a copy of it: one the
//! client can see. Reported 2026-11-16 in an NPC duel: "Select position for Doom Shaman" with nothing on either button,
//! so nothing to click. The NPC's hidden cards reach the client with row 0 (since v0.0.11), and the dialog had picked
//! a copy still in the NPC's hand or deck rather than the one in its Graveyard the player was summoning.
use std::collections::HashMap;

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};
use yo3_server::translate::{CardStats, Prompt, Translator};

const DOOM_SHAMAN: u16 = 7202;
const SHAMAN_ROW: u16 = 1;
const FILLER_ROW: u16 = 2;

fn cards() -> HashMap<u16, CardStats> {
    let stats = |cid| CardStats { cid, atk: 1000, def: 1000, level: 4, attribute: 0x20, race: 0x8 };
    HashMap::from([(SHAMAN_ROW, stats(DOOM_SHAMAN)), (FILLER_ROW, stats(4028))])
}

/// The card table commands (1000 {uid, row, ...}) among `messages`, newest last.
fn table(rows: &mut HashMap<u32, u32>, messages: &[Message]) {
    for m in messages.iter().filter(|m| m.get("id") == Some(&Value::U32(1000))) {
        let int = |k| m.get(k).and_then(Value::as_u32).unwrap_or(0);
        rows.insert(int("param1") & 0xFF_FFFF, int("param2"));
    }
}

fn place(location: u32, sequence: u32) -> Json { json!({ "controller": 1, "location": location, "sequence": sequence, "position": 1 }) }

#[test]
fn the_position_dialog_shows_a_copy_the_client_can_see() {
    const DECK: u32 = 0x01;
    const HAND: u32 = 0x02;
    const GRAVE: u32 = 0x10;
    // Each order of the opponent's zones: the copy the dialog finds mustn't depend on a map's order.
    for round in 0..20 {
        // The opponent's deck: Doom Shaman at the top and bottom, the client's an ordinary one.
        let opponent = [SHAMAN_ROW, FILLER_ROW, FILLER_ROW, FILLER_ROW, SHAMAN_ROW, SHAMAN_ROW];
        let (mut t, setup) = Translator::against_player(0, [&[FILLER_ROW; 6], &opponent], cards());
        let mut rows = HashMap::new();
        table(&mut rows, &setup);
        // One Doom Shaman to the opponent's hand, one to its Graveyard (face up, so the client gets its row).
        let shaman = json!({ "code": 94944637, "cid": DOOM_SHAMAN, "name": "Doom Shaman" });
        let moved = |from: Json, to: Json| { let mut m = shaman.clone(); m["type"] = json!(50); m["from"] = from; m["to"] = to; m };
        table(&mut rows, &t.events(&[moved(place(DECK, 5), place(HAND, 0)), moved(place(DECK, 4), place(GRAVE, 0))]));
        // The client Special Summons it from the opponent's Graveyard: its position.
        let ask = json!({ "type": 19, "player": 0, "code": 94944637, "cid": DOOM_SHAMAN, "name": "Doom Shaman", "positions": 5 });
        let Prompt::Client(out) = t.prompt(&ask) else { panic!("no position dialog") };
        table(&mut rows, &out);
        let dialog = out.iter().find(|m| m.get("id") == Some(&Value::U32(0x36)) && m.get("param1") == Some(&Value::U32(8)))
            .expect("the position dialog");
        let uid = dialog.get("param3").and_then(Value::as_u32).unwrap() & 0xFFFF;
        assert_eq!(rows.get(&uid).copied(), Some(SHAMAN_ROW as u32),
            "round {round}: the dialog names uid {uid}, which the client holds as row {:?}: nothing to draw", rows.get(&uid));
    }
}

/// Every copy hidden (summoned from the opponent's hand): the copy the dialog shows gets its row first.
#[test]
fn a_hidden_card_asked_its_position_is_shown_to_the_client_first() {
    const DECK: u32 = 0x01;
    const HAND: u32 = 0x02;
    let opponent = [SHAMAN_ROW, FILLER_ROW, FILLER_ROW, FILLER_ROW, FILLER_ROW, FILLER_ROW];
    let (mut t, setup) = Translator::against_player(0, [&[FILLER_ROW; 6], &opponent], cards());
    let mut rows = HashMap::new();
    table(&mut rows, &setup);
    let mut m = json!({ "type": 50, "code": 94944637, "cid": DOOM_SHAMAN, "name": "Doom Shaman" });
    m["from"] = place(DECK, 5);
    m["to"] = place(HAND, 0);
    table(&mut rows, &t.events(&[m]));
    let ask = json!({ "type": 19, "player": 0, "code": 94944637, "cid": DOOM_SHAMAN, "name": "Doom Shaman", "positions": 5 });
    let Prompt::Client(out) = t.prompt(&ask) else { panic!("no position dialog") };
    table(&mut rows, &out);
    let dialog = out.iter().find(|m| m.get("id") == Some(&Value::U32(0x36)) && m.get("param1") == Some(&Value::U32(8))).unwrap();
    let uid = dialog.get("param3").and_then(Value::as_u32).unwrap() & 0xFFFF;
    assert_eq!(rows.get(&uid).copied(), Some(SHAMAN_ROW as u32), "the dialog's card has its row: {out:?}");
}
