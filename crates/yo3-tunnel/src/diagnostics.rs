//! Bounded, line-redacted session logs shared by the manager, independent server and game helper.
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
pub const PART_BYTES: u64 = 2 * 1024 * 1024;
pub const RETAIN_BYTES: u64 = 100 * 1024 * 1024;
pub const RETAIN_SECS: u64 = 14 * 86400;
const LINE_BYTES: usize = 16 * 1024;
static CURRENT: std::sync::OnceLock<Session> = std::sync::OnceLock::new();
pub fn set_current(session: Session) { let _ = CURRENT.set(session); }
pub fn event(message: &str) { if let Some(session) = CURRENT.get() { session.line(message); } }
pub fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
pub fn valid_id(id: &str) -> bool { !id.is_empty() && id.len() < 100 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') }
#[derive(Clone, Serialize, Deserialize)]
pub struct Metadata { pub id: String, pub kind: String, pub pid: u32, pub parent: Option<String>, pub started: u64,
    pub release: String, pub commit: String, pub os: String, pub protocol: u16, pub mod_abi: u16 }
#[derive(Clone)]
pub struct Session(Arc<Mutex<Inner>>);
struct Inner { root: PathBuf, meta: Metadata, part: u32, bytes: u64, file: std::fs::File, secrets: Vec<String> }
pub fn private_create(path: &Path) -> io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new(); options.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    options.open(path)
}
impl Session {
    pub fn start(root: &Path, kind: &str, parent: Option<String>) -> anyhow::Result<Self> {
        anyhow::ensure!(["app", "server", "game", "direct"].contains(&kind), "Invalid diagnostic kind");
        std::fs::create_dir_all(root)?;
        anyhow::ensure!(!std::fs::symlink_metadata(root)?.file_type().is_symlink(), "Diagnostic directory must not be a symlink");
        let mut random = [0; 6]; ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut random).map_err(|_| anyhow::anyhow!("OS randomness unavailable"))?;
        let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let id = format!("{kind}-{}-{}-{nonce}", now(), std::process::id());
        let meta = Metadata { id: id.clone(), kind: kind.into(), pid: std::process::id(), parent: parent.filter(|p| valid_id(p)),
            started: now(), release: crate::BUILD_VERSION.into(), commit: crate::BUILD_COMMIT.into(), os: std::env::consts::OS.into(),
            protocol: crate::PROTOCOL_VERSION, mod_abi: crate::MOD_ABI };
        private_create(&root.join(format!("{id}.json")))?.write_all(&serde_json::to_vec_pretty(&meta)?)?;
        let file = private_create(&root.join(format!("{id}.0000.log")))?;
        let result = Self(Arc::new(Mutex::new(Inner { root: root.into(), meta, file, part: 0, bytes: 0, secrets: vec![] })));
        // Windows helpers under Wine have Windows PIDs; a native Linux manager cannot inspect those via /proc.
        // A tiny heartbeat protects their current file across that boundary without trusting a foreign PID.
        let lease_path = root.join(format!("{id}.live")); let mut lease = private_create(&lease_path)?;
        lease.write_all(&now().to_le_bytes())?;
        let weak = Arc::downgrade(&result.0);
        std::thread::Builder::new().name("diagnostic-heartbeat".into()).spawn(move || {
            use std::io::{Seek, SeekFrom};
            loop {
                std::thread::sleep(std::time::Duration::from_secs(10));
                if weak.strong_count() == 0 { drop(lease); let _ = std::fs::remove_file(lease_path); break }
                let _ = lease.seek(SeekFrom::Start(0)); let _ = lease.write_all(&now().to_le_bytes()); let _ = lease.flush();
            }
        })?;
        if kind == "game" || kind == "direct" {
            let temporary = root.join(format!("{id}.latest"));
            private_create(&temporary)?.write_all(id.as_bytes())?;
            let _ = std::fs::rename(&temporary, root.join("latest-game"));
        }
        result.line("Session started"); prune(root, now(), RETAIN_BYTES, RETAIN_SECS)?; Ok(result)
    }
    pub fn id(&self) -> String { self.0.lock().unwrap().meta.id.clone() }
    pub fn secret(&self, value: &str) { if !value.is_empty() { self.0.lock().unwrap().secrets.push(value.into()); } }
    pub fn writer(&self) -> LineWriter { LineWriter { session: self.clone(), line: Vec::new(), dropped: false, private_key: false } }
    pub fn line(&self, line: &str) {
        let mut inner = self.0.lock().unwrap();
        let clean = redact(line, &inner.secrets, None); let text = format!("{} {}\n", now(), clean);
        if inner.bytes + text.len() as u64 > PART_BYTES {
            inner.part += 1;
            if let Ok(file) = private_create(&inner.root.join(format!("{}.{:04}.log", inner.meta.id, inner.part))) {
                inner.file = file; inner.bytes = 0;
                let _ = prune(&inner.root, now(), RETAIN_BYTES, RETAIN_SECS);
            }
        }
        if text.len() <= LINE_BYTES && inner.file.write_all(text.as_bytes()).is_ok() { inner.bytes += text.len() as u64; }
    }
}
pub struct LineWriter { session: Session, line: Vec<u8>, dropped: bool, private_key: bool }
pub struct ConsoleWriter { logger: Option<LineWriter> }
impl ConsoleWriter { pub fn new(session: Option<Session>) -> Self { Self { logger: session.map(|s| s.writer()) } } }
impl Write for ConsoleWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(log) = &mut self.logger { log.write_all(bytes)?; }
        std::io::stdout().write_all(bytes)?; Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { if let Some(log) = &mut self.logger { log.flush()?; } std::io::stdout().flush() }
}
impl LineWriter {
    fn finish_line(&mut self) {
        if self.dropped { self.session.line("[overlong diagnostic line omitted]"); }
        else {
            let text = String::from_utf8_lossy(&self.line);
            if text.contains("-----BEGIN") && text.contains("PRIVATE KEY-----") { self.private_key = true; }
            if self.private_key { if text.contains("-----END") && text.contains("PRIVATE KEY-----") { self.private_key = false; } self.session.line("[private key omitted]"); }
            else { self.session.line(&text); }
        }
        self.line.clear(); self.dropped = false;
    }
}
impl Write for LineWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for b in bytes { if *b == b'\n' { self.finish_line(); } else if !self.dropped {
            if self.line.len() == LINE_BYTES { self.line.clear(); self.dropped = true; } else { self.line.push(*b); }
        } }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { self.session.0.lock().unwrap().file.flush() }
}
impl Drop for LineWriter { fn drop(&mut self) { if !self.line.is_empty() || self.dropped { self.finish_line(); } } }
/// Conservative redaction at both capture and export. Named-secret lines are omitted in full.
pub fn redact(text: &str, secrets: &[String], home: Option<&str>) -> String {
    let lower = text.to_ascii_lowercase();
    if ["password", "authorization", "private key", "\"key\"", "key=", "key:", "cookie", "token", "session/"].iter().any(|needle| lower.contains(needle)) {
        return "[sensitive diagnostic line omitted]".into()
    }
    let mut result = text.to_string();
    for secret in secrets { if !secret.is_empty() { result = result.replace(secret, "[redacted]"); } }
    // Account credentials are 20 ASCII letters/digits; session tickets/cookies are 64 hex characters.
    let mut out = String::new(); let mut token = String::new();
    let append = |token: &mut String, out: &mut String| {
        let opaque = token.len() >= 20;
        if opaque { out.push_str("[redacted]"); } else { out.push_str(token); } token.clear();
    };
    for c in result.chars() { if c.is_ascii_alphanumeric() { token.push(c); } else { append(&mut token, &mut out); out.push(c); } }
    append(&mut token, &mut out);
    if let Some(home) = home.filter(|s| !s.is_empty()) { out = out.replace(home, "<user>"); }
    out
}
#[derive(Clone, Serialize)]
pub struct Record { pub name: String, pub id: String, pub bytes: u64, pub modified: u64 }
/// Only our flat, bounded file names. No recursive traversal and no symbolic links, profiles, keys or databases.
pub fn records(root: &Path) -> io::Result<Vec<Record>> {
    let mut out = vec![];
    if !root.exists() { return Ok(out) }
    if std::fs::symlink_metadata(root)?.file_type().is_symlink() { return Err(io::Error::other("Diagnostic directory is a symlink")) }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?; let name = entry.file_name().to_string_lossy().into_owned();
        let Some((id, suffix)) = name.split_once('.') else { continue };
        if !valid_id(id) || !["app-", "server-", "game-", "direct-"].iter().any(|p| id.starts_with(p)) { continue }
        let log = suffix.strip_suffix(".log").is_some_and(|p| p.len() >= 4 && p.bytes().all(|b| b.is_ascii_digit()));
        if suffix != "json" && !log { continue }
        let meta = std::fs::symlink_metadata(entry.path())?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > PART_BYTES + LINE_BYTES as u64 { continue }
        out.push(Record { id: id.into(), name, bytes: meta.len(), modified: meta.modified()?.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() });
    }
    out.sort_by_key(|r| (r.modified, r.name.clone())); Ok(out)
}
fn alive(pid: u32) -> bool {
    #[cfg(not(windows))] { Path::new("/proc").join(pid.to_string()).exists() }
    #[cfg(windows)] unsafe {
        use windows_sys::Win32::{System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE}, Foundation::CloseHandle};
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid); if handle.is_null() { return false }
        let active = WaitForSingleObject(handle, 0) == 258; CloseHandle(handle); active
    }
}
pub fn prune(root: &Path, time: u64, max_bytes: u64, max_age: u64) -> io::Result<()> {
    let all = records(root)?; let mut protected = std::collections::HashSet::new();
    for record in all.iter().filter(|r| r.name.ends_with(".json")) {
        if let Ok(meta) = std::fs::read(root.join(&record.name)).and_then(|b| serde_json::from_slice::<Metadata>(&b).map_err(io::Error::other))
            && meta.id == record.id && if meta.os == std::env::consts::OS { alive(meta.pid) } else {
                let path = root.join(format!("{}.live", meta.id));
                std::fs::symlink_metadata(path).ok().filter(|m| m.is_file() && !m.file_type().is_symlink() && m.len() == 8)
                    .and_then(|m| m.modified().ok()).is_some_and(|t| time.saturating_sub(t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()) < 90)
            } {
                protected.insert(record.name.clone());
                if let Some(last) = all.iter().filter(|r| r.id == meta.id && r.name.ends_with(".log")).max_by(|a,b| a.name.cmp(&b.name)) { protected.insert(last.name.clone()); }
            }
    }
    let mut bytes: u64 = all.iter().map(|r| r.bytes).sum();
    for r in all { if !protected.contains(&r.name) && (time.saturating_sub(r.modified) > max_age || bytes > max_bytes)
        && std::fs::remove_file(root.join(&r.name)).is_ok() {
            if r.name.ends_with(".json") { let _ = std::fs::remove_file(root.join(format!("{}.live", r.id))); }
            bytes = bytes.saturating_sub(r.bytes);
        } }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wine_helpers_keep_their_live_log_when_the_native_manager_prunes() {
        let p = std::env::temp_dir().join(format!("yo3-foreign-lease-{}", std::process::id())); let _ = std::fs::remove_dir_all(&p);
        let s = Session::start(&p, "direct", None).unwrap(); let id = s.id();
        let file = p.join(format!("{id}.json")); let mut meta: Metadata = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        meta.os = "other-platform".into(); meta.pid = 0; std::fs::write(&file, serde_json::to_vec(&meta).unwrap()).unwrap();
        prune(&p, now(), 0, 0).unwrap();
        assert!(file.exists() && p.join(format!("{id}.0000.log")).exists(), "a Wine PID is not a native PID; protect it using its heartbeat");
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn retention_keeps_active_file_but_removes_old_parts_and_ignores_foreign_files() {
        let p = std::env::temp_dir().join(format!("yo3-retention-{}", std::process::id())); let _ = std::fs::remove_dir_all(&p);
        let s = Session::start(&p, "server", None).unwrap(); let id = s.id();
        std::fs::write(p.join(format!("{id}.0001.log")), b"old part").unwrap();
        std::fs::write(p.join("account.json"), b"never touch").unwrap();
        prune(&p, now()+RETAIN_SECS+1, 1, RETAIN_SECS).unwrap();
        assert!(p.join(format!("{id}.json")).exists()); assert!(p.join(format!("{id}.0001.log")).exists());
        assert!(!p.join(format!("{id}.0000.log")).exists()); assert!(p.join("account.json").exists());
        let _ = std::fs::remove_dir_all(p);
    }
}
