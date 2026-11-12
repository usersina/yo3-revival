//! The check a deck passes before a duel, the client's own (`0x51AB00` -> `0x51AA80`, sizes `0x51AC50`, regulation
//! `0x51A9A0`/`0x51A930`) repeated on the server, since only a modified client skips it.
use std::collections::BTreeMap;

use yo3_data::Regulation;

use crate::front::{Deck, World};

/// Why a deck is refused (the client's codes: 2 sizes, 3 cards not owned, 4 regulation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Sizes,
    NotOwned,
    Regulation,
}

/// The main deck holds 40 to 60 cards, the extra and side decks under 16 (`0x51AC50`).
fn sizes_fit(deck: &Deck) -> bool {
    (40..=60).contains(&deck.main.len()) && deck.extra.len() < 16 && deck.side.len() < 16
}

/// Whether `owned` (copies by CARD_Prop row) covers the deck. Copies are counted per Konami id, so a card's other
/// artworks count too: more lenient than the client, which counts per row.
fn owns(world: &World, owned: &BTreeMap<u16, u32>, deck: &Deck) -> bool {
    let cid = |row: &u16| world.cards.get(row).map(|c| c.cid);
    let mut have: BTreeMap<Option<u16>, u32> = BTreeMap::new();
    for (row, &n) in owned {
        *have.entry(cid(row)).or_default() += n;
    }
    let mut want: BTreeMap<Option<u16>, u32> = BTreeMap::new();
    for row in deck.cards() {
        *want.entry(cid(&row)).or_default() += 1;
    }
    want.iter().all(|(card, n)| card.is_some() && have.get(card).copied().unwrap_or(0) >= *n)
}

/// Checks `deck` for a duel under `regulation` (an id of the client's lists; an unknown id or 0 means the login one),
/// with `owned` the player's collection (CARD_Prop row -> copies, borrowed cards included). Without the game's data
/// (no regulations) every deck passes.
pub fn check(world: &World, regulation: u32, owned: &BTreeMap<u16, u32>, deck: &Deck) -> Result<(), Refusal> {
    let Some(rules) = pick(world, regulation) else { return Ok(()) };
    if !sizes_fit(deck) {
        return Err(Refusal::Sizes);
    }
    if !owns(world, owned, deck) {
        return Err(Refusal::NotOwned);
    }
    // cards missing from the table were refused above, so every row has an id
    let cids = || deck.cards().filter_map(|row| world.cards.get(&row).map(|c| c.cid));
    if !rules.allows(cids()) {
        return Err(Refusal::Regulation);
    }
    // A card the rules engine lacks can't be played: refused like a banned card.
    if cids().any(|cid| crate::engine::NOT_IN_THE_ENGINE.contains(&cid)) {
        return Err(Refusal::Regulation);
    }
    Ok(())
}

fn pick(world: &World, regulation: u32) -> Option<&Regulation> {
    world.regulations.get(&regulation).or_else(|| world.regulations.get(&world.login_regulation))
}
