//! Where each card comes from (#55): the booster packs, structure decks, Blister's Trade Card pools, the shop's rental
//! and the decks every player starts with, all read from the client's own files. `/where NAME` in the chat looks
//! a card up here.
use std::collections::HashSet;

/// What kind of place a card comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pack,
    StructureDeck,
    TradePool,
    Rental,
    /// The starter deck and the main characters' decks, which every player owns.
    FreeDeck,
}

/// One place, by the Konami ids of the cards it holds (a card's artworks share one).
#[derive(Clone, Debug)]
pub struct CardSource {
    pub kind: Kind,
    pub name: String,
    pub cards: HashSet<u16>,
}

/// The booster packs a Trade Card pool covers, by the pool's pack file (264-268; `shop::EXCHANGE_POOLS`).
pub fn trade_pool_name(pool: u32) -> String {
    const RUNS: [(u32, u32); 5] = [(1, 14), (15, 23), (24, 32), (33, 40), (41, 48)];
    if pool == crate::shop::EXTRA_POOL {
        return "Blister's Trade Card (the extra card list)".to_string();
    }
    let at = pool.saturating_sub(*crate::shop::EXCHANGE_POOLS.start()) as usize;
    match RUNS.get(at) {
        Some((first, last)) => format!("Blister's Trade Card (cards from Booster Packs {first}-{last})"),
        None => "Blister's Trade Card".to_string(),
    }
}

/// How many cards one answer lists at most, and how many places each card lists.
pub const MAX_CARDS: usize = 3;
pub const MAX_PLACES: usize = 8;

/// Whether `name` (lowercase) matches `query` (lowercase): the query's text inside it, where a run of `*` stands for
/// anything. The game's chat filter masks some words with stars before a line is sent ("red-eyes black" arrives as
/// "red-eyes *****"), so the stars are what a player's word could have been.
fn matches(name: &str, query: &str) -> bool {
    let mut rest = name;
    // The first text may be anywhere in the name; each later one follows the one before.
    for part in query.split('*').filter(|p| !p.is_empty()) {
        let Some(at) = rest.find(part) else { return false };
        rest = &rest[at + part.len()..];
    }
    true
}

/// The answer to `/where QUERY` as chat lines: for each card whose name is QUERY (or contains it, ignoring case) the
/// places it comes from; "no normal route" for a card no place holds. `names` are the cards' names by Konami id; a name
/// that several Konami ids share (another print) is one card with the places of all of them.
pub fn answer(query: &str, names: &std::collections::HashMap<u16, String>, sources: &[CardSource]) -> Vec<String> {
    let query = query.trim().to_lowercase();
    if query.replace('*', "").trim().chars().count() < 2 { return vec!["Usage: where CARD NAME (or part of it), e.g. where red-eyes".into()] }
    // One entry per name (lowercase), with every Konami id that has it.
    let mut by_name: std::collections::BTreeMap<String, (&str, Vec<u16>)> = Default::default();
    for (&cid, name) in names {
        let key = name.to_lowercase();
        if matches(&key, &query) { by_name.entry(key).or_insert((name.as_str(), vec![])).1.push(cid) }
    }
    // An exact name wins over the cards that merely contain it.
    if by_name.contains_key(&query) { by_name.retain(|k, _| *k == query) }
    let shown_query: String = query.chars().take(40).collect();
    if by_name.is_empty() { return vec![format!("No card is named like \"{shown_query}\".")] }
    if by_name.len() > MAX_CARDS {
        let shown: Vec<&str> = by_name.values().take(6).map(|(n, _)| *n).collect();
        return vec![format!("{} cards match \"{shown_query}\": {}{}. Be more specific.", by_name.len(), shown.join(", "),
            if by_name.len() > shown.len() { ", ..." } else { "" })]
    }
    by_name.into_values().map(|(name, cids)| {
        let places: Vec<&CardSource> = sources.iter().filter(|s| cids.iter().any(|c| s.cards.contains(c))).collect();
        if places.is_empty() { return format!("{name}: no normal route (the game's shop and decks don't hold it).") }
        let mut parts: Vec<String> = places.iter().take(MAX_PLACES).map(|s| s.name.clone()).collect();
        if places.len() > MAX_PLACES { parts.push(format!("and {} more", places.len() - MAX_PLACES)) }
        format!("{name}: {}", parts.join("; "))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools_are_named_by_the_booster_packs_they_cover() {
        assert!(trade_pool_name(264).contains("1-14") && trade_pool_name(268).contains("41-48"));
    }

    #[test]
    fn a_name_lists_its_places_or_none() {
        let names = [(1, "Dark Magician".to_string()), (2, "Dark Magician Girl".to_string()), (3, "Kuriboh".to_string())].into();
        let sources = [CardSource { kind: Kind::Pack, name: "Pack A".into(), cards: [1].into() }];
        assert_eq!(answer("dark mag", &names, &sources), ["Dark Magician: Pack A", "Dark Magician Girl: no normal route (the game's shop and decks don't hold it)."]);
        assert_eq!(answer("dark magician", &names, &sources), ["Dark Magician: Pack A"], "an exact name wins");
        assert_eq!(answer("KURI", &names, &sources).len(), 1);
        assert!(answer("zzz", &names, &sources)[0].starts_with("No card"));
    }

    #[test]
    fn stars_from_the_games_word_filter_match_any_text() {
        let names = [(1, "Red-Eyes Black Dragon".to_string()), (2, "Red-Eyes Darkness Metal Dragon".to_string()), (3, "Black Luster Soldier".to_string())].into();
        let sources = [CardSource { kind: Kind::Pack, name: "Pack A".into(), cards: [1].into() }];
        assert_eq!(answer("red-eyes *****", &names, &sources).len(), 2, "both Red-Eyes cards");
        assert_eq!(answer("red-eyes ***** metal", &names, &sources), ["Red-Eyes Darkness Metal Dragon: no normal route (the game's shop and decks don't hold it)."]);
        assert!(answer("*****", &names, &sources)[0].starts_with("Usage"), "only stars is no query");
    }

    #[test]
    fn prints_of_one_name_are_one_card() {
        let names = [(1, "Dark Magician".to_string()), (2, "Dark Magician".to_string())].into();
        let sources = [CardSource { kind: Kind::Pack, name: "Pack A".into(), cards: [1].into() }, CardSource { kind: Kind::Rental, name: "Rental".into(), cards: [2].into() }];
        assert_eq!(answer("dark magician", &names, &sources), ["Dark Magician: Pack A; Rental"]);
    }
}
