//! Card profiles (#60, `profile.rs`): the client's card table, rewritten to show a card the way the server plays it.
//! The mapping from the rules engine's card data to the client's row words is checked against every card of the real
//! client; the proof cards (Flame Swordsman, Mystical Elf, Negate Attack) are checked end to end through the real map.
//! Skipped unless the game is installed (YO3_GAME_DIR or the default Wine path) and engine/fetch.sh has run.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use yo3_data::{GameData, default_game_dir};
use yo3_server::front::{Deck, World};
use yo3_server::profile::{CardView, Profile, Profiles, Stat, Target, client_attribute, client_kind, client_race, client_sub, read_anime};

const MYSTICAL_ELF: u16 = 4008;
const FLAME_SWORDSMAN: u16 = 4021;
const NEGATE_ATTACK: u16 = 4804;
/// Earthbound Immortal Wiraqocha Rasca, 1/1: shown exactly by the DLL's ATK/DEF hooks (docs/client-internals.md).
const WIRAQOCHA_RASCA: u16 = 8461;

fn game() -> Option<GameData> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    if !dir.join("package/data002.ypk").is_file() {
        eprintln!("game not installed; skipping");
        return None;
    }
    Some(GameData::load(&dir).expect("the client's data loads"))
}

fn engine_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    (dir.join("vendor/cardinfo.json").is_file() && yo3_server::engine::anime_problem(&dir).is_none()).then_some(dir).or_else(|| {
        eprintln!("anime data not fetched; skipping");
        None
    })
}

fn anime_profile(data: &GameData, dir: &std::path::Path) -> Profile {
    Profile::load("anime", data.cards.iter(), dir).expect("the anime profile builds")
}

/// Every Monster, Spell and Trap of the client against the official database's type, level, race and attribute: the
/// functions the profile writes the client's words with must give the numbers the client's own table has.
#[test]
fn the_clients_words_follow_from_the_engines_card_data_for_every_card() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    let ids: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("card-ids.json")).unwrap()).unwrap();
    let mut passcode: HashMap<u32, u64> = HashMap::new();
    for c in ids["data"].as_array().unwrap() {
        for m in c["misc_info"].as_array().into_iter().flatten() {
            if let Some(k) = m["konami_id"].as_u64().filter(|&k| k > 0) { passcode.entry(k as u32).or_insert(c["id"].as_u64().unwrap()); }
        }
    }
    let db = rusqlite::Connection::open_with_flags(dir.join("vendor/BabelCDB/cards.cdb"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (mut checked, mut wrong) = (0, vec![]);
    for card in data.cards.iter() {
        let Some(&code) = passcode.get(&u32::from(card.cid)) else { continue };
        let Ok((ty, level, race, attribute)) = db.query_row("select type, level & 255, race, attribute from datas where id = ?", [code as i64], |r| Ok((r.get::<_, i64>(0)? as u32, r.get::<_, i64>(1)? as u32, r.get::<_, i64>(2)? as u32, r.get::<_, i64>(3)? as u32))) else { continue };
        checked += 1;
        let monster = ty & 1 != 0;
        let mut ok = client_kind(ty) == Some(card.kind);
        if monster {
            ok &= card.level == level.min(15) && client_race(race) == Some(card.race) && client_attribute(attribute) == Some(card.attribute);
        } else {
            ok &= client_sub(ty).unwrap_or(0) == card.words[1] >> 16 & 7;
        }
        if !ok { wrong.push(format!("{} ({}): type {ty:#x} -> kind {} sub {}", card.name, card.cid, card.kind, card.words[1] >> 16 & 7)); }
    }
    assert!(checked > 4000, "{checked} cards checked");
    // The database and the client differ on a handful of cards (reprints, errata): the mapping holds for the rest.
    assert!(wrong.len() <= 12, "{} of {checked} differ: {wrong:#?}", wrong.len());
}

/// The map's entries by Konami id (`engine/anime-cards.json`).
fn map(dir: &std::path::Path) -> Vec<serde_json::Value> {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("anime-cards.json")).unwrap()).unwrap();
    json["cards"].as_array().unwrap().clone()
}

/// A card with no difference between the game's and its anime version keeps its row words: the profile never changes
/// what it does not have to. The cards that do change are the ones the map says show as the anime card (its `profile`
/// list: the differences the released categories cover), and each of those has a row that differs from the client's.
#[test]
fn a_card_the_anime_version_does_not_change_keeps_its_words() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    let profile = anime_profile(&data, &dir);
    assert!(profile.targets.len() > 300, "{} rows", profile.targets.len());
    let changed: std::collections::BTreeSet<u16> = profile.targets.values().filter(|t| t.words != t.original).map(|t| t.cid).collect();
    let shown: std::collections::BTreeSet<u16> = map(&dir).iter().filter(|c| c["status"] == "mapped" && c.get("profile").is_some()).map(|c| c["cid"].as_u64().unwrap() as u16).collect();
    for cid in &changed {
        assert!(shown.contains(cid), "Konami id {cid} would show other data than the client's own while the map has not released that difference");
    }
    // The other way: a flag-only difference (Flip, Special Summon only) is shown nowhere, so its row stays the client's.
    for cid in shown.difference(&changed) {
        let entry = map(&dir).into_iter().find(|c| c["cid"] == *cid).unwrap();
        let categories: Vec<&str> = entry["differs"].as_array().unwrap().iter().filter_map(|c| c.as_str()).collect();
        assert!(categories.iter().all(|c| ["flags", "atk", "def"].contains(c)), "{cid}: the map says {categories:?} but the row did not change");
    }
    assert!(changed.len() >= 3, "the proof cards change: {changed:?}");
}

#[test]
fn the_proof_cards_show_as_their_anime_versions() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    let profile = anime_profile(&data, &dir);
    let rows = |cid| profile.targets.values().filter(|t| t.cid == cid).collect::<Vec<_>>();
    // Flame Swordsman: the game's Fusion Monster is the anime's Normal Monster, a Main Deck card, on both artworks.
    let flame = rows(FLAME_SWORDSMAN);
    assert!(flame.len() >= 2, "both artworks: {flame:?}");
    let original = data.cards.by_cid(FLAME_SWORDSMAN).unwrap();
    assert!(original.extra_deck(), "the game's Flame Swordsman is a Fusion Monster");
    for t in &flame {
        assert_eq!((t.kind(), t.level(), t.atk.shown(), t.def.shown()), (0, 5, 1800, 1600));
        assert!(!t.extra_deck());
        assert_eq!(t.words[1] >> 27 & 7, data.cards.by_row(t.row).unwrap().art, "the artwork stays");
        assert_eq!(t.words[0] & 0x3FFF, u32::from(FLAME_SWORDSMAN));
    }
    // Mystical Elf: Normal in the game, an Effect Monster in the anime.
    assert_eq!(data.cards.by_cid(MYSTICAL_ELF).unwrap().kind, 0);
    assert!(rows(MYSTICAL_ELF).iter().all(|t| t.kind() == 1 && !t.extra_deck()));
    // Negate Attack: a Trap in the game, a Quick-Play Spell in the anime.
    assert_eq!(data.cards.by_cid(NEGATE_ATTACK).unwrap().kind, 14);
    assert!(rows(NEGATE_ATTACK).iter().all(|t| t.kind() == 13 && t.words[1] >> 16 & 7 == 5));
    // Their text is the anime card's, and the client file names them.
    let file = profile.render();
    assert!(file.contains(&format!(" {NEGATE_ATTACK} ")) && file.contains("\ntext "), "the file has cards and texts");
    assert_eq!(Profile::identify(&file), Some(("anime".into(), profile.hash())));
}

/// A deck keeps its cards and quantities when a card changes class, whichever profile is selected, and a card the profile
/// does not cover stays where the deck had it.
#[test]
fn decks_follow_the_selected_profile_and_keep_every_card() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    let profile = Arc::new(anime_profile(&data, &dir));
    let row_of = |cid| data.cards.by_cid(cid).unwrap().row;
    let (flame, elf, other) = (row_of(FLAME_SWORDSMAN), row_of(MYSTICAL_ELF), row_of(6960));
    let base = CardView { extra_deck_rows: data.cards.iter().filter(|c| c.extra_deck()).map(|c| c.row).collect(), ..Default::default() };
    let world = World { profiles: Profiles::with_anime(CardView::over(&base, profile)), extra_deck_rows: base.extra_deck_rows.clone(), ..World::new(vec![]) };
    let deck = Deck { id: 1, name: "d".into(), main: vec![elf, other, elf], extra: vec![flame, flame], side: vec![other] };
    assert_eq!(world.place_deck(&deck), deck, "the game's own cards: the deck is as it was");
    world.settings.set("anime-cards", "on").unwrap();
    let placed = world.place_deck(&deck);
    assert_eq!((placed.main.clone(), placed.extra.clone(), placed.side.clone()), (vec![elf, other, elf, flame, flame], vec![], vec![other]),
        "Flame Swordsman is a Main Deck card now");
    assert_eq!(world.place_deck(&placed), placed, "placing twice changes nothing");
    world.settings.reset("anime-cards").unwrap();
    let back = world.place_deck(&placed);
    assert_eq!((back.main, back.extra), (vec![elf, other, elf], vec![flame, flame]), "and an Extra Deck card again with the game's own cards selected");
}

#[test]
fn the_anime_data_the_profile_reads_matches_the_map() {
    let Some(dir) = engine_dir() else { return };
    let anime = read_anime(&dir).expect("the map and the database read");
    assert!(anime.len() >= 300 && anime.contains_key(&FLAME_SWORDSMAN) && anime[&NEGATE_ATTACK].ty & 2 != 0);
}

// One card of each released category (issue #60, `PROFILE_COVERS`): what the client's row says before and under the profile,
// and where a deck keeps the card. The category's real-core scenario is in tests/anime_cards.rs, and the self-play duel check
// holds the engine's field to the profile (tests/duel_check.rs).
const WALL_OF_IVY: u16 = 7872;
const FIENDS_SANCTUARY: u16 = 5632;
const BOOST_WARRIOR: u16 = 9234;
const PERFECT_MACHINE_KING: u16 = 5804;
const RAPID_WARRIOR: u16 = 8861;
const GRANADORA: u16 = 5695;
const CHIRON: u16 = 5810;
const PUMPKING: u16 = 4105;
const CHACU_CHALLHUA: u16 = 8308;
const MIND_CONTROL: u16 = 4817;
const DRAGON_CAPTURE_JAR: u16 = 4335;

/// The profile's rows of a card, each with the client's own row of it.
/// A check of one row of the profile against the client's own row of the card.
type Check = Box<dyn Fn(&Target, &yo3_data::Card)>;

fn rows_of<'a>(profile: &'a Profile, data: &'a GameData, cid: u16) -> Vec<(&'a Target, &'a yo3_data::Card)> {
    let rows: Vec<_> = profile.targets.values().filter(|t| t.cid == cid).map(|t| (t, data.cards.by_row(t.row).unwrap())).collect();
    assert!(!rows.is_empty(), "Konami id {cid} is not in the profile: its category is not released");
    rows
}

/// Whether the map released this card for this category: mapped, and the difference is one the category covers.
fn released(dir: &std::path::Path, cid: u16, category: &str) -> bool {
    map(dir).iter().any(|c| c["cid"] == cid && c["status"] == "mapped" && c["differs"].as_array().is_some_and(|d| d.iter().any(|x| x == category)))
}

#[test]
fn each_released_category_shows_in_the_clients_row() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    let profile = anime_profile(&data, &dir);
    let sub = |t: &Target| t.words[1] >> 16 & 7;
    let sub_of = |c: &yo3_data::Card| c.words[1] >> 16 & 7;
    let stat = Stat::Value;
    // (category, card, a check of the profile's row against the client's own)
    let checks: Vec<(&str, u16, Check)> = vec![
        // flags: Flip and Special Summon only are shown nowhere, so the row is the client's.
        ("flags", WALL_OF_IVY, Box::new(|t, c| assert_eq!(t.words, c.words, "a type flag the client does not show changes no word"))),
        ("subtype", FIENDS_SANCTUARY, Box::new(move |t, c| assert_eq!((sub_of(c), sub(t), t.kind()), (0, 5, 13), "a Normal Spell is a Quick-Play Spell"))),
        ("type", BOOST_WARRIOR, Box::new(|t, c| assert_eq!((c.kind, t.kind()), (1, 16), "an Effect Monster is an Effect Tuner"))),
        ("deck", PERFECT_MACHINE_KING, Box::new(|t, c| assert!(!c.extra_deck() && t.extra_deck() && t.kind() == 3, "a Main Deck monster is a Fusion Monster"))),
        ("level", RAPID_WARRIOR, Box::new(|t, c| assert_eq!((c.level, t.level()), (4, 3)))),
        ("attribute", GRANADORA, Box::new(|t, c| assert_eq!((c.attribute, t.attribute()), (client_attribute(0x2).unwrap(), client_attribute(0x20).unwrap()), "WATER is DARK"))),
        ("race", CHIRON, Box::new(|t, c| assert_eq!((c.race, t.race()), (client_race(0x8000).unwrap(), client_race(0x4000).unwrap()), "Beast-Warrior is Beast"))),
        ("atk", PUMPKING, Box::new(move |t, c| assert_eq!((c.atk, t.atk, t.stats().atk), (1800, stat(1500), 1500)))),
        ("atk", WIRAQOCHA_RASCA, Box::new(move |t, c| assert_eq!((c.atk, t.atk, t.atk.needs_exact(), t.words[0] >> 14 & 0x1FF), (100, stat(1), true, 0), "1 ATK is not a multiple of ten: the DLL answers the number"))),
        ("def", CHACU_CHALLHUA, Box::new(move |t, c| assert_eq!((c.def, t.def, t.stats().def), (2400, stat(1600), 1600)))),
        ("kind", MIND_CONTROL, Box::new(|t, c| assert_eq!((c.kind, t.kind()), (13, 14), "a Spell is a Trap"))),
        ("kind", DRAGON_CAPTURE_JAR, Box::new(|t, c| assert_eq!((c.kind, t.kind(), t.level(), t.atk.shown(), t.def.shown()), (14, 1, 2, 100, 200), "a Trap is an Effect Monster with stats"))),
    ];
    let mut proven = std::collections::BTreeSet::new();
    for (category, cid, check) in &checks {
        assert!(released(&dir, *cid, category), "{cid} is not released for {category}");
        for (t, c) in rows_of(&profile, &data, *cid) {
            check(t, c);
            assert!(t.differs.iter().any(|d| d == category), "{cid}: the profile's row does not name {category}: {:?}", t.differs);
        }
        proven.insert(*category);
    }
    // Every category the map has released is one of those proven here.
    let released: std::collections::BTreeSet<String> = map(&dir).iter().filter(|c| c["status"] == "mapped").flat_map(|c| c["differs"].as_array().into_iter().flatten().map(|d| d.as_str().unwrap().to_string()).collect::<Vec<_>>()).collect();
    for category in &released {
        assert!(proven.contains(category.as_str()), "category {category} is released without a card proving it here");
    }
}

/// deck: a card whose class changes is placed in the list the profile puts it in, by the server's own rule, in both
/// directions (Perfect Machine King, Main Deck -> Fusion, and Flame Swordsman, Fusion -> Main Deck), whichever list the
/// deck had it in.
#[test]
fn decks_move_cards_whose_class_changes_in_both_directions() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    if !released(&dir, PERFECT_MACHINE_KING, "deck") {
        return;
    }
    let profile = Arc::new(anime_profile(&data, &dir));
    let row_of = |cid| data.cards.by_cid(cid).unwrap().row;
    let (king, flame, other) = (row_of(PERFECT_MACHINE_KING), row_of(FLAME_SWORDSMAN), row_of(6960));
    let base = CardView { extra_deck_rows: data.cards.iter().filter(|c| c.extra_deck()).map(|c| c.row).collect(), ..Default::default() };
    let world = World { profiles: Profiles::with_anime(CardView::over(&base, profile)), extra_deck_rows: base.extra_deck_rows.clone(), ..World::new(vec![]) };
    world.settings.set("anime-cards", "on").unwrap();
    let deck = Deck { id: 1, name: "d".into(), main: vec![king, other, king], extra: vec![flame], side: vec![king] };
    let placed = world.place_deck(&deck);
    assert_eq!((placed.main.clone(), placed.extra.clone(), placed.side.clone()), (vec![other, flame], vec![king, king], vec![king]),
        "the Fusion goes to the Extra Deck, Flame Swordsman to the Main Deck, the Side Deck stays");
    assert_eq!(world.place_deck(&placed), placed, "placing twice changes nothing");
}

/// The Tuners the duel's Synchro summons read come from the profile: Boost Warrior is one only with it on.
#[test]
fn the_profile_makes_boost_warrior_a_tuner() {
    let (Some(data), Some(dir)) = (game(), engine_dir()) else { return };
    if !released(&dir, BOOST_WARRIOR, "type") {
        return;
    }
    let profile = Arc::new(anime_profile(&data, &dir));
    let base = CardView::default();
    let view = CardView::over(&base, profile);
    assert!(view.tuners.contains(&BOOST_WARRIOR) && !base.tuners.contains(&BOOST_WARRIOR));
}
