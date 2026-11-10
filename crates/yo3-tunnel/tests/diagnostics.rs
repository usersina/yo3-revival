use std::io::Write;
use yo3_tunnel::diagnostics::{Session, records, redact};
#[test]
fn report_redaction_masks_native_and_wine_home_paths() {
    let text = r"failed /home/alice/game, Z:\home\sina\Documents\game and C:\Users\Alice Smith\game";
    let redacted = redact(text, &[], Some("/home/sina"));
    for name in ["alice", "sina", "Alice Smith"] { assert!(!redacted.contains(name), "{redacted}"); }
    assert!(redacted.contains("game"));
}
#[test]
fn log_capture_redacts_split_secrets_and_drops_overlong_lines() {
    let root = std::env::temp_dir().join(format!("yo3-capture-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
    let log = Session::start(&root, "game", None).unwrap();
    let key = "abcdEFGH2345ijkmNPQR"; log.secret(key);
    let mut writer = log.writer(); writer.write_all(b"key=abcdEFGH").unwrap(); writer.write_all(b"2345ijkmNPQR\nready\n").unwrap();
    writer.write_all(format!("cookie={}\n", "a7".repeat(32)).as_bytes()).unwrap();
    writer.write_all(format!("{}{}\n", "x".repeat(20000), key).as_bytes()).unwrap();
    writer.flush().unwrap();
    let text = records(&root).unwrap().into_iter().filter(|r| r.name.ends_with(".log")).map(|r| std::fs::read_to_string(root.join(r.name)).unwrap()).collect::<String>();
    assert!(!text.contains(key) && !text.contains(&"a7".repeat(32))); assert!(text.contains("ready"));
    assert!(text.len() < 2000 && text.contains("overlong"));
    assert!(!redact("password: huntertwo", &[], None).contains("huntertwo"));
    let _ = std::fs::remove_dir_all(root);
}
