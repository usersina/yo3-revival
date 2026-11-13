//! Chat rooms and messages (ChatWindow dispatcher 0x435950), with several players online.
mod common;

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Packet, Value};

fn enter(c: &mut Client, room: u32) -> Message {
    one(c.send(req_with(0x2136, &[("room_id", Value::U32(room))])))
}

/// The real client's shape (captured): fields in one tag-10 item; room chat is chat_type 1 with
/// to_id = the room id.
fn chat_request(fields: &[(&str, Value)]) -> Packet {
    let body = Message { tag: 10, fields: fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(), items: vec![] };
    Packet::Message(Message::reply(0x2134).item(body))
}

fn say(room: u32, text: &str) -> Packet {
    chat_request(&[("chat_str", Value::text(text)), ("chat_type", Value::U32(1)), ("to_id", Value::U32(room))])
}

fn room_num(list: &Message, room: u32) -> u32 {
    let r = list.items.iter().find(|i| i.get("id") == Some(&Value::U32(room))).expect("room listed");
    r.get("num").and_then(Value::as_u32).unwrap()
}

#[test]
fn room_list_carries_what_the_client_reads() {
    // 0x2458: top-level lang; each item read by 0x434D20 (id, name, capacity, num, region, status, type).
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    let list = one(a.send(req(0x2135)));
    assert_eq!(list.request(), Some(0x2458));
    assert!(list.get("lang").is_some());
    assert!(!list.items.is_empty());
    for room in &list.items {
        for f in ["id", "name", "capacity", "num", "region", "status", "type"] {
            assert!(room.get(f).is_some(), "room field {f} missing");
        }
        assert!(room.get("id").and_then(Value::as_u32).unwrap() >= 1, "0x245A ignores room ids below 1");
    }
}

#[test]
fn entering_and_leaving_rooms() {
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    let entered = enter(&mut a, 1);
    assert_eq!(entered.request(), Some(0x245A));
    assert_eq!(entered.get("room_id"), Some(&Value::U32(1)));
    let refused = enter(&mut a, 999);
    assert_eq!(refused.request(), Some(0x245B));
    assert!(refused.get("error_code").is_some());
    assert_eq!(one(a.send(req(0x2137))).request(), Some(0x245C));
}

#[test]
fn room_occupancy_follows_players() {
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    let mut b = s.join(1002, "Kaiba");
    enter(&mut a, 1);
    assert_eq!(room_num(&one(b.send(req(0x2135))), 1), 1);
    drop(a); // disconnect
    assert_eq!(room_num(&one(b.send(req(0x2135))), 1), 0);
}

#[test]
fn room_chat_reaches_the_room_and_echoes_to_the_sender() {
    let s = Server::new();
    let (mut a, mut b, mut c) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"), s.join(1003, "Joey"));
    enter(&mut a, 1);
    enter(&mut b, 1);
    enter(&mut c, 2);
    let replies = a.send(say(1, "hello duelists"));
    assert_eq!(replies[0].request(), Some(0x2456), "send success first");
    // The client shows nothing on 0x2456; the sender sees its line through the 0x2455 echo.
    let echo = &replies[1];
    assert_eq!(echo.request(), Some(0x2455));
    let line = &echo.items[0];
    assert_eq!(line.get("from_id"), Some(&Value::U32(1001)));
    assert_eq!(line.get("pname"), Some(&Value::text("Yugi")));
    assert_eq!(line.get("chat_str"), Some(&Value::text("hello duelists")));
    assert!(line.get("chat_type").is_some());
    let got = b.pushed();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].items[0].get("chat_str"), Some(&Value::text("hello duelists")));
    assert!(c.pushed().is_empty(), "other rooms don't hear it");
}

#[test]
fn leaving_a_room_stops_its_chat() {
    let s = Server::new();
    let (mut a, mut b) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"));
    enter(&mut a, 1);
    enter(&mut b, 1);
    b.send(req(0x2137));
    a.send(say(1, "anyone?"));
    assert!(b.pushed().is_empty());
}

#[test]
fn chat_to_a_room_the_sender_is_not_in_fails() {
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    enter(&mut a, 1);
    assert_eq!(one(a.send(say(2, "wrong room"))).request(), Some(0x2457));
}

#[test]
fn whispers_by_name_go_to_one_player() {
    let s = Server::new();
    let (mut a, mut b, mut c) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"), s.join(1003, "Joey"));
    enter(&mut c, 1);
    let replies = a.send(chat_request(&[
        ("chat_str", Value::text("psst")),
        ("chat_type", Value::U32(2)),
        ("to_name", Value::text("Kaiba")),
    ]));
    assert_eq!(replies[0].request(), Some(0x2456));
    assert_eq!(b.pushed()[0].items[0].get("chat_str"), Some(&Value::text("psst")));
    assert!(c.pushed().is_empty());
}

#[test]
fn an_npcs_line_is_a_chat_push_the_duel_screens_window_shows() {
    // The duel screen's chat window shows 0x2455 lines of chat_type 0 and 1 (live, 2026-11-13; 2 and up went to other
    // tabs); ChatWindow 0x435950 colours the name of a chat_type 0 line red (colour 0xB), like the opponent's name
    // plate. The NPC's npc_id is its from_id: never a player's (1001 and up), so never drawn as the player's own line.
    let push = yo3_server::banter::chat_line(135, "Maximillion Pegasus", "I see your cards!");
    assert_eq!(push.request(), Some(0x2455));
    let line = &push.items[0];
    let text = |k| line.get(k).and_then(Value::as_str);
    assert_eq!(text("pname").as_deref(), Some("Maximillion Pegasus"));
    assert_eq!(text("chat_str").as_deref(), Some("I see your cards!"));
    assert_eq!(line.get("chat_type"), Some(&Value::U32(0)));
    assert_eq!(line.get("from_id"), Some(&Value::U32(135)));
}
