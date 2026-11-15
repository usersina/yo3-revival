//! The admin API (#25): the desktop's and the CLI's operations change the running server itself, never a second copy
//! of its database; each change is audited; and no player connection reaches it.
use yo3_server::{Config, start, admin, api::{self, Op}};

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
    assert!(leaderboard.starts_with("HTTP/1.1 200") && leaderboard.contains("Content-Type: text/html"), "{leaderboard}");
    assert!(leaderboard.contains("<title>Leaderboard</title>"));
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
    tokio::task::spawn_blocking(move || api::call(&live, &Op::Shutdown)).await.unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), server.wait()).await.unwrap();
}
