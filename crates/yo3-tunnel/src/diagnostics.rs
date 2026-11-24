//! Bounded session logs: credentials scrubbed at capture, optional personal-data filtering at export.
//!
//! Every line is one JSON object in the yo3-log/1 schema (docs/logs.md): `ts` (UTC, milliseconds), `level`, `src`,
//! `session`, `msg`, then the event's own fields. Captured output (a game's, Wine's, the DLL's, a server's text) is
//! classified into `src` and `level` as it is written.
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
/// The log schema every session log line follows (docs/logs.md).
pub const LOG_FORMAT: &str = "yo3-log/1";
/// `t` as yo3-log/1's `ts`: UTC, RFC 3339 with milliseconds (`2026-11-24T08:12:37.123Z`).
pub fn timestamp(t: SystemTime) -> String {
    let d = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let (secs, ms) = (d.as_secs() as i64, d.subsec_millis());
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468; let era = z.div_euclid(146_097); let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153; let day = doy - (153 * mp + 2) / 5 + 1; let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}
/// The `src` of a session's own lines: the Desktop window, the helper that runs a game, or the server.
fn own_src(kind: &str) -> &'static str { match kind { "app" => "app", "server" => "server", _ => "helper" } }
/// A captured line's `src`, `level`, own time (a server's text carries one) and message, from its markers: the
/// connection DLL's `[YO3 MOD]`, the game's `[YO3 CLIENT]` trace, `[YO3 PACKET]`/`[YO3 ENGINE]`, Wine's
/// `0123:fixme:...`, a server's `<ISO time>  LEVEL message`; else `default` and the level word in it.
pub fn classify(text: &str, default: &str) -> (String, &'static str, Option<String>, String) {
    let word = |l: Option<Level>| match l { Some(Level::Error) => "error", Some(Level::Warn) => "warn", Some(Level::Debug) => "debug", _ => "info" };
    for (marker, src) in [("[YO3 MOD] ", "dll"), ("[YO3 CLIENT] ", "client"), ("[YO3 PACKET] ", "packet"), ("[YO3 ENGINE] ", "engine")] {
        if let Some(rest) = text.strip_prefix(marker) { return (src.into(), if src == "dll" { word(level(rest)) } else { "debug" }, None, rest.into()) }
    }
    let b = text.as_bytes();
    if b.len() > 6 && b[..4].iter().all(u8::is_ascii_hexdigit) && b[4] == b':' {
        let rest = &text[5..];
        let level = if rest.starts_with("err:") { "error" } else if rest.starts_with("warn:") { "warn" } else { "debug" };
        return ("wine".into(), level, None, text.into());
    }
    // A server's text line: `2026-11-21T20:17:12.967137Z  INFO message`.
    if b.len() > 28 && b[4] == b'-' && b[10] == b'T' && let Some(z) = text[..32.min(text.len())].find('Z') {
        let mut words = text[z + 1..].trim_start().splitn(2, ' ');
        let lvl = match words.next() { Some("ERROR") => Some("error"), Some("WARN") => Some("warn"), Some("INFO") => Some("info"), Some("DEBUG") => Some("debug"), Some("TRACE") => Some("trace"), _ => None };
        if let Some(lvl) = lvl {
            let ts = format!("{}Z", &text[..23.min(z)]);
            return ("server".into(), lvl, Some(ts), words.next().unwrap_or_default().trim_start().into());
        }
    }
    (default.into(), word(level(text)), None, text.into())
}
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
pub fn private_dir(path: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(path)?;
    anyhow::ensure!(!std::fs::symlink_metadata(path)?.file_type().is_symlink(), "Private application directory must not be a symlink");
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::File::open(path)?.set_permissions(std::fs::Permissions::from_mode(0o700))?; }
    Ok(())
}
impl Session {
    pub fn start(root: &Path, kind: &str, parent: Option<String>) -> anyhow::Result<Self> {
        anyhow::ensure!(["app", "server", "game", "direct"].contains(&kind), "Invalid diagnostic kind");
        private_dir(root)?;
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
    /// The newest `max` bytes of this session's log, across its parts, from a line's start.
    pub fn tail(&self, max: usize) -> String {
        let (root, id) = { let inner = self.0.lock().unwrap(); (inner.root.clone(), inner.meta.id.clone()) };
        tail_of(&root, &id, max)
    }
    pub fn secret(&self, value: &str) { if !value.is_empty() { self.0.lock().unwrap().secrets.push(value.into()); } }
    pub fn writer(&self) -> LineWriter { self.prefixed_writer("") }
    pub fn client_writer(&self) -> LineWriter { self.prefixed_writer("[YO3 CLIENT] ") }
    fn prefixed_writer(&self, prefix: &'static str) -> LineWriter { LineWriter { session: self.clone(), line: Vec::new(), dropped: false, private_key: false, prefix } }
    /// A line of this session: its own (an information line), or captured output, classified ([`classify`]).
    pub fn line(&self, line: &str) {
        if line.starts_with('{') && let Ok(serde_json::Value::Object(event)) = serde_json::from_str(line) { return self.event(event) }
        let default = own_src(&self.0.lock().unwrap().meta.kind);
        let (src, level, ts, msg) = classify(line, default);
        self.write(level, &src, ts, &msg, serde_json::Map::new());
    }
    /// A warning of this session's own.
    pub fn warn(&self, line: &str) { let src = own_src(&self.0.lock().unwrap().meta.kind); self.write("warn", src, None, line, serde_json::Map::new()) }
    /// An error of this session's own.
    pub fn error(&self, line: &str) { let src = own_src(&self.0.lock().unwrap().meta.kind); self.write("error", src, None, line, serde_json::Map::new()) }
    /// A structured event (a server's, from its log layer): `ts`, `level`, `src` and `msg` from it when present, its
    /// other keys as fields.
    pub fn event(&self, mut event: serde_json::Map<String, serde_json::Value>) {
        let take = |e: &mut serde_json::Map<String, serde_json::Value>, k: &str| e.remove(k).and_then(|v| v.as_str().map(str::to_owned));
        let level = match take(&mut event, "level").as_deref().map(str::to_ascii_lowercase).as_deref() {
            Some("error") => "error", Some("warn") => "warn", Some("debug") => "debug", Some("trace") => "trace", _ => "info" };
        let src = take(&mut event, "src").unwrap_or_else(|| own_src(&self.0.lock().unwrap().meta.kind).into());
        let (ts, msg) = (take(&mut event, "ts"), take(&mut event, "msg").unwrap_or_default());
        event.remove("session");
        self.write(level, &src, ts, &msg, event);
    }
    fn write(&self, level: &str, src: &str, ts: Option<String>, msg: &str, mut fields: serde_json::Map<String, serde_json::Value>) {
        let mut inner = self.0.lock().unwrap();
        let clean = redact_secrets(msg, &inner.secrets);
        let msg = if clean.len() > LINE_BYTES - 512 { "[overlong diagnostic line omitted]".to_owned() } else { clean };
        for value in fields.values_mut() { if let serde_json::Value::String(text) = value { *text = redact_secrets(text, &inner.secrets) } }
        // ts, level, src, session and msg first, in that order, for a person reading the file; then the fields.
        let quote = |v: &str| serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into());
        let mut json = format!("{{\"ts\":{},\"level\":{},\"src\":{},\"session\":{},\"msg\":{}",
            quote(&ts.unwrap_or_else(|| timestamp(SystemTime::now()))), quote(level), quote(src), quote(&inner.meta.id), quote(&msg));
        for (key, value) in fields {
            if ["ts", "level", "src", "session", "msg"].contains(&key.as_str()) { continue }
            if let Ok(value) = serde_json::to_string(&value) { json.push_str(&format!(",{}:{value}", quote(&key))); }
        }
        json.push('}');
        let text = format!("{json}\n");
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
/// The newest `max` bytes of session `id`'s log in `root`, across its parts, from a line's start.
pub fn tail_of(root: &Path, id: &str, max: usize) -> String {
    if !valid_id(id) { return String::new() }
    let mut parts: Vec<(u32, PathBuf)> = std::fs::read_dir(root).into_iter().flatten().flatten().filter_map(|e| {
        let name = e.file_name().into_string().ok()?;
        let part = name.strip_prefix(id)?.strip_prefix('.')?.strip_suffix(".log")?.parse().ok()?;
        Some((part, e.path()))
    }).collect();
    parts.sort_by_key(|p| std::cmp::Reverse(p.0));
    let mut kept: Vec<u8> = Vec::new();
    for (_, path) in parts {
        let Ok(bytes) = std::fs::read(path) else { continue };
        kept.splice(0..0, bytes);
        if kept.len() >= max { break }
    }
    let start = kept.len().saturating_sub(max);
    let start = if start == 0 { 0 } else { kept[start..].iter().position(|b| *b == b'\n').map_or(kept.len(), |p| start + p + 1) };
    String::from_utf8_lossy(&kept[start..]).into_owned()
}

pub struct LineWriter { session: Session, line: Vec<u8>, dropped: bool, private_key: bool, prefix: &'static str }
pub struct ConsoleWriter { logger: Option<LineWriter>, color: bool }
impl ConsoleWriter {
    pub fn new(session: Option<Session>) -> Self { Self { logger: session.map(|s| s.writer()), color: false } }
    /// Colours the terminal copy by level (the session log stays plain text): see [`console_color`].
    pub fn colored(mut self, color: bool) -> Self { self.color = color; self }
}
impl Write for ConsoleWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(log) = &mut self.logger { log.write_all(bytes)?; }
        if self.color { std::io::stdout().write_all(colorize(&String::from_utf8_lossy(bytes)).as_bytes())?; }
        else { std::io::stdout().write_all(bytes)?; }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { if let Some(log) = &mut self.logger { log.flush()?; } std::io::stdout().flush() }
}
impl LineWriter {
    fn finish_line(&mut self) {
        if self.dropped { self.session.line(&format!("{}[overlong diagnostic line omitted]", self.prefix)); }
        else {
            let text = String::from_utf8_lossy(&self.line);
            if text.contains("-----BEGIN") && text.contains("PRIVATE KEY-----") { self.private_key = true; }
            if self.private_key { if text.contains("-----END") && text.contains("PRIVATE KEY-----") { self.private_key = false; } self.session.line(&format!("{}[private key omitted]", self.prefix)); }
            else { self.session.line(&format!("{}{text}", self.prefix)); }
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
/// Credentials are never optional, including in locally retained and full diagnostic reports.
pub fn redact_secrets(text: &str, secrets: &[String]) -> String {
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
    out
}
/// Extra personal-path filtering for error displays and privacy-filtered exports.
pub fn redact(text: &str, secrets: &[String], home: Option<&str>) -> String {
    let mut out = redact_secrets(text, secrets);
    if let Some(home) = home.filter(|s| !s.is_empty()) { out = out.replace(home, "<user>"); }
    // A Linux report can contain Windows/Wine paths, and vice versa; mask both without relying on this process's OS.
    for root in ["\\\\users\\\\", "\\\\home\\\\", "/home/", "/users/", "\\users\\", "\\home\\"] {
        let mut from = 0;
        while let Some(offset) = out[from..].to_ascii_lowercase().find(root) {
            let start = from + offset + root.len();
            let end = out[start..].find(['/', '\\', '"', '\'', '\r', '\n']).map(|n| start+n).unwrap_or(out.len());
            if start == end { from = start; continue }
            out.replace_range(start..end, "<user>"); from = start + "<user>".len();
        }
    }
    out
}
/// Verbose payloads stay on disk, while the ordinary activity view remains readable.
pub fn is_trace(line: &str) -> bool {
    ["[YO3 CLIENT]", "[YO3 PACKET]", "[YO3 ENGINE]", r#""src":"client""#, r#""src":"packet""#, r#""src":"engine""#].iter().any(|marker| line.contains(marker))
}

/// A yo3-log/1 line as a person reads it: `08:12:37.123 INFO  server  message  key=value...`; any other line as it is.
pub fn human(line: &str) -> String {
    let Ok(serde_json::Value::Object(mut e)) = serde_json::from_str::<serde_json::Value>(line) else { return line.to_owned() };
    let mut take = |k: &str| e.remove(k).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned)).unwrap_or_default();
    let (ts, level, src, msg) = (take("ts"), take("level"), take("src"), take("msg"));
    e.remove("session");
    let time = ts.get(11..23).unwrap_or(&ts).to_owned();
    let mut out = format!("{time} {:<5} {src:<6} {msg}", level.to_ascii_uppercase());
    for (k, v) in e { out.push_str(&format!("  {k}={}", v.as_str().map_or_else(|| v.to_string(), str::to_owned))); }
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
    #[test]
    fn every_line_is_one_yo3_log_object_with_its_source_and_level() {
        // docs/logs.md: ts (UTC, ms), level, src, session, msg, then fields; captured lines classified by their markers.
        assert_eq!(timestamp(UNIX_EPOCH + std::time::Duration::from_millis(1_795_504_357_123)), "2026-11-24T07:12:37.123Z");
        assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        assert_eq!(classify("[YO3 MOD] Game network: EasyMultiPlay recv failed", "helper").0, "dll");
        assert_eq!(classify("[YO3 CLIENT] recv 0x2398", "helper").0, "client");
        let wine = classify("02b8:err:module:import_dll Library X not found", "helper");
        assert_eq!((wine.0.as_str(), wine.1), ("wine", "error"));
        let server = classify("2026-11-21T20:17:12.967137Z  WARN [tls] 127.0.0.1:5 handshake failed", "app");
        assert_eq!((server.0.as_str(), server.1, server.2.as_deref(), server.3.as_str()), ("server", "warn", Some("2026-11-21T20:17:12.967Z"), "[tls] 127.0.0.1:5 handshake failed"));
        assert_eq!(classify("Starting the game", "helper"), ("helper".into(), "info", None, "Starting the game".into()));
        let root = std::env::temp_dir().join(format!("yo3-log1-{}-{}", std::process::id(), now()));
        let session = Session::start(&root, "game", None).unwrap();
        session.line("[YO3 MOD] v0.1.22 interface ready");
        session.warn("The relay lost the front connection");
        let mut fields = serde_json::Map::new();
        fields.insert("user".into(), 1001.into());
        fields.insert("level".into(), "WARN".into());
        fields.insert("msg".into(), "a seat left".into());
        fields.insert("src".into(), "server".into());
        session.event(fields);
        session.line("password=hunter2 should never be kept");
        let lines: Vec<serde_json::Value> = session.tail(1 << 20).lines().map(|l| serde_json::from_str(l).expect("JSON")).collect();
        let id = session.id();
        assert!(lines.iter().all(|l| l["session"] == id.as_str() && l["ts"].as_str().is_some_and(|t| t.ends_with('Z') && t.len() == 24)), "{lines:?}");
        assert_eq!((lines[0]["src"].as_str(), lines[0]["msg"].as_str()), (Some("helper"), Some("Session started")));
        assert_eq!((lines[1]["src"].as_str(), lines[1]["msg"].as_str()), (Some("dll"), Some("v0.1.22 interface ready")));
        assert_eq!(lines[2]["level"], "warn");
        assert_eq!((lines[3]["src"].as_str(), lines[3]["level"].as_str(), lines[3]["user"].as_u64()), (Some("server"), Some("warn"), Some(1001)));
        assert_eq!(lines[4]["msg"], "[sensitive diagnostic line omitted]");
        assert!(human(&serde_json::to_string(&lines[3]).unwrap()).ends_with("WARN  server a seat left  user=1001"), "{}", human(&serde_json::to_string(&lines[3]).unwrap()));
        assert_eq!(human("1795292305 an old line"), "1795292305 an old line");
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_sessions_tail_is_its_newest_lines_across_parts() {
        let p = std::env::temp_dir().join(format!("yo3-tail-{}", std::process::id())); let _ = std::fs::remove_dir_all(&p);
        let s = Session::start(&p, "game", None).unwrap();
        for i in 0..40_000 { s.line(&format!("line {i:05} {}", "x-".repeat(30))); }
        let tail = s.tail(100_000);
        assert!(tail.len() <= 100_000 && tail.len() > 90_000, "{}", tail.len());
        let msg = |line: &str| serde_json::from_str::<serde_json::Value>(line).expect("a whole yo3-log line")["msg"].as_str().unwrap().to_owned();
        assert_eq!(msg(tail.lines().last().unwrap()), format!("line 39999 {}", "x-".repeat(30)));
        assert!(msg(tail.lines().next().unwrap()).starts_with("line "), "it starts at a line's start");
        assert!(s.tail(10_000_000).contains("Session started"), "all of it when it fits, the first part too");
        let _ = std::fs::remove_dir_all(p);
    }

    #[test]
    fn log_levels_are_found_and_only_the_terminal_copy_is_coloured() {
        use super::{Level, colorize, level};
        assert_eq!(level("2026-11-12T13:02:07.593510Z  WARN accounts.json is an old copy"), Some(Level::Warn));
        assert_eq!(level("14:25:39 ERROR duel check"), Some(Level::Error));
        assert_eq!(level("14:25:39 INFO rules engine: engine"), Some(Level::Info));
        assert_eq!(level("Game/helper session ended"), None);
        assert_eq!(level("14:25:39 INFO a player named WARN"), Some(Level::Info), "the level word comes first");
        let shown = colorize("t  WARN careful\nt  INFO fine\nplain\n");
        assert_eq!(shown, "\x1b[33mt  WARN careful\x1b[0m\nt  \x1b[32mINFO\x1b[0m fine\nplain\n");
    }

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

/// A log line's severity, from its level word (`<time>  WARN message`, or Desktop's `<hh:mm:ss> WARN message`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level { Error, Warn, Info, Debug }
pub fn level(line: &str) -> Option<Level> {
    line.split_whitespace().take(4).find_map(|word| match word {
        "ERROR" => Some(Level::Error), "WARN" => Some(Level::Warn), "INFO" => Some(Level::Info),
        "DEBUG" | "TRACE" => Some(Level::Debug), _ => None,
    })
}

/// Terminal colours by level: errors red and warnings yellow (the whole line, so they stand out), the level word of
/// information green and of debugging lines blue.
pub fn colorize(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    for line in text.split_inclusive('\n') {
        let (body, end) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        match level(body) {
            Some(Level::Error) => out.push_str(&format!("\x1b[1;31m{body}\x1b[0m")),
            Some(Level::Warn) => out.push_str(&format!("\x1b[33m{body}\x1b[0m")),
            Some(found @ (Level::Info | Level::Debug)) => {
                let word = if found == Level::Info { "INFO" } else if body.contains("DEBUG") { "DEBUG" } else { "TRACE" };
                let code = if found == Level::Info { "32" } else { "34" };
                out.push_str(&body.replacen(word, &format!("\x1b[{code}m{word}\x1b[0m"), 1));
            }
            None => out.push_str(body),
        }
        out.push_str(end);
    }
    out
}

/// Whether to colour console output: stdout is a terminal, `NO_COLOR` is unset, and (on Windows) the console accepts
/// colour sequences, which it does once virtual-terminal processing is switched on here.
pub fn console_color() -> bool {
    use std::io::IsTerminal;
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) || !std::io::stdout().is_terminal() { return false }
    #[cfg(windows)] {
        use windows_sys::Win32::System::Console::{ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleMode};
        // SAFETY: plain console API calls on this process's own standard output handle.
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE); let mut mode = 0;
            if GetConsoleMode(handle, &mut mode) == 0 { return false }
            return mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0 || SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
        }
    }
    #[allow(unreachable_code, reason = "Windows returns from its own block above")]
    true
}
