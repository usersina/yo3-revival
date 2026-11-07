//! The rules engine: EDOPro's core run by `engine/duelcore.ts` under Node, one process per duel, spoken to in
//! JSON lines. Cards go in and come back as Konami card ids. `engine/fetch.sh` fetches what it needs.
//!
//! The calls block (a step of the core takes milliseconds).
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

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

pub struct Engine {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Engine {
    /// The engine directory: `YO3_ENGINE_DIR`, else `engine/` under the current directory. None until
    /// `fetch.sh` has run there.
    pub fn dir() -> Option<PathBuf> {
        let dir = std::env::var_os("YO3_ENGINE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("engine"));
        dir.join("vendor/cardinfo.json").is_file().then_some(dir)
    }

    pub fn start(dir: &Path) -> io::Result<Engine> {
        let mut child = Command::new("node")
            .arg(dir.join("duelcore.ts"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Engine { child, stdin, stdout })
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

    fn call(&mut self, request: Json) -> io::Result<Answer> {
        writeln!(self.stdin, "{request}")?;
        self.stdin.flush()?;
        let mut line = String::new();
        if self.stdout.read_line(&mut line)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "the engine exited"));
        }
        let mut v: Json = serde_json::from_str(&line).map_err(io::Error::other)?;
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
