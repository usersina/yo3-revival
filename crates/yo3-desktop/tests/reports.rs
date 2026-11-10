use std::io::Read;
use yo3_desktop::reports::Report;
use yo3_tunnel::diagnostics::Session;
#[cfg(unix)]
#[test]
fn private_profile_and_log_directories_protect_wine_created_files() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("yo3-private-directories-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
    for name in ["profiles", "logs/sessions"] { let p = root.join(name); std::fs::create_dir_all(&p).unwrap(); std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap(); }
    let _backend = yo3_desktop::Backend::new(root.clone()).unwrap();
    for name in ["profiles", "logs/sessions"] { assert_eq!(std::fs::metadata(root.join(name)).unwrap().permissions().mode() & 0o077, 0, "{name}"); }
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn current_and_retained_zip_reports_are_bounded_redacted_and_allowlisted() {
    let root = std::env::temp_dir().join(format!("yo3-report-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap(); let logs = root.join("sessions");
    let old = Session::start(&logs, "game", None).unwrap(); old.line("previous game");
    let app = Session::start(&logs, "app", None).unwrap(); let server = Session::start(&logs, "server", Some(app.id())).unwrap();
    server.line("new server ready"); server.line("key=abcdEFGH2345ijkmNPQR");
    std::fs::write(logs.join("server.key"), b"never export this").unwrap(); std::fs::write(logs.join("launcher.db"), b"private db").unwrap();
    #[cfg(unix)] std::os::unix::fs::symlink(logs.join("server.key"), logs.join("game-foreign.0000.log")).unwrap();
    let current = Report::prepare(&logs, Some(&[app.id(), server.id()])).unwrap(); assert!(current.preview.bytes > 0);
    let target = root.join("current.zip"); current.export(&target).unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(target).unwrap()).unwrap(); let mut combined = String::new();
    for i in 0..archive.len() { let mut f = archive.by_index(i).unwrap(); assert!(!f.name().unwrap().contains(&old.id())); f.read_to_string(&mut combined).unwrap(); }
    assert!(combined.contains("new server ready") && !combined.contains("abcdEFGH2345ijkmNPQR") && !combined.contains("never export") && !combined.contains("private db"));
    let all = Report::prepare(&logs, None).unwrap(); assert!(all.preview.files > current.preview.files);
    all.export(&root.join("all.zip")).unwrap();
    assert!(current.export(&root.join("launcher.db")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_rechecks_redaction_for_private_key_blocks_and_edited_session_metadata() {
    let root = std::env::temp_dir().join(format!("yo3-report-edited-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
    let log = Session::start(&root, "game", None).unwrap(); let id = log.id();
    let meta = root.join(format!("{id}.json")); let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&meta).unwrap()).unwrap();
    value["os"] = "key=abcdEFGH2345ijkmNPQR".into(); std::fs::write(meta, serde_json::to_vec(&value).unwrap()).unwrap();
    std::fs::write(root.join(format!("{id}.0000.log")), "-----BEGIN PRIVATE KEY-----\nSENSITIVEBASE64\n-----END PRIVATE KEY-----\nready\n").unwrap();
    let destination = root.join("report.zip"); Report::prepare(&root, None).unwrap().export(&destination).unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(destination).unwrap()).unwrap(); let mut text = String::new();
    for i in 0..zip.len() { zip.by_index(i).unwrap().read_to_string(&mut text).unwrap(); }
    assert!(!text.contains("abcdEFGH2345ijkmNPQR") && !text.contains("SENSITIVEBASE64")); assert!(text.contains("ready"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reopened_manager_keeps_latest_game_and_collects_its_failed_server_start() {
    let root = std::env::temp_dir().join(format!("yo3-report-attach-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
    let backend = yo3_desktop::Backend::new(root.clone()).unwrap();
    let server = Session::start(&root.join("logs/sessions"), "server", Some(backend.diagnostics.id())).unwrap(); server.line("bind failed");
    let game = Session::start(&root.join("logs/sessions"), "game", Some(backend.diagnostics.id())).unwrap(); game.line("last game failed");
    assert!(backend.current_sessions().contains(&server.id())); assert!(backend.logs(true).contains("bind failed"));
    drop(backend); let reopened = yo3_desktop::Backend::new(root.clone()).unwrap();
    assert!(reopened.current_sessions().contains(&game.id())); assert!(reopened.logs(false).contains("last game failed"));
    std::fs::remove_dir_all(root).unwrap();
}
