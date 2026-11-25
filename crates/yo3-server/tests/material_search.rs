//! Cards that widen a summon's materials (Synchro Materials from the hand, Ritual Materials from the Graveyard...) make
//! the rules engine search more combinations each time it lists the summons a player can make. The anime T.G. Cyber
//! Magician took any monster in the hand as a Synchro Material and froze the engine past the live server's 30 s limit
//! (self-play, 2026-11-25). Every card the engine plays whose script has such an effect (the game's cards, the added
//! ones and the mapped anime versions, found by scanning their scripts, so a new card is covered without a line here)
//! goes on a crowded board: three copies (hand, field, Graveyard), Tuners and non-Tuners on the field and in the hand,
//! Ritual Spells with their monsters, and an Extra Deck of Synchro Monsters. The first prompt that lists the summons
//! must come within `YO3_SLOW_ANSWER_SECS` (5 s by default, as the self-play's slow-answer rule).
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use yo3_data::{GameData, default_game_dir};
use yo3_server::duelcheck::scenario::{Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

/// The effects that widen a summon's materials.
const WIDENING: [&str; 7] = ["EFFECT_HAND_SYNCHRO", "EFFECT_EXTRA_SYNCHRO_MATERIAL", "EFFECT_EXTRA_MATERIAL", "EFFECT_EXTRA_FUSION_MATERIAL",
    "EFFECT_EXTRA_RITUAL_MATERIAL", "EFFECT_SYNCHRO_MATERIAL_CUSTOM", "EFFECT_HAND_LINK"];

const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf, Level 4
const JUNK_SYNCHRON: u16 = 7687; // a Level 3 Tuner
const TURBO_SYNCHRON: u16 = 7870; // a Level 1 Tuner
const BLACK_LUSTER_RITUAL: u16 = 4676;
const BLACK_LUSTER_SOLDIER: u16 = 4370;
const BLACK_MAGIC_RITUAL: u16 = 4727;
const MAGICIAN_OF_BLACK_CHAOS: u16 = 4728;
/// Synchro Monsters of Levels 5 to 8.
const EXTRA: [u16; 10] = [7734, 7735, 8137, 7898, 8090, 7696, 7697, 7736, 7733, 8860];

fn slow_answer() -> Duration {
    Duration::from_secs_f64(std::env::var("YO3_SLOW_ANSWER_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(5.0))
}

fn game() -> Option<GameData> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    dir.join("package/data002.ypk").is_file().then(|| GameData::load(&dir).expect("the client's data loads"))
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

/// A script's text as the engine loads it: ours (engine/overrides/) first, else ProjectIgnis'.
fn script(dir: &Path, rel: &str) -> String {
    let name = Path::new(rel).file_name().unwrap();
    std::fs::read_to_string(dir.join("overrides").join(name)).or_else(|_| std::fs::read_to_string(dir.join("vendor/CardScripts").join(rel))).unwrap_or_default()
}

fn widens(text: &str) -> Option<&'static str> {
    WIDENING.iter().copied().find(|w| text.contains(w))
}

/// (client id, anime cards on, the card's name for the report, the effect found) for every card the engine plays with a
/// widening effect.
fn widening_cards(dir: &Path, game: &GameData) -> Vec<(u16, bool, String, &'static str)> {
    let mut out = vec![];
    let in_game: std::collections::HashSet<u16> = game.cards.iter().map(|c| c.cid).collect();
    let ids = json(&dir.join("card-ids.json"));
    for d in ids["data"].as_array().unwrap() {
        let (Some(passcode), Some(konami)) = (d["id"].as_u64(), d["misc_info"][0]["konami_id"].as_u64()) else { continue };
        if konami > u16::MAX as u64 || !in_game.contains(&(konami as u16)) {
            continue;
        }
        if let Some(w) = widens(&script(dir, &format!("official/c{passcode}.lua"))) {
            out.push((konami as u16, false, format!("game card {konami}"), w));
        }
    }
    for c in json(&dir.join("added-cards.json"))["cards"].as_array().unwrap() {
        let anime_only = c["anime_only"].as_bool().unwrap_or(false);
        let folder = if anime_only { "unofficial" } else { "official" };
        if let Some(w) = widens(&script(dir, &format!("{folder}/c{}.lua", c["passcode"]))) {
            out.push((c["cid"].as_u64().unwrap() as u16, anime_only, c["name"].as_str().unwrap_or("").to_string(), w));
        }
    }
    for c in json(&dir.join("anime-cards.json"))["cards"].as_array().unwrap() {
        let (Some("mapped"), Some(rel)) = (c["status"].as_str(), c["script"].as_str()) else { continue };
        if let Some(w) = widens(&script(dir, rel)) {
            out.push((c["cid"].as_u64().unwrap() as u16, true, format!("{} (anime)", c["name"].as_str().unwrap_or("")), w));
        }
    }
    out
}

#[test]
fn the_summons_a_wider_material_card_allows_are_listed_in_time() {
    let (Some(dir), Some(game)) = (engine_dir(), game()) else { return };
    let cards = widening_cards(&dir, &game);
    // The game's own: the Djinns of Rituals and the hand-Synchro Tuners Eccentric Boy and Malefic Parallel Gear.
    assert!(cards.len() >= 7, "the scan finds the game's cards: {cards:?}");
    let limit = slow_answer();
    let mut slow = vec![];
    for (cid, anime, name, effect) in &cards {
        let mut spawns: Vec<Spawn> = vec![Spawn { team: 0, place: Hand, cid: *cid }, Spawn { team: 0, place: Monster, cid: *cid },
            Spawn { team: 0, place: Grave, cid: *cid }];
        for (place, card) in [(Monster, WARWOLF), (Monster, KURIBOH), (Monster, JUNK_SYNCHRON), (Hand, WARWOLF), (Hand, KURIBOH),
            (Hand, TURBO_SYNCHRON), (Hand, BLACK_LUSTER_RITUAL), (Hand, BLACK_LUSTER_SOLDIER), (Hand, BLACK_MAGIC_RITUAL),
            (Hand, MAGICIAN_OF_BLACK_CHAOS), (Grave, WARWOLF)] {
            spawns.push(Spawn { team: 0, place, cid: card });
        }
        let decks = [(&[WARWOLF; 40][..], &EXTRA[..]), (&[KURIBOH; 40][..], &[][..])];
        let started = Instant::now();
        let s = if *anime { Scenario::start_anime_field(&dir, 3, 8000, decks, &spawns) } else { Scenario::start_field(&dir, 3, 8000, decks, &spawns) };
        let mut s = s.unwrap_or_else(|e| panic!("{name} ({cid}): the duel didn't start: {e}"));
        s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD && p["player"] == 0).unwrap_or_else(|e| panic!("{name} ({cid}): {e}"));
        let took = started.elapsed();
        if took > limit {
            slow.push(format!("{name} ({cid}, {effect}): the first prompt took {:.1} s", took.as_secs_f64()));
        }
    }
    assert!(slow.is_empty(), "listing the summons took longer than {:.1} s (the live limit is 30 s):\n{}", limit.as_secs_f64(), slow.join("\n"));
}
