//! The rules engine sidecar (engine/duelcore.ts). Skipped unless engine/fetch.sh has run.
use std::path::PathBuf;

use yo3_server::engine::Engine;

fn engine() -> Option<Engine> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    Some(Engine::start(&dir).expect("node runs the engine"))
}

#[test]
fn a_duel_deals_five_each_and_asks_the_first_player() {
    let Some(mut engine) = engine() else { return };
    // 4007 = Blue-Eyes White Dragon, a Normal Monster (no script needed).
    let deck = [4007u16; 40];
    let answer = engine.new_duel(7, [(&deck, &[]), (&deck, &[])]).expect("a duel");
    assert_eq!(answer.status, "await", "{answer:?}");
    assert!(answer.errors.is_empty(), "{:?}", answer.errors);
    // MSG_DRAW (90) for both players, with the client's card ids.
    let draws: Vec<_> = answer.messages.iter().filter(|m| m["type"] == 90).collect();
    assert!(draws.len() >= 2);
    for (player, draw) in draws.iter().take(2).enumerate() {
        assert_eq!(draw["player"], player);
        let drawn = draw["drawn"].as_array().unwrap();
        assert_eq!(drawn.len(), 5);
        assert!(drawn.iter().all(|c| c["cid"] == 4007 && c["code"] == 89631139));
    }
}
