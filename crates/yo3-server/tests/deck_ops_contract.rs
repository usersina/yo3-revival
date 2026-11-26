//! TLS operation 8, a player's EDOPro decks (deck_ops.rs): Desktop's Multiplayer tab lists, imports and exports the
//! signed-in account's decks on a friend's server, under its session.
use serde_json::json;
use yo3_server::{Config, admin, api::{self, Op}, logins, start};
use yo3_tunnel::Endpoint;

async fn server(test: &str) -> (Endpoint, std::path::PathBuf, yo3_server::Running) {
    logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-decks-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = Config::ephemeral(dir);
    cfg.settings.push(("registration".into(), "open".into()));
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    (Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap(), data, running)
}

async fn signed_in(endpoint: &Endpoint) -> yo3_tunnel::Token {
    let pass = "CorrectHorse42";
    endpoint.account(&json!({ "op": "register", "user": "Yugi", "password": pass, "rules": true })).await.unwrap();
    let reply = endpoint.account(&json!({ "op": "login", "user": "Yugi", "password": pass })).await.unwrap();
    let hex = reply["session"].as_str().unwrap();
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

#[tokio::test]
async fn a_signed_in_player_imports_lists_and_exports_an_edopro_deck() {
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    let game = std::env::var_os("YO3_GAME_DIR").map(std::path::PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    if !engine.join("vendor/cardinfo.json").is_file() || !game.join("package/data002.ypk").is_file() {
        return;
    }
    // SAFETY: nextest runs each test in its own process, and the server reads it only once it has started.
    unsafe { std::env::set_var("YO3_ENGINE_DIR", &engine) };
    let (endpoint, data, _running) = server("import").await;
    let session = signed_in(&endpoint).await;
    // A new account's player comes from the game's Select Player; until then there is none to keep decks for.
    assert_eq!(endpoint.decks(&session, &json!({ "op": "decks" })).await.unwrap()["error"], "player");
    let live = admin::running(&data).unwrap();
    tokio::task::spawn_blocking(move || api::call(&live, &Op::AddPlayer { who: "Yugi".into(), name: "Yugi".into() })).await.unwrap().unwrap();
    let listed = endpoint.decks(&session, &json!({ "op": "decks" })).await.unwrap();
    assert_eq!(listed["ok"], true, "{listed}");
    let before = listed["decks"].as_array().map_or(0, Vec::len);
    // Three Blue-Eyes and a Dark Magician by an alternate artwork's passcode, and one unknown passcode.
    let ydk = "#main\n89631139\n89631139\n89631139\n46986414\n12345678\n#extra\n!side\n";
    let got = endpoint.decks(&session, &json!({ "op": "import", "name": "Dragons", "ydk": ydk })).await.unwrap();
    assert_eq!((got["ok"].as_bool(), got["deck"].as_str(), got["main"].as_u64()), (Some(true), Some("Dragons"), Some(4)), "{got}");
    assert_eq!(got["unknown"], json!([{ "passcode": 12345678, "copies": 1 }]));
    let listed = endpoint.decks(&session, &json!({ "op": "decks" })).await.unwrap();
    assert_eq!(listed["decks"].as_array().map_or(0, Vec::len), before + 1, "{listed}");
    let out = endpoint.decks(&session, &json!({ "op": "export", "deck": "Dragons" })).await.unwrap();
    let file = out["ydk"].as_str().unwrap_or_default();
    assert_eq!(file.lines().filter(|l| *l == "89631139").count(), 3, "{out}");
    assert!(out["link"].as_str().unwrap_or_default().starts_with("ydke://"));
}

#[tokio::test]
async fn a_deck_request_needs_a_live_session() {
    let (endpoint, _, _running) = server("session").await;
    let refused = endpoint.decks(&[7; 32], &json!({ "op": "decks" })).await.unwrap();
    assert_eq!((refused["ok"].as_bool(), refused["error"].as_str()), (Some(false), Some("session")));
    let session = signed_in(&endpoint).await;
    assert_eq!(endpoint.decks(&session, &json!({ "op": "drop-table" })).await.unwrap()["error"], "bad_request");
    endpoint.logout(&session).await.unwrap();
    assert_eq!(endpoint.decks(&session, &json!({ "op": "decks" })).await.unwrap()["error"], "session", "signed out");
}
