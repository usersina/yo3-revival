//! Decks are checked on the server too. The client checks the deck it picks (`0x51AB00` -> `0x51AA80`: sizes, cards
//! owned, the regulation) before it asks for a duel, but only a modified client skips that, so Select Deck's
//! `0x1F4F` (DuelManager `0x50ECF0`) answers `0x22CB` (the dialog with text `0x2919`, then the duel scene leaves)
//! for a deck the same rules refuse: main 40-60 cards and extra and side under 16 (`0x51AC50`), every card owned
//! (the starter deck, bought cards, trades, rentals), and the duel's regulation (`0x51A9A0`, `0x51A930`): the rule's
//! `regulation_id`, else the login one.
mod common;

use std::collections::HashMap;

use common::{Client, Server, one, req_with};
use yo3_data::Regulation;
use yo3_proto::{Message, Packet, Value};
use yo3_server::front::{Deck, World};
use yo3_server::translate::CardStats;

/// Rows 1-100 are cards with Konami ids 1001-1100. Regulation 5 forbids 1003 and limits 1002; 6 allows one copy of
/// 1004 only by default (3 elsewhere); 3 is unlimited.
fn world(login: u32) -> World {
    let stats = |row: u16| CardStats { cid: 1000 + row, atk: 0, def: 0, level: 4, attribute: 1, race: 1 };
    let reg = |id, lists: [Vec<u16>; 4]| Regulation { id, kind: 0, status: 0, name_id: 0, lists, default_limit: 3, record: Message::record() };
    let regulations = [reg(3, Default::default()), reg(5, [vec![1003], vec![1002], vec![], vec![]])];
    let starter = Deck { id: 1, name: "Starter Deck".into(), main: (1..=20).chain(1..=20).collect(), extra: vec![], side: vec![] };
    World {
        card_rows: 100,
        starter_deck: Some(starter),
        cards: (1..=100).map(|row| (row, stats(row))).collect::<HashMap<_, _>>(),
        regulations: regulations.into_iter().map(|r| (r.id, r)).collect(),
        login_regulation: login,
        ..World::new(vec![3, 5])
    }
}

/// An NPC duel asked for with `regulation_id` in its rule (0 leaves it out), as 0x17AC makes one.
fn npc_duel(c: &mut Client, regulation_id: u32) -> u32 {
    let mut rule = Message::record().with("cpu_id", Value::U32(124)).with("match_type", Value::U32(4));
    if regulation_id > 0 {
        rule = rule.with("regulation_id", Value::U32(regulation_id));
    }
    let duel = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(rule);
    one(c.send(Packet::Message(duel))).get("duel_id").and_then(Value::as_u32).unwrap()
}

fn pick(c: &mut Client, duel_id: u32, main: &[u16], extra: &[u16], side: &[u16]) -> u32 {
    let deck = Message::deck()
        .with("deck_name", Value::text("Mine"))
        .with("main", Value::u16_be_array(main))
        .with("extra", Value::u16_be_array(extra))
        .with("side", Value::u16_be_array(side));
    let update = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))]);
    let Packet::Message(m) = update else { unreachable!() };
    one(c.send(Packet::Message(m.item(deck)))).request().unwrap()
}

fn starter() -> Vec<u16> {
    (1..=20).chain(1..=20).collect()
}

#[test]
fn a_legal_deck_goes_on_to_the_duel() {
    let server = Server::with_world(world(3));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 0);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CA);
    assert!(server.hub.lock().unwrap().duel_entry(1001).is_some());
}

#[test]
fn a_deck_of_the_wrong_size_is_refused() {
    let server = Server::with_world(world(3));
    let mut player = yo3_server::front::Player::new(1001, "Yugi");
    player.cards.extend((41..=56).map(|row| (row, 1)));
    let mut c = server.join_as(player);
    let id = npc_duel(&mut c, 0);
    let few: Vec<u16> = starter().into_iter().take(39).collect();
    assert_eq!(pick(&mut c, id, &few, &[], &[]), 0x22CB);
    let sixteen: Vec<u16> = (41..=56).collect();
    assert_eq!(pick(&mut c, id, &starter(), &sixteen, &[]), 0x22CB);
    assert_eq!(pick(&mut c, id, &starter(), &[], &sixteen), 0x22CB);
    // 15 extra cards are fine, and the main deck may be 60
    assert_eq!(pick(&mut c, id, &starter(), &sixteen[..15], &sixteen[15..]), 0x22CA);
    assert!(server.hub.lock().unwrap().duel_entry(1001).is_some());
}

#[test]
fn a_refused_deck_is_not_kept_for_the_duel() {
    let server = Server::with_world(world(3));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 0);
    assert_eq!(pick(&mut c, id, &[1; 10], &[], &[]), 0x22CB);
    assert!(server.hub.lock().unwrap().duel_entry(1001).is_none());
}

#[test]
fn cards_the_player_does_not_own_are_refused() {
    let server = Server::with_world(world(3));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 0);
    // the starter deck owns two of each of rows 1-20: a third copy, or a card outside it
    let mut three = starter();
    three[39] = 1;
    assert_eq!(pick(&mut c, id, &three, &[], &[]), 0x22CB);
    let mut other = starter();
    other[0] = 50;
    assert_eq!(pick(&mut c, id, &other, &[], &[]), 0x22CB);
}

#[test]
fn bought_cards_count_as_owned() {
    let server = Server::with_world(world(3));
    let mut player = yo3_server::front::Player::new(1001, "Yugi");
    player.cards.insert(50, 1);
    let mut c = server.join_as(player);
    let id = npc_duel(&mut c, 0);
    let mut deck = starter();
    deck[0] = 50;
    assert_eq!(pick(&mut c, id, &deck, &[], &[]), 0x22CA);
}

#[test]
fn the_rules_regulation_decides_else_the_login_one() {
    // regulation 5 forbids 1003 (row 3) and limits 1002 (row 2) to one copy; the starter deck holds two of each
    let server = Server::with_world(world(3));
    let mut player = yo3_server::front::Player::new(1001, "Yugi");
    player.cards.extend((21..=30).map(|row| (row, 2)));
    let mut c = server.join_as(player);
    let id = npc_duel(&mut c, 5);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CB);
    // no row 3, one row 2: legal
    let mut legal: Vec<u16> = (4..=20).chain(4..=20).chain([2, 21, 21, 22, 22, 23]).collect();
    assert_eq!(legal.len(), 40);
    assert_eq!(pick(&mut c, id, &legal, &[], &[]), 0x22CA);
    // a second row 2 breaks the limit, row 3 is forbidden
    legal[39] = 2;
    assert_eq!(pick(&mut c, id, &legal, &[], &[]), 0x22CB);
    legal[39] = 3;
    legal[0] = 21;
    assert_eq!(pick(&mut c, id, &legal, &[], &[]), 0x22CB);
    // the same deck is fine in an unlimited duel
    let unlimited = npc_duel(&mut c, 3);
    assert_eq!(pick(&mut c, unlimited, &starter(), &[], &[]), 0x22CA);
}

#[test]
fn the_login_regulation_applies_to_a_rule_that_names_none() {
    let server = Server::with_world(world(5));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 0);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CB);
    // an unknown regulation id falls back to the login one too
    let id = npc_duel(&mut c, 99);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CB);
    let server = Server::with_world(world(3));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 99);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CA);
}

#[test]
fn a_card_swap_is_not_checked() {
    // update_type 2 sends the deck as swapped between a match's duels; it only keeps the match's entry
    let server = Server::with_world(world(3));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 0);
    let update = req_with(0x1F4F, &[("duel_id", Value::U32(id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(2))]);
    let Packet::Message(m) = update else { unreachable!() };
    let short = Message::deck().with("main", Value::u16_be_array(&[1, 2, 3]));
    assert_eq!(one(c.send(Packet::Message(m.item(short)))).request(), Some(0x22CA));
}

#[test]
fn the_testing_aid_overrides_the_duels_regulation() {
    // --duel-regulation 5 checks every duel against list 5, though this one names 3 (unlimited)
    let server = Server::with_world(World { test_regulation: Some(5), ..world(3) });
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 3);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CB);
}

#[test]
fn the_opponent_of_a_refused_player_is_sent_out_of_the_duel() {
    // 1001's deck is refused (0x22CB; its client goes back to the map), so 1002 would wait at the janken for good: its
    // front gets a 0x2009 with duel_end 3, which DuelScene (0x515BD3) answers with text 0x2918 and leaves.
    let server = Server::with_world(World { test_regulation: Some(5), ..world(3) });
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let auto = |c: &mut Client| {
        let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
        let fields = [("event_id", Value::U32(2)), ("join_id", join.get("join_id").cloned().unwrap()), ("match_type", Value::U32(1)), ("level", Value::U32(0))];
        c.send(req_with(0x1798, &fields));
    };
    auto(&mut a);
    auto(&mut b);
    let wait = |c: &mut Client| {
        let start = std::time::Instant::now();
        let mut out = c.pushed();
        while out.is_empty() && start.elapsed() < std::time::Duration::from_secs(3) {
            std::thread::sleep(std::time::Duration::from_millis(20));
            out = c.pushed();
        }
        out
    };
    let push = one(wait(&mut a));
    let duel_id = push.items.iter().find_map(|i| i.get("duel_id").and_then(Value::as_u32)).expect("duel_id");
    wait(&mut b);
    assert_eq!(pick(&mut a, duel_id, &starter(), &[], &[]), 0x22CB);
    let cancel = one(b.pushed());
    assert_eq!(cancel.request(), Some(0x2009));
    let result = &cancel.items[0].items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("result")).unwrap();
    assert_eq!(result.get("duel_end"), Some(&Value::U32(3)));
    // the match is over: a second refusal sends nothing more
    assert_eq!(pick(&mut a, duel_id, &starter(), &[], &[]), 0x22CB);
    assert!(b.pushed().is_empty());
}
