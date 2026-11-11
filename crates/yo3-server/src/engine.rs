//! The rules engine: EDOPro's core run by `engine/duelcore.ts` under Node, one process per duel, spoken to in
//! JSON lines. Cards go in and come back as Konami card ids. `engine/fetch.sh` fetches what it needs.
//!
//! The calls block (a step of the core takes milliseconds), but never for longer than [`CALL_TIMEOUT`]: a sidecar that
//! hangs is killed and the call fails, like one that died. Diagnostic dependency hashing has a separate bounded
//! budget because it reads thousands of files; it does not extend the deadlines for playing a duel.
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use serde_json::{Value as Json, json};

/// What the core did up to its next prompt (or the end of the duel).
#[derive(Debug)]
pub struct Answer {
    /// The core's messages, as ocgcore-wasm parses them (`type` = OcgMessageType); every `code` has a `cid`.
    pub messages: Vec<Json>,
    /// "await" (the last message is a prompt), "end", or "error".
    pub status: String,
    pub errors: Vec<String>,
}

/// How long one call may take (a duel's first call loads the card database, a step takes milliseconds).
pub const CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Debug recording hashes thousands of dependency files, which can be cold on Windows.
const IDENTITY_TIMEOUT: Duration = Duration::from_secs(120);

/// The oldest Node.js that runs `duelcore.ts` directly (type stripping without a flag).
pub const MIN_NODE: (u32, u32) = (22, 18);

/// `(major, minor)` of a `node --version` line such as `v22.18.0`.
pub fn parse_node_version(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.trim().strip_prefix('v')?.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// The Node.js that runs the sidecar: the release bundles' own (`node/node`, `node\\node.exe` on Windows, in the
/// engine directory), else `node` on the PATH.
pub fn node(dir: &Path) -> PathBuf {
    let bundled = dir.join("node").join(format!("node{}", std::env::consts::EXE_SUFFIX));
    if bundled.is_file() { bundled } else { PathBuf::from("node") }
}

/// Why duels can't be played: the engine's files are missing, or its Node.js ([`node`]) is missing or older than
/// [`MIN_NODE`]. None when the engine can run. Runs `node --version` once (fast), not the sidecar.
pub fn problem(dir: Option<&Path>) -> Option<String> {
    let (want_major, want_minor) = MIN_NODE;
    let Some(dir) = dir else {
        return Some("the rules engine is missing (unpack the release bundle again, whole; from source, run engine/fetch.sh)".into());
    };
    let out = match Command::new(node(dir)).arg("--version").output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => return Some(format!("Node.js is missing (the release bundle's engine/node, else one on the PATH, version {want_major}.{want_minor} or newer)")),
    };
    match parse_node_version(&out) {
        Some(v) if v >= MIN_NODE => None,
        _ => Some(format!("Node.js {} is too old (version {want_major}.{want_minor} or newer is needed)", out.trim())),
    }
}

pub struct Engine {
    child: Child,
    stdin: ChildStdin,
    /// The sidecar's output lines, read by a thread of their own so a call can time out.
    lines: Receiver<io::Result<String>>,
    timeout: Duration,
    identity_timeout: Duration,
    recording: Option<(u64, u32)>,
}
static RECORDING_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Engine {
    /// The engine directory: `YO3_ENGINE_DIR`, else `engine/` under the current directory. None until
    /// `fetch.sh` has run there.
    pub fn dir() -> Option<PathBuf> {
        let dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("engine"));
        dir.join("vendor/cardinfo.json").is_file().then_some(dir)
    }

    pub fn start(dir: &Path) -> io::Result<Engine> {
        let mut node = Command::new(node(dir));
        node.arg(dir.join("duelcore.ts"));
        let mut engine = Engine::spawn(node, CALL_TIMEOUT)?;
        engine.identity_timeout = IDENTITY_TIMEOUT;
        Ok(engine)
    }

    /// Runs `command` as the sidecar (JSON lines on its stdin and stdout), each call allowed `timeout`.
    pub fn spawn(mut command: Command, timeout: Duration) -> io::Result<Engine> {
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in stdout.lines() {
                let failed = line.is_err();
                if tx.send(line).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Engine { child, stdin, lines, timeout, identity_timeout: timeout, recording: None })
    }

    pub fn identity(&mut self) -> io::Result<Json> {
        let identity = self.line_with_timeout(json!({"op":"identity"}), self.identity_timeout)?;
        let fingerprints = ["sidecar", "core", "bindings", "scripts", "database", "card_ids"];
        if !identity["node"].is_string() || fingerprints.iter().any(|k| !identity[*k].as_str()
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())))
        { return Err(io::Error::other("the engine did not supply its diagnostic dependency identity")); }
        Ok(identity)
    }

    /// Record exact final deck order, seeds, responses and engine answers in the existing diagnostic session.
    /// Chunking keeps large states below the session logger's line limit. No account credentials are involved.
    pub fn record(&mut self) {
        self.recording = Some((RECORDING_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed), 0));
        if let Err(e) = self.identity() { tracing::error!("duel check: recording identity unavailable: {e}"); }
    }

    fn record_line(&self, direction: &str, data: &str) {
        if let Some((duel, call)) = self.recording {
            let parts = data.len().div_ceil(1000);
            for (part, chunk) in data.as_bytes().chunks(1000).enumerate() {
                // Short hex groups preserve arbitrary JSON/64-bit seeds through credential redaction, without
                // exempting diagnostic lines from that filter. This payload only contains rules-engine traffic.
                let data: Vec<String> = chunk.chunks(8).map(|b| b.iter().map(|v| format!("{v:02x}")).collect()).collect();
                tracing::info!("[YO3 ENGINE] {}", json!({"duel":duel,"call":call,"direction":direction,"part":part,"parts":parts,"encoding":"hex","data":data}));
            }
        }
    }

    /// Starts a duel with `lp` each; team 0 goes first. Decks are (main, extra) as Konami card ids.
    pub fn new_duel(&mut self, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2]) -> io::Result<Answer> {
        let decks: Vec<Json> = decks.iter().map(|(main, extra)| json!({ "main": main, "extra": extra })).collect();
        self.call(json!({ "op": "new", "seed": seed, "lp": lp, "decks": decks }))
    }

    /// Answers the core's prompt (an OcgResponse object).
    pub fn respond(&mut self, response: Json) -> io::Result<Answer> {
        self.call(json!({ "op": "respond", "response": response }))
    }

    /// Where every card is now, by engine team: `[{hand, mzone, szone, grave, removed, extra, deck}]` (see the sidecar's
    /// `state()`). Read only: the duel goes on as it was. For the duel check (`crate::duelcheck`).
    pub fn state(&mut self) -> io::Result<Json> {
        let mut v = self.line(json!({ "op": "state" }))?;
        match v.get("status").and_then(Json::as_str) {
            Some("state") => Ok(v.get_mut("teams").map(Json::take).unwrap_or_default()),
            _ => Err(io::Error::other(format!("engine: no state ({})", v.get("error").and_then(Json::as_str).unwrap_or("unknown")))),
        }
    }

    /// Sends one request line and reads the answer line, within the timeout.
    fn line(&mut self, request: Json) -> io::Result<Json> {
        self.line_with_timeout(request, self.timeout)
    }

    fn line_with_timeout(&mut self, request: Json, timeout: Duration) -> io::Result<Json> {
        if let Some((_, call)) = &mut self.recording { *call += 1; }
        self.record_line("request", &request.to_string());
        writeln!(self.stdin, "{request}")?;
        self.stdin.flush()?;
        let line = match self.lines.recv_timeout(timeout) {
            Ok(line) => line?,
            Err(RecvTimeoutError::Timeout) => {
                let _ = self.child.kill();
                return Err(io::Error::new(io::ErrorKind::TimedOut, "the engine did not answer in time"));
            }
            Err(RecvTimeoutError::Disconnected) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "the engine exited")),
        };
        self.record_line("answer", &line);
        serde_json::from_str(&line).map_err(io::Error::other)
    }

    fn call(&mut self, request: Json) -> io::Result<Answer> {
        let mut v = self.line(request)?;
        let mut take_list = |k: &str| match v.get_mut(k).map(Json::take) {
            Some(Json::Array(items)) => items,
            _ => vec![],
        };
        let messages = take_list("messages");
        let errors = take_list("errors").iter().filter_map(|e| e.as_str().map(String::from)).collect();
        let status = v.get("status").and_then(Json::as_str).unwrap_or("error").to_string();
        if status == "error" {
            let why = v.get("error").and_then(Json::as_str).unwrap_or("unknown");
            return Err(io::Error::other(format!("engine: {why}")));
        }
        Ok(Answer { messages, status, errors })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn cold_diagnostic_fingerprints_have_their_own_budget_without_unbounding_duel_steps() {
        let hash = "0".repeat(64);
        let identity = json!({"node":"v26.13.0", "sidecar":hash, "core":hash, "bindings":hash,
            "scripts":hash, "database":hash, "card_ids":hash});
        let mut fixture = Command::new("sh");
        fixture.arg("-c").arg(format!("read -r request; sleep 0.15; printf '%s\\n' '{identity}'; read -r request; printf '%s\\n' '{{\"status\":\"await\",\"messages\":[],\"errors\":[]}}'; read -r request; exec sleep 10"));
        let mut engine = Engine::spawn(fixture, Duration::from_millis(50)).unwrap();
        engine.identity_timeout = Duration::from_secs(2);
        engine.identity().expect("cold dependency hashing must not use the short duel-step deadline");
        engine.new_duel(1, 8000, [(&[4007; 40], &[]), (&[4007; 40], &[])]).unwrap();
        let started = std::time::Instant::now();
        assert_eq!(engine.state().unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(1), "duel steps keep the original deadline");
    }

    #[test]
    fn the_bundled_node_is_preferred_to_the_one_on_the_path() {
        let dir = std::env::temp_dir().join(format!("yo3-bundled-node-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(node(&dir), PathBuf::from("node"), "no bundle: the PATH's");
        let bundled = dir.join("node").join(format!("node{}", std::env::consts::EXE_SUFFIX));
        std::fs::create_dir_all(bundled.parent().unwrap()).unwrap();
        std::fs::write(&bundled, b"").unwrap();
        assert_eq!(node(&dir), bundled);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn node_versions_compare_by_major_then_minor() {
        assert_eq!(parse_node_version("v22.18.0\n"), Some((22, 18)));
        assert_eq!(parse_node_version("v20.0.0"), Some((20, 0)));
        assert_eq!(parse_node_version("nonsense"), None);
        assert!(parse_node_version("v22.9.0").unwrap() < MIN_NODE);
        assert!(parse_node_version("v22.18.0").unwrap() >= MIN_NODE);
        assert!(parse_node_version("v24.1.0").unwrap() >= MIN_NODE);
    }

    #[test]
    fn a_missing_engine_directory_is_a_problem() {
        assert!(problem(None).unwrap().contains("rules engine is missing"));
    }
}
