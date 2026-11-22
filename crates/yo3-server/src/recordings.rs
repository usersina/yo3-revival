//! Duel recordings: every duel's rules-engine traffic, kept for a while so that a player's "an effect went wrong" can be
//! replayed exactly (`tools/monkey/duel-replay.py --recording FILE`).
//!
//! Each duel writes one gzipped JSON-lines file, `recordings/<YYYY-MM-DD>/<id>.jsonl.gz` in the data folder: a header
//! (`format`, the duel's id, kind, start, players, the anime setting and the engine's dependency identity), then one line
//! per engine call, `{"call": n, "request": …, "answer": …}`, flushed as it goes so a duel still running or a server that
//! stopped leaves a readable file. The duel bot's `think` calls are left out: the `respond` after each carries the bot's
//! answer, and a replay never repeats them. The `recording-days` setting keeps them that many days (0: none are made).
//!
//! The `bug` chat command pins a player's current or last duel: `recordings/pinned/<id>.jsonl.gz`, a hard link (a copy
//! where the file system has none), which the daily pruning leaves alone. The file holds decks, moves and the engine's
//! answers: no account, password or address.
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value as Json, json};

/// The header's `format`.
pub const FORMAT: &str = "yo3-duel-recording";
/// How long after a duel `bug` still finds it.
pub const BUG_WINDOW: Duration = Duration::from_secs(2 * 3600);

/// One side of a recorded duel: a player (their id) or the computer (no id).
#[derive(Clone, Debug, PartialEq)]
pub struct Seat {
    pub player_id: Option<u32>,
    pub name: String,
}

/// A player's current or last recorded duel.
#[derive(Clone, Debug)]
pub struct Latest {
    pub id: String,
    /// The other side: a player (their id) or an NPC (no id), by name.
    pub opponent: Seat,
    pub started: Instant,
}

/// The recordings of one data folder.
#[derive(Debug)]
pub struct Recordings {
    dir: PathBuf,
    latest: Mutex<HashMap<u32, Latest>>,
    /// Ids made in this second, so two duels starting together get different ones.
    made: Mutex<(String, u32)>,
}

impl Recordings {
    /// The recordings under `data_dir/recordings`.
    pub fn at(data_dir: &Path) -> Recordings {
        Recordings { dir: data_dir.join("recordings"), latest: Mutex::default(), made: Mutex::default() }
    }

    fn next_id(&self) -> String {
        let now = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let mut made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        if made.0 != now { *made = (now.clone(), 0) }
        made.1 += 1;
        format!("{now}-{:03}", made.1)
    }

    /// A new recording of a duel between `seats` (`kind`: "npc" or "pvp"), whose engine reports `identity`. Each seat's
    /// player has it as their latest duel. Its header is written at once.
    pub fn start(&self, kind: &str, seats: &[Seat], anime: bool, identity: Option<Json>) -> io::Result<Recorder> {
        let id = self.next_id();
        let day = self.dir.join(format!("{}-{}-{}", &id[0..4], &id[4..6], &id[6..8]));
        std::fs::create_dir_all(&day)?;
        let path = day.join(format!("{id}.jsonl.gz"));
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        let mut recorder = Recorder { id: id.clone(), out: Some(GzEncoder::new(BufWriter::new(file), Compression::default())), calls: 0 };
        let players: Vec<Json> = seats.iter().map(|s| json!({ "player_id": s.player_id, "name": s.name })).collect();
        let header = json!({ "format": FORMAT, "version": 1, "id": id, "kind": kind, "started": chrono::Utc::now().to_rfc3339(),
            "server": env!("CARGO_PKG_VERSION"), "players": players, "anime": anime, "identity": identity });
        recorder.line(&header.to_string());
        let mut latest = self.latest.lock().unwrap_or_else(|e| e.into_inner());
        for (i, seat) in seats.iter().enumerate() {
            let Some(player) = seat.player_id else { continue };
            let opponent = seats.get(1 - i.min(1)).filter(|_| seats.len() == 2).cloned().unwrap_or(Seat { player_id: None, name: String::new() });
            latest.insert(player, Latest { id: id.clone(), opponent, started: Instant::now() });
        }
        Ok(recorder)
    }

    /// `player`'s current or last recorded duel, if it started within [`BUG_WINDOW`].
    pub fn latest(&self, player: u32) -> Option<Latest> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner()).get(&player).filter(|l| l.started.elapsed() < BUG_WINDOW).cloned()
    }

    /// The file of recording `id`: the pinned one, else the dated one. None for an id that isn't one or isn't there.
    pub fn find(&self, id: &str) -> Option<PathBuf> {
        if !valid_id(id) { return None }
        let pinned = self.dir.join("pinned").join(format!("{id}.jsonl.gz"));
        let dated = self.dir.join(format!("{}-{}-{}", &id[0..4], &id[4..6], &id[6..8])).join(format!("{id}.jsonl.gz"));
        [pinned, dated].into_iter().find(|p| p.is_file())
    }

    /// Keeps recording `id` past the pruning: a hard link in `pinned/` (a copy where links aren't possible). Its path.
    pub fn pin(&self, id: &str) -> io::Result<PathBuf> {
        let source = self.find(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no recording {id}")))?;
        let pinned = self.dir.join("pinned");
        std::fs::create_dir_all(&pinned)?;
        let target = pinned.join(format!("{id}.jsonl.gz"));
        if source == target || target.is_file() { return Ok(target) }
        if std::fs::hard_link(&source, &target).is_err() { std::fs::copy(&source, &target)?; }
        Ok(target)
    }

    /// Removes the days of recordings older than `days` (0 removes every dated one); pinned ones stay. How many files.
    pub fn prune(&self, days: u32) -> io::Result<usize> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(i64::from(days))).format("%Y-%m-%d").to_string();
        let mut removed = 0;
        let entries = match std::fs::read_dir(&self.dir) { Ok(e) => e, Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0), Err(e) => return Err(e) };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let dated = name.len() == 10 && chrono::NaiveDate::parse_from_str(&name, "%Y-%m-%d").is_ok();
            if !dated || (days > 0 && name >= cutoff) { continue }
            removed += std::fs::read_dir(entry.path()).map(|d| d.count()).unwrap_or(0);
            std::fs::remove_dir_all(entry.path())?;
        }
        Ok(removed)
    }
}

/// The lines of a recording file, the header first. A duel still running (or a server that stopped mid-duel) leaves the
/// gzip stream without its end: every complete line before it is read.
pub fn read(path: &Path) -> io::Result<Vec<Json>> {
    use std::io::BufRead;
    let mut lines = vec![];
    for line in io::BufReader::new(flate2::read::GzDecoder::new(File::open(path)?)).lines() {
        match line {
            Ok(text) => lines.push(serde_json::from_str(&text).map_err(io::Error::other)?),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }
    Ok(lines)
}

/// A recording id: `YYYYMMDD-HHMMSS-NNN` (digits and dashes only, so it can't name another path).
pub fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() >= 19 && b.len() <= 24 && b[8] == b'-' && b[15] == b'-'
        && b.iter().enumerate().all(|(i, c)| if i == 8 || i == 15 { *c == b'-' } else { c.is_ascii_digit() })
}

/// One duel's recording file, written call by call. A write that fails stops the recording (logged once), never the duel.
pub struct Recorder {
    id: String,
    out: Option<GzEncoder<BufWriter<File>>>,
    calls: u32,
}

impl Recorder {
    pub fn id(&self) -> &str { &self.id }

    fn line(&mut self, text: &str) {
        let Some(out) = &mut self.out else { return };
        if let Err(e) = out.write_all(text.as_bytes()).and_then(|_| out.write_all(b"\n")).and_then(|_| out.flush()) {
            tracing::warn!("duel recording {} stopped: {e}", self.id);
            self.out = None;
        }
    }

    /// One engine call: its request and the engine's answer line (JSON, written as it came). `think` is left out.
    pub fn call(&mut self, request: &Json, answer: &str) {
        if request["op"] == "think" || request["op"] == "identity" { return }
        self.calls += 1;
        let answer = if serde_json::from_str::<Json>(answer).is_ok() { answer.to_string() } else { Json::String(answer.into()).to_string() };
        self.line(&format!("{{\"call\":{},\"request\":{request},\"answer\":{answer}}}", self.calls));
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        if let Some(out) = self.out.take() && let Err(e) = out.finish().and_then(|mut w| w.flush()) {
            tracing::warn!("duel recording {} not finished: {e}", self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-recordings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn lines(path: &Path) -> Vec<Json> { read(path).unwrap() }

    #[test]
    fn a_duel_is_recorded_call_by_call_and_readable_while_it_runs() {
        let dir = scratch("calls");
        let recordings = Recordings::at(&dir);
        let seats = [Seat { player_id: Some(1001), name: "Yugi".into() }, Seat { player_id: None, name: "Pegasus".into() }];
        let mut r = recordings.start("npc", &seats, true, Some(json!({ "core": "abc" }))).unwrap();
        r.call(&json!({ "op": "new", "seed": 7 }), r#"{"status":"await","messages":[]}"#);
        r.call(&json!({ "op": "think" }), r#"{"status":"thought","response":{}}"#);
        r.call(&json!({ "op": "respond", "response": { "type": 1 } }), r#"{"status":"await","messages":[{"type":40}]}"#);
        let path = recordings.find(r.id()).expect("its file");
        let read = lines(&path);
        assert_eq!(read[0]["format"], FORMAT);
        assert_eq!((read[0]["kind"].as_str(), read[0]["anime"].as_bool(), read[0]["identity"]["core"].as_str()), (Some("npc"), Some(true), Some("abc")));
        assert_eq!(read.len(), 3, "the header and two calls, the bot's thinking left out: {read:?}");
        assert_eq!((read[1]["call"].as_u64(), read[1]["request"]["seed"].as_u64()), (Some(1), Some(7)));
        assert_eq!((read[2]["call"].as_u64(), read[2]["answer"]["messages"][0]["type"].as_u64()), (Some(2), Some(40)));
        drop(r);
        assert_eq!(lines(&path).len(), 3);
        let latest = recordings.latest(1001).expect("the player's latest duel");
        assert_eq!((latest.opponent.name.as_str(), latest.opponent.player_id), ("Pegasus", None));
        assert!(recordings.latest(1002).is_none());
    }

    #[test]
    fn both_players_of_a_table_have_the_duel_and_each_other_as_the_opponent() {
        let dir = scratch("pvp");
        let recordings = Recordings::at(&dir);
        let seats = [Seat { player_id: Some(1), name: "A".into() }, Seat { player_id: Some(2), name: "B".into() }];
        let first = recordings.start("pvp", &seats, false, None).unwrap();
        let second = recordings.start("pvp", &seats, false, None).unwrap();
        assert_ne!(first.id(), second.id(), "two duels in one second get their own ids");
        assert_eq!(recordings.latest(1).unwrap().opponent, seats[1]);
        assert_eq!(recordings.latest(2).unwrap().opponent, seats[0]);
        assert_eq!(recordings.latest(2).unwrap().id, second.id());
    }

    #[test]
    fn a_pinned_recording_outlives_the_pruning_and_ids_cant_name_other_paths() {
        let dir = scratch("prune");
        let recordings = Recordings::at(&dir);
        let seat = [Seat { player_id: Some(1), name: "A".into() }];
        let kept = recordings.start("npc", &seat, false, None).unwrap().id().to_string();
        let gone = recordings.start("npc", &seat, false, None).unwrap().id().to_string();
        let old = dir.join("recordings/2020-01-01");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("20200101-000000-001.jsonl.gz"), b"x").unwrap();
        assert_eq!(recordings.prune(14).unwrap(), 1, "only the old day goes");
        assert!(!old.exists() && recordings.find(&kept).is_some());
        recordings.pin(&kept).unwrap();
        assert_eq!(recordings.prune(0).unwrap(), 2, "0 clears every dated recording");
        assert!(recordings.find(&kept).is_some(), "the pinned one stays");
        assert!(recordings.find(&gone).is_none());
        for bad in ["../../server.db", "20200101-000000-00a", "2020010100000000001", "", "20200101-000000-001/../x"] {
            assert!(!valid_id(bad) && recordings.find(bad).is_none(), "{bad:?}");
        }
        assert!(valid_id(&kept));
    }
}
