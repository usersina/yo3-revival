//! Card trades between players: Player Information's Send Trade Request (agent step 0x4A91D0) invites, the other
//! player answers from their bell (slot 25, flag 0x400), and both go to the trade scene (CardTrade.cpp, handler
//! 0x4248F0), whose requests and pushes the front pairs between the two.
mod common;

use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::Directory;
use yo3_server::front::World;

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn send(c: &mut Client, id: u32, fields: &[(&str, u32)]) -> Message {
    let fields: Vec<(&str, Value)> = fields.iter().map(|(k, v)| (*k, Value::U32(*v))).collect();
    one(c.send(req_with(id, &fields)))
}

fn requests(pushed: &[Message]) -> Vec<u32> {
    pushed.iter().filter_map(Message::request).collect()
}

/// The owned count of a row in a trunk (9000's cards, a byte at row - 1).
fn count(trunk: &Message, row: usize) -> u8 {
    trunk.items[0].get("cards").and_then(Value::as_bytes).unwrap()[row - 1]
}

struct Table {
    server: Server,
    dir: Arc<Mutex<Directory>>,
    a: Client,
    b: Client,
    a_id: u32,
    b_id: u32,
}

/// Two players of their own accounts; the first owns two of row 10, the second one of row 20.
fn table() -> Table {
    let dir = Arc::new(Mutex::new(Directory::new(vec!["Duelist".into()])));
    let (a_addr, b_addr) = ("10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap());
    let a_id = dir.lock().unwrap().account_for(a_addr).players[0].player_id;
    let b_id = dir.lock().unwrap().account_for(b_addr).players[0].player_id;
    dir.lock().unwrap().update_player(a_id, |p| {
        p.cards.insert(10, 2);
    });
    dir.lock().unwrap().update_player(b_id, |p| {
        p.cards.insert(20, 1);
    });
    let server = Server::with_world(World { card_rows: 50, ..World::new((1..=20).collect()) });
    server.hub.lock().unwrap().set_directory(dir.clone());
    let (a, b) = (server.join_from(&dir, a_addr), server.join_from(&dir, b_addr));
    Table { server, dir, a, b, a_id, b_id }
}

/// The invitation and both scenes ready: 0x2BC0 -> 0x2BD7, 0x2BC2 answer 2 -> 0x2BDB, 0x2BD5 each -> 0x2BED.
fn in_scene(t: &mut Table) {
    assert_eq!(send(&mut t.a, 0x2BC0, &[("to_id", t.b_id)]).request(), Some(0x2BD7));
    t.b.pushed();
    assert_eq!(send(&mut t.b, 0x2BC2, &[("to_id", t.a_id), ("answer", 2)]).request(), Some(0x2BDB));
    t.a.pushed();
    assert_eq!(one(t.a.send(req(0x2BD5))).request(), Some(0x2BED));
    assert_eq!(one(t.b.send(req(0x2BD5))).request(), Some(0x2BED));
    t.a.pushed();
    t.b.pushed();
}

fn offer(c: &mut Client, rows: &[u16]) -> Message {
    one(c.send(req_with(0x2BC6, &[("main_list", Value::u16_be_array(rows))])))
}

#[test]
fn a_trade_invitation_reaches_the_other_player() {
    // 0x2BC0 {to_id} -> 0x2BD7 {pname} ("Sending Trade Request to %s..."); the other gets 0x2BC1 {from_id, from_name}.
    // 0x2BC2 {to_id, answer 2} -> 0x2BDB; the inviter gets 0x2BC3 {answer}, 2 opening the scene.
    let mut t = table();
    t.b.pushed();
    let r = send(&mut t.a, 0x2BC0, &[("to_id", t.b_id)]);
    // The second address's player is "Duelist2".
    assert_eq!((r.request(), r.get("pname").and_then(Value::as_str).as_deref()), (Some(0x2BD7), Some("Duelist2")));
    let invite = one(t.b.pushed());
    assert_eq!((invite.request(), int(&invite, "from_id")), (Some(0x2BC1), Some(t.a_id)));
    send(&mut t.b, 0x2BC2, &[("to_id", t.a_id), ("answer", 2)]);
    let answer = one(t.a.pushed());
    assert_eq!((answer.request(), int(&answer, "answer")), (Some(0x2BC3), Some(2)));
    // Both scenes say ready (0x2BD5 -> 0x2BED); when both are, both get 0x2BD6 (NOTICE_START).
    one(t.a.send(req(0x2BD5)));
    assert!(t.a.pushed().is_empty());
    one(t.b.send(req(0x2BD5)));
    assert_eq!(requests(&t.a.pushed()), [0x2BD6]);
    assert_eq!(requests(&t.b.pushed()), [0x2BD6]);
}

#[test]
fn an_invitation_to_someone_away_fails() {
    // 0x2BD8 {error_code}: 106 "This player cannot receive another request." (0x296C).
    let mut t = table();
    let r = send(&mut t.a, 0x2BC0, &[("to_id", 4242)]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2BD8), Some(106)));
}

#[test]
fn both_offer_agree_and_the_cards_change_hands() {
    // An offer 0x2BC6 {main_list: big-endian u16 rows} -> 0x2BE1 ("card list send ok"); the partner gets 0x2BCF
    // {main_list} (NOTICE_OTHER), acks with 0x2BC7 -> 0x2BE3, and the offerer gets 0x2BCE and 0x2BD0. OK is 0x2BC8
    // {status 1} -> 0x2BE5, the partner gets 0x2BD1 {status}; both OK, both get 0x2BD2 (ALLREADY) and "Execute Trade?"
    // sends 0x2BC9 {answer} -> 0x2BE7. Both yes: both get 0x2BCA {error_code 0, own_status, other_status,
    // trade_result 1} ("Trade Completed") and a new trunk (9000), the collections changed for good.
    let mut t = table();
    in_scene(&mut t);
    assert_eq!(offer(&mut t.a, &[10]).request(), Some(0x2BE1));
    let other = one(t.b.pushed());
    assert_eq!(other.request(), Some(0x2BCF));
    assert_eq!(other.get("main_list"), Some(&Value::u16_be_array(&[10])));
    assert_eq!(one(t.b.send(req(0x2BC7))).request(), Some(0x2BE3));
    assert_eq!(requests(&t.a.pushed()), [0x2BCE, 0x2BD0]);
    assert_eq!(offer(&mut t.b, &[20]).request(), Some(0x2BE1));
    t.a.pushed();
    one(t.a.send(req(0x2BC7)));
    t.b.pushed();
    assert_eq!(send(&mut t.a, 0x2BC8, &[("status", 1)]).request(), Some(0x2BE5));
    let status = one(t.b.pushed());
    assert_eq!((status.request(), int(&status, "status")), (Some(0x2BD1), Some(1)));
    send(&mut t.b, 0x2BC8, &[("status", 1)]);
    assert_eq!(requests(&t.a.pushed()), [0x2BD1, 0x2BD2]);
    assert_eq!(requests(&t.b.pushed()), [0x2BD2]);
    // Yes is answer 2 (live), No 1.
    assert_eq!(send(&mut t.a, 0x2BC9, &[("answer", 2)]).request(), Some(0x2BE7));
    assert!(t.b.pushed().is_empty(), "waits for the other answer");
    assert_eq!(send(&mut t.b, 0x2BC9, &[("answer", 2)]).request(), Some(0x2BE7));
    // The first had two of row 10 and gets row 20; the second had one of row 20 and gets row 10.
    for (pushed, rows_10_20) in [(t.a.pushed(), (1, 1)), (t.b.pushed(), (1, 0))] {
        let done = pushed.iter().find(|m| m.request() == Some(0x2BCA)).expect("the result");
        assert_eq!((int(done, "error_code"), int(done, "trade_result")), (Some(0), Some(1)));
        let trunk = pushed.iter().find(|m| m.request() == Some(9000)).expect("a new trunk");
        assert_eq!((count(trunk, 10), count(trunk, 20)), rows_10_20);
    }
    let d = t.dir.lock().unwrap();
    let player = |id: u32| d.accounts().flat_map(|(_, a)| a.players.clone()).find(|p| p.player_id == id).unwrap();
    assert_eq!((player(t.a_id).cards.get(&10), player(t.a_id).cards.get(&20)), (Some(&1), Some(&1)));
    assert_eq!((player(t.b_id).cards.get(&10), player(t.b_id).cards.get(&20)), (Some(&1), None));
}

#[test]
fn an_offer_of_cards_not_owned_is_refused() {
    // 0x2BE2 {error_code}: 504 ERRORCODE_TRADE_CARD_NUM_ILLEGAL.
    let mut t = table();
    in_scene(&mut t);
    let r = offer(&mut t.a, &[10, 10, 10]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2BE2), Some(504)));
}

#[test]
fn leaving_the_scene_tells_the_partner() {
    // 0x2BCB -> 0x2BE9; the partner gets 0x2BCD ("The trade partner has cancelled."). A "No" to "Execute Trade?" ends
    // the trade unchanged (0x2BCA trade_result 0).
    let mut t = table();
    in_scene(&mut t);
    assert_eq!(one(t.a.send(req(0x2BCB))).request(), Some(0x2BE9));
    assert_eq!(requests(&t.b.pushed()), [0x2BCD]);
    let mut t = table();
    in_scene(&mut t);
    offer(&mut t.a, &[10]);
    for c in [&mut t.a, &mut t.b] {
        send(c, 0x2BC8, &[("status", 1)]);
    }
    send(&mut t.a, 0x2BC9, &[("answer", 2)]);
    t.a.pushed();
    send(&mut t.b, 0x2BC9, &[("answer", 1)]);
    let pushed = t.b.pushed();
    let done = pushed.iter().find(|m| m.request() == Some(0x2BCA)).expect("the result");
    // A non-zero error_code: the scene shows "Trade Cancelled" ($14313) and closes (0x424F71); with 0 it would stay,
    // both Ready marks on, after the server has forgotten the trade.
    assert_eq!((int(done, "trade_result"), int(done, "error_code")), (Some(0), Some(503)));
    let d = t.dir.lock().unwrap();
    let a = d.accounts().flat_map(|(_, a)| a.players.clone()).find(|p| p.player_id == t.a_id).unwrap();
    assert_eq!(a.cards.get(&10), Some(&2));
}

/// A third player, on an account of their own.
fn third(t: &Table) -> (Client, u32) {
    let addr = "10.0.0.3".parse().unwrap();
    let id = t.dir.lock().unwrap().account_for(addr).players[0].player_id;
    (t.server.join_from(&t.dir, addr), id)
}

#[test]
fn a_player_in_a_trade_cannot_start_another() {
    // The busy check looked only at the invitee: a player in a scene could invite someone else and end up in two.
    let mut t = table();
    let (mut c, c_id) = third(&t);
    in_scene(&mut t);
    let r = send(&mut t.a, 0x2BC0, &[("to_id", c_id)]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2BD8), Some(106)));
    assert!(c.pushed().is_empty(), "the third player was not bothered");
}

#[test]
fn an_invitation_accepted_after_the_inviter_started_another_trade_is_refused() {
    // Two invitations out, both accepted: the acceptance checked only the invitee's own state, so the inviter sat in
    // two trades and each end stranded a partner.
    let mut t = table();
    let (mut c, c_id) = third(&t);
    assert_eq!(send(&mut t.a, 0x2BC0, &[("to_id", t.b_id)]).request(), Some(0x2BD7));
    assert_eq!(send(&mut t.a, 0x2BC0, &[("to_id", c_id)]).request(), Some(0x2BD7));
    assert_eq!(send(&mut t.b, 0x2BC2, &[("to_id", t.a_id), ("answer", 2)]).request(), Some(0x2BDB));
    c.pushed();
    let r = send(&mut c, 0x2BC2, &[("to_id", t.a_id), ("answer", 2)]);
    assert_eq!(r.request(), Some(0x2BDC), "refused: the lender is trading already");
    assert_eq!(send(&mut c, 0x2BD5, &[]).request(), Some(0x2BEE), "no scene for the third player");
    // The inviter's own trade is the one with b, and leaving it tells b alone.
    t.b.pushed();
    assert_eq!(one(t.a.send(req(0x2BCB))).request(), Some(0x2BE9));
    assert_eq!(requests(&t.b.pushed()), [0x2BCD]);
}
