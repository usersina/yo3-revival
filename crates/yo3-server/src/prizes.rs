//! What the server hands out beyond the shop's shelves (#19, #18): the Special Pack an Official Event win earns, the
//! rare alternate artwork of a pulled card, and the deck recipe an NPC gives for a first win. All of it comes from the
//! client's own files (the card table, the packs, the decks); the numbers are ours (**guesses**, roadmap).
use std::collections::{HashMap, HashSet};

use crate::shop::Arts;

/// An Official Event's win (a duel, or a match) earns one Special Pack, at most this many to a player in one event
/// (**guess**: the original's rule is unknown).
pub const SPECIAL_PACK_CAP: u32 = 10;
/// The five cards of the 101 without a route that stay unobtainable: the Egyptian God Cards Obelisk and Ra, and the
/// three cards no deck may hold (the client's regulations ban them, `0x51A9A0`).
pub const UNOBTAINABLE: [&str; 5] =
    ["Obelisk the Tormentor", "The Winged Dragon of Ra", "Spirit Elimination", "Secret Pass to the Treasures", "Pole Position"];

/// A deck recipe an NPC gives the first time the player beats it with that deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcRecipe {
    /// The recipe's item (`pbin_no`): its bit puts the deck on Load Deck's Official tab.
    pub item: u16,
    pub npc: String,
    /// The deck's title ("EVOLUTION HERO" and the like) as the client names it.
    pub deck: String,
}

#[derive(Clone, Debug, Default)]
pub struct Prizes {
    /// The Special Pack's cards (CARD_Prop rows) by rarity tier (**guess**: by the card's type, see [`special_tiers`]).
    pub special: Vec<Vec<u16>>,
    /// The alternate artworks of the cards packs give, and how rare a pull of one is.
    pub arts: Arts,
    /// NPC level id -> the recipe of the deck it plays.
    pub npc_recipes: HashMap<u32, NpcRecipe>,
}

/// The rarity tiers of the Special Pack's cards (**guess**: the original's pack files are not in the client): the
/// commons are Spells, Traps and Normal monsters, then the other monsters, and the rarest the Fusion and Synchro
/// Monsters. Empty tiers are left out.
pub fn special_tiers(cards: &[&yo3_data::Card]) -> Vec<Vec<u16>> {
    let mut tiers: [Vec<u16>; 3] = Default::default();
    for c in cards {
        let at = if c.extra_deck() { 2 } else if matches!(c.kind, 0 | 13 | 14) { 0 } else { 1 };
        tiers[at].push(c.row);
    }
    tiers.into_iter().filter(|t| !t.is_empty()).collect()
}

/// The Konami ids with a route to the player in the original game: `sources` are the booster packs, structure decks,
/// Trade Card pools, rental and free decks (`sources::CardSource`); a card in none of them and not one of the
/// [`UNOBTAINABLE`] is a Special Pack card (a name some routed card has is not a gap). One row per card: its first artwork (the others come as alternate artworks).
pub fn special_cards<'a>(cards: impl Iterator<Item = &'a yo3_data::Card>, routed: &HashSet<u16>) -> Vec<&'a yo3_data::Card> {
    // #19 counts by name: a second print of a card the packs give (Dark Magician 5165, Polymerization 4867) is no gap.
    let cards: Vec<&'a yo3_data::Card> = cards.collect();
    let routed_names: HashSet<&str> = cards.iter().copied().filter(|c| routed.contains(&c.cid)).map(|c| c.name.as_str()).collect();
    let mut first: HashMap<u16, &yo3_data::Card> = HashMap::new();
    for c in cards.iter().copied().filter(|c| c.kind != 10 && !routed.contains(&c.cid) && !routed_names.contains(c.name.as_str()) && !UNOBTAINABLE.contains(&c.name.as_str())) {
        let slot = first.entry(c.cid).or_insert(c);
        if (c.art, c.row) < (slot.art, slot.row) {
            *slot = c;
        }
    }
    let mut all: Vec<&yo3_data::Card> = first.into_values().collect();
    all.sort_by_key(|c| c.row);
    all
}

/// Every row's alternate artworks: the other rows of its card that no booster pack, structure deck or free deck gives
/// (`routed_rows`). A card with one artwork has none.
pub fn alternate_arts<'a>(cards: impl Iterator<Item = &'a yo3_data::Card>, routed_rows: &HashSet<u16>) -> HashMap<u16, Vec<u16>> {
    let mut by_cid: HashMap<u16, Vec<u16>> = HashMap::new();
    for c in cards.filter(|c| c.kind != 10) {
        by_cid.entry(c.cid).or_default().push(c.row);
    }
    let mut alts = HashMap::new();
    for rows in by_cid.values().filter(|r| r.len() > 1) {
        for &row in rows {
            let others: Vec<u16> = rows.iter().copied().filter(|r| *r != row && !routed_rows.contains(r)).collect();
            if !others.is_empty() {
                alts.insert(row, others);
            }
        }
    }
    alts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(row: u16, cid: u16, art: u32, kind: u32, name: &str) -> yo3_data::Card {
        yo3_data::Card {
            row, cid, atk: 0, def: 0, level: 0, attribute: 0, race: 0, kind, art, words: [0; 2], genre: 0, name: name.into(), desc: String::new(),
        }
    }

    #[test]
    fn the_special_pack_holds_the_cards_with_no_route_but_the_five_that_stay_out() {
        let cards = [card(1, 10, 1, 0, "Obelisk the Tormentor"), card(2, 11, 1, 1, "Malefic Stardust Dragon"), card(3, 11, 2, 1, "Malefic Stardust Dragon"),
            card(4, 12, 1, 0, "Kuriboh"), card(5, 13, 1, 10, "Token"), card(6, 14, 1, 13, "Synchro Gift"), card(7, 15, 1, 0, "Kuriboh")];
        let routed: HashSet<u16> = [12].into();
        let found: Vec<u16> = special_cards(cards.iter(), &routed).iter().map(|c| c.row).collect();
        assert_eq!(found, [2, 6], "one row per card (its first artwork), no token, none with a route, no second print of one that has");
    }

    #[test]
    fn tiers_follow_the_cards_type() {
        let cards = [card(1, 1, 1, 13, "Spell"), card(2, 2, 1, 1, "Effect"), card(3, 3, 1, 17, "Synchro"), card(4, 4, 1, 0, "Normal")];
        let refs: Vec<&yo3_data::Card> = cards.iter().collect();
        assert_eq!(special_tiers(&refs), [vec![1, 4], vec![2], vec![3]]);
        assert_eq!(special_tiers(&refs[1..2]), [vec![2]]);
    }

    #[test]
    fn an_unrouted_artwork_is_the_alternate_of_the_one_a_pack_gives() {
        let cards = [card(574, 4396, 2, 1, "Launcher Spider"), card(575, 4396, 1, 1, "Launcher Spider"), card(9, 9, 1, 0, "Kuriboh")];
        let alts = alternate_arts(cards.iter(), &[574].into());
        assert_eq!(alts.get(&574), Some(&vec![575]), "the packs' art 2 can come as art 1");
        assert_eq!(alts.get(&575), None, "art 1 has no other unrouted art");
        assert!(!alts.contains_key(&9));
    }
}
