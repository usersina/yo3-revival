//! Card rental between players: Player Information's Send Rental Request (agent steps 0x4A9D80, 0x4AA430) invites,
//! the other player answers from their bell (flag 0x800, pushes 0x2C25/0x2C27/0x2C29), and both go to the rental
//! scene (CardRental.cpp, handler 0x41C7E0): the sender is the lender, the one who accepts the borrower. The
//! rentals are the records 0x2D6C: the client adds a record's cards to the collection of its `to_id` and takes them
//! from the one of its `from_id` while its confirm flag is 0 (collection builder 0x4F6340), so the server moves no
//! cards between collections.
mod common;

use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::Directory;
use yo3_server::front::World;

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn text(m: &Message, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or_default()
}

fn send(c: &mut Client, id: u32, fields: &[(&str, u32)]) -> Message {
    let fields: Vec<(&str, Value)> = fields.iter().map(|(k, v)| (*k, Value::U32(*v))).collect();
    one(c.send(req_with(id, &fields)))
}

fn requests(pushed: &[Message]) -> Vec<u32> {
    pushed.iter().filter_map(Message::request).collect()
}

struct Table {
    server: Server,
    dir: Arc<Mutex<Directory>>,
    a: Client,
    b: Client,
    a_id: u32,
    b_id: u32,
}

/// Two players of their own accounts; the first (the lender) owns two of row 10 and one of row 11, the second one of
/// row 20.
fn table() -> Table {
    let dir = Arc::new(Mutex::new(Directory::new(vec!["Duelist".into()])));
    let (a_addr, b_addr) = ("10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap());
    let a_id = dir.lock().unwrap().account_for(a_addr).players[0].player_id;
    let b_id = dir.lock().unwrap().account_for(b_addr).players[0].player_id;
    dir.lock().unwrap().update_player(a_id, |p| {
        p.cards.insert(10, 2);
        p.cards.insert(11, 1);
    });
    dir.lock().unwrap().update_player(b_id, |p| {
        p.cards.insert(20, 1);
    });
    let server = Server::with_world(World { card_rows: 50, ..World::new((1..=20).collect()) });
    server.hub.lock().unwrap().set_directory(dir.clone());
    let (a, b) = (server.join_from(&dir, a_addr), server.join_from(&dir, b_addr));
    Table { server, dir, a, b, a_id, b_id }
}

/// The invitation, and both scenes ready: 0x2C24 -> 0x2D50, 0x2C26 answer 2 -> 0x2D54, 0x2C3E each.
fn in_scene(t: &mut Table) {
    assert_eq!(send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]).request(), Some(0x2D50));
    t.b.pushed();
    assert_eq!(send(&mut t.b, 0x2C26, &[("to_id", t.a_id), ("answer", 2)]).request(), Some(0x2D54));
    t.a.pushed();
    one(t.a.send(req(0x2C3E)));
    one(t.b.send(req(0x2C3E)));
    t.a.pushed();
    t.b.pushed();
}

fn cards(c: &mut Client, main: &[u16], extra: &[u16], side: &[u16]) -> Message {
    one(c.send(req_with(
        0x2C2A,
        &[("main_list", Value::u16_be_array(main)), ("extra_list", Value::u16_be_array(extra)), ("side_list", Value::u16_be_array(side))],
    )))
}

fn conditions(c: &mut Client, kind: u32, days: u32) -> Message {
    send(c, 0x2C2B, &[("type", kind), ("days", days)])
}

/// The rental of rows 10 and 11 for 7 days (Free), confirmed by the borrower; the lender's pushes are drained.
fn rented(t: &mut Table) {
    in_scene(t);
    assert_eq!(conditions(&mut t.a, 0, 7).request(), Some(0x2D5C));
    assert_eq!(cards(&mut t.a, &[10, 11], &[], &[]).request(), Some(0x2D5A));
    assert_eq!(one(t.a.send(req(0x2C2D))).request(), Some(0x2D60));
    assert_eq!(send(&mut t.b, 0x2C2F, &[("answer", 2)]).request(), Some(0x2D64));
}

#[test]
fn a_rental_invitation_reaches_the_other_player() {
    // 0x2C24 {to_id} -> 0x2D50 {pname} ("Sending Rental Request to %s..."); the other gets 0x2C25 {from_id,
    // from_name}. 0x2C26 {to_id, answer 2} -> 0x2D54; the inviter gets 0x2C27 {answer}, 2 opening the scene.
    let mut t = table();
    t.b.pushed();
    let r = send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]);
    assert_eq!((r.request(), text(&r, "pname").as_str()), (Some(0x2D50), "Duelist2"));
    let invite = one(t.b.pushed());
    assert_eq!((invite.request(), int(&invite, "from_id"), text(&invite, "from_name").as_str()), (Some(0x2C25), Some(t.a_id), "Duelist"));
    send(&mut t.b, 0x2C26, &[("to_id", t.a_id), ("answer", 2)]);
    let answer = one(t.a.pushed());
    assert_eq!((answer.request(), int(&answer, "answer")), (Some(0x2C27), Some(2)));
    // Both scenes say ready (0x2C3E); when both are, both get 0x2C3F (NOTICE_START).
    one(t.a.send(req(0x2C3E)));
    assert!(t.a.pushed().is_empty());
    one(t.b.send(req(0x2C3E)));
    assert_eq!(requests(&t.a.pushed()), [0x2C3F]);
    assert_eq!(requests(&t.b.pushed()), [0x2C3F]);
}

#[test]
fn an_invitation_can_be_cancelled_or_refused() {
    let mut t = table();
    send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]);
    t.b.pushed();
    // The inviter cancels: 0x2C28 -> 0x2D56, the player gets 0x2C29 (their bell clears).
    assert_eq!(one(t.a.send(req(0x2C28))).request(), Some(0x2D56));
    assert_eq!(requests(&t.b.pushed()), [0x2C29]);
    // A refusal (answer 1) gives the inviter 0x2C27 {answer 1} and opens no scene.
    send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]);
    send(&mut t.b, 0x2C26, &[("to_id", t.a_id), ("answer", 1)]);
    assert_eq!(int(&t.a.pushed().pop().unwrap(), "answer"), Some(1));
    assert_eq!(send(&mut t.a, 0x2C2B, &[("type", 0), ("days", 1)]).request(), Some(0x2D5D), "no scene to set conditions in");
}

#[test]
fn an_invitation_to_someone_away_or_busy_fails() {
    // 0x2D51 {error_code}: 106 "This player cannot receive another request." (agent step 0x4A9D80).
    let mut t = table();
    let r = send(&mut t.a, 0x2C24, &[("to_id", 4242)]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2D51), Some(106)));
    send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]);
    let again = send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]);
    assert_eq!((again.request(), int(&again, "error_code")), (Some(0x2D51), Some(106)));
}

#[test]
fn the_lender_sets_conditions_and_puts_up_cards() {
    // 0x2C2B {type 0 Free / 1 Deck, days 1, 3, 7 or 14} -> 0x2D5C; the borrower gets 0x2C39 {type, days} (SetType).
    // 0x2C2A {main_list, extra_list, side_list} -> 0x2D5A; the borrower gets 0x2C38 with the lists, and acks with
    // 0x2C2C -> 0x2D5E; the lender gets 0x2C3A. 0x2D5B/0x2D5D {error_code}: 504 cards the lender doesn't own.
    let mut t = table();
    in_scene(&mut t);
    assert_eq!(conditions(&mut t.a, 1, 14).request(), Some(0x2D5C));
    let set = one(t.b.pushed());
    assert_eq!((set.request(), int(&set, "type"), int(&set, "days")), (Some(0x2C39), Some(1), Some(14)));
    assert_eq!(conditions(&mut t.a, 1, 5).request(), Some(0x2D5D), "5 days is no term");
    assert_eq!(cards(&mut t.a, &[10, 10, 11], &[11], &[]).request(), Some(0x2D5B), "row 11 only once");
    assert_eq!(cards(&mut t.a, &[10, 10, 11], &[], &[]).request(), Some(0x2D5A));
    let list = one(t.b.pushed());
    assert_eq!(list.request(), Some(0x2C38));
    assert_eq!(list.get("main_list"), Some(&Value::u16_be_array(&[10, 10, 11])));
    assert_eq!(list.get("extra_list"), Some(&Value::u16_be_array(&[])));
    assert_eq!(one(t.b.send(req(0x2C2C))).request(), Some(0x2D5E));
    assert_eq!(requests(&t.a.pushed()), [0x2C3A]);
    // The borrower can't choose cards, nor a Free list hold more than 90 or a deck's main list more than 60.
    assert_eq!(cards(&mut t.b, &[20], &[], &[]).request(), Some(0x2D5B));
    assert_eq!(cards(&mut t.a, &[10; 61], &[], &[]).request(), Some(0x2D5B));
}

#[test]
fn a_key_only_lenders_offer_is_checked_against_their_saved_cards() {
    // CardRental 0x41C7E0: a key-only account must not bypass the same ownership check as an address account.
    let mut t = table();
    let mut d = t.dir.lock().unwrap();
    d.add_account(&["KeyLender".into()]).unwrap();
    let user = d.user_of("KeyLender").unwrap();
    t.a_id = d.account_of(user).unwrap().players[0].player_id;
    d.update_player(t.a_id, |p| { p.cards.insert(10, 1); });
    drop(d);
    t.a = t.server.join_saved(&t.dir, user);
    in_scene(&mut t);
    let r = cards(&mut t.a, &[99], &[], &[]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2D5B), Some(504)));
    assert_eq!(cards(&mut t.a, &[10], &[], &[]).request(), Some(0x2D5A));
    one(t.a.send(req(0x2C2D)));
    send(&mut t.b, 0x2C2F, &[("answer", 2)]);
    assert_eq!(one(t.a.send(req(0x2C34))).items.len(), 1);
}

#[test]
fn the_borrower_confirms_and_the_rental_starts() {
    // 0x2C2D (the lender's Confirm) -> 0x2D60; the borrower gets 0x2C2E ("Borrow these cards?"), answers 0x2C2F
    // {answer 2} -> 0x2D64; both get 0x2C30 {trade_result 1} ("Rental Completed.") and the records 0x2D6C.
    let mut t = table();
    in_scene(&mut t);
    conditions(&mut t.a, 0, 7);
    assert_eq!(one(t.a.send(req(0x2C2D))).request(), Some(0x2D61), "nothing put up");
    cards(&mut t.a, &[10, 11], &[], &[]);
    t.b.pushed();
    assert_eq!(one(t.a.send(req(0x2C2D))).request(), Some(0x2D60));
    assert_eq!(requests(&t.b.pushed()), [0x2C2E]);
    assert_eq!(send(&mut t.b, 0x2C2F, &[("answer", 2)]).request(), Some(0x2D64));
    for c in [&mut t.a, &mut t.b] {
        let pushed = c.pushed();
        let done = pushed.iter().find(|m| m.request() == Some(0x2C30)).expect("0x2C30");
        assert_eq!(int(done, "trade_result"), Some(1));
        let records = pushed.iter().find(|m| m.request() == Some(0x2D6C)).expect("the records");
        assert_eq!(records.items.len(), 1);
        let r = &records.items[0];
        assert_eq!((int(r, "from_id"), int(r, "to_id")), (Some(t.a_id), Some(t.b_id)));
        assert_eq!((text(r, "from_name").as_str(), text(r, "to_name").as_str()), ("Duelist", "Duelist2"));
        assert_eq!((int(r, "from_type"), int(r, "deck_type"), int(r, "confirm_from"), int(r, "confirm_to")), (Some(0), Some(0), Some(0), Some(0)));
        assert_eq!(r.get("main"), Some(&Value::u16_be_array(&[10, 11])));
        assert_eq!(text(r, "rent_end").len(), "2026-11-15 12:00:00".len());
    }
}

#[test]
fn a_refused_or_withdrawn_confirmation_rents_nothing() {
    let mut t = table();
    in_scene(&mut t);
    cards(&mut t.a, &[10], &[], &[]);
    one(t.a.send(req(0x2C2D)));
    t.b.pushed();
    // The lender takes the Confirm back: 0x2C31 -> 0x2D66, the borrower gets 0x2C32 and can't answer any more.
    assert_eq!(one(t.a.send(req(0x2C31))).request(), Some(0x2D66));
    assert_eq!(requests(&t.b.pushed()), [0x2C32]);
    assert_eq!(send(&mut t.b, 0x2C2F, &[("answer", 2)]).request(), Some(0x2D65));
    // The borrower says no (answer 1): both get 0x2C30 {trade_result 0} ("Rental Cancelled."), no records.
    one(t.a.send(req(0x2C2D)));
    t.b.pushed();
    assert_eq!(send(&mut t.b, 0x2C2F, &[("answer", 1)]).request(), Some(0x2D64));
    for c in [&mut t.a, &mut t.b] {
        let pushed = c.pushed();
        assert_eq!(pushed.iter().filter_map(|m| int(m, "trade_result")).collect::<Vec<_>>(), [0]);
        assert!(pushed.iter().all(|m| m.request() != Some(0x2D6C)));
    }
    assert!(one(t.a.send(req(0x2C34))).items.is_empty());
}

#[test]
fn leaving_the_scene_tells_the_partner() {
    let mut t = table();
    in_scene(&mut t);
    assert_eq!(one(t.b.send(req(0x2C33))).request(), Some(0x2D6A));
    assert_eq!(requests(&t.a.pushed()), [0x2C37]);
    // The scene is gone: the lender is free to invite again, and a player who drops out ends it as well.
    in_scene(&mut t);
    drop(t.b);
    assert_eq!(requests(&t.a.pushed()), [0x2C37]);
}

#[test]
fn rented_cards_count_for_the_borrower_and_not_the_lender() {
    // The records are all the client needs (the collection builder adds the borrower's and takes the lender's): the
    // trunk 9000 stays what each owns, and each gets the records at login, at the Rental Card List (0x2C34) and
    // when it changes.
    let mut t = table();
    rented(&mut t);
    let for_lender = one(t.a.send(req(0x2C34)));
    let for_borrower = one(t.b.send(req(0x2C34)));
    assert_eq!((for_lender.request(), for_lender.items.len()), (Some(0x2D6C), 1));
    assert_eq!(for_lender.items[0], for_borrower.items[0]);
    let owned = |replies: &[Message], row: usize| replies[0].items[0].get("cards").and_then(Value::as_bytes).unwrap()[row - 1];
    let lender = t.a.send(req_with(0x1FA4, &[("player_id", Value::U32(t.a_id))]));
    let borrower = t.b.send(req_with(0x1FA4, &[("player_id", Value::U32(t.b_id))]));
    assert_eq!((lender[0].request(), owned(&lender, 10), owned(&lender, 11)), (Some(9000), 2, 1), "the lender's trunk is what they own");
    assert_eq!((owned(&borrower, 10), owned(&borrower, 11), owned(&borrower, 20)), (0, 0, 1), "the client adds the rented cards");
    assert_eq!(lender[3].items.len(), 1, "the login's records carry the rental");
    assert_eq!(borrower[3].items.len(), 1);
    // The lent cards can't be offered in a trade (the trade scene takes them away from the lender's trunk): row 10
    // is owned twice, rented once; row 11 once, rented.
    assert_eq!(send(&mut t.a, 0x2BC0, &[("to_id", t.b_id)]).request(), Some(0x2BD7));
    t.b.pushed();
    send(&mut t.b, 0x2BC2, &[("to_id", t.a_id), ("answer", 2)]);
    one(t.a.send(req(0x2BD5)));
    one(t.b.send(req(0x2BD5)));
    let offer = |c: &mut Client, rows: &[u16]| one(c.send(req_with(0x2BC6, &[("main_list", Value::u16_be_array(rows))]))).request();
    assert_eq!(offer(&mut t.a, &[11]), Some(0x2BE2), "row 11 is lent");
    assert_eq!(offer(&mut t.a, &[10, 10]), Some(0x2BE2), "one of row 10 is lent");
    assert_eq!(offer(&mut t.a, &[]), Some(0x2BE1));
    let _ = (&t.dir, &t.server);
}

#[test]
fn a_card_lent_cannot_be_lent_twice() {
    let mut t = table();
    rented(&mut t);
    t.a.pushed();
    t.b.pushed();
    // A second rental to the borrower: row 11 is out, one of the two row 10 is left.
    send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]);
    t.b.pushed();
    send(&mut t.b, 0x2C26, &[("to_id", t.a_id), ("answer", 2)]);
    assert_eq!(cards(&mut t.a, &[11], &[], &[]).request(), Some(0x2D5B));
    assert_eq!(cards(&mut t.a, &[10, 10], &[], &[]).request(), Some(0x2D5B));
    assert_eq!(cards(&mut t.a, &[10], &[], &[]).request(), Some(0x2D5A));
}

#[test]
fn the_rental_lists_show_who_lent_and_borrowed() {
    // 0x2C35 {rental_kind 0 lent, 1 borrowed} -> 0x2D6E (the client keeps the first reply's records for the Lent tab,
    // GetLoanCount 0x482FE0 reads +0x78, and the second's for Borrowed, GetBorrowCount 0x482970 reads +0x90): the
    // borrowed rows read from_name, from_type, rent_end, deck_type and confirm_to (0x482A90), the lent rows to_name,
    // rent_end, deck_type and confirm_from (0x483100).
    let mut t = table();
    rented(&mut t);
    let borrowed = send(&mut t.b, 0x2C35, &[("rental_kind", 1), ("base_id", 0)]);
    let lent = send(&mut t.a, 0x2C35, &[("rental_kind", 0), ("base_id", 0)]);
    assert_eq!((borrowed.request(), borrowed.items.len(), lent.request(), lent.items.len()), (Some(0x2D6E), 1, Some(0x2D6E), 1));
    assert_eq!(text(&borrowed.items[0], "from_name"), "Duelist");
    assert_eq!(text(&lent.items[0], "to_name"), "Duelist2");
    // Each side lists only its own: the lender has borrowed nothing, the borrower lent nothing.
    assert!(send(&mut t.a, 0x2C35, &[("rental_kind", 1), ("base_id", 0)]).items.is_empty());
    assert!(send(&mut t.b, 0x2C35, &[("rental_kind", 0), ("base_id", 0)]).items.is_empty());
}

#[test]
fn a_rental_ends_when_its_time_is_up() {
    // 7 days: before that nothing changes; after, the confirm flags are 1 ("over"), the active records are gone from
    // 0x2D6C (pushed to both) and the lender owns the cards again.
    let mut t = table();
    rented(&mut t);
    t.a.pushed();
    t.b.pushed();
    let now = chrono::Utc::now().timestamp();
    t.server.hub.lock().unwrap().tick_rentals(now + 6 * 86_400);
    assert!(t.a.pushed().is_empty());
    t.server.hub.lock().unwrap().tick_rentals(now + 8 * 86_400);
    for c in [&mut t.a, &mut t.b] {
        let records = one(c.pushed());
        assert_eq!((records.request(), records.items.len()), (Some(0x2D6C), 0));
    }
    let borrowed = send(&mut t.b, 0x2C35, &[("rental_kind", 1), ("base_id", 0)]);
    assert_eq!((int(&borrowed.items[0], "confirm_to"), int(&borrowed.items[0], "confirm_from")), (Some(1), Some(1)));
    // Row 11 can be traded again.
    send(&mut t.a, 0x2BC0, &[("to_id", t.b_id)]);
    t.b.pushed();
    send(&mut t.b, 0x2BC2, &[("to_id", t.a_id), ("answer", 2)]);
    one(t.a.send(req(0x2BD5)));
    one(t.b.send(req(0x2BD5)));
    let offer = one(t.a.send(req_with(0x2BC6, &[("main_list", Value::u16_be_array(&[11]))])));
    assert_eq!(offer.request(), Some(0x2BE1));
}

#[test]
fn rentals_survive_a_restart() {
    let file = std::env::temp_dir().join(format!("yo3-rentals-{}", std::process::id())).join("rentals.json");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    let mut t = table();
    t.server.hub.lock().unwrap().keep_rentals(&file);
    rented(&mut t);
    // A new server (hub) reads them back; the records come with the login's pushes, and time still runs.
    let again = Server::with_world(World { card_rows: 50, ..World::new((1..=20).collect()) });
    again.hub.lock().unwrap().keep_rentals(&file);
    again.hub.lock().unwrap().set_directory(t.dir.clone());
    let mut lender = again.join_from(&t.dir, "10.0.0.1".parse().unwrap());
    let records = one(lender.send(req(0x2C34)));
    assert_eq!(records.items.len(), 1);
    assert_eq!(records.items[0].get("main"), Some(&Value::u16_be_array(&[10, 11])));
    again.hub.lock().unwrap().tick_rentals(chrono::Utc::now().timestamp() + 8 * 86_400);
    assert!(one(lender.send(req(0x2C34))).items.is_empty());
    // And the ended rental stays ended in the file.
    let third = Server::with_world(World::new((1..=20).collect()));
    third.hub.lock().unwrap().keep_rentals(&file);
    third.hub.lock().unwrap().set_directory(t.dir.clone());
    let mut lender = third.join_from(&t.dir, "10.0.0.1".parse().unwrap());
    assert!(one(lender.send(req(0x2C34))).items.is_empty());
}

/// A third player, on an account of their own.
fn third(t: &Table) -> (Client, u32) {
    let addr = "10.0.0.3".parse().unwrap();
    let id = t.dir.lock().unwrap().account_for(addr).players[0].player_id;
    (t.server.join_from(&t.dir, addr), id)
}

#[test]
fn a_player_in_a_rental_scene_cannot_start_another() {
    let mut t = table();
    let (mut c, c_id) = third(&t);
    in_scene(&mut t);
    let r = send(&mut t.a, 0x2C24, &[("to_id", c_id)]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2D51), Some(106)));
    assert!(c.pushed().is_empty());
}

#[test]
fn a_rental_invitation_accepted_after_the_lender_started_another_is_refused() {
    let mut t = table();
    let (mut c, c_id) = third(&t);
    assert_eq!(send(&mut t.a, 0x2C24, &[("to_id", t.b_id)]).request(), Some(0x2D50));
    assert_eq!(send(&mut t.a, 0x2C24, &[("to_id", c_id)]).request(), Some(0x2D50));
    assert_eq!(send(&mut t.b, 0x2C26, &[("to_id", t.a_id), ("answer", 2)]).request(), Some(0x2D54));
    c.pushed();
    assert_eq!(send(&mut c, 0x2C26, &[("to_id", t.a_id), ("answer", 2)]).request(), Some(0x2D55));
    assert_eq!(send(&mut c, 0x2C3E, &[]).request(), Some(0x2D69), "no scene for the third player");
    t.b.pushed();
    assert_eq!(one(t.a.send(req(0x2C33))).request(), Some(0x2D6A));
    assert_eq!(requests(&t.b.pushed()), [0x2C37]);
}
