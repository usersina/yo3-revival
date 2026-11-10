use std::path::PathBuf;
#[cfg(unix)] use std::path::Path;
use yo3_desktop::updates::{Asset, Release, install, activation};
#[cfg(unix)]
fn fixture(script: &str) -> (PathBuf, install::Installed) {
    let root = std::env::temp_dir().join(format!("yo3-update-health-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&root).unwrap();
    let names = ["yo3-revival", "yo3-server", "yo3-launcher", "yo3-auth-launch.exe", "yo3-auth.dll", "yo3-bootstrap.dll", "yo3-launcher-windows.exe"];
    let roles = ["desktop", "server", "launcher", "injector", "connection_mod", "loader", "direct_launcher"];
    let mut files = vec![]; let mut components = serde_json::Map::new();
    for (name,role) in names.into_iter().zip(roles) {
        let bytes = if role == "desktop" { script.as_bytes().to_vec() } else { b"our fixture".to_vec() };
        components.insert(role.into(), serde_json::json!({"file":name,"sha256":yo3_launcher::components::hash(&bytes)})); files.push((name,bytes));
    }
    for name in ["engine/duelcore.ts", "engine/package.json", "engine/vendor/cardinfo.json", "engine/node/node", "engine/node_modules/ocgcore-wasm/package.json", "engine/vendor/BabelCDB/cards.cdb", "engine/vendor/CardScripts/utility.lua"] { files.push((name,b"fixture".to_vec())); }
    files.push(("bundle.json", serde_json::json!({"schema":1,"release":"v0.3.0","protocol":2,"mod_abi":2,"components":components}).to_string().into_bytes()));
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast()); let mut tar = tar::Builder::new(gzip);
    for (name,bytes) in files { let mut header=tar::Header::new_gnu(); header.set_size(bytes.len() as u64); header.set_mode(0o700); header.set_cksum(); tar.append_data(&mut header,format!("yo3-revival-v0.3.0-linux-x86_64/{name}"),bytes.as_slice()).unwrap(); }
    let bytes=tar.into_inner().unwrap().finish().unwrap(); let archive=root.join("release.tar.gz"); std::fs::write(&archive,&bytes).unwrap();
    let name="yo3-revival-v0.3.0-linux-x86_64.tar.gz";
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
#[test]
#[ignore = "requires a built release archive and an explicitly selected private Xvfb display"]
fn packaged_update_opens_its_window_before_activation() {
    let archive=PathBuf::from(std::env::var_os("YO3_UPDATE_TEST_ARCHIVE").expect("release archive"));
    let tag=std::env::var("YO3_UPDATE_TEST_TAG").expect("release tag");
    let root=PathBuf::from(std::env::var_os("YO3_UPDATE_TEST_ROOT").expect("scratch application data"));
    assert_eq!(std::env::var("DISPLAY").unwrap(),std::env::var("YO3_UPDATE_TEST_DISPLAY").expect("private display confirmation"));
    let bytes=std::fs::read(&archive).unwrap(); let name=archive.file_name().unwrap().to_str().unwrap().to_string();
    let release=Release{tag:tag.clone(),url:String::new(),notes:String::new(),asset:Asset{name:name.clone(),url:format!("https://github.com/usersina/yo3-revival/releases/download/{tag}/{name}"),bytes:bytes.len() as u64,sha256:yo3_launcher::components::hash(&bytes)}};
    let staged=install::stage_archive(&root,&release,&archive).unwrap();
    let mut child=activation::launch(&root,&staged,false,std::time::Duration::from_secs(25)).unwrap();
    assert_eq!(install::active(&root).unwrap(),Some(staged)); child.kill().unwrap(); child.wait().unwrap();
}
