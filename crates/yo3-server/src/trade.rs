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

/// What a player owns, by CARD_Prop row: their starter deck's cards, the ones they bought or got, less the ones they
/// gave away.
pub fn collection(world: &World, player: &Player) -> BTreeMap<u16, u32> {
    let mut owned: BTreeMap<u16, u32> = BTreeMap::new();
    for row in world.starter_deck.iter().flat_map(Deck::cards) {
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

/// Whether a player owns every card of an offer (copies counted).
pub fn owns(world: &World, player: &Player, offer: &[u16]) -> bool {
    let owned = collection(world, player);
    let mut wanted: BTreeMap<u16, u32> = BTreeMap::new();
    for &row in offer {
        *wanted.entry(row).or_default() += 1;
    }
    wanted.iter().all(|(row, n)| owned.get(row).copied().unwrap_or(0) >= *n)
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
/// card, one byte at row - 1 (0x410DB0); the NPC duel writes into both unchecked (0x4F6A82).
pub fn trunk(world: &World, player: &Player) -> Message {
    let mut cards = vec![0u8; world.card_rows];
    for (row, count) in collection(world, player) {
        if let Some(n) = (row as usize).checked_sub(1).and_then(|i| cards.get_mut(i)) {
            *n = count.min(255) as u8;
        }
    }
    let no_trade = Value::Bin(vec![0; world.card_rows]);
    Message::reply(9000).item(Message::record().with("cards", Value::Bin(cards)).with("noTrade", no_trade))
}
