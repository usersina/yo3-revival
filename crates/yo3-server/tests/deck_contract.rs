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
    World { card_rows: 4833, starter_deck: Some(starter), ..World::new((1..=20).collect()) }
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
}

#[test]
fn the_trunk_owns_every_card_of_the_starter_deck() {
    // cards: owned count per card at index row - 1 (0x410DB0); the deck must fit in it to be valid.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let Some(Value::Bin(cards)) = trunk(&mut c).get("cards").cloned() else { panic!("no cards") };
    assert_eq!((cards[9], cards[19], cards[29]), (2, 1, 1));
    assert_eq!(cards.iter().map(|&n| n as u32).sum::<u32>(), 4);
}
