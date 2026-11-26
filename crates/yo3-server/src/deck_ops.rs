//! A player's EDOPro decks on any server they are signed in to (TLS operation 8, `yo3_tunnel::Client::decks`): the same
//! `decks`, `import` and `export` as the admin API's (`crate::ydk`), for the session's own account only. Desktop's
//! Multiplayer tab sends them for a friend's server, where its player is no administrator.
//!
//! The request is deflated JSON `{op: "decks"|"import"|"export", player?, name?, ydk?, deck?}` after the session; the
//! answer is deflated JSON, the result with `ok: true`, or `{ok: false, error}`.
use std::io::Read;
use std::sync::{Arc, Mutex};
use serde_json::{Value as Json, json};
use crate::{accounts::Directory, front::{Deck, World}, lock::Locked, ydk};

/// What the operations read of the server, made once: every card (no Tokens), the rows every player starts with, the
/// starter deck and the passcodes.
pub struct DeckData {
    cards: Vec<crate::gifts::Card>,
    starting: Vec<u16>,
    starter: Option<Deck>,
    passcodes: ydk::Passcodes,
}

impl DeckData {
    pub fn from_world(world: &World) -> Result<DeckData, String> {
        let dir = world.engine_dir.clone().or_else(crate::engine::Engine::dir).ok_or("the rules engine's folder is missing")?;
        let passcodes = ydk::Passcodes::load(&dir)?;
        let mut cards: Vec<crate::gifts::Card> = world.cards.iter().filter(|(row, _)| world.card_kinds.get(row) != Some(&10))
            .map(|(&row, s)| crate::gifts::Card { row, cid: s.cid, art: 0, name: world.card_names.get(&row).cloned().unwrap_or_default(), description: String::new() })
            .collect();
        cards.sort_by_key(|c| c.row);
        let starting = world.starter_deck.iter().chain(&world.official_decks).flat_map(Deck::cards).collect();
        Ok(DeckData { cards, starting, starter: world.starter_deck.clone(), passcodes })
    }
}

/// Inflates the request: at most [`yo3_tunnel::MAX_DECK_PLAIN`] bytes of a JSON object.
pub fn inflate(body: &[u8]) -> Option<Json> {
    let mut plain = Vec::new();
    flate2::read::DeflateDecoder::new(body).take(yo3_tunnel::MAX_DECK_PLAIN as u64 + 1).read_to_end(&mut plain).ok()?;
    if plain.len() > yo3_tunnel::MAX_DECK_PLAIN { return None }
    serde_json::from_slice::<Json>(&plain).ok().filter(Json::is_object)
}

fn refused(error: &str) -> Json { json!({ "ok": false, "error": error }) }

/// One request of the account `user_id` (the session's): its players only, the first when none is named.
pub fn handle(request: &Json, user_id: u32, accounts: &Arc<Mutex<Directory>>, data: &DeckData) -> Json {
    let mut d = accounts.locked();
    let Some(account) = d.account_of(user_id) else { return refused("session") };
    let mut players = account.players.clone();
    players.sort_by_key(|p| p.index);
    let named = request["player"].as_str().filter(|n| !n.is_empty());
    let Some(p) = (match named { Some(n) => players.iter().find(|p| p.name == n), None => players.first() }) else { return refused("player") };
    let Some(p) = d.player(p.player_id).cloned() else { return refused("player") };
    let decks = ydk::decks_of(&p, data.starter.as_ref());
    let cards = ydk::Cards { cards: &data.cards, starting: &data.starting, passcodes: &data.passcodes };
    let text = |k: &str| request[k].as_str().unwrap_or_default();
    let answer = match request["op"].as_str().unwrap_or_default() {
        "decks" => Ok(ydk::list(&p.name, &decks)),
        "export" => ydk::export_named(&p.name, &decks, text("deck"), &cards),
        "import" => ydk::import_for(&p, &decks, text("name"), text("ydk"), &cards).and_then(|kept| {
            d.update_player(p.player_id, |q| q.uploaded.push(kept.deck.clone()))?;
            tracing::info!(user = user_id, player = %p.name, deck = %kept.deck.name, "[decks] an EDOPro deck imported");
            Ok(kept.json(&p.name))
        }),
        _ => return refused("bad_request"),
    };
    match answer {
        Ok(mut v) => { v["ok"] = json!(true); v }
        Err(message) => json!({ "ok": false, "error": "refused", "message": message }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_acts_on_the_sessions_own_players_only() {
        let mut d = Directory::new(vec![]);
        let yugi = d.add_account(&["Yugi".into()]).unwrap();
        d.add_account(&["Kaiba".into()]).unwrap();
        let user = d.entries().find(|(_, _, a)| a.players.iter().any(|p| p.name == "Yugi")).map(|(_, _, a)| a.user_id).unwrap();
        let _ = yugi;
        let accounts = Arc::new(Mutex::new(d));
        let data = DeckData { cards: vec![crate::gifts::Card { row: 816, cid: 4007, art: 1, name: "Blue-Eyes White Dragon".into(), description: String::new() }],
            starting: vec![], starter: None, passcodes: ydk::Passcodes::default() };
        let listed = handle(&json!({ "op": "decks" }), user, &accounts, &data);
        assert_eq!((listed["ok"].as_bool(), listed["player"].as_str()), (Some(true), Some("Yugi")), "{listed}");
        assert_eq!(handle(&json!({ "op": "decks", "player": "Kaiba" }), user, &accounts, &data)["error"], "player", "another account's player");
        assert_eq!(handle(&json!({ "op": "decks" }), 999_999, &accounts, &data)["error"], "session");
        assert_eq!(handle(&json!({ "op": "nope" }), user, &accounts, &data)["error"], "bad_request");
        // No passcode is known here: a refusal with its words.
        let none = handle(&json!({ "op": "import", "ydk": "#main\n89631139\n" }), user, &accounts, &data);
        assert_eq!((none["ok"].as_bool(), none["error"].as_str()), (Some(false), Some("refused")), "{none}");
    }
}
