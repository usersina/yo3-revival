//! The client side of the wire: a seeded RNG, a connection with the client's real framing, hex helpers and the
//! corpus of real requests recorded in the packet logs.
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{MAX_FRAME, Message, Packet, Value, decrypt, frame, frame_len};

/// SplitMix64 (the flag says whether this run may build huge values: only a few plans do, since a message is built whole
/// before it is cut to a frame): small, fast, and the same on every machine, so a seed always means the same run.
#[derive(Clone)]
pub struct Rng(pub u64, pub bool);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng(seed ^ 0x9E37_79B9_7F4A_7C15, false);
        r.next();
        r
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// 0..n (0 when n is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below((hi - lo + 1) as usize) as u32
    }
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64 <= p
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
    /// Random bytes, between `lo` and `hi` of them.
    pub fn blob(&mut self, lo: u32, hi: u32) -> Vec<u8> {
        let n = self.range(lo, hi) as usize;
        self.bytes(n)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()).collect()
}

/// The client's opening version string as a plain body (`00 len text`).
pub fn version_plain() -> Vec<u8> {
    let v = b"2010.03.23 skelton";
    let mut out = vec![0, v.len() as u8];
    out.extend(v);
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Gate,
    Front,
    Duel,
    Http,
}

impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Target::Gate => "gate",
            Target::Front => "front",
            Target::Duel => "duel",
            Target::Http => "http",
        }
    }
    pub fn parse(s: &str) -> Option<Target> {
        [Target::Gate, Target::Front, Target::Duel, Target::Http].into_iter().find(|t| t.name() == s)
    }
}

/// Where the server listens.
#[derive(Clone, Debug)]
pub struct Ports {
    pub host: String,
    pub gate: u16,
    pub front: u16,
    pub duel: u16,
    pub http: u16,
}

impl Ports {
    pub fn addr(&self, t: Target) -> String {
        let port = match t {
            Target::Gate => self.gate,
            Target::Front => self.front,
            Target::Duel => self.duel,
            Target::Http => self.http,
        };
        format!("{}:{port}", self.host)
    }
}

/// A connection to one of the server's ports. What the server sends is decoded by a task of its own, so the server
/// never blocks on a full socket while the monkey is busy writing.
pub struct Conn {
    w: OwnedWriteHalf,
    rx: UnboundedReceiver<Message>,
    pub received: usize,
}

impl Conn {
    pub async fn open(addr: &str) -> std::io::Result<Conn> {
        let stream = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "connect timed out"))??;
        stream.set_nodelay(true).ok();
        let (mut r, w) = stream.into_split();
        let (tx, rx) = unbounded_channel();
        tokio::spawn(async move {
            let mut head = [0u8; 4];
            loop {
                if r.read_exact(&mut head).await.is_err() {
                    return;
                }
                let n = frame_len(head);
                if n == 0 || n > MAX_FRAME {
                    return;
                }
                let mut body = vec![0u8; n];
                if r.read_exact(&mut body).await.is_err() {
                    return;
                }
                if let Ok(plain) = decrypt(&body)
                    && let Ok(Packet::Message(m)) = Packet::decode(&plain)
                    && tx.send(m).is_err()
                {
                    return;
                }
            }
        });
        Ok(Conn { w, rx, received: 0 })
    }

    /// Writes bytes as they are (a frame, half a frame, garbage); an error means the server closed or stalled.
    pub async fn write_raw(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        match tokio::time::timeout(Duration::from_secs(10), self.w.write_all(bytes)).await {
            Ok(r) => r,
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "write stalled for 10 s")),
        }
    }

    pub async fn send_plain(&mut self, plain: &[u8]) -> std::io::Result<()> {
        self.write_raw(&frame(plain)).await
    }

    pub async fn send(&mut self, m: &Message) -> std::io::Result<()> {
        self.send_plain(&m.encode()).await
    }

    pub async fn send_version(&mut self) -> std::io::Result<()> {
        self.send_plain(&version_plain()).await
    }

    /// The next message from the server, or None after `wait` (or when it closed).
    pub async fn recv(&mut self, wait: Duration) -> Option<Message> {
        let m = tokio::time::timeout(wait, self.rx.recv()).await.ok().flatten();
        if m.is_some() {
            self.received += 1;
        }
        m
    }

    /// Everything that arrives within `wait` of the last message.
    pub async fn drain(&mut self, wait: Duration) -> Vec<Message> {
        let mut out = vec![];
        while let Some(m) = self.recv(wait).await {
            out.push(m);
        }
        out
    }

    /// The first message whose request is `id`, within `wait` overall; others are returned too.
    pub async fn expect(&mut self, id: u32, wait: Duration) -> (Option<Message>, Vec<Message>) {
        let end = tokio::time::Instant::now() + wait;
        let mut others = vec![];
        loop {
            let left = end.saturating_duration_since(tokio::time::Instant::now());
            match self.recv(left).await {
                Some(m) if m.request() == Some(id) => return (Some(m), others),
                Some(m) => others.push(m),
                None => return (None, others),
            }
        }
    }
}

pub fn req(id: u32) -> Message {
    Message::reply(id)
}

pub fn req_with(id: u32, fields: &[(&str, u32)]) -> Message {
    fields.iter().fold(Message::reply(id), |m, (k, v)| m.with(k, Value::U32(*v)))
}

/// What the packet logs hold of the real client's side: its messages in order, per log file and server.
pub struct Session {
    pub target: Target,
    pub msgs: Vec<Message>,
}

#[derive(Default)]
pub struct Corpus {
    pub sessions: Vec<Session>,
    /// Every field name and request id the real client uses, for mutations and random messages.
    pub names: Vec<String>,
    pub ids: Vec<u32>,
    /// A real deck update (0x1F4F, update_type 1) with its deck item, for the duel monkeys.
    pub deck_update: Option<Message>,
}

impl Corpus {
    /// Loads every `packets-*.jsonl` in `dir` (newest `max_files` of them): the `<-` lines whose raw bytes were logged
    /// in full.
    pub fn load(dir: &Path, max_files: usize) -> Corpus {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        files.retain(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("packets-") && n.ends_with(".jsonl")));
        files.sort();
        let skip = files.len().saturating_sub(max_files);
        let mut c = Corpus::default();
        let (mut names, mut ids) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        for path in files.into_iter().skip(skip) {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let mut per_target: Vec<(Target, Vec<Message>)> = vec![];
            for line in text.lines().filter(|l| l.contains("\"dir\":\"<-\"")) {
                let Ok(j) = serde_json::from_str::<serde_json::Value>(line) else { continue };
                let (Some(server), Some(raw)) = (j["server"].as_str(), j["raw"].as_str()) else { continue };
                if j["raw_len"].as_u64().is_some_and(|n| n as usize > raw.len() / 2) {
                    continue; // logged in part
                }
                let (Some(target), Some(plain)) = (Target::parse(server), unhex(raw)) else { continue };
                let Ok(Packet::Message(m)) = Packet::decode(&plain) else { continue };
                if m.fields.is_empty() {
                    continue;
                }
                for (k, _) in &m.fields {
                    names.insert(k.clone());
                }
                if let Some(id) = m.request() {
                    ids.insert(id);
                }
                if m.request() == Some(0x1F4F) && m.get("update_type").and_then(Value::as_u32) == Some(1) && !m.items.is_empty() {
                    c.deck_update.get_or_insert_with(|| m.clone());
                }
                match per_target.iter_mut().find(|(t, _)| *t == target) {
                    Some((_, v)) => v.push(m),
                    None => per_target.push((target, vec![m])),
                }
            }
            for (target, msgs) in per_target {
                if msgs.len() >= 3 {
                    c.sessions.push(Session { target, msgs });
                }
            }
        }
        c.names = names.into_iter().collect();
        c.ids = ids.into_iter().collect();
        c
    }

    pub fn of(&self, t: Target) -> Vec<&Session> {
        self.sessions.iter().filter(|s| s.target == t).collect()
    }
}

/// A plain body as the wire carries it: encrypted, with the masked length in front.
pub fn framed(plain: &[u8]) -> Vec<u8> {
    frame(plain)
}
