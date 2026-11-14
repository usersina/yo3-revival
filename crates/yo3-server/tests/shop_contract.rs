//! The shop (shopmain.cpp, reply handler 0x48AA60), the map's top-left facility. After its trunk
//! reply it asks for its text (0x1B66, sent at 0x48BEDD), and the text reply asks for the items
//! (0x1B67, 0x4F3590). Requests and replies pair by +0xBA. Neither reply has a global case.
mod common;

use common::{Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::front::{Deck, Player, World};
use yo3_server::shop::{self, ExchangePool, ShopDeck, ShopPack};

#[test]
fn shop_text_carries_the_greeting() {
    // 0x1C20 case: "text" (string) is shown in the shopkeeper's bubble. The case also reads evalnum,
    // evaltotal and evallevel, but from the player's record (UserData+0x2C), not from this reply.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x1B66)));
    assert_eq!(r.request(), Some(0x1C20));
    assert!(r.get("text").and_then(Value::as_str).is_some_and(|t| !t.is_empty()));
    assert_eq!(r.fields.len(), 2, "request and text only: {r:?}");
}

#[test]
fn every_catalogue_request_gets_its_list_even_when_empty() {
    // Each case checks the item count (0x410D90) before reading items, then asks for the next list:
    // 0x1C22 items (the packs on sale; none in this world) -> 0x1B6F (0x48AE5F); 0x1C26 exchange items {item_id,
    // item_cost, sale_id, ...} -> 0x1B6B (0x48B3B8); 0x1C32 protectors {item_id} -> 0x1B6D
    // (0x48B4C3); 0x1C2E event items; 0x1C2A avatar items {item_type, item_param}, the last, which
    // clears isLoading.
    let mut c = Server::new().join(1001, "Yugi");
    let chain = [(0x1B67, 0x1C22), (0x1B6F, 0x1C26), (0x1B6B, 0x1C32), (0x1B6D, 0x1C2E), (0x1B69, 0x1C2A)];
    for (request, reply) in chain {
        let r = one(c.send(req(request)));
        assert_eq!(r.request(), Some(reply), "{request:#x}");
        assert!(r.items.is_empty());
    }
}

#[test]
fn the_last_request_gets_the_perk_count() {
    // The event-items case sends 0x1B72 (0x48B64D); 0x1C36 (0x48B68D) reads perk_num and compares
    // it with 500 to show or hide _root.expack_del. Meaning unknown; 0 is a guess.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x1B72)));
    assert_eq!(r.request(), Some(0x1C36));
    assert_eq!(r.get("perk_num"), Some(&Value::U32(0)));
}

#[test]
fn rental_terms_are_answered() {
    // After 0x1C36 the shop sends 0x2C40 (0x48B777); 0x2D75 (0x48C20E) reads rental_cardnum,
    // rental_point and rental_term from the reply, then asks for the avatar items (0x1B69, 0x48C391).
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x2C40)));
    assert_eq!(r.request(), Some(0x2D75));
    for k in ["rental_cardnum", "rental_point", "rental_term"] {
        assert_eq!(r.get(k), Some(&Value::U32(0)), "{k}");
    }
}

/// Two packs (pack 1 in three rarity tiers, pack 271 in one) and structure deck 4.
fn shop() -> Server {
    let pack = |id, tiers: Vec<Vec<u16>>| ShopPack {
        id,
        released: "2009-11-02 18:42:28".into(),
        modified: "2009-11-24 10:24:51".into(),
        tiers,
    };
    let packs = vec![pack(1, vec![vec![10, 11, 12, 13], vec![20, 21], vec![30]]), pack(271, vec![vec![40, 41, 42]])];
    let deck = Deck { id: 4, name: "Equip to Win!".into(), main: vec![40, 40], extra: vec![41], side: vec![] };
    let deck = ShopDeck { released: "2009-11-26 16:26:17".into(), modified: "2009-11-26 16:26:17".into(), deck, item: Some(2560) };
    Server::with_world(World { packs, structure_decks: vec![deck], card_rows: 50, ..World::new((1..=20).collect()) })
}

#[test]
fn the_shelves_carry_the_booster_packs() {
    // 0x1C22 items (0x485E50): item_type 0 (a pack: kinds 0 and 3 go to AddPack, 0x48C790, which takes the name,
    // description and image from the client's own pack file item_id), item_id, item_cost, item_max, sale_id, sale,
    // item_trade, and the dates modified and rel_time as text.
    let mut c = shop().join(1001, "Yugi");
    let r = one(c.send(req(0x1B67)));
    assert_eq!(r.request(), Some(0x1C22));
    assert_eq!(r.items.iter().filter(|i| i.get("item_type") == Some(&Value::U32(0))).count(), 2);
    let item = &r.items[0];
    let int = |k| item.get(k).and_then(Value::as_u32);
    let fields = ["item_type", "item_id", "item_cost", "item_max", "sale_id", "sale", "item_trade"].map(int);
    assert_eq!(fields, [Some(0), Some(1), Some(100), Some(10), Some(1), Some(0), Some(0)]);
    assert_eq!(item.get("rel_time").and_then(Value::as_str).as_deref(), Some("2009-11-02 18:42:28"));
    assert_eq!(item.get("modified").and_then(Value::as_str).as_deref(), Some("2009-11-24 10:24:51"));
    assert_eq!(r.items[1].get("item_id").and_then(Value::as_u32), Some(271));
}

/// The rows of a 0x2777 present's item (big-endian u16s).
fn rows(present: &Message) -> Vec<u16> {
    let bytes = present.get("item").and_then(Value::as_bytes).expect("an item");
    bytes.chunks(2).map(|b| u16::from_be_bytes([b[0], b[1]])).collect()
}

#[test]
fn bought_packs_cost_bp_and_arrive_as_presents() {
    // 0x1B68 {sale_id, item_num, end_time} -> 0x1C24 {dpoint}: the BP left (shown as user_bp). The client then asks
    // for presents (0x2775) until 0x2778: each pack is a 0x2777 (PresentWindow 0x476DE0) {present_id, type 0, level,
    // form, item_img, end_time, item: the cards as big-endian u16 rows, pack_id}, shown one card at a time. The cards
    // join the player's collection, which the next trunk (0x1FA4) brings.
    let mut c = shop().join_with_bp(1001, "Yugi", 1000);
    let buy = req_with(0x1B68, &[("sale_id", Value::U32(1)), ("item_num", Value::U32(2)), ("end_time", Value::text("\"\""))]);
    let r = one(c.send(buy));
    assert_eq!((r.request(), r.get("dpoint")), (Some(0x1C24), Some(&Value::U32(800))));
    let mut pulled = vec![];
    for n in 1..=2 {
        let p = one(c.send(req(0x2775)));
        assert_eq!(p.request(), Some(0x2777));
        let int = |k| p.get(k).and_then(Value::as_u32);
        assert_eq!(["present_id", "type", "pack_id"].map(int), [Some(n), Some(0), Some(1)]);
        for k in ["level", "form"] {
            assert!(p.get(k).is_some(), "{k}");
        }
        assert!(p.get("item_img").and_then(Value::as_str).is_some() && p.get("end_time").and_then(Value::as_str).is_some());
        let cards = rows(&p);
        assert_eq!(cards.len(), 5);
        assert!(cards[..4].iter().all(|r| (10..=13).contains(r)), "four of the first tier: {cards:?}");
        assert!([20, 21, 30].contains(&cards[4]), "one of a higher tier: {cards:?}");
        pulled.extend(cards);
    }
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778));
    // The trunk counts them (cards: one byte per row at row - 1).
    let trunk = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
    let counts = trunk[0].items[0].get("cards").and_then(Value::as_bytes).unwrap().to_vec();
    for row in [10u16, 11, 12, 13, 20, 21, 30] {
        let n = pulled.iter().filter(|&&r| r == row).count();
        assert_eq!(counts[row as usize - 1] as usize, n, "row {row}");
    }
}

#[test]
fn a_purchase_beyond_the_bp_or_the_shelves_fails() {
    // 0x1C25 is "shop buy error"; nothing is spent and no present comes.
    let mut c = shop().join_with_bp(1001, "Yugi", 150);
    let buy = |sale_id, item_num| req_with(0x1B68, &[("sale_id", Value::U32(sale_id)), ("item_num", Value::U32(item_num))]);
    assert_eq!(one(c.send(buy(1, 2))).request(), Some(0x1C25));
    assert_eq!(one(c.send(buy(99, 1))).request(), Some(0x1C25));
    assert_eq!(one(c.send(buy(1, 0))).request(), Some(0x1C25));
    // A count whose price overflows u32 (a panic in a debug build) is just refused.
    assert_eq!(one(c.send(buy(1, u32::MAX))).request(), Some(0x1C25));
    assert_eq!(one(c.send(buy(271, u32::MAX / 2))).request(), Some(0x1C25));
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778));
    assert_eq!(one(c.send(buy(271, 1))).get("dpoint"), Some(&Value::U32(50)));
}

#[test]
fn a_pack_holds_four_of_its_first_tier_and_one_rarer_card() {
    // Pack files list their cards in ascending runs (44, 18, 9, 9 for an 80-card pack: the OCG's Normal, Rare, Super
    // and Ultra Rares), read as rarity tiers (guess); a one-tier pack gives five of it.
    assert_eq!(shop::tiers(&[38, 99, 142, 852, 880, 31, 131, 264]), [vec![38, 99, 142, 852, 880], vec![31, 131, 264]]);
    let pack = ShopPack { id: 1, released: String::new(), modified: String::new(), tiers: vec![vec![1, 2], vec![3], vec![4]] };
    let mut rng = 7;
    let mut higher = std::collections::HashSet::new();
    for _ in 0..200 {
        let cards = shop::open(&pack, &mut rng);
        assert_eq!(cards.len(), 5);
        assert!(cards[..4].iter().all(|r| [1, 2].contains(r)), "{cards:?}");
        higher.insert(cards[4]);
    }
    assert_eq!(higher, [3, 4].into(), "every higher tier comes up");
    let one_tier = ShopPack { tiers: vec![vec![5, 6]], ..pack };
    assert!(shop::open(&one_tier, &mut rng).iter().all(|r| [5, 6].contains(r)));
}

#[test]
fn duels_earn_bp_that_the_next_purchase_spends() {
    // The client's BP (UserData +0x76C) is set only by the login's dpoint, 0x1C24, 0x2D77 (rental) and 0x23F0 (the
    // BOOSTERPASS code window): nothing after a duel. The server pays each counted duel result into the account (win
    // 100, draw 50, loss 30, a guess) and re-reads the account for each purchase, whose 0x1C24 brings the new total.
    use std::sync::{Arc, Mutex};
    use yo3_server::accounts::{Directory, START_BP};
    let server = shop();
    let directory = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into(), "Kaiba".into()])));
    server.hub.lock().unwrap().set_directory(directory.clone());
    let mut c = server.join_from(&directory, std::net::IpAddr::from([10, 0, 0, 3]));
    let players: Vec<u32> = directory.lock().unwrap().accounts().flat_map(|(_, a)| a.players.iter().map(|p| p.player_id)).collect();
    let (yugi, kaiba) = (players[0], players[1]);
    let mut hub = server.hub.lock().unwrap();
    hub.record_result(yugi, kaiba, 1);
    hub.record_result(kaiba, yugi, 2);
    hub.record_result(yugi, kaiba, 3);
    drop(hub);
    let bp = || directory.lock().unwrap().accounts().next().map(|(_, a)| a.dpoint);
    assert_eq!(bp(), Some(START_BP + 100 + 30 + 50));
    let buy = req_with(0x1B68, &[("sale_id", Value::U32(1)), ("item_num", Value::U32(1))]);
    assert_eq!(one(c.send(buy)).get("dpoint"), Some(&Value::U32(START_BP + 180 - 100)));
    assert_eq!(bp(), Some(START_BP + 80));
}

#[test]
fn structure_decks_sit_on_their_own_tab() {
    // Kinds 1 and 4 go to the STRUCTURE DECK tab (AddStruct, 0x48CE40), which takes the name, description, art and
    // date from the client's preset deck item_id (0x52DC10). Pack and deck ids overlap, so a deck's sale id is
    // 20000 + its id.
    let mut c = shop().join(1001, "Yugi");
    let r = one(c.send(req(0x1B67)));
    let deck = r.items.iter().find(|i| i.get("item_type") == Some(&Value::U32(1))).expect("a structure deck");
    let int = |k| deck.get(k).and_then(Value::as_u32);
    assert_eq!(["item_id", "item_cost", "item_max", "sale_id"].map(int), [Some(4), Some(500), Some(3), Some(20004)]);
    assert_eq!(deck.get("rel_time").and_then(Value::as_str).as_deref(), Some("2009-11-26 16:26:17"));
}

#[test]
fn a_bought_structure_deck_arrives_whole() {
    // A structure deck present is type 1 (PresentWindow case 1, 0x4777D3): the deck id is item's first u16, and desc
    // "-1" shows the deck's own description. Its cards join the collection.
    let mut c = shop().join_with_bp(1001, "Yugi", 1000);
    let buy = req_with(0x1B68, &[("sale_id", Value::U32(20004)), ("item_num", Value::U32(1))]);
    assert_eq!(one(c.send(buy)).get("dpoint"), Some(&Value::U32(500)));
    let p = one(c.send(req(0x2775)));
    assert_eq!(p.request(), Some(0x2777));
    assert_eq!(["present_id", "type"].map(|k| p.get(k).and_then(Value::as_u32)), [Some(1), Some(1)]);
    assert_eq!(rows(&p), [4]);
    assert_eq!(p.get("desc").and_then(Value::as_str).as_deref(), Some("-1"));
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778));
    let trunk = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
    let counts = trunk[0].items[0].get("cards").and_then(Value::as_bytes).unwrap().to_vec();
    assert_eq!((counts[39], counts[40]), (2, 1));
    // Beyond item_max (3) or the BP: refused.
    let four = req_with(0x1B68, &[("sale_id", Value::U32(20004)), ("item_num", Value::U32(4))]);
    assert_eq!(one(c.send(four)).request(), Some(0x1C25));
}

#[test]
fn a_bought_structure_deck_is_on_the_official_tab() {
    // A bought structure deck's present sets its recipe's item bit (pbin_no) in the client's own copy (PresentWindow
    // case 1); the server keeps it in the player's record, so Load Deck's Official tab (0x43D8D0, catalogue kind 5)
    // lists the deck after every login, from the client's own recipe file. It is not one of the player's own decks
    // (0x22C8); a deck bought twice is one item.
    let mut c = shop().join_with_bp(1001, "Yugi", 2000);
    assert!(!has_bit(&one(c.send(req(0x2CEE))).items.remove(0), 2560));
    for _ in 0..2 {
        let buy = req_with(0x1B68, &[("sale_id", Value::U32(20004)), ("item_num", Value::U32(1))]);
        assert_eq!(one(c.send(buy)).request(), Some(0x1C24));
    }
    assert!(one(c.send(req(0x1F4E))).items.is_empty());
    assert!(has_bit(&one(c.send(req(0x2CEE))).items.remove(0), 2560));
}

/// The shop with two protectors (pbin ids 2305, 2306) and one avatar item (pbin id 6, "Bald", text 26006).
fn item_shop() -> Server {
    Server::with_world(World { protectors: vec![2305, 2306], avatar_items: vec![(6, 26006)], ..World::new((1..=20).collect()) })
}

fn has_bit(record: &Message, bit: usize) -> bool {
    let pbin = record.get("pbin").and_then(Value::as_bytes).expect("pbin");
    pbin[bit >> 3] & 1 << (bit & 7) != 0
}

#[test]
fn protectors_and_avatar_items_sit_on_their_shelves() {
    // 0x1B6B -> 0x1C32: card protectors, item_type 6 (AddProtector 0x48D520), item_id the pbin id (2305-2512 in the
    // catalogue, kind 11); the client leaves out the ones the player's pbin owns. 0x1B69 -> 0x1C2A: avatar items,
    // item_type above 2 with item_param the pbin id, which the client leaves out once owned; it sorts them into tabs by
    // id (0x496A50). Both go through 0x485E50 like the packs. Prices are ours (**guess**).
    let mut c = item_shop().join(1001, "Yugi");
    let r = one(c.send(req(0x1B6B)));
    assert_eq!((r.request(), r.items.len()), (Some(0x1C32), 2));
    let int = |m: &Message, k| m.get(k).and_then(Value::as_u32);
    let p = &r.items[0];
    assert_eq!(["item_type", "item_id", "item_cost", "item_max", "sale", "item_trade"].map(|k| int(p, k)), [Some(6), Some(2305), Some(200), Some(1), Some(0), Some(0)]);
    assert!(p.get("modified").and_then(Value::as_str).is_some() && p.get("rel_time").and_then(Value::as_str).is_some());
    let r = one(c.send(req(0x1B69)));
    assert_eq!((r.request(), r.items.len()), (Some(0x1C2A), 1));
    let a = &r.items[0];
    assert_eq!(["item_type", "item_param", "item_id", "item_cost", "item_max"].map(|k| int(a, k)), [Some(3), Some(6), Some(6), Some(100), Some(1)]);
    assert_ne!(int(a, "sale_id"), int(p, "sale_id"));
}

#[test]
fn a_bought_protector_or_avatar_item_is_the_players() {
    // The purchase is 0x1B68 {sale_id, item_num 1} -> 0x1C24 {dpoint}, then the present: type 7 for a protector
    // (PresentWindow case 7, 0x47800E: "Protector") and 6 for an avatar item (with name_id, the item's name text), item
    // the pbin id as a big-endian u16; the client sets the pbin bit itself. The server keeps it in the player's pbin
    // (the deck editor's protector list 0x47E490, Avatar Settings), and a second purchase fails.
    let mut c = item_shop().join_with_bp(1001, "Yugi", 1000);
    let sale = |c: &mut common::Client, item_type| {
        let shelf = one(c.send(req(if item_type == 6 { 0x1B6B } else { 0x1B69 })));
        shelf.items[0].get("sale_id").and_then(Value::as_u32).unwrap()
    };
    let protector = sale(&mut c, 6);
    let r = one(c.send(req_with(0x1B68, &[("sale_id", Value::U32(protector)), ("item_num", Value::U32(1))])));
    assert_eq!((r.request(), r.get("dpoint")), (Some(0x1C24), Some(&Value::U32(800))));
    let p = one(c.send(req(0x2775)));
    assert_eq!((p.get("type"), rows(&p)), (Some(&Value::U32(7)), vec![2305]));
    let avatar = sale(&mut c, 3);
    let r = one(c.send(req_with(0x1B68, &[("sale_id", Value::U32(avatar)), ("item_num", Value::U32(1))])));
    assert_eq!(r.get("dpoint"), Some(&Value::U32(700)));
    let p = one(c.send(req(0x2775)));
    assert_eq!((p.get("type"), rows(&p), p.get("name_id")), (Some(&Value::U32(6)), vec![6], Some(&Value::U32(26006))));
    let me = one(c.send(req(0x2CEE))).items.remove(0);
    assert!(has_bit(&me, 2305) && has_bit(&me, 6));
    let again = one(c.send(req_with(0x1B68, &[("sale_id", Value::U32(protector)), ("item_num", Value::U32(1))])));
    assert_eq!(again.request(), Some(0x1C25));
}

/// A stored account with one player: the directory, its folder and the player's id.
fn stored_account(name: &str) -> (yo3_server::accounts::Directory, std::path::PathBuf, u32) {
    let dir = std::env::temp_dir().join(format!("yo3-purchase-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let mut d = yo3_server::accounts::Directory::open(vec!["Yugi".into()], &dir.join("accounts.json")).unwrap();
    d.login(std::net::IpAddr::from([10, 0, 0, 9]), std::time::Instant::now());
    let player = d.accounts().next().unwrap().1.players[0].player_id;
    (d, dir, player)
}

#[test]
fn a_purchase_that_cannot_be_saved_takes_no_bp_and_gives_no_cards() {
    // The BP and the cards used to be two saves: a failure between them took the BP and gave nothing.
    use yo3_server::accounts::START_BP;
    let (mut d, dir, player) = stored_account("unsaved");
    std::fs::remove_file(dir.join("server.db")).unwrap();
    assert!(d.purchase(player, 100, None, |p| { p.cards.insert(10, 1); }).is_err());
    assert_eq!(d.accounts().next().unwrap().1.dpoint, START_BP);
    assert!(!d.player(player).unwrap().cards.contains_key(&10));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_item_is_bought_once_per_player_whatever_the_session_saw() {
    // Ownership is checked on the stored player: two sessions of one account cannot both buy it.
    let (mut d, dir, player) = stored_account("item-once");
    let item = |p: &mut yo3_server::front::Player| { p.items.insert(2305); };
    assert!(d.purchase(player, 200, Some(2305), item).is_ok());
    let left = d.accounts().next().unwrap().1.dpoint;
    assert_eq!(d.purchase(player, 200, Some(2305), item).unwrap_err(), "already owned");
    assert_eq!(d.accounts().next().unwrap().1.dpoint, left, "no BP taken for a refused purchase");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bp_given_during_a_session_can_be_spent_at_once() {
    // The host's give-bp changes the stored account; the shop charges the stored amount, not the session's copy.
    use yo3_server::accounts::START_BP;
    let (mut d, dir, player) = stored_account("gift");
    d.give_bp("Yugi", 500).unwrap();
    let (left, stored) = d.purchase(player, START_BP + 400, None, |p| { p.cards.insert(10, 1); }).unwrap();
    assert_eq!((left, stored.cards.get(&10)), (100, Some(&1)));
    std::fs::remove_dir_all(dir).unwrap();
}

/// Blister's counters: exchange pools 264 (rows 2, 10, 11) and 265 (row 40), whose cards first came in packs 1 and 271,
/// and the rental pool of rows 2 to 9.
fn blister() -> Server {
    let pool = |id, cards: Vec<u16>| ExchangePool { id, released: "2010-03-23 17:19:41".into(), modified: "2010-03-23 17:19:41".into(), cards };
    let pack = |id, tiers: Vec<Vec<u16>>| ShopPack { id, released: String::new(), modified: String::new(), tiers };
    Server::with_world(World {
        packs: vec![pack(1, vec![vec![2, 10, 11]]), pack(271, vec![vec![40]])],
        exchange_pools: vec![pool(264, vec![2, 10, 11]), pool(265, vec![40])],
        rental_pool: (2..=9).collect(),
        card_rows: 50,
        ..World::new((1..=20).collect())
    })
}

/// A player owning `n` copies of each of `rows`.
fn owner(rows: &[u16], n: u32) -> Player {
    let mut p = Player::new(1001, "Yugi");
    p.cards = rows.iter().map(|&r| (r, n)).collect();
    p
}

fn bin(rows: &[u16]) -> Value {
    Value::u16_be_array(rows)
}

fn trunk_count(c: &mut common::Client, row: u16) -> u8 {
    let trunk = c.send(req_with(0x1FA4, &[("player_id", Value::U32(1001))]));
    trunk[0].items[0].get("cards").and_then(Value::as_bytes).unwrap()[row as usize - 1]
}

#[test]
fn the_trade_card_list_carries_the_exchange_pools() {
    // 0x1C26 items (case 0x48AE5F's reply): the client loads its own pack file item_id and lists one card per row, each
    // at item_cost of the player's cards ("Cards Required:"), with sale_id, item_trade, item_type, modified and rel_time.
    // Live: the client's pools 264-268 listed 2,612 cards at 10 each.
    let mut c = blister().join(1001, "Yugi");
    let r = one(c.send(req(0x1B6F)));
    assert_eq!(r.request(), Some(0x1C26));
    assert_eq!(r.items.len(), 2);
    let int = |k| r.items[0].get(k).and_then(Value::as_u32);
    assert_eq!(["item_id", "sale_id", "item_cost", "item_trade", "item_type"].map(int), [Some(264), Some(264), Some(shop::EXCHANGE_COST), Some(0), Some(0)]);
    for k in ["modified", "rel_time"] {
        assert!(r.items[0].get(k).and_then(Value::as_str).is_some(), "{k}");
    }
}

#[test]
fn an_exchange_gives_the_card_picked_for_the_cost_in_cards() {
    // 0x1B6A {sale_id: the pool, item_id: the card's row, kaban: the player's cards as big-endian u16 rows} (live:
    // sale_id 264, item_id 2, kaban 20 bytes) -> 0x1C28 with no fields; the client then asks for presents (0x47C4A0).
    // The card comes as a 0x2777 of type 0 named after the pack it first came in (the pool's own file has no name:
    // live, the window showed "Card" over "Attack"; with pack 5, "Pack 5").
    let mut c = blister().join_as(owner(&[20, 21], 5));
    let given = [20, 20, 20, 20, 20, 21, 21, 21, 21, 21];
    let r = one(c.send(req_with(0x1B6A, &[("sale_id", Value::U32(264)), ("item_id", Value::U32(2)), ("kaban", bin(&given))])));
    assert_eq!(r.request(), Some(0x1C28));
    assert_eq!(r.fields.len(), 1, "no fields: {r:?}");
    let p = one(c.send(req(0x2775)));
    let int = |k| p.get(k).and_then(Value::as_u32);
    assert_eq!(["type", "pack_id"].map(int), [Some(0), Some(1)]);
    assert_eq!(rows(&p), [2]);
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778));
    assert_eq!([2, 20, 21].map(|row| trunk_count(&mut c, row)), [1, 0, 0]);
}

#[test]
fn an_exchange_needs_the_exact_cost_in_cards_the_player_owns() {
    // 0x1C29 (an error dialog, 0x493D30) for a pool or card not on the list, a count other than item_cost, an odd byte,
    // or cards the player doesn't own; nothing changes hands.
    let mut c = blister().join_as(owner(&[20], 10));
    let ten = [20u16; 10];
    let exchange = |sale: u32, card: u32, kaban: Value| req_with(0x1B6A, &[("sale_id", Value::U32(sale)), ("item_id", Value::U32(card)), ("kaban", kaban)]);
    let refused = [
        exchange(266, 2, bin(&ten)),
        exchange(264, 40, bin(&ten)),
        exchange(264, 2, bin(&ten[..9])),
        exchange(264, 2, bin(&[20; 11])),
        exchange(264, 2, Value::Bin([bin(&ten).as_bytes().unwrap(), &[0][..]].concat())),
        exchange(264, 2, bin(&[[20u16; 9].as_slice(), &[21]].concat())),
        req_with(0x1B6A, &[("sale_id", Value::U32(264)), ("item_id", Value::U32(2))]),
    ];
    for (i, r) in refused.into_iter().enumerate() {
        assert_eq!(one(c.send(r)).request(), Some(0x1C29), "case {i}");
    }
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778));
    assert_eq!([2, 20].map(|row| trunk_count(&mut c, row)), [0, 10]);
}

#[test]
fn an_exchange_checks_the_stored_cards() {
    // Ownership is checked on the stored player under the accounts' lock (as trades do): a card the account no longer
    // has can't be given, whatever the session saw.
    use std::sync::{Arc, Mutex};
    use yo3_server::accounts::Directory;
    let server = blister();
    let directory = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    server.hub.lock().unwrap().set_directory(directory.clone());
    let mut c = server.join_from(&directory, std::net::IpAddr::from([10, 0, 0, 4]));
    let player = directory.lock().unwrap().accounts().next().unwrap().1.players[0].player_id;
    let exchange = || req_with(0x1B6A, &[("sale_id", Value::U32(265)), ("item_id", Value::U32(40)), ("kaban", bin(&[20; 10]))]);
    assert_eq!(one(c.send(exchange())).request(), Some(0x1C29));
    directory.lock().unwrap().update_players(vec![(player, Box::new(|p: &mut Player| { p.cards.insert(20, 10); }))]);
    assert_eq!(one(c.send(exchange())).request(), Some(0x1C28));
    let stored = directory.lock().unwrap().player(player).cloned().unwrap();
    assert_eq!((stored.cards.get(&20), stored.cards.get(&40)), (None, Some(&1)));
    assert_eq!(one(c.send(exchange())).request(), Some(0x1C29), "the ten cards are gone");
}

#[test]
fn the_shop_offers_a_rental_of_its_pool() {
    // 0x2D75 (0x48C20E): "You can borrow %d cards for %dBP for 7 days." (text 15081); with the client's rental pool
    // (pack 10000) the shop offers RENTAL_CARDS cards for RENTAL_BP for RENTAL_DAYS days. Live: "You can borrow 5 cards
    // for 100BP for 7 days."
    let mut c = blister().join(1001, "Yugi");
    let r = one(c.send(req(0x2C40)));
    let int = |k| r.get(k).and_then(Value::as_u32);
    assert_eq!(["rental_cardnum", "rental_point", "rental_term"].map(int), [Some(shop::RENTAL_CARDS), Some(shop::RENTAL_BP), Some(shop::RENTAL_DAYS)]);
}

#[test]
fn a_rental_from_the_shop_costs_bp_and_lends_the_cards() {
    // 0x2C41 {main_list: big-endian u16 rows} -> 0x2D77 {dpoint} (the client sets its BP from it). Live, the client then
    // asked for the rental records (0x2C34): one record from the shop (from_type 1, which the Rental Card List shows as
    // "<Shop>", text 14266) with the cards as main, which the client adds to the collection.
    let mut c = blister().join_with_bp(1001, "Yugi", 250);
    let rent = |rows: &[u16]| req_with(0x2C41, &[("main_list", bin(rows))]);
    let r = one(c.send(rent(&[2, 3, 4, 5, 5])));
    assert_eq!((r.request(), r.get("dpoint")), (Some(0x2D77), Some(&Value::U32(250 - shop::RENTAL_BP))));
    let records = one(c.send(req(0x2C34)));
    assert_eq!(records.request(), Some(0x2D6C));
    assert_eq!(records.items.len(), 1);
    let record = &records.items[0];
    let int = |k| record.get(k).and_then(Value::as_u32);
    assert_eq!(["from_type", "from_id", "to_id", "deck_type", "confirm_to"].map(int), [Some(1), Some(0), Some(1001), Some(0), Some(0)]);
    assert_eq!(rows_of(record, "main"), [2, 3, 4, 5, 5]);
    // A second rental returns the first ("current borrowed cards will be returned automatically!", text 15082).
    assert_eq!(one(c.send(rent(&[6, 7, 8, 9, 9]))).get("dpoint"), Some(&Value::U32(250 - 2 * shop::RENTAL_BP)));
    let records = one(c.send(req(0x2C34)));
    assert_eq!(records.items.len(), 1);
    assert_eq!(rows_of(&records.items[0], "main"), [6, 7, 8, 9, 9]);
    // The borrower's history keeps both.
    let history = one(c.send(req_with(0x2C35, &[("rental_kind", Value::U32(1))])));
    assert_eq!(history.items.len(), 2);
}

fn rows_of(record: &Message, key: &str) -> Vec<u16> {
    record.get(key).and_then(Value::as_bytes).unwrap().chunks(2).map(|b| u16::from_be_bytes([b[0], b[1]])).collect()
}

#[test]
fn a_rental_from_the_shop_needs_the_offer_the_pool_and_the_bp() {
    // 0x2D78 (shopmain's error case) for another count than rental_cardnum, a card outside the pool, an odd byte or too
    // few BP; no BP is taken and nothing is lent.
    let mut c = blister().join_with_bp(1001, "Yugi", 150);
    let rent = |main: Value| req_with(0x2C41, &[("main_list", main)]);
    let refused = [
        rent(bin(&[2, 3, 4, 5])),
        rent(bin(&[2, 3, 4, 5, 6, 7])),
        rent(bin(&[2, 3, 4, 5, 40])),
        rent(Value::Bin(vec![0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0])),
        req(0x2C41),
    ];
    for (i, r) in refused.into_iter().enumerate() {
        assert_eq!(one(c.send(r)).request(), Some(0x2D78), "case {i}");
    }
    assert!(one(c.send(req(0x2C34))).items.is_empty());
    assert_eq!(one(c.send(rent(bin(&[2, 3, 4, 5, 6])))).get("dpoint"), Some(&Value::U32(50)));
    assert_eq!(one(c.send(rent(bin(&[2, 3, 4, 5, 6])))).request(), Some(0x2D78), "50 BP left");
    assert_eq!(one(c.send(req(0x2C34))).items.len(), 1);
}
