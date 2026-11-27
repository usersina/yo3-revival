//! Banlists the players keep on the server (#77, `player_banlists`): imported from EDOPro lflist.conf files in Desktop,
//! they are banlists like the game's for every player while the server runs: the deck checks, the `banlist` names,
//! Create Duel's list (0x2398) and the launcher's added-cards file, which writes them into the game.
mod common;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use serde_json::json;
use yo3_data::Regulation;
use yo3_proto::{Message, Packet, Value};
use yo3_server::accounts::Directory;
use yo3_server::front::World;
use yo3_server::player_banlists::Store;
use yo3_server::translate::CardStats;

/// Rows 1-100 are cards with Konami ids 1001-1100; banlist 3 "Unlimited", 20 "Season 16" (the login's) forbids 1004.
fn world() -> World {
    let stats = |row: u16| CardStats { cid: 1000 + row, atk: 0, def: 0, level: 4, attribute: 1, race: 1 };
    let reg = |id, lists: [Vec<u16>; 4]| Regulation { id, kind: 0, status: 0, name_id: 0, lists, default_limit: 3, record: Message::record() };
    let banlists: Vec<(u32, String)> = vec![(3, "Unlimited".into()), (20, "Season 16".into())];
    let settings = yo3_server::settings::Live::open(None, banlists.clone()).unwrap();
    World {
        card_rows: 100,
        cards: (1..=100).map(|row| (row, stats(row))).collect::<HashMap<_, _>>(),
        card_names: (1..=100).map(|row| (1000 + row, format!("Card {row}"))).collect(),
        regulations: [reg(3, Default::default()), reg(20, [vec![1004], vec![], vec![], vec![]])].into_iter().map(|r| (r.id, r)).collect(),
        banlists,
        settings: Arc::new(settings),
        npcs: vec![124],
        ..World::new(vec![3, 20])
    }
}

/// A player's list forbidding Card 1 (Konami id 1001) and limiting Card 2, kept for account `owner` as `name`.
fn keep(world: &World, owner: u32, name: &str) -> u32 {
    let conf = format!("!{name}\n1001 0 --Card 1\n1002 1 --Card 2\n99999999 0 --not a card\n");
    let cid = |code: u32| (1001..=1100).contains(&code).then_some(code as u16);
    let mut store = world.player_banlists.lock().unwrap();
    let (list, unknown) = store.import(owner, "Yugi", "", &conf, 3, &["Unlimited".into(), "Season 16".into()], cid).unwrap();
    assert_eq!(unknown, 1, "the unknown passcode is left out");
    list.id
}

#[test]
fn a_players_banlist_is_one_of_the_servers_banlists_at_once() {
    let w = world();
    w.player_banlists.lock().unwrap().raise_floor(21);
    let id = keep(&w, 1001, "Goat Format");
    assert_eq!(id, 21);
    assert_eq!(w.banlist_names().last(), Some(&(21, "Goat Format".to_string())));
    assert_eq!(w.regulation_list(), [3, 20, 21], "Create Duel's list");
    let rule = w.regulation(21).expect("checked like the game's");
    assert_eq!((rule.limit(1001), rule.limit(1002), rule.limit(1003)), (0, 1, 3));
    // The launcher's file: after the host's lists, with its name's text id.
    let added: serde_json::Value = serde_json::from_str(&w.added_render(false)).unwrap();
    assert_eq!(added["banlists"][0]["name"], "Goat Format");
    assert_eq!(added["banlists"][0]["name_id"], json!(yo3_data::added::BANLIST_NAME_TEXT));
    // Deleted: gone from all of them, its number never given again.
    w.player_banlists.lock().unwrap().delete(1001, "goat format").unwrap();
    assert!(w.regulation(21).is_none() && !w.regulation_list().contains(&21));
    assert_eq!(keep(&w, 1001, "Edison"), 22);
}

fn addr(n: u8) -> IpAddr { IpAddr::V4(Ipv4Addr::new(10, 0, 0, n)) }

fn ask(c: &mut Client, text: &str) -> String {
    let body = Message { tag: 10, fields: vec![("chat_str".into(), Value::text(text)), ("chat_type".into(), Value::U32(0)), ("to_name".into(), Value::text("Server"))], items: vec![] };
    c.send(Packet::Message(Message::reply(0x2134).item(body))).iter().filter(|m| m.request() == Some(0x2455)).flat_map(|m| m.items.iter())
        .filter_map(|i| i.get("chat_str").and_then(Value::as_str)).collect::<Vec<_>>().join("\n")
}

#[test]
fn another_player_picks_it_for_their_computer_duels_and_its_limits_hold() {
    let s = Server::with_world(world());
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let mut c = s.join_from(&d, addr(1));
    let id = keep(&s.world, 9999, "Goat Format");
    let done = ask(&mut c, "banlist goat format");
    assert!(done.contains("computer is now Goat Format"), "{done}");
    assert!(ask(&mut c, "banlist").contains("Goat Format"), "listed among the choices");
    // An NPC duel naming it (as the DLL makes the game do): the starter deck holds Card 1, Forbidden there.
    let rule = Message::record().with("cpu_id", Value::U32(124)).with("match_type", Value::U32(4)).with("regulation_id", Value::U32(id));
    let duel = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(rule);
    let duel_id = one(c.send(Packet::Message(duel))).get("duel_id").and_then(Value::as_u32).unwrap();
    let deck = Message::deck().with("deck_name", Value::text("Mine")).with("main", Value::u16_be_array(&(1..=20).chain(1..=20).collect::<Vec<u16>>()))
        .with("extra", Value::u16_be_array(&[])).with("side", Value::u16_be_array(&[]));
    let Packet::Message(m) = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))]) else { unreachable!() };
    assert_eq!(one(c.send(Packet::Message(m.item(deck)))).request(), Some(0x22CB));
    // Create Duel's list has it.
    let list = one(c.send(req(0x2072)));
    assert_eq!(list.get("regulations"), Some(&Value::u16_be_array(&[3, 20, id as u16])));
}

#[test]
fn the_store_survives_a_restart() {
    let dir = std::env::temp_dir().join(format!("yo3-player-banlists-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    yo3_server::store::initialize(&dir).unwrap();
    let cid = |code: u32| Some(code as u16);
    let mut store = Store::open(Some(&dir), 30, None);
    store.import(1001, "Yugi", "Mine", "!x\n1001 0\n", 3, &[], cid).unwrap();
    let again = Store::open(Some(&dir), 30, None);
    assert_eq!(again.all().iter().map(|l| (l.id, l.name.as_str(), l.owner)).collect::<Vec<_>>(), [(30, "Mine", 1001)]);
    let _ = std::fs::remove_dir_all(&dir);
}
