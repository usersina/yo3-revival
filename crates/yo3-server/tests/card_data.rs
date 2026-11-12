//! The card check: every card the client knows (its CARD_Prop table) goes through the rules engine, which must find the
//! card, load its script without an error and agree with the client on its level, ATK and DEF. A card that fails here
//! would play differently from what the client shows, or not at all. Skipped unless the game is installed (YO3_GAME_DIR
//! or the default Wine path) and engine/fetch.sh has run.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use serde_json::Value as Json;
use yo3_data::{GameData, default_game_dir};
use yo3_server::engine::Engine;

/// Cards of the client's that EDOPro doesn't have at all (game and promotion exclusives, YGOProDeck's `501…` ids):
/// they can't be played, so the deck check refuses them (`engine::NOT_IN_THE_ENGINE`). By Konami id, with names.
const NOT_IN_THE_ENGINE: [(u16, &str); 12] = [
    (5925, "Ulevo"),
    (5929, "Meteo the Matchless"),
    (6533, "King of Destruction - Xexex"),
    (6534, "Queen of Fate - Eternia"),
    (6958, "Testament of the Arcane Lords"),
    (6959, "Armament of the Lethal Lords"),
    (7374, "Chimaera, the Master of Beasts"),
    (7375, "Emperor of Lightning"),
    (7992, "Tyr, the Vanquishing Warlord"),
    (7993, "Lorelei, the Symphonic Arsenal"),
    (8610, "Aggiba, the Malevolent Sh'nn S'yo"),
    (8611, "Skuna, the Leonine Rakan"),
];

const TYPE_MONSTER: u64 = 0x1;
const TYPE_NORMAL: u64 = 0x10;
const TYPE_EFFECT: u64 = 0x20;
const TYPE_TOKEN: u64 = 0x4000;

fn game() -> Option<GameData> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    if !dir.join("package/data002.ypk").is_file() {
        eprintln!("game not installed; skipping");
        return None;
    }
    Some(GameData::load(&dir).expect("the client's data loads"))
}

fn engine() -> Option<Engine> {
    let dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    Some(Engine::start(&dir).expect("node runs the engine"))
}

/// Every Konami id the client knows (tokens, 3900–3999, come from card effects and are matched by name instead), with
/// its English name.
fn client_cards(game: &GameData) -> BTreeMap<u16, String> {
    let mut cards = BTreeMap::new();
    for c in game.cards.iter().filter(|c| !(3900..4000).contains(&c.cid)) {
        cards.entry(c.cid).or_insert_with(|| c.name.clone());
    }
    cards
}

fn checked(engine: &mut Engine, cards: &BTreeMap<u16, String>) -> Vec<Json> {
    let list: Vec<(u16, &str)> = cards.iter().map(|(&cid, name)| (cid, name.as_str())).collect();
    list.chunks(500).flat_map(|chunk| engine.cards(chunk).expect("the engine checks cards")).collect()
}

#[test]
fn every_client_card_loads_in_the_engine_as_the_client_shows_it() {
    let (Some(game), Some(mut engine)) = (game(), engine()) else { return };
    let cards = client_cards(&game);
    assert!(cards.len() > 4500, "{} cards", cards.len());
    let answers = checked(&mut engine, &cards);
    assert_eq!(answers.len(), cards.len());

    assert_eq!(NOT_IN_THE_ENGINE.map(|(cid, _)| cid), yo3_server::engine::NOT_IN_THE_ENGINE);
    let missing: BTreeSet<u16> = answers.iter().filter(|a| a["passcode"].is_null()).map(|a| a["cid"].as_u64().unwrap() as u16).collect();
    let expected: BTreeSet<u16> = NOT_IN_THE_ENGINE.iter().map(|&(cid, _)| cid).collect();
    assert_eq!(missing, expected, "cards the engine can't find (update NOT_IN_THE_ENGINE only for cards it really lacks)");

    let mut problems = vec![];
    for a in answers.iter().filter(|a| !a["passcode"].is_null()) {
        let cid = a["cid"].as_u64().unwrap() as u16;
        let name = &cards[&cid];
        let data = &a["data"];
        let kind = data["type"].as_u64().unwrap_or(0);
        if let Some(errors) = a["errors"].as_array().filter(|e| !e.is_empty()) {
            problems.push(format!("{cid} {name}: script errors {errors:?}"));
        }
        // c0.lua is asked for by every duel; a Normal Monster has no script.
        let scripts: Vec<&str> = a["missing_scripts"].as_array().into_iter().flatten().filter_map(Json::as_str).filter(|s| *s != "c0.lua").collect();
        let vanilla = kind & TYPE_MONSTER != 0 && kind & TYPE_NORMAL != 0 && kind & TYPE_EFFECT == 0;
        if !scripts.is_empty() && !vanilla {
            problems.push(format!("{cid} {name}: no script {scripts:?}"));
        }
        if kind & TYPE_MONSTER != 0 && kind & TYPE_TOKEN == 0 {
            let client = game.cards.by_cid(cid).unwrap();
            let (level, atk, def) = (data["level"].as_u64().unwrap_or(0) as u32, data["attack"].as_i64().unwrap_or(0), data["defense"].as_i64().unwrap_or(0));
            // "?" ATK/DEF is -2 in the engine.
            let differs = |client: u32, engine: i64| engine >= 0 && engine != client as i64;
            if level != client.level || differs(client.atk, atk) || differs(client.def, def) {
                problems.push(format!(
                    "{cid} {name}: client level {} ATK {} DEF {}, engine level {level} ATK {atk} DEF {def}",
                    client.level, client.atk, client.def
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{} card(s) the engine plays differently:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn no_pack_or_preset_deck_holds_a_card_the_engine_lacks() {
    let Some(game) = game() else { return };
    let lacking: HashMap<u16, &str> = NOT_IN_THE_ENGINE.into_iter().collect();
    let cid = |row: &u16| game.cards.by_row(*row).map(|c| c.cid);
    let mut found = vec![];
    for pack in &game.packs {
        found.extend(pack.cards.iter().filter_map(cid).filter(|c| lacking.contains_key(c)).map(|c| format!("pack {}: {}", pack.id, lacking[&c])));
    }
    for deck in &game.decks {
        found.extend(deck.cards().filter_map(|r| cid(&r)).filter(|c| lacking.contains_key(c)).map(|c| format!("deck {}: {}", deck.id, lacking[&c])));
    }
    assert!(found.is_empty(), "obtainable cards the engine lacks:\n{}", found.join("\n"));
}

#[test]
fn two_prints_of_a_card_keep_their_own_ids_in_one_duel() {
    let (Some(game), Some(mut engine)) = (game(), engine()) else { return };
    let names: HashMap<u16, String> = client_cards(&game).into_iter().collect();
    // 4041 and 5165 are both Dark Magician in the client; the engine's card list knows only 4041.
    assert_eq!(names[&4041], names[&5165]);
    let deck = [4041u16, 5165, 4041, 5165, 4041];
    let answer = engine.new_duel_named(3, 8000, [(&deck, &[]), (&deck, &[])], &names).expect("a duel");
    assert!(answer.errors.is_empty(), "{:?}", answer.errors);
    let state = engine.state().expect("state");
    let hand: Vec<u64> = state[0]["hand"].as_array().unwrap().iter().map(|c| c["cid"].as_u64().unwrap()).collect();
    assert_eq!(hand.iter().filter(|&&c| c == 4041).count(), 3, "{hand:?}");
    assert_eq!(hand.iter().filter(|&&c| c == 5165).count(), 2, "{hand:?}");
}
