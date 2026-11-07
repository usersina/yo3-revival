//! The shop (shopmain.cpp, reply handler 0x48AA60), the map's top-left facility. After its trunk
//! reply it asks for its text (0x1B66, sent at 0x48BEDD), and the text reply asks for the items
//! (0x1B67, 0x4F3590). Requests and replies pair by +0xBA. Neither reply has a global case.
mod common;

use common::{Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::front::World;
use yo3_server::shop::{self, ShopPack};

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

/// Two packs: pack 1 in three rarity tiers, pack 271 in one.
fn shop() -> Server {
    let pack = |id, tiers: Vec<Vec<u16>>| ShopPack {
        id,
        released: "2009-11-02 18:42:28".into(),
        modified: "2009-11-24 10:24:51".into(),
        tiers,
    };
    let packs = vec![pack(1, vec![vec![10, 11, 12, 13], vec![20, 21], vec![30]]), pack(271, vec![vec![40, 41, 42]])];
    Server::with_world(World { packs, card_rows: 50, ..World::new((1..=20).collect()) })
}

#[test]
fn the_shelves_carry_the_booster_packs() {
    // 0x1C22 items (0x485E50): item_type 0 (a pack: kinds 0 and 3 go to AddPack, 0x48C790, which takes the name,
    // description and image from the client's own pack file item_id), item_id, item_cost, item_max, sale_id, sale,
    // item_trade, and the dates modified and rel_time as text.
    let mut c = shop().join(1001, "Yugi");
    let r = one(c.send(req(0x1B67)));
    assert_eq!(r.request(), Some(0x1C22));
    assert_eq!(r.items.len(), 2);
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
