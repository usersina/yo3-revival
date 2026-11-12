use std::io::Read;
use yo3_desktop::reports::Report;
use yo3_tunnel::diagnostics::Session;
fn archive_text(path: &std::path::Path) -> String {
    let mut zip=zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap(); let mut text=String::new();
    for i in 0..zip.len() {zip.by_index(i).unwrap().read_to_string(&mut text).unwrap();}
    text
}
#[test]
fn export_can_filter_personal_data_without_destroying_the_original_diagnostics() {
    let root=std::env::temp_dir().join(format!("yo3-report-privacy-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
    let log=Session::start(&root,"game",None).unwrap();
    log.line(r"Runtime check failed at C:\Users\Alice Smith\game from 192.168.1.45:7443 and [2001:db8::45]:7443");
    log.line("[YO3 CLIENT] Alice says private chat 日本語");
    log.line("[YO3 PACKET] player=Alice deck=3437");
    log.line("[YO3 ENGINE] shuffled private deck recording");
    log.line("[tls] 192.168.1.45 user 9 launcher 3 Front");
    log.line("Account PrivateDuelist created");
    log.line("Connection failed for https://private-host.example/status; contact alice@example.org");
    log.line("INFO [front] -> 0x2398 {name=LegacyDuelist}");
    log.line("key=abcdEFGH2345ijkmNPQR");
    let report=Report::prepare(&root,None).unwrap();
    let full=root.join("full.zip"); report.export(&full).unwrap(); let full_text=archive_text(&full);
    assert!(full_text.contains("Alice Smith") && full_text.contains("private chat 日本語") && full_text.contains("3437") && full_text.contains("192.168.1.45"));
    let filtered=root.join("filtered.zip"); report.export_with_privacy(&filtered,true).unwrap(); let filtered_text=archive_text(&filtered);
    assert!(filtered_text.contains("Runtime check failed"),"technical failures must survive filtering");
    for personal in ["Alice","private chat","3437","192.168.1.45","2001:db8::45","shuffled private deck","PrivateDuelist","private-host.example","alice@example.org","LegacyDuelist"] {assert!(!filtered_text.contains(personal),"{personal}: {filtered_text}");}
    for text in [&full_text,&filtered_text] {assert!(!text.contains("abcdEFGH2345ijkmNPQR"));}
    assert!(full_text.contains("\"personal_data_filtered\": false") && filtered_text.contains("\"personal_data_filtered\": true"));
    let again=root.join("full-again.zip"); report.export(&again).unwrap(); assert!(archive_text(&again).contains("private chat 日本語"));
    std::fs::remove_dir_all(root).unwrap();
}
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

#[test]
fn bug_report_embeds_reviewed_markdown_context_and_system_with_the_same_privacy_choice() {
    let root=std::env::temp_dir().join(format!("yo3-bug-report-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(&root).unwrap();
    let log=Session::start(&root.join("sessions"),"app",None).unwrap();log.line("Original native failure: process_exit_code=0xc0000005");log.line("[YO3 CLIENT] Chat: Alice hello");
    let context=serde_json::json!({"failure":{"message":r"Cannot load C:\Users\Alice\Game\yo3-auth.dll from 192.168.1.45:7443","stage":"mod-load","process_exit_code":"0xc0000005"},
        "account":{"name":"Alice","access_key":"short-secret"},"nested":[{"token":"short-token"}],"missing_files":[r"C:\Users\Alice\Game\yo3-auth.dll"]});
    let system=yo3_desktop::system_info::SystemInfo{os:"Windows 11 Pro 24H2 (build 26100.1234)".into(),arch:"x86_64".into(),wine:None};
    let markdown=format!("# DLL cannot load\n\nMy player cannot summon Stardust Dragon.\nCannot load C:\\Users\\Alice\\Game\\yo3-auth.dll from 192.168.1.45:7443\npassword=short-secret\n-----BEGIN PRIVATE KEY-----\nSHORT PEM CONTENT\n-----END PRIVATE KEY-----\n{}Final reproduction detail remains\n","User step. ".repeat(2000));
    let report=Report::prepare(&root.join("sessions"),None).unwrap();let full=root.join("full.zip");report.export_bug(&full,false,&context,&markdown,&system).unwrap();
    let full_text=archive_text(&full);assert!(full_text.contains("Alice") && full_text.contains("192.168.1.45"));assert!(!full_text.contains("short-secret") && !full_text.contains("short-token"));
    let mut archive=zip::ZipArchive::new(std::fs::File::open(&full).unwrap()).unwrap();
    let mut metadata=String::new();archive.by_name("report.json").unwrap().read_to_string(&mut metadata).unwrap();let metadata:serde_json::Value=serde_json::from_str(&metadata).unwrap();
    assert_eq!(metadata["system"]["os"],system.os);assert_eq!(metadata["system"]["arch"],"x86_64");assert_eq!(metadata["context"]["failure"]["stage"],"mod-load");
    assert_eq!(metadata["release"],yo3_tunnel::BUILD_VERSION);assert_eq!(metadata["personal_data_filtered"],false);assert!(archive.by_name("bug-report.md").is_ok());
    let filtered=root.join("filtered.zip");report.export_bug(&filtered,true,&context,&markdown,&system).unwrap();let filtered_text=archive_text(&filtered);
    assert!(!filtered_text.contains("Alice") && !filtered_text.contains("192.168.1.45") && !filtered_text.contains("short-secret") && !filtered_text.contains("short-token"),"{filtered_text}");
    assert!(filtered_text.contains("yo3-auth.dll") && filtered_text.contains("0xc0000005") && filtered_text.contains("Windows 11 Pro"));
    let public_markdown=yo3_desktop::reports::filter_text(&markdown,true);assert!(!public_markdown.contains("Alice") && !public_markdown.contains("short-secret"));
    for text in [&full_text,&filtered_text,&public_markdown] {assert!(text.contains("My player cannot summon Stardust Dragon.") && text.contains("Final reproduction detail remains"),"manual narrative stays intact, including long fallback descriptions");assert!(!text.contains("SHORT PEM CONTENT"),"private key blocks are never optional");}
    let regular=root.join("regular.zip");report.export(&regular).unwrap();let mut regular=zip::ZipArchive::new(std::fs::File::open(regular).unwrap()).unwrap();assert!(regular.by_name("bug-report.md").is_err(),"ordinary exports retain their existing format");
    std::fs::remove_dir_all(root).unwrap();
}
