//! Packet log: one JSON line per packet, plus a readable summary through `tracing`.
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::lock::Locked;
use serde_json::{Value as Json, json};
use yo3_proto::{Message, Packet, Value};

/// The most bytes of a packet written to the log in full (hex), and the most text, fields and items shown of one
/// message: the log is for reading, and a client that sends megabytes would otherwise fill the disk and hold the lock.
const RAW_MAX: usize = 2048;
const TEXT_MAX: usize = 256;
const FIELDS_MAX: usize = 64;
const ITEMS_MAX: usize = 32;

/// A log file is closed and the next one started at this size, and only the newest few of a run's files are kept:
/// without it a client that only sends (monkey testing wrote 2.6 GB in five minutes) fills the disk.
const FILE_MAX: u64 = 128 << 20;
const FILES_KEPT: usize = 4;

struct Current {
    file: File,
    path: PathBuf,
    bytes: u64,
    /// The files before this one, oldest first.
    older: VecDeque<PathBuf>,
    parts: u32,
}

pub struct PacketLog {
    enabled: bool,
    /// Whether decoded traces go to the session log ([`diagnostic`]).
    traces: bool,
    dir: PathBuf,
    name: String,
    max_bytes: u64,
    keep: usize,
    cur: Mutex<Current>,
}

/// Session exports need nested fields too. Keep these bounded fragments separate from optional raw developer files.
fn diagnostic(who: &str, direction: &str, summary: &str, message: &Json, error: Option<&str>) {
    if !tracing::enabled!(tracing::Level::INFO) { return }
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let text = yo3_tunnel::diagnostics::redact_secrets(&json!({"server":who,"dir":direction,"summary":summary,"msg":message,"error":error}).to_string(), &[]);
    let mut rest = text.as_str(); let mut part = 0;
    while !rest.is_empty() {
        let mut end = rest.len().min(8000); while !rest.is_char_boundary(end) { end -= 1; }
        let chunk = &rest[..end]; let last = end == rest.len();
        tracing::info!("[YO3 PACKET] packet={id} part={part} last={last} {chunk}");
        rest = &rest[end..]; part += 1;
    }
}

impl PacketLog {
    pub fn create(dir: &Path) -> std::io::Result<PacketLog> {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        std::fs::create_dir_all(dir)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let name = format!("packets-{stamp}-{}-{seq}", std::process::id());
        let path = dir.join(format!("{name}.jsonl"));
        let cur = Current { file: File::create(&path)?, path, bytes: 0, older: VecDeque::new(), parts: 0 };
        Ok(PacketLog { enabled: true, traces: true, dir: dir.to_path_buf(), name, max_bytes: FILE_MAX, keep: FILES_KEPT, cur: Mutex::new(cur) })
    }
    pub fn disabled() -> std::io::Result<PacketLog> {
        let path = PathBuf::from(if cfg!(windows) { "NUL" } else { "/dev/null" });
        let file = std::fs::OpenOptions::new().write(true).open(&path)?;
        let cur = Current { file, path: PathBuf::from("disabled"), bytes: 0, older: VecDeque::new(), parts: 0 };
        Ok(PacketLog { enabled: false, traces: true, dir: PathBuf::new(), name: String::new(), max_bytes: FILE_MAX, keep: 1, cur: Mutex::new(cur) })
    }

    /// With `traces` false, no packet reaches the session log either (a hosted server's privacy, #37).
    pub fn with_traces(mut self, traces: bool) -> PacketLog {
        self.traces = traces;
        self
    }

    /// Other limits (a file's size in bytes, how many files are kept); for tests.
    #[cfg(test)]
    pub fn with_limits(mut self, max_bytes: u64, keep: usize) -> PacketLog {
        self.max_bytes = max_bytes;
        self.keep = keep.max(1);
        self
    }

    /// The file being written.
    pub fn path(&self) -> PathBuf {
        self.cur.locked().path.clone()
    }

    pub fn incoming(&self, who: &str, plain: &[u8], packet: Option<&Packet>, error: Option<&str>) {
        let (summary, msg) = match packet {
            Some(Packet::Version(v)) => (format!("<- version {v:?}"), json!({ "version": v })),
            Some(Packet::Message(m)) => (format!("<- {}", summarize(m)), message_json(m)),
            None => (format!("<- undecodable: {}", error.unwrap_or("?")), Json::Null),
        };
        if self.traces { diagnostic(who, "<-", &summary, &msg, error); }
        if self.enabled { self.write(who, "<-", plain, msg, error); }
    }

    pub fn outgoing(&self, who: &str, plain: &[u8], msg: &Message) {
        let data = message_json(msg);
        if self.traces { diagnostic(who, "->", &summarize(msg), &data, None); }
        if self.enabled { self.write(who, "->", plain, data, None); }
    }

    fn rotate(&self, cur: &mut Current) {
        cur.parts += 1;
        let path = self.dir.join(format!("{}-{}.jsonl", self.name, cur.parts));
        let Ok(file) = File::create(&path) else { return };
        let old = std::mem::replace(&mut cur.path, path);
        cur.file = file;
        cur.bytes = 0;
        cur.older.push_back(old);
        while cur.older.len() + 1 > self.keep {
            if let Some(gone) = cur.older.pop_front() {
                let _ = std::fs::remove_file(gone);
            }
        }
    }

    /// One packet as a yo3-log/1 line (docs/logs.md): `src` packet, the connection (`conn`), its direction, the request
    /// id in hex (`req`), the message with its values' own types (`packet`), and the frame's bytes (`raw`).
    fn write(&self, who: &str, dir: &str, plain: &[u8], packet: Json, error: Option<&str>) {
        let req = packet["fields"]["request"].as_u64().map(|r| format!("{r:#06x}"));
        let summary = match (&req, packet["version"].as_str()) {
            (_, Some(version)) => format!("{dir} version {version}"),
            (Some(req), None) => format!("{dir} {req}"),
            (None, None) => format!("{dir} {}", error.map_or("(no request id)".to_owned(), |e| format!("undecodable: {e}"))),
        };
        let mut line = serde_json::Map::new();
        line.insert("ts".into(), yo3_tunnel::diagnostics::timestamp(std::time::SystemTime::now()).into());
        line.insert("level".into(), if error.is_some() { "warn" } else { "debug" }.into());
        line.insert("src".into(), "packet".into());
        line.insert("session".into(), yo3_tunnel::diagnostics::current_id().unwrap_or_default().into());
        line.insert("msg".into(), summary.into());
        line.insert("conn".into(), who.into());
        line.insert("dir".into(), dir.into());
        if let Some(req) = req { line.insert("req".into(), req.into()); }
        line.insert("packet".into(), packet);
        line.insert("raw".into(), plain.iter().take(RAW_MAX).map(|b| format!("{b:02x}")).collect::<String>().into());
        line.insert("raw_len".into(), plain.len().into());
        if let Some(error) = error { line.insert("error".into(), error.into()); }
        let text = format!("{}\n", yo3_tunnel::diagnostics::log_line(&line));
        let mut cur = self.cur.locked();
        if cur.bytes > 0 && cur.bytes + text.len() as u64 > self.max_bytes {
            self.rotate(&mut cur);
        }
        if cur.file.write_all(text.as_bytes()).is_ok() {
            cur.bytes += text.len() as u64;
        }
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::U32(n) | Value::U32Alt(n) => n.to_string(),
        Value::Text(_) | Value::TextAlt(_) => {
            let text = v.as_str().unwrap_or_default();
            match text.char_indices().nth(TEXT_MAX) {
                Some((end, _)) => format!("{:?}…({} bytes)", &text[..end], text.len()),
                None => format!("{text:?}"),
            }
        }
        Value::Bin(b) => {
            let hex: String = b.iter().take(24).map(|x| format!("{x:02x}")).collect();
            format!("bin[{}]:{hex}{}", b.len(), if b.len() > 24 { "…" } else { "" })
        }
    }
}

pub fn summarize(m: &Message) -> String {
    let id = m.request().map(|r| format!("{r:#06x}")).unwrap_or_else(|| "(no id)".into());
    let fields: Vec<String> =
        m.fields.iter().filter(|(k, _)| k != "request").take(FIELDS_MAX).map(|(k, v)| format!("{k}={}", show(v))).collect();
    let items = if m.items.is_empty() { String::new() } else { format!(" +{} item(s)", m.items.len()) };
    format!("{id} {{{}}}{items}", fields.join(", "))
}

/// A message for the packet log: its fields with their own types (numbers, text without quotes around it, binary as
/// `bin:<hex>`), its items the same way.
fn message_json(m: &Message) -> Json {
    let fields: serde_json::Map<String, Json> = m
        .fields
        .iter()
        .take(FIELDS_MAX)
        .map(|(k, v)| (k.clone(), match v {
            Value::U32(n) | Value::U32Alt(n) => json!(n),
            Value::Text(_) | Value::TextAlt(_) => {
                let text = v.as_str().unwrap_or_default();
                json!(match text.char_indices().nth(TEXT_MAX) { Some((end, _)) => format!("{}…", &text[..end]), None => text })
            }
            Value::Bin(b) => json!(format!("bin:{}{}", b.iter().take(RAW_MAX / 4).map(|x| format!("{x:02x}")).collect::<String>(), if b.len() > RAW_MAX / 4 { "…" } else { "" })),
        }))
        .collect();
    json!({
        "tag": m.tag,
        "fields": fields,
        "items": m.items.iter().take(ITEMS_MAX).map(message_json).collect::<Vec<_>>(),
        "items_total": m.items.len(),
    })
}

/// The server's session log in yo3-log/1 (docs/logs.md): each `tracing` event as one structured line, its fields as
/// keys (`user`, `peer`, `duel`, `report`...) and a leading `[topic] ` of its message as `topic`. The console keeps the
/// readable text; this is the file.
pub struct SessionLayer(pub yo3_tunnel::diagnostics::Session);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for SessionLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields(serde_json::Map::new());
        event.record(&mut fields);
        let mut line = fields.0;
        let mut msg = line.remove("message").and_then(|m| m.as_str().map(str::to_owned)).unwrap_or_default();
        let level = *event.metadata().level();
        line.insert("level".into(), level.as_str().to_ascii_lowercase().into());
        // `[YO3 PACKET] ...` and `[YO3 ENGINE] ...` are traces of their own; `[tls] ...` a topic.
        let mut src = "server";
        for (marker, s) in [("[YO3 PACKET] ", "packet"), ("[YO3 ENGINE] ", "engine")] {
            if let Some(rest) = msg.strip_prefix(marker) { src = s; msg = rest.to_owned(); }
        }
        if src == "server" && msg.starts_with('[') && let Some(end) = msg.find("] ").filter(|&e| e > 1 && e < 24)
            && msg[1..end].bytes().all(|b| b.is_ascii_lowercase() || b == b'-' || b == b'_') && !line.contains_key("topic")
        {
            line.insert("topic".into(), msg[1..end].into());
            msg = msg[end + 2..].to_owned();
        }
        line.insert("src".into(), src.into());
        line.insert("msg".into(), msg.into());
        self.0.event(line);
    }
}

/// A `tracing` event's fields as JSON values: numbers and booleans as such, the rest as text.
struct Fields(serde_json::Map<String, Json>);
impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) { self.0.insert(field.name().into(), value.into()); }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) { self.0.insert(field.name().into(), value.into()); }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) { self.0.insert(field.name().into(), value.into()); }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) { self.0.insert(field.name().into(), value.into()); }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().into(), format!("{value:?}").into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_diagnostics_include_nested_packets_even_without_developer_raw_files() {
        let dir = std::env::temp_dir().join(format!("yo3-session-packets-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
        let session = yo3_tunnel::diagnostics::Session::start(&dir, "server", None).unwrap();
        let writer = session.clone();
        let subscriber = tracing_subscriber::fmt().without_time().with_ansi(false).with_writer(move || writer.writer()).finish();
        let message = Message::reply(0x2398).item(Message::record().with("name", Value::text("TestDuelist")).with("card", Value::U32(3437)));
        tracing::subscriber::with_default(subscriber, || {
            let packets = PacketLog::disabled().unwrap();
            packets.incoming("front", &message.encode(), Some(&Packet::Message(message.clone())), None);
            packets.outgoing("front", &message.encode(), &message);
        });
        let text = yo3_tunnel::diagnostics::records(&dir).unwrap().into_iter().filter(|r| r.name.ends_with(".log")).map(|r| std::fs::read_to_string(dir.join(r.name)).unwrap()).collect::<String>();
        assert!(text.contains("[YO3 PACKET]") && text.contains("TestDuelist") && text.contains("3437"), "nested packet evidence was lost: {text}");
        assert!(text.contains("<-") && text.contains("->"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_private_server_keeps_packets_out_of_the_session_log() {
        // A hosted server's `packet_log = false` (#37): players' chat lines and names stay out of every log.
        let dir = std::env::temp_dir().join(format!("yo3-session-private-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
        let session = yo3_tunnel::diagnostics::Session::start(&dir, "server", None).unwrap();
        let writer = session.clone();
        let subscriber = tracing_subscriber::fmt().without_time().with_ansi(false).with_writer(move || writer.writer()).finish();
        let message = Message::reply(0x2455).item(Message::record().with("chat_str", Value::text("PrivateChatLine")));
        tracing::subscriber::with_default(subscriber, || {
            let packets = PacketLog::disabled().unwrap().with_traces(false);
            packets.incoming("front", &message.encode(), Some(&Packet::Message(message.clone())), None);
            packets.outgoing("front", &message.encode(), &message);
        });
        let text = yo3_tunnel::diagnostics::records(&dir).unwrap().into_iter().filter(|r| r.name.ends_with(".log")).map(|r| std::fs::read_to_string(dir.join(r.name)).unwrap()).collect::<String>();
        assert!(!text.contains("PrivateChatLine") && !text.contains("[YO3 PACKET]"), "{text}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_packet_log_rotates_and_keeps_only_its_newest_files() {
        let dir = std::env::temp_dir().join(format!("yo3-log-rotate-{}", std::process::id()));
        let log = PacketLog::create(&dir).unwrap().with_limits(20_000, 3);
        let m = Message::reply(0x1F4F).with("name", Value::text(&"x".repeat(200)));
        let plain = m.encode();
        // A monkey (monkey testing, 2026-11-08) wrote 2.6 GB of log in five minutes: a client can fill the disk.
        for _ in 0..2000 {
            log.incoming("front", &plain, Some(&Packet::Message(m.clone())), None);
        }
        let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.metadata().unwrap().len()).collect();
        files.sort();
        assert!(files.len() <= 3, "{} log files kept", files.len());
        assert!(files.iter().sum::<u64>() < 3 * 21_000 + 3_000, "{files:?} bytes of log kept");
        assert!(log.path().exists(), "the current file exists");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_huge_packet_is_logged_in_part() {
        let dir = std::env::temp_dir().join(format!("yo3-log-cap-{}", std::process::id()));
        let log = PacketLog::create(&dir).unwrap();
        let mut m = Message::reply(0x1F4F).with("name", Value::text(&"x".repeat(500_000)));
        for _ in 0..5000 {
            m = m.item(Message::record().with("k", Value::U32(1)));
        }
        let plain = m.encode();
        assert!(plain.len() > 50_000);
        log.incoming("front", &plain, Some(&Packet::Message(m.clone())), None);
        log.outgoing("front", &plain, &m);
        let written = std::fs::read_to_string(log.path()).unwrap();
        assert_eq!(written.lines().count(), 2);
        assert!(written.len() < 40_000, "{} bytes logged for two big packets", written.len());
    }
}
