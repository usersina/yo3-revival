//! Every token a client card can make, against the client's own token cards: the token is shown (a client card by its
//! Konami id or by the name the rules engine gives it) and the position dialog it may be asked for before it appears
//! shows that card. Live, Gorz the Emissary of Darkness's token was asked its position with no card in the dialog,
//! and the duel stopped there; Sweet Corn's token is "Maize Token" to the engine and "Grain Corn Token" to the client.
//!
//! The tokens a card makes are the token passcodes its script names outside `listed_names` (our override first), or
//! its own passcode plus one to four (`id+1`). Skipped unless the game is installed and engine/fetch.sh has run.
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use serde_json::json;
use yo3_data::{GameData, default_game_dir};
use yo3_proto::Value;
use yo3_server::translate::{CardStats, Prompt, Translator};

const TYPE_TOKEN: i64 = 0x4000;

fn dirs() -> Option<(PathBuf, PathBuf)> {
    let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let engine = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !game.join("package/data002.ypk").is_file() || !engine.join("vendor/cardinfo.json").is_file() {
        eprintln!("game not installed or engine not fetched; skipping");
        return None;
    }
    Some((game, engine))
}

/// A token as the rules engine's messages carry it: passcode, Konami id (0 for none) and name.
#[derive(Clone, Debug)]
struct Token {
    code: u32,
    cid: u16,
    name: String,
}

/// Each token a client card's script makes, with the client cards (Konami ids) that make it.
fn tokens_made(game: &GameData, engine: &std::path::Path) -> BTreeMap<u32, (Token, Vec<u16>)> {
    let info: serde_json::Value = serde_json::from_slice(&std::fs::read(engine.join("vendor/cardinfo.json")).unwrap()).unwrap();
    let konami: HashMap<u32, u16> = info["data"].as_array().unwrap().iter()
        .filter_map(|c| Some((c["id"].as_u64()? as u32, c["misc_info"][0]["konami_id"].as_u64()? as u16)))
        .collect();
    let passcode: HashMap<u16, u32> = konami.iter().map(|(&p, &k)| (k, p)).collect();
    let db = rusqlite::Connection::open(engine.join("vendor/BabelCDB/cards.cdb")).unwrap();
    let mut query = db.prepare("SELECT d.id, t.name FROM datas d JOIN texts t ON t.id = d.id WHERE d.type & ?1").unwrap();
    let tokens: HashMap<u32, String> = query.query_map([TYPE_TOKEN], |r| Ok((r.get::<_, i64>(0)? as u32, r.get(1)?)))
        .unwrap().map(Result::unwrap).collect();
    let number = regex_numbers();
    let mut made: BTreeMap<u32, (Token, Vec<u16>)> = BTreeMap::new();
    for card in game.cards.iter().filter(|c| !(3900..4000).contains(&c.cid)) {
        let Some(&code) = passcode.get(&card.cid) else { continue };
        let script = [engine.join(format!("overrides/c{code}.lua")), engine.join(format!("vendor/CardScripts/official/c{code}.lua"))]
            .into_iter().find_map(|p| std::fs::read_to_string(p).ok());
        let Some(script) = script.filter(|s| s.contains("CreateToken")) else { continue };
        // The cards a script lists aren't all ones it makes (Spider Egg's lists the Speedroid Token).
        let script: String = script.lines().filter(|l| !l.contains("listed_names")).collect::<Vec<_>>().join("\n");
        let named = number(&script).into_iter().filter(|n| tokens.contains_key(n));
        let own = (1..=4).filter(|n| script.contains(&format!("id+{n}"))).map(|n| code + n).filter(|n| tokens.contains_key(n));
        for token in named.chain(own) {
            let entry = made.entry(token).or_insert_with(|| {
                (Token { code: token, cid: konami.get(&token).copied().unwrap_or(0), name: tokens[&token].clone() }, vec![])
            });
            if !entry.1.contains(&card.cid) {
                entry.1.push(card.cid);
            }
        }
    }
    made
}

/// The numbers of five digits or more in a script.
fn regex_numbers() -> impl Fn(&str) -> Vec<u32> {
    |text: &str| {
        text.split(|c: char| !c.is_ascii_digit()).filter(|w| w.len() >= 5 && w.len() <= 9).filter_map(|w| w.parse().ok()).collect()
    }
}

/// A translator with the client's cards and token cards, as a server builds it (`lib.rs`).
fn translator(game: &GameData) -> Translator {
    let cards: HashMap<u16, CardStats> = game.cards.iter()
        .map(|c| (c.row, CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race }))
        .collect();
    let (mut t, _) = Translator::new(0, [&[], &[]], cards);
    t.tokens(yo3_server::token_rows(&game.cards));
    t
}

#[test]
fn every_token_a_client_card_makes_has_a_client_card_and_shows_in_its_position_dialog() {
    let Some((game_dir, engine)) = dirs() else { return };
    let game = GameData::load(&game_dir).unwrap();
    let made = tokens_made(&game, &engine);
    assert!(made.len() >= 50, "the scripts' tokens weren't found: {}", made.len());
    let row_of: HashMap<u16, u16> = game.cards.iter().map(|c| (c.row, c.cid)).collect();
    let mut failures = vec![];
    for (token, makers) in made.values() {
        let what = format!("{} ({}, Konami id {}) made by {makers:?}", token.name, token.code, token.cid);
        // The dialog the player gets for its position (Attack or Defense: 5), its player the client.
        let mut t = translator(&game);
        let ask = json!({ "type": 19, "player": 0, "code": token.code, "cid": token.cid, "name": token.name, "positions": 5 });
        let Prompt::Client(out) = t.prompt(&ask) else {
            failures.push(format!("{what}: no position dialog"));
            continue;
        };
        let commands: Vec<(u32, [u32; 4])> = out.iter()
            .filter(|m| m.get("duelRequestName") == Some(&Value::U32(0x11)))
            .map(|m| {
                let int = |k| m.get(k).and_then(Value::as_u32).unwrap_or(0);
                (int("id"), [int("param1"), int("param2"), int("param3"), int("param4")])
            })
            .collect();
        let Some(&(_, [_, _, shown, _])) = commands.iter().find(|(id, p)| *id == 0x36 && p[0] == 8) else {
            failures.push(format!("{what}: no position dialog in {commands:?}"));
            continue;
        };
        let uid = shown & 0xFFFF;
        let row = commands.iter().find(|(id, p)| *id == 1000 && p[0] & 0xFF_FFFF == uid).map(|(_, p)| p[1] as u16);
        let Some(row) = row.filter(|r| row_of.get(r).is_some_and(|cid| (3900..4000).contains(cid))) else {
            failures.push(format!("{what}: the dialog names uid {uid}, with no token card in the card table ({commands:?})"));
            continue;
        };
        if !commands.iter().any(|(id, p)| *id == 0x3ED && p.contains(&(5 << 28 | row_of[&row] as u32))) {
            failures.push(format!("{what}: the dialog doesn't name its card {} ({commands:?})", row_of[&row]));
        }
        // It appears as that card, with that uid.
        let nowhere = json!({ "controller": 0, "location": 0, "sequence": 0, "position": 0 });
        let to = json!({ "controller": 0, "location": 4, "sequence": 2, "position": 1 });
        let moved = t.events(&[json!({ "type": 50, "code": token.code, "cid": token.cid, "name": token.name, "from": nowhere, "to": to })]);
        let entry = moved.iter().find(|m| m.get("id") == Some(&Value::U32(1000)));
        let same = entry.is_some_and(|m| m.get("param1") == Some(&Value::U32(uid)) && m.get("param2") == Some(&Value::U32(row as u32)));
        if !same {
            failures.push(format!("{what}: appeared as another card than its dialog's (uid {uid}, row {row}): {moved:?}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} tokens:\n{}", failures.len(), made.len(), failures.join("\n"));
}
