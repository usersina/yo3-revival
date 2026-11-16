//! `launcher.db`'s schema upgrades: a database an older release made opens in this one with what it kept.
use yo3_desktop::LauncherDb;

fn root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("yo3-launcher-db-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); std::fs::create_dir_all(&root).unwrap(); root
}

/// v0.0.21's schema (user_version 1), word for word, with its saved connections: rows that carried a friend's account
/// key. Account keys are gone from Multiplayer (#23); the table goes, and with it the keys' bytes on disk.
#[test]
fn a_v0_0_21_database_upgrades_without_its_saved_connections_and_their_keys() {
    let root = root("v0021");
    let key = "Zq8wKp3TnV7xR2mL5cYh9bJd4fGs6uEa";
    {
        let db = rusqlite::Connection::open(root.join("launcher.db")).unwrap();
        db.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS servers (id INTEGER PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1; COMMIT;").unwrap();
        db.execute("INSERT INTO settings VALUES(1,?1)", [r#"{"advanced_mode":true,"bind_address":"127.0.0.1","listen_port":7443,"resolution":"auto"}"#]).unwrap();
        db.execute("INSERT INTO servers VALUES(1,?1)", [format!(r#"{{"id":1,"label":"Friend","address":"192.0.2.7:7443","server_name":"192.0.2.7","certificate":"-----BEGIN CERTIFICATE-----","key":"{key}"}}"#)]).unwrap();
    }
    let db = LauncherDb::open(&root).unwrap();
    let settings = db.settings().unwrap();
    assert!(settings.advanced_mode && settings.resolution == "auto", "the settings stay");
    assert!(db.remote_servers().unwrap().is_empty());
    drop(db);
    let raw = rusqlite::Connection::open(root.join("launcher.db")).unwrap();
    let version: u32 = raw.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
    assert_eq!(version, yo3_desktop::COMPATIBLE, "released launchers can still open it");
    let schema: u32 = raw.query_row("SELECT version FROM launcher_schema WHERE id=1", [], |r| r.get(0)).unwrap();
    assert_eq!(schema, yo3_desktop::LAUNCHER_SCHEMA);
    let tables: Vec<String> = raw.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").unwrap()
        .query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
    assert_eq!(tables, ["launcher_schema", "remote_servers", "settings"], "the old saved connections are dropped");
    drop(raw);
    let bytes = std::fs::read(root.join("launcher.db")).unwrap();
    assert!(!bytes.windows(key.len()).any(|w| w == key.as_bytes()), "the dropped rows' keys are overwritten, not left in free pages");
    // Opening it again changes nothing.
    LauncherDb::open(&root).unwrap();
    let _ = std::fs::remove_dir_all(root);
}

/// The beta's own schema (user_version 2: the Multiplayer servers) upgrades the same way and keeps those servers.
#[test]
fn a_version_2_database_keeps_its_multiplayer_servers() {
    let root = root("v2");
    {
        let db = rusqlite::Connection::open(root.join("launcher.db")).unwrap();
        db.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS servers (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS remote_servers (key TEXT PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=2; COMMIT;").unwrap();
        let identity = "ab".repeat(32);
        db.execute("INSERT INTO remote_servers VALUES(?1,?2)", [identity.clone(), format!(r#"{{"name":"Friends","address":"192.0.2.7:7443","server_name":"192.0.2.7","identities":["{identity}"],"official":false}}"#)]).unwrap();
    }
    let db = LauncherDb::open(&root).unwrap();
    assert_eq!(db.remote_servers().unwrap().len(), 1);
    drop(db);
    let raw = rusqlite::Connection::open(root.join("launcher.db")).unwrap();
    assert_eq!(raw.query_row("SELECT count(*) FROM sqlite_master WHERE name='servers'", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_newer_database_is_refused() {
    let root = root("newer");
    rusqlite::Connection::open(root.join("launcher.db")).unwrap().execute_batch("PRAGMA user_version=4;").unwrap();
    assert!(LauncherDb::open(&root).is_err());
    let _ = std::fs::remove_dir_all(root);
}
