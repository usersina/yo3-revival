//! Contract of the front server, as established by reverse-engineering the client.
//! Each assertion names the client code that depends on it; most of these were crashes.
use std::sync::Arc;

use yo3_proto::{Message, Packet, Value};
use yo3_server::front::{Account, FrontSession, World};

fn world() -> Arc<World> {
    Arc::new(World::new((1..=20).collect()))
}

fn req(id: u32) -> Packet {
    Packet::Message(Message::reply(id))
}

fn req_with(id: u32, name: &str, value: u32) -> Packet {
    Packet::Message(Message::reply(id).with(name, Value::U32(value)))
}

fn one(replies: Vec<Message>) -> Message {
    assert_eq!(replies.len(), 1, "expected exactly one reply, got {replies:?}");
    replies.into_iter().next().unwrap()
}

fn fresh() -> FrontSession {
    let mut s = FrontSession::new(Account::test(), world());
    assert_eq!(one(s.handle(&Packet::Version("2010.03.23 skelton".into()))).request(), Some(0x219F));
    s
}

fn in_world() -> FrontSession {
    let mut s = fresh();
    s.handle(&req(0x1F47));
    s.handle(&req(0x1F48));
    s.handle(&req(0x1F49));
    s.handle(&req_with(0x1F4C, "player_id", 1001));
    s.handle(&req_with(0x1FA4, "player_id", 1001));
    s
}

#[test]
fn first_packet_gets_version_valid() {
    fresh();
}

#[test]
fn auth_returns_the_account() {
    let r = one(fresh().handle(&req(0x1F47)));
    assert_eq!(r.request(), Some(0x219A));
    assert_eq!(r.get("user_id"), Some(&Value::U32(1001)));
    assert_eq!(r.get("uname"), Some(&Value::text("test")));
}

#[test]
fn login_puts_the_account_in_the_first_item() {
    // LoginWindow 0x469C00 reads user_id/uname/dpoint from item[0], regulation_id from the top.
    let r = one(fresh().handle(&req(0x1F48)));
    assert_eq!(r.request(), Some(0x2201));
    assert!(r.get("regulation_id").is_some());
    assert_eq!(r.items[0].get("user_id"), Some(&Value::U32(1001)));
}

#[test]
fn player_records_carry_everything_the_loader_reads() {
    let r = one(fresh().handle(&req(0x1F49)));
    assert_eq!(r.request(), Some(0x2260));
    let p = &r.items[0];
    // avatar1 must be binary: tags 7/9 stop at the first NUL and the client crashes (0x46C750).
    assert!(matches!(p.get("avatar1"), Some(Value::Bin(b)) if b.len() == 34));
    // pbin is dereferenced by the main screen at 0x4727DE; missing => NULL => crash.
    assert!(matches!(p.get("pbin"), Some(Value::Bin(b)) if b.len() == 512));
    for name in ["pname", "pindex", "player_id", "dlv", "title", "n_win", "n_lose", "n_draw", "speech00", "speech22", "comment01"] {
        assert!(p.get(name).is_some(), "{name} missing");
    }
}

#[test]
fn selecting_a_player() {
    let mut s = fresh();
    assert_eq!(one(s.handle(&req_with(0x1F4C, "player_id", 1001))).request(), Some(0x2266));
    assert!(s.handle(&req_with(0x1F4C, "player_id", 4242)).is_empty());
}

#[test]
fn world_entry_sends_exactly_one_trunk_record() {
    // The 9000 handler copies item[0] into UserData+0x8C8 without a NULL check.
    let mut s = fresh();
    s.handle(&req_with(0x1F4C, "player_id", 1001));
    let r = one(s.handle(&req_with(0x1FA4, "player_id", 1001)));
    assert_eq!(r.request(), Some(9000));
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].tag, 3);
}

#[test]
fn regulation_list_is_present_and_big_endian() {
    // Global dispatcher 0x4F4A00 reads "regulations" for 0x2398 without a NULL check.
    let r = one(in_world().handle(&req(0x2072)));
    assert_eq!(r.request(), Some(0x2398));
    assert_eq!(r.get("regulations"), Some(&Value::u16_be_array(&(1..=20).collect::<Vec<_>>())));
}

#[test]
fn reply_27d9_carries_one_record() {
    // 0x4F5BE9 dereferences item[0] unchecked.
    let r = one(in_world().handle(&req(0x27D8)));
    assert_eq!(r.request(), Some(0x27D9));
    assert_eq!(r.items.len(), 1);
}

#[test]
fn player_refresh_matches_the_selected_player() {
    // 0x4F5FC2 compares item[0].player_id with the client's own player id.
    let r = one(in_world().handle(&req(0x2CEE)));
    assert_eq!(r.request(), Some(0x2CF3));
    assert_eq!(r.items[0].get("player_id"), Some(&Value::U32(1001)));
    assert!(matches!(r.items[0].get("pbin"), Some(Value::Bin(_))));
}

#[test]
fn main_screen_polls_are_answered() {
    let mut s = in_world();
    for (req_id, reply) in [(0x271A, 0x271B), (0x2710, 0x2711), (0x2AFB, 0x2B13), (0x2AFD, 0x2B17)] {
        assert_eq!(one(s.handle(&req(req_id))).request(), Some(reply), "{req_id:#x}");
    }
    assert_eq!(one(s.handle(&req(0x2583))).get("count"), Some(&Value::U32(0)));
}

#[test]
fn news_list_is_nested_under_a_lang_record() {
    // InfoWindow 0x458A64: item[0] {lang} whose own items are {starttime, title, body}.
    let r = one(in_world().handle(&req(0x1F41)));
    assert_eq!(r.request(), Some(0x21FC));
    assert!(r.items[0].get("lang").is_some());
    let entry = &r.items[0].items[0];
    let start = entry.get("starttime").and_then(Value::as_str).unwrap();
    let parsed = chrono::NaiveDateTime::parse_from_str(&start, "%Y-%m-%d %H:%M:%S").expect(&start);
    // The client treats server times as JST (UTC+9) and converts them to local time.
    let jst_now = chrono::Utc::now().naive_utc() + chrono::Duration::hours(9);
    assert!((jst_now - parsed).num_seconds().abs() < 60, "{start} is not JST");
    assert!(entry.get("title").is_some() && entry.get("body").is_some());
}

#[test]
fn player_search_succeeds() {
    assert_eq!(one(in_world().handle(&req_with(0x2CF0, "search_type", 1))).request(), Some(0x2CF7));
}

#[test]
fn heartbeat_and_unknown_requests_get_no_reply() {
    let mut s = in_world();
    assert!(s.handle(&req(0x2724)).is_empty());
    assert!(s.handle(&req(0x7777)).is_empty());
}
