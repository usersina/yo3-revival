//! Character settings (CharMake, the map's GoCharMake): Confirm sends the player update 0x2CEF (built by 0x51C960)
//! {avatar1 (binary, BE u16 appearance slots), speech00-speech22, comment00-comment01 (text), title}; 0x2CF5 is
//! "avatar update success", 0x2CF6 the error (handler 0x42AC70). The client then reads its player again (0x2CEE ->
//! 0x2CF3), so the saved settings must come back in the player record.
mod common;

use std::net::{IpAddr, Ipv4Addr};

use common::{Server, one, req};
use yo3_proto::{Message, Packet, Value};
use yo3_server::accounts::Directory;

/// The update as Confirm sends it: an appearance, the first speech line, a comment and a title.
fn update(avatar: &[u8]) -> Packet {
    let mut m = Message::reply(0x2CEF).with("avatar1", Value::Bin(avatar.to_vec()));
    for i in 0..23 {
        m = m.with(&format!("speech{i:02}"), Value::text(if i == 0 { "Let's duel!" } else { "" }));
    }
    for i in 0..2 {
        m = m.with(&format!("comment{i:02}"), Value::text(if i == 1 { "From the Dueling Network" } else { "" }));
    }
    Packet::Message(m.with("title", Value::U32(3)))
}

#[test]
fn the_character_settings_come_back_in_the_player_record() {
    let mut c = Server::new().join(1001, "Yugi");
    let avatar: Vec<u8> = (0..82).collect();
    assert_eq!(one(c.send(update(&avatar))).request(), Some(0x2CF5));
    let r = one(c.send(req(0x2CEE)));
    assert_eq!(r.request(), Some(0x2CF3));
    let p = &r.items[0];
    assert_eq!(p.get("avatar1").and_then(Value::as_bytes), Some(avatar.as_slice()));
    assert_eq!(p.get("speech00").and_then(Value::as_str).as_deref(), Some("Let's duel!"));
    assert_eq!(p.get("comment01").and_then(Value::as_str).as_deref(), Some("From the Dueling Network"));
    assert_eq!(p.get("title").and_then(Value::as_u32), Some(3));
}

#[test]
fn a_new_look_is_what_others_see_at_once() {
    // The hub's copy of an online player's record (an opponent's vsPlayerData, a user event's bracket, a duel log's
    // side, what Pegasus sees) was made at login and missed a later Confirm.
    let s = Server::new();
    let mut c = s.join(1001, "Yugi");
    let avatar: Vec<u8> = (0..82).collect();
    assert_eq!(one(c.send(update(&avatar))).request(), Some(0x2CF5));
    assert_eq!(s.hub.lock().unwrap().avatar_of(1001), Some(avatar));
}

#[test]
fn every_player_owns_the_free_avatar_parts_and_titles() {
    // Avatar Settings lists only the parts and titles whose pbin bit is set in the player's own record (and 0x433AC0
    // checks the worn ones), so without the free ones (World::free_items) every part list but the colours is empty.
    // The player list (0x1F49 -> 0x2260) and the refresh (0x2CEE -> 0x2CF3) both carry them.
    let world = yo3_server::front::World { free_items: vec![1, 224, 1024], ..yo3_server::front::World::new((1..=20).collect()) };
    let mut c = Server::with_world(world).join(1001, "Yugi");
    let owns = |p: &Message, bit: usize| p.get("pbin").and_then(Value::as_bytes).is_some_and(|b| b[bit >> 3] & 1 << (bit & 7) != 0);
    let me = one(c.send(req(0x2CEE))).items.remove(0);
    assert!([1, 224, 1024].iter().all(|&b| owns(&me, b)));
    assert!(!owns(&me, 6), "a part for sale stays unowned");
    let list = one(c.send(req(0x1F49)));
    assert!(list.items.iter().all(|p| owns(p, 224)));
}

#[test]
fn an_update_without_an_appearance_fails() {
    // avatar1 must be binary (as text it crashes the client's PlayerData loader, 0x46C750).
    let mut c = Server::new().join(1001, "Yugi");
    let bad = Packet::Message(Message::reply(0x2CEF).with("avatar1", Value::text("x")));
    assert_eq!(one(c.send(bad)).request(), Some(0x2CF6));
}

#[test]
fn the_settings_are_kept_with_the_account() {
    let dir = std::env::temp_dir().join(format!("yo3-character-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("accounts.json");
    let addr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9));
    let player_id = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        let id = d.account_for(addr).players[0].player_id;
        d.update_player(id, |p| {
            p.avatar = vec![7; 82];
            p.speeches[0] = "Hello".into();
            p.title = 2;
        }).unwrap();
        id
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let p = d.account_for(addr).players.into_iter().find(|p| p.player_id == player_id).unwrap();
    assert_eq!((p.avatar, p.speeches[0].as_str(), p.title), (vec![7; 82], "Hello", 2));
}

#[test]
fn an_avatar_of_another_length_is_refused_and_the_old_one_kept() {
    // The client's avatar is 41 BE u16 entries, 82 bytes (its own 0x2CEF; the record's reader copies entries from
    // PlayerData+0x28). Review (2026-11-15): a 34-byte avatar was accepted and sent back to every client that shows
    // this player. 0x2CF6 refuses it.
    let mut c = Server::new().join(1001, "Yugi");
    let good: Vec<u8> = (0..82).collect();
    assert_eq!(one(c.send(update(&good))).request(), Some(0x2CF5));
    for bad in [vec![1u8; 34], vec![1u8; 81], vec![1u8; 84], vec![]] {
        assert_eq!(one(c.send(update(&bad))).request(), Some(0x2CF6), "{} bytes", bad.len());
    }
    let r = one(c.send(req(0x2CEE)));
    assert_eq!(r.items[0].get("avatar1").and_then(Value::as_bytes), Some(good.as_slice()), "the last good one stays");
}
