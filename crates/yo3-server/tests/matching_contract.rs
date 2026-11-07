//! Duels between players: the main screen's DUEL button (GoDuel 0x46FEB0) opens DuelMenu (0x49D070), whose automatic
//! matching (decideAuto 0x49DF50, the client's saved "autoMatching" {match_type, level}) starts DuelMatchingAgent in
//! mode 2. The agent checks the deck (0x4A0E90), asks for an entry (0x1784 {event_id 2}, 0x4A1090), then for a match
//! (0x1798, 0x4A1380), and waits (0x4A1D10) for the opponent (0x1978, read by 0x4A2D30); the duel then starts as an
//! NPC duel does (Select Deck 0x1F4F, ready 0x17D4, the duel server push 0x22C4).
mod common;

use std::time::{Duration, Instant};

use common::{Client, Server, one, req_with};
use yo3_proto::{Message, Value};
use yo3_server::front::World;

fn world() -> World {
    World { duel_server: ("10.0.0.5".into(), 4321), ..World::new((1..=20).collect()) }
}

/// 0x1784 {event_id 2} then 0x1798 {event_id, join_id, match_type, level}: the replies.
fn auto_match(c: &mut Client, match_type: u32) -> Vec<Message> {
    let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
    assert_eq!(join.request(), Some(0x1785));
    let join_id = join.get("join_id").and_then(Value::as_u32).expect("join_id");
    let fields = [
        ("event_id", Value::U32(2)),
        ("join_id", Value::U32(join_id)),
        ("match_type", Value::U32(match_type)),
        ("level", Value::U32(0)),
    ];
    c.send(req_with(0x1798, &fields))
}

/// What the client was pushed within `wait`.
fn pushed_within(c: &mut Client, wait: Duration) -> Vec<Message> {
    let start = Instant::now();
    let mut out = c.pushed();
    while out.is_empty() && start.elapsed() < wait {
        std::thread::sleep(Duration::from_millis(20));
        out = c.pushed();
    }
    out
}

/// The item named `name` of a message.
fn named<'a>(m: &'a Message, name: &str) -> &'a Message {
    m.items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some(name)).unwrap_or_else(|| panic!("no {name} in {m:?}"))
}

#[test]
fn a_player_asking_for_a_match_waits() {
    // 0x1799 (0x4A1380) reads nothing; the agent then waits (0x4A1D10, a "waiting" dialog, text 0x32D0).
    let server = Server::with_world(world());
    let mut a = server.join(1001, "Yugi");
    let r = auto_match(&mut a, 0);
    assert_eq!(r.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x1799)]);
    assert!(pushed_within(&mut a, Duration::from_millis(100)).is_empty());
}

#[test]
fn two_waiting_players_are_matched() {
    // The match push 0x1978 (0x4A2D30): item "vsPlayerData" (the opponent's record, loaded into MainGamen+0x408 like
    // the player list's records) and item "duelData" (copied into MainGamen+0x88C as the duel's rule; its duel_id is
    // kept). The second player gets it after its 0x1799 (**guess**: the agent reads one message per step).
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 0);
    let r = auto_match(&mut b, 0);
    assert_eq!(r.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x1799)]);
    let to_a = one(pushed_within(&mut a, Duration::from_secs(3)));
    let to_b = one(pushed_within(&mut b, Duration::from_secs(3)));
    for (push, opponent, name) in [(&to_a, 1002, "Kaiba"), (&to_b, 1001, "Yugi")] {
        assert_eq!(push.request(), Some(0x1978));
        let vs = named(push, "vsPlayerData");
        assert_eq!(vs.get("player_id").and_then(Value::as_u32), Some(opponent));
        assert_eq!(vs.get("pname").and_then(Value::as_str).as_deref(), Some(name));
        assert_eq!(named(push, "duelData").get("match_type").and_then(Value::as_u32), Some(0));
    }
    let duel_id = |m: &Message| named(m, "duelData").get("duel_id").and_then(Value::as_u32).expect("duel_id");
    assert_eq!(duel_id(&to_a), duel_id(&to_b));
}

#[test]
fn only_the_same_kind_of_duel_is_matched() {
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 0);
    auto_match(&mut b, 1);
    assert!(pushed_within(&mut a, Duration::from_millis(1500)).is_empty());
    assert!(b.pushed().is_empty());
}

#[test]
fn a_cancelled_wait_is_left_out() {
    // Cancelling the wait in mode 2 sends 0x17A2 {event_id, join_id} (0x4A15D0); 0x17A3 lets the agent finish.
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 0);
    let r = one(a.send(req_with(0x17A2, &[("event_id", Value::U32(2)), ("join_id", Value::U32(1))])));
    assert_eq!(r.request(), Some(0x17A3));
    auto_match(&mut b, 0);
    assert!(pushed_within(&mut a, Duration::from_millis(1500)).is_empty());
}

#[test]
fn a_matched_duel_keeps_both_decks_and_announces_the_duel_server() {
    // After 0x1978 the duel runs as an NPC duel does: Select Deck (0x1F4F with the match's duel_id), ready (0x17D4),
    // then the duel server push 0x22C4 {duelData {host, port}, rule}. The duel server finds both entries by player.
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 0);
    auto_match(&mut b, 0);
    let push = one(pushed_within(&mut a, Duration::from_secs(3)));
    let duel_id = named(&push, "duelData").get("duel_id").and_then(Value::as_u32).expect("duel_id");
    pushed_within(&mut b, Duration::from_secs(3));
    let deck = Message::record().with("main", Value::u16_be_array(&[100, 101, 102]));
    for c in [&mut a, &mut b] {
        let pick = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))]);
        let yo3_proto::Packet::Message(m) = pick else { unreachable!() };
        assert_eq!(one(c.send(yo3_proto::Packet::Message(m.item(deck.clone())))).request(), Some(0x22CA));
    }
    let hub = server.hub.lock().unwrap();
    let (ea, eb) = (hub.duel_entry(1001).expect("entry"), hub.duel_entry(1002).expect("entry"));
    assert_eq!(ea.npc_id, None);
    assert_eq!((ea.duel_id, eb.duel_id), (Some(duel_id), Some(duel_id)));
    assert_eq!((ea.opponent, eb.opponent), (Some(1002), Some(1001)));
    assert_eq!(ea.deck.main, [100, 101, 102]);
    drop(hub);
    let r = a.send(req_with(0x17D4, &[("duel_id", Value::U32(duel_id))]));
    assert_eq!(r.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x17D5), Some(0x22C4)]);
    let data = named(&r[1], "duelData");
    assert_eq!(data.get("host").and_then(Value::as_str).as_deref(), Some("10.0.0.5"));
    assert_eq!(named(data, "rule").get("match_type").and_then(Value::as_u32), Some(0));
}

#[test]
fn the_duel_menu_counts_the_players() {
    // DuelMenu (0x49D310) opens with a bare 0x17FC; 0x17FD gives duel_playing, duel_ready and duel_auto (menu +0x58,
    // +0x5C, +0x60), 0x17FE fails. Hosted duels (duel_ready) aren't offered yet (**guess**: the counts are players).
    let server = Server::with_world(world());
    let (mut a, mut b, mut c) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    let counts = |c: &mut Client| {
        let r = one(c.send(common::req(0x17FC)));
        assert_eq!(r.request(), Some(0x17FD));
        ["duel_playing", "duel_ready", "duel_auto"].map(|k| r.get(k).and_then(Value::as_u32).expect(k))
    };
    assert_eq!(counts(&mut c), [0, 0, 0]);
    auto_match(&mut a, 0);
    assert_eq!(counts(&mut c), [0, 0, 1]);
    auto_match(&mut b, 0);
    assert_eq!(counts(&mut c), [2, 0, 0]);
}
