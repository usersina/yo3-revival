//! The shop (shopmain.cpp, reply handler 0x48AA60), the map's top-left facility. After its trunk
//! reply it asks for its text (0x1B66, sent at 0x48BEDD), and the text reply asks for the items
//! (0x1B67, 0x4F3590). Requests and replies pair by +0xBA. Neither reply has a global case.
mod common;

use common::{Server, one, req};
use yo3_proto::Value;

#[test]
fn shop_text_carries_the_greeting() {
    // 0x1C20 case: "text" (string) is shown in the shopkeeper's bubble. The case also reads evalnum,
    // evaltotal and evallevel, but from the player's record (UserData+0x2C), not from this reply.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x1B66)));
    assert_eq!(r.request(), Some(0x1C20));
    assert!(r.get("text").and_then(Value::as_str).is_some_and(|t| !t.is_empty()));
    assert_eq!(r.fields.len(), 2, "request and text only: {r:?}");
}

#[test]
fn every_catalogue_request_gets_its_list_even_when_empty() {
    // Each case checks the item count (0x410D90) before reading items, then asks for the next list:
    // 0x1C22 items {item_type, item_param} -> 0x1B6F (0x48AE5F); 0x1C26 exchange items {item_id,
    // item_cost, sale_id, ...} -> 0x1B6B (0x48B3B8); 0x1C32 protectors {item_id} -> 0x1B6D
    // (0x48B4C3); 0x1C2E event items; 0x1C2A avatar items {item_type, item_param}, the last, which
    // clears isLoading.
    let mut c = Server::new().join(1001, "Yugi");
    let chain = [(0x1B67, 0x1C22), (0x1B6F, 0x1C26), (0x1B6B, 0x1C32), (0x1B6D, 0x1C2E), (0x1B69, 0x1C2A)];
    for (request, reply) in chain {
        let r = one(c.send(req(request)));
        assert_eq!(r.request(), Some(reply), "{request:#x}");
        assert!(r.items.is_empty());
    }
}

#[test]
fn the_last_request_gets_the_perk_count() {
    // The event-items case sends 0x1B72 (0x48B64D); 0x1C36 (0x48B68D) reads perk_num and compares
    // it with 500 to show or hide _root.expack_del. Meaning unknown; 0 is a guess.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x1B72)));
    assert_eq!(r.request(), Some(0x1C36));
    assert_eq!(r.get("perk_num"), Some(&Value::U32(0)));
}

#[test]
fn rental_terms_are_answered() {
    // After 0x1C36 the shop sends 0x2C40 (0x48B777); 0x2D75 (0x48C20E) reads rental_cardnum,
    // rental_point and rental_term from the reply, then asks for the avatar items (0x1B69, 0x48C391).
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x2C40)));
    assert_eq!(r.request(), Some(0x2D75));
    for k in ["rental_cardnum", "rental_point", "rental_term"] {
        assert_eq!(r.get(k), Some(&Value::U32(0)), "{k}");
    }
}
