//! What the host can give a player beyond what the server hands out (#19): any card of the client's table, by its name
//! or Konami id and its artwork, and any deck recipe for Load Deck's Official tab, by its title or preset id. A card is
//! a CARD_Prop row in the player's collection (`Player::cards`; each artwork is its own row), a recipe the item (pbin
//! id) that owns it (`Player::items`, the bit 0x43D8D0 tests).
use std::collections::BTreeSet;

/// A card the host can give: one artwork of one card.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub row: u16,
    pub cid: u16,
    pub art: u32,
    pub name: String,
    pub description: String,
}

/// A deck recipe the host can give.
#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    /// Its preset deck id.
    pub id: u32,
    pub title: String,
    /// The item that owns it.
    pub item: u16,
    /// Its cards (CARD_Prop rows): main, extra, then side.
    pub cards: Vec<u16>,
}

/// Every card and recipe the client knows.
#[derive(Clone, Debug, Default)]
pub struct Gifts {
    pub cards: Vec<Card>,
    pub recipes: Vec<Recipe>,
}

impl Gifts {
    /// From the client's files: every card but the tokens (kind 10, never in a collection), every preset deck that
    /// has an item.
    pub fn from_game(data: &yo3_data::GameData) -> Gifts {
        let cards = data.cards.iter().filter(|c| c.kind != 10)
            .map(|c| Card { row: c.row, cid: c.cid, art: c.art, name: c.name.clone(), description: c.desc.clone() }).collect();
        let recipes = data.decks.iter().filter_map(|d| Some(Recipe {
            id: d.id,
            title: d.name_id().and_then(|t| data.text(t)).map_or_else(|| format!("deck {}", d.id), str::to_owned),
            item: d.pbin_no()?,
            cards: d.cards().collect(),
        })).collect();
        Gifts { cards, recipes }
    }

    /// The card named `card` (any case) or of that Konami id, artwork `art` (its first when none).
    pub fn card(&self, card: &str, art: Option<u32>) -> Result<&Card, String> {
        let card = card.trim();
        let rows: Vec<&Card> = match card.parse::<u16>() {
            Ok(cid) => self.cards.iter().filter(|c| c.cid == cid).collect(),
            Err(_) => self.cards.iter().filter(|c| c.name.eq_ignore_ascii_case(card)).collect(),
        };
        let cids: BTreeSet<u16> = rows.iter().map(|c| c.cid).collect();
        match cids.len() {
            0 => return Err(format!("no card named or numbered {card:?} (a card's English name, or its Konami id)")),
            1 => {}
            _ => return Err(format!("{} cards are named {card:?} (Konami ids {}): give one by its id", cids.len(),
                cids.iter().map(u16::to_string).collect::<Vec<_>>().join(", "))),
        }
        let arts: BTreeSet<u32> = rows.iter().map(|c| c.art).collect();
        let want = art.or_else(|| arts.first().copied()).unwrap_or(1);
        rows.into_iter().filter(|c| c.art == want).min_by_key(|c| c.row).ok_or_else(|| format!("{} has no artwork {want} (it has {})",
            card, arts.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")))
    }

    /// The recipe titled `deck` (any case) or of that preset id.
    pub fn recipe(&self, deck: &str) -> Result<&Recipe, String> {
        let deck = deck.trim();
        let found: Vec<&Recipe> = match deck.parse::<u32>() {
            Ok(id) => self.recipes.iter().filter(|r| r.id == id).collect(),
            Err(_) => self.recipes.iter().filter(|r| r.title.eq_ignore_ascii_case(deck)).collect(),
        };
        match found.as_slice() {
            [] => Err(format!("no deck recipe titled or numbered {deck:?} (`recipes` lists them)")),
            [one] => Ok(one),
            many => Err(format!("{} recipes are titled {deck:?} (ids {}): give one by its id", many.len(),
                many.iter().map(|r| r.id.to_string()).collect::<Vec<_>>().join(", "))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gifts() -> Gifts {
        let card = |row, cid, art, name: &str| Card { row, cid, art, name: name.into(), description: String::new() };
        let recipe = |id, title: &str, item| Recipe { id, title: title.into(), item, cards: vec![816, 816] };
        Gifts {
            cards: vec![card(816, 4007, 1, "Blue-Eyes White Dragon"), card(817, 4007, 2, "Blue-Eyes White Dragon"),
                card(574, 4396, 2, "Launcher Spider"), card(575, 4396, 1, "Launcher Spider"),
                card(100, 4041, 1, "Dark Magician"), card(200, 5165, 1, "Dark Magician")],
            recipes: vec![recipe(7, "E-HERO EVOLUTION DECK", 769), recipe(40, "Five Emperors", 900), recipe(41, "Five Emperors", 901)],
        }
    }

    #[test]
    fn a_card_is_found_by_name_or_konami_id_and_artwork() {
        let g = gifts();
        assert_eq!(g.card("blue-eyes white dragon", None).unwrap().row, 816, "the first artwork when none is named");
        assert_eq!(g.card("4007", Some(2)).unwrap().row, 817);
        assert_eq!(g.card("Launcher Spider", None).unwrap().row, 575, "art 1 is the later row");
        assert!(g.card("Blue-Eyes White Dragon", Some(3)).unwrap_err().contains("has no artwork 3 (it has 1, 2)"));
        assert!(g.card("Dark Magician", None).unwrap_err().contains("4041, 5165"), "two cards share the name");
        assert_eq!(g.card("5165", None).unwrap().row, 200);
        assert!(g.card("Nothing", None).is_err());
    }

    #[test]
    fn a_recipe_is_found_by_title_or_id() {
        let g = gifts();
        assert_eq!(g.recipe("e-hero evolution deck").unwrap().item, 769);
        assert_eq!(g.recipe("41").unwrap().item, 901);
        assert!(g.recipe("Five Emperors").unwrap_err().contains("ids 40, 41"));
        assert!(g.recipe("No such deck").is_err());
    }
}
