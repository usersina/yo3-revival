//! Card trades between two players (CardTrade.cpp, handler 0x4248F0): the invitation, then a scene where each puts up
//! cards, both press OK, and both confirm.
use std::collections::BTreeMap;

use yo3_proto::{Message, Value};

use crate::front::{Deck, Player, World};

/// A trade under way: its two players (the inviter first) and, per player, whether their scene is ready, the cards
/// they offer (CARD_Prop rows), whether they pressed OK, and their answer to "Execute Trade?".
#[derive(Debug)]
pub struct Trade {
    pub players: [u32; 2],
    pub ready: [bool; 2],
    pub offers: [Vec<u16>; 2],
    pub ok: [bool; 2],
    pub answers: [Option<bool>; 2],
}

impl Trade {
    pub fn new(players: [u32; 2]) -> Trade {
        Trade { players, ready: [false; 2], offers: [vec![], vec![]], ok: [false; 2], answers: [None; 2] }
    }

    /// The seat of a player in it.
    pub fn seat(&self, player_id: u32) -> Option<usize> {
        self.players.iter().position(|&p| p == player_id)
    }
}

/// What a player owns, by CARD_Prop row: their starter deck's and the official decks' cards, the ones they bought or
/// got, less the ones they gave away.
pub fn collection(world: &World, player: &Player) -> BTreeMap<u16, u32> {
    let mut owned: BTreeMap<u16, u32> = BTreeMap::new();
    for row in world.starter_deck.iter().chain(&world.official_decks).flat_map(Deck::cards) {
        *owned.entry(row).or_default() += 1;
    }
    for (&row, &n) in &player.cards {
        *owned.entry(row).or_default() += n;
    }
    for (&row, &n) in &player.given {
        if let Some(have) = owned.get_mut(&row) {
            *have = have.saturating_sub(n);
        }
    }
    owned.retain(|_, n| *n > 0);
    owned
}

/// Whether a player owns every card of an offer (copies counted), less the cards they have lent out (`lent`: the
/// trade and rental scenes' collection takes them away, flag 8 of 0x4F6340). Borrowed cards are not in the player's
/// collection here: the client adds them itself and keeps them out of offers (`noTrade`).
pub fn owns(world: &World, player: &Player, lent: &[u16], offer: &[u16]) -> bool {
    let owned = usable(world, player, lent, &[]);
    let mut wanted: BTreeMap<u16, u32> = BTreeMap::new();
    for &row in offer {
        *wanted.entry(row).or_default() += 1;
    }
    wanted.iter().all(|(row, n)| owned.get(row).copied().unwrap_or(0) >= *n)
}

/// What a player can put in a deck now: their [`collection`] less the cards they lent and plus the ones they borrowed
/// (a lent card is out of the lender's hands until the rental ends, #39).
pub fn usable(world: &World, player: &Player, lent: &[u16], borrowed: &[u16]) -> BTreeMap<u16, u32> {
    let mut owned = collection(world, player);
    for row in lent {
        if let Some(n) = owned.get_mut(row) {
            *n = n.saturating_sub(1);
        }
    }
    for &row in borrowed {
        *owned.entry(row).or_default() += 1;
    }
    owned.retain(|_, n| *n > 0);
    owned
}

/// Moves an offer's cards out of a player's collection: the bought ones first, then the starter deck's.
pub fn give(player: &mut Player, offer: &[u16]) {
    for &row in offer {
        match player.cards.get_mut(&row) {
            Some(n) if *n > 0 => {
                *n -= 1;
                if *n == 0 {
                    player.cards.remove(&row);
                }
            }
            _ => *player.given.entry(row).or_default() += 1,
        }
    }
}

/// Moves an offer's cards into a player's collection: back where they were given away from first.
pub fn receive(player: &mut Player, offer: &[u16]) {
    for &row in offer {
        match player.given.get_mut(&row) {
            Some(n) if *n > 0 => {
                *n -= 1;
                if *n == 0 {
                    player.given.remove(&row);
                }
            }
            _ => *player.cards.entry(row).or_default() += 1,
        }
    }
}

/// The trunk 9000 {cards, noTrade} (copied wholesale into the trunk dataset, slot 18): owned and untradeable count per
/// card, one byte at row - 1 (0x410DB0); the NPC duel writes into both unchecked (0x4F6A82). Edit Deck lists rows
/// 1 .. len - 1 (0x4125D8), so the arrays hold one byte more than the highest row, or the last card is never listed.
pub fn trunk(world: &World, player: &Player) -> Message {
    let len = world.card_rows + 1;
    let mut cards = vec![0u8; len];
    for (row, count) in collection(world, player) {
        if let Some(n) = (row as usize).checked_sub(1).and_then(|i| cards.get_mut(i)) {
            *n = count.min(255) as u8;
        }
    }
    let no_trade = Value::Bin(vec![0; len]);
    Message::reply(9000).item(Message::record().with("cards", Value::Bin(cards)).with("noTrade", no_trade))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trunk_has_room_for_the_last_row() {
        // Edit Deck lists rows 1 .. len - 1 of `cards` (the loop at 0x4125D8 runs while row < 0x410C90's length) but reads
        // a row's count at row - 1 (0x410DB0): with one byte per row the last row was never listed. Live (#75): Blue Rose
        // Dragon (row 4833, a Special Pack card) and then the first added card were missing from the trunk.
        let world = World { card_rows: 50, ..World::new(vec![]) };
        let mut player = Player::new(1001, "Owner");
        player.cards.insert(50, 2);
        let trunk = trunk(&world, &player);
        let cards = trunk.items[0].get("cards").and_then(Value::as_bytes).unwrap().to_vec();
        let no_trade = trunk.items[0].get("noTrade").and_then(Value::as_bytes).unwrap().to_vec();
        assert_eq!((cards.len(), cards[49], no_trade.len()), (51, 2, 51), "row 50 at index 49, and the list reaches it");
    }

    #[test]
    fn a_lent_card_is_not_the_lenders_to_play_and_a_borrowed_one_is_the_borrowers() {
        // Security review (#39): the lender's deck check took the whole collection, so a lent card was in two decks.
        let world = World::new(vec![]);
        let mut player = Player::new(1001, "Lender");
        player.cards.insert(10, 2);
        player.cards.insert(11, 1);
        let lender = usable(&world, &player, &[10, 11], &[]);
        assert_eq!((lender.get(&10), lender.get(&11)), (Some(&1), None));
        let borrower = usable(&world, &Player::new(1002, "Borrower"), &[], &[10]);
        assert_eq!(borrower.get(&10), Some(&1));
    }
}
