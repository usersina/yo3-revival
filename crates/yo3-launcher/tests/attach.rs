use std::time::Duration;
use yo3_launcher::{Ports, attach::{ReadyFiles, wait_for_ack}};
#[tokio::test]
async fn attachment_requires_the_current_cookie_and_a_live_owner_and_preserves_existing_files() {
    let root = std::env::temp_dir().join(format!("yo3-attach-contract-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready.json"); let ack = root.join("ack"); let cookie = [7; 32];
    std::fs::write(&ready, "keep this file").unwrap();
    assert!(ReadyFiles::prepare(&ready, &ack, 123, Ports::default(), &cookie).is_err());
    yo3_launcher::attach::report_error(&ready, "this must not replace an existing file");
    assert_eq!(std::fs::read_to_string(&ready).unwrap(), "keep this file"); std::fs::remove_file(&ready).unwrap();
    let files = ReadyFiles::prepare(&ready, &ack, 123, Ports::default(), &cookie).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&ready).unwrap()).unwrap();
    assert_eq!(value["state"], "prepared"); assert_eq!(value["game_pid"], 123);
    std::fs::write(&ack, "00".repeat(32)).unwrap();
    assert!(wait_for_ack(&ack, &cookie, Duration::from_millis(100), || Ok(true)).await.is_err());
    assert!(ack.exists(), "a bad/stale acknowledgement is preserved, not treated as owned");
    std::fs::write(&ack, yo3_launcher::hex(&cookie)).unwrap();
    assert!(wait_for_ack(&ack, &cookie, Duration::from_millis(100), || Ok(false)).await.is_err(), "an exited game cannot activate a session");
    wait_for_ack(&ack, &cookie, Duration::from_millis(100), || Ok(true)).await.unwrap();
    files.authenticated().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&ready).unwrap()).unwrap(); assert_eq!(value["state"], "authenticated");
    drop(files); assert!(!ready.exists() && !ack.exists());
    assert!(wait_for_ack(&ack, &cookie, Duration::from_millis(30), || Ok(true)).await.is_err());
    std::fs::remove_dir_all(root).unwrap();
}
