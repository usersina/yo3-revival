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
    let r = auto_match(&mut a, 1);
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
    auto_match(&mut a, 1);
    let r = auto_match(&mut b, 1);
    assert_eq!(r.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x1799)]);
    let to_a = one(pushed_within(&mut a, Duration::from_secs(3)));
    let to_b = one(pushed_within(&mut b, Duration::from_secs(3)));
    for (push, opponent, name) in [(&to_a, 1002, "Kaiba"), (&to_b, 1001, "Yugi")] {
        assert_eq!(push.request(), Some(0x1978));
        let vs = named(push, "vsPlayerData");
        assert_eq!(vs.get("player_id").and_then(Value::as_u32), Some(opponent));
        assert_eq!(vs.get("pname").and_then(Value::as_str).as_deref(), Some(name));
        assert_eq!(named(push, "duelData").get("match_type").and_then(Value::as_u32), Some(1));
    }
    let duel_id = |m: &Message| named(m, "duelData").get("duel_id").and_then(Value::as_u32).expect("duel_id");
    assert_eq!(duel_id(&to_a), duel_id(&to_b));
}

#[test]
fn only_a_shared_kind_of_duel_is_matched() {
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 1);
    auto_match(&mut b, 3);
    assert!(pushed_within(&mut a, Duration::from_millis(1500)).is_empty());
    assert!(b.pushed().is_empty());
}

#[test]
fn a_cancelled_wait_is_left_out() {
    // Cancelling the wait in mode 2 sends 0x17A2 {event_id, join_id} (0x4A15D0); 0x17A3 lets the agent finish.
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 1);
    let r = one(a.send(req_with(0x17A2, &[("event_id", Value::U32(2)), ("join_id", Value::U32(1))])));
    assert_eq!(r.request(), Some(0x17A3));
    auto_match(&mut b, 1);
    assert!(pushed_within(&mut a, Duration::from_millis(1500)).is_empty());
}

#[test]
fn a_matched_duel_keeps_both_decks_and_announces_the_duel_server() {
    // After 0x1978 the duel runs as an NPC duel does: Select Deck (0x1F4F with the match's duel_id), ready (0x17D4),
    // then the duel server push 0x22C4 {duelData {host, port}, rule}. The duel server finds both entries by player.
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 1);
    auto_match(&mut b, 1);
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
    assert_eq!(named(data, "rule").get("match_type").and_then(Value::as_u32), Some(1));
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
    auto_match(&mut a, 1);
    assert_eq!(counts(&mut c), [0, 0, 1]);
    auto_match(&mut b, 1);
    assert_eq!(counts(&mut c), [2, 0, 0]);
}

/// Hosts a duel as Create Duel does (agent mode 0): 0x1784 {event_id 1}, then 0x17AC {event_id, join_id} with the
/// settings item (live: {match_type 1, surrender 1, level 1, party 1, password, watch 1, lang_mask 127, lp_start
/// 8000, time_turn 180, time_total 600, regulation_id 1, status 1, duel_server_id 0}); returns the duel_id.
fn host(c: &mut Client, password: &str) -> u32 {
    let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(1))])));
    let join_id = join.get("join_id").cloned().unwrap();
    let settings = [("match_type", 1), ("surrender", 1), ("level", 1), ("party", 1), ("watch", 1), ("lang_mask", 127)]
        .into_iter()
        .chain([("lp_start", 4000), ("time_turn", 180), ("time_total", 600), ("regulation_id", 1), ("status", 1), ("duel_server_id", 0)])
        .fold(Message { tag: 4, ..Message::default() }, |m, (k, v)| m.with(k, Value::U32(v)))
        .with("password", Value::text(password));
    let ask = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", join_id).item(settings);
    let r = one(c.send(yo3_proto::Packet::Message(ask)));
    assert_eq!(r.request(), Some(0x17AD));
    r.get("duel_id").and_then(Value::as_u32).expect("duel_id")
}

/// A challenger's offer (agent mode 1, 0x4A28F0): 0x1784 {event_id 1}, then 0x17B6 {event_id, join_id, duel_id,
/// password}; the reply.
fn challenge(c: &mut Client, duel_id: u32, password: &str) -> Message {
    let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(1))])));
    let join_id = join.get("join_id").cloned().unwrap();
    let offer = req_with(0x17B6, &[("event_id", Value::U32(1)), ("join_id", join_id), ("duel_id", Value::U32(duel_id))]);
    let yo3_proto::Packet::Message(m) = offer else { unreachable!() };
    let m = if password.is_empty() { m } else { m.with("password", Value::text(password)) };
    one(c.send(yo3_proto::Packet::Message(m)))
}

fn answer(c: &mut Client, duel_id: u32, answer: u32) -> Message {
    let fields = [("event_id", 1), ("join_id", 1), ("duel_id", duel_id), ("answer", answer)].map(|(k, v)| (k, Value::U32(v)));
    one(c.send(req_with(0x17CA, &fields)))
}

/// The player search entry of `player_id` as `searcher` sees it (0x2CF0 -> 0x2CF7 items).
fn found(searcher: &mut Client, player_id: u32) -> Message {
    let r = one(searcher.send(req_with(0x2CF0, &[("search_flag", Value::U32(1)), ("base_point", Value::U32(0)), ("front_num", Value::U32(100))])));
    r.items.into_iter().find(|i| i.get("player_id").and_then(Value::as_u32) == Some(player_id)).expect("listed")
}

#[test]
fn a_hosted_duel_shows_on_its_host_in_player_search() {
    // The player info window (PlayerInfoWindow) looks a player up by player search and enables its Duel button when
    // duel_status is 1 (0x4B6060; the language and level limits checked by 0x4B6510), and shows the settings from the
    // same record (requestDuelSettings 0x4B8730, requestDuelDetail 0x4B8F30). The Duel Menu counts hosted duels as
    // duel_ready ("Create Duel").
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let duel_id = host(&mut a, "");
    let entry = found(&mut b, 1001);
    let int = |k: &str| entry.get(k).and_then(Value::as_u32);
    assert_eq!((int("duel_status"), int("duel_id")), (Some(1), Some(duel_id)));
    assert_eq!((int("duel_match_type"), int("duel_password_flag"), int("duel_lp_start")), (Some(1), Some(0), Some(4000)));
    assert_eq!((int("duel_lang_mask"), int("duel_level"), int("duel_watch"), int("duel_regulation_id")), (Some(127), Some(1), Some(1), Some(1)));
    let counts = one(b.send(common::req(0x17FC)));
    assert_eq!(counts.get("duel_ready").and_then(Value::as_u32), Some(1));
    // Cancel (0x4A2500): 0x17F2 {event_id, duel_id} -> 0x17F3; no longer hosted.
    assert_eq!(one(a.send(req_with(0x17F2, &[("event_id", Value::U32(1)), ("duel_id", Value::U32(duel_id))]))).request(), Some(0x17F3));
    assert_eq!(found(&mut b, 1001).get("duel_status").and_then(Value::as_u32).unwrap_or(0), 0);
}

#[test]
fn a_challenge_reaches_the_host_and_acceptance_starts_the_duel() {
    // 0x17B7 tells the challenger the offer went ("Requesting Duel", text 0x32CC); the host's wait step (0x4A1D10) gets
    // the offer as 0x17B6 with item "playerInfo" (the challenger's record) and answers 0x17CA {event_id, join_id,
    // duel_id, answer: 2 accept, 1 decline} (-> 0x17CB; 0x17CC would fail, 0x4A2D30). The challenger gets the answer
    // as 0x17CA {answer} (0x4A28F0), then both the match 0x1978 (with the host's duel_id and LP).
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let duel_id = host(&mut a, "");
    assert_eq!(challenge(&mut b, duel_id, "").request(), Some(0x17B7));
    let offer = one(pushed_within(&mut a, Duration::from_millis(500)));
    assert_eq!(offer.request(), Some(0x17B6));
    assert_eq!(named(&offer, "playerInfo").get("player_id").and_then(Value::as_u32), Some(1002));
    assert_eq!(named(&offer, "playerInfo").get("pname").and_then(Value::as_str).as_deref(), Some("Kaiba"));
    assert_eq!(answer(&mut a, duel_id, 2).request(), Some(0x17CB));
    let to_b = pushed_within(&mut b, Duration::from_millis(200));
    assert_eq!(to_b.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x17CA)]);
    assert_eq!(to_b[0].get("answer").and_then(Value::as_u32), Some(2));
    let (to_a, to_b) = (one(pushed_within(&mut a, Duration::from_secs(3))), one(pushed_within(&mut b, Duration::from_secs(3))));
    for (push, opponent) in [(&to_a, 1002), (&to_b, 1001)] {
        assert_eq!(push.request(), Some(0x1978));
        assert_eq!(named(push, "vsPlayerData").get("player_id").and_then(Value::as_u32), Some(opponent));
        assert_eq!(named(push, "duelData").get("duel_id").and_then(Value::as_u32), Some(duel_id));
    }
    // The duel runs as an automatic match's does, with the host's starting LP.
    let deck = Message::record().with("main", Value::u16_be_array(&[100, 101, 102]));
    let pick = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("update_type", Value::U32(1))]);
    let yo3_proto::Packet::Message(pick) = pick else { unreachable!() };
    one(b.send(yo3_proto::Packet::Message(pick.item(deck))));
    let entry = server.hub.lock().unwrap().duel_entry(1002).expect("entry");
    assert_eq!((entry.duel_id, entry.opponent, entry.lp), (Some(duel_id), Some(1001), Some(4000)));
    // Hosting ends with the match.
    assert_eq!(found(&mut b, 1001).get("duel_status").and_then(Value::as_u32), Some(3));
}

#[test]
fn a_declined_challenge_leaves_the_duel_hosted() {
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let duel_id = host(&mut a, "");
    challenge(&mut b, duel_id, "");
    pushed_within(&mut a, Duration::from_millis(500));
    assert_eq!(answer(&mut a, duel_id, 1).request(), Some(0x17CB));
    let to_b = one(pushed_within(&mut b, Duration::from_millis(500)));
    assert_eq!((to_b.request(), to_b.get("answer").and_then(Value::as_u32)), (Some(0x17CA), Some(1)));
    assert_eq!(found(&mut b, 1001).get("duel_status").and_then(Value::as_u32), Some(1));
}

#[test]
fn offers_that_cant_go_through_say_why() {
    // 0x17B8 {error_code} (0x4A28F0, texts by code): 3000 "Another player is requesting a duel.", 3001 "Already
    // Duelling" (no such duel any more), 4006 "Password is Incorrect.".
    let server = Server::with_world(world());
    let (mut a, mut b, mut c) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    let code = |m: &Message| (m.request(), m.get("error_code").and_then(Value::as_u32));
    let duel_id = host(&mut a, "secret");
    assert_eq!(found(&mut c, 1001).get("duel_password_flag").and_then(Value::as_u32), Some(1));
    assert_eq!(code(&challenge(&mut b, duel_id, "wrong")), (Some(0x17B8), Some(4006)));
    assert_eq!(challenge(&mut b, duel_id, "secret").request(), Some(0x17B7));
    assert_eq!(code(&challenge(&mut c, duel_id, "secret")), (Some(0x17B8), Some(3000)));
    assert_eq!(code(&challenge(&mut c, 999, "")), (Some(0x17B8), Some(3001)));
}

#[test]
fn a_withdrawn_challenge_frees_the_host() {
    // The challenger's Cancel sends 0x17C0 {event_id, join_id, duel_id} and ends its agent; the host's wait step closes
    // the offer on 0x17C0 and waits again.
    let server = Server::with_world(world());
    let (mut a, mut b, mut c) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    let duel_id = host(&mut a, "");
    challenge(&mut b, duel_id, "");
    pushed_within(&mut a, Duration::from_millis(500));
    b.send(req_with(0x17C0, &[("event_id", Value::U32(1)), ("join_id", Value::U32(1)), ("duel_id", Value::U32(duel_id))]));
    assert_eq!(one(pushed_within(&mut a, Duration::from_millis(500))).request(), Some(0x17C0));
    assert_eq!(challenge(&mut c, duel_id, "").request(), Some(0x17B7));
}

#[test]
fn both_rule_formats_match_either() {
    // decideAuto (0x49DF50) sends the Rule Format by the table at 0x790C50: Single 1, Match 3, Both 6 (live: 3 and 6).
    // Both matches a player of either format, and plays theirs; two Both players get a Single duel.
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    auto_match(&mut a, 6);
    auto_match(&mut b, 3);
    let push = one(pushed_within(&mut a, Duration::from_secs(3)));
    assert_eq!(named(&push, "duelData").get("match_type").and_then(Value::as_u32), Some(3));
    let (mut c, mut d) = (server.join(1003, "Joey"), server.join(1004, "Mai"));
    auto_match(&mut c, 6);
    auto_match(&mut d, 6);
    let push = one(pushed_within(&mut c, Duration::from_secs(3)));
    assert_eq!(named(&push, "duelData").get("match_type").and_then(Value::as_u32), Some(1));
}

#[test]
fn a_hosted_match_mode_duel_is_a_match() {
    // Create Duel's Match Mode sends match_type 3 (live), a best of three.
    let server = Server::with_world(world());
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let join = one(a.send(req_with(0x1784, &[("event_id", Value::U32(1))])));
    let settings = Message { tag: 4, ..Message::default() }.with("match_type", Value::U32(3)).with("status", Value::U32(1));
    let ask = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", join.get("join_id").cloned().unwrap()).item(settings);
    let duel_id = one(a.send(yo3_proto::Packet::Message(ask))).get("duel_id").and_then(Value::as_u32).unwrap();
    challenge(&mut b, duel_id, "");
    pushed_within(&mut a, Duration::from_millis(500));
    answer(&mut a, duel_id, 2);
    let push = one(pushed_within(&mut a, Duration::from_secs(3)));
    assert_eq!(named(&push, "duelData").get("match_type").and_then(Value::as_u32), Some(3));
    let deck = Message::record().with("main", Value::u16_be_array(&[100, 101, 102]));
    let yo3_proto::Packet::Message(pick) = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id))]) else { unreachable!() };
    one(a.send(yo3_proto::Packet::Message(pick.item(deck))));
    assert_eq!(server.hub.lock().unwrap().duel_entry(1001).unwrap().best_of, 3);
}
