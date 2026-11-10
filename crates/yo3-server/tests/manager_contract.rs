//! Local desktop management must mutate the live authority, never a second copy of its database.
use yo3_server::{Config, start, admin::{self, ManagerOp}};

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
        admin::manage(&worker, &ManagerOp::CreateAccount { name: "Alice".into() }).unwrap();
        admin::manage(&worker, &ManagerOp::AddPlayer { who: "Alice".into(), name: "Bob".into() }).unwrap();
        let key = admin::manage(&worker, &ManagerOp::ExportAccount { who: "Alice".into() }).unwrap()["key"].as_str().unwrap().to_string();
        let accounts = admin::manage(&worker, &ManagerOp::Accounts).unwrap();
        assert!(admin::manage(&admin::Live { token: "wrong".into(), ..worker.clone() }, &ManagerOp::Accounts).is_err());
        (key, accounts)
    }).await.unwrap();
    assert!(accounts.to_string().contains("Alice") && accounts.to_string().contains("Bob"));
    assert!(!accounts.to_string().contains(&key), "account listings never reveal credentials");
    let endpoint = yo3_tunnel::Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    endpoint.login(&key).await.unwrap();
    let fresh = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json")).unwrap();
    assert!(fresh.has_key(&key), "live mutation committed to SQLite");
    assert_eq!(fresh.entries().find(|(_, k, _)| *k == Some(key.as_str())).unwrap().2.players.len(), 2);
    tokio::task::spawn_blocking(move || admin::manage(&live, &ManagerOp::Shutdown)).await.unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), server.wait()).await.unwrap();
}
