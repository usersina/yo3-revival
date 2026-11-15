//! The launcher's own server (Singleplayer, #29): it keeps running while its owner lives, and once the owner has gone
//! (the launcher closed or crashed) it stops by itself after nobody has played for the idle time.
use std::time::Duration;
use yo3_server::{Config, start};

fn scratch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("yo3-lifetime-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn an_owned_server_stops_once_idle_after_its_owner_has_gone() {
    let mut running = start(Config::ephemeral(scratch())).await.unwrap();
    let (gone, owner_gone) = tokio::sync::oneshot::channel::<()>();
    running.exit_when_idle_without_owner(async move { let _ = owner_gone.await; }, Duration::from_millis(300));
    let waiting = tokio::spawn(running.wait());
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert!(!waiting.is_finished(), "an idle server keeps running while its owner lives");
    drop(gone);
    tokio::time::timeout(Duration::from_secs(5), waiting).await.expect("stopped within the idle time").unwrap();
}
