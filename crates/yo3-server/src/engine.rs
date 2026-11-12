//! The rules engine: EDOPro's core run by `engine/duelcore.ts` under Node, one process per duel, spoken to in
//! JSON lines. Cards go in and come back as Konami card ids. `engine/fetch.sh` fetches what it needs.
//!
//! The calls block (a step of the core takes milliseconds), but never for longer than [`CALL_TIMEOUT`]: a sidecar that
//! hangs is killed and the call fails, like one that died. Diagnostic dependency hashing has a separate bounded
//! budget because it reads thousands of files; it does not extend the deadlines for playing a duel.
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
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
const REQUIRED_FILES: &[&str] = &["duelcore.ts", "package.json", "node_modules/ocgcore-wasm/package.json",
    "node_modules/ocgcore-wasm/dist/index.js", "node_modules/ocgcore-wasm/lib/ocgcore.sync.mjs",
    "node_modules/ocgcore-wasm/lib/ocgcore.sync.wasm", "vendor/cardinfo.json", "vendor/BabelCDB/cards.cdb",
    "vendor/CardScripts/constant.lua", "vendor/CardScripts/utility.lua"];

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

/// Lists every absent known anchor. Existing unreadable/invalid files and script-directory problems are separate.
pub fn missing_files(dir: &Path) -> Vec<PathBuf> {
    REQUIRED_FILES.iter().map(|name|dir.join(name))
        .filter(|path|matches!(std::fs::metadata(path),Err(error) if error.kind()==io::ErrorKind::NotFound)).collect()
}

/// Checks a finite set of required engine anchors, not every imported module or card script.
/// Runtime startup still checks whether the complete sidecar can load and execute.
pub fn files_problem(dir: &Path) -> Option<String> {
    let readable=|path: &Path| -> io::Result<()> {
        if !std::fs::metadata(path)?.is_file() {return Err(io::Error::new(io::ErrorKind::InvalidData,"not a regular file"))}
        let mut file=std::fs::File::open(path)?;
        if !file.metadata()?.is_file() {return Err(io::Error::new(io::ErrorKind::InvalidData,"not a regular file"))}
        if file.read(&mut [0;1])?==0 {return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"file is empty"))}
        Ok(())
    };
    for name in REQUIRED_FILES {
        let path=dir.join(name);
        if let Err(error)=readable(&path) {return Some(format!("Required engine file unavailable: {} ({error})",path.display()))}
    }
    let scripts=dir.join("vendor/CardScripts/official");
    let entries=match std::fs::read_dir(&scripts) {Ok(entries)=>entries,Err(error)=>return Some(format!("Required card scripts unavailable: {} ({error})",scripts.display()))};
    let mut unreadable=None;
    for entry in entries {
        let entry=match entry {Ok(entry)=>entry,Err(error)=>return Some(format!("Cannot read card script directory: {} ({error})",scripts.display()))};
        let path=entry.path();if path.extension().is_none_or(|extension|extension!="lua") {continue}
        match readable(&path) {Ok(())=>return None,Err(error)=>unreadable=Some(format!("Required card script unavailable: {} ({error})",path.display()))}
    }
    unreadable.or_else(||Some(format!("No readable, nonempty .lua card scripts found in {}",scripts.display())))
}

/// Why duels can't be played: the engine's files are missing, or its Node.js ([`node`]) is missing or older than
/// [`MIN_NODE`]. None when the required anchors and Node version pass. Runs `node --version`, not the sidecar.
pub fn problem(dir: Option<&Path>) -> Option<String> {
    let (want_major, want_minor) = MIN_NODE;
    let Some(dir) = dir else {
        return Some("the rules engine is missing (unpack the release bundle again, whole; from source, run engine/fetch.sh)".into());
    };
    if let Some(problem)=files_problem(dir) {return Some(problem)}
    let mut command=Command::new(node(dir));command.arg("--version");
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let out = match command.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => return Some(format!("Node.js is missing (the release bundle's engine/node, else one on the PATH, version {want_major}.{want_minor} or newer)")),
    };
    match parse_node_version(&out) {
        Some(v) if v >= MIN_NODE => None,
        _ => Some(format!("Node.js {} is too old (version {want_major}.{want_minor} or newer is needed)", out.trim())),
    }
}

/// Konami ids of client cards EDOPro doesn't have at all (game and promotion exclusives: Ulevo, Meteo the Matchless,
/// Xexex, Eternia, the Arcane Lords' Testament and Armament, Chimaera the Master of Beasts, Emperor of Lightning, Tyr,
/// Lorelei, Aggiba, Skuna). No pack or preset deck holds them; the deck check refuses them, since a duel can't play
/// them. `tests/card_data.rs` checks that this is exactly the set the engine can't find.
pub const NOT_IN_THE_ENGINE: [u16; 12] = [5925, 5929, 6533, 6534, 6958, 6959, 7374, 7375, 7992, 7993, 8610, 8611];

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
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        let stderr = child.stderr.take().expect("piped stderr");
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        if let Err(error) = std::thread::Builder::new().name("engine-errors".into()).spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || capture_errors(stderr));
        }) { let _ = child.kill(); let _ = child.wait(); return Err(error) }
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
        self.new_duel_named(seed, lp, decks, &HashMap::new())
    }

    /// `new_duel` with the English names of the decks' cards (the client's `names`, by Konami id, as many as given):
    /// the engine finds a print its card list lacks (the client's second Dark Magician, 5165) by name.
    pub fn new_duel_named(&mut self, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2], names: &HashMap<u16, String>) -> io::Result<Answer> {
        let used: std::collections::BTreeMap<String, &String> = decks
            .iter()
            .flat_map(|(main, extra)| main.iter().chain(extra.iter()))
            .filter_map(|cid| names.get(cid).map(|name| (cid.to_string(), name)))
            .collect();
        let decks: Vec<Json> = decks.iter().map(|(main, extra)| json!({ "main": main, "extra": extra })).collect();
        self.call(json!({ "op": "new", "seed": seed, "lp": lp, "decks": decks, "names": used }))
    }

    /// How the engine loads each card `(Konami id, English name)`: one answer per card, `{cid, passcode}` (null: the
    /// engine has no such card) with `data` (its card data: type, level, attack, defense, attribute, race, alias,
    /// name), `missing_scripts` and `errors` (the core's, loading it). For the card check (`tests/card_data.rs`).
    pub fn cards(&mut self, cards: &[(u16, &str)]) -> io::Result<Vec<Json>> {
        let cards: Vec<Json> = cards.iter().map(|(cid, name)| json!({ "cid": cid, "name": name })).collect();
        let mut v = self.line(json!({ "op": "cards", "cards": cards }))?;
        match v.get("status").and_then(Json::as_str) {
            Some("cards") => Ok(v.get_mut("cards").map(Json::take).and_then(|c| serde_json::from_value(c).ok()).unwrap_or_default()),
            _ => Err(io::Error::other(format!("engine: no card check ({})", v.get("error").and_then(Json::as_str).unwrap_or("unknown")))),
        }
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

fn capture_errors(mut source: impl Read) {
    let mut buffer = [0; 4096]; let mut line = Vec::new(); let mut overflow = false; let mut private = false;
    let mut emit = |bytes: &[u8], overflow: bool| {
        let text = String::from_utf8_lossy(bytes);
        if text.contains("-----BEGIN") && text.contains("PRIVATE KEY-----") { private = true; }
        let message = if overflow { "[overlong diagnostic line omitted]".into() } else if private { "[private key omitted]".into() }
            else { yo3_tunnel::diagnostics::redact_secrets(&text, &[]) };
        if text.contains("-----END") && text.contains("PRIVATE KEY-----") { private = false; }
        tracing::warn!("[YO3 ENGINE STDERR] {message}");
    };
    loop {
        let n = match source.read(&mut buffer) { Ok(0) => break, Ok(n) => n, Err(error) => { tracing::warn!("Could not read engine stderr: {error}"); break } };
        for byte in &buffer[..n] {
            if *byte == b'\n' { emit(&line, overflow); line.clear(); overflow = false; }
            else if line.len() < 16000 { line.push(*byte); } else { overflow = true; }
        }
    }
    if !line.is_empty() || overflow { emit(&line, overflow); }
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

    const REQUIRED_ENGINE_FILES: &[&str] = &["duelcore.ts", "package.json", "node_modules/ocgcore-wasm/package.json",
        "node_modules/ocgcore-wasm/dist/index.js", "node_modules/ocgcore-wasm/lib/ocgcore.sync.mjs",
        "node_modules/ocgcore-wasm/lib/ocgcore.sync.wasm", "vendor/cardinfo.json", "vendor/BabelCDB/cards.cdb",
        "vendor/CardScripts/constant.lua", "vendor/CardScripts/utility.lua"];
    fn files_fixture(name: &str) -> PathBuf {
        let dir=std::env::temp_dir().join(format!("yo3-engine-files-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&dir);
        for name in REQUIRED_ENGINE_FILES.iter().copied().chain(["vendor/CardScripts/official/c1.lua"]) {
            let path=dir.join(name);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,b"fixture").unwrap();
        }
        dir
    }
    #[test]
    #[cfg(unix)]
    fn a_missing_sidecar_is_rejected_even_when_node_reports_a_supported_version() {
        use std::os::unix::fs::PermissionsExt;
        let dir=files_fixture("sidecar");let executable=dir.join("node/node");std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable,b"#!/bin/sh\nprintf 'v22.18.0\\n'\n").unwrap();
        std::fs::set_permissions(&executable,std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(problem(Some(&dir)).is_none(),"fixture's Node version is sufficient before a required file disappears");
        let sidecar=dir.join("duelcore.ts");std::fs::remove_file(&sidecar).unwrap();
        let failure=problem(Some(&dir));std::fs::remove_dir_all(dir).unwrap();
        assert!(failure.is_some_and(|message|message.contains(&sidecar.to_string_lossy().to_string())),"a supported Node binary must not hide a missing sidecar");
    }
    #[test]
    fn required_engine_files_reject_missing_empty_and_nonfile_anchors_with_their_paths() {
        let dir=files_fixture("anchors");assert!(files_problem(&dir).is_none());
        for name in REQUIRED_ENGINE_FILES {
            let path=dir.join(name);std::fs::remove_file(&path).unwrap();
            assert!(files_problem(&dir).unwrap().contains(&path.to_string_lossy().to_string()),"name the missing file: {name}");
            std::fs::write(&path,b"").unwrap();
            assert!(files_problem(&dir).unwrap().contains(&path.to_string_lossy().to_string()),"reject an empty file: {name}");
            std::fs::remove_file(&path).unwrap();std::fs::create_dir(&path).unwrap();
            assert!(files_problem(&dir).unwrap().contains(&path.to_string_lossy().to_string()),"reject a directory in place of a file: {name}");
            std::fs::remove_dir(&path).unwrap();std::fs::write(&path,b"restored").unwrap();assert!(files_problem(&dir).is_none());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn all_missing_engine_anchors_are_listed_without_mislabeling_existing_invalid_files() {
        let dir=files_fixture("missing-list");assert!(missing_files(&dir).is_empty());
        let expected=["duelcore.ts","node_modules/ocgcore-wasm/dist/index.js","vendor/BabelCDB/cards.cdb"]
            .into_iter().map(|name|dir.join(name)).collect::<Vec<_>>();
        for path in &expected {std::fs::remove_file(path).unwrap();}
        std::fs::write(dir.join("package.json"),b"").unwrap();
        let nonfile=dir.join("vendor/CardScripts/constant.lua");std::fs::remove_file(&nonfile).unwrap();std::fs::create_dir(nonfile).unwrap();
        assert_eq!(missing_files(&dir),expected,"report every known absent path, but not empty files or directories that already exist");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn official_scripts_require_a_readable_nonempty_lua_file_rather_than_just_a_directory() {
        let dir=files_fixture("scripts");let scripts=dir.join("vendor/CardScripts/official");
        std::fs::remove_dir_all(&scripts).unwrap();assert!(files_problem(&dir).unwrap().contains(&scripts.to_string_lossy().to_string()));
        std::fs::create_dir(&scripts).unwrap();assert!(files_problem(&dir).unwrap().contains(".lua"));
        std::fs::write(scripts.join("README.txt"),b"scripts were removed").unwrap();assert!(files_problem(&dir).is_some());
        std::fs::create_dir(scripts.join("directory.lua")).unwrap();assert!(files_problem(&dir).is_some());
        std::fs::write(scripts.join("empty.lua"),b"").unwrap();assert!(files_problem(&dir).is_some());
        std::fs::write(scripts.join("c1.lua"),b"return {}").unwrap();assert!(files_problem(&dir).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn an_unreadable_required_engine_file_is_reported_with_its_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir=files_fixture("unreadable");let path=dir.join("duelcore.ts");
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o000)).unwrap();
        let permission_enforced=std::fs::File::open(&path).is_err();let failure=files_problem(&dir);let absent=missing_files(&dir);
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o600)).unwrap();std::fs::remove_dir_all(dir).unwrap();
        // An elevated test process can bypass POSIX read permissions; do not confuse that with a checker failure.
        if permission_enforced {assert!(failure.unwrap().contains(&path.to_string_lossy().to_string()));}
        assert!(absent.is_empty(),"an unreadable existing file must not be reported as absent");
    }
    #[test]
    #[cfg(target_os="linux")]
    fn nonfile_engine_anchors_do_not_wait_for_pipe_writers() {
        let dir=files_fixture("fifo");let path=dir.join("duelcore.ts");std::fs::remove_file(&path).unwrap();
        assert!(Command::new("mkfifo").arg(&path).status().unwrap().success());
        let checked=dir.clone();let (send,received)=channel();
        let check=std::thread::spawn(move ||send.send(files_problem(&checked)).unwrap());
        let result=received.recv_timeout(Duration::from_secs(1));let finished=result.is_ok();
        if !finished {
            // Linux permits opening a FIFO read/write without a peer. Unblock the old checker before failing.
            let release=std::fs::OpenOptions::new().read(true).write(true).open(&path).unwrap();
            check.join().unwrap();drop(release);
        } else {check.join().unwrap();}
        std::fs::remove_dir_all(dir).unwrap();
        assert!(finished,"the file-type check must reject a FIFO before a blocking open");
        assert!(result.unwrap().unwrap().contains(&path.to_string_lossy().to_string()));
    }

    #[test]
    #[cfg(unix)]
    fn engine_stderr_is_saved_in_diagnostics_instead_of_only_the_terminal() {
        let root=std::env::temp_dir().join(format!("yo3-engine-stderr-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let session=yo3_tunnel::diagnostics::Session::start(&root,"server",None).unwrap();let writer=session.clone();
        let subscriber=tracing_subscriber::fmt().without_time().with_ansi(false).with_writer(move||writer.writer()).finish();
        let mut command=Command::new("sh");command.args(["-c","printf 'source.lua:17: effect failed\\n' >&2; read -r request; printf '%s\\n' '{\"status\":\"end\",\"messages\":[],\"errors\":[]}'"]);
        let mut engine=tracing::subscriber::with_default(subscriber,||Engine::spawn(command,Duration::from_secs(2))).unwrap();
        engine.new_duel(1,8000,[(&[4007;40],&[]),(&[4007;40],&[])]).unwrap();drop(engine);
        let deadline=std::time::Instant::now()+Duration::from_secs(2);
        let text=loop {
            let text=yo3_tunnel::diagnostics::records(&root).unwrap().into_iter().filter(|r|r.name.ends_with(".log")).map(|r|std::fs::read_to_string(root.join(r.name)).unwrap()).collect::<String>();
            if text.contains("source.lua:17: effect failed") || std::time::Instant::now()>=deadline {break text}
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(text.contains("source.lua:17: effect failed"),"engine errors must survive Scripts hosting: {text}");
        std::fs::remove_dir_all(root).unwrap();
    }

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
