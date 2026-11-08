//! Player creation and deletion on the Select Player screen (LoginWindow.cpp, dispatcher 0x469C00). The screen shows
//! four slots by `pindex`; an empty one starts Avatar Settings and then sends 0x1F4A {pindex, pname} (builder
//! 0x4F2D50), answered by 0x2262 "my player sub success" (the client asks for the list again, 0x4F2B70) or 0x2263
//! (the text $10525, back to the avatar editor). Delete sends 0x1F4B {player_id} (0x4F2EB0): 0x2264 "unsub success"
//! (list again, DeleteRet) or 0x2265 ("Login Error!!").
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::Directory;

const ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));

fn create(c: &mut Client, pindex: u32, name: &str) -> Message {
    one(c.send(req_with(0x1F4A, &[("pindex", Value::U32(pindex)), ("pname", Value::text(name))])))
}

fn delete(c: &mut Client, player_id: u32) -> Message {
    one(c.send(req_with(0x1F4B, &[("player_id", Value::U32(player_id))])))
}

/// The list as the client reads it: (pindex, player_id, pname) per item.
fn list(c: &mut Client) -> Vec<(u32, u32, String)> {
    let r = one(c.send(req(0x1F49)));
    assert_eq!(r.request(), Some(0x2260));
    let int = |m: &Message, k| m.get(k).and_then(Value::as_u32).unwrap();
    r.items.iter().map(|p| (int(p, "pindex"), int(p, "player_id"), p.get("pname").and_then(Value::as_str).unwrap().to_string())).collect()
}

fn setup() -> (Server, Arc<Mutex<Directory>>, Client) {
    let server = Server::new();
    let directory = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let c = server.join_from(&directory, ADDR);
    (server, directory, c)
}

#[test]
fn a_player_is_made_in_a_free_slot_and_listed_with_it() {
    let (_server, _d, mut c) = setup();
    assert_eq!(create(&mut c, 2, "Kaiba").request(), Some(0x2262));
    let players = list(&mut c);
    assert_eq!(players.len(), 2);
    assert_eq!((players[1].0, players[1].2.as_str()), (2, "Kaiba"));
    // the new player can be chosen like any other, and gets the deck list (the starter deck is the world's)
    let id = players[1].1;
    assert_eq!(one(c.send(req_with(0x1F4C, &[("player_id", Value::U32(id))]))).request(), Some(0x2266));
    assert_eq!(one(c.send(req(0x1F4E))).request(), Some(0x22C8));
}

#[test]
fn a_bad_player_is_refused() {
    let (_server, _d, mut c) = setup();
    assert_eq!(create(&mut c, 1, "Kaiba").request(), Some(0x2262));
    for (pindex, name) in [(1, "Joey"), (0, "Joey"), (2, "kaiba"), (2, ""), (2, "   "), (4, "Joey"), (2, "A\nB")] {
        assert_eq!(create(&mut c, pindex, name).request(), Some(0x2263), "{pindex} {name:?}");
    }
    assert_eq!(list(&mut c).len(), 2);
}

#[test]
fn an_account_holds_four_players() {
    let (_server, _d, mut c) = setup();
    for (i, name) in ["A", "B", "C"].iter().enumerate() {
        assert_eq!(create(&mut c, i as u32 + 1, name).request(), Some(0x2262));
    }
    assert_eq!(list(&mut c).len(), 4);
    let id = list(&mut c)[3].1;
    assert_eq!(delete(&mut c, id).request(), Some(0x2264));
    assert_eq!(create(&mut c, 3, "D").request(), Some(0x2262));
    assert_eq!(create(&mut c, 3, "E").request(), Some(0x2263));
}

#[test]
fn a_deleted_player_leaves_their_slot_empty() {
    let (_server, _d, mut c) = setup();
    create(&mut c, 1, "Kaiba");
    create(&mut c, 2, "Joey");
    let kaiba = list(&mut c)[1].1;
    assert_eq!(delete(&mut c, kaiba).request(), Some(0x2264));
    let left: Vec<_> = list(&mut c).into_iter().map(|(i, _, n)| (i, n)).collect();
    assert_eq!(left, [(0, "Yugi".to_string()), (2, "Joey".to_string())]);
    // the name and the slot are free again
    assert_eq!(create(&mut c, 1, "Kaiba").request(), Some(0x2262));
}

#[test]
fn deleting_what_does_not_exist_or_the_last_player_fails() {
    let (_server, _d, mut c) = setup();
    assert_eq!(delete(&mut c, 99).request(), Some(0x2265));
    let only = list(&mut c)[0].1;
    assert_eq!(delete(&mut c, only).request(), Some(0x2265));
    assert_eq!(list(&mut c).len(), 1);
}

#[test]
fn another_account_cannot_delete_a_player() {
    let (server, d, mut c) = setup();
    create(&mut c, 1, "Kaiba");
    let kaiba = list(&mut c)[1].1;
    let mut other = server.join_from(&d, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)));
    assert_eq!(delete(&mut other, kaiba).request(), Some(0x2265));
    assert_eq!(list(&mut c).len(), 2);
}

#[test]
fn players_and_their_slots_are_kept_with_the_account() {
    let dir = std::env::temp_dir().join(format!("yo3-players-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("accounts.json");
    {
        let server = Server::new();
        let d = Arc::new(Mutex::new(Directory::open(vec!["Yugi".into()], &file).unwrap()));
        let mut c = server.join_from(&d, ADDR);
        create(&mut c, 1, "Kaiba");
        create(&mut c, 3, "Joey");
        let kaiba = list(&mut c)[1].1;
        delete(&mut c, kaiba);
    }
    let server = Server::new();
    let d = Arc::new(Mutex::new(Directory::open(vec!["Yugi".into()], &file).unwrap()));
    let mut c = server.join_from(&d, ADDR);
    let slots: Vec<_> = list(&mut c).into_iter().map(|(i, _, n)| (i, n)).collect();
    assert_eq!(slots, [(0, "Yugi".to_string()), (3, "Joey".to_string())]);
}

#[test]
fn a_second_session_sees_a_player_made_by_the_first() {
    let (server, d, mut c) = setup();
    let mut other = server.join_from(&d, ADDR);
    create(&mut c, 1, "Kaiba");
    assert_eq!(list(&mut other).len(), 2);
}

#[test]
fn a_deleted_player_leaves_the_friends_and_ignore_lists() {
    // The deleted player's id stayed in other players' friend lists (and their own), to be inherited by whoever is
    // given that id.
    let (server, _d, mut c) = setup();
    create(&mut c, 1, "Kaiba");
    let kaiba = list(&mut c)[1].1;
    {
        let mut hub = server.hub.lock().unwrap();
        hub.befriend((kaiba, "Kaiba"), (4242, "Joey"));
        assert_eq!(hub.friends_of(4242).len(), 1);
    }
    assert_eq!(delete(&mut c, kaiba).request(), Some(0x2264));
    let hub = server.hub.lock().unwrap();
    assert!(hub.friends_of(4242).is_empty());
    assert!(hub.friends_of(kaiba).is_empty());
}
