//! Packet log: one JSON line per packet, plus a readable summary through `tracing`.
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Packet, Value};

pub struct PacketLog {
    path: PathBuf,
    file: Mutex<File>,
}

impl PacketLog {
    pub fn create(dir: &Path) -> std::io::Result<PacketLog> {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        std::fs::create_dir_all(dir)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!("packets-{stamp}-{}-{seq}.jsonl", std::process::id()));
        Ok(PacketLog { file: Mutex::new(File::create(&path)?), path })
    }

    pub fn path(&self) -> &Path {
        &self.path
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

    fn write(&self, who: &str, dir: &str, plain: &[u8], msg: Json, error: Option<&str>) {
        let line = json!({
            "t": chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            "server": who, "dir": dir, "msg": msg,
            "raw": plain.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "error": error,
        });
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{line}");
        }
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::U32(n) | Value::U32Alt(n) => n.to_string(),
        Value::Text(_) | Value::TextAlt(_) => format!("{:?}", v.as_str().unwrap_or_default()),
        Value::Bin(b) => {
            let hex: String = b.iter().take(24).map(|x| format!("{x:02x}")).collect();
            format!("bin[{}]:{hex}{}", b.len(), if b.len() > 24 { "…" } else { "" })
        }
    }
}

pub fn summarize(m: &Message) -> String {
    let id = m.request().map(|r| format!("{r:#06x}")).unwrap_or_else(|| "(no id)".into());
    let fields: Vec<String> =
        m.fields.iter().filter(|(k, _)| k != "request").map(|(k, v)| format!("{k}={}", show(v))).collect();
    let items = if m.items.is_empty() { String::new() } else { format!(" +{} item(s)", m.items.len()) };
    format!("{id} {{{}}}{items}", fields.join(", "))
}

fn message_json(m: &Message) -> Json {
    let fields: serde_json::Map<String, Json> = m
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), match v {
            Value::U32(n) | Value::U32Alt(n) => json!(n),
            other => json!(show(other)),
        }))
        .collect();
    json!({ "tag": m.tag, "fields": fields, "items": m.items.iter().map(message_json).collect::<Vec<_>>() })
}
