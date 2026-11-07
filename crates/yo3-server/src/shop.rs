//! The shop's stock (shopmain.cpp): booster packs and structure decks from the client's own pack and recipe files,
//! what a pack holds, and the presents bought goods arrive as (PresentWindow).
use yo3_proto::{Message, Value};

/// BOOSTERPASS Points a pack costs, and the most one purchase may take (item_cost, item_max; **guess**: the
/// original prices are unknown).
pub const PACK_COST: u32 = 100;
pub const PACK_MAX: u32 = 10;
/// The same for a structure deck (**guess**), sold under DECK_SALE + its deck id since pack and deck ids overlap.
pub const DECK_COST: u32 = 500;
pub const DECK_MAX: u32 = 3;
pub const DECK_SALE: u32 = 20000;
/// Protectors and avatar items, one of each a player (**guess** prices), sold under ITEM_SALE + their pbin id.
pub const PROTECTOR_COST: u32 = 200;
pub const AVATAR_ITEM_COST: u32 = 100;
pub const ITEM_SALE: u32 = 30000;
/// The shelf types: a protector (AddProtector), an avatar item (any type above 2 hides once owned).
const PROTECTOR: u32 = 6;
const AVATAR_ITEM: u32 = 3;
/// Shelf dates for goods without a file of their own.
const ITEM_DATE: &str = "2010-03-23 00:00:00";
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

/// A structure deck on sale: its dates and the deck itself, whose id is its preset deck (the client's recipe file,
/// which gives the shelf its name, description and art) and whose name (from the client's text table) it takes in a
/// buyer's deck list.
#[derive(Clone, Debug)]
pub struct ShopDeck {
    pub released: String,
    pub modified: String,
    pub deck: crate::front::Deck,
}

/// What a sale id sells.
pub enum Goods<'a> {
    Pack(&'a ShopPack),
    Deck(&'a ShopDeck),
    /// A protector or an avatar item (with its name text), by pbin id.
    Protector(u16),
    AvatarItem(u16, u16),
}

impl<'a> Goods<'a> {
    pub fn find(world: &'a crate::front::World, sale_id: u32) -> Option<Goods<'a>> {
        if let Some(pack) = world.packs.iter().find(|p| p.id == sale_id) {
            return Some(Goods::Pack(pack));
        }
        let item = sale_id.checked_sub(ITEM_SALE).and_then(|id| u16::try_from(id).ok());
        if let Some(id) = item {
            if world.protectors.contains(&id) {
                return Some(Goods::Protector(id));
            }
            return world.avatar_items.iter().find(|(a, _)| *a == id).map(|&(id, name)| Goods::AvatarItem(id, name));
        }
        world.structure_decks.iter().find(|d| Some(d.deck.id) == sale_id.checked_sub(DECK_SALE)).map(Goods::Deck)
    }

    /// Its price in BP and the most one purchase may take.
    pub fn price(&self) -> (u32, u32) {
        match self {
            Goods::Pack(_) => (PACK_COST, PACK_MAX),
            Goods::Deck(_) => (DECK_COST, DECK_MAX),
            Goods::Protector(_) => (PROTECTOR_COST, 1),
            Goods::AvatarItem(..) => (AVATAR_ITEM_COST, 1),
        }
    }

    /// The pbin id it gives the player, for an item.
    pub fn item(&self) -> Option<u16> {
        match self {
            Goods::Protector(id) | Goods::AvatarItem(id, _) => Some(*id),
            _ => None,
        }
    }

    /// One of it, as the present it arrives as: a pack opened now, the deck, or the item.
    pub fn take(&self, rng: &mut u64) -> Present {
        match self {
            Goods::Pack(pack) => Present::Pack { id: pack.id, cards: open(pack, rng) },
            Goods::Deck(deck) => Present::Deck { id: deck.deck.id, cards: deck.deck.cards().collect() },
            Goods::Protector(id) => Present::Protector(*id),
            Goods::AvatarItem(id, name) => Present::AvatarItem(*id, *name),
        }
    }
}

/// A bought pack or structure deck waiting for the client to show it.
#[derive(Clone, Debug)]
pub enum Present {
    Pack { id: u32, cards: Vec<u16> },
    Deck { id: u32, cards: Vec<u16> },
    Protector(u16),
    AvatarItem(u16, u16),
}

impl Present {
    /// The cards it puts in the collection.
    pub fn cards(&self) -> &[u16] {
        match self {
            Present::Pack { cards, .. } | Present::Deck { cards, .. } => cards,
            Present::Protector(_) | Present::AvatarItem(..) => &[],
        }
    }

    /// As a 0x2777 (PresentWindow 0x476DE0, which reads present_id, type, level, form, item_img, end_time and item):
    /// a pack is type 0 with its cards as item (big-endian u16 rows, shown one at a time) and pack_id, whose name
    /// comes from the client's pack file; a structure deck is type 1 with its deck id as item's first u16 (case 1,
    /// 0x4777D3) and desc "-1" for the deck's own description; a protector type 7 (case 7, 0x47800E) and an avatar item
    /// type 6 with its name text as name_id (the shop's own 0x2777 case), each with its pbin id as item's first u16.
    pub fn message(&self, present_id: u32) -> Message {
        let m = Message::reply(0x2777)
            .with("present_id", Value::U32(present_id))
            .with("level", Value::U32(0))
            .with("form", Value::U32(0))
            .with("item_img", Value::text(""))
            .with("end_time", Value::text(""));
        match self {
            Present::Pack { id, cards } => {
                m.with("type", Value::U32(0)).with("item", Value::u16_be_array(cards)).with("pack_id", Value::U32(*id))
            }
            Present::Deck { id, .. } => {
                m.with("type", Value::U32(1)).with("item", Value::u16_be_array(&[*id as u16])).with("desc", Value::text("-1"))
            }
            Present::Protector(id) => m.with("type", Value::U32(7)).with("item", Value::u16_be_array(&[*id])),
            Present::AvatarItem(id, name) => {
                m.with("type", Value::U32(6)).with("item", Value::u16_be_array(&[*id])).with("name_id", Value::U32(*name as u32))
            }
        }
    }
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

/// A structure deck on the shelves: item_type 1 puts it on the STRUCTURE DECK tab (AddStruct, 0x48CE40).
pub fn shelf_deck(deck: &ShopDeck) -> Message {
    Message::record()
        .with("item_type", Value::U32(1))
        .with("item_id", Value::U32(deck.deck.id))
        .with("item_cost", Value::U32(DECK_COST))
        .with("item_max", Value::U32(DECK_MAX))
        .with("sale_id", Value::U32(DECK_SALE + deck.deck.id))
        .with("sale", Value::U32(0))
        .with("item_trade", Value::U32(0))
        .with("modified", Value::text(&deck.modified))
        .with("rel_time", Value::text(&deck.released))
}

/// An item on its shelf (0x1C32 protectors, 0x1C2A avatar items; read by 0x485E50 like the packs).
fn shelf_entry(item_type: u32, id: u16, cost: u32) -> Message {
    Message::record()
        .with("item_type", Value::U32(item_type))
        .with("item_id", Value::U32(id as u32))
        .with("item_param", Value::U32(id as u32))
        .with("item_cost", Value::U32(cost))
        .with("item_max", Value::U32(1))
        .with("sale_id", Value::U32(ITEM_SALE + id as u32))
        .with("sale", Value::U32(0))
        .with("item_trade", Value::U32(0))
        .with("modified", Value::text(ITEM_DATE))
        .with("rel_time", Value::text(ITEM_DATE))
}

/// A card protector on its shelf: item_type 6 (AddProtector, 0x48D520).
pub fn shelf_protector(id: u16) -> Message {
    shelf_entry(PROTECTOR, id, PROTECTOR_COST)
}

/// An avatar item on its shelf: item_type above 2 with item_param its pbin id, which the client hides once owned.
pub fn shelf_avatar_item(id: u16) -> Message {
    shelf_entry(AVATAR_ITEM, id, AVATAR_ITEM_COST)
}
