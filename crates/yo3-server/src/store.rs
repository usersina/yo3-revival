//! Versioned SQLite persistence. Existing JSON documents are imported once, in one transaction, and retained.
//! The serializers are unchanged during the cutover: each domain snapshot is an atomic SQLite document.
use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value as Json;
use rusqlite::{Connection, OptionalExtension};

const DOCUMENTS: &[&str] = &["accounts.json", "records.json", "friends.json", "mail.json", "teams.json", "events.json", "rentals.json"];
const APPLICATION_ID: u32 = 0x594f3301;
const FORMAT_MARKER: &str = "storage.version";

fn sql(e: rusqlite::Error) -> io::Error { io::Error::other(e.to_string()) }

fn connection(dir: &Path) -> io::Result<Connection> {
    let conn = Connection::open_with_flags(dir.join("server.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(sql)?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(sql)?;
    Ok(conn)
}

/// Import all persistent domains together. Failure rolls back the import; the original files remain untouched.
/// Retrying after repairing an input is safe. A completed import is never repeated over newer SQLite state.
pub fn initialize(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    if dir.join(FORMAT_MARKER).exists() && !dir.join("server.db").is_file() {
        return Err(io::Error::other("server.db is missing from a migrated directory; restore its database backup instead of reimporting stale JSON"));
    }
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let mut options = fs::OpenOptions::new(); options.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    match options.open(dir.join("server.db")) {
        Ok(_) => {}, Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}, Err(e) => return Err(e),
    }
    let mut conn = connection(dir)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(sql)?;
    let app: u32 = tx.pragma_query_value(None, "application_id", |r| r.get(0)).map_err(sql)?;
    let version: u32 = tx.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(sql)?;
    if app == APPLICATION_ID && version == 1 {
        let check: String = tx.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(sql)?;
        if check != "ok" { return Err(io::Error::other(format!("server.db integrity check: {check}"))) }
        tx.commit().map_err(sql)?;
        for name in edited_after_import(dir) {
            tracing::warn!("{name} was edited after it was imported into server.db: the edit was ignored. To give BP, use \
                Give BP in the app (Hosting → Players & invitations) or `yo3-server give-bp PLAYER AMOUNT`");
        }
        let old = legacy_documents(dir);
        if !old.is_empty() {
            tracing::warn!("{} {} old copies: their data was imported into server.db, which the server uses, and they are no \
                longer read. They are safe to delete (server.db is backed up in backups/ at every start).",
                old.join(", "), if old.len() == 1 { "is an" } else { "are" });
        }
        fs::write(dir.join(FORMAT_MARKER), "sqlite:1\n")?;
        return Ok(());
    }
    let tables: u32 = tx.query_row("SELECT count(*) FROM sqlite_master WHERE type='table'", [], |r| r.get(0)).map_err(sql)?;
    if app != 0 || version != 0 || tables != 0 {
        return Err(io::Error::other("server.db has an unsupported application or schema version; keep it and use the matching server"));
    }
    tx.execute_batch("CREATE TABLE state_documents (name TEXT PRIMARY KEY, body TEXT NOT NULL CHECK(json_valid(body)));").map_err(sql)?;
    for name in DOCUMENTS {
        let path = dir.join(name);
        let body = match fs::read_to_string(&path) {
            Ok(s) => s, Err(e) if e.kind() == io::ErrorKind::NotFound => continue, Err(e) => return Err(e),
        };
        let document = serde_json::from_str::<Json>(&body).map_err(|e| io::Error::other(format!("{}: {e}; import stopped, repair the source and retry", path.display())))?;
        if *name == "accounts.json" { crate::accounts::Directory::from_document(vec![], &path, Some(document))?; }
        tx.execute("INSERT INTO state_documents(name, body) VALUES (?1, ?2)", (name, body)).map_err(sql)?;
    }
    tx.pragma_update(None, "application_id", APPLICATION_ID).map_err(sql)?;
    tx.pragma_update(None, "user_version", 1).map_err(sql)?;
    tx.commit().map_err(sql)?;
    fs::write(dir.join(FORMAT_MARKER), "sqlite:1\n")?;
    Ok(())
}

/// The legacy JSON documents still beside an imported server.db.
pub fn legacy_documents(dir: &Path) -> Vec<&'static str> {
    DOCUMENTS.iter().copied().filter(|name| dir.join(name).is_file()).collect()
}

/// The legacy JSON documents modified after the last start's marker: hand edits that the database never sees.
pub fn edited_after_import(dir: &Path) -> Vec<&'static str> {
    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    let Some(marker) = modified(&dir.join(FORMAT_MARKER)) else { return vec![] };
    DOCUMENTS.iter().copied().filter(|name| modified(&dir.join(name)).is_some_and(|t| t > marker)).collect()
}

fn database_document(path: &Path) -> Option<(&Path, &str)> {
    let name = path.file_name()?.to_str()?;
    let dir = path.parent()?;
    (DOCUMENTS.contains(&name) && (dir.join("server.db").is_file() || dir.join(FORMAT_MARKER).exists())).then_some((dir, name))
}

/// How many start-up backups are kept.
const BACKUPS: usize = 10;

/// The file's JSON, or None when it doesn't exist yet.
pub fn read(path: &Path) -> io::Result<Option<Json>> {
    if let Some((dir, name)) = database_document(path) {
        let body: Option<String> = connection(dir)?.query_row("SELECT body FROM state_documents WHERE name=?1", [name], |r| r.get(0)).optional().map_err(sql)?;
        return body.map(|s| serde_json::from_str(&s).map_err(io::Error::other)).transpose();
    }
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// The file's JSON for a side file that may start empty (`Json::Null` when it doesn't exist yet). A file that won't
/// parse is not thrown away: the next save would overwrite it, and the docs invite hand-editing. It is renamed to
/// `NAME.corrupt-<time>` and said so loudly, and the caller starts empty.
pub fn read_or_keep(path: &Path, what: &str) -> Json {
    match read(path) {
        Ok(saved) => saved.unwrap_or(Json::Null),
        Err(e) => {
            assert!(database_document(path).is_none(), "cannot load SQLite state for {what}: {e}; refusing to start with empty state");
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
            let mut kept = path.as_os_str().to_owned();
            kept.push(format!(".corrupt-{stamp}"));
            let kept = std::path::PathBuf::from(kept);
            match fs::rename(path, &kept) {
                Ok(()) => tracing::error!("{what} NOT LOADED: {e}. The file was kept as {} (fix it and put it back); starting empty", kept.display()),
                Err(re) => tracing::error!("{what} NOT LOADED: {e}. It could not be moved aside either ({re}); the next save overwrites it"),
            }
            Json::Null
        }
    }
}

/// Writes the whole file (pretty, so the host can read and edit it).
pub fn write(path: &Path, value: &Json) -> io::Result<()> {
    if let Some((dir, name)) = database_document(path) {
        connection(dir)?.execute("INSERT INTO state_documents(name, body) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET body=excluded.body",
            (name, value.to_string())).map_err(sql)?;
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value).map_err(io::Error::other)?)?;
    fs::rename(&tmp, path)
}

/// Copies the SQLite database consistently into `backups/<time>/`, keeping the newest few.
/// Legacy standalone callers without a database retain their JSON backup behavior.
pub fn back_up(dir: &Path) -> io::Result<()> {
    if dir.join("server.db").is_file() {
        let backups = dir.join("backups");
        let to = backups.join(chrono::Local::now().format("%Y%m%d-%H%M%S%.9f").to_string());
        fs::create_dir_all(&to)?;
        connection(dir)?.backup("main", to.join("server.db"), None::<fn(rusqlite::backup::Progress)>).map_err(sql)?;
        rotate(&backups)?;
        return Ok(());
    }
    // Only files that parse are backed up: a damaged file must not rotate the last good copy out.
    let parses = |p: &std::path::PathBuf| match fs::read(p).map(|b| serde_json::from_slice::<Json>(&b)) {
        Ok(Ok(_)) => true,
        _ => {
            tracing::error!("{} does not parse; it is not backed up", p.display());
            false
        }
    };
    let files: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter(parses)
            .collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if files.is_empty() {
        return Ok(());
    }
    let backups = dir.join("backups");
    let to = backups.join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string());
    fs::create_dir_all(&to)?;
    for f in &files {
        if let Some(name) = f.file_name() {
            fs::copy(f, to.join(name))?;
        }
    }
    rotate(&backups)
}

fn rotate(backups: &Path) -> io::Result<()> {
    let mut kept: Vec<_> = fs::read_dir(backups)?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    kept.sort();
    for old in kept.iter().rev().skip(BACKUPS) {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("yo3-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sqlite_import_preserves_every_document_and_never_imports_stale_json_again() {
        let d = dir("sqlite-import");
        for name in ["accounts", "records", "friends", "mail", "teams", "events", "rentals"] {
            fs::write(d.join(format!("{name}.json")), format!("{{\"source\":\"{name}\",\"id\":1007}}")).unwrap();
        }
        initialize(&d).unwrap();
        assert!(d.join("server.db").is_file());
        for name in ["accounts", "records", "friends", "mail", "teams", "events", "rentals"] {
            let file = d.join(format!("{name}.json"));
            assert_eq!(read(&file).unwrap().unwrap()["source"], name);
            let original = fs::read(&file).unwrap();
            write(&file, &serde_json::json!({ "updated": true })).unwrap();
            assert_eq!(fs::read(&file).unwrap(), original, "original JSON retained as migration backup");
        }
        initialize(&d).unwrap();
        assert_eq!(read(&d.join("accounts.json")).unwrap().unwrap(), serde_json::json!({ "updated": true }));
        back_up(&d).unwrap();
        let backup = fs::read_dir(d.join("backups")).unwrap().next().unwrap().unwrap().path();
        let conn = rusqlite::Connection::open(backup.join("server.db")).unwrap();
        let count: u32 = conn.query_row("SELECT count(*) FROM state_documents", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 7);
        drop(conn);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn json_edited_after_the_import_is_reported_not_reimported() {
        // A host who raised BP by editing accounts.json after the import saw nothing change (issue #6).
        let d = dir("sqlite-edited");
        fs::write(d.join("accounts.json"), "{\"next_user\":1042}").unwrap();
        initialize(&d).unwrap();
        assert!(edited_after_import(&d).is_empty());
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        fs::write(d.join("accounts.json"), "{\"next_user\":9999}").unwrap();
        fs::File::options().write(true).open(d.join("accounts.json")).unwrap().set_modified(later).unwrap();
        assert_eq!(edited_after_import(&d), ["accounts.json"]);
        assert_eq!(legacy_documents(&d), ["accounts.json"], "reported as an old copy that is safe to delete");
        initialize(&d).unwrap();
        assert_eq!(read(&d.join("accounts.json")).unwrap().unwrap()["next_user"], 1042, "server.db stays authoritative");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn a_failed_sqlite_import_is_retryable_and_preserves_original_files() {
        let d = dir("sqlite-retry");
        fs::write(d.join("accounts.json"), "{\"next_user\":1042}").unwrap();
        fs::write(d.join("friends.json"), "broken").unwrap();
        assert!(initialize(&d).is_err());
        assert_eq!(fs::read_to_string(d.join("friends.json")).unwrap(), "broken");
        fs::write(d.join("friends.json"), "{\"friends\":[]}").unwrap();
        initialize(&d).unwrap();
        assert_eq!(read(&d.join("accounts.json")).unwrap().unwrap()["next_user"], 1042);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn a_missing_migrated_database_never_falls_back_to_stale_json() {
        let d = dir("sqlite-missing");
        fs::write(d.join("accounts.json"), "{\"original\":true}").unwrap();
        initialize(&d).unwrap();
        fs::remove_file(d.join("server.db")).unwrap();
        assert!(initialize(&d).is_err());
        assert!(read(&d.join("accounts.json")).is_err());
        assert!(write(&d.join("accounts.json"), &serde_json::json!({"lost":true})).is_err());
        assert_eq!(fs::read_to_string(d.join("accounts.json")).unwrap(), "{\"original\":true}");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn invalid_account_records_do_not_complete_migration_or_reveal_credentials() {
        let d = dir("sqlite-invalid-account");
        let key = "s".repeat(20);
        fs::write(d.join("accounts.json"), serde_json::json!({"accounts":[{"user_id":"bad","key":key,"players":[]}]}).to_string()).unwrap();
        let error = initialize(&d).unwrap_err().to_string();
        assert!(!error.contains(&key));
        assert!(!d.join(FORMAT_MARKER).exists());
        fs::write(d.join("accounts.json"), serde_json::json!({"accounts":[{"user_id":1042,"key":key,"players":[]}]}).to_string()).unwrap();
        initialize(&d).unwrap();
        let directory = crate::accounts::Directory::open(vec![], &d.join("accounts.json")).unwrap();
        assert_eq!(directory.entries().next().unwrap().2.user_id, 1042);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn a_file_that_does_not_parse_is_kept_not_overwritten() {
        let d = dir("corrupt");
        let file = d.join("friends.json");
        fs::write(&file, "{ \"friends\": [[1, 2]").unwrap();
        assert_eq!(read_or_keep(&file, "friends"), Json::Null);
        assert!(!file.exists(), "moved aside");
        let kept: Vec<_> = fs::read_dir(&d).unwrap().filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(kept.len(), 1);
        assert!(kept[0].starts_with("friends.json.corrupt-"), "{kept:?}");
        assert_eq!(fs::read_to_string(d.join(&kept[0])).unwrap(), "{ \"friends\": [[1, 2]");
        // A missing file is simply new.
        assert_eq!(read_or_keep(&d.join("none.json"), "none"), Json::Null);
    }

    #[test]
    fn a_damaged_file_is_not_backed_up() {
        let d = dir("backup");
        write(&d.join("good.json"), &serde_json::json!({ "a": 1 })).unwrap();
        fs::write(d.join("bad.json"), "{{{").unwrap();
        back_up(&d).unwrap();
        let backup = fs::read_dir(d.join("backups")).unwrap().next().unwrap().unwrap().path();
        assert!(backup.join("good.json").is_file());
        assert!(!backup.join("bad.json").exists());
        // With nothing sound to copy no backup directory is made, so none of the good ones is rotated out.
        let d = dir("backup-none");
        fs::write(d.join("bad.json"), "{{{").unwrap();
        back_up(&d).unwrap();
        assert!(!d.join("backups").exists());
    }
}
