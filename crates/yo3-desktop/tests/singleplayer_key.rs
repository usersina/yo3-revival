//! Singleplayer's account key in the system's credential store (#23), not in the launcher's profile files: the helper's
//! profiles name the store's entry. Only a PC without a store keeps it in the owner-only file, and the log says so.
//! These tests use stores in memory; none reaches the desktop's own.
use std::sync::Arc;
use yo3_desktop::{Backend, KeyConnection};
use yo3_desktop::multiplayer::{MemoryVault, Vault};

const KEY: &str = "abcdEFGH2345ijkmNPQR";

fn root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("yo3-sp-key-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); root
}

/// Your own server's connection, as Play makes it.
fn connection(root: &std::path::Path) -> (KeyConnection, String) {
    let cfg = yo3_server::Config::ephemeral(root.join("server"));
    let certificate = std::fs::read_to_string(&cfg.tls.certificate).unwrap();
    let identity = yo3_tunnel::identity_from_pem(certificate.as_bytes()).unwrap();
    (KeyConnection { address: "127.0.0.1:17443".into(), server_name: "localhost".into(), certificate, identity: None, key: KEY.into() }, identity)
}

fn log_text(root: &std::path::Path) -> String {
    let mut text = String::new();
    for entry in walk(&root.join("logs")) { text += &std::fs::read_to_string(entry).unwrap_or_default() }
    text
}
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut all = vec![];
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() { all.extend(walk(&path)) } else { all.push(path) }
    }
    all
}

/// A store that can't be reached, as on a Linux PC without a Secret Service.
struct NoStore;
impl Vault for NoStore {
    fn get(&self, _: &str) -> anyhow::Result<Option<String>> { anyhow::bail!("the system's credential store can't be read") }
    fn set(&self, _: &str, _: &str) -> anyhow::Result<()> { anyhow::bail!("the system's credential store can't be written") }
    fn delete(&self, _: &str) -> anyhow::Result<()> { anyhow::bail!("the system's credential store can't be written") }
}

#[test]
fn plays_profile_names_the_stores_entry_and_holds_no_key() {
    let root = root("store");
    let vault = Arc::new(MemoryVault::default());
    let backend = Backend::with_vault(root.clone(), vault.clone()).unwrap();
    let (server, identity) = connection(&root);
    let file = backend.write_profile(&server).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains(KEY), "{text}");
    let entry = yo3_launcher::vault::key_account(&identity);
    assert_eq!(vault.get(&entry).unwrap().as_deref(), Some(KEY));
    let loaded = yo3_launcher::Profile::read_with(&file, vault.as_ref()).unwrap();
    assert!(matches!(loaded.credential, yo3_launcher::Credential::Key(k) if k == KEY));
    assert!(!log_text(&root).contains("owner-only file"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn without_a_credential_store_the_key_stays_in_the_owner_only_file_and_the_log_says_so() {
    let root = root("nostore");
    let backend = Backend::with_vault(root.clone(), Arc::new(NoStore)).unwrap();
    let (server, _) = connection(&root);
    let file = backend.write_profile(&server).unwrap();
    assert!(std::fs::read_to_string(&file).unwrap().contains(KEY));
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600); }
    assert!(matches!(yo3_launcher::Profile::read_with(&file, &NoStore).unwrap().credential, yo3_launcher::Credential::Key(k) if k == KEY));
    let log = log_text(&root);
    assert!(log.contains("credential store") && log.contains("owner-only file"), "{log}");
    assert!(!log.contains(KEY), "never the key itself");
    let _ = std::fs::remove_dir_all(root);
}

/// A launcher from before keeps the key in `profiles/connection-0.json` and `profiles/direct.json`: the first start
/// moves it into the store and takes it out of both files. Direct start under Wine reads the prefix's own store, so
/// the key is copied there first (`mirror`); when that fails, direct.json keeps its copy.
#[test]
fn the_first_start_moves_an_old_key_into_the_store() {
    let root = root("migrate");
    let (server, identity) = connection(&root);
    let profiles = root.join("profiles");
    std::fs::create_dir_all(&profiles).unwrap();
    let old = serde_json::json!({"server":server.address,"server_name":"localhost","identity":identity,"key":KEY});
    std::fs::write(profiles.join("connection-0.json"), old.to_string()).unwrap();
    let mut direct = old.clone(); direct["start"] = serde_json::json!({"program":"yo3-server"});
    std::fs::write(profiles.join("direct.json"), direct.to_string()).unwrap();
    // A Multiplayer profile has no key: left alone.
    let multiplayer = serde_json::json!({"server":"x:1","server_name":"x","identity":identity,"vault":"server:x"}).to_string();
    std::fs::write(profiles.join("multiplayer.json"), &multiplayer).unwrap();

    // The Wine store refuses: connection-0.json is moved, direct.json keeps its key.
    let vault = Arc::new(MemoryVault::default());
    let backend = Backend::with_vault(root.clone(), vault.clone()).unwrap();
    backend.migrate_keys(&|_, _| anyhow::bail!("the Wine credential store refused it"));
    let entry = yo3_launcher::vault::key_account(&identity);
    assert_eq!(vault.get(&entry).unwrap().as_deref(), Some(KEY));
    assert!(!std::fs::read_to_string(profiles.join("connection-0.json")).unwrap().contains(KEY));
    assert!(std::fs::read_to_string(profiles.join("direct.json")).unwrap().contains(KEY), "direct start must still find it");

    let mirrored = std::sync::Mutex::new(vec![]);
    backend.migrate_keys(&|entry, key| { mirrored.lock().unwrap().push((entry.to_string(), key.to_string())); Ok(()) });
    assert_eq!(*mirrored.lock().unwrap(), [(entry.clone(), KEY.to_string())]);
    let direct: serde_json::Value = serde_json::from_slice(&std::fs::read(profiles.join("direct.json")).unwrap()).unwrap();
    assert!(direct.get("key").is_none() && direct["key_vault"] == entry.as_str(), "{direct}");
    assert_eq!(direct["start"]["program"], "yo3-server", "the rest stays");
    assert!(matches!(yo3_launcher::Profile::read_with(&profiles.join("direct.json"), vault.as_ref()).unwrap().credential,
        yo3_launcher::Credential::Key(k) if k == KEY));
    assert_eq!(std::fs::read_to_string(profiles.join("multiplayer.json")).unwrap(), multiplayer);
    #[cfg(unix)] for name in ["connection-0.json", "direct.json"] {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(profiles.join(name)).unwrap().permissions().mode() & 0o777, 0o600, "{name}");
    }
    let _ = std::fs::remove_dir_all(&root);

    // Without a store nothing moves.
    let root = self::root("migrate-nostore");
    std::fs::create_dir_all(root.join("profiles")).unwrap();
    std::fs::write(root.join("profiles/connection-0.json"), old.to_string()).unwrap();
    Backend::with_vault(root.clone(), Arc::new(NoStore)).unwrap().migrate_keys(&|_, _| Ok(()));
    assert!(std::fs::read_to_string(root.join("profiles/connection-0.json")).unwrap().contains(KEY));
    let _ = std::fs::remove_dir_all(root);
}
