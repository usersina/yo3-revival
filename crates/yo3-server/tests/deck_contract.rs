//! Decks. The client keeps them as local files (players\<pname>\decks\<deck_name>.y3d, saved by
//! 0x52EE60). The deck editor (DeckEditTest, handler 0x43FC60) asks for the deck list (0x1F4E, sent
//! at 0x440C43) and saves every listed deck it doesn't have yet. The NPC duel (0x4BD880) then needs
//! a deck that validates (0x51AB00) against the collection built from the trunk (0x4F6340).
mod common;

use common::{Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::front::{Deck, World};

fn world() -> World {
    let starter = Deck { id: 1, name: "Starter Deck".into(), main: vec![10, 10, 20], extra: vec![30], side: vec![] };
    // NPC 124 is the one the uploads' duels name (an NPC the server lacks is refused at Select Deck).
    World { card_rows: 4833, starter_deck: Some(starter), npcs: vec![124], ..World::new((1..=20).collect()) }
}

fn trunk(c: &mut common::Client) -> Message {
    let replies = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
    assert_eq!(replies[0].request(), Some(9000));
    replies[0].items[0].clone()
}

#[test]
fn the_deck_list_carries_the_starter_deck() {
    // 0x22C8 case (0x43FC60): per item deck_id (int), deck_name and discription (string),
    // card_protector (int), and main / extra / side (binary u16 BE CARD_Prop rows, read by 0x414410).
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let r = one(c.send(req(0x1F4E)));
    assert_eq!(r.request(), Some(0x22C8));
    assert_eq!(r.items.len(), 1);
    let d = &r.items[0];
    // The client builds each item by its tag (factory 0x531B90): 6 makes a YO3Deck (0x414240),
    // whose vectors 0x414410 fills. A plain record (tag 3) aborts the client there.
    assert_eq!(d.tag, 6);
    assert_eq!(d.get("deck_id"), Some(&Value::U32(1)));
    assert_eq!(d.get("deck_name").and_then(Value::as_str).as_deref(), Some("Starter Deck"));
    assert!(d.get("discription").and_then(Value::as_str).is_some());
    assert_eq!(d.get("card_protector"), Some(&Value::U32(0)));
    assert_eq!(d.get("main"), Some(&Value::u16_be_array(&[10, 10, 20])));
    assert_eq!(d.get("extra"), Some(&Value::u16_be_array(&[30])));
    assert_eq!(d.get("side"), Some(&Value::u16_be_array(&[])));
}

#[test]
fn no_starter_deck_means_an_empty_list() {
    let mut c = Server::new().join(1001, "Yugi");
    assert!(one(c.send(req(0x1F4E))).items.is_empty());
    c.send(req_with(0x1F4C, &[("player_id", Value::U32(1001))]));
    assert!(c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))])).iter()
        .all(|m| m.request() != Some(0x22C8)), "no automatic list without a candidate");
}

#[test]
fn login_supplies_a_native_starter_after_collection_and_rentals() {
    // DLL post-dispatch hook 0x4F44BD: player decks were loaded by 0x46BDB0; 0x51AA80
    // needs the trunk (slot 18) and rental records (slot 8) before validating/saving tag 6.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    c.send(req_with(0x1F4C, &[("player_id", Value::U32(1001))]));
    let replies = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
    assert_eq!(replies.iter().map(Message::request).collect::<Vec<_>>(),
        [Some(9000), Some(0x251C), Some(0x252A), Some(0x2D6C), Some(0x22C8)]);
    let deck = &replies[4].items[0];
    assert_eq!(deck.tag, 6);
    assert_eq!(deck, &one(c.send(req(0x1F4E))).items[0]);
}

#[test]
fn automatic_decks_are_only_pushed_on_the_first_trunk_after_each_player_selection() {
    let mut c = Server::with_world(world()).join(1001, "Yugi"); // already received the login trunk
    let has_bootstrap = |messages: Vec<Message>| messages.iter().any(|m| m.request() == Some(0x22C8));
    let trunk_request = || req_with(0x1FA4, &[("player_id", Value::U32(1001))]);
    assert!(!has_bootstrap(c.send(trunk_request())), "ordinary editor/shop refreshes keep their existing replies");
    c.send(req_with(0x1F4C, &[("player_id", Value::U32(1001))]));
    assert!(has_bootstrap(c.send(trunk_request())));
    assert!(!has_bootstrap(c.send(trunk_request())));
    c.send(req_with(0x1F4C, &[("player_id", Value::U32(9999))]));
    assert!(!has_bootstrap(c.send(trunk_request())), "a refused selection cannot initialize a player's decks");
}

#[test]
fn the_trunk_owns_every_card_of_the_starter_deck() {
    // cards: owned count per card at index row - 1 (0x410DB0); the deck must fit in it to be valid.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let Some(Value::Bin(cards)) = trunk(&mut c).get("cards").cloned() else { panic!("no cards") };
    assert_eq!((cards[9], cards[19], cards[29]), (2, 1, 1));
    assert_eq!(cards.iter().map(|&n| n as u32).sum::<u32>(), 4);
}

#[test]
fn deleting_a_deck_is_acknowledged() {
    // The editor deletes the local file (0x52F0C0), then sends 0x1F50 (0x4449A9); 0x22CC only logs
    // "deck delete succes", 0x22CD would show an error. Saving a deck never reaches the server.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let delete = req_with(0x1F50, &[("delete_type", Value::U32(0)), ("deck_name", Value::text("Starter Deck"))]);
    let r = one(c.send(delete));
    assert_eq!(r.request(), Some(0x22CC));
    assert_eq!(r.fields.len(), 1);
}

// Decks follow a player to another machine. The deck editor never uploads a save (the local file only), but a duel's
// Select Deck sends the whole deck it picked (0x1F4F update_type 1, DuelManager 0x50ECF0), so the server keeps those
// and lists them in 0x22C8, where a client that lacks a deck of that name saves it (0x43E550).
mod follow {
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::{Arc, Mutex};

    use super::*;
    use yo3_proto::Packet;
    use yo3_server::accounts::Directory;

    const ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));

    fn upload(c: &mut common::Client, update_type: u32, name: &str, main: &[u16]) {
        // A real Select Deck names a duel this session created; an arbitrary id grants no membership.
        let duel = one(c.send(Packet::Message(Message::reply(0x17AC).item(Message::record().with("cpu_id", Value::U32(124))))));
        let duel_id = duel.get("duel_id").cloned().unwrap();
        let deck = Message::deck()
            .with("deck_id", Value::U32(7))
            .with("deck_name", Value::text(name))
            .with("discription", Value::text(""))
            .with("card_protector", Value::U32(0))
            .with("main", Value::u16_be_array(main))
            .with("extra", Value::u16_be_array(&[]))
            .with("side", Value::u16_be_array(&[]));
        let Packet::Message(m) = req_with(0x1F4F, &[("duel_id", duel_id), ("event_id", Value::U32(0)), ("update_type", Value::U32(update_type))])
        else {
            unreachable!()
        };
        assert_eq!(one(c.send(Packet::Message(m.item(deck)))).request(), Some(0x22CA));
    }

    fn names(c: &mut common::Client) -> Vec<String> {
        one(c.send(req(0x1F4E))).items.iter().map(|d| d.get("deck_name").and_then(Value::as_str).unwrap()).collect()
    }

    fn setup() -> (Server, Arc<Mutex<Directory>>) {
        (Server::with_world(world()), Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()]))))
    }

    #[test]
    fn a_deck_picked_for_a_duel_is_listed_on_another_machine() {
        let (server, d) = setup();
        let mut here = server.join_from(&d, ADDR);
        upload(&mut here, 1, "Dragons", &[10, 11, 12]);
        let mut there = server.join_from(&d, ADDR);
        assert_eq!(names(&mut there), ["Starter Deck", "Dragons"]);
        let list = one(there.send(req(0x1F4E)));
        assert_eq!(list.items[1].tag, 6);
        assert_eq!(list.items[1].get("main"), Some(&Value::u16_be_array(&[10, 11, 12])));
        // the same name again updates it rather than adding one
        upload(&mut here, 1, "Dragons", &[10]);
        let list = one(there.send(req(0x1F4E)));
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[1].get("main"), Some(&Value::u16_be_array(&[10])));
    }

    #[test]
    fn a_card_swap_and_the_starters_name_are_not_kept() {
        let (server, d) = setup();
        let mut c = server.join_from(&d, ADDR);
        upload(&mut c, 2, "Swapped", &[10]);
        upload(&mut c, 1, "Starter Deck", &[99]);
        let list = one(c.send(req(0x1F4E)));
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].get("main"), Some(&Value::u16_be_array(&[10, 10, 20])));
    }

    #[test]
    fn deleting_a_deck_forgets_it_and_players_do_not_share_decks() {
        let (server, d) = setup();
        let mut c = server.join_from(&d, ADDR);
        upload(&mut c, 1, "Dragons", &[10]);
        upload(&mut c, 1, "Zombies", &[11]);
        let delete = req_with(0x1F50, &[("delete_type", Value::U32(0)), ("deck_name", Value::text("Dragons"))]);
        assert_eq!(one(c.send(delete)).request(), Some(0x22CC));
        assert_eq!(names(&mut c), ["Starter Deck", "Zombies"]);
        let mut other = server.join_from(&d, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 6)));
        assert_eq!(names(&mut other), ["Starter Deck"]);
    }

    #[test]
    fn the_client_uses_ten_decks_at_most() {
        let (server, d) = setup();
        let mut c = server.join_from(&d, ADDR);
        for i in 0..12 {
            upload(&mut c, 1, &format!("Deck {i}"), &[10]);
        }
        assert_eq!(names(&mut c).len(), 10);
    }

    #[test]
    fn kept_decks_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("yo3-decks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("accounts.json");
        let server = Server::with_world(world());
        {
            let d = Arc::new(Mutex::new(Directory::open(vec!["Yugi".into()], &file).unwrap()));
            let mut c = server.join_from(&d, ADDR);
            upload(&mut c, 1, "Dragons", &[10, 11]);
        }
        let d = Arc::new(Mutex::new(Directory::open(vec!["Yugi".into()], &file).unwrap()));
        let mut c = server.join_from(&d, ADDR);
        assert_eq!(names(&mut c), ["Starter Deck", "Dragons"]);
    }
}

#[test]
fn every_player_gets_the_official_character_decks() {
    // The client's own NPC decks (deck_type 3 presets) include the main characters', three each: Yami Yugi's, Jaden's,
    // Yusei's. Every player owns their cards on top of the starter deck's, so they pass the deck check, and gets them
    // in the deck list after the starter deck.
    let yugi = Deck { id: 20, name: "Yugi - Miracle Magic".into(), main: vec![10, 40], extra: vec![], side: vec![] };
    let yusei = Deck { id: 261, name: "Yusei - Power of Bonds".into(), main: vec![50], extra: vec![60], side: vec![] };
    let mut c = Server::with_world(World { official_decks: vec![yugi, yusei], ..world() }).join(1001, "Yugi");
    let r = one(c.send(req(0x1F4E)));
    let names: Vec<String> = r.items.iter().filter_map(|d| d.get("deck_name").and_then(Value::as_str)).collect();
    assert_eq!(names, ["Starter Deck", "Yugi - Miracle Magic", "Yusei - Power of Bonds"]);
    assert_eq!(r.items[2].get("extra"), Some(&Value::u16_be_array(&[60])));
    let Some(Value::Bin(cards)) = trunk(&mut c).get("cards").cloned() else { panic!("no cards") };
    // Row 10: twice in the starter deck and once in Yugi's; rows 40, 50, 60 once each.
    assert_eq!((cards[9], cards[39], cards[49], cards[59]), (3, 1, 1, 1));
}

#[test]
fn the_official_decks_take_turns_in_the_room_the_list_has() {
    // The client uses at most 10 decks of a list and keeps every listed deck it lacks (by name) as a local file, so
    // listing one it has costs nothing. The official decks fill the room the starter deck and the player's own decks
    // leave, each list going on where the last stopped: any PC gets them all over a few lists, with nothing kept
    // per player. Twelve here: nine fit beside the starter deck.
    let official: Vec<Deck> = (0..12).map(|i| Deck { id: 100 + i, name: format!("Official {i}"), main: vec![10], extra: vec![], side: vec![] }).collect();
    let mut c = Server::with_world(World { official_decks: official, ..world() }).join(1001, "Yugi");
    let numbers = |c: &mut common::Client| -> Vec<u32> {
        let names: Vec<String> = one(c.send(req(0x1F4E))).items.iter().filter_map(|d| d.get("deck_name").and_then(Value::as_str)).collect();
        assert_eq!(names[0], "Starter Deck");
        names[1..].iter().map(|n| n.trim_start_matches("Official ").parse().unwrap()).collect()
    };
    assert_eq!(numbers(&mut c), (0..9).collect::<Vec<_>>());
    assert_eq!(numbers(&mut c), [9, 10, 11, 0, 1, 2, 3, 4, 5]);
    assert_eq!(numbers(&mut c), [6, 7, 8, 9, 10, 11, 0, 1, 2]);
}
