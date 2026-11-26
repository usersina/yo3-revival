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
    /// An anime-only added card (its CARD_Prop row) while the `anime-cards` setting is off: refused whatever the banlist.
    AnimeOnly(u16),
}

/// The first anime-only added card (#75) in `deck`, when the anime card versions are off: such a card has no version
/// the engine plays then, so every duel with it is refused, whatever the banlist (docs/added-cards.md).
pub fn anime_only_card(world: &World, deck: &Deck) -> Option<u16> {
    if world.settings.now().anime_cards {
        return None;
    }
    let anime_only = |row: &u16| world.added.cards.iter().any(|c| c.anime_only && c.row == *row);
    deck.cards().find(|row| anime_only(row))
}

/// The main deck holds 40 to 60 cards, the extra and side decks under 16 (`0x51AC50`).
fn sizes_fit(deck: &Deck) -> bool {
    (40..=60).contains(&deck.main.len()) && deck.extra.len() < 16 && deck.side.len() < 16
}

/// Fusion and Synchro Monsters only in the Extra Deck, and nothing else there, as the client's deck editor keeps them:
/// the rules engine moves such a card to the Extra Deck by itself, which would split the client's view from the duel.
/// Without the game's data (no kinds known) every deck passes.
fn in_their_decks(world: &World, deck: &Deck) -> bool {
    // The selected card profile's placement (#60): a card the profile makes a Main Deck card is not an Extra Deck one.
    let rows = world.card_view().extra_deck_rows;
    let extra = |row: &u16| rows.contains(row);
    rows.is_empty() || (!deck.main.iter().any(extra) && deck.extra.iter().all(extra))
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
    if let Some(row) = anime_only_card(world, deck) {
        return Err(Refusal::AnimeOnly(row));
    }
    let Some(rules) = pick(world, regulation) else { return Ok(()) };
    if !sizes_fit(deck) || !in_their_decks(world, deck) {
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
    world.regulations.get(&regulation).or_else(|| world.regulations.get(&crate::front::LOGIN_REGULATION))
}

/// The most problems [`explain`] lists: the chat window shows a few lines at once.
pub const MAX_PROBLEMS: usize = 6;

/// What is wrong with `deck` for a duel under `regulation` (as [`check`] decides it, every reason and not only the
/// first), in a player's words naming the cards: empty when [`check`] passes it. The game's own check refuses a deck
/// without saying why ("There are no valid Decks available.", Select Deck's "Invalid Deck"), and a refusal at the
/// duel's start shows only the disconnect dialog, so the server says it in the chat. At most [`MAX_PROBLEMS`], the last
/// saying how many more.
pub fn explain(world: &World, regulation: u32, owned: &BTreeMap<u16, u32>, deck: &Deck) -> Vec<String> {
    let mut out = vec![];
    let cid = |row: &u16| world.cards.get(row).map(|c| c.cid);
    let name = |row: &u16| -> String {
        world.added.cards.iter().find(|c| c.row == *row).map(|c| c.name.clone()).or_else(|| cid(row).and_then(|c| world.card_names.get(&c).cloned()))
            .unwrap_or_else(|| format!("card {row}"))
    };
    if !world.settings.now().anime_cards {
        let mut seen = vec![];
        for row in deck.cards().filter(|row| world.added.cards.iter().any(|c| c.anime_only && c.row == *row)) {
            if !seen.contains(&row) { seen.push(row); out.push(format!("{} {}", name(&row), crate::front::ANIME_ONLY_OFF)) }
        }
    }
    let Some(rules) = pick(world, regulation) else { return cut(out) };
    if !(40..=60).contains(&deck.main.len()) { out.push(format!("The Main Deck has {} cards: it needs 40 to 60.", deck.main.len())) }
    if deck.extra.len() >= 16 { out.push(format!("The Extra Deck has {} cards: at most 15.", deck.extra.len())) }
    if deck.side.len() >= 16 { out.push(format!("The Side Deck has {} cards: at most 15.", deck.side.len())) }
    let rows = world.card_view().extra_deck_rows;
    if !rows.is_empty() {
        let mut seen = vec![];
        for row in deck.main.iter().filter(|r| rows.contains(r)) {
            if !seen.contains(row) { seen.push(*row); out.push(format!("{} belongs in the Extra Deck.", name(row))) }
        }
        for row in deck.extra.iter().filter(|r| !rows.contains(r)) {
            if !seen.contains(row) { seen.push(*row); out.push(format!("{} can't be in the Extra Deck.", name(row))) }
        }
    }
    // Cards held, counted per card (any artwork), as `owns` counts them.
    let mut have: BTreeMap<u16, u32> = BTreeMap::new();
    for (row, &n) in owned { if let Some(c) = cid(row) { *have.entry(c).or_default() += n } }
    let mut want: Vec<(u16, u16, u32)> = vec![]; // (card, a row of it, copies)
    for row in deck.cards() {
        let Some(c) = cid(&row) else { out.push(format!("{} is no card of this server.", name(&row))); continue };
        match want.iter_mut().find(|(w, _, _)| *w == c) { Some(w) => w.2 += 1, None => want.push((c, row, 1)) }
    }
    for &(c, row, n) in &want {
        let held = have.get(&c).copied().unwrap_or(0);
        if held < n { out.push(format!("You hold {held} {} but the deck has {n}.", name(&row))) }
    }
    let banlist = world.banlists.iter().find(|(id, _)| *id == rules.id).map_or_else(|| "this banlist".to_owned(), |(_, n)| n.clone());
    for &(c, row, n) in &want {
        let limit = rules.limit(c);
        if Regulation::NEVER_ALLOWED.contains(&c) { out.push(format!("{} may never be in a deck.", name(&row))) }
        else if limit == 0 { out.push(format!("{} is Forbidden in {banlist}.", name(&row))) }
        else if n > limit { out.push(format!("{}: {n} copies, but {banlist} allows {limit}.", name(&row))) }
        if crate::engine::NOT_IN_THE_ENGINE.contains(&c) { out.push(format!("{} can't be played on this server yet.", name(&row))) }
    }
    cut(out)
}

fn cut(mut out: Vec<String>) -> Vec<String> {
    if out.len() > MAX_PROBLEMS {
        let more = out.len() - (MAX_PROBLEMS - 1);
        out.truncate(MAX_PROBLEMS - 1);
        out.push(format!("…and {more} more."));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn world() -> World {
        let mut world = World::new((1..=20).collect());
        world.cards = (1..=60u16).map(|row| (row, crate::translate::CardStats { cid: 4000 + row, atk: 0, def: 0, level: 0, attribute: 0, race: 0 })).collect();
        world.card_names = (1..=60u16).map(|row| (4000 + row, format!("Card {row}"))).collect::<HashMap<_, _>>();
        let mut rule = yo3_data::Regulation { id: 4, kind: 0, status: 0, name_id: 0, lists: [vec![4001], vec![4002], vec![], vec![]], default_limit: 3,
            record: yo3_proto::Message::record() };
        rule.lists[1].push(4002);
        world.regulations = [(4, rule.clone()), (crate::front::LOGIN_REGULATION, rule)].into();
        world.banlists = vec![(4, "Season 16".into())];
        world
    }

    #[test]
    fn a_deck_is_explained_card_by_card_and_a_good_one_not_at_all() {
        let world = world();
        let deck = |main: Vec<u16>| Deck { id: 1, name: "D".into(), main, extra: vec![], side: vec![] };
        let all: BTreeMap<u16, u32> = (1..=60u16).map(|r| (r, 3)).collect();
        // A legal deck: 40 cards, three each of 13 and one more, all held.
        let good: Vec<u16> = (10..=22).flat_map(|r| [r; 3]).chain([23]).collect();
        assert!(explain(&world, 4, &all, &deck(good.clone())).is_empty());
        assert_eq!(check(&world, 4, &all, &deck(good.clone())), Ok(()));
        // Card 1 Forbidden, two of Card 2 (Limited), 38 cards, and one card not held.
        let mut bad: Vec<u16> = (10..=21).flat_map(|r| [r; 3]).collect();
        bad.extend([1, 2, 2]);
        let mut held = all.clone();
        held.insert(10, 2);
        let said = explain(&world, 4, &held, &deck(bad.clone()));
        assert_eq!(said, ["The Main Deck has 39 cards: it needs 40 to 60.", "You hold 2 Card 10 but the deck has 3.",
            "Card 1 is Forbidden in Season 16.", "Card 2: 2 copies, but Season 16 allows 1."], "{said:?}");
        assert!(check(&world, 4, &held, &deck(bad)).is_err(), "the check refuses what explain lists");
        // Many problems: the list stops at six lines, the last counting the rest.
        let many: Vec<u16> = (1..=2).chain((30..=40).flat_map(|r| [r; 4])).collect();
        let said = explain(&world, 4, &BTreeMap::new(), &deck(many));
        assert_eq!(said.len(), MAX_PROBLEMS);
        assert!(said.last().unwrap().starts_with("…and "), "{said:?}");
    }
}
