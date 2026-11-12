use yo3_desktop::{bug_evidence::{self,PendingPost,Receipt},reports::Report,system_info::SystemInfo};
use yo3_tunnel::diagnostics::Session;

fn root(name:&str)->std::path::PathBuf {let root=std::env::temp_dir().join(format!("yo3-evidence-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(&root).unwrap();root}
fn system()->SystemInfo {SystemInfo{os:"Windows 11 Pro (build 26100)".into(),arch:"x86_64".into(),wine:None}}

#[test]
fn gist_files_reuse_diagnostic_allowlist_and_privacy_without_losing_manual_narrative() {
    let root=root("gist-files");let logs=root.join("sessions");let session=Session::start(&logs,"game",None).unwrap();
    session.line(r"Cannot load C:\Users\Alice\Game\yo3-auth.dll from 192.168.1.45:7443");session.line("[YO3 CLIENT] Alice chat");session.line("password=never-share");
    std::fs::write(logs.join("accounts.json"),b"private account database").unwrap();
    #[cfg(unix)] std::os::unix::fs::symlink(logs.join("accounts.json"),logs.join("app-foreign.0000.log")).unwrap();
    let report=Report::prepare(&logs,None).unwrap();let context=serde_json::json!({"stage":"mod-load","access_key":"short-secret","path":r"C:\Users\Alice\Game\yo3-auth.dll"});
    let markdown="# My player cannot summon Stardust Dragon\n\nManual steps remain.\npassword=short-secret\n";
    let full=report.gist_files(false,&context,markdown,&system()).unwrap();
    assert!(full.contains_key("report.json") && full.contains_key("bug-report.md"));assert!(full.keys().all(|name|!name.contains(['/', '\\'])));
    assert!(full.keys().any(|name|name.ends_with(".log")) && full.keys().any(|name|name.starts_with("session-") && name.ends_with(".json")));
    let text=full.values().cloned().collect::<String>();assert!(text.contains("Alice") && !text.contains("never-share") && !text.contains("short-secret") && !text.contains("private account database"));
    let filtered=report.gist_files(true,&context,markdown,&system()).unwrap();let filtered_text=filtered.values().cloned().collect::<String>();
    assert!(!filtered_text.contains("Alice") && !filtered_text.contains("192.168.1.45"));assert!(filtered["bug-report.md"].contains("My player cannot summon Stardust Dragon"));
    let metadata:serde_json::Value=serde_json::from_str(&filtered["report.json"]).unwrap();assert_eq!(metadata["system"]["os"],system().os);assert_eq!(metadata["context"]["stage"],"mod-load");
    let zip=root.join("report.zip");report.export_bug(&zip,true,&context,markdown,&system()).unwrap();
    let mut archive=zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();use std::io::Read;
    for (name,content) in &filtered {if name=="report.json" {continue}let zip_name=name.strip_prefix("session-").map(|name|format!("sessions/{name}")).unwrap_or_else(||name.clone());let mut zipped=String::new();archive.by_name(&zip_name).unwrap().read_to_string(&mut zipped).unwrap();assert_eq!(&zipped,content);}
    drop(archive);std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn gist_limits_reject_excess_evidence_instead_of_omitting_files_or_text() {
    let root=root("gist-limits");let logs=root.join("sessions");std::fs::create_dir_all(&logs).unwrap();
    for index in 0..99 {std::fs::write(logs.join(format!("app-{index}.0000.log")),b"native failure").unwrap();}
    let report=Report::prepare(&logs,None).unwrap();let error=report.gist_files(false,&serde_json::json!({}),"Manual description",&system()).unwrap_err();assert!(error.to_string().contains("100"),"{error:#}");
    for index in [97,98] {std::fs::remove_file(logs.join(format!("app-{index}.0000.log"))).unwrap();}
    let report=Report::prepare(&logs,None).unwrap();assert_eq!(report.gist_files(false,&serde_json::json!({}),"Manual description",&system()).unwrap().len(),99);
    std::fs::write(logs.join("app-97.0000.log"),b"native failure").unwrap();let report=Report::prepare(&logs,None).unwrap();assert!(report.gist_files(false,&serde_json::json!({}),"Manual description",&system()).is_err(),"reserve one of GitHub's 100 files for the transport's durable report marker");
    let empty=Report::prepare(&root.join("empty"),None).unwrap();let text="detail ".repeat(1_200_000);let error=empty.gist_files(false,&serde_json::json!({}),&text,&system()).unwrap_err();assert!(error.to_string().contains("8 MiB"),"{error:#}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sqlite_receipt_keeps_intent_and_uploaded_evidence_across_reopen_without_storing_authentication() {
    let root=root("receipts");assert!(bug_evidence::latest_pending(&root).unwrap().is_none());
    let mut receipt=Receipt::new("report-a");receipt.account_login=Some("test-duelist".into());receipt.pending=Some(PendingPost::Gist);receipt.payload=Some(serde_json::json!({"title":"Native failure","files":{"report.json":"sanitized diagnostic"}}));
    bug_evidence::save(&root,&receipt).unwrap();assert_eq!(bug_evidence::load(&root,"report-a").unwrap(),Some(receipt.clone()));assert_eq!(bug_evidence::latest_pending(&root).unwrap(),Some(receipt.clone()));
    receipt.gist_id=Some("abc123".into());receipt.gist_url=Some("https://gist.github.com/test-duelist/abc123".into());receipt.pending=None;bug_evidence::save(&root,&receipt).unwrap();
    assert_eq!(bug_evidence::latest_pending(&root).unwrap(),Some(receipt.clone()),"uploaded evidence with no issue remains resumable after an issue failure");
    let mut oversized=receipt.clone();oversized.payload=Some(serde_json::json!({"data":"x".repeat(10*1024*1024+1)}));assert!(bug_evidence::save(&root,&oversized).is_err());assert_eq!(bug_evidence::load(&root,"report-a").unwrap(),Some(receipt.clone()),"failed replacement cannot lose the previous intent/evidence receipt");
    receipt.issue_number=Some(42);receipt.issue_url=Some("https://github.com/usersina/yo3-revival/issues/42".into());bug_evidence::save(&root,&receipt).unwrap();assert!(bug_evidence::latest_pending(&root).unwrap().is_none());assert_eq!(bug_evidence::load(&root,"report-a").unwrap(),Some(receipt));
    #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;assert_eq!(std::fs::metadata(root.join("reports/bug-reports.db")).unwrap().permissions().mode()&0o077,0);}
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn receipt_retention_limits_finished_and_aged_unstarted_reports_but_keeps_pending_intents() {
    let root=root("receipt-retention");let mut pending=Receipt::new("pending-a");pending.pending=Some(PendingPost::Issue);pending.gist_id=Some("already-uploaded".into());bug_evidence::save(&root,&pending).unwrap();
    let mut old_draft=Receipt::new("old-unstarted");old_draft.payload=Some(serde_json::json!({"description":"Prepared but never posted"}));bug_evidence::save(&root,&old_draft).unwrap();
    for index in 0..22 {let mut complete=Receipt::new(format!("complete-{index}"));complete.issue_number=Some(index);bug_evidence::save(&root,&complete).unwrap();}
    assert!(bug_evidence::load(&root,"complete-0").unwrap().is_none());assert!(bug_evidence::load(&root,"complete-21").unwrap().is_some());assert_eq!(bug_evidence::latest_pending(&root).unwrap(),Some(pending.clone()));
    let mut uploaded=Receipt::new("old-uploaded");uploaded.gist_id=Some("remote-evidence".into());uploaded.gist_url=Some("https://gist.github.com/abc123".into());bug_evidence::save(&root,&uploaded).unwrap();
    let conn=rusqlite::Connection::open(root.join("reports/bug-reports.db")).unwrap();conn.execute("UPDATE receipts SET updated=0",[]).unwrap();drop(conn);
    let mut newer=Receipt::new("pending-b");newer.pending=Some(PendingPost::Gist);bug_evidence::save(&root,&newer).unwrap();assert_eq!(bug_evidence::latest_pending(&root).unwrap(),Some(newer));assert_eq!(bug_evidence::load(&root,"pending-a").unwrap(),Some(pending));assert!(bug_evidence::load(&root,"complete-21").unwrap().is_none(),"finished reports older than 14 days are removed");
    assert!(bug_evidence::load(&root,&old_draft.report_id).unwrap().is_none(),"unstarted payloads older than 14 days must not accumulate after canceled preparation");
    assert_eq!(bug_evidence::load(&root,&uploaded.report_id).unwrap(),Some(uploaded),"remote evidence without an issue remains resumable regardless of age");
    for index in 0..22 {let mut draft=Receipt::new(format!("recent-draft-{index}"));draft.payload=Some(serde_json::json!({"description":"A recent active preparation"}));bug_evidence::save(&root,&draft).unwrap();}
    assert!(bug_evidence::load(&root,"recent-draft-0").unwrap().is_some(),"the completed-report count cap must not discard a recent prepared draft");
    std::fs::remove_dir_all(root).unwrap();
}
