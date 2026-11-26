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
