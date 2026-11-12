//! The engine's half of the card check, which needs no game files, so CI runs it, also against the release bundle's
//! engine (`YO3_ENGINE_DIR`): every Konami id of the client's era (below 10000; the client's cards end near 9,700)
//! in the pinned card ids with a real passcode must reach one the card database holds, with card data, and load its script without
//! an error. tests/card_data.rs checks the client's own cards against the engine where the game is installed.
use std::path::PathBuf;

use serde_json::Value as Json;
use yo3_server::engine::Engine;

/// OCG cards of the era the card database lacks (Assault Lion, Thoroughbred Elf, Power Connection): not in the client
/// (tests/card_data.rs), so no duel needs them. Listed so that a change in the pinned data shows.
const NOT_IN_THE_DATABASE: [u16; 3] = [6631, 6634, 6652];

const TYPE_MONSTER: u64 = 0x1;
const TYPE_NORMAL: u64 = 0x10;
const TYPE_EFFECT: u64 = 0x20;
const TYPE_TOKEN: u64 = 0x4000;

fn engine_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    Some(dir)
}

#[test]
fn every_card_of_the_clients_era_loads_in_the_engine() {
    let Some(dir) = engine_dir() else { return };
    let ids: Json = serde_json::from_slice(&std::fs::read(dir.join("vendor/cardinfo.json")).unwrap()).unwrap();
    let mut cids: Vec<u16> = ids["data"]
        .as_array()
        .unwrap()
        .iter()
        // YGOPRODeck gives game exclusives placeholder ids above 100,000,000: no card EDOPro has (see
        // engine::NOT_IN_THE_ENGINE for the client's).
        .filter(|c| c["id"].as_u64().is_some_and(|id| id < 100_000_000))
        .flat_map(|c| c["misc_info"].as_array().into_iter().flatten().filter_map(|m| m["konami_id"].as_u64()))
        .filter(|&k| (4000..10000).contains(&k) && !NOT_IN_THE_DATABASE.contains(&(k as u16)))
        .map(|k| k as u16)
        .collect();
    cids.sort_unstable();
    cids.dedup();
    assert!(cids.len() > 4500, "{} Konami ids", cids.len());
    let mut engine = Engine::start(&dir).expect("node runs the engine");
    let list: Vec<(u16, &str)> = cids.iter().map(|&c| (c, "")).collect();
    let answers: Vec<Json> = list.chunks(500).flat_map(|chunk| engine.cards(chunk).expect("the engine checks cards")).collect();
    let mut problems = vec![];
    for a in &answers {
        let cid = &a["cid"];
        let Some(passcode) = a["passcode"].as_u64() else {
            problems.push(format!("{cid}: no passcode"));
            continue;
        };
        let data = &a["data"];
        if data.is_null() {
            problems.push(format!("{cid}: passcode {passcode} has no card data"));
            continue;
        }
        let name = data["name"].as_str().unwrap_or("");
        if let Some(errors) = a["errors"].as_array().filter(|e| !e.is_empty()) {
            problems.push(format!("{cid} {name}: script errors {errors:?}"));
        }
        let kind = data["type"].as_u64().unwrap_or(0);
        let vanilla = kind & TYPE_MONSTER != 0 && kind & TYPE_NORMAL != 0 && kind & TYPE_EFFECT == 0;
        let scripts: Vec<&str> = a["missing_scripts"].as_array().into_iter().flatten().filter_map(Json::as_str).filter(|s| *s != "c0.lua").collect();
        if !scripts.is_empty() && !vanilla && kind & TYPE_TOKEN == 0 {
            problems.push(format!("{cid} {name}: no script {scripts:?}"));
        }
    }
    assert!(problems.is_empty(), "{} card(s) the engine can't play:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn the_installed_engine_data_is_the_pinned_data() {
    // engine/fetch.sh pins the card scripts' and database's commits and copies the pinned card ids: the tests (and the
    // release, which runs fetch.sh) use exactly that data. A checkout of other commits (an old install, or a manual
    // update) fails here; `fetch.sh` moves it back, `fetch.sh --update` moves the pins.
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !engine.join("vendor/cardinfo.json").is_file() || std::env::var_os("YO3_ENGINE_DIR").is_some() {
        eprintln!("engine not fetched here; skipping");
        return;
    }
    let fetch = std::fs::read_to_string(engine.join("fetch.sh")).unwrap();
    let pin = |key: &str| fetch.lines().find_map(|l| l.strip_prefix(key)).map(str::to_string).expect("a pin");
    let head = |repo: &str| {
        let out = std::process::Command::new("git").args(["-C", repo, "rev-parse", "HEAD"]).current_dir(&engine).output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    assert_eq!(head("vendor/CardScripts"), pin("SCRIPTS_COMMIT="), "card scripts: run engine/fetch.sh");
    assert_eq!(head("vendor/BabelCDB"), pin("DATABASE_COMMIT="), "card database: run engine/fetch.sh");
    let installed = std::fs::read(engine.join("vendor/cardinfo.json")).unwrap();
    assert!(installed == std::fs::read(engine.join("card-ids.json")).unwrap(), "card ids: run engine/fetch.sh");
}
