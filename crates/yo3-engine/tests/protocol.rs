//! The engine program over its JSON lines, as the server speaks to it (crates/yo3-server/src/engine.rs). The duel parts
//! skip without `engine/fetch.sh` run. (This test also makes `cargo test` build the program, which the server's tests
//! start from `target/`.)
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{Value as Json, json};

fn engine_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    dir.join("vendor/cardinfo.json").is_file().then_some(dir)
}

/// The answers to `requests`, one line each.
fn talk(requests: &[Json]) -> Vec<Json> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_yo3-engine"))
        .arg(engine_dir().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let answers = requests
        .iter()
        .map(|r| {
            writeln!(stdin, "{r}").unwrap();
            serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap()
        })
        .collect();
    drop(stdin);
    assert!(child.wait().unwrap().success(), "the engine ends cleanly when its input does");
    answers
}

#[test]
fn the_program_names_itself_and_its_core() {
    let out = Command::new(env!("CARGO_BIN_EXE_yo3-engine")).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("yo3-engine "));
}

#[test]
fn a_duel_starts_and_answers_with_konami_ids() {
    if engine_dir().is_none() {
        eprintln!("engine/fetch.sh has not run; skipping");
        return;
    }
    let deck = vec![4007; 40];
    let answers = talk(&[
        json!({"op": "new", "seed": 7, "lp": 8000, "decks": [{"main": deck, "extra": []}, {"main": deck, "extra": []}]}),
        json!({"op": "state"}),
        json!({"op": "nonsense"}),
        json!({"op": "cards", "cards": [{"cid": 4007}, {"cid": 1}]}),
        json!({"op": "identity"}),
    ]);
    let [new, state, nonsense, cards, identity] = &answers[..] else { unreachable!() };
    assert_eq!(new["status"], "await", "{new}");
    assert_eq!(new["errors"], json!([]));
    let drawn = new["messages"].as_array().unwrap().iter().find(|m| m["type"] == 90).expect("a draw");
    assert!(drawn["drawn"].as_array().unwrap().iter().all(|c| c["cid"] == 4007), "{drawn}");
    assert_eq!(state["status"], "state");
    let hand = state["teams"][1]["hand"].as_array().unwrap();
    assert_eq!(hand.len(), 5, "the second player's opening hand");
    assert!(hand.iter().all(|c| c["cid"] == 4007 && c["counters"] == json!({})), "{hand:?}");
    assert_eq!(state["teams"][1]["deck"], 35);
    assert_eq!(state["teams"][1]["lp"], 8000);
    assert_eq!(nonsense["status"], "error");
    assert_eq!(nonsense["messages"], json!([]));
    assert_eq!(cards["status"], "cards");
    assert_eq!(cards["cards"][0]["passcode"], 89631139, "Blue-Eyes White Dragon");
    assert_eq!(cards["cards"][0]["data"]["attack"], 3000);
    assert_eq!(cards["cards"][1]["passcode"], Json::Null);
    for key in ["sidecar", "core", "bindings", "scripts", "database", "card_ids"] {
        assert!(identity[key].as_str().is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())), "{key}");
    }
    assert!(identity["node"].as_str().is_some_and(|n| n.starts_with("native")));
}
