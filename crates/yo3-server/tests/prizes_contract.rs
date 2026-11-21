//! What the server hands out beyond the shelves (#19, #18): another artwork of a card as a rare pull, the Special Pack an
//! Official Event win earns (a present like the host's gifts, `0x2774` then `0x2777`), the event titles, the deck recipe
//! an NPC gives for a first win, and the extra Trade Card pool (pack file 232). The client's own files give the
//! artworks, the cards and the recipes; the odds, the cap and the tiers are ours (roadmap's guesses).
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::Directory;
use yo3_server::front::{Ended, Player, World};
use yo3_server::official::{Kind, NewEvent, Prize};
use yo3_server::prizes::{NpcRecipe, Prizes, SPECIAL_PACK_CAP};
use yo3_server::shop::{Arts, ExchangePool, ShopPack};
use yo3_server::titles::TitleData;

fn int(m: &Message, k: &str) -> Option<u32> { m.get(k).and_then(Value::as_u32) }

fn rows(present: &Message) -> Vec<u16> {
    let bytes = present.get("item").and_then(Value::as_bytes).expect("an item");
    bytes.chunks(2).map(|b| u16::from_be_bytes([b[0], b[1]])).collect()
}

fn now() -> i64 { chrono::Utc::now().timestamp() }

// Alternate artworks (#19): a card a pack gives comes, one time in `odds`, as another artwork of the card.

fn pack_world(arts: Arts) -> Server {
    let pack = ShopPack { id: 1, released: String::new(), modified: String::new(), tiers: vec![vec![10]] };
    let prizes = Prizes { arts, ..Prizes::default() };
    Server::with_world(World { packs: vec![pack], card_rows: 50, prizes: Arc::new(prizes), ..World::new((1..=20).collect()) })
}

fn open_one_pack(server: &Server) -> Vec<u16> {
    let mut c = server.join_with_bp(1001, "Yugi", 1000);
    let buy = req_with(0x1B68, &[("sale_id", Value::U32(1)), ("item_num", Value::U32(1)), ("end_time", Value::text("\"\""))]);
    assert_eq!(one(c.send(buy)).request(), Some(0x1C24));
    rows(&one(c.send(req(0x2775))))
}

#[test]
fn a_pack_can_give_another_artwork_of_a_card_it_holds() {
    // Row 10 is the art the packs give; row 11 is another artwork of the same card (an own CARD_Prop row) no pack holds.
    let always = Arts { alts: [(10, vec![11])].into(), odds: 1 };
    assert_eq!(open_one_pack(&pack_world(always)), [11; 5], "one in one: every pull is the other artwork");
    let never = Arts { alts: [(10, vec![11])].into(), odds: 0 };
    assert_eq!(open_one_pack(&pack_world(never)), [10; 5]);
    let none = Arts { alts: Default::default(), odds: 1 };
    assert_eq!(open_one_pack(&pack_world(none)), [10; 5], "a card with one artwork has no other to give");
}

#[test]
fn the_alternate_artwork_is_rare_by_default() {
    let arts = Arts { alts: [(10, vec![11])].into(), odds: yo3_server::shop::ALT_ART_ODDS };
    let mut rng = 12345u64;
    let pulls: Vec<u16> = (0..4000).map(|_| arts.pull(10, &mut rng)).collect();
    let rare = pulls.iter().filter(|&&r| r == 11).count();
    assert!((100..=300).contains(&rare), "about one in {} of 4000: {rare}", yo3_server::shop::ALT_ART_ODDS);
}

// The extra Trade Card pool (pack file 232): a card no booster pack holds has no pack to be named after.

#[test]
fn the_extra_pool_gives_a_card_no_pack_holds_as_a_gift_without_a_pack() {
    let pool = |id, cards: Vec<u16>| ExchangePool { id, released: String::new(), modified: String::new(), cards };
    let server = Server::with_world(World {
        packs: vec![ShopPack { id: 1, released: String::new(), modified: String::new(), tiers: vec![vec![2]] }],
        exchange_pools: vec![pool(264, vec![2]), pool(yo3_server::shop::EXTRA_POOL, vec![2, 40])],
        card_rows: 50,
        ..World::new((1..=20).collect())
    });
    let mut owner = Player::new(1001, "Yugi");
    owner.cards = [(20, 10)].into();
    let mut c = server.join_as(owner);
    let list = one(c.send(req(0x1B6F)));
    assert_eq!(list.items.iter().map(|i| int(i, "item_id")).collect::<Vec<_>>(), [Some(264), Some(232)]);
    let kaban = Value::u16_be_array(&[20; 10]);
    let r = one(c.send(req_with(0x1B6A, &[("sale_id", Value::U32(232)), ("item_id", Value::U32(40)), ("kaban", kaban)])));
    assert_eq!(r.request(), Some(0x1C28));
    let p = one(c.send(req(0x2775)));
    assert_eq!((int(&p, "type"), int(&p, "pack_id")), (Some(0), Some(0)), "a gift of one card, no pack named");
    assert_eq!(rows(&p), [40]);
}

// Official Events: the Special Pack, the titles and the champions.

const ADDR_A: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
const ADDR_B: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

fn titles() -> TitleData {
    let names = [("TRIALIST", 228), ("ULTRA TRIALIST", 504), ("CHAMPION TRIALIST", 493), ("THE ANNIVERSARIAN", 498), ("GUNSLINGER", 229)];
    TitleData { by_name: names.iter().map(|(n, id)| (n.to_string(), *id)).collect(), ..TitleData::default() }
}

fn enter(c: &mut Client) -> u32 {
    let list = one(c.send(req(0x1F49)));
    let player = int(&list.items[0], "player_id").unwrap();
    one(c.send(req_with(0x1F4C, &[("player_id", Value::U32(player))])));
    player
}

struct Event {
    server: Server,
    d: Arc<Mutex<Directory>>,
    a: Client,
    b: Client,
    players: [u32; 2],
    event: u32,
    duel: u32,
    end: i64,
}

/// Two players of a directory entered in an official event of `kind` and matched; the event's end is an hour away.
fn event(kind: Kind) -> Event {
    let server = Server::new();
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into(), "Kaiba".into()])));
    let mut hub = server.hub.lock().unwrap();
    hub.set_directory(d.clone());
    hub.set_titles(Arc::new(titles()));
    hub.set_prizes(Arc::new(Prizes { special: vec![vec![5, 6, 7], vec![8]], ..Prizes::default() }));
    let end = now() + 3600;
    let id = hub.official.create(NewEvent { kind, name: "Test event".into(), start: now() - 60, end, regulation: None, prize: Prize::default() }).unwrap().id;
    drop(hub);
    let (mut a, mut b) = (server.join_from(&d, ADDR_A), server.join_from(&d, ADDR_B));
    let players = [enter(&mut a), enter(&mut b)];
    let join = |c: &mut Client| {
        let joined = one(c.send(req_with(0x1784, &[("event_id", Value::U32(id))])));
        let join_id = int(&joined, "join_id").unwrap();
        one(c.send(req_with(0x1798, &[("event_id", Value::U32(id)), ("join_id", Value::U32(join_id))])));
    };
    join(&mut a);
    join(&mut b);
    let mut duel = 0;
    let start = std::time::Instant::now();
    while duel == 0 && start.elapsed() < std::time::Duration::from_secs(3) {
        std::thread::sleep(std::time::Duration::from_millis(20));
        duel = a.pushed().iter().flat_map(|m| &m.items).find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("duelData"))
            .and_then(|i| int(i, "duel_id")).unwrap_or(0);
    }
    assert_ne!(duel, 0, "the two entrants were matched");
    b.pushed();
    Event { server, d, a, b, players, event: id, duel, end }
}

fn ended(player: u32, opponent: u32, duel: u32, result: u32) -> Ended {
    Ended { player_id: player, opponent, npc: false, duel_id: duel, match_type: 1, result, replay: None, stats: None, start_lp: 8000 }
}

impl Event {
    fn win(&self) {
        let mut hub = self.server.hub.lock().unwrap();
        hub.finish_duel(&ended(self.players[0], self.players[1], self.duel, 1));
        hub.finish_duel(&ended(self.players[1], self.players[0], self.duel, 2));
    }
}

#[test]
fn an_event_win_earns_a_special_pack_as_a_present_and_a_mail() {
    let mut e = event(Kind::DuelistTrial);
    e.win();
    let pushed = e.a.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(0x2774) && int(m, "present_count") == Some(1)), "the present notice");
    assert!(pushed.iter().any(|m| m.request() == Some(0x2581)), "the mail bell");
    let p = one(e.a.send(req(0x2775)));
    assert_eq!((p.request(), int(&p, "type"), int(&p, "pack_id")), (Some(0x2777), Some(0), Some(0)));
    let cards = rows(&p);
    assert_eq!(cards.len(), 5);
    assert!(cards[..4].iter().all(|r| [5, 6, 7].contains(r)), "four of the first tier: {cards:?}");
    assert_eq!(cards[4], 8, "one of the higher tier");
    // The end of the presents brings the trunk so the cards show in Edit Deck.
    assert_eq!(e.a.send(req(0x2775)).iter().map(Message::request).collect::<Vec<_>>(), [Some(0x2778), Some(9000)]);
    // The cards are the player's, kept with the account.
    let stored = e.d.lock().unwrap().player(e.players[0]).unwrap().cards.clone();
    assert_eq!(stored.values().sum::<u32>(), 5);
    let mail = one(e.a.send(req_with(0x2582, &[("count", Value::U32(1))])));
    assert!(mail.items[0].get("title").and_then(Value::as_str).is_some_and(|t| t.contains("Special Pack")), "{mail:?}");
    // A loss earns none.
    let lost = e.b.pushed();
    assert!(!lost.iter().any(|m| m.request() == Some(0x2774)), "the loser has no present");
}

#[test]
fn the_special_packs_of_one_event_are_capped_per_player() {
    let e = event(Kind::DuelistTrial);
    for _ in 0..SPECIAL_PACK_CAP + 3 {
        e.win();
    }
    let stored = e.d.lock().unwrap().player(e.players[0]).unwrap().cards.clone();
    assert_eq!(stored.values().sum::<u32>(), 5 * SPECIAL_PACK_CAP, "{SPECIAL_PACK_CAP} packs of five, not more");
    let waiting = e.server.hub.lock().unwrap().next_host_present(e.players[0]).is_some();
    assert!(waiting);
}

#[test]
fn event_wins_earn_titles_and_first_place_the_champion_title_once_the_event_ends() {
    let mut e = event(Kind::DuelistTrial);
    let owns = |e: &Event, player: u32, title: u16| e.d.lock().unwrap().player(player).unwrap().items.contains(&title);
    e.win();
    e.win();
    assert!(!owns(&e, e.players[0], 228), "two wins are not three");
    e.win();
    assert!(owns(&e, e.players[0], 228), "TRIALIST: 3 wins in the Duelist Trial (the client's line)");
    let lines: Vec<String> = e.a.pushed().iter().filter(|m| m.request() == Some(0x2455))
        .filter_map(|m| m.items.first()?.get("chat_str")?.as_str().map(|s| s.to_string())).collect();
    assert!(lines.iter().any(|l| l.contains("TRIALIST")), "{lines:?}");
    assert!(!owns(&e, e.players[1], 228), "a loser earns nothing");
    assert!(!owns(&e, e.players[0], 493), "no champion before the event is over");
    e.server.hub.lock().unwrap().tick_official(e.end - 1);
    assert!(!owns(&e, e.players[0], 493));
    e.server.hub.lock().unwrap().tick_official(e.end);
    assert!(owns(&e, e.players[0], 493), "CHAMPION TRIALIST: first place of an event of two");
    assert!(!owns(&e, e.players[1], 493));
    let _ = e.event;
}

#[test]
fn an_anniversary_win_earns_its_title_and_a_gunslingers_streak_counts_at_its_best() {
    let e = event(Kind::Anniversary);
    e.win();
    let items = e.d.lock().unwrap().player(e.players[0]).unwrap().items.clone();
    assert!(items.contains(&498), "THE ANNIVERSARIAN");
    let g = event(Kind::Gunslinger);
    for _ in 0..3 {
        g.win();
    }
    let items = g.d.lock().unwrap().player(g.players[0]).unwrap().items.clone();
    assert!(items.contains(&229), "GUNSLINGER: a 3 win streak");
    assert_eq!(g.d.lock().unwrap().player(g.players[0]).unwrap().counters.get("official_streak"), Some(&3));
}

// Deck recipes from NPCs (#19).

#[test]
fn the_first_win_against_an_npc_gives_the_recipe_of_its_deck_and_a_mail() {
    let server = Server::new();
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let recipes = [(130, NpcRecipe { item: 900, npc: "Yami Marik".into(), deck: "DARK RITUAL".into() })].into();
    {
        let mut hub = server.hub.lock().unwrap();
        hub.set_directory(d.clone());
        hub.set_prizes(Arc::new(Prizes { npc_recipes: recipes, ..Prizes::default() }));
    }
    let mut c = server.join_from(&d, ADDR_A);
    let player = enter(&mut c);
    let npc = |duel_id, result, opponent| Ended { npc: true, ..ended(player, opponent, duel_id, result) };
    let owns = |c: &mut Client| {
        let me = one(c.send(req(0x2CEE))).items.remove(0);
        me.get("pbin").and_then(Value::as_bytes).is_some_and(|b| b[900 >> 3] & 1 << (900 & 7) != 0)
    };
    server.hub.lock().unwrap().finish_duel(&npc(1, 2, 130));
    assert!(!owns(&mut c) && !d.lock().unwrap().player(player).unwrap().items.contains(&900), "a loss gives nothing");
    server.hub.lock().unwrap().finish_duel(&npc(2, 1, 131));
    assert!(!d.lock().unwrap().player(player).unwrap().items.contains(&900), "another NPC's deck is another recipe");
    c.pushed();
    server.hub.lock().unwrap().finish_duel(&npc(3, 1, 130));
    assert!(d.lock().unwrap().player(player).unwrap().items.contains(&900), "kept with the account");
    assert!(owns(&mut c), "on Load Deck's Official tab (the item's bit in the pbin)");
    let pushed = c.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(0x2581)), "the mail bell");
    let mail = one(c.send(req_with(0x2582, &[("count", Value::U32(1))])));
    assert!(mail.items[0].get("body").and_then(Value::as_str).is_some_and(|b| b.contains("DARK RITUAL") && b.contains("Yami Marik")), "{mail:?}");
    // Once: a second win says nothing new.
    server.hub.lock().unwrap().finish_duel(&npc(4, 1, 130));
    assert!(!c.pushed().iter().any(|m| m.request() == Some(0x2581)));
}
