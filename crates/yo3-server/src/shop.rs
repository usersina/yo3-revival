//! The shop's stock (shopmain.cpp): booster packs from the client's own pack files, what a pack holds, and the
//! presents bought packs arrive as (PresentWindow).
use yo3_proto::{Message, Value};

/// BOOSTERPASS Points a pack costs, and the most one purchase may take (item_cost, item_max; **guess**: the
/// original prices are unknown).
pub const PACK_COST: u32 = 100;
pub const PACK_MAX: u32 = 10;
/// A pack holds this many cards of its first rarity tier, then one of a higher tier (**guess**).
const COMMONS: usize = 4;

/// The BP a duel's result (1 won, 2 lost, 3 draw) pays into the player's account. The original's duels paid none (no
/// reply after a duel sets the client's BP); a server choice, so friends can keep buying packs.
pub fn duel_bp(result: u32) -> u32 {
    match result {
        1 => 100,
        3 => 50,
        _ => 30,
    }
}

/// A pack on sale: its id (the client's pack file, which gives the name, description and image), its dates, and its
/// cards (CARD_Prop rows) by rarity tier.
#[derive(Clone, Debug)]
pub struct ShopPack {
    pub id: u32,
    pub released: String,
    pub modified: String,
    pub tiers: Vec<Vec<u16>>,
}

/// A pack file's cards split into their ascending runs, read as rarity tiers: 44, 18, 9, 9 for an 80-card pack, the
/// OCG's Normal, Rare, Super and Ultra Rares (**guess**).
pub fn tiers(cards: &[u16]) -> Vec<Vec<u16>> {
    let mut tiers: Vec<Vec<u16>> = vec![];
    for &row in cards {
        match tiers.last_mut() {
            Some(tier) if tier.last().is_some_and(|&last| row > last) => tier.push(row),
            _ => tiers.push(vec![row]),
        }
    }
    tiers
}

/// The cards of one pack: four of the first tier, then one of a higher tier, each tier half as likely as the one
/// below it (**guess**); a pack of one tier gives five of it.
pub fn open(pack: &ShopPack, rng: &mut u64) -> Vec<u16> {
    let Some(first) = pack.tiers.first().filter(|t| !t.is_empty()) else { return vec![] };
    let pick = |tier: &[u16], rng: &mut u64| tier[(crate::duel::next(rng) % tier.len() as u64) as usize];
    let mut cards: Vec<u16> = (0..COMMONS).map(|_| pick(first, rng)).collect();
    let higher: Vec<&Vec<u16>> = pack.tiers[1..].iter().filter(|t| !t.is_empty()).collect();
    let weight = |i: usize| 1u64 << (higher.len() - 1 - i);
    let mut roll = crate::duel::next(rng) % (0..higher.len()).map(weight).sum::<u64>().max(1);
    let mut last = first;
    for (i, tier) in higher.iter().enumerate() {
        if roll < weight(i) {
            last = tier;
            break;
        }
        roll -= weight(i);
    }
    cards.push(pick(last, rng));
    cards
}

/// A pack on the shelves (a 0x1C22 item, read by 0x485E50): item_type 0 puts it on the Booster Packs shelf
/// (AddPack, 0x48C790). sale_id is what a purchase names: the pack's id.
pub fn shelf_item(pack: &ShopPack) -> Message {
    Message::record()
        .with("item_type", Value::U32(0))
        .with("item_id", Value::U32(pack.id))
        .with("item_cost", Value::U32(PACK_COST))
        .with("item_max", Value::U32(PACK_MAX))
        .with("sale_id", Value::U32(pack.id))
        .with("sale", Value::U32(0))
        .with("item_trade", Value::U32(0))
        .with("modified", Value::text(&pack.modified))
        .with("rel_time", Value::text(&pack.released))
}

/// A bought pack as a present (0x2777, PresentWindow 0x476DE0): type 0 is a pack, whose name comes from the client's
/// pack file pack_id; item holds its cards as big-endian u16 rows, shown one at a time.
pub fn present(present_id: u32, pack_id: u32, cards: &[u16]) -> Message {
    Message::reply(0x2777)
        .with("present_id", Value::U32(present_id))
        .with("type", Value::U32(0))
        .with("level", Value::U32(0))
        .with("form", Value::U32(0))
        .with("item_img", Value::text(""))
        .with("end_time", Value::text(""))
        .with("item", Value::u16_be_array(cards))
        .with("pack_id", Value::U32(pack_id))
}
