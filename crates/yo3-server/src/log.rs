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
    dir: PathBuf,
    name: String,
    max_bytes: u64,
    keep: usize,
    cur: Mutex<Current>,
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
        Ok(PacketLog { dir: dir.to_path_buf(), name, max_bytes: FILE_MAX, keep: FILES_KEPT, cur: Mutex::new(cur) })
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
        tracing::info!("[{who}] {summary}");
        self.write(who, "<-", plain, msg, error);
    }

    pub fn outgoing(&self, who: &str, plain: &[u8], msg: &Message) {
        tracing::info!("[{who}] -> {}", summarize(msg));
        self.write(who, "->", plain, message_json(msg), None);
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

    fn write(&self, who: &str, dir: &str, plain: &[u8], msg: Json, error: Option<&str>) {
        let line = json!({
            "t": chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            "server": who, "dir": dir, "msg": msg,
            "raw": plain.iter().take(RAW_MAX).map(|b| format!("{b:02x}")).collect::<String>(),
            "raw_len": plain.len(),
            "error": error,
        });
        let text = format!("{line}\n");
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

fn message_json(m: &Message) -> Json {
    let fields: serde_json::Map<String, Json> = m
        .fields
        .iter()
        .take(FIELDS_MAX)
        .map(|(k, v)| (k.clone(), match v {
            Value::U32(n) | Value::U32Alt(n) => json!(n),
            other => json!(show(other)),
        }))
        .collect();
    json!({
        "tag": m.tag,
        "fields": fields,
        "items": m.items.iter().take(ITEMS_MAX).map(message_json).collect::<Vec<_>>(),
        "items_total": m.items.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
