//! The admin API (#25): the desktop's and the CLI's operations change the running server itself, never a second copy
//! of its database; each change is audited; and no player connection reaches it.
use yo3_server::{Config, start, admin, logins, api::{self, Op}};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accounts_can_be_created_and_exported_while_the_tls_server_stays_online() {
    let dir = std::env::temp_dir().join(format!("yo3-manager-contract-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config::ephemeral(dir.clone());
    let certificate = cfg.tls.certificate.clone();
    let data = cfg.data_dir.clone().unwrap();
    let server = start(cfg).await.unwrap();
    let live = admin::running(&data).unwrap();
    let worker = live.clone();
    let (key, accounts) = tokio::task::spawn_blocking(move || {
        api::call(&worker, &Op::CreateAccount { names: vec!["Alice".into()] }).unwrap();
        api::call(&worker, &Op::AddPlayer { who: "Alice".into(), name: "Bob".into() }).unwrap();
        let key = api::call(&worker, &Op::Key { who: "Alice".into() }).unwrap()["key"].as_str().unwrap().to_string();
        let accounts = api::call(&worker, &Op::Accounts).unwrap();
        assert!(api::call(&admin::Live { token: "wrong".into(), ..worker.clone() }, &Op::Accounts).is_err());
        // BP and BOOSTERPASS codes from the Desktop app: a random code, its claims (none yet), removal.
        assert_eq!(api::call(&worker, &Op::GiveBp { who: "Alice".into(), amount: 250 }).unwrap()["bp"], yo3_server::accounts::START_BP + 250);
        let made = api::call(&worker, &Op::AddCode { code: String::new(), bp: 500, uses: Some(3) }).unwrap()["code"].as_str().unwrap().to_string();
        let codes = api::call(&worker, &Op::Codes).unwrap();
        assert_eq!((codes[0]["code"].as_str(), codes[0]["bp"].as_u64(), codes[0]["uses"].as_u64()), (Some(made.as_str()), Some(500), Some(3)));
        assert_eq!(codes[0]["claims"].as_array().map(Vec::len), Some(0));
        assert!(api::call(&worker, &Op::AddCode { code: "short".into(), bp: 1, uses: None }).is_err());
        api::call(&worker, &Op::RemoveCode { code: made }).unwrap();
        assert_eq!(api::call(&worker, &Op::Codes).unwrap().as_array().map(Vec::len), Some(0));
        // Each change has its audit row, newest first, with the values before and after where there are some; reads
        // have none, and a password's hash or a key is never in one.
        let rows = api::call(&worker, &Op::Audit { limit: None }).unwrap()["rows"].clone();
        let ops: Vec<&str> = rows.as_array().unwrap().iter().map(|r| r["op"].as_str().unwrap()).collect();
        assert_eq!(ops, ["remove-code", "add-code", "give-bp", "key", "add-player", "create-account"]);
        assert_eq!((rows[2]["target"].as_str(), rows[2]["before"].as_u64(), rows[2]["after"].as_u64()),
            (Some("Alice"), Some(yo3_server::accounts::START_BP as u64), Some(yo3_server::accounts::START_BP as u64 + 250)));
        assert_eq!((rows[2]["actor"].as_str(), rows[2]["origin"].as_str()), (Some("host"), Some("server")));
        (key, accounts)
    }).await.unwrap();
    assert!(accounts.to_string().contains("Alice") && accounts.to_string().contains("Bob"));
    assert!(!accounts.to_string().contains(&key), "account listings never reveal credentials");
    let endpoint = yo3_tunnel::Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    endpoint.login(&key).await.unwrap();
    let fresh = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json")).unwrap();
    assert!(fresh.has_key(&key), "live mutation committed to SQLite");
    assert_eq!(fresh.entries().find(|(_, k, _)| *k == Some(key.as_str())).unwrap().2.players.len(), 2);
    // Duel logs are served from server.db; only plain .dlog names are looked up.
    yo3_server::store::put_duel_log(&data, "20261112-9-1001.dlog", b"log bytes").unwrap();
    let get = |path: &'static str| { let port = server.http.port(); async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).await.unwrap();
        let mut out = Vec::new(); s.read_to_end(&mut out).await.unwrap(); String::from_utf8_lossy(&out).into_owned()
    } };
    let found = get("/duellogs/20261112-9-1001.dlog").await;
    assert!(found.starts_with("HTTP/1.1 200") && found.ends_with("log bytes"), "{found}");
    assert!(get("/duellogs/..%2Fserver.key").await.starts_with("HTTP/1.1 404"));
    assert!(get("/duellogs/missing.dlog").await.starts_with("HTTP/1.1 404"));
    // The game's links to Konami's old website, sent here by the connection DLL (tools/mod/web-links.h): a page each.
    let leaderboard = get("/web/v3/events/rank_info?country=GB").await;
    assert!(leaderboard.starts_with("HTTP/1.1 200") && leaderboard.to_ascii_lowercase().contains("content-type: text/html"), "{leaderboard}");
    assert!(leaderboard.contains("<title>Leaderboard - ") && leaderboard.contains("<nav>"), "{leaderboard}");
    // The old routes are gone, and the API takes only POST with the token.
    for old in ["/admin?op=codes", "/manager"] { assert!(get(old).await.starts_with("HTTP/1.1 404"), "{old}") }
    assert!(get("/api/admin/v1/status").await.starts_with("HTTP/1.1 403"), "no token");
    // A player's launcher can't reach it through its TLS HTTP role, even with this PC's token.
    let session = endpoint.login(&key).await.unwrap();
    let ticket = endpoint.ticket(&session, yo3_tunnel::Role::Http).await.unwrap();
    let mut tunnel = endpoint.connect(&ticket, yo3_tunnel::Role::Http, 0).await.unwrap();
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let body = r#"{"op":"status"}"#;
        tunnel.write_all(format!("POST /api/admin/v1/status HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {}\r\nContent-Length: {}\r\n\r\n{body}", live.token, body.len()).as_bytes()).await.unwrap();
        let mut out = Vec::new(); let _ = tunnel.read_to_end(&mut out).await;
        assert!(String::from_utf8_lossy(&out).starts_with("HTTP/1.1 403"), "{}", String::from_utf8_lossy(&out));
    }
    tokio::task::spawn_blocking(move || api::stop(&live)).await.unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), server.wait()).await.unwrap();
}

/// Giving cards, an alternate artwork or a deck recipe (#19): stored in the player (CARD_Prop rows in its collection, the
/// recipe's item among its items), audited with the count before and after, an admin's only.
#[test]
fn unlock_cards_tops_every_artwork_up_to_three_for_every_player_once() {
    // Desktop's Unlock all cards (Settings → Singleplayer → Your data): every artwork of every catalogue card, topped up
    // to three copies beyond the starting cards, for every player of every account; a letter to each; a second time
    // adds nothing. A named player gets them alone; a moderator can't.
    use yo3_server::gifts::{Card, Gifts};
    let settings = std::sync::Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let mut d = yo3_server::accounts::Directory::new(vec![]);
    d.add_account(&["Yugi".into(), "Joey".into()]).unwrap();
    d.add_account(&["Kaiba".into()]).unwrap();
    let accounts = std::sync::Arc::new(std::sync::Mutex::new(d));
    let card = |row, cid, art, name: &str| Card { row, cid, art, name: name.into(), description: String::new() };
    let gifts = Gifts { cards: vec![card(816, 4007, 1, "Blue-Eyes White Dragon"), card(817, 4007, 2, "Blue-Eyes White Dragon"), card(1199, 5017, 1, "Command Knight")], recipes: vec![] };
    let a = api::Api::new(accounts.clone(), settings, None, std::path::PathBuf::new(), "server").with_gifts(gifts);
    let cards = |name: &str| { let d = accounts.lock().unwrap(); d.player(d.player_id_named(name).unwrap()).unwrap().cards.clone() };
    // Yugi has one extra Blue-Eyes and five Command Knights already: the Blue-Eyes is topped up, the Knights kept.
    a.apply(&Op::GiveCards { player: "Yugi".into(), card: "4007".into(), art: Some(1), count: Some(1) }, "host").unwrap();
    a.apply(&Op::GiveCards { player: "Yugi".into(), card: "5017".into(), art: None, count: Some(5) }, "host").unwrap();
    let one = a.apply(&Op::UnlockCards { player: Some("Joey".into()) }, "host").unwrap();
    assert_eq!((one["players"].as_u64(), one["added"].as_u64()), (Some(1), Some(9)));
    assert!(cards("Kaiba").is_empty(), "a named player gets them alone");
    let all = a.apply(&Op::UnlockCards { player: None }, "host").unwrap();
    assert_eq!((all["players"].as_u64(), all["added"].as_u64()), (Some(2), Some(2 + 3 + 9)), "Yugi's two Blue-Eyes and art 2, Kaiba's nine; Joey had them");
    assert_eq!(cards("Yugi").into_iter().collect::<Vec<_>>(), [(816, 3), (817, 3), (1199, 5)]);
    for name in ["Joey", "Kaiba"] { assert_eq!(cards(name).into_iter().collect::<Vec<_>>(), [(816, 3), (817, 3), (1199, 3)], "{name}") }
    assert_eq!(a.apply(&Op::UnlockCards { player: None }, "host").unwrap()["message"], "Every card was already unlocked.");
    assert!(a.apply(&Op::UnlockCards { player: Some("Nobody".into()) }, "host").is_err());
    assert!(!Op::UnlockCards { player: None }.for_moderators());
    let rows = a.audit(10).unwrap();
    assert_eq!((rows[0].op.as_str(), rows[0].target.as_str()), ("unlock-cards", "every player"));
}

#[test]
fn unlock_cards_for_every_player_happens_once_and_then_only_for_new_cards() {
    // Desktop's Unlock all cards, once: the server keeps which cards it unlocked (in the accounts' document), a player
    // made later starts with them, a second unlock gives nothing, and after an update adds a card only that one. `collection`
    // says what Desktop shows: the player's cards (starting cards included) out of all, and how many an unlock would give.
    use yo3_server::gifts::{Card, Gifts};
    let dir = std::env::temp_dir().join(format!("yo3-unlock-once-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let open = || yo3_server::accounts::Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    let mut d = open();
    d.add_account(&["Yugi".into()]).unwrap();
    let accounts = std::sync::Arc::new(std::sync::Mutex::new(d));
    let settings = std::sync::Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let card = |row, cid, art, name: &str| Card { row, cid, art, name: name.into(), description: String::new() };
    let three = vec![card(816, 4007, 1, "Blue-Eyes White Dragon"), card(817, 4007, 2, "Blue-Eyes White Dragon"), card(1199, 5017, 1, "Command Knight")];
    let api_with = |cards: Vec<Card>| api::Api::new(accounts.clone(), settings.clone(), Some(dir.clone()), std::path::PathBuf::new(), "server")
        .with_gifts(Gifts { cards, recipes: vec![] });
    let a = api_with(three.clone());
    let seen = |a: &api::Api| { let v = a.apply(&Op::Collection { player: None }, "host").unwrap(); (v["owned"].as_u64(), v["total"].as_u64(), v["new"].as_u64()) };
    assert_eq!(seen(&a), (Some(0), Some(3), Some(3)), "nothing held (no game data: no starting cards), everything to unlock");
    a.apply(&Op::GiveCards { player: "Yugi".into(), card: "5017".into(), art: None, count: Some(1) }, "host").unwrap();
    assert_eq!(seen(&a), (Some(1), Some(3), Some(3)), "a card given counts");
    assert_eq!(a.apply(&Op::UnlockCards { player: None }, "host").unwrap()["added"], 8);
    assert_eq!(seen(&a), (Some(3), Some(3), Some(0)));
    assert_eq!(open().unlocked_cards(), [816, 817, 1199], "kept with the accounts");
    assert_eq!(a.apply(&Op::UnlockCards { player: None }, "host").unwrap()["message"], "Every card was already unlocked.");
    // A player made after the unlock starts with every unlocked card.
    let user = accounts.lock().unwrap().entries().next().map(|(_, _, acc)| acc.user_id).unwrap();
    accounts.lock().unwrap().create_player(user, 1, "Joey").unwrap();
    let cards = |name: &str| { let d = accounts.lock().unwrap(); d.player(d.player_id_named(name).unwrap()).unwrap().cards.clone() };
    assert_eq!(cards("Joey").into_iter().collect::<Vec<_>>(), [(816, 3), (817, 3), (1199, 3)]);
    // An update adds a card: only it is new, and the next unlock gives only it.
    let mut four = three.clone();
    four.push(card(5217, 5183, 1, "Necro Wall"));
    let b = api_with(four);
    assert_eq!(seen(&b), (Some(3), Some(4), Some(1)));
    let more = b.apply(&Op::UnlockCards { player: None }, "host").unwrap();
    assert_eq!((more["players"].as_u64(), more["added"].as_u64()), (Some(2), Some(6)), "three Necro Walls for each of the two players");
    assert_eq!(seen(&b), (Some(4), Some(4), Some(0)));
    assert!(!Op::Collection { player: None }.changes());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_admin_gives_cards_artworks_and_recipes_and_a_moderator_cannot() {
    use yo3_server::gifts::{Card, Gifts, Recipe};
    let settings = std::sync::Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let mut d = yo3_server::accounts::Directory::new(vec![]);
    d.add_account(&["Yugi".into()]).unwrap();
    d.register(logins::Login::new("Pegasus", "HASH".into(), "RECOVERY".into()), None, logins::Registration::Open).unwrap();
    d.set_role("Pegasus", logins::Role::Admin).unwrap();
    let accounts = std::sync::Arc::new(std::sync::Mutex::new(d));
    let card = |row, cid, art, name: &str| Card { row, cid, art, name: name.into(), description: String::new() };
    let gifts = Gifts {
        cards: vec![card(816, 4007, 1, "Blue-Eyes White Dragon"), card(817, 4007, 2, "Blue-Eyes White Dragon")],
        recipes: vec![Recipe { id: 7, title: "E-HERO EVOLUTION DECK".into(), item: 769, cards: vec![816, 816, 816] }],
    };
    let a = api::Api::new(accounts.clone(), settings, None, std::path::PathBuf::new(), "server").with_gifts(gifts);
    let player = |f: &dyn Fn(&yo3_server::front::Player)| {
        let d = accounts.lock().unwrap();
        f(d.player(d.player_id_named("Yugi").unwrap()).unwrap())
    };
    let give = |card: &str, art, count| Op::GiveCards { player: "Yugi".into(), card: card.into(), art, count };
    let admin = Some(logins::Role::Admin);
    assert_eq!(a.apply_as(&give("blue-eyes white dragon", Some(2), Some(3)), "Pegasus", "panel", admin).unwrap()["count"], 3);
    assert_eq!(a.apply(&give("4007", Some(2), None), "host").unwrap()["count"], 4, "by Konami id, one copy");
    player(&|p| assert_eq!(p.cards.get(&817), Some(&4), "art 2's row"));
    for wrong in [give("Blue-Eyes White Dragon", Some(5), None), give("Nobody", None, None), give("4007", None, Some(0)), give("4007", None, Some(256)),
        Op::GiveCards { player: "Nobody".into(), card: "4007".into(), art: None, count: None }] {
        assert!(a.apply(&wrong, "host").is_err(), "{wrong:?}");
    }
    let recipe = |cards| Op::GiveRecipe { player: "Yugi".into(), deck: "e-hero evolution deck".into(), cards };
    a.apply(&recipe(false), "host").unwrap();
    player(&|p| assert!(p.items.contains(&769) && !p.cards.contains_key(&816), "the recipe alone"));
    assert!(a.apply(&recipe(false), "host").unwrap_err().contains("already has"));
    a.apply(&recipe(true), "host").unwrap();
    player(&|p| assert_eq!(p.cards.get(&816), Some(&3), "with its cards"));
    assert!(a.apply(&Op::Recipes, "host").unwrap()["message"].as_str().unwrap().contains("E-HERO EVOLUTION DECK"));
    // A moderator can't give anything, not even to their own players.
    let moderator = Some(logins::Role::Moderator);
    for op in [give("4007", None, None), recipe(true)] {
        assert!(!op.for_moderators());
        assert!(a.apply_as(&op, "Yugi", "panel", moderator).unwrap_err().starts_with("only an admin"), "{op:?}");
    }
    // Audited: who, what, and the count before and after.
    let rows = a.audit(10).unwrap();
    let ops: Vec<&str> = rows.iter().map(|r| r.op.as_str()).collect();
    assert_eq!(ops, ["give-recipe", "give-recipe", "give-cards", "give-cards"], "refusals and reads write no row");
    assert_eq!((rows[3].actor.as_str(), rows[3].target.as_str(), rows[3].before.clone(), rows[3].after.clone()),
        ("Pegasus", "Yugi", Some(serde_json::json!({ "card": "Blue-Eyes White Dragon", "art": 2, "count": 0 })),
            Some(serde_json::json!({ "card": "Blue-Eyes White Dragon", "art": 2, "count": 3 }))));
    assert_eq!(rows[3].detail.as_ref().unwrap()["card"], "blue-eyes white dragon");
}

#[test]
fn an_edopro_deck_is_imported_for_a_player_and_exported_back() {
    // Desktop's Import and Export on your own server: a .ydk file's passcodes become the game's rows (an artwork the
    // player holds first), the deck is kept with the player's decks under a free name, unknown passcodes are left out and
    // cards not held are named; export writes the deck back as a .ydk file and a ydke:// link.
    use yo3_server::gifts::{Card, Gifts};
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !engine.join("vendor/cardinfo.json").is_file() {
        return;
    }
    // SAFETY: nextest runs each test in its own process; nothing else reads the environment here.
    unsafe { std::env::set_var("YO3_ENGINE_DIR", &engine) };
    let dir = std::env::temp_dir().join(format!("yo3-ydk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let mut d = yo3_server::accounts::Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    d.add_account(&["Yugi".into()]).unwrap();
    let accounts = std::sync::Arc::new(std::sync::Mutex::new(d));
    let settings = std::sync::Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let card = |row, cid, art, name: &str| Card { row, cid, art, name: name.into(), description: String::new() };
    let cards = vec![card(816, 4007, 1, "Blue-Eyes White Dragon"), card(817, 4007, 2, "Blue-Eyes White Dragon"), card(1199, 5017, 1, "Command Knight")];
    let a = api::Api::new(accounts.clone(), settings, Some(dir.clone()), std::path::PathBuf::new(), "server").with_gifts(Gifts { cards, recipes: vec![] });
    // Yugi holds the second Blue-Eyes artwork: the import picks it.
    a.apply(&Op::GiveCards { player: "Yugi".into(), card: "4007".into(), art: Some(2), count: Some(1) }, "host").unwrap();
    // Blue-Eyes by an alternate artwork's passcode, Command Knight, and a passcode no card has.
    let ydk = "#created by Player\n#main\n89631139\n89631140\n89631139\n10375182\n12345678\n#extra\n!side\n10375182\n";
    let got = a.apply(&Op::ImportDeck { player: None, name: "Dragons".into(), ydk: ydk.into() }, "host").unwrap();
    assert_eq!((got["deck"].as_str(), got["main"].as_u64(), got["side"].as_u64()), (Some("Dragons"), Some(4), Some(1)), "{got}");
    assert_eq!(got["unknown"], serde_json::json!([{ "passcode": 12345678, "copies": 1 }]));
    assert_eq!(got["unowned"], serde_json::json!([{ "name": "Blue-Eyes White Dragon", "copies": 2 }, { "name": "Command Knight", "copies": 2 }]), "{got}");
    let kept = { let d = accounts.lock().unwrap(); d.player(d.player_id_named("Yugi").unwrap()).unwrap().uploaded.clone() };
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].main.clone(), kept[0].side.clone()), (vec![817, 817, 817, 1199], vec![1199]));
    // The same name again: a free one.
    let again = a.apply(&Op::ImportDeck { player: Some("Yugi".into()), name: "Dragons".into(), ydk: ydk.into() }, "host").unwrap();
    assert_eq!(again["deck"], "Dragons (2)");
    let listed = a.apply(&Op::Decks { player: None }, "host").unwrap();
    assert_eq!(listed["decks"].as_array().map(Vec::len), Some(2), "{listed}");
    // Export: the card's passcode (its first artwork's), as EDOPro reads it, and the same deck from the link.
    let out = a.apply(&Op::ExportDeck { player: None, deck: "Dragons".into() }, "host").unwrap();
    let file = out["ydk"].as_str().unwrap();
    assert_eq!(file, "#created by yo3-revival\n#main\n89631139\n89631139\n89631139\n10375182\n#extra\n!side\n10375182\n");
    let back = a.apply(&Op::ImportDeck { player: None, name: "Linked".into(), ydk: out["link"].as_str().unwrap().into() }, "host").unwrap();
    assert_eq!((back["main"].as_u64(), back["side"].as_u64()), (Some(4), Some(1)), "{back}");
    // A deck of nothing the server has, and a deck name that isn't there, are refused.
    assert!(a.apply(&Op::ImportDeck { player: None, name: "x".into(), ydk: "#main\n12345678\n".into() }, "host").is_err());
    assert!(a.apply(&Op::ExportDeck { player: None, deck: "Nope".into() }, "host").is_err());
    assert!(Op::ImportDeck { player: None, name: String::new(), ydk: String::new() }.changes() && !Op::ExportDeck { player: None, deck: String::new() }.changes());
    let _ = std::fs::remove_dir_all(&dir);
}
