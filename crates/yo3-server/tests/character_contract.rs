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
        });
        id
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let p = d.account_for(addr).players.into_iter().find(|p| p.player_id == player_id).unwrap();
    assert_eq!((p.avatar, p.speeches[0].as_str(), p.title), (vec![7; 82], "Hello", 2));
}
