//! The card profile's lifecycle (#60): a game that loaded another card profile than the server pins now, or none while one
//! is pinned, cannot enter any duel. The game reads its card table once at start, so it would duel with stale card data;
//! it is refused at every way in with the reply the client shows for that request and a line from "Server" in its chat,
//! and told to restart (Play again). A game that loaded the pinned profile, and every game while the game's own cards
//! are pinned, is unaffected. Hermetic: a one-card anime profile, no game files.
mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::{Client, Server, one, req_with};
use yo3_data::Card;
use yo3_proto::{Message, Packet, Value};
use yo3_server::auth::Identity;
use yo3_server::front::World;
use yo3_server::profile::{AnimeData, CardView, Profile, Profiles};

const NOTE: &str = yo3_server::engine::STALE_GAME_NOTE;

/// A world whose anime profile changes one card, the `anime-cards` setting as given.
fn world(anime: bool) -> World {
    let card = Card { row: 100, cid: 4021, atk: 1800, def: 1600, level: 5, attribute: 2, race: 2, kind: 3, art: 1, words: [4021, 3 << 3], genre: 0xAB, name: String::new(), desc: String::new() };
    let data = HashMap::from([(4021, AnimeData { code: 511000000, ty: 0x11, atk: 1800, def: 1600, level: 5, race: 1, attribute: 4, text: "Anime.".into(), tag: yo3_server::profile::ANIME_TAG, differs: vec![] })]);
    let (profile, _) = Profile::build("anime", [&card], &data);
    let world = World { profiles: Profiles::with_anime(CardView::over(&CardView::default(), Arc::new(profile))), ..World::new((1..=20).collect()) };
    if anime { world.settings.set("anime-cards", "on").unwrap(); }
    world
}

fn hash(world: &World) -> String {
    world.profiles.anime.as_ref().unwrap().profile.hash()
}

/// A client signed in as `user`, whose game loaded `loaded` ("anime" with the pinned hash, "" for the game's own, None: it
/// fetched nothing).
fn game(server: &Server, user: u32, name: &str, loaded: Option<&str>) -> Client {
    let mut c = server.join(user, name);
    let who = Identity { user_id: user, session_id: u64::from(user) };
    c.session.set_identity(who);
    if let Some(profile) = loaded {
        let name = if profile.is_empty() { "official" } else { "anime" };
        server.world.profiles.note_launch(who, (name.into(), profile.into()));
    }
    c
}

fn told(c: &mut Client) -> bool {
    c.pushed().iter().any(|m| m.request() == Some(0x2455) && m.items[0].get("chat_str") == Some(&Value::text(NOTE)))
}

fn join_id(c: &mut Client, event_id: u32) -> Value {
    one(c.send(req_with(0x1784, &[("event_id", Value::U32(event_id))]))).get("join_id").cloned().unwrap()
}

/// 0x1784 then 0x1798 (Auto Duel, or an event's Participate).
fn auto(c: &mut Client, event_id: u32) -> Vec<Message> {
    let join = join_id(c, event_id);
    c.send(req_with(0x1798, &[("event_id", Value::U32(event_id)), ("join_id", join), ("match_type", Value::U32(1)), ("level", Value::U32(0))]))
}

fn npc(c: &mut Client) -> Message {
    let rule = Message::record().with("cpu_id", Value::U32(124)).with("match_type", Value::U32(4));
    one(c.send(Packet::Message(Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(rule))))
}

/// Create Duel's request (0x17AC with a status and no cpu_id).
fn create_duel(c: &mut Client) -> Message {
    let settings = [("match_type", 1), ("surrender", 1), ("party", 1), ("watch", 1), ("lp_start", 4000), ("regulation_id", 1), ("status", 1)]
        .into_iter().fold(Message { tag: 4, ..Message::default() }, |m, (k, v)| m.with(k, Value::U32(v))).with("password", Value::text(""));
    one(c.send(Packet::Message(Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(settings))))
}

fn select_deck(c: &mut Client, duel_id: u32) -> Message {
    let deck = Message::record().with("main", Value::u16_be_array(&[100, 101, 102]));
    let Packet::Message(m) = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))]) else { unreachable!() };
    one(c.send(Packet::Message(m.item(deck))))
}

/// The Duel Menu's counts (0x17FD): (waiting for an automatic match, hosted duels).
fn counts(c: &mut Client) -> (u32, u32) {
    let r = one(c.send(common::req(0x17FC)));
    let n = |k: &str| r.get(k).and_then(Value::as_u32).unwrap();
    (n("duel_auto"), n("duel_ready"))
}

fn wait_push(c: &mut Client, request: u32) -> Option<Message> {
    for _ in 0..150 {
        if let Some(m) = c.pushed().into_iter().find(|m| m.request() == Some(request)) { return Some(m) }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

#[test]
fn a_game_that_loaded_another_profile_cannot_start_any_duel() {
    // The game loaded the anime profile; the host turned the setting off (the same holds for the other way round).
    let server = Server::with_world(world(true));
    let loaded = hash(&server.world);
    let mut stale = game(&server, 1001, "Yugi", Some(&loaded));
    // Current: Auto Duel queues (0x1799), an NPC duel gets its id (0x17AD), Create Duel too.
    assert_eq!(auto(&mut stale, 2).iter().map(Message::request).collect::<Vec<_>>(), [Some(0x1799)]);
    stale.send(common::req(0x17A2));
    assert_eq!(npc(&mut stale).request(), Some(0x17AD));
    assert!(!told(&mut stale), "a matching game is told nothing");
    server.world.settings.set("anime-cards", "off").unwrap();
    // Auto Duel (0x4A1380): 0x179A, and the reason in chat. Nothing is queued.
    let refused = auto(&mut stale, 2);
    assert_eq!(refused.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x179A)]);
    assert!(told(&mut stale));
    assert_eq!(counts(&mut stale), (0, 0), "nothing is queued");
    // An NPC duel (0x4A1790): 0x17AE, the game's "A transmission error has occurred.", and the reason first.
    assert_eq!(npc(&mut stale).request(), Some(0x17AE));
    assert!(told(&mut stale));
    // Create Duel is the same request.
    assert_eq!(create_duel(&mut stale).request(), Some(0x17AE));
    assert_eq!(counts(&mut stale), (0, 0), "no duel is hosted");
    // An official event (Participate with its id) is Auto Duel's request.
    assert_eq!(auto(&mut stale, 3).first().and_then(Message::request), Some(0x179A));
    // User events: Create and entry (0x3390, 0x3397) answer their error twins.
    assert_eq!(one(stale.send(req_with(0x3390, &[("event_name", Value::text("Cup"))]))).request(), Some(0x33F5));
    assert_eq!(one(stale.send(req_with(0x3397, &[("event_id", Value::U32(100))]))).request(), Some(0x33FB));
    // The game that loaded the profile now pinned goes ahead again after the setting returns.
    server.world.settings.set("anime-cards", "on").unwrap();
    stale.pushed();
    assert_eq!(npc(&mut stale).request(), Some(0x17AD));
}

#[test]
fn a_game_that_loaded_nothing_cannot_duel_while_a_profile_is_pinned_but_standard_mode_serves_every_client() {
    // A game started before the host turned the profile on, or without the connection DLL's profile support: it fetched
    // nothing, so nothing proves it shows the pinned cards.
    let server = Server::with_world(world(true));
    let mut old = game(&server, 1001, "Yugi", None);
    assert_eq!(npc(&mut old).request(), Some(0x17AE));
    assert!(told(&mut old));
    assert_eq!(auto(&mut old, 2).first().and_then(Message::request), Some(0x179A));
    // The launcher's own read of the profile (to place the decks) is not the game's fetch: nothing is noted for it.
    assert!(!server.world.profiles.current_for(Identity { user_id: 1001, session_id: 1001 }, true));
    // The game's own cards pinned (the setting off, or a server with no anime data): every client duels, with or without a
    // fetch, and a client signed in without an identity (the front's tests) too.
    server.world.settings.set("anime-cards", "off").unwrap();
    assert_eq!(npc(&mut old).request(), Some(0x17AD));
    let mut official = game(&server, 1002, "Kaiba", Some(""));
    assert_eq!(npc(&mut official).request(), Some(0x17AD));
    let mut plain = server.join(1003, "Joey");
    assert_eq!(npc(&mut plain).request(), Some(0x17AD));
    // And the other way: a server that never had anime data keeps every game current while the setting is on.
    let bare = Server::new();
    bare.world.settings.set("anime-cards", "on").unwrap();
    let mut c = game(&bare, 1001, "Yugi", None);
    assert_eq!(npc(&mut c).request(), Some(0x17AD));
}

#[test]
fn a_game_that_became_stale_while_waiting_is_refused_at_select_deck_and_its_opponent_is_released() {
    // Auto Duel matched two current games; the host then turned the setting off. The one whose game loaded the profile
    // cannot pick a deck (0x22CB: "You have been disconnected from the Duel Server.", back to the map) and the match
    // ends for its opponent as for any refused deck (0x2009, duel_end 3).
    let server = Server::with_world(world(true));
    let loaded = hash(&server.world);
    let (mut a, mut b) = (game(&server, 1011, "Mai", Some(&loaded)), game(&server, 1012, "Tea", Some(&loaded)));
    auto(&mut a, 2);
    auto(&mut b, 2);
    let push = wait_push(&mut a, 0x1978).expect("matched");
    wait_push(&mut b, 0x1978).expect("matched");
    let duel = push.items.iter().find_map(|i| i.get("duel_id").and_then(Value::as_u32)).expect("duel_id");
    server.world.settings.set("anime-cards", "off").unwrap();
    assert_eq!(select_deck(&mut a, duel).request(), Some(0x22CB), "refused");
    assert!(told(&mut a));
    assert!(server.hub.lock().unwrap().duel_entry(1011).is_none(), "no entry was made for the duel");
    wait_push(&mut b, 0x2009).expect("the opponent is told the duel is cancelled (duel_end 3)");
    assert!(server.hub.lock().unwrap().match_ids().is_empty(), "the match is over");
}

#[test]
fn a_stale_challenger_is_refused_and_a_stale_host_declines() {
    let server = Server::with_world(world(true));
    let loaded = hash(&server.world);
    let (mut host, mut challenger) = (game(&server, 1001, "Yugi", Some(&loaded)), game(&server, 1002, "Kaiba", Some(&loaded)));
    let duel_id = create_duel(&mut host).get("duel_id").and_then(Value::as_u32).expect("hosted");
    let offer = |c: &mut Client| {
        let join = join_id(c, 1);
        let Packet::Message(m) = req_with(0x17B6, &[("event_id", Value::U32(1)), ("join_id", join), ("duel_id", Value::U32(duel_id))]) else { unreachable!() };
        one(c.send(Packet::Message(m.with("password", Value::text("")))))
    };
    assert_eq!(offer(&mut challenger).request(), Some(0x17B7), "a current challenger is offered");
    // The host's game goes stale before it answers: its Accept is a decline, and no match is made.
    server.world.settings.set("anime-cards", "off").unwrap();
    let fields = [("event_id", 1), ("join_id", 1), ("duel_id", duel_id), ("answer", 2)].map(|(k, v)| (k, Value::U32(v)));
    assert_eq!(one(host.send(req_with(0x17CA, &fields))).request(), Some(0x17CB));
    assert!(told(&mut host));
    let answer = wait_push(&mut challenger, 0x17CA).expect("the challenger is answered");
    assert_ne!(answer.get("answer").and_then(Value::as_u32), Some(2), "declined");
    assert!(server.hub.lock().unwrap().match_ids().is_empty());
    // A stale challenger meets a refusal with the game's "Duel Request was refused." (201) and the reason.
    let other = create_duel(&mut game(&server, 1003, "Joey", Some(""))).get("duel_id").and_then(Value::as_u32).unwrap();
    let join = join_id(&mut challenger, 1);
    let Packet::Message(m) = req_with(0x17B6, &[("event_id", Value::U32(1)), ("join_id", join), ("duel_id", Value::U32(other))]) else { unreachable!() };
    let refused = one(challenger.send(Packet::Message(m.with("password", Value::text("")))));
    assert_eq!((refused.request(), refused.get("error_code").and_then(Value::as_u32)), (Some(0x17B8), Some(201)));
    assert!(told(&mut challenger));
}

#[test]
fn a_launch_is_remembered_per_session_and_a_relaunch_is_current() {
    // A restart of the game fetches again, under a new session: its record replaces nothing of the old one's, and the
    // old session (its game closed) goes with it.
    let server = Server::with_world(world(true));
    let profiles = &server.world.profiles;
    let (old, new) = (Identity { user_id: 7, session_id: 1 }, Identity { user_id: 7, session_id: 2 });
    profiles.note_launch(old, ("official".into(), String::new()));
    assert!(!profiles.current_for(old, true) && profiles.current_for(old, false));
    assert!(!profiles.current_for(new, true), "a new session's game has fetched nothing yet");
    profiles.note_launch(new, ("anime".into(), hash(&server.world)));
    assert!(profiles.current_for(new, true) && !profiles.current_for(new, false));
    assert!(!profiles.current_for(old, true), "the old game still has the old cards");
}

#[test]
fn switching_anime_off_and_on_again_keeps_every_card_and_quantity_of_the_servers_decks() {
    // Row 100 is an Extra Deck card in the game and a Main Deck card under the profile. The server places every deck it
    // reads (list, uploads, checks, official decks) by the profile selected at that moment, never losing a card.
    let world = World { extra_deck_rows: [100].into(), ..world(false) };
    let deck = yo3_server::front::Deck { id: 1, name: "d".into(), main: vec![10, 11, 11, 12], extra: vec![100, 100, 50], side: vec![12, 100] };
    let counts = |d: &yo3_server::front::Deck| {
        let mut all = std::collections::BTreeMap::<u16, usize>::new();
        for r in d.main.iter().chain(&d.extra).chain(&d.side) { *all.entry(*r).or_default() += 1 }
        all
    };
    let mut placed = world.place_deck(&deck);
    assert_eq!(placed, deck, "the game's own cards: the deck is as it was");
    for round in 0..3 {
        world.settings.set("anime-cards", "on").unwrap();
        placed = world.place_deck(&placed);
        assert_eq!(counts(&placed), counts(&deck), "round {round}: on");
        assert_eq!((placed.main.iter().filter(|r| **r == 100).count(), placed.extra.contains(&100)), (2, false), "the profile's card plays from the Main Deck");
        assert_eq!(placed.side, deck.side, "the side deck is never touched");
        world.settings.reset("anime-cards").unwrap();
        placed = world.place_deck(&placed);
        assert_eq!(counts(&placed), counts(&deck), "round {round}: off");
        // The same cards in the same lists (the card that moved goes to the end of its list).
        let sorted = |mut v: Vec<u16>| { v.sort(); v };
        assert_eq!((sorted(placed.main.clone()), sorted(placed.extra.clone())), (sorted(deck.main.clone()), sorted(deck.extra.clone())), "round {round}: the saved deck again");
    }
}

#[test]
fn a_stale_game_cannot_watch_a_duel() {
    // Watch Duel (0x1838 -> 0x1839) shows the duel with the watcher's own card data: a game that loaded another profile
    // would see other cards' stats and texts. It gets 0x183A (the game's "cannot watch") and the reason in its chat; a
    // current game asking for a duel that isn't on is refused the same way, without the reason.
    let server = Server::with_world(world(true));
    let loaded = hash(&server.world);
    let watch = || req_with(0x1838, &[("duel_id", Value::U32(7))]);
    let mut stale = game(&server, 1021, "Rex", Some(""));
    assert_eq!(one(stale.send(watch())).request(), Some(0x183A));
    assert!(told(&mut stale));
    let mut current = game(&server, 1022, "Weevil", Some(&loaded));
    assert_eq!(one(current.send(watch())).request(), Some(0x183A));
    assert!(!told(&mut current));
}
