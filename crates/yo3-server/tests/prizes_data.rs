//! The numbers #19 counts, read from the client's own files (skipped unless the game is installed, YO3_GAME_DIR or the
//! default Wine path): the Special Pack's 96 cards, the 45 alternate artworks, the deck recipes NPCs give (66) and the
//! decks the shop sells besides the structure decks (the 22 theme decks and the 2 other starters).
use std::path::PathBuf;

use yo3_data::{GameData, default_game_dir};

fn game() -> Option<GameData> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    if !dir.join("package/data002.ypk").is_file() {
        eprintln!("game not installed; skipping");
        return None;
    }
    Some(GameData::load(&dir).expect("the client's data loads"))
}

#[test]
fn the_special_pack_holds_the_96_cards_without_any_route_and_no_god_card() {
    let Some(data) = game() else { return };
    let prizes = yo3_server::game_prizes(&data);
    let rows: Vec<u16> = prizes.special.iter().flatten().copied().collect();
    assert_eq!(rows.len(), 96, "#19: 101 cards without a pack, deck or rental route, less the five that stay out");
    let names: Vec<String> = rows.iter().filter_map(|r| data.cards.by_row(*r)).map(|c| c.name.clone()).collect();
    for out in yo3_server::prizes::UNOBTAINABLE {
        assert!(!names.iter().any(|n| n == out), "{out} stays unobtainable");
    }
    for card in ["Malefic Stardust Dragon", "Z-ONE", "Blue Rose Dragon", "Dragunity Arma Leyvaten", "Blackwing - Zephyrus the Elite"] {
        assert!(names.iter().any(|n| n == card), "{card} is in the Special Pack");
    }
    assert!(prizes.special.len() >= 2, "more than one rarity tier");
}

#[test]
fn the_alternate_artworks_are_the_45_of_39_cards_and_launcher_spiders_art_1() {
    let Some(data) = game() else { return };
    let prizes = yo3_server::game_prizes(&data);
    let spider: Vec<&yo3_data::Card> = data.cards.iter().filter(|c| c.name == "Launcher Spider").collect();
    let packs = spider.iter().find(|c| prizes.arts.alts.contains_key(&c.row) && c.art == 2).expect("the packs' art 2 has an alternate");
    let other: Vec<u16> = prizes.arts.alts[&packs.row].clone();
    assert_eq!(other.len(), 1);
    assert_eq!(data.cards.by_row(other[0]).map(|c| c.art), Some(1), "Launcher Spider: its art 1");
    // Rows no pack, structure deck or free deck gives, whose card another row of is given: the issue counts 45 of 39 cards.
    let given: std::collections::HashSet<u16> = prizes.arts.alts.values().flatten().copied().collect();
    let cids: std::collections::HashSet<u16> = given.iter().filter_map(|r| data.cards.by_row(*r)).map(|c| c.cid).collect();
    eprintln!("alternate rows: {}, cards: {}", given.len(), cids.len());
    assert!(given.len() >= 45 && cids.len() >= 39);
}

#[test]
fn the_npcs_give_66_recipes_and_the_shop_sells_the_theme_decks_and_the_other_starters() {
    let Some(data) = game() else { return };
    let prizes = yo3_server::game_prizes(&data);
    // The client has 132 NPC deck recipes: 75 played by the 25 selectable NPCs (ids 120-194, three levels each), less the
    // nine character decks every player owns, leaves 66. The other 57 belong to NPC ids 0-52 and 195-197 that the
    // client's NPC roster doesn't list: no one to beat for them.
    assert_eq!(prizes.npc_recipes.len(), 66, "the playable NPCs' decks less the nine every player has");
    let sold: Vec<&yo3_data::Deck> = data.decks.iter().filter(|d| yo3_server::recipe_deck(&data, d)).collect();
    assert_eq!(sold.iter().filter(|d| d.kind == 2).count(), 22, "the theme decks");
    assert_eq!(sold.iter().filter(|d| d.kind == 1).count(), 2, "the two other starters");
}
