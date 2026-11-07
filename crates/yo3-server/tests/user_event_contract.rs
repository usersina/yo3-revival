//! User events: Satellite Plaza's list and Create (UserEventScene 0x4D9BB0), the lobby (TournamentScene in user mode,
//! 0x4CBD70), and the event's duels, which the server pairs round by round and starts with the match push 0x1978.
mod common;

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn text(m: &Message, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or_default()
}

fn requests(pushed: &[Message]) -> Vec<u32> {
    pushed.iter().filter_map(Message::request).collect()
}

/// Create (0x339B -> 0x3400 {create_status 0}, then the form 0x3390 -> 0x33F4 {event_id}) and the host's entry
/// (0x3397 {event_id} -> 0x33FA); the event's id.
fn create(c: &mut Client, name: &str) -> u32 {
    let r = one(c.send(req(0x339B)));
    assert_eq!((r.request(), int(&r, "create_status")), (Some(0x3400), Some(0)));
    let form = [
        ("event_name", Value::text(name)),
        ("comment", Value::text("Friday cup")),
        ("entry_max", Value::U32(4)),
        ("player_limit", Value::U32(1)),
        ("password1", Value::text("")),
        ("match_type", Value::U32(1)),
        ("regulation_id", Value::U32(1)),
        ("duel_lp_start", Value::U32(8000)),
        ("duel_time_turn", Value::U32(180)),
        ("duel_time_total", Value::U32(600)),
    ];
    let r = one(c.send(req_with(0x3390, &form)));
    assert_eq!(r.request(), Some(0x33F4));
    let event_id = int(&r, "event_id").expect("event_id");
    assert_eq!(one(c.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33FA));
    event_id
}

fn search(c: &mut Client) -> Message {
    let fields = [("search_flag", Value::U32(1)), ("base_point", Value::U32(0)), ("front_num", Value::U32(30))];
    one(c.send(req_with(0x1789, &fields)))
}

#[test]
fn a_created_event_is_listed_with_its_host_entered() {
    // 0x339A -> 0x33FE {remain, event_id}: the player's own event (kept while remain isn't 0; it turns Create off).
    // The list 0x1789 -> 0x177B {result_number} with rows {event_id, event_name, status 12 open, join_flag, password1,
    // prize_type, entry_max, entry_num, match_type, regulation_id, ...} (0x4D66A0, the detail view 0x4D75F0).
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    assert_eq!(int(&one(yugi.send(req(0x339A))), "remain"), Some(0));
    let event_id = create(&mut yugi, "Duelist Cup");
    let own = one(yugi.send(req(0x339A)));
    assert_eq!((int(&own, "remain"), int(&own, "event_id")), (Some(1), Some(event_id)));
    let list = search(&mut server.join(1002, "Kaiba"));
    assert_eq!(int(&list, "result_number"), Some(1));
    let row = &list.items[0];
    assert_eq!((int(row, "event_id"), text(row, "event_name"), int(row, "status")), (Some(event_id), "Duelist Cup".into(), Some(12)));
    assert_eq!((int(row, "entry_num"), int(row, "entry_max"), int(row, "match_type")), (Some(1), Some(4), Some(1)));
    // join_flag 0: open to all (live: 1 crossed the row out and the details offered no Participate).
    assert_eq!(int(row, "join_flag"), Some(0));
}

#[test]
fn the_lobby_names_the_event_and_its_entrants() {
    // TournamentScene in user mode asks 0x177F {event_id} -> 0x1780 item[0] {official 0, event_name, player_id: the
    // host, which it compares with its own}, and 0x32CA {event_id, start_index} -> 0x32FD {total_num, items
    // {player_id}}.
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    let event_id = create(&mut yugi, "Duelist Cup");
    let info = one(yugi.send(req_with(0x177F, &[("event_id", Value::U32(event_id))])));
    assert_eq!(info.request(), Some(0x1780));
    let item = &info.items[0];
    assert_eq!((int(item, "official"), text(item, "event_name"), int(item, "player_id")), (Some(0), "Duelist Cup".into(), Some(1001)));
    let entries = one(yugi.send(req_with(0x32CA, &[("event_id", Value::U32(event_id)), ("start_index", Value::U32(0))])));
    assert_eq!((entries.request(), int(&entries, "total_num")), (Some(0x32FD), Some(1)));
    assert_eq!(int(&entries.items[0], "player_id"), Some(1001));
    // The lobby asks with the event_id the global dispatcher kept from 0x1785 (live: 0 until it was echoed), then for
    // its state (0x4CC5B0): 0x32CC -> 0x3301 {timer}, 0x32CD -> 0x3303 {alive_num, entry_num, watch_num}, 0x32CE ->
    // 0x3305 {phase_block1-3, remain}, 0x32C9 -> 0x32FB {event_id, status, phase, round, total_round, winner...}.
    let joined = one(yugi.send(req_with(0x1784, &[("event_id", Value::U32(event_id))])));
    assert_eq!(int(&joined, "event_id"), Some(event_id));
    let ask = |c: &mut Client, id: u32| one(c.send(req_with(id, &[("event_id", Value::U32(event_id))])));
    assert_eq!(ask(&mut yugi, 0x32CC).request(), Some(0x3301));
    let counts = ask(&mut yugi, 0x32CD);
    assert_eq!((counts.request(), int(&counts, "entry_num")), (Some(0x3303), Some(1)));
    assert_eq!(ask(&mut yugi, 0x32CE).request(), Some(0x3305));
    // 0x32D5 -> 0x3309 {map_data}: 0x49 raw bytes the lobby copies unchecked (0x4D0F40; live, a crash without them).
    let map = ask(&mut yugi, 0x32D5);
    assert!(matches!(map.get("map_data"), Some(Value::Bin(b)) if b.len() == 0x49));
    let state = ask(&mut yugi, 0x32C9);
    // Its status is the lobby's state (+0x2F8), not the list's: above 1 the host's Start and Cancel go (0x4CE670;
    // live, 12 left only Close in the Event Menu).
    assert_eq!((state.request(), int(&state, "event_id"), int(&state, "status")), (Some(0x32FB), Some(event_id), Some(0)));
}

#[test]
fn players_enter_and_leave_and_the_lobby_hears_of_it() {
    // An entry (0x3397) or a leave (0x178E {event_id} -> 0x178F) reaches the others as 0x3399 {entry_name, entry_type:
    // 1 joined "… has entered", else left}.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    yugi.pushed();
    assert_eq!(one(kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33FA));
    let joined = one(yugi.pushed());
    assert_eq!((joined.request(), text(&joined, "entry_name"), int(&joined, "entry_type")), (Some(0x3399), "Kaiba".into(), Some(1)));
    assert_eq!(one(kaiba.send(req_with(0x178E, &[("event_id", Value::U32(event_id))]))).request(), Some(0x178F));
    let left = one(yugi.pushed());
    assert_eq!((left.request(), int(&left, "entry_type")), (Some(0x3399), Some(2)));
    assert_eq!(int(&search(&mut kaiba).items[0], "entry_num"), Some(1));
}

#[test]
fn starting_pairs_the_entrants_and_the_winners_go_on() {
    // The host's start 0x3395 {event_id} -> 0x33F8; everyone gets 0x3396 ("The event has started"), and each pair the
    // match push 0x1978 {vsPlayerData, duelData {duel_id, the event's rule}}, which TournamentScene takes like the
    // matching agent (0x4CC60C) to open the duel. A duel's winner goes on; the last one standing ends the event.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    yugi.pushed();
    assert_eq!(one(yugi.send(req_with(0x3395, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33F8));
    let (to_yugi, to_kaiba) = (yugi.pushed(), kaiba.pushed());
    assert_eq!(requests(&to_yugi), [0x3396, 0x1978]);
    assert_eq!(requests(&to_kaiba), [0x3396, 0x1978]);
    let data = to_kaiba[1].items.iter().find(|i| text(i, "name") == "duelData").expect("duelData");
    let duel_id = int(data, "duel_id").expect("duel_id");
    assert_eq!((int(data, "match_type"), int(data, "lp_start")), (Some(1), Some(8000)));
    assert_eq!(int(&search(&mut kaiba).items[0], "status"), Some(13));
    // Live: both picked their decks, Yugi won the duel, and Next took both back to the lobby.
    server.hub.lock().unwrap().end_match(duel_id, Some(1002));
    let state = one(kaiba.send(req_with(0x32C9, &[("event_id", Value::U32(event_id))])));
    assert_eq!((int(&state, "status"), int(&state, "winner"), text(&state, "winner_name")), (Some(3), Some(1002), "Kaiba".into()));
    // A finished event leaves the list, and its host may create another.
    assert_eq!(int(&search(&mut kaiba), "result_number"), Some(0));
    assert_eq!(int(&one(yugi.send(req(0x339A))), "remain"), Some(0));
}

#[test]
fn the_host_cancels_the_event() {
    // 0x3393 {event_id} -> 0x33F6; the others get 0x3394 ("The event has been cancelled", the lobby closes).
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    assert_eq!(one(yugi.send(req_with(0x3393, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33F6));
    assert_eq!(requests(&kaiba.pushed()), [0x3394]);
    assert_eq!(int(&search(&mut kaiba), "result_number"), Some(0));
}
