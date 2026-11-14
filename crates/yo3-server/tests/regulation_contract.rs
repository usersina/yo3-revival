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
        // NPC 124 is the one the duels name (an NPC the server lacks is refused at Select Deck).
        npcs: vec![124],
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
fn automatic_starter_is_withheld_when_banned_or_no_longer_owned() {
    // Native 0x51AA80 checks collection and regulation before 0x52C320 writes anything.
    for login in [3, 5] {
        let mut c = Server::with_world(world(login)).join(1001, "Yugi");
        c.send(req_with(0x1F4C, &[("player_id", Value::U32(1001))]));
        let replies = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
        assert_eq!(replies.iter().any(|m| m.request() == Some(0x22C8)), login == 3);
    }
    let mut player = yo3_server::front::Player::new(1001, "Yugi");
    player.given.insert(1, 1);
    let mut c = Server::with_world(world(3)).join_as(player);
    c.send(req_with(0x1F4C, &[("player_id", Value::U32(1001))]));
    assert!(c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))])).iter()
        .all(|m| m.request() != Some(0x22C8)));
}

#[test]
fn automatic_decks_offer_a_legal_official_fallback_without_the_banned_starter() {
    let mut w = world(5);
    w.official_decks.push(Deck { id: 120, name: "Yugi".into(), main: (41..=60).chain(41..=60).collect(), extra: vec![], side: vec![] });
    let mut c = Server::with_world(w).join(1001, "Yugi");
    c.send(req_with(0x1F4C, &[("player_id", Value::U32(1001))]));
    let replies = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
    let bootstrap = replies.iter().find(|m| m.request() == Some(0x22C8)).expect("legal fallback");
    assert_eq!(bootstrap.items.len(), 1);
    assert_eq!(bootstrap.items[0].get("deck_name").and_then(Value::as_str).as_deref(), Some("Yugi"));
}

fn matched(server: &Server) -> (Client, Client, u32) {
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    for c in [&mut a, &mut b] {
        let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
        c.send(req_with(0x1798, &[("event_id", Value::U32(2)), ("join_id", join.get("join_id").cloned().unwrap()), ("match_type", Value::U32(1))]));
    }
    let wait = |c: &mut Client| {
        let start = std::time::Instant::now();
        loop {
            let out = c.pushed();
            if !out.is_empty() { return one(out) }
            assert!(start.elapsed() < std::time::Duration::from_secs(3), "match push");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    let push = wait(&mut a);
    let id = push.items.iter().find_map(|i| i.get("duel_id").and_then(Value::as_u32)).expect("duel_id");
    wait(&mut b);
    (a, b, id)
}

#[test]
fn an_outsider_cannot_upload_a_deck_for_another_players_match() {
    // 0x50ECF0 / 0x50FDF0 identify the selected match, not permission to join it. Check both Select Deck and swaps.
    let server = Server::with_world(world(3));
    let (mut a, mut b, id) = matched(&server);
    let mut outsider = server.join(1003, "Joey");
    assert_eq!(pick(&mut outsider, id, &starter(), &[], &[]), 0x22CB);
    for update_type in [0, 2, 3] {
        assert_eq!(pick_as(&mut outsider, id, update_type, &starter()), 0x22CA);
    }
    assert!(server.hub.lock().unwrap().duel_entry(1003).is_none());
    assert!(a.pushed().is_empty());
    assert!(b.pushed().is_empty());
    // Both matched players can still enter normally.
    for c in [&mut a, &mut b] {
        assert_eq!(pick(c, id, &starter(), &[], &[]), 0x22CA);
    }
    assert_eq!(server.hub.lock().unwrap().duel_entry(1001).unwrap().opponent, Some(1002));
}

#[test]
fn an_outsiders_invalid_deck_cannot_cancel_another_players_match() {
    let server = Server::with_world(world(3));
    let (mut a, mut b, id) = matched(&server);
    let mut outsider = server.join(1003, "Joey");
    assert_eq!(pick(&mut outsider, id, &[1; 10], &[], &[]), 0x22CB);
    // The Hub also protects its cancellation entry point.
    server.hub.lock().unwrap().refuse_deck(id, 1003);
    assert!(a.pushed().is_empty());
    assert!(b.pushed().is_empty());
    for (player_id, c) in [(1001, &mut a), (1002, &mut b)] {
        assert_eq!(pick(c, id, &starter(), &[], &[]), 0x22CA);
        assert!(server.hub.lock().unwrap().duel_entry(player_id).is_some());
    }
}

#[test]
fn an_outsider_is_not_sent_another_matches_duel_server_handoff() {
    let server = Server::with_world(world(3));
    let (mut a, _b, id) = matched(&server);
    let mut outsider = server.join(1003, "Joey");
    let ready = || req_with(0x17D4, &[("duel_id", Value::U32(id))]);
    assert_eq!(outsider.send(ready()).iter().map(Message::request).collect::<Vec<_>>(), [Some(0x17D5)]);
    assert_eq!(a.send(ready()).iter().map(Message::request).collect::<Vec<_>>(), [Some(0x17D5), Some(0x22C4)]);
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
fn a_card_the_rules_engine_lacks_is_refused() {
    // Row 41 is Ulevo (5925), a game exclusive the rules engine doesn't have (engine::NOT_IN_THE_ENGINE): owned and
    // unrestricted, but the duel couldn't play it, so the deck is refused like a regulation breach.
    let mut w = world(3);
    w.cards.insert(41, CardStats { cid: 5925, atk: 0, def: 0, level: 1, attribute: 1, race: 1 });
    let server = Server::with_world(w);
    let mut player = yo3_server::front::Player::new(1001, "Yugi");
    player.cards.insert(41, 1);
    let mut c = server.join_as(player);
    let id = npc_duel(&mut c, 0);
    let mut deck = starter();
    deck.push(41);
    assert_eq!(pick(&mut c, id, &deck, &[], &[]), 0x22CB);
    assert_eq!(pick(&mut c, id, &starter(), &[], &[]), 0x22CA);
}

#[test]
fn an_extra_deck_monster_in_the_main_deck_is_refused() {
    // The client's deck editor keeps Fusion and Synchro Monsters in the Extra Deck; a deck that mixes them up (a
    // modified client) would split client and engine (the engine moves the card to the Extra Deck by itself; found
    // replaying the decks of Konami's own duel logs), so the server refuses it.
    let mut w = world(3);
    w.extra_deck_rows.insert(41);
    w.extra_deck_rows.insert(42);
    let server = Server::with_world(w);
    let mut player = yo3_server::front::Player::new(1001, "Yugi");
    player.cards.extend([(41, 1), (42, 1), (43, 1)]);
    let mut c = server.join_as(player);
    let id = npc_duel(&mut c, 0);
    let mut main = starter();
    main.push(41);
    assert_eq!(pick(&mut c, id, &main, &[], &[]), 0x22CB);
    assert_eq!(pick(&mut c, id, &starter(), &[43], &[]), 0x22CB, "a main deck card in the Extra Deck");
    assert_eq!(pick(&mut c, id, &starter(), &[41, 42], &[]), 0x22CA);
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

fn check_hosted_regulation(login: u32, hosted: u32, expected: u32) {
    let server = Server::with_world(world(login));
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    // Create Duel (0x49EE90) supplies the selected regulation; acceptance must keep it for Select Deck (0x51A9A0).
    let settings = Message::record().with("status", Value::U32(1)).with("match_type", Value::U32(1))
        .with("regulation_id", Value::U32(hosted));
    let host = one(a.send(Packet::Message(Message::reply(0x17AC).item(settings))));
    let id = host.get("duel_id").and_then(Value::as_u32).unwrap();
    assert_eq!(one(b.send(req_with(0x17B6, &[("duel_id", Value::U32(id))]))).request(), Some(0x17B7));
    a.send(req_with(0x17CA, &[("duel_id", Value::U32(id)), ("answer", Value::U32(2))]));
    let mut announced = vec![];
    for c in [&mut a, &mut b] {
        let ready = c.send(req_with(0x17D4, &[("duel_id", Value::U32(id))]));
        let rule = &ready[1].items[0].items[0];
        announced.push(rule.get("regulation_id").cloned());
    }
    // Check the challenger first: a real refusal cancels the match for its opponent.
    assert_eq!(pick(&mut b, id, &starter(), &[], &[]), expected);
    if expected == 0x22CA {
        assert_eq!(pick(&mut a, id, &starter(), &[], &[]), expected);
    }
    assert_eq!(announced, vec![Some(Value::U32(hosted)); 2]);
}

#[test]
fn a_hosted_banlist_is_enforced_with_an_unlimited_login() {
    check_hosted_regulation(3, 5, 0x22CB);
}

#[test]
fn a_hosted_unlimited_duel_overrides_the_logins_banlist() {
    check_hosted_regulation(5, 3, 0x22CA);
}

#[test]
fn a_card_swap_that_fails_the_check_is_ignored_without_ending_the_match() {
    // update_type 2 sends the deck as swapped between a match's duels; it only keeps the match's entry, so a deck
    // that fails the check is not kept (and nothing is refused)
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

/// The same as `pick` with any update_type (1 Select Deck, 2 the swap between a match's duels, 0 something else).
fn pick_as(c: &mut Client, duel_id: u32, update_type: u32, main: &[u16]) -> u32 {
    let deck = Message::deck().with("deck_name", Value::text("Mine")).with("main", Value::u16_be_array(main));
    let update = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(update_type))]);
    let Packet::Message(m) = update else { unreachable!() };
    one(c.send(Packet::Message(m.item(deck)))).request().unwrap()
}

#[test]
fn the_deck_check_applies_to_every_update_that_names_a_duel() {
    // Only update_type 1 was checked: a modified client could name its duel with another type and bring any deck
    // (cards it doesn't own, banned ones) to the table. Such an update is ignored (the answer is the same, the
    // duel's deck stays what it was), not a reason to end the match: a swap only keeps a deck of the same cards.
    let server = Server::with_world(world(3));
    let mut c = server.join(1001, "Yugi");
    let id = npc_duel(&mut c, 0);
    let cheat: Vec<u16> = (41..=80).collect();
    for update_type in [0, 2, 3] {
        assert_eq!(pick_as(&mut c, id, update_type, &cheat), 0x22CA, "update_type {update_type}");
    }
    assert!(server.hub.lock().unwrap().duel_entry(1001).is_none(), "nothing was kept for the duel");
    assert_eq!(pick_as(&mut c, id, 2, &starter()), 0x22CA);
    assert!(server.hub.lock().unwrap().duel_entry(1001).is_some());
}

#[test]
fn a_decks_row_counts_are_capped() {
    // A deck as the client sends it can be half a megabyte of rows.
    let rows: Vec<u8> = (0..400_000u32).flat_map(|i| ((i % 100) as u16 + 1).to_be_bytes()).collect();
    let record = Message::deck().with("deck_name", Value::text("Big")).with("main", Value::Bin(rows));
    let deck = Deck::from_record(&record);
    assert!(deck.main.len() <= yo3_server::front::MAX_DECK_ROWS);
}

#[test]
fn the_default_banlist_still_lets_a_new_player_duel() {
    // With the game's own data: the starter deck (the first preset the Beta list allows) passes the default login
    // banlist, so a new player has a legal deck for Auto Duel and NPC duels. The character decks don't (Heavy Storm,
    // Premature Burial...): players edit them, or the host picks "unlimited". Skipped without the game.
    let Ok(data) = yo3_data::GameData::load(&yo3_data::default_game_dir()) else { return };
    let cids = |d: &yo3_data::Deck| d.cards().filter_map(|row| data.cards.by_row(row)).map(|c| c.cid).collect::<Vec<_>>();
    let default = data.regulations.iter().find(|r| r.id == yo3_server::front::LOGIN_REGULATION).expect("the default list");
    assert_eq!(data.text(default.name_id), Some("Season 16"));
    let starter = data.starter_deck(yo3_server::front::STARTER_REGULATION).expect("a starter deck");
    assert!(default.allows(cids(starter)), "the starter deck breaks the default banlist");
    let characters = yo3_server::OFFICIAL_DECKS.iter().filter_map(|&(npc, _)| data.npc_deck(npc));
    assert!(characters.clone().count() == 9 && characters.into_iter().all(|d| !default.allows(cids(d))), "the docs say the character decks break it");
}
