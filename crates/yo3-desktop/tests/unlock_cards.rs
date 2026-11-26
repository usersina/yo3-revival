//! Unlock all cards (Settings → Singleplayer → Your data): on your own server's stopped data folder it makes the account the
//! first start would, gives every player three of every card artwork the game has, and a second time adds nothing. Its
//! count (`collection`): the starting cards out of every card before, every card after, and nothing more to unlock.
//! Needs the game's files (YO3_GAME_DIR or the default folder); skips without them.
use std::path::PathBuf;
use yo3_desktop::{Backend, Settings};

#[test]
fn unlock_all_cards_on_a_stopped_server_gives_every_card_to_the_first_player() {
    let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    if !game.join("yo3.exe").is_file() { eprintln!("no game; skipping"); return }
    let root = std::env::temp_dir().join(format!("yo3-unlock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let backend = Backend::new(root.join("app")).unwrap();
    let settings = Settings { game_dir: game.display().to_string(), data_dir: root.join("data").display().to_string(), player_name: "Yugi".into(), ..Settings::default() };
    let before = backend.collection(&settings).unwrap();
    assert!(!before.unlocked && before.new == before.total && before.total > 4000, "{before:?}");
    let sentence = backend.unlock_cards(&settings).unwrap();
    assert!(sentence.starts_with("Every card is unlocked: ") && sentence.ends_with(" cards added."), "{sentence}");
    let d = yo3_server::accounts::Directory::open(vec![], &root.join("data/accounts.json")).unwrap();
    let cards = &d.player(d.player_id_named("Yugi").unwrap()).unwrap().cards;
    assert!(cards.len() > 4000 && cards.values().all(|&n| n >= 3), "{} artworks", cards.len());
    assert_eq!(backend.unlock_cards(&settings).unwrap(), "Every card was already unlocked.");
    let after = backend.collection(&settings).unwrap();
    assert_eq!((after.owned, after.new, after.unlocked), (after.total, 0, true), "{after:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// EDOPro decks (Settings → Singleplayer → Your data) on a stopped server: a .ydk file is kept as the player's deck, the
/// deck is listed, and Export… writes it back with the same passcodes.
#[test]
fn an_edopro_deck_is_imported_and_exported_on_a_stopped_server() {
    let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !game.join("yo3.exe").is_file() || !engine.join("vendor/cardinfo.json").is_file() { eprintln!("no game or engine; skipping"); return }
    // SAFETY: nextest runs each test in its own process; nothing else reads the environment here.
    unsafe { std::env::set_var("YO3_ENGINE_DIR", &engine) };
    let root = std::env::temp_dir().join(format!("yo3-ydk-desktop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    // The account your server's first start makes.
    yo3_server::store::initialize(&root.join("data")).unwrap();
    let d = yo3_server::accounts::Directory::open(vec![], &root.join("data/accounts.json")).unwrap();
    let live = yo3_server::settings::Live::open(None, vec![]).unwrap();
    let api = yo3_server::api::Api::new(std::sync::Arc::new(std::sync::Mutex::new(d)), std::sync::Arc::new(live), Some(root.join("data")), game.clone(), "offline");
    api.apply_request(&yo3_server::api::Op::CreateAccount { names: vec!["Yugi".into()] }, "Desktop", "offline", None, &yo3_server::logins::new_token()).unwrap();
    drop(api);
    let backend = Backend::new(root.join("app")).unwrap();
    let settings = Settings { game_dir: game.display().to_string(), data_dir: root.join("data").display().to_string(), player_name: "Yugi".into(), ..Settings::default() };
    let file = root.join("Blue-Eyes.ydk");
    std::fs::write(&file, "#created by Player\n#main\n89631139\n89631139\n89631139\n#extra\n!side\n").unwrap();
    let sentence = backend.import_deck(&settings, &file).unwrap();
    assert!(sentence.starts_with("Imported \"Blue-Eyes\" for Yugi: 3 + 0 + 0 cards."), "{sentence}");
    assert!(backend.decks(&settings).unwrap().contains(&"Blue-Eyes".to_owned()));
    let out = root.join("out.ydk");
    backend.export_deck(&settings, "Blue-Eyes", &out).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "#created by yo3-revival\n#main\n89631139\n89631139\n89631139\n#extra\n!side\n");
    let _ = std::fs::remove_dir_all(&root);
}
