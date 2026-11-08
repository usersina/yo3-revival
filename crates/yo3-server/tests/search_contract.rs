//! Player search (0x2CF0 -> 0x2CF7): InfoWindow 0x458410 reads result_number and each item
//! through 0x475A20 (player mode); 0x4B6AF0 looks one player up by id.
mod common;

use common::{Client, Server, one, req_with};
use yo3_proto::{Message, Value};
use yo3_server::front::Player;

/// The default search the client sends (captured).
fn everyone() -> Vec<(&'static str, Value)> {
    vec![
        ("search_flag", Value::U32(1)),
        ("search_type", Value::U32(1)),
        ("base_point", Value::U32(0)),
        ("front_num", Value::U32(100)),
        ("back_num", Value::U32(0)),
    ]
}

fn search(c: &mut Client, fields: &[(&str, Value)]) -> Message {
    let r = one(c.send(req_with(0x2CF0, fields)));
    assert_eq!(r.request(), Some(0x2CF7));
    r
}

fn names(r: &Message) -> Vec<String> {
    let mut v: Vec<String> = r.items.iter().map(|i| i.get("pname").and_then(Value::as_str).unwrap()).collect();
    v.sort();
    v
}

fn leveled(player_id: u32, name: &str, level: u32) -> Player {
    Player { level, ..Player::new(player_id, name) }
}

#[test]
fn search_lists_the_players_online_with_what_the_reader_needs() {
    let s = Server::new();
    let (mut a, _b, _c) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"), s.join(1003, "Joey"));
    let r = search(&mut a, &everyone());
    // 0x2CF7: result_number is the total (pages of 100); 0x475A20 reads player_id (0 is skipped),
    // pname and dlv. The handler hides the searcher's own entry itself.
    assert_eq!(names(&r), ["Joey", "Kaiba", "Yugi"]);
    assert_eq!(r.get("result_number"), Some(&Value::U32(3)));
    for item in &r.items {
        assert!(item.get("player_id").and_then(Value::as_u32).unwrap() != 0);
        assert!(item.get("dlv").is_some());
    }
}

#[test]
fn players_who_left_are_not_listed() {
    let s = Server::new();
    let (mut a, b) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"));
    drop(b);
    assert_eq!(names(&search(&mut a, &everyone())), ["Yugi"]);
}

#[test]
fn search_by_name_matches_part_of_it() {
    // search_flag 0x2 carries pname (InfoWindow 0x45F390).
    let s = Server::new();
    let (mut a, _b, _c) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"), s.join(1003, "Joey"));
    let r = search(&mut a, &[("search_flag", Value::U32(2)), ("pname", Value::text("kai"))]);
    assert_eq!(names(&r), ["Kaiba"]);
    assert_eq!(r.get("result_number"), Some(&Value::U32(1)));
}

#[test]
fn search_by_level_range() {
    // search_flag 0x10 carries min_level and max_level.
    let s = Server::new();
    let mut a = s.join_as(leveled(1001, "Yugi", 1));
    let (_b, _c) = (s.join_as(leveled(1002, "Kaiba", 5)), s.join_as(leveled(1003, "Joey", 9)));
    let range = [("search_flag", Value::U32(0x10)), ("min_level", Value::U32(4)), ("max_level", Value::U32(9))];
    assert_eq!(names(&search(&mut a, &range)), ["Joey", "Kaiba"]);
}

#[test]
fn lookup_by_player_id_puts_that_player_first() {
    // 0x4B6AF0: {search_flag 0x40000, player_id, search_type 0}; it reads item[0].
    let s = Server::new();
    let (mut a, _b, _c) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"), s.join(1003, "Joey"));
    let lookup = [("search_flag", Value::U32(0x40000)), ("player_id", Value::U32(1003)), ("search_type", Value::U32(0))];
    let r = search(&mut a, &lookup);
    assert_eq!(names(&r), ["Joey"]);
    assert_eq!(r.items[0].get("player_id"), Some(&Value::U32(1003)));
}

#[test]
fn pages_skip_base_point_and_hold_at_most_front_num() {
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    let others: Vec<Client> = (1002..1006).map(|id| s.join(id, &format!("p{id}"))).collect();
    let page = [("search_flag", Value::U32(1)), ("base_point", Value::U32(1)), ("front_num", Value::U32(2))];
    let r = search(&mut a, &page);
    assert_eq!(r.items.len(), 2);
    assert_eq!(r.get("result_number"), Some(&Value::U32(1 + others.len() as u32)), "the total, not the page");
}

#[test]
fn search_by_language_matches_the_clients_language() {
    // search_flag 0x4: `lang` is a mask of languages, bit i language i (J, E, G, F, I, S, K, C; 0x74E863). Each client
    // gives its own in the login 0x1F48 {lang} (live: 1, English).
    let s = Server::new();
    let (mut joey, _yugi, mut kaiba) = (s.join(1003, "Joey"), s.join(1001, "Yugi"), s.join(1002, "Kaiba"));
    kaiba.send(req_with(0x1F48, &[("lang", Value::U32(0))]));
    let mut english = everyone();
    english[0] = ("search_flag", Value::U32(0x4));
    english.push(("lang", Value::U32(1 << 1)));
    assert_eq!(names(&search(&mut joey, &english)), ["Joey", "Yugi"]);
}

#[test]
fn search_by_status_finds_hosts_and_duellists() {
    // The status bits (Detailed Search 0x45F84A, the preset tabs): 0x1000 a duel in progress, 0x800 waiting for a
    // duel of any rule, 0x200 a Single Mode one, 0x400 a Match Mode one (**guess** from the UI texts; the "waiting"
    // tab sends 0xE00). The manner level everyone has without ratings is 2 (0x100 asks for levels as bits).
    let s = Server::new();
    let (mut joey, mut kaiba) = (s.join(1003, "Joey"), s.join(1002, "Kaiba"));
    let join = one(kaiba.send(req_with(0x1784, &[("event_id", Value::U32(1))])));
    // Create Duel's settings (agent mode 0) come with a status.
    let settings = Message { tag: 4, ..Message::default() }.with("match_type", Value::U32(1)).with("status", Value::U32(1)).with("password", Value::text(""));
    let ask = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", join.get("join_id").cloned().unwrap()).item(settings);
    assert_eq!(one(kaiba.send(yo3_proto::Packet::Message(ask))).request(), Some(0x17AD));
    let with_flags = |flags: u32| {
        let mut f = everyone();
        f[0] = ("search_flag", Value::U32(flags));
        f
    };
    assert_eq!(names(&search(&mut joey, &with_flags(0xE00))), ["Kaiba"]);
    assert_eq!(names(&search(&mut joey, &with_flags(0x200))), ["Kaiba"]);
    assert!(names(&search(&mut joey, &with_flags(0x400))).is_empty(), "not a match");
    assert!(names(&search(&mut joey, &with_flags(0x1000))).is_empty(), "nobody duels");
    let mut manner = with_flags(0x100);
    manner.push(("eval_level", Value::U32(1 << 2)));
    assert_eq!(names(&search(&mut joey, &manner)), ["Joey", "Kaiba"]);
    manner[5] = ("eval_level", Value::U32(1 << 4));
    assert!(names(&search(&mut joey, &manner)).is_empty());
    let entry = search(&mut joey, &everyone()).items.remove(0);
    assert_eq!(entry.get("evallevel"), Some(&Value::U32(2)));
}

#[test]
fn the_chat_room_tab_lists_the_players_in_the_searchers_room() {
    // The preset tab's search_flag 0x20000 (live): the players in the searcher's chat room.
    let s = Server::new();
    let (mut yugi, mut kaiba, _joey) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"), s.join(1003, "Joey"));
    for c in [&mut yugi, &mut kaiba] {
        c.send(req_with(0x2136, &[("room_id", Value::U32(1))]));
    }
    let mut room = everyone();
    room[0] = ("search_flag", Value::U32(0x20000));
    assert_eq!(names(&search(&mut yugi, &room)), ["Kaiba", "Yugi"]);
}

#[test]
fn a_huge_language_number_does_not_overflow_the_language_search() {
    // `lang >> o.lang` panics in a debug build once a client gives a language of 32 or more.
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    let mut b = s.join(1002, "Kaiba");
    b.send(req_with(0x1F48, &[("lang", Value::U32(200))]));
    let mut by_language = everyone();
    by_language[0] = ("search_flag", Value::U32(1 | 4));
    by_language.push(("lang", Value::U32(0xFFFF_FFFF)));
    assert!(!names(&search(&mut a, &by_language)).contains(&"Kaiba".to_string()));
}
