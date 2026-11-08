//! Running the monkey: the supervised server, plans, the canary, duels and floods, and what is watched meanwhile.
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use yo3_proto::{Message, Value};

use crate::plans::{Plan, Step};
use crate::wire::{Conn, Corpus, Ports, Rng, Target, req, req_with};

pub fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------------------------------------------
// The server under test.

pub struct Supervisor {
    bin: PathBuf,
    cwd: PathBuf,
    scratch: PathBuf,
    ports: Ports,
    extra: Vec<String>,
    child: Option<Child>,
    pub starts: usize,
}

impl Supervisor {
    pub fn new(bin: PathBuf, cwd: PathBuf, scratch: PathBuf, ports: Ports, extra: Vec<String>) -> Supervisor {
        Supervisor { bin, cwd, scratch, ports, extra, child: None, starts: 0 }
    }

    pub fn log_path(&self) -> PathBuf {
        self.scratch.join("server.log")
    }
    pub fn data_dir(&self) -> PathBuf {
        self.scratch.join("data")
    }
    pub fn packet_dir(&self) -> PathBuf {
        self.scratch.join("logs")
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.data_dir())?;
        let log = std::fs::OpenOptions::new().create(true).append(true).open(self.log_path())?;
        let child = Command::new(&self.bin)
            .args(["--log-dir"])
            .arg(self.packet_dir())
            .arg("--data-dir")
            .arg(self.data_dir())
            .args(["--players", "Yugi,Kaiba"])
            .args(["--gate-port", &self.ports.gate.to_string(), "--front-port", &self.ports.front.to_string()])
            .args(["--duel-port", &self.ports.duel.to_string(), "--http-port", &self.ports.http.to_string()])
            .args(&self.extra)
            .current_dir(&self.cwd)
            .env("RUST_LOG", "warn")
            .env("NO_COLOR", "1")
            .env("RUST_BACKTRACE", "1")
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?;
        self.child = Some(child);
        self.starts += 1;
        Ok(())
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }

    /// Some(status) once the server has exited.
    pub fn exited(&mut self) -> Option<String> {
        let c = self.child.as_mut()?;
        match c.try_wait() {
            Ok(Some(s)) => Some(s.to_string()),
            Ok(None) => None,
            Err(e) => Some(format!("wait failed: {e}")),
        }
    }

    pub async fn wait_ready(&self) -> bool {
        for _ in 0..200 {
            if Conn::open(&self.ports.addr(Target::Gate)).await.is_ok() && Conn::open(&self.ports.addr(Target::Http)).await.is_ok() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            // SIGTERM first so the server can flush, then the hard kill.
            let _ = Command::new("kill").arg(c.id().to_string()).status();
            for _ in 0..30 {
                if matches!(c.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// What is watched.

/// New lines of the server's log that mean trouble: a panic, an ERROR.
pub struct LogWatch {
    path: PathBuf,
    pos: u64,
    pub warns: HashMap<String, usize>,
}

pub fn signature(line: &str) -> String {
    let t: String = line.chars().filter(|c| !c.is_ascii_digit()).collect();
    t.chars().take(160).collect()
}

impl LogWatch {
    pub fn new(path: PathBuf) -> LogWatch {
        let pos = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        LogWatch { path, pos, warns: HashMap::new() }
    }

    /// The troubling lines since the last call (a panic with its message line).
    pub fn trouble(&mut self) -> Vec<String> {
        let Ok(mut f) = std::fs::File::open(&self.path) else { return vec![] };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.pos {
            self.pos = 0;
        }
        if f.seek(SeekFrom::Start(self.pos)).is_err() {
            return vec![];
        }
        let mut buf = Vec::new();
        let _ = f.take(64 << 20).read_to_end(&mut buf);
        self.pos += buf.len() as u64;
        let text = String::from_utf8_lossy(&buf);
        let lines: Vec<&str> = text.lines().collect();
        let mut out = vec![];
        for (i, line) in lines.iter().enumerate() {
            if line.contains("panicked at") || line.contains("stack overflow") || line.contains("fatal runtime error") {
                out.push(format!("{} | {}", line, lines.get(i + 1).unwrap_or(&"")));
            } else if line.contains(" ERROR ") {
                out.push(line.to_string());
            } else if line.contains(" WARN ") {
                let key: String = signature(line.split(" WARN ").nth(1).unwrap_or(line));
                *self.warns.entry(key).or_default() += 1;
            }
        }
        out
    }

    pub fn tail(&self, n: usize) -> String {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        lines[lines.len().saturating_sub(n)..].join("\n")
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub rss_kb: u64,
    pub fds: usize,
    pub threads: u64,
    pub data_bytes: u64,
    pub log_bytes: u64,
}

fn dir_size(p: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(p) else { return 0 };
    rd.flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn sample(pid: u32, data: &Path, packets: &Path) -> Sample {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let field = |k: &str| status.lines().find_map(|l| l.strip_prefix(k)).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse().ok()).unwrap_or(0u64);
    Sample {
        rss_kb: field("VmRSS:"),
        threads: field("Threads:"),
        fds: std::fs::read_dir(format!("/proc/{pid}/fd")).map(|d| d.count()).unwrap_or(0),
        data_bytes: dir_size(data),
        log_bytes: dir_size(packets),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Failures.

pub struct Reporter {
    dir: PathBuf,
    state: Mutex<(usize, HashMap<String, usize>)>,
}

impl Reporter {
    pub fn new(dir: PathBuf) -> Reporter {
        let _ = std::fs::create_dir_all(&dir);
        Reporter { dir, state: Mutex::new((0, HashMap::new())) }
    }

    pub fn total(&self) -> usize {
        lock(&self.state).0
    }

    pub fn signatures(&self) -> Vec<(String, usize)> {
        let mut v: Vec<_> = lock(&self.state).1.iter().map(|(k, n)| (k.clone(), *n)).collect();
        v.sort_by_key(|b| std::cmp::Reverse(b.1));
        v
    }

    /// Records a failure: the first three of each kind and signature are saved with the plans around them.
    pub fn fail(&self, kind: &str, detail: &str, plans: &[Arc<Plan>], log_tail: &str) {
        let sig = format!("{kind}: {}", signature(detail));
        let (n, seen) = {
            let mut st = lock(&self.state);
            st.0 += 1;
            let seen = {
                let e = st.1.entry(sig.clone()).or_default();
                *e += 1;
                *e
            };
            (st.0, seen)
        };
        println!("FAIL #{n} [{kind}] {}", detail.lines().next().unwrap_or(""));
        if seen > 3 {
            return;
        }
        let dir = self.dir.join(format!("{n:03}-{}", kind.replace([' ', '/'], "-")));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("info.txt"), format!("{kind}\n{detail}\n\nplans saved: {}\n", plans.len()));
        let _ = std::fs::write(dir.join("server-tail.txt"), log_tail);
        for (i, p) in plans.iter().enumerate() {
            let _ = std::fs::write(dir.join(format!("plan-{i:02}-seed{}.txt", p.seed)), p.to_text());
        }
        println!("     saved in {}", dir.display());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Plans.

#[derive(Default, Debug)]
pub struct Outcome {
    pub sent: usize,
    pub received: usize,
    pub error: Option<String>,
}

/// Plays a plan against its port. A write that fails (the server closed on a bad frame) ends the plan, unless a
/// later step reconnects.
pub async fn run_plan(ports: &Ports, plan: &Plan) -> Outcome {
    let addr = ports.addr(plan.target);
    let mut out = Outcome::default();
    let mut conn = match Conn::open(&addr).await {
        Ok(c) => c,
        Err(e) => {
            out.error = Some(format!("connect: {e}"));
            return out;
        }
    };
    let mut dead = false;
    for (i, step) in plan.steps.iter().enumerate() {
        let r = match step {
            Step::Plain(p) if !dead => {
                out.sent += 1;
                conn.send_plain(p).await
            }
            Step::Wire(w) if !dead => {
                out.sent += 1;
                conn.write_raw(w).await
            }
            Step::Drip(w) if !dead => {
                out.sent += 1;
                let mut rng = Rng::new(plan.seed ^ i as u64);
                let mut r = Ok(());
                let head = w.len().min(48);
                let mut at = 0;
                while at < head && r.is_ok() {
                    let n = (rng.below(3) + 1).min(head - at);
                    r = conn.write_raw(&w[at..at + n]).await;
                    at += n;
                    tokio::time::sleep(Duration::from_millis(rng.below(4) as u64)).await;
                }
                if r.is_ok() && head < w.len() {
                    r = conn.write_raw(&w[head..]).await;
                }
                r
            }
            Step::Sleep(ms) => {
                tokio::time::sleep(Duration::from_millis(*ms)).await;
                Ok(())
            }
            Step::Reconnect => {
                out.received += conn.received;
                match Conn::open(&addr).await {
                    Ok(c) => {
                        conn = c;
                        dead = false;
                        Ok(())
                    }
                    Err(e) => {
                        out.error = Some(format!("reconnect: {e}"));
                        return out;
                    }
                }
            }
            _ => Ok(()),
        };
        if let Err(e) = r {
            dead = true;
            out.error.get_or_insert(e.to_string());
        }
        if i % 8 == 7 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    // Let what the server answers be read, so its socket never fills.
    let _ = conn.drain(Duration::from_millis(40)).await;
    out.received += conn.received;
    out
}

// ---------------------------------------------------------------------------------------------------------------
// The canary: a well-behaved client that logs in and asks for the player list, timing every round trip.

#[derive(Default)]
pub struct CanaryStats {
    pub rounds: AtomicUsize,
    pub failures: AtomicUsize,
    pub spikes: AtomicUsize,
    times_ms: Mutex<Vec<u32>>,
}

impl CanaryStats {
    pub fn percentile(&self, p: f64) -> u32 {
        let mut v = lock(&self.times_ms).clone();
        v.sort_unstable();
        if v.is_empty() { 0 } else { v[((v.len() - 1) as f64 * p) as usize] }
    }
    pub fn record(&self, ms: u32) {
        self.rounds.fetch_add(1, Ordering::Relaxed);
        let mut t = lock(&self.times_ms);
        if t.len() < 200_000 {
            t.push(ms);
        }
    }
}

/// One round: gate, then the front's login and the player list. Ok with how long it took, or what went wrong.
pub async fn canary_round(ports: &Ports, pid: u32) -> Result<Duration, String> {
    let start = Instant::now();
    let wait = Duration::from_secs(8);
    let mut gate = Conn::open(&ports.addr(Target::Gate)).await.map_err(|e| format!("gate connect: {e}"))?;
    gate.send_version().await.map_err(|e| format!("gate write: {e}"))?;
    gate.expect(0x21FE, wait).await.0.ok_or("gate: no 0x21FE")?;
    let mut c = Conn::open(&ports.addr(Target::Front)).await.map_err(|e| format!("front connect: {e}"))?;
    c.send_version().await.map_err(|e| format!("front write: {e}"))?;
    c.expect(0x219F, wait).await.0.ok_or("front: no version reply 0x219F")?;
    for (ask, answer) in [(req(0x1F47), 0x219A), (req_with(0x1F48, &[("lang", 1)]), 0x2201), (req(0x1F49), 0x2260), (req_with(0x1F4C, &[("player_id", pid)]), 0x2266)] {
        let name = format!("{:#x}", ask.request().unwrap_or(0));
        c.send(&ask).await.map_err(|e| format!("write {name}: {e}"))?;
        if c.expect(answer, wait).await.0.is_none() {
            return Err(format!("no {answer:#x} after {name}"));
        }
    }
    c.send(&req_with(0x1FA4, &[("player_id", pid)])).await.map_err(|e| e.to_string())?;
    c.drain(Duration::from_millis(30)).await;
    let list = req_with(0x2CF0, &[("back_num", 0), ("base_point", 0), ("front_num", 100), ("search_flag", 1), ("search_type", 1)]);
    c.send(&list).await.map_err(|e| e.to_string())?;
    if c.expect(0x2CF7, wait).await.0.is_none() {
        return Err("no player list 0x2CF7".into());
    }
    Ok(start.elapsed())
}

// ---------------------------------------------------------------------------------------------------------------
// Floods and the HTTP port.

/// Opens `n` connections to a port at once, some sending junk, holds them for a while, and drops them.
pub async fn flood(ports: &Ports, target: Target, n: usize, hold: Duration, seed: u64) -> usize {
    let mut handles = vec![];
    for i in 0..n {
        let addr = ports.addr(target);
        let mut rng = Rng::new(seed.wrapping_add(i as u64));
        handles.push(tokio::spawn(async move {
            let Ok(mut c) = Conn::open(&addr).await else { return false };
            match rng.below(4) {
                0 => {
                    let _ = c.send_version().await;
                }
                1 => {
                    let n = rng.below(30);
                    let _ = c.write_raw(&rng.bytes(n)).await;
                }
                _ => {}
            }
            tokio::time::sleep(hold).await;
            true
        }));
    }
    let mut ok = 0;
    for h in handles {
        ok += h.await.unwrap_or(false) as usize;
    }
    ok
}

// ---------------------------------------------------------------------------------------------------------------
// Duels: two protocol clients that match through Auto Duel and then play at random.

#[derive(Default)]
pub struct DuelStats {
    pub pairs: AtomicUsize,
    pub matched: AtomicUsize,
    pub entered: AtomicUsize,
    pub janken: AtomicUsize,
    pub began: AtomicUsize,
    pub ended: AtomicUsize,
    pub actions: AtomicUsize,
    pub left_early: AtomicUsize,
}

const SERVER_DUEL: u32 = 0x200D; // the server's duel messages (request 8205)
const CLIENT_DUEL: u32 = 0x200C;

fn duel_message(name: u32, fields: &[(&str, u32)]) -> Message {
    let mut m = req_with(CLIENT_DUEL, &[("duelRequestName", name)]);
    for (k, v) in fields {
        m = m.with(k, Value::U32(*v));
    }
    m
}

/// One random duel message: a card command, a phase button, a dialog result, a pick, a janken hand, and sometimes
/// nonsense (a wrong name, huge values, a mutated one).
pub fn random_duel_action(rng: &mut Rng, c: &Corpus, chaos: f64) -> Message {
    let small = |rng: &mut Rng, n: u32| if rng.chance(chaos) { crate::plans::random_u32(rng) } else { rng.range(0, n) };
    let m = match rng.below(12) {
        0..=3 => duel_message(2, &[("CommandID", small(rng, 12)), ("index", small(rng, 44)), ("player", small(rng, 2)), ("position", small(rng, 70))]),
        4..=5 => duel_message(3, &[("phase", small(rng, 8))]),
        6..=7 => duel_message(4, &[("DLGResult", small(rng, 6)), ("debug", 0)]),
        8 => duel_message(7, &[("index", small(rng, 44)), ("data", small(rng, 44))]),
        9 => duel_message(0xC, &[("selectHand", small(rng, 4))]),
        10 => duel_message(0xD, &[("choice", small(rng, 3))]),
        _ => duel_message(small(rng, 0x20), &[("x", rng.next() as u32)]),
    };
    let mut m = m;
    if rng.chance(chaos * 0.2) {
        crate::plans::mutate(rng, &mut m, c);
    }
    m
}

pub async fn login_front(ports: &Ports, pid: u32) -> Result<Conn, String> {
    let mut c = Conn::open(&ports.addr(Target::Front)).await.map_err(|e| e.to_string())?;
    c.send_version().await.map_err(|e| e.to_string())?;
    for m in [req(0x1F47), req_with(0x1F48, &[("lang", 1)]), req(0x1F49), req_with(0x1F4C, &[("player_id", pid)]), req_with(0x1FA4, &[("player_id", pid)])] {
        c.send(&m).await.map_err(|e| e.to_string())?;
    }
    c.drain(Duration::from_millis(60)).await;
    Ok(c)
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
enum Stage {
    Login,
    Queued,
    Matched,
    Entered,
    Playing,
    Ended,
}

/// One player's side of a duel, played at random: Auto Duel against another client, or an NPC duel against the server's
/// CPU (`npc`). Returns the furthest stage reached.
async fn duel_client(ports: Ports, corpus: Arc<Corpus>, stats: Arc<DuelStats>, pid: u32, seed: u64, play_secs: f64, npc: bool, match_type: u32) -> Stage {
    let mut rng = Rng::new(seed);
    let mut stage = Stage::Login;
    let Ok(mut front) = login_front(&ports, pid).await else { return stage };
    let wait = Duration::from_secs(25);
    let (join_id, duel_id) = if npc {
        // 0x17AC {event_id, join_id} with the rule as its item -> 0x17AD {duel_id}.
        let cpu = *rng.pick(&[120u32, 120, 121, 122, 100, 1, 0, 99_999]);
        let rule = Message::record()
            .with("cpu_id", Value::U32(cpu))
            .with("duel_server_id", Value::U32(0))
            .with("match_type", Value::U32(4))
            .with("regulation_id", Value::U32(3))
            .with("surrender", Value::U32(1))
            .with("time_total", Value::U32(0))
            .with("time_turn", Value::U32(0));
        if front.send(&req_with(0x17AC, &[("event_id", 1), ("join_id", 1)]).item(rule)).await.is_err() {
            return stage;
        }
        let (r, _) = front.expect(0x17AD, Duration::from_secs(5)).await;
        let Some(duel_id) = r.and_then(|m| m.get("duel_id").and_then(Value::as_u32)) else { return stage };
        stats.matched.fetch_add(1, Ordering::Relaxed);
        stage = Stage::Matched;
        (1, duel_id)
    } else {
        if front.send(&req_with(0x1784, &[("event_id", 2)])).await.is_err() {
            return stage;
        }
        // 0x1784 -> 0x1785 {join_id}
        let (join, _) = front.expect(0x1785, Duration::from_secs(5)).await;
        let join_id = join.and_then(|m| m.get("join_id").and_then(Value::as_u32)).unwrap_or(1);
        let _ = front.send(&req_with(0x1798, &[("event_id", 2), ("join_id", join_id), ("level", 1), ("match_type", match_type)])).await;
        stage = Stage::Queued;
        // The match push 0x1978 holds the duel's id.
        let (pushed, _) = front.expect(0x1978, wait).await;
        let Some(pushed) = pushed else { return stage };
        stats.matched.fetch_add(1, Ordering::Relaxed);
        stage = Stage::Matched;
        (join_id, pushed.items.iter().find_map(|i| i.get("duel_id").and_then(Value::as_u32)).unwrap_or(1))
    };
    // Leave at a random moment: some players go before the duel opens.
    let leave_after = if rng.chance(0.25) { Some(rng.range(0, (play_secs * 1000.0) as u32) as u64) } else { None };
    let began = Instant::now();
    let gone = |began: Instant| leave_after.is_some_and(|ms| began.elapsed() > Duration::from_millis(ms));
    if let Some(deck) = &corpus.deck_update {
        let mut d = deck.clone();
        for (k, v) in d.fields.iter_mut() {
            if k == "duel_id" {
                *v = Value::U32(duel_id);
            }
        }
        let _ = front.send(&d).await;
        let _ = front.expect(0x22CA, Duration::from_secs(5)).await;
    }
    if gone(began) {
        stats.left_early.fetch_add(1, Ordering::Relaxed);
        return stage;
    }
    let _ = front.send(&req_with(0x17D4, &[("event_id", 0), ("join_id", join_id), ("duel_id", duel_id)])).await;
    let (data, _) = front.expect(0x22C4, Duration::from_secs(10)).await;
    let Some(data) = data else { return stage };
    let item = data.items.first();
    let port = item.and_then(|i| i.get("port")).and_then(Value::as_u32).map(|p| p as u16).unwrap_or(ports.duel);
    let _ = port; // the server advertises the port it listens on; the monkey keeps to the configured one
    let Ok(mut duel) = Conn::open(&ports.addr(Target::Duel)).await else { return stage };
    let _ = duel.send(&req_with(0x200A, &[("player_id", pid), ("autoSet", 1)])).await;
    stats.entered.fetch_add(1, Ordering::Relaxed);
    stage = Stage::Entered;
    let deadline = Instant::now() + Duration::from_secs_f64(play_secs);
    let mut heartbeat = Instant::now();
    let chaos = *rng.pick(&[0.0, 0.0, 0.1, 0.5]);
    while Instant::now() < deadline && !gone(began) {
        // What the server sent: answer the janken as a client does, and act when it waits for input.
        let msgs = tokio::select! {
            m = duel.recv(Duration::from_millis(150)) => m.into_iter().collect::<Vec<_>>(),
            m = front.recv(Duration::from_millis(150)) => {
                if m.as_ref().and_then(|m| m.request()) == Some(0x2009) {
                    stage = Stage::Ended;
                    stats.ended.fetch_add(1, Ordering::Relaxed);
                    break;
                }
                vec![]
            }
        };
        let mut acted = false;
        for m in msgs {
            if m.request() != Some(SERVER_DUEL) {
                continue;
            }
            let int = |k: &str| m.get(k).and_then(Value::as_u32);
            if int("duelRequestName") == Some(21) && stage < Stage::Playing {
                stage = Stage::Playing;
                stats.began.fetch_add(1, Ordering::Relaxed);
            }
            if int("duelRequestName") != Some(17) {
                continue;
            }
            match int("id") {
                Some(1500) => {
                    stats.janken.fetch_add(1, Ordering::Relaxed);
                    let hand = if rng.chance(chaos) { rng.next() as u32 } else { rng.range(0, 2) };
                    let _ = duel.send(&duel_message(0xC, &[("selectHand", hand)])).await;
                    acted = true;
                }
                Some(1502) if int("param1") == Some(1) => {
                    let _ = duel.send(&duel_message(0xD, &[("choice", rng.range(1, 2))])).await;
                    acted = true;
                }
                Some(4) => {
                    let a = random_duel_action(&mut rng, &corpus, chaos);
                    let _ = duel.send(&a).await;
                    stats.actions.fetch_add(1, Ordering::Relaxed);
                    acted = true;
                }
                _ => {}
            }
        }
        if !acted && rng.chance(0.3) {
            let a = random_duel_action(&mut rng, &corpus, chaos);
            let _ = duel.send(&a).await;
            stats.actions.fetch_add(1, Ordering::Relaxed);
        }
        if heartbeat.elapsed() > Duration::from_secs(5) {
            heartbeat = Instant::now();
            let _ = duel.send(&req(0x2724)).await;
        }
    }
    if gone(began) {
        stats.left_early.fetch_add(1, Ordering::Relaxed);
    }
    stage
}

/// Two clients (Yugi and Kaiba) in one Auto Duel; they match each other, play at random and leave at random moments.
pub async fn duel_pair(ports: &Ports, corpus: &Arc<Corpus>, stats: &Arc<DuelStats>, seed: u64) -> (usize, usize) {
    stats.pairs.fetch_add(1, Ordering::Relaxed);
    let mut rng = Rng::new(seed);
    let secs = rng.range(4, 30) as f64;
    // One in three: a duel against the CPU instead.
    if rng.chance(0.33) {
        let pid = *rng.pick(&[1001u32, 1002]);
        let a = tokio::spawn(duel_client(ports.clone(), corpus.clone(), stats.clone(), pid, rng.next(), secs, true, 4)).await;
        return (a.unwrap_or(Stage::Login) as usize, 0);
    }
    let match_type = *rng.pick(&[1u32, 1, 1, 3, 6]);
    let a = tokio::spawn(duel_client(ports.clone(), corpus.clone(), stats.clone(), 1001, rng.next(), secs, false, match_type));
    tokio::time::sleep(Duration::from_millis(rng.range(0, 400) as u64)).await;
    let b = tokio::spawn(duel_client(ports.clone(), corpus.clone(), stats.clone(), 1002, rng.next(), secs, false, match_type));
    let (a, b) = (a.await.unwrap_or(Stage::Login), b.await.unwrap_or(Stage::Login));
    (a as usize, b as usize)
}

// ---------------------------------------------------------------------------------------------------------------
// A ring of recent plans, so a failure in a busy run can be saved with what was being sent.

#[derive(Default)]
pub struct Recent {
    ring: Mutex<VecDeque<(Instant, Arc<Plan>)>>,
    pub plans: AtomicU64,
    pub messages: AtomicU64,
}

impl Recent {
    pub fn note(&self, p: Arc<Plan>) {
        self.plans.fetch_add(1, Ordering::Relaxed);
        self.messages.fetch_add(p.messages() as u64, Ordering::Relaxed);
        let mut r = lock(&self.ring);
        r.push_back((Instant::now(), p));
        while r.len() > 120 {
            r.pop_front();
        }
    }
    pub fn since(&self, secs: u64, max: usize) -> Vec<Arc<Plan>> {
        let r = lock(&self.ring);
        let cut = Instant::now().checked_sub(Duration::from_secs(secs));
        r.iter().rev().filter(|(t, _)| cut.is_none_or(|c| *t >= c)).take(max).map(|(_, p)| p.clone()).collect()
    }
}

pub fn stop_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

impl Supervisor {
    pub fn ports(&self) -> &Ports {
        &self.ports
    }
}
