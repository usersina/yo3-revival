use std::path::{Path, PathBuf};
use std::io::Write;
use yo3_desktop::updates::{Asset, Release, install};
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!("yo3-update-install-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&p).unwrap(); p
}
fn fixture(root: &Path, extra: Option<(&str, &[u8])>, corrupt: bool) -> (Release, PathBuf) {
    let tag = "v0.3.0"; let prefix = "yo3-revival-desktop";
    let mut files: Vec<(&str, Vec<u8>)> = vec![("yo3-revival.exe", b"our GUI fixture".to_vec()), ("yo3-server.exe", b"server".to_vec()),
        ("yo3-launcher.exe", b"launcher".to_vec()), ("yo3-auth-launch.exe", b"injector".to_vec()), ("yo3-auth.dll", b"mod".to_vec()),
        ("yo3-bootstrap.dll", b"loader".to_vec()), ("engine/duelcore.ts", b"sidecar".to_vec()), ("engine/node/node.exe", b"node".to_vec()),
        ("engine/vendor/cardinfo.json", b"{}".to_vec()), ("engine/package.json", b"{}".to_vec()), ("engine/node_modules/ocgcore-wasm/package.json", b"{}".to_vec()),
        ("engine/vendor/BabelCDB/cards.cdb", b"fixture".to_vec()), ("engine/vendor/CardScripts/utility.lua", b"-- fixture".to_vec())];
    let roles=["desktop","server","launcher","injector","connection_mod","loader"];
    let mut components=serde_json::Map::new();
    for (role,(name,bytes)) in roles.into_iter().zip(&files) { components.insert(role.into(),serde_json::json!({"file":name,"sha256":yo3_launcher::components::hash(bytes)})); }
    let manifest=serde_json::json!({"schema":1,"edition":"desktop","release":tag,"protocol":2,"mod_abi":2,"components":components}).to_string().into_bytes();
    files.push(("bundle.json",manifest));
    if corrupt { files[0].1=b"changed GUI".to_vec(); }
    let mut zip=zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options=zip::write::SimpleFileOptions::default().unix_permissions(0o644);
    for (name,bytes) in files { zip.start_file(format!("{prefix}/{name}"),options).unwrap(); zip.write_all(&bytes).unwrap(); }
    if let Some((path,bytes))=extra { zip.start_file(path,options).unwrap(); zip.write_all(bytes).unwrap(); }
    let bytes=zip.finish().unwrap().into_inner(); let archive=root.join("update.zip"); std::fs::write(&archive,&bytes).unwrap();
    let name=format!("yo3-revival-desktop-{tag}-windows-x86_64.zip");
    (Release {tag:tag.into(),url:format!("https://github.com/usersina/yo3-revival/releases/tag/{tag}"),notes:String::new(),
        asset:Asset{name:name.clone(),url:format!("https://github.com/usersina/yo3-revival/releases/download/{tag}/{name}"),bytes:bytes.len() as u64,sha256:yo3_launcher::components::hash(&bytes)}},archive)
}
#[test]
fn complete_updates_are_staged_without_replacing_user_data_and_retries_are_idempotent() {
    let p=root(); std::fs::create_dir(p.join("server")).unwrap(); std::fs::write(p.join("server/server.db"),b"user data").unwrap();
    let (release,archive)=fixture(&p,None,false);
    let staged=install::stage_archive(&p,&release,&archive).unwrap();
    assert!(staged.directory.join("yo3-revival.exe").is_file());
    assert_eq!(staged,install::stage_archive(&p,&release,&archive).unwrap());
    assert_eq!(std::fs::read(p.join("server/server.db")).unwrap(),b"user data");
    assert!(install::active(&p).unwrap().is_none(), "staging must not switch the running app");
    std::fs::write(staged.directory.join("yo3-revival.exe"),b"tampered").unwrap();
    assert!(install::validate(&p,&staged).is_err());
    std::fs::remove_dir_all(p).unwrap();
}
#[test]
fn changed_archives_and_components_or_unsafe_paths_never_become_an_installed_update() {
    for extra in [Some(("../../escaped",b"bad".as_slice())),Some(("yo3-revival-desktop/server.key",b"private".as_slice())),None] {
        let p=root(); let (mut release,archive)=fixture(&p,extra,extra.is_none());
        assert!(install::stage_archive(&p,&release,&archive).is_err()); assert!(install::active(&p).unwrap().is_none());
        release.asset.sha256="00".repeat(32); assert!(install::stage_archive(&p,&release,&archive).is_err());
        assert!(!p.join("escaped").exists()); std::fs::remove_dir_all(p).unwrap();
    }
}
#[test]
fn windows_path_aliases_are_refused_even_when_unpacking_on_linux() {
    for path in ["yo3-revival-desktop/CON", "yo3-revival-desktop/file:stream", "yo3-revival-desktop/data "] {
        let p=root(); let (release,archive)=fixture(&p,Some((path,b"bad")),false);
        assert!(install::stage_archive(&p,&release,&archive).is_err(), "unsafe Windows path: {path}");
        std::fs::remove_dir_all(p).unwrap();
    }
}
#[test]
fn archive_links_cannot_escape_staging() {
    let p=root(); let (mut release,archive)=fixture(&p,None,false);
    let bytes=std::fs::read(&archive).unwrap(); let mut source=zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut zip=zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..source.len() { zip.raw_copy_file(source.by_index(i).unwrap()).unwrap(); }
    zip.add_symlink("yo3-revival-desktop/link","../../escape",zip::write::SimpleFileOptions::default()).unwrap();
    let bytes=zip.finish().unwrap().into_inner(); release.asset.bytes=bytes.len() as u64; release.asset.sha256=yo3_launcher::components::hash(&bytes); std::fs::write(&archive,bytes).unwrap();
    assert!(install::stage_archive(&p,&release,&archive).is_err()); assert!(install::active(&p).unwrap().is_none());
    assert_eq!(std::fs::read_dir(p.join("updates/jobs")).unwrap().count(),0);
    std::fs::remove_dir_all(p).unwrap();
}
