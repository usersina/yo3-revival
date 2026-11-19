//! The server's state in SQLite (`server.db`, schema 6), one row per thing (#22), with durable operation receipts.
//!
//! Each domain (accounts, records, friends, mail, teams, events, rentals) still hands the store its whole state as one
//! JSON value, through `write(dir/NAME.json, value)`, and gets it back from `read`. The store splits that value into
//! rows: one per account (with its players), BP code, record pair, friendship, mail, team, event and rental, keyed by
//! the item's own id, plus the domain's counters. A save compares the rows with what was last written and writes only
//! the changed ones, in one transaction, on a connection kept open in WAL mode. One deck save rewrites one account's
//! row, not every account's collection.
//!
//! An action that changes more than one domain saves them together ([`together`]: one transaction, the domains'
//! saves as savepoints inside it), so a duel's BP and its record are kept both or neither. Every user of the folder's
//! one connection holds it while it works ([`hold`]); the others wait for it.
//!
//! A save that fails marks the store unhealthy ([`healthy`]): new logins are refused until a save succeeds again, so
//! memory and disk never silently drift apart while players keep playing. Backups: at start, every hour and at
//! shutdown (`back_up`), restored offline with `yo3-server restore` ([`restore`]).
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use serde_json::Value as Json;
use rusqlite::{Connection, OptionalExtension};

/// The domains kept in server.db, by the file name their code still uses as a handle.
const DOMAINS: &[&str] = &["accounts.json", "records.json", "friends.json", "mail.json", "teams.json", "events.json", "rentals.json", "settings.json"];
const APPLICATION_ID: u32 = 0x594f3301;
/// Schema 1 kept each domain as one JSON document (`state_documents`); schema 2 keeps one row per item.
const SCHEMA: u32 = 6;
const SCHEMA_SQL: &str = "
    CREATE TABLE IF NOT EXISTS domains (name TEXT PRIMARY KEY, scalars TEXT NOT NULL CHECK(json_valid(scalars)),
        shape TEXT NOT NULL CHECK(json_valid(shape)));
    CREATE TABLE IF NOT EXISTS items (domain TEXT NOT NULL, field TEXT NOT NULL, key TEXT NOT NULL,
        body TEXT NOT NULL CHECK(json_valid(body)), PRIMARY KEY (domain, field, key)) WITHOUT ROWID;";

pub(crate) fn sql(e: rusqlite::Error) -> io::Error { io::Error::other(e.to_string()) }

/// A data folder's one open connection, in WAL mode: saves don't reopen the database, and backups read beside them.
/// Whoever uses it takes it out ([`hold`]) and puts it back; the others wait for it.
struct Slot { conn: Mutex<Option<Connection>>, back: std::sync::Condvar }
static CONNECTIONS: LazyLock<Mutex<HashMap<PathBuf, Arc<Slot>>>> = LazyLock::new(Default::default);

fn connection(dir: &Path) -> io::Result<Arc<Slot>> {
    let mut all = CONNECTIONS.lock().unwrap();
    if let Some(slot) = all.get(dir) { return Ok(slot.clone()) }
    let conn = Connection::open_with_flags(dir.join("server.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(sql)?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(sql)?;
    conn.pragma_update(None, "journal_mode", "WAL").map_err(sql)?;
    conn.pragma_update(None, "synchronous", "NORMAL").map_err(sql)?;
    conn.pragma_update(None, "foreign_keys", true).map_err(sql)?;
    let slot = Arc::new(Slot { conn: Mutex::new(Some(conn)), back: std::sync::Condvar::new() });
    all.insert(dir.to_path_buf(), slot.clone());
    Ok(slot)
}

/// Saves on this thread made inside [`together`]: its folder, the connection while no save holds it, and the caches'
/// updates, made only once the transaction commits.
struct Batch { dir: PathBuf, conn: Option<Connection>, after: Vec<Box<dyn FnOnce()>> }
thread_local! { static BATCH: std::cell::RefCell<Option<Batch>> = const { std::cell::RefCell::new(None) }; }

fn in_batch(dir: &Path) -> bool { BATCH.with(|b| b.borrow().as_ref().is_some_and(|b| b.dir == dir)) }

/// `dir`'s connection, held until dropped: from this thread's [`together`] when it is in one for `dir`, else from the
/// folder's slot (waiting while another thread holds it).
pub(crate) struct Held { conn: Option<Connection>, home: Option<Arc<Slot>> }
impl std::ops::Deref for Held { type Target = Connection; fn deref(&self) -> &Connection { self.conn.as_ref().unwrap() } }
impl std::ops::DerefMut for Held { fn deref_mut(&mut self) -> &mut Connection { self.conn.as_mut().unwrap() } }
impl Drop for Held {
    fn drop(&mut self) {
        let Some(conn) = self.conn.take() else { return };
        match &self.home {
            Some(slot) => { *slot.conn.lock().unwrap_or_else(|e| e.into_inner()) = Some(conn); slot.back.notify_one(); }
            None => BATCH.with(|b| if let Some(b) = b.borrow_mut().as_mut() { b.conn = Some(conn) }),
        }
    }
}

pub(crate) fn hold(dir: &Path) -> io::Result<Held> {
    if in_batch(dir) {
        let conn = BATCH.with(|b| b.borrow_mut().as_mut().and_then(|b| b.conn.take()));
        return conn.map(|conn| Held { conn: Some(conn), home: None }).ok_or_else(|| io::Error::other("the store's connection is already held on this thread"));
    }
    let slot = connection(dir)?;
    let conn = {
        let mut conn = slot.conn.lock().unwrap_or_else(|e| e.into_inner());
        while conn.is_none() { conn = slot.back.wait(conn).unwrap_or_else(|e| e.into_inner()); }
        conn.take()
    };
    Ok(Held { conn, home: Some(slot) })
}

/// Runs `f` so that what it saves to `dir` is saved together, in one transaction, or not at all (#22: a duel's BP and
/// its record). Saves inside run as savepoints; another thread's saves to `dir` wait until it ends. Its result, or
/// why nothing was saved: `f`'s error, or the commit's. Without a database (state kept in files), `f` alone.
pub fn together<T>(dir: &Path, f: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    if in_batch(dir) || !dir.join("server.db").is_file() { return f() }
    let mut held = hold(dir)?;
    held.execute_batch("BEGIN IMMEDIATE").map_err(sql)?;
    BATCH.with(|b| *b.borrow_mut() = Some(Batch { dir: dir.to_path_buf(), conn: held.conn.take(), after: vec![] }));
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let batch = BATCH.with(|b| b.borrow_mut().take()).expect("together's batch");
    held.conn = batch.conn;
    let out = match out {
        Err(panic) => { let _ = held.execute_batch("ROLLBACK"); drop(held); std::panic::resume_unwind(panic) }
        Ok(out) => out,
    };
    let done = match out {
        Ok(value) => held.execute_batch("COMMIT").map_err(sql).map(|()| value),
        Err(e) => Err(e),
    };
    match done {
        Ok(value) => { drop(held); batch.after.into_iter().for_each(|update| update()); Ok(value) }
        Err(e) => {
            if !held.is_autocommit() { let _ = held.execute_batch("ROLLBACK"); }
            if !UNHEALTHY.swap(true, Ordering::Relaxed) {
                tracing::error!("SAVE FAILED (together: {e}). New logins are refused until a save succeeds; players already in keep playing. Check the disk.");
            }
            Err(e)
        }
    }
}

/// Runs `update` (a cache of what was written) once what was just written is committed: now, or at the end of this
/// thread's [`together`] (never, if it rolls back).
fn after_commit(dir: &Path, update: impl FnOnce() + 'static) {
    let update: Box<dyn FnOnce()> = Box::new(update);
    let pending = BATCH.with(|b| match b.borrow_mut().as_mut() {
        Some(batch) if batch.dir == dir => { batch.after.push(update); None }
        _ => Some(update),
    });
    if let Some(update) = pending { update() }
}

/// Runs `f` as one write: its own immediate transaction, or a savepoint inside this thread's [`together`].
fn write_in<R>(dir: &Path, conn: &mut Connection, f: impl FnOnce(&Connection) -> io::Result<R>) -> io::Result<R> {
    if in_batch(dir) {
        conn.execute_batch("SAVEPOINT store_write").map_err(sql)?;
        return match f(conn) {
            Ok(value) => conn.execute_batch("RELEASE store_write").map_err(sql).map(|()| value),
            Err(e) => { let _ = conn.execute_batch("ROLLBACK TO store_write; RELEASE store_write"); Err(e) }
        };
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(sql)?;
    let value = f(&tx)?;
    tx.commit().map_err(sql)?;
    Ok(value)
}

/// Closes this process's connection to `dir`'s database (before a restore replaces the file, and in tests).
pub fn close(dir: &Path) {
    CONNECTIONS.lock().unwrap().remove(dir);
    SAVED.lock().unwrap().retain(|(d, _), _| d != dir);
}

/// The data folder held by one process (`lock`): released when dropped or when the process ends, however it ends.
#[derive(Debug)]
pub struct DataLock {
    _file: fs::File,
}

/// Takes the data folder for this process: an exclusive OS lock on its `server.lock`, refused while another process
/// (or another handle in this one) holds it. The file left behind after a crash holds nothing. Two owners of one
/// folder each saved their own copy of the accounts over the other's.
pub fn lock(dir: &Path) -> io::Result<DataLock> {
    use std::io::{Read, Seek, Write};
    fs::create_dir_all(dir)?;
    let path = dir.join("server.lock");
    let mut file = fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            let mut holder = String::new();
            let _ = file.read_to_string(&mut holder);
            let holder = holder.trim();
            let holder = if holder.is_empty() { String::new() } else { format!(" ({holder})") };
            return Err(io::Error::new(io::ErrorKind::WouldBlock, format!(
                "{} is in use by another YO3 Revival server or command{holder}. Stop it first, or choose another --data-dir",
                dir.display())));
        }
        Err(fs::TryLockError::Error(e)) => return Err(e),
    }
    // Who holds it, for the message above (a note only: the lock is the OS's hold, not this text).
    file.set_len(0)?;
    file.seek(io::SeekFrom::Start(0))?;
    let _ = writeln!(file, "pid {}", std::process::id());
    Ok(DataLock { _file: file })
}

/// Opens (or creates) `dir`'s database. Older schemas are converted in place, in one transaction.
/// JSON documents from versions before the database are not imported: the beta starts fresh (D9).
pub fn initialize(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let mut options = fs::OpenOptions::new(); options.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    match options.open(dir.join("server.db")) {
        Ok(_) => {}, Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}, Err(e) => return Err(e),
    }
    let mut conn = hold(dir)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(sql)?;
    let app: u32 = tx.pragma_query_value(None, "application_id", |r| r.get(0)).map_err(sql)?;
    let version: u32 = tx.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(sql)?;
    let tables: u32 = tx.query_row("SELECT count(*) FROM sqlite_master WHERE type='table'", [], |r| r.get(0)).map_err(sql)?;
    match (app, version) {
        (APPLICATION_ID, SCHEMA) => {
            let check: String = tx.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(sql)?;
            if check != "ok" { return Err(io::Error::other(format!("server.db integrity check: {check}"))) }
        }
        (APPLICATION_ID, 1) => {
            // Schema 1: each domain one document. Split them into rows.
            tx.execute_batch(SCHEMA_SQL).map_err(sql)?;
            let documents: Vec<(String, String)> = {
                let mut query = tx.prepare("SELECT name, body FROM state_documents").map_err(sql)?;
                query.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(sql)?.collect::<Result<_, _>>().map_err(sql)?
            };
            for (name, body) in documents {
                let value: Json = serde_json::from_str(&body).map_err(io::Error::other)?;
                store_rows(&tx, &name, &split(&value), None)?;
            }
            tx.execute_batch("DROP TABLE state_documents;").map_err(sql)?;
            tx.pragma_update(None, "user_version", SCHEMA).map_err(sql)?;
        }
        (APPLICATION_ID, 2..=5) => { tx.pragma_update(None, "user_version", SCHEMA).map_err(sql)?; }
        (0, 0) if tables == 0 => {
            tx.execute_batch(SCHEMA_SQL).map_err(sql)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID).map_err(sql)?;
            tx.pragma_update(None, "user_version", SCHEMA).map_err(sql)?;
        }
        _ => return Err(io::Error::other("server.db has an unsupported application or schema version; keep it and use the matching server")),
    }
    tx.execute_batch(DUEL_LOGS_TABLE).map_err(sql)?;
    tx.execute_batch(AUDIT_TABLE).map_err(sql)?;
    tx.execute_batch(crate::reports::TABLE).map_err(sql)?;
    tx.execute_batch(RECEIPTS_TABLE).map_err(sql)?;
    let effects_exist: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='operation_effects')", [], |r| r.get(0)).map_err(sql)?;
    if effects_exist {
        let has_kind: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('operation_effects') WHERE name='kind')", [], |r| r.get(0)).map_err(sql)?;
        if !has_kind || version < SCHEMA {
            if version == SCHEMA { return Err(io::Error::other("server.db effect schema is incomplete")) }
            tx.execute_batch("ALTER TABLE operation_effects RENAME TO operation_effects_previous;").map_err(sql)?;
            tx.execute_batch(EFFECTS_TABLE).map_err(sql)?;
            let kind=if has_kind { "kind" } else { "'disconnect'" };
            tx.execute_batch(&format!("INSERT INTO operation_effects(actor,request_id,user_id,kind,state,created,updated) SELECT actor,request_id,user_id,{kind},state,created,updated FROM operation_effects_previous; DROP TABLE operation_effects_previous;")).map_err(sql)?;
        }
    }
    tx.execute_batch(EFFECTS_TABLE).map_err(sql)?;
    tx.commit().map_err(sql)
}

// Receipts deliberately survive audit pruning. No expiry: a request ID is never silently reusable.
const RECEIPTS_TABLE: &str = "CREATE TABLE IF NOT EXISTS operation_receipts (
    actor TEXT NOT NULL, request_id TEXT NOT NULL, operation TEXT NOT NULL, input TEXT NOT NULL CHECK(json_valid(input)),
    result TEXT NOT NULL CHECK(json_valid(result)), created INTEGER NOT NULL,
    PRIMARY KEY(actor, request_id)) WITHOUT ROWID;";

#[derive(Clone)]
pub(crate) struct Receipt { pub operation: String, pub input: String, pub result: Json }

pub(crate) fn receipt(dir: &Path, actor: &str, request_id: &str) -> io::Result<Option<Receipt>> {
    hold(dir)?.query_row("SELECT operation, input, result FROM operation_receipts WHERE actor=?1 AND request_id=?2", (actor, request_id), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    }).optional().map_err(sql)?.map(|(operation, input, result)| {
        Ok(Receipt { operation, input, result: serde_json::from_str(&result).map_err(io::Error::other)? })
    }).transpose()
}

/// A receipt is written only as part of the operation's transaction, never on its own.
pub(crate) fn put_receipt(dir: &Path, actor: &str, request_id: &str, receipt: &Receipt) -> io::Result<()> {
    if !in_batch(dir) { return Err(io::Error::other("an operation receipt needs its operation's transaction")) }
    hold(dir)?.execute("INSERT INTO operation_receipts(actor, request_id, operation, input, result, created) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![actor, request_id, receipt.operation, receipt.input, receipt.result.to_string(), crate::logins::now()]).map_err(sql)?;
    Ok(())
}

/// Every administrative change (#25, `api.rs`): who made it, from where, what and on what, the values before and after.
// Effects are separate from prunable audit rows. A receipt retry observes the outcome; it never executes it again.
const EFFECTS_TABLE: &str = "CREATE TABLE IF NOT EXISTS operation_effects (
    actor TEXT NOT NULL, request_id TEXT NOT NULL, user_id INTEGER,
    kind TEXT NOT NULL DEFAULT 'disconnect' CHECK(kind IN ('disconnect','stop','backup')),
    state TEXT NOT NULL CHECK(state IN ('pending','completed','interrupted','failed')),
    created INTEGER NOT NULL, updated INTEGER NOT NULL,
    CHECK((kind='disconnect' AND user_id IS NOT NULL) OR (kind IN ('stop','backup') AND user_id IS NULL)),
    PRIMARY KEY(actor, request_id), FOREIGN KEY(actor, request_id) REFERENCES operation_receipts(actor, request_id)
) WITHOUT ROWID;";
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectState { Pending, Completed, Interrupted, Failed }
impl EffectState {
    fn text(self) -> &'static str { match self { Self::Pending => "pending", Self::Completed => "completed", Self::Interrupted => "interrupted", Self::Failed => "failed" } }
}
pub(crate) fn put_effect(dir: &Path, actor: &str, id: &str, user: u32) -> io::Result<()> {
    if !in_batch(dir) { return Err(io::Error::other("an effect intent needs its operation's transaction")) }
    let now = crate::logins::now();
    hold(dir)?.execute("INSERT INTO operation_effects(actor,request_id,user_id,state,created,updated) VALUES (?1,?2,?3,'pending',?4,?4)",
        rusqlite::params![actor,id,user,now]).map_err(sql)?; Ok(())
}
pub(crate) fn put_stop_effect(dir: &Path, actor: &str, id: &str) -> io::Result<()> {
    if !in_batch(dir) { return Err(io::Error::other("a stop intent needs its operation's transaction")) }
    let now = crate::logins::now();
    hold(dir)?.execute("INSERT INTO operation_effects(actor,request_id,kind,state,created,updated) VALUES (?1,?2,'stop','pending',?3,?3)",
        rusqlite::params![actor,id,now]).map_err(sql)?; Ok(())
}
pub(crate) fn put_backup_effect(dir: &Path, actor: &str, id: &str) -> io::Result<()> {
    if !in_batch(dir) { return Err(io::Error::other("a backup intent needs its operation's transaction")) }
    let now=crate::logins::now();
    hold(dir)?.execute("INSERT INTO operation_effects(actor,request_id,kind,state,created,updated) VALUES (?1,?2,'backup','pending',?3,?3)",rusqlite::params![actor,id,now]).map_err(sql)?;Ok(())
}
pub(crate) fn effect(dir: &Path, actor: &str, id: &str) -> io::Result<Option<EffectState>> {
    let state: Option<String> = hold(dir)?.query_row("SELECT state FROM operation_effects WHERE actor=?1 AND request_id=?2",(actor,id),|r|r.get(0)).optional().map_err(sql)?;
    state.map(|s| match s.as_str() { "pending" => Ok(EffectState::Pending), "completed" => Ok(EffectState::Completed), "interrupted" => Ok(EffectState::Interrupted), "failed" => Ok(EffectState::Failed), _ => Err(io::Error::other("unknown effect state")) }).transpose()
}
fn record_effect_state(dir: &Path, actor: &str, id: &str, state: EffectState) -> io::Result<()> {
    let conn = hold(dir)?;
    let kind: String = conn.query_row("SELECT kind FROM operation_effects WHERE actor=?1 AND request_id=?2", (actor,id), |r| r.get(0)).map_err(sql)?;
    let path = match kind.as_str() { "disconnect" => "$.disconnect", "stop" => "$.stop", "backup" => "$.backup", _ => return Err(io::Error::other("unknown effect kind")) };
    conn.execute("UPDATE operation_effects SET state=?3, updated=?4 WHERE actor=?1 AND request_id=?2 AND state='pending'",
        rusqlite::params![actor,id,state.text(),crate::logins::now()]).map_err(sql)?;
    conn.execute("UPDATE audit SET detail=json_set(detail,?4,?3) WHERE json_extract(detail,'$.receipt_actor')=?1 AND json_extract(detail,'$.request_id')=?2",
        rusqlite::params![actor,id,state.text(),path]).map_err(sql)?;
    Ok(())
}
pub(crate) fn finish_backup_effect(dir:&Path,actor:&str,id:&str,success:bool)->io::Result<()> {
    together(dir,||record_effect_state(dir,actor,id,if success {EffectState::Completed}else{EffectState::Failed}))
}
pub(crate) fn complete_effect(dir: &Path, actor: &str, id: &str) -> io::Result<()> {
    together(dir,|| record_effect_state(dir,actor,id,EffectState::Completed))
}
/// Only at process startup under the exclusive data-folder lock. Never replay a possibly executed disconnect.
pub fn interrupt_effects(dir: &Path) -> io::Result<usize> {
    together(dir,|| {
        let pending: Vec<(String,String)> = {
            let conn = hold(dir)?;
            let mut q = conn.prepare("SELECT actor,request_id FROM operation_effects WHERE state='pending'").map_err(sql)?;
            q.query_map([],|r|Ok((r.get(0)?,r.get(1)?))).map_err(sql)?.collect::<Result<_,_>>().map_err(sql)?
        };
        for (actor,id) in &pending { record_effect_state(dir,actor,id,EffectState::Interrupted)?; }
        Ok(pending.len())
    })
}

const AUDIT_TABLE: &str = "CREATE TABLE IF NOT EXISTS audit (id INTEGER PRIMARY KEY, at TEXT NOT NULL, actor TEXT NOT NULL,
    origin TEXT NOT NULL, op TEXT NOT NULL, target TEXT NOT NULL, before TEXT, after TEXT, detail TEXT);";

/// One administrative change, as the audit log keeps it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AuditRow {
    pub at: String,
    pub actor: String,
    pub origin: String,
    pub op: String,
    pub target: String,
    pub before: Option<Json>,
    pub after: Option<Json>,
    /// The request's other values (secrets left out).
    pub detail: Option<Json>,
}

/// Adds `row` to `dir`'s audit log.
pub fn audit(dir: &Path, row: &AuditRow) -> io::Result<()> {
    let conn = hold(dir)?;
    let text = |v: &Option<Json>| v.as_ref().map(Json::to_string);
    conn.execute("INSERT INTO audit (at, actor, origin, op, target, before, after, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![row.at, row.actor, row.origin, row.op, row.target, text(&row.before), text(&row.after), text(&row.detail)]).map_err(sql)?;
    Ok(())
}

/// The newest `limit` rows of `dir`'s audit log, newest first.
pub fn audit_rows(dir: &Path, limit: u32) -> io::Result<Vec<AuditRow>> {
    let conn = hold(dir)?;
    let mut query = conn.prepare("SELECT at, actor, origin, op, target, before, after, detail FROM audit ORDER BY id DESC LIMIT ?1").map_err(sql)?;
    let json = |s: Option<String>| s.and_then(|s| serde_json::from_str(&s).ok());
    query.query_map([limit], |r| Ok(AuditRow { at: r.get(0)?, actor: r.get(1)?, origin: r.get(2)?, op: r.get(3)?, target: r.get(4)?,
        before: json(r.get(5)?), after: json(r.get(6)?), detail: json(r.get(7)?) }))
        .map_err(sql)?.collect::<Result<_, _>>().map_err(sql)
}

/// Audit projections retain SQLite IDs as i64 internally; the HTTP contract sends them as decimal strings.
pub(crate) struct AuditRecord {
    pub id:i64,pub at:String,pub actor:String,pub origin:String,pub op:String,pub target:String,
    pub before:Option<String>,pub after:Option<String>,pub detail:Option<String>,
}
pub(crate) fn audit_page(dir:&Path,limit:u32,before:Option<i64>,search:&str)->io::Result<Vec<AuditRecord>> {
    let conn=hold(dir)?;
    let mut query=conn.prepare("SELECT id,at,actor,origin,op,target,before,after,detail FROM audit WHERE (?1 IS NULL OR id < ?1)
        AND instr(lower(actor || ' ' || origin || ' ' || op || ' ' || target || ' ' || COALESCE(detail,'')),lower(?3))>0 ORDER BY id DESC LIMIT ?2").map_err(sql)?;
    query.query_map(rusqlite::params![before,limit,search],|r|Ok(AuditRecord{id:r.get(0)?,at:r.get(1)?,actor:r.get(2)?,origin:r.get(3)?,op:r.get(4)?,target:r.get(5)?,before:r.get(6)?,after:r.get(7)?,detail:r.get(8)?}))
        .map_err(sql)?.collect::<Result<_,_>>().map_err(sql)
}

/// Drops `dir`'s audit rows older than `days` days (`audit-keep-days`; 0 keeps them all). How many went.
pub fn prune_audit(dir: &Path, days: u32) -> io::Result<usize> {
    if days == 0 || !dir.join("server.db").is_file() { return Ok(0) }
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(i64::from(days))).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    // Rows are written as RFC 3339 UTC to the second ("…Z"), so they sort as text.
    hold(dir)?.execute("DELETE FROM audit WHERE at < ?1", [cutoff]).map_err(sql)
}

/// Takes out of `dir`'s audit log every row about one of `names` (a deleted account's User ID and players, compared
/// ignoring case): its target, a name in a list target, or a new name in its details. How many went.
pub fn forget_in_audit(dir: &Path, names: &[String]) -> io::Result<usize> {
    let conn = hold(dir)?;
    let rows: Vec<(i64, String, Option<String>)> = {
        let mut query = conn.prepare("SELECT id, target, detail FROM audit").map_err(sql)?;
        query.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(sql)?.collect::<Result<_, _>>().map_err(sql)?
    };
    let mut gone = 0;
    for (id, target, detail) in rows {
        let detail = detail.and_then(|d| serde_json::from_str::<Json>(&d).ok());
        if mentions(names, &target, detail.as_ref()) {
            gone += conn.execute("DELETE FROM audit WHERE id=?1", [id]).map_err(sql)?;
        }
    }
    Ok(gone)
}

/// Whether an audit row with `target` and `detail` is about one of `names`.
pub fn mentions(names: &[String], target: &str, detail: Option<&Json>) -> bool {
    let named = |n: &str| names.iter().any(|m| m.eq_ignore_ascii_case(n.trim()));
    target.split(", ").any(named)
        || detail.is_some_and(|d| ["new", "name", "who", "player", "user", "old"].iter().any(|k| d[*k].as_str().is_some_and(named)))
}

/// Deletes the duel logs kept for `players` (each player's own copy: `…-<player id>.dlog`).
pub fn delete_duel_logs_of(dir: &Path, players: &[u32]) -> io::Result<usize> {
    if !dir.join("server.db").is_file() { return Ok(0) }
    let conn = hold(dir)?;
    let mut gone = 0;
    for p in players {
        gone += conn.execute("DELETE FROM duel_logs WHERE name LIKE ?1", [format!("%-{p}.dlog")]).map_err(sql)?;
    }
    Ok(gone)
}

/// Warns about files an older version left in the data folder that the server doesn't read. Called once when a
/// server starts (not by the admin commands, which open the same database).
pub fn report_leftovers(dir: &Path) {
    if dir.join("duellogs").is_dir() {
        tracing::warn!("duellogs/ holds duel logs from an older version: they now live in server.db, and each \
            player's game keeps its own copy. The folder is safe to delete.");
    }
    let old: Vec<&str> = DOMAINS.iter().copied().filter(|name| dir.join(name).is_file()).collect();
    if !old.is_empty() {
        tracing::warn!("{} {} from a version before server.db: the server doesn't read them (the beta starts fresh).",
            old.join(", "), if old.len() == 1 { "is a file" } else { "are files" });
    }
    let _ = fs::remove_file(dir.join("storage.version"));
}

/// Duel logs (`.dlog` files) the client downloads right after a duel between players (`Hub::finish_duel`).
const DUEL_LOGS_TABLE: &str = "CREATE TABLE IF NOT EXISTS duel_logs (name TEXT PRIMARY KEY, body BLOB NOT NULL, created INTEGER NOT NULL);";
/// How many duel logs are kept: each player's game saves its own copy on download, so the server needs only recent ones.
const DUEL_LOGS_KEPT: u32 = 1000;

/// Keeps a duel log in server.db under `name`, dropping the oldest beyond [`DUEL_LOGS_KEPT`].
pub fn put_duel_log(dir: &Path, name: &str, body: &[u8]) -> io::Result<()> {
    if !dir.join("server.db").is_file() { initialize(dir)?; }
    let mut conn = hold(dir)?;
    let tx = conn.transaction().map_err(sql)?;
    tx.execute("INSERT OR REPLACE INTO duel_logs(name, body, created) VALUES (?1, ?2, ?3)",
        (name, body, chrono::Utc::now().timestamp())).map_err(sql)?;
    tx.execute("DELETE FROM duel_logs WHERE name NOT IN (SELECT name FROM duel_logs ORDER BY created DESC, rowid DESC LIMIT ?1)",
        [DUEL_LOGS_KEPT]).map_err(sql)?;
    tx.commit().map_err(sql)
}

/// A kept duel log, if there is one by that name.
pub fn duel_log(dir: &Path, name: &str) -> io::Result<Option<Vec<u8>>> {
    if !dir.join("server.db").is_file() { return Ok(None) }
    hold(dir)?.query_row("SELECT body FROM duel_logs WHERE name=?1", [name], |r| r.get(0)).optional().map_err(sql)
}

/// The kept duel logs' names, newest first.
pub fn duel_log_names(dir: &Path) -> io::Result<Vec<String>> {
    let conn = hold(dir)?;
    let mut query = conn.prepare("SELECT name FROM duel_logs ORDER BY created DESC, rowid DESC").map_err(sql)?;
    query.query_map([], |r| r.get(0)).map_err(sql)?.collect::<Result<_, _>>().map_err(sql)
}

fn database_domain(path: &Path) -> Option<(&Path, &str)> {
    let name = path.file_name()?.to_str()?;
    let dir = path.parent()?;
    (DOMAINS.contains(&name) && dir.join("server.db").is_file()).then_some((dir, name))
}

/// Whether the last save succeeded. While it is false, new logins are refused (players already in keep playing).
pub fn healthy() -> bool { !UNHEALTHY.load(Ordering::Relaxed) }
static UNHEALTHY: AtomicBool = AtomicBool::new(false);

/// A domain's value split into rows: its scalar fields, and per list or map field the items by key.
struct Rows { scalars: serde_json::Map<String, Json>, shape: serde_json::Map<String, Json>, items: Vec<(String, String, String)> }

/// The key of one list item: its own id, so it survives other items coming and going.
fn item_key(item: &Json, seen: &mut HashMap<String, u32>) -> String {
    let number = |v: &Json| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_owned));
    let base = match item {
        Json::Array(pair) if pair.len() == 2 => pair.iter().filter_map(number).collect::<Vec<_>>().join(":"),
        Json::Object(o) if o.contains_key("player_id") && o.contains_key("opponent") =>
            format!("{}:{}", number(&o["player_id"]).unwrap_or_default(), number(&o["opponent"]).unwrap_or_default()),
        Json::Object(o) => ["user_id", "id", "code"].iter().find_map(|k| o.get(*k).and_then(number)).unwrap_or_default(),
        _ => String::new(),
    };
    let base = if base.is_empty() { format!("{:016x}", fnv(item.to_string().as_bytes())) } else { base };
    // The same key twice (it shouldn't happen): keep both, in order.
    let n = seen.entry(base.clone()).or_insert(0);
    *n += 1;
    if *n == 1 { base } else { format!("{base}#{n}") }
}

fn fnv(bytes: &[u8]) -> u64 { bytes.iter().fold(0xcbf29ce484222325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)) }

fn split(value: &Json) -> Rows {
    let mut rows = Rows { scalars: Default::default(), shape: Default::default(), items: Vec::new() };
    let Some(object) = value.as_object() else {
        rows.scalars.insert(String::new(), value.clone());
        return rows;
    };
    for (field, v) in object {
        match v {
            Json::Array(list) => {
                rows.shape.insert(field.clone(), "list".into());
                let mut seen = HashMap::new();
                for item in list { rows.items.push((field.clone(), item_key(item, &mut seen), item.to_string())) }
            }
            Json::Object(map) => {
                rows.shape.insert(field.clone(), "map".into());
                for (key, item) in map { rows.items.push((field.clone(), key.clone(), item.to_string())) }
            }
            other => { rows.scalars.insert(field.clone(), other.clone()); }
        }
    }
    rows
}

/// What each domain's rows were when last read or written, by (domain, field, key): a save writes only differences.
type Written = HashMap<(String, String), String>;
/// Per (data folder, domain): its scalars and shape as written, and its rows.
type Remembered = HashMap<(PathBuf, String), (String, Written)>;
static SAVED: LazyLock<Mutex<Remembered>> = LazyLock::new(Default::default);

/// Writes `rows` for `domain`: the changed items, the removed ones, and the scalars, against `before` (all, if None).
fn store_rows(tx: &Connection, domain: &str, rows: &Rows, before: Option<&(String, Written)>) -> io::Result<()> {
    let head = serde_json::json!([rows.scalars, rows.shape]).to_string();
    if before.is_none_or(|(h, _)| *h != head) {
        tx.execute("INSERT INTO domains(name, scalars, shape) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET scalars=excluded.scalars, shape=excluded.shape",
            (domain, Json::Object(rows.scalars.clone()).to_string(), Json::Object(rows.shape.clone()).to_string())).map_err(sql)?;
    }
    let mut upsert = tx.prepare("INSERT INTO items(domain, field, key, body) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(domain, field, key) DO UPDATE SET body=excluded.body").map_err(sql)?;
    for (field, key, body) in &rows.items {
        if before.is_some_and(|(_, w)| w.get(&(field.clone(), key.clone())) == Some(body)) { continue }
        upsert.execute((domain, field, key, body)).map_err(sql)?;
    }
    match before {
        Some((_, written)) => {
            let now: std::collections::HashSet<(&str, &str)> = rows.items.iter().map(|(f, k, _)| (f.as_str(), k.as_str())).collect();
            let mut delete = tx.prepare("DELETE FROM items WHERE domain=?1 AND field=?2 AND key=?3").map_err(sql)?;
            for (field, key) in written.keys() {
                if !now.contains(&(field.as_str(), key.as_str())) { delete.execute((domain, field, key)).map_err(sql)?; }
            }
        }
        None => {
            // Everything was written above; anything else in the table for this domain is gone.
            let keep: std::collections::HashSet<(&str, &str)> = rows.items.iter().map(|(f, k, _)| (f.as_str(), k.as_str())).collect();
            let present: Vec<(String, String)> = {
                let mut q = tx.prepare("SELECT field, key FROM items WHERE domain=?1").map_err(sql)?;
                q.query_map([domain], |r| Ok((r.get(0)?, r.get(1)?))).map_err(sql)?.collect::<Result<_, _>>().map_err(sql)?
            };
            let mut delete = tx.prepare("DELETE FROM items WHERE domain=?1 AND field=?2 AND key=?3").map_err(sql)?;
            for (field, key) in present.iter().filter(|(f, k)| !keep.contains(&(f.as_str(), k.as_str()))) {
                delete.execute((domain, field, key)).map_err(sql)?;
            }
        }
    }
    Ok(())
}

fn remember(dir: &Path, domain: &str, rows: &Rows) {
    let head = serde_json::json!([rows.scalars, rows.shape]).to_string();
    let written = rows.items.iter().map(|(f, k, b)| ((f.clone(), k.clone()), b.clone())).collect();
    SAVED.lock().unwrap().insert((dir.to_path_buf(), domain.to_owned()), (head, written));
}

/// Sorts keys as numbers where they are numbers (`1002` after `999`, `1001:7` after `1001:12`'s neighbours).
fn key_order(key: &str) -> Vec<(u8, u64, String)> {
    key.split([':', '#']).map(|part| match part.parse::<u64>() { Ok(n) => (0, n, String::new()), Err(_) => (1, 0, part.to_owned()) }).collect()
}

fn read_domain(dir: &Path, domain: &str) -> io::Result<Option<Json>> {
    let conn = hold(dir)?;
    let head: Option<(String, String)> = conn.query_row("SELECT scalars, shape FROM domains WHERE name=?1", [domain], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(sql)?;
    let Some((scalars, shape)) = head else { return Ok(None) };
    let mut scalars: serde_json::Map<String, Json> = serde_json::from_str(&scalars).map_err(io::Error::other)?;
    let shape: serde_json::Map<String, Json> = serde_json::from_str(&shape).map_err(io::Error::other)?;
    let mut items: Vec<(String, String, String)> = {
        let mut q = conn.prepare("SELECT field, key, body FROM items WHERE domain=?1").map_err(sql)?;
        q.query_map([domain], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(sql)?.collect::<Result<_, _>>().map_err(sql)?
    };
    items.sort_by_key(|(field, key, _)| (field.clone(), key_order(key)));
    let rows = Rows { scalars: scalars.clone(), shape: shape.clone(), items: items.clone() };
    if let Some(whole) = scalars.remove("") { remember(dir, domain, &rows); return Ok(Some(whole)) }
    let mut value = scalars;
    for (field, kind) in &shape {
        value.insert(field.clone(), if kind == "map" { Json::Object(Default::default()) } else { Json::Array(vec![]) });
    }
    for (field, key, body) in items {
        let item: Json = serde_json::from_str(&body).map_err(io::Error::other)?;
        match value.get_mut(&field) {
            Some(Json::Array(list)) => list.push(item),
            Some(Json::Object(map)) => { map.insert(key, item); }
            _ => {}
        }
    }
    remember(dir, domain, &rows);
    Ok(Some(Json::Object(value)))
}

fn write_domain(dir: &Path, domain: &str, value: &Json) -> io::Result<()> {
    let rows = split(value);
    let mut conn = hold(dir)?;
    let before = SAVED.lock().unwrap().get(&(dir.to_path_buf(), domain.to_owned())).cloned();
    write_in(dir, &mut conn, |tx| store_rows(tx, domain, &rows, before.as_ref()))?;
    drop(conn);
    let (dir, domain) = (dir.to_path_buf(), domain.to_owned());
    after_commit(&dir.clone(), move || remember(&dir, &domain, &rows));
    Ok(())
}

/// How many backups are kept: a day of hourly ones, or the last starts.
const BACKUPS: usize = 24;
/// How often a running server backs up its database.
pub const BACKUP_EVERY: std::time::Duration = std::time::Duration::from_secs(3600);

/// The file's JSON, or None when it doesn't exist yet.
pub fn read(path: &Path) -> io::Result<Option<Json>> {
    if let Some((dir, name)) = database_domain(path) { return read_domain(dir, name) }
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
            assert!(database_domain(path).is_none(), "cannot load SQLite state for {what}: {e}; refusing to start with empty state");
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

/// Saves a domain's whole state: in server.db only the rows that changed (see the module), else the whole file
/// (pretty, for the tests and tools that keep state outside a data folder).
pub fn write(path: &Path, value: &Json) -> io::Result<()> {
    if let Some((dir, name)) = database_domain(path) {
        let result = write_domain(dir, name, value);
        match &result {
            Ok(()) => if UNHEALTHY.swap(false, Ordering::Relaxed) { tracing::info!("saving works again: logins are open") },
            Err(e) => if !UNHEALTHY.swap(true, Ordering::Relaxed) {
                tracing::error!("SAVE FAILED ({name}: {e}). New logins are refused until a save succeeds; players already in keep playing. Check the disk.");
            },
        }
        return result;
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
pub(crate) fn valid_backup_name(name:&str)->bool {
    (1..=96).contains(&name.len()) && name.as_bytes()[0].is_ascii_alphanumeric()
        && name.bytes().all(|b|b.is_ascii_alphanumeric() || matches!(b,b'-'|b'_'|b'.'))
}
/// Publish a checked SQLite snapshot atomically, never list or download an unfinished copy.
pub(crate) fn back_up_as(dir:&Path,name:&str)->io::Result<()> {
    if !valid_backup_name(name){return Err(io::Error::other("invalid backup name"))}
    let backups=dir.join("backups");fs::create_dir_all(&backups)?;
    if fs::symlink_metadata(&backups)?.file_type().is_symlink(){return Err(io::Error::other("the backup folder cannot be a symbolic link"))}
    let destination=backups.join(name);
    if destination.exists(){return Err(io::Error::new(io::ErrorKind::AlreadyExists,"backup already exists"))}
    let temporary=backups.join(format!(".pending-{name}-{}",crate::logins::new_token()));fs::create_dir(&temporary)?;
    let result=(||{
        let database=temporary.join("server.db");hold(dir)?.backup("main",&database,None::<fn(rusqlite::backup::Progress)>).map_err(sql)?;
        {let check=Connection::open_with_flags(&database,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(sql)?;
            let integrity:String=check.query_row("PRAGMA quick_check",[],|r|r.get(0)).map_err(sql)?;
            if integrity!="ok"{return Err(io::Error::other("backup integrity check failed"))}}
        #[cfg(unix)]{use std::os::unix::fs::PermissionsExt;fs::set_permissions(&database,fs::Permissions::from_mode(0o600))?;}
        fs::OpenOptions::new().read(true).write(true).open(&database)?.sync_all()?;
        #[cfg(unix)]fs::File::open(&temporary)?.sync_all()?;
        fs::rename(&temporary,&destination)?;
        #[cfg(unix)]fs::File::open(&backups)?.sync_all()?;
        Ok(())
    })();
    if result.is_err(){let _=fs::remove_dir_all(&temporary);}
    result?;
    if let Err(e)=rotate(&backups){tracing::error!("Backup completed, but retention cleanup failed: {e}");}
    Ok(())
}

pub fn back_up(dir: &Path) -> io::Result<()> {
    if dir.join("server.db").is_file() {
        return back_up_as(dir,&chrono::Local::now().format("%Y%m%d-%H%M%S%.9f").to_string());
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
    let mut kept: Vec<_> = fs::read_dir(backups)?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.file_name().and_then(|n|n.to_str()).is_some_and(valid_backup_name) && fs::symlink_metadata(p).is_ok_and(|m|m.is_dir() && !m.file_type().is_symlink())).collect();
    kept.sort();
    for old in kept.iter().rev().skip(BACKUPS) {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

/// Whether `path` names a domain kept in server.db (where [`write_changes`] applies).
pub fn in_database(path: &Path) -> bool { database_domain(path).is_some() }

/// Saves part of a domain kept in server.db: its scalars and small lists whole (diffed like [`write`]), and for big
/// lists only the items that changed (`upsert`) or went (`delete`), by key, in one transaction. For a domain whose
/// owner knows what changed (the accounts: copy-on-write), so it never serializes what didn't.
pub fn write_changes(path: &Path, whole: &Json, field: &str, upsert: &[(String, Json)], delete: &[String]) -> io::Result<()> {
    let (dir, domain) = database_domain(path).ok_or_else(|| io::Error::other(format!("{} is not kept in server.db", path.display())))?;
    let result = (|| {
        let mut rows = split(whole);
        rows.shape.insert(field.to_owned(), "list".into());
        let head = serde_json::json!([rows.scalars, rows.shape]).to_string();
        let key = (dir.to_path_buf(), domain.to_owned());
        let mut conn = hold(dir)?;
        // What to write, against what was last written (the cache is updated once this commits).
        let small: std::collections::HashSet<String> = rows.shape.keys().filter(|f| *f != field).cloned().collect();
        let (head_changed, puts, gone) = {
            let saved = SAVED.lock().unwrap();
            let (old_head, written) = saved.get(&key).map_or((None, None), |(h, w)| (Some(h), Some(w)));
            let puts: Vec<&(String, String, String)> = rows.items.iter()
                .filter(|(f, k, body)| written.and_then(|w| w.get(&(f.clone(), k.clone()))) != Some(body)).collect();
            let now: std::collections::HashSet<(&str, &str)> = rows.items.iter().map(|(f, k, _)| (f.as_str(), k.as_str())).collect();
            let gone: Vec<(String, String)> = written.into_iter().flat_map(|w| w.keys())
                .filter(|(f, k)| small.contains(f) && !now.contains(&(f.as_str(), k.as_str()))).cloned().collect();
            (old_head != Some(&head), puts.into_iter().cloned().collect::<Vec<_>>(), gone)
        };
        let bodies: Vec<(String, String)> = upsert.iter().map(|(k, item)| (k.clone(), item.to_string())).collect();
        write_in(dir, &mut conn, |tx| {
            if head_changed {
                tx.execute("INSERT INTO domains(name, scalars, shape) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET scalars=excluded.scalars, shape=excluded.shape",
                    (domain, Json::Object(rows.scalars.clone()).to_string(), Json::Object(rows.shape.clone()).to_string())).map_err(sql)?;
            }
            let mut put = tx.prepare("INSERT INTO items(domain, field, key, body) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(domain, field, key) DO UPDATE SET body=excluded.body").map_err(sql)?;
            let mut drop_item = tx.prepare("DELETE FROM items WHERE domain=?1 AND field=?2 AND key=?3").map_err(sql)?;
            // The small lists, whole: what differs from what was written, and what went.
            for (f, k, body) in &puts { put.execute((domain, f, k, body)).map_err(sql)?; }
            for (f, k) in &gone { drop_item.execute((domain, f, k)).map_err(sql)?; }
            // The big list: only what changed.
            for (k, body) in &bodies { put.execute((domain, field, k, body)).map_err(sql)?; }
            for k in delete { drop_item.execute((domain, field, k)).map_err(sql)?; }
            Ok(())
        })?;
        drop(conn);
        // Remember it as written, in place: the big list's other items are untouched.
        let after: Written = rows.items.into_iter().map(|(f, k, b)| ((f, k), b)).collect();
        let (field, delete) = (field.to_owned(), delete.to_vec());
        after_commit(dir, move || {
            let mut saved = SAVED.lock().unwrap();
            let (old_head, written) = saved.entry(key).or_insert_with(|| (String::new(), Written::new()));
            written.retain(|(f, _), _| !small.contains(f));
            written.extend(after);
            for k in delete { written.remove(&(field.clone(), k)); }
            for (k, body) in bodies { written.insert((field.clone(), k), body); }
            *old_head = head;
        });
        Ok(())
    })();
    match &result {
        Ok(()) => if UNHEALTHY.swap(false, Ordering::Relaxed) { tracing::info!("saving works again: logins are open") },
        Err(e) => if !UNHEALTHY.swap(true, Ordering::Relaxed) {
            tracing::error!("SAVE FAILED ({domain}: {e}). New logins are refused until a save succeeds; players already in keep playing. Check the disk.");
        },
    }
    result
}

/// For tests: makes saves of one `domain` (`records.json`) to `dir` fail, the others working, or work again.
#[doc(hidden)]
pub fn fail_domain(dir: &Path, domain: &str, fail: bool) -> io::Result<()> {
    let conn = hold(dir)?;
    let quoted = domain.replace('\'', "''");
    for (table, event, row, column) in [("items", "INSERT", "NEW", "domain"), ("items", "UPDATE", "NEW", "domain"), ("items", "DELETE", "OLD", "domain"),
        ("domains", "INSERT", "NEW", "name"), ("domains", "UPDATE", "NEW", "name")] {
        let name = format!("test_fail_domain_{table}_{}", event.to_lowercase());
        let statement = if fail {
            format!("CREATE TRIGGER IF NOT EXISTS {name} BEFORE {event} ON {table} WHEN {row}.{column} = '{quoted}' BEGIN SELECT RAISE(FAIL, 'test: {quoted} refused'); END;")
        } else { format!("DROP TRIGGER IF EXISTS {name};") };
        conn.execute_batch(&statement).map_err(sql)?;
    }
    Ok(())
}

/// For tests: makes every save to `dir` fail (as a full disk would), or work again.
#[doc(hidden)]
pub fn fail_saves(dir: &Path, fail: bool) -> io::Result<()> {
    let conn = hold(dir)?;
    for (table, event) in [("items", "INSERT"), ("items", "UPDATE"), ("items", "DELETE"), ("domains", "INSERT"), ("domains", "UPDATE")] {
        let name = format!("test_fail_{table}_{}", event.to_lowercase());
        let statement = if fail { format!("CREATE TRIGGER IF NOT EXISTS {name} BEFORE {event} ON {table} BEGIN SELECT RAISE(FAIL, 'test: disk full'); END;") }
            else { format!("DROP TRIGGER IF EXISTS {name};") };
        conn.execute_batch(&statement).map_err(sql)?;
    }
    Ok(())
}

/// The backups in `dir`, newest first.
pub fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = fs::read_dir(dir.join("backups")).into_iter().flatten().flatten().map(|e| e.path())
        .filter(|p|p.file_name().and_then(|n|n.to_str()).is_some_and(valid_backup_name) && p.join("server.db").is_file()).collect();
    all.sort();
    all.reverse();
    all
}

/// Puts a backup back as `dir`'s database (the server must be stopped and the folder locked by the caller): `which`
/// is a backup's folder name, or "latest". The database it replaces is kept beside it as `server.db.replaced-<time>`.
pub fn restore(dir: &Path, which: &str) -> io::Result<PathBuf> {
    let all = backups(dir);
    let chosen = if which == "latest" { all.first().cloned() } else { all.iter().find(|p| p.file_name().is_some_and(|n| n == which)).cloned() };
    let chosen = chosen.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no backup {which:?} in {}", dir.join("backups").display())))?;
    close(dir);
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    for suffix in ["", "-wal", "-shm"] {
        let current = dir.join(format!("server.db{suffix}"));
        if current.exists() { fs::rename(&current, dir.join(format!("server.db{suffix}.replaced-{stamp}")))?; }
    }
    fs::copy(chosen.join("server.db"), dir.join("server.db"))?;
    initialize(dir)?;
    Ok(chosen)
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("yo3-store-{name}-{}", std::process::id()));
        close(&d);
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }
    fn accounts(n: u32) -> Json {
        let list: Vec<Json> = (0..n).map(|i| json!({ "user_id": 1001 + i, "dpoint": 500, "players": [{ "player_id": 2001 + i, "name": format!("P{i}") }] })).collect();
        json!({ "next_user": 1001 + n, "next_player": 2001 + n, "accounts": list, "bp_codes": [{ "code": "ABC", "bp": 10 }] })
    }
    fn changes(d: &Path) -> u64 { hold(d).unwrap().total_changes() }

    #[test]
    fn saves_made_together_commit_together_and_leave_the_store_usable_whatever_happens() {
        let d = dir("together");
        initialize(&d).unwrap();
        let (records, mail) = (d.join("records.json"), d.join("mail.json"));
        let one = |n: u32| json!({ "records": [{ "player_id": 1, "opponent": 2, "win": n, "lose": 0, "draw": 0 }] });
        together(&d, || { write(&records, &one(1))?; write(&mail, &json!({ "mail": [{ "id": 1 }] })) }).unwrap();
        assert_eq!((read(&records).unwrap(), read(&mail).unwrap().is_some()), (Some(one(1)), true));
        // An error inside: nothing it saved is kept, and the caches still match the disk (the same value is written).
        let refused = together(&d, || { write(&records, &one(2))?; Err::<(), _>(io::Error::other("no")) });
        assert!(refused.is_err());
        assert_eq!(read(&records).unwrap(), Some(one(1)));
        write(&records, &one(2)).unwrap();
        assert_eq!(read(&records).unwrap(), Some(one(2)), "written again after the rollback");
        // A panic inside: rolled back, and the connection is back for the next save (no wait forever).
        let panicked = std::panic::catch_unwind(|| together(&d, || -> io::Result<()> { write(&records, &one(3)).unwrap(); panic!("inside together") }));
        assert!(panicked.is_err());
        assert_eq!(read(&records).unwrap(), Some(one(2)));
        // Another thread's save waits for the transaction, then goes through.
        std::thread::scope(|scope| {
            together(&d, || {
                let other = scope.spawn(|| write(&mail, &json!({ "mail": [] })));
                std::thread::sleep(std::time::Duration::from_millis(100));
                assert!(!other.is_finished(), "waits while the transaction is open");
                write(&records, &one(4))
            }).unwrap();
        });
        assert_eq!((read(&records).unwrap(), read(&mail).unwrap()), (Some(one(4)), Some(json!({ "mail": [] }))));
        close(&d);
    }

    #[test]
    fn a_domain_comes_back_as_it_was_saved() {
        let d = dir("roundtrip");
        initialize(&d).unwrap();
        let file = d.join("accounts.json");
        assert_eq!(read(&file).unwrap(), None, "nothing saved yet");
        let value = accounts(12);
        write(&file, &value).unwrap();
        close(&d);
        assert_eq!(read(&file).unwrap(), Some(value), "after reopening, in user_id order (1012 after 1009)");
        let friends = json!({ "friends": [[1001, 1002]], "ignored": [], "names": { "1001": "Yugi" } });
        write(&d.join("friends.json"), &friends).unwrap();
        assert_eq!(read(&d.join("friends.json")).unwrap(), Some(friends));
    }

    /// The point of rows (#22): one account's change writes that account's row, not every account.
    #[test]
    fn a_change_writes_only_the_rows_that_changed() {
        let d = dir("only-changed");
        initialize(&d).unwrap();
        let file = d.join("accounts.json");
        let mut value = accounts(300);
        write(&file, &value).unwrap();
        let before = changes(&d);
        value["accounts"][150]["dpoint"] = json!(900);
        write(&file, &value).unwrap();
        assert_eq!(changes(&d) - before, 1, "one row");
        let before = changes(&d);
        value["accounts"].as_array_mut().unwrap().remove(10);
        value["next_user"] = json!(9999);
        write(&file, &value).unwrap();
        assert_eq!(changes(&d) - before, 2, "the removed account and the counters");
        close(&d);
        assert_eq!(read(&file).unwrap(), Some(value));
    }

    #[test]
    fn a_schema_1_database_is_converted_in_place() {
        let d = dir("schema1");
        {
            let conn = Connection::open(d.join("server.db")).unwrap();
            conn.execute_batch("CREATE TABLE state_documents (name TEXT PRIMARY KEY, body TEXT NOT NULL);").unwrap();
            conn.execute("INSERT INTO state_documents VALUES ('accounts.json', ?1)", [accounts(3).to_string()]).unwrap();
            conn.execute("INSERT INTO state_documents VALUES ('mail.json', ?1)", [json!({ "last_id": 7, "waiting": [{ "id": 7, "kind": 1 }] }).to_string()]).unwrap();
            conn.pragma_update(None, "application_id", APPLICATION_ID).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
        }
        initialize(&d).unwrap();
        assert_eq!(read(&d.join("accounts.json")).unwrap(), Some(accounts(3)));
        assert_eq!(read(&d.join("mail.json")).unwrap().unwrap()["waiting"][0]["id"], 7);
        let conn = hold(&d).unwrap();
        assert_eq!(conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0)).unwrap(), SCHEMA);
        let old: u32 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE name='state_documents'", [], |r| r.get(0)).unwrap();
        assert_eq!(old, 0);
    }

    /// A failed save is said, and refuses new logins until a save works again (no silent drift).
    #[test]
    fn a_failed_save_marks_the_store_unhealthy_until_a_save_succeeds() {
        let d = dir("unhealthy");
        initialize(&d).unwrap();
        let file = d.join("records.json");
        write(&file, &json!({ "records": [] })).unwrap();
        hold(&d).unwrap().execute_batch(
            "CREATE TRIGGER refuse BEFORE INSERT ON items BEGIN SELECT RAISE(FAIL, 'disk full'); END;").unwrap();
        assert!(write(&file, &json!({ "records": [{ "player_id": 1, "opponent": 2, "win": 1 }] })).is_err());
        assert!(!healthy());
        hold(&d).unwrap().execute_batch("DROP TRIGGER refuse;").unwrap();
        write(&file, &json!({ "records": [{ "player_id": 1, "opponent": 2, "win": 1 }] })).unwrap();
        assert!(healthy());
    }

    #[test]
    fn a_backup_can_be_restored_and_the_replaced_database_is_kept() {
        let d = dir("restore");
        initialize(&d).unwrap();
        let file = d.join("teams.json");
        write(&file, &json!({ "last_id": 1, "teams": [{ "id": 1, "name": "Old" }] })).unwrap();
        back_up(&d).unwrap();
        write(&file, &json!({ "last_id": 2, "teams": [{ "id": 2, "name": "New" }] })).unwrap();
        assert_eq!(backups(&d).len(), 1);
        assert!(restore(&d, "nothing-like-this").is_err());
        restore(&d, "latest").unwrap();
        assert_eq!(read(&file).unwrap().unwrap()["teams"][0]["name"], "Old");
        assert!(fs::read_dir(&d).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("server.db.replaced-")));
    }

    #[test]
    fn duel_logs_live_in_the_database_and_only_the_newest_are_kept() {
        let d = dir("duel-logs");
        for n in 0..DUEL_LOGS_KEPT + 5 { put_duel_log(&d, &format!("{n}.dlog"), &n.to_le_bytes()).unwrap(); }
        let names = duel_log_names(&d).unwrap();
        assert_eq!(names.len(), DUEL_LOGS_KEPT as usize);
        assert_eq!(names[0], format!("{}.dlog", DUEL_LOGS_KEPT + 4), "newest first");
        assert_eq!(duel_log(&d, "0.dlog").unwrap(), None, "the oldest were dropped");
        initialize(&d).unwrap();
        assert_eq!(duel_log_names(&d).unwrap().len(), DUEL_LOGS_KEPT as usize, "a restart keeps them");
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
        assert_eq!(read_or_keep(&d.join("none.json"), "none"), Json::Null);
    }
}
