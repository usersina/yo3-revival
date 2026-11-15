//! A server that runs for long (#25): on SIGTERM, Ctrl+C or `yo3-server stop` it stops gracefully (no new logins, the
//! duels between players ended, every change kept, a last backup), and it renews its certificate without a restart.
use std::time::Duration;

use yo3_server::accounts::Directory;
use yo3_server::{Config, start};

#[tokio::test]
async fn a_stop_keeps_every_change_and_takes_a_last_backup() {
    let dir = std::env::temp_dir().join(format!("yo3-stop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config::ephemeral(dir);
    let data = cfg.data_dir.clone().unwrap();
    let running = start(cfg).await.unwrap();
    let auth = running.auth;
    let before = yo3_server::store::backups(&data).len();
    let live = yo3_server::admin::running(&data).unwrap();
    tokio::task::spawn_blocking(move || yo3_server::api::call(&live, &yo3_server::api::Op::GiveBp { who: "test".into(), amount: 77 }).unwrap()).await.unwrap();
    running.stopper().send_replace(true);
    tokio::time::timeout(Duration::from_secs(5), running.wait()).await.expect("stops promptly");
    assert!(tokio::net::TcpStream::connect(auth).await.is_err(), "no new connections");
    assert_eq!(yo3_server::store::backups(&data).len(), before + 1, "a backup at the stop");
    let d = Directory::open(vec![], &data.join("accounts.json")).unwrap();
    let account = d.entries().find(|(_, _, a)| a.players.iter().any(|p| p.name == "test")).unwrap().2.clone();
    assert_eq!(account.dpoint, yo3_server::accounts::START_BP + 77, "the change is kept");
}

/// The real program on SIGTERM. It needs the game's data to start, so it skips without the game.
#[cfg(unix)]
#[tokio::test]
async fn sigterm_stops_the_program_gracefully() {
    let game = yo3_data::default_game_dir();
    if yo3_data::GameData::load(&game).is_err() {
        eprintln!("no game data at {}; skipping", game.display());
        return;
    }
    let root = std::env::temp_dir().join(format!("yo3-sigterm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data = root.join("data");
    yo3_server::auth::initialize(&data, vec!["localhost".into()]).unwrap();
    let server = env!("CARGO_BIN_EXE_yo3-server");
    let child = tokio::process::Command::new(server).arg("--data-dir").arg(&data).arg("--log-dir").arg(root.join("logs"))
        .args(["--auth-port", "0", "--http-port", "0", "--no-packet-log"])
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).kill_on_drop(true).spawn().unwrap();
    let mut live = None;
    for _ in 0..200 {
        if let Some(found) = yo3_server::admin::running(&data) { live = Some(found); break }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(live.is_some(), "the server came up");
    let cli = |args: &[&str]| std::process::Command::new(server).args(args).arg("--data-dir").arg(&data).output().unwrap();
    assert!(cli(&["add-account", "Yugi"]).status.success());
    let before = yo3_server::store::backups(&data).len();
    let pid = child.id().unwrap().to_string();
    assert!(std::process::Command::new("kill").args(["-TERM", &pid]).status().unwrap().success());
    let out = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output()).await.expect("stops within 10 s").unwrap();
    let log = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a clean exit: {:?}\n{log}", out.status);
    assert!(log.contains("stop signal received") && log.contains("backed up at shutdown"), "{log}");
    assert_eq!(yo3_server::store::backups(&data).len(), before + 1);
    let accounts = String::from_utf8_lossy(&cli(&["accounts"]).stdout).into_owned();
    assert!(accounts.contains("Yugi"), "the change made just before is kept: {accounts}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[tokio::test]
async fn an_expired_certificate_is_renewed_while_the_server_runs() {
    // A server left running for a year served an expired certificate: launchers stopped connecting until a restart.
    // Now it checks while running, renews with the same key and names, and serves the new one.
    let dir = std::env::temp_dir().join(format!("yo3-renew-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = Config::ephemeral(dir);
    cfg.identity_check = Duration::from_millis(300);
    let (certificate, private_key) = (cfg.tls.certificate.clone(), cfg.tls.private_key.clone());
    let key_before = std::fs::read(&private_key).unwrap();
    let key = rcgen::KeyPair::from_pem(std::str::from_utf8(&key_before).unwrap()).unwrap();
    let mut expired = rcgen::CertificateParams::new(vec!["localhost".into()]).unwrap();
    expired.not_before = rcgen::date_time_ymd(2000, 1, 1);
    expired.not_after = rcgen::date_time_ymd(2001, 1, 1);
    std::fs::write(&certificate, expired.self_signed(&key).unwrap().pem()).unwrap();
    let running = start(cfg).await.unwrap();
    let info = || async { yo3_tunnel::Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap().info().await };
    assert!(info().await.is_err(), "the expired certificate is refused");
    let mut renewed = false;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if info().await.is_ok() { renewed = true; break }
    }
    assert!(renewed, "the renewed certificate is served without a restart");
    assert_eq!(std::fs::read(&private_key).unwrap(), key_before, "the same key: pinned launchers keep connecting");
    let pem = std::fs::read(&certificate).unwrap();
    let identity = yo3_tunnel::identity_from_pem(&pem).unwrap();
    assert_eq!(identity, yo3_tunnel::identity_from_pem(expired.self_signed(&key).unwrap().pem().as_bytes()).unwrap());
}
