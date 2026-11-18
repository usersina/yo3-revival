use std::path::PathBuf;
#[cfg(unix)] use std::path::Path;
use yo3_desktop::updates::{Asset, Release, install, activation};
#[cfg(unix)]
fn fixture(script: &str) -> (PathBuf, install::Installed) {
    let root = std::env::temp_dir().join(format!("yo3-update-health-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&root).unwrap();
    let names = ["yo3-revival", "yo3-server", "yo3-launcher", "yo3-auth-launch.exe", "yo3-auth.dll", "yo3-bootstrap.dll", "yo3-launcher-windows.exe", "yo3-engine"];
    let roles = ["desktop", "server", "launcher", "injector", "connection_mod", "loader", "direct_launcher", "engine"];
    let mut files = vec![]; let mut components = serde_json::Map::new();
    for (name,role) in names.into_iter().zip(roles) {
        let bytes = if role == "desktop" { script.as_bytes().to_vec() } else { b"our fixture".to_vec() };
        components.insert(role.into(), serde_json::json!({"file":name,"sha256":yo3_launcher::components::hash(&bytes)})); files.push((name,bytes));
    }
    for name in ["engine/vendor/cardinfo.json", "engine/vendor/BabelCDB/cards.cdb", "engine/vendor/CardScripts/utility.lua"] { files.push((name,b"fixture".to_vec())); }
    files.push(("bundle.json", serde_json::json!({"schema":1,"edition":"desktop","release":"v0.3.0","protocol":2,"mod_abi":2,"components":components}).to_string().into_bytes()));
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast()); let mut tar = tar::Builder::new(gzip);
    for (name,bytes) in files { let mut header=tar::Header::new_gnu(); header.set_size(bytes.len() as u64); header.set_mode(0o700); header.set_cksum(); tar.append_data(&mut header,format!("yo3-revival-desktop/{name}"),bytes.as_slice()).unwrap(); }
    let bytes=tar.into_inner().unwrap().finish().unwrap(); let archive=root.join("release.tar.gz"); std::fs::write(&archive,&bytes).unwrap();
    let name="yo3-revival-desktop-v0.3.0-linux-x86_64.tar.gz";
    let release=Release{tag:"v0.3.0".into(),url:String::new(),notes:String::new(),asset:Asset{name:name.into(),url:format!("https://github.com/usersina/yo3-revival/releases/download/v0.3.0/{name}"),bytes:bytes.len() as u64,sha256:yo3_launcher::components::hash(&bytes)}};
    let installed=install::stage_archive(&root,&release,&archive).unwrap(); (root,installed)
}
#[test]
#[cfg(unix)]
fn failed_startup_never_switches_the_active_version() {
    let (root,installed)=fixture("#!/bin/sh\nexit 7\n");
    assert!(activation::launch(&root,&installed,false,std::time::Duration::from_millis(500)).is_err());
    assert!(install::active(&root).unwrap().is_none());
    assert_eq!(std::fs::read_dir(root.join("updates/jobs")).unwrap().count(),0);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn only_a_matching_ready_acknowledgement_activates_and_original_shortcuts_route_forward() {
    let script=r##"#!/bin/sh
while [ "$#" -gt 0 ]; do
 case "$1" in --update-ready-file) ready="$2"; shift 2;; --update-nonce) nonce="$2"; shift 2;; *) shift;; esac
done
printf '{"nonce":"%s","version":"0.3.0","pid":%s}' "$nonce" "$$" > "$ready"
exec sleep 30
"##;
    let (root,installed)=fixture(script);
    let mut child=activation::launch(&root,&installed,false,std::time::Duration::from_secs(2)).unwrap();
    assert_eq!(install::active(&root).unwrap(),Some(installed.clone()));
    assert_eq!(activation::target(&root,"0.2.0",Path::new("/original/yo3-revival")).unwrap(),Some(installed.clone()));
    assert!(activation::target(&root,"0.3.0",&installed.executable()).unwrap().is_none());
    assert!(activation::target(&root,"0.4.0",Path::new("/original/yo3-revival")).unwrap().is_none());
    assert!(activation::target(&root,"0.1.0-dev",Path::new("/original/yo3-revival")).unwrap().is_none());
    child.kill().unwrap(); child.wait().unwrap();
    std::fs::write(installed.executable(),b"changed").unwrap();
    assert!(activation::target(&root,"0.2.0",Path::new("/original/yo3-revival")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn readiness_requests_cannot_write_outside_private_update_jobs_or_clobber_files() {
    let (root,_)=fixture("#!/bin/sh\nexit 0\n");
    let nonce="11".repeat(32); let outside=root.join("server.key");
    assert!(activation::Readiness::new(&root,outside.clone(),nonce.clone()).is_err()); assert!(!outside.exists());
    let job=root.join("updates/jobs/fixture"); std::fs::create_dir(&job).unwrap();
    let ack=activation::Readiness::new(&root,job.join("ready.json"),nonce).unwrap();
    ack.write("0.3.0").unwrap(); assert!(ack.write("0.3.0").is_err());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn a_live_process_with_the_wrong_acknowledgement_or_no_window_never_activates() {
    for reply in ["printf '{\"nonce\":\"wrong\",\"version\":\"0.3.0\",\"pid\":%s}' \"$$\" > \"$ready\"", ":"] {
        let script=format!("#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do case \"$1\" in --update-ready-file) ready=\"$2\"; shift 2;; *) shift;; esac; done\n{reply}\nexec sleep 30\n");
        let (root,installed)=fixture(&script);
        assert!(activation::launch(&root,&installed,false,std::time::Duration::from_millis(300)).is_err());
        assert!(install::active(&root).unwrap().is_none()); std::fs::remove_dir_all(root).unwrap();
    }
}
/// A ready script: acknowledges its window as the update's version.
#[cfg(unix)]
const READY: &str = r##"#!/bin/sh
while [ "$#" -gt 0 ]; do
 case "$1" in --update-ready-file) ready="$2"; shift 2;; --update-nonce) nonce="$2"; shift 2;; *) shift;; esac
done
printf '{"nonce":"%s","version":"0.3.0","pid":%s}' "$nonce" "$$" > "$ready"
exec sleep 30
"##;
#[test]
#[cfg(unix)]
fn a_downloaded_update_installs_at_the_next_start_but_never_while_the_game_runs() {
    let (root,installed)=fixture(READY);
    let original=Path::new("/original/yo3-revival"); let quick=std::time::Duration::from_secs(5);
    assert_eq!(activation::install_pending(&root,"0.2.0",original,||false,quick),activation::Pending::Nothing,"nothing downloaded yet");
    install::set_pending(&root,&installed).unwrap();
    assert!(install::active(&root).unwrap().is_none(),"downloading never switches the running version");
    assert_eq!(activation::install_pending(&root,"0.2.0",original,||true,quick),activation::Pending::Nothing,"the game is open: it waits");
    assert_eq!(install::pending(&root).unwrap(),Some(installed.clone()));
    assert_eq!(activation::install_pending(&root,"0.2.0",original,||false,quick),activation::Pending::Installed);
    assert_eq!(install::active(&root).unwrap(),Some(installed.clone()),"promoted once its window was up");
    assert!(install::pending(&root).unwrap().is_none() && install::failed_start(&root).unwrap().is_none());
    let _ = std::process::Command::new("pkill").arg("-f").arg(installed.executable()).status();
    install::set_pending(&root,&installed).unwrap();
    assert_eq!(activation::install_pending(&root,"0.3.0",original,||false,quick),activation::Pending::Nothing,"already this version");
    assert!(install::pending(&root).unwrap().is_none(),"and forgotten");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn an_update_that_fails_to_start_rolls_back_with_a_notice_and_isnt_retried() {
    let (root,installed)=fixture("#!/bin/sh\nexit 7\n");
    install::set_pending(&root,&installed).unwrap();
    let result=activation::install_pending(&root,"0.2.0",Path::new("/original/yo3-revival"),||false,std::time::Duration::from_millis(500));
    let activation::Pending::Failed{summary,details}=result else { panic!("{result:?}") };
    assert_eq!(summary,"Update 0.3.0 didn't start, so YO3 Revival kept version 0.2.0.");
    assert!(details.contains("exited before it was ready"),"{details}");
    assert!(install::active(&root).unwrap().is_none(),"the current version stays active");
    assert!(install::pending(&root).unwrap().is_none());
    assert_eq!(install::failed_start(&root).unwrap().as_deref(),Some("v0.3.0"),"not downloaded again by itself");
    std::fs::remove_dir_all(root).unwrap();
}
/// The release under test, as `tools/release/test-packaged-update.sh` hands it over.
fn packaged_release() -> (PathBuf, PathBuf, Release) {
    let archive=PathBuf::from(std::env::var_os("YO3_UPDATE_TEST_ARCHIVE").expect("release archive"));
    let tag=std::env::var("YO3_UPDATE_TEST_TAG").expect("release tag");
    let root=PathBuf::from(std::env::var_os("YO3_UPDATE_TEST_ROOT").expect("scratch application data"));
    assert_eq!(std::env::var("DISPLAY").unwrap(),std::env::var("YO3_UPDATE_TEST_DISPLAY").expect("private display confirmation"));
    let bytes=std::fs::read(&archive).unwrap(); let name=archive.file_name().unwrap().to_str().unwrap().to_string();
    let release=Release{tag:tag.clone(),url:String::new(),notes:String::new(),asset:Asset{name:name.clone(),url:format!("https://github.com/usersina/yo3-revival/releases/download/{tag}/{name}"),bytes:bytes.len() as u64,sha256:yo3_launcher::components::hash(&bytes)}};
    (archive,root,release)
}
#[cfg(unix)]
fn stop(executable: &Path) { let _ = std::process::Command::new("pkill").arg("-f").arg(executable).status(); std::thread::sleep(std::time::Duration::from_millis(500)); }
#[test]
#[ignore = "requires a built release archive and an explicitly selected private Xvfb display"]
fn packaged_update_is_signed_and_opens_its_window_before_activation() {
    let (archive,root,release)=packaged_release();
    // The release's SHA256SUMS, signed by the Release workflow's own script with a key made for this test only.
    use yo3_desktop::updates::signature;
    let key=signature::parse_key(&std::env::var("YO3_UPDATE_TEST_PUBLIC_KEY").expect("the test's public key")).expect("a raw Ed25519 key in hex");
    let signed=PathBuf::from(std::env::var_os("YO3_UPDATE_TEST_SIGNED").expect("the signed release folder"));
    let manifest=std::fs::read(signed.join(signature::MANIFEST)).unwrap(); let sig=std::fs::read(signed.join(signature::SIGNATURE)).unwrap();
    signature::vouch(&release,Some(&manifest),Some(&sig),&key).expect("the workflow's signature vouches for the archive");
    let mut flipped=sig.clone(); flipped[10]^=1;
    assert!(signature::vouch(&release,Some(&manifest),Some(&flipped),&key).unwrap_err().to_string().contains("signature does not match"),"a bad signature");
    assert!(signature::vouch(&release,Some(&manifest),Some(&sig),&signature::PUBLIC_KEY.unwrap()).is_err(),"another key (the project's)");
    let error=signature::vouch(&release,Some(&manifest),None,&key).unwrap_err().to_string();
    assert!(error.contains("isn't signed"),"a release without SHA256SUMS.sig: {error}");
    let mut other=release.clone(); other.asset.sha256="00".repeat(32);
    assert!(signature::vouch(&other,Some(&manifest),Some(&sig),&key).is_err(),"a download the manifest doesn't list");
    // As the window does (#32): staged in the background, then installed by the launcher's next start.
    let staged=install::stage_archive(&root,&release,&archive).unwrap();
    install::set_pending(&root,&staged).unwrap();
    assert!(install::active(&root).unwrap().is_none(),"downloading never switches the running version");
    let result=activation::install_pending(&root,"0.0.1",std::path::Path::new("/original/yo3-revival"),||false,std::time::Duration::from_secs(25));
    assert_eq!(result,activation::Pending::Installed);
    assert_eq!(install::active(&root).unwrap(),Some(staged.clone())); assert!(install::pending(&root).unwrap().is_none());
    stop(&staged.executable());
}
/// A released launcher (`YO3_UPDATE_TEST_FROM`, its Desktop archive, e.g. v0.0.21) meets the release under test: its
/// launcher.db is opened by the new version, and its original shortcut still opens the installed update (it launches
/// and promotes it with its own code, which opens launcher.db and checks the new bundle by its rules).
#[test]
#[cfg(unix)]
#[ignore = "requires a built release archive, a released one and an explicitly selected private Xvfb display"]
fn a_released_launcher_updates_to_this_release_and_its_shortcut_reaches_it() {
    let (archive,root,release)=packaged_release();
    let root=root.with_file_name("app-released"); // apart from the other packaged test's data
    let from=PathBuf::from(std::env::var_os("YO3_UPDATE_TEST_FROM").expect("the released archive"));
    let old_dir=root.parent().unwrap().join("released");
    std::fs::create_dir_all(&old_dir).unwrap();
    tar::Archive::new(flate2::read::GzDecoder::new(std::fs::File::open(&from).unwrap())).unpack(&old_dir).unwrap();
    let old=old_dir.join("yo3-revival-desktop/yo3-revival");
    let run_old=|| std::process::Command::new(&old).env("YO3_DESKTOP_DATA",&root).env_remove("YO3_BUNDLE_DIR").env_remove("YO3_DESKTOP_BIN")
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
    // The player used the released version: its launcher.db and settings exist.
    let mut first=run_old();
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(20);
    while !root.join("launcher.db").exists() && std::time::Instant::now()<deadline { std::thread::sleep(std::time::Duration::from_millis(200)); }
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert!(first.try_wait().unwrap().is_none(),"the released launcher opened");
    first.kill().unwrap(); first.wait().unwrap();
    assert!(root.join("launcher.db").exists(),"the released launcher made its database");
    // Its updater installed this release (the new version opened its window and was promoted).
    let staged=install::stage_archive(&root,&release,&archive).unwrap();
    let mut opened=activation::launch(&root,&staged,false,std::time::Duration::from_secs(25)).expect("the new version opens on the released launcher's data");
    assert_eq!(install::active(&root).unwrap(),Some(staged.clone()));
    opened.kill().unwrap(); opened.wait().unwrap();
    // The original shortcut: the released binary must hand over to the installed update (and exit), not open itself
    // with "Launcher database is newer than this application".
    let mut shortcut=run_old();
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(40);
    let status=loop {
        if let Some(status)=shortcut.try_wait().unwrap() { break Some(status) }
        if std::time::Instant::now()>deadline { break None }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    let Some(status)=status else { let _=shortcut.kill(); stop(&staged.executable()); panic!("the released launcher opened itself instead of the installed update") };
    assert!(status.success(),"the released launcher's hand-over failed: {status}");
    let running=std::process::Command::new("pgrep").arg("-f").arg(staged.executable()).output().unwrap();
    assert!(running.status.success(),"the installed update is running");
    assert_eq!(install::active(&root).unwrap(),Some(staged.clone()),"still the active version");
    stop(&staged.executable());
}
