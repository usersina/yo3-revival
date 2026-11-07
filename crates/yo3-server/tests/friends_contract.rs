//! Friends and the ignore list: PlayerInfoWindow's buttons start the community agent (0x4A4EE0), whose requests the
//! front answers and relays to the other player; the global dispatcher keeps both lists (slots 6 and 7).
mod common;

use common::{Client, Server, one, req_with};
use yo3_proto::{Message, Value};

fn send(c: &mut Client, id: u32, fields: &[(&str, u32)]) -> Message {
    let fields: Vec<(&str, Value)> = fields.iter().map(|(k, v)| (*k, Value::U32(*v))).collect();
    one(c.send(req_with(id, &fields)))
}

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn text(m: &Message, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or_default()
}

/// The friend and ignore lists a login gets after the trunk (0x1FA4 -> 9000, 0x251C, 0x252A, ...), as (id, name,
/// login_flag).
fn lists(c: &mut Client, player_id: u32) -> [Vec<(u32, String, u32)>; 2] {
    let replies = c.send(req_with(0x1FA4, &[("player_id", Value::U32(player_id))]));
    let list = |id: u32, prefix: &str| -> Vec<(u32, String, u32)> {
        let m = replies.iter().find(|m| m.request() == Some(id)).expect("list");
        let row = |i: &Message| {
            (int(i, &format!("{prefix}_id")).unwrap(), text(i, &format!("{prefix}_name")), int(i, "login_flag").unwrap())
        };
        m.items.iter().map(row).collect()
    };
    [list(0x251C, "friend"), list(0x252A, "black")]
}

/// Yugi (1001) asks Kaiba (1002) to be friends; Kaiba gets the push.
fn asked() -> (Server, Client, Client) {
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    kaiba.pushed();
    assert_eq!(send(&mut yugi, 0x24B9, &[("to_id", 1002)]).request(), Some(0x251E));
    (server, yugi, kaiba)
}

#[test]
fn a_friend_request_reaches_the_other_player() {
    // 0x24B9 {to_id} (0x4A50D0) -> 0x251E; the other player gets 0x24BA {from_id, from_name}, which the dispatcher keeps
    // in slot 24 with flag 0x100: the bell's notification, whose dialog (0x4A5B70) names from_name.
    let (_server, _yugi, mut kaiba) = asked();
    let push = one(kaiba.pushed());
    assert_eq!(push.request(), Some(0x24BA));
    assert_eq!((int(&push, "from_id"), text(&push, "from_name")), (Some(1001), "Yugi".to_string()));
}

#[test]
fn accepting_makes_them_friends_on_both_sides() {
    // 0x24BB {to_id, answer 2} -> 0x2522 {answer, friend_id, friend_name}, and the requester's waiting agent gets
    // 0x24BC with the same fields: with answer 2 the dispatcher adds the friend to each list (0x4F55AA). Their next
    // logins list each other in 0x251C, online (login_flag 1).
    let (_server, mut yugi, mut kaiba) = asked();
    kaiba.pushed();
    let reply = send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 2)]);
    assert_eq!(reply.request(), Some(0x2522));
    assert_eq!((int(&reply, "answer"), int(&reply, "friend_id"), text(&reply, "friend_name")), (Some(2), Some(1001), "Yugi".into()));
    let push = one(yugi.pushed());
    assert_eq!(push.request(), Some(0x24BC));
    assert_eq!((int(&push, "answer"), int(&push, "friend_id"), text(&push, "friend_name")), (Some(2), Some(1002), "Kaiba".into()));
    assert_eq!(lists(&mut yugi, 1001)[0], [(1002, "Kaiba".to_string(), 1)]);
    assert_eq!(lists(&mut kaiba, 1002)[0], [(1001, "Yugi".to_string(), 1)]);
}

#[test]
fn declining_makes_no_friends() {
    let (_server, mut yugi, mut kaiba) = asked();
    kaiba.pushed();
    assert_eq!(int(&send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 1)]), "answer"), Some(1));
    assert_eq!(int(&one(yugi.pushed()), "answer"), Some(1));
    assert_eq!(lists(&mut yugi, 1001)[0], []);
}

#[test]
fn a_request_needs_the_other_player_online_and_free() {
    // 0x251F {error_code}: 112 " The opponent has left the lobby." (not online), 106 "This player cannot receive
    // another request." (one is already waiting for them) (0x4A522E).
    let (server, mut yugi, _kaiba) = asked();
    assert_eq!(int(&send(&mut yugi, 0x24B9, &[("to_id", 1003)]), "error_code"), Some(112));
    let mut joey = server.join(1003, "Joey");
    let refused = send(&mut joey, 0x24B9, &[("to_id", 1002)]);
    assert_eq!((refused.request(), int(&refused, "error_code")), (Some(0x251F), Some(106)));
}

#[test]
fn a_withdrawn_request_closes_the_other_players_dialog() {
    // The waiting dialog's button sends a bare 0x24BD -> 0x2524; the other player's answer dialog gets 0x24BE (the
    // request was withdrawn), and an answer after that finds nothing: 0x2523.
    let (_server, mut yugi, mut kaiba) = asked();
    kaiba.pushed();
    assert_eq!(send(&mut yugi, 0x24BD, &[]).request(), Some(0x2524));
    assert_eq!(one(kaiba.pushed()).request(), Some(0x24BE));
    assert_eq!(send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 2)]).request(), Some(0x2523));
}

#[test]
fn removing_a_friend_removes_it_on_both_sides() {
    // 0x24BF {to_id} (0x4A5880) -> 0x2528 {to_id}: the dispatcher drops that friend (0x4F57CE); the other player gets
    // 0x2528 {to_id: the remover} for theirs.
    let (_server, mut yugi, mut kaiba) = asked();
    send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 2)]);
    yugi.pushed();
    kaiba.pushed();
    let reply = send(&mut yugi, 0x24BF, &[("to_id", 1002)]);
    assert_eq!((reply.request(), int(&reply, "to_id")), (Some(0x2528), Some(1002)));
    let push = one(kaiba.pushed());
    assert_eq!((push.request(), int(&push, "to_id")), (Some(0x2528), Some(1001)));
    assert_eq!(lists(&mut yugi, 1001)[0], []);
    assert_eq!(lists(&mut kaiba, 1002)[0], []);
}

#[test]
fn friends_see_each_other_come_and_go() {
    // When a player logs in or out, their online friends get their friend list again (0x251C, which the dispatcher
    // stores in slot 6 and the left panel redraws) with the new login_flag.
    let (server, mut yugi, mut kaiba) = asked();
    send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 2)]);
    yugi.pushed();
    drop(kaiba);
    let list = one(yugi.pushed());
    assert_eq!(list.request(), Some(0x251C));
    assert_eq!((int(&list.items[0], "friend_id"), int(&list.items[0], "login_flag")), (Some(1002), Some(0)));
    let _kaiba = server.join(1002, "Kaiba");
    let list = one(yugi.pushed());
    assert_eq!((int(&list.items[0], "friend_id"), int(&list.items[0], "login_flag")), (Some(1002), Some(1)));
}

#[test]
fn the_ignore_list_takes_and_drops_players() {
    // 0x24C2 {to_id} -> 0x252C {to_id, to_name}, which the dispatcher adds as {black_id, black_name, login_flag}
    // (0x4F58DD); 0x24C3 {to_id} -> 0x252E {to_id} drops it (0x4F5A54). The login list is 0x252A.
    let server = Server::new();
    let (mut yugi, _kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let added = send(&mut yugi, 0x24C2, &[("to_id", 1002)]);
    assert_eq!((added.request(), int(&added, "to_id"), text(&added, "to_name")), (Some(0x252C), Some(1002), "Kaiba".into()));
    assert_eq!(lists(&mut yugi, 1001)[1], [(1002, "Kaiba".to_string(), 1)]);
    let dropped = send(&mut yugi, 0x24C3, &[("to_id", 1002)]);
    assert_eq!((dropped.request(), int(&dropped, "to_id")), (Some(0x252E), Some(1002)));
    assert_eq!(lists(&mut yugi, 1001)[1], []);
}

#[test]
fn friends_are_kept_across_restarts() {
    let dir = std::env::temp_dir().join(format!("yo3-friends-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("friends.json");
    let mut hub = yo3_server::front::Hub::default();
    hub.keep_social(&path);
    hub.befriend((1001, "Yugi"), (1002, "Kaiba"));
    let mut hub = yo3_server::front::Hub::default();
    hub.keep_social(&path);
    assert_eq!(hub.friends_of(1001), [(1002, "Kaiba".to_string())]);
    assert_eq!(hub.friends_of(1002), [(1001, "Yugi".to_string())]);
}

#[test]
fn the_left_panels_friend_and_ignore_views_ask_for_their_lists() {
    // Main Menu's Friend List puts InfoWindow in its list view (+0xCE, 0x459690) and sends a bare 0x24B8 (0x4F2FB0);
    // it draws 0x251C's items by friend_id (0x458410). The ignore list view sends 0x24C1 (0x4F3090) for 0x252A.
    let (_server, mut yugi, mut kaiba) = asked();
    send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 2)]);
    send(&mut yugi, 0x24C2, &[("to_id", 1002)]);
    let friends = send(&mut yugi, 0x24B8, &[]);
    assert_eq!((friends.request(), int(&friends.items[0], "friend_id")), (Some(0x251C), Some(1002)));
    let ignored = send(&mut yugi, 0x24C1, &[]);
    assert_eq!((ignored.request(), int(&ignored.items[0], "black_id")), (Some(0x252A), Some(1002)));
}

#[test]
fn the_online_friends_tab_lists_only_friends() {
    // The left panel's preset tabs search with a community bit (0x45F390): 0x40 "Display online friends.", 0x80 the
    // ignore list, 0x20 the team (no teams yet: nobody). The searcher may be listed too (the handler skips it).
    let (server, mut yugi, mut kaiba) = asked();
    send(&mut kaiba, 0x24BB, &[("to_id", 1001), ("answer", 2)]);
    let _joey = server.join(1003, "Joey");
    send(&mut yugi, 0x24C2, &[("to_id", 1003)]);
    let listed = |c: &mut Client, flag: u32| -> Vec<u32> {
        let r = send(c, 0x2CF0, &[("search_flag", flag), ("search_type", 1), ("base_point", 0), ("front_num", 100)]);
        r.items.iter().filter_map(|i| int(i, "player_id")).filter(|&id| id != 1001).collect()
    };
    assert_eq!(listed(&mut yugi, 0x40), [1002], "friends");
    assert_eq!(listed(&mut yugi, 0x80), [1003], "ignored");
    assert_eq!(listed(&mut yugi, 0x20), Vec::<u32>::new(), "team");
    assert_eq!(listed(&mut yugi, 0x1), [1002, 1003], "everyone");
}
