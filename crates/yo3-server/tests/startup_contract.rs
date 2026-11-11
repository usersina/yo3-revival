//! A normal host must never accept players with an empty collection because game data failed to load.
use std::time::Duration;
use std::process::Stdio;

async fn refuses_game_data(case: &str, corrupt: bool) {
    let root = std::env::temp_dir().join(format!("yo3-startup-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data = root.join("data"); let game = root.join("game");
    yo3_server::auth::initialize(&data, vec!["localhost".into()]).unwrap();
    let key = std::fs::read(data.join("server.key")).unwrap();
    let preserved = data.join("existing-player-deck.y3d"); std::fs::write(&preserved, b"keep existing data").unwrap();
    if corrupt {
        std::fs::create_dir_all(game.join("package")).unwrap();
        std::fs::write(game.join("package/data002.ypk"), b"incomplete archive").unwrap();
    }
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_yo3-server"))
        .arg("--data-dir").arg(&data).arg("--game-dir").arg(&game)
        .args(["--auth-port", "0", "--http-port", "0"]).arg("--log-dir").arg(root.join("logs"))
        .stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true).spawn().unwrap();
    let output = tokio::time::timeout(Duration::from_secs(3), child.wait_with_output()).await
        .expect("hosting must fail before accepting logins with no game data").unwrap();
    assert!(!output.status.success());
    let error = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(error.contains("game data") && error.contains("--game-dir") && error.contains("1.153"), "{error}");
    assert_eq!(std::fs::read(preserved).unwrap(), b"keep existing data");
    assert_eq!(std::fs::read(data.join("server.key")).unwrap(), key);
    assert!(!data.join("server.db").exists(), "failed startup must not initialize or migrate player storage");
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn missing_game_data_refuses_hosting_and_preserves_existing_data() { refuses_game_data("missing", false).await; }

#[tokio::test]
async fn corrupt_game_data_refuses_hosting_and_preserves_existing_data() { refuses_game_data("corrupt", true).await; }

#[tokio::test]
async fn explicit_protocol_fixtures_can_still_run_without_a_game_installation() {
    let root = std::env::temp_dir().join(format!("yo3-protocol-fixture-{}", std::process::id()));
    let mut cfg = yo3_server::Config::ephemeral(root.clone()); cfg.game_dir = root.join("absent-game");
    let certificate = cfg.tls.certificate.clone();
    let server = yo3_server::start(cfg).await.unwrap();
    let endpoint = yo3_tunnel::Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    endpoint.info().await.unwrap();
}
