//! Host for friends shows the admin panel's address (#37) only when the hosted server answers there: the address the
//! launcher probes must be one the server's panel (#36) serves before any sign-in.
use std::io::{Read, Write};

use yo3_desktop::hosting::panel_path;
use yo3_server::{Config, admin, api::{self, Op}};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_launcher_probes_the_panel_where_the_server_serves_it() {
    let root = std::env::temp_dir().join(format!("yo3-desktop-panel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = Config::ephemeral(root.join("host"));
    let data = cfg.data_dir.clone().unwrap();
    let running = yo3_server::start(cfg).await.unwrap();
    let live = admin::running(&data).unwrap();
    let path = panel_path();
    let status = tokio::task::spawn_blocking(move || {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", live.port)).unwrap();
        write!(stream, "GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut head = String::new();
        stream.read_to_string(&mut head).unwrap();
        head.split_whitespace().nth(1).unwrap_or_default().to_string()
    }).await.unwrap();
    assert_eq!(status, "200", "the panel's sign-in page answers at {}", panel_path());
    let live = admin::running(&data).unwrap();
    tokio::task::spawn_blocking(move || api::call(&live, &Op::Shutdown)).await.unwrap().unwrap();
    running.wait().await;
}

/// A host using the launcher has no terminal on its server: Host for friends' Make admin gives its own account the
/// admin role, which the panel's sign-in asks for.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn make_admin_gives_the_hosts_account_the_panel() {
    yo3_server::logins::use_fast_hashing();
    let root = std::env::temp_dir().join(format!("yo3-desktop-make-admin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let backend = yo3_desktop::Backend::new(root.join("app")).unwrap();
    let mut cfg = Config::ephemeral(root.join("logs"));
    cfg.data_dir = Some(backend.hosting_dir());
    cfg.settings.push(("registration".into(), "open".into()));
    let running = yo3_server::start(cfg).await.unwrap();
    let server = yo3_desktop::multiplayer::first_contact(&format!("127.0.0.1:{}", running.auth.port())).await.unwrap();
    yo3_desktop::multiplayer::register(&server, "HostYugi", "LongPassphrase1", "LongPassphrase1", "", true).await.unwrap();
    let role = |backend: std::sync::Arc<yo3_desktop::Backend>| async move {
        tokio::task::spawn_blocking(move || {
            let live = admin::running(&backend.hosting_dir()).unwrap();
            let all = api::call(&live, &Op::Accounts).unwrap();
            all.as_array().unwrap().iter().find(|a| a["login"] == "HostYugi").map(|a| a["role"].clone()).unwrap()
        }).await.unwrap()
    };
    assert_eq!(role(backend.clone()).await, serde_json::json!("player"));
    let b = backend.clone();
    tokio::task::spawn_blocking(move || b.hosting_make_admin(" HostYugi ")).await.unwrap().unwrap();
    assert_eq!(role(backend.clone()).await, serde_json::json!("admin"));
    let b = backend.clone();
    assert!(tokio::task::spawn_blocking(move || b.hosting_make_admin("nobody")).await.unwrap().is_err(), "an unknown User ID says so");
    let live = admin::running(&backend.hosting_dir()).unwrap();
    tokio::task::spawn_blocking(move || api::call(&live, &Op::Shutdown)).await.unwrap().unwrap();
    running.wait().await;
}
