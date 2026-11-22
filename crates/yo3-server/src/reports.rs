//! Player reports (#44): a player tells the staff about another with `/report PLAYER reason` in the chat. They are
//! kept in `server.db` (`reports`), with a per-sender limit per hour; the admin API lists them and marks them handled
//! or dismissed (audited). Without a data folder (tests) they are kept in memory.
//!
//! A duel problem (`bug what happened` in the chat) is a report too: about the duel's opponent (a player, or an NPC
//! with id 0), with the duel's recording id (`crate::recordings`), pinned so the staff can replay it.
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

pub const TABLE: &str = "CREATE TABLE IF NOT EXISTS reports (id INTEGER PRIMARY KEY, at TEXT NOT NULL, reporter_id INTEGER NOT NULL,
    reporter TEXT NOT NULL, target_id INTEGER NOT NULL, target TEXT NOT NULL, reason TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'open',
    handled_by TEXT, handled_at TEXT, note TEXT, recording TEXT);";
/// Adds the `recording` column to a database made before duel problems (v0.1.16).
pub const RECORDING_COLUMN: &str = "ALTER TABLE reports ADD COLUMN recording TEXT;";

/// The longest reason a report keeps, in characters.
pub const MAX_REASON: usize = 300;
/// The longest note a moderator adds when handling one.
pub const MAX_NOTE: usize = 300;

/// Where a report stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Open,
    Handled,
    Dismissed,
}

impl Status {
    fn text(self) -> &'static str {
        match self { Status::Open => "open", Status::Handled => "handled", Status::Dismissed => "dismissed" }
    }
    fn of(text: &str) -> Status {
        match text { "handled" => Status::Handled, "dismissed" => Status::Dismissed, _ => Status::Open }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Report {
    #[schema(maximum = 4294967295_u32)]
    pub id: u32,
    /// RFC 3339 UTC, to the second.
    pub at: String,
    pub reporter_id: u32,
    pub reporter: String,
    pub target_id: u32,
    pub target: String,
    pub reason: String,
    pub status: Status,
    pub handled_by: Option<String>,
    pub handled_at: Option<String>,
    pub note: Option<String>,
    /// A duel problem's recording id (`bug` in the chat), pinned in the data folder's `recordings/pinned/`. Null for a
    /// report about a player, and for a duel problem sent without a recent duel.
    #[serde(default)]
    pub recording: Option<String>,
    /// A duel problem (`bug`) rather than a report about a player.
    #[serde(default)]
    pub duel_problem: bool,
}

/// What a duel problem report's `reason` starts with, so the list tells it from a report about a player.
pub const DUEL_PROBLEM: &str = "Duel problem: ";

/// The reports of one server.
#[derive(Clone, Debug, Default)]
pub struct Reports {
    dir: Option<PathBuf>,
    memory: Vec<Report>,
}

fn now() -> String { chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true) }

fn row(r: &rusqlite::Row) -> rusqlite::Result<Report> {
    Ok(Report { id: r.get(0)?, at: r.get(1)?, reporter_id: r.get(2)?, reporter: r.get(3)?, target_id: r.get(4)?, target: r.get(5)?,
        reason: r.get(6)?, status: Status::of(&r.get::<_, String>(7)?), handled_by: r.get(8)?, handled_at: r.get(9)?, note: r.get(10)?,
        recording: r.get(11)?, duel_problem: r.get::<_, String>(6)?.starts_with(DUEL_PROBLEM) })
}

const COLUMNS: &str = "id, at, reporter_id, reporter, target_id, target, reason, status, handled_by, handled_at, note, recording";

impl Reports {
    /// The reports kept in `dir`'s database.
    pub fn at(dir: &Path) -> Reports { Reports { dir: Some(dir.to_path_buf()), memory: vec![] } }

    /// A report from `reporter` about `target`, unless the reporter already made `limit` in the last hour (0: none
    /// may be made). The new report's id.
    pub fn add(&mut self, reporter: (u32, &str), target: (u32, &str), reason: &str, limit: u32) -> Result<u32, String> {
        let reason: String = reason.chars().filter(|c| !c.is_control()).take(MAX_REASON).collect();
        self.insert(reporter, target, reason, None, limit)
    }

    /// A duel problem from `reporter` (`bug what happened`): about the duel's `opponent` (id 0 for an NPC or no duel),
    /// with its pinned `recording`, under the same hourly limit as reports. The new report's id.
    pub fn add_duel_problem(&mut self, reporter: (u32, &str), opponent: (u32, &str), note: &str, recording: Option<&str>, limit: u32) -> Result<u32, String> {
        let note: String = note.chars().filter(|c| !c.is_control()).take(MAX_REASON).collect();
        self.insert(reporter, opponent, format!("{DUEL_PROBLEM}{note}"), recording.map(String::from), limit)
    }

    fn insert(&mut self, reporter: (u32, &str), target: (u32, &str), reason: String, recording: Option<String>, limit: u32) -> Result<u32, String> {
        let (at, since) = (now(), (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let recent = match &self.dir {
            Some(dir) => crate::store::hold(dir).and_then(|c| c.query_row("SELECT count(*) FROM reports WHERE reporter_id=?1 AND at>=?2",
                rusqlite::params![reporter.0, since], |r| r.get::<_, u32>(0)).map_err(crate::store::sql)).map_err(|e| e.to_string())?,
            None => self.memory.iter().filter(|r| r.reporter_id == reporter.0 && r.at >= since).count() as u32,
        };
        if recent >= limit { return Err("too many reports".into()) }
        match &self.dir {
            Some(dir) => {
                let conn = crate::store::hold(dir).map_err(|e| e.to_string())?;
                conn.execute("INSERT INTO reports (at, reporter_id, reporter, target_id, target, reason, recording) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![at, reporter.0, reporter.1, target.0, target.1, reason, recording]).map_err(|e| e.to_string())?;
                Ok(conn.last_insert_rowid() as u32)
            }
            None => {
                let id = self.memory.last().map_or(1, |r| r.id + 1);
                self.memory.push(Report { id, at, reporter_id: reporter.0, reporter: reporter.1.into(), target_id: target.0,
                    duel_problem: reason.starts_with(DUEL_PROBLEM), target: target.1.into(), reason, status: Status::Open, handled_by: None,
                    handled_at: None, note: None, recording });
                Ok(id)
            }
        }
    }

    /// The reports with `status` (all when none), newest first, at most `limit`.
    pub fn list(&self, status: Option<Status>, limit: u32) -> io::Result<Vec<Report>> {
        match &self.dir {
            Some(dir) => {
                let conn = crate::store::hold(dir)?;
                let mut query = conn.prepare(&format!("SELECT {COLUMNS} FROM reports WHERE (?1 IS NULL OR status=?1) ORDER BY id DESC LIMIT ?2"))
                    .map_err(crate::store::sql)?;
                query.query_map(rusqlite::params![status.map(Status::text), limit], row).map_err(crate::store::sql)?
                    .collect::<Result<_, _>>().map_err(crate::store::sql)
            }
            None => Ok(self.memory.iter().rev().filter(|r| status.is_none_or(|s| r.status == s)).take(limit as usize).cloned().collect()),
        }
    }

    pub fn get(&self, id: u32) -> io::Result<Option<Report>> {
        match &self.dir {
            Some(dir) => crate::store::hold(dir)?.query_row(&format!("SELECT {COLUMNS} FROM reports WHERE id=?1"), [id], row)
                .optional().map_err(crate::store::sql),
            None => Ok(self.memory.iter().find(|r| r.id == id).cloned()),
        }
    }

    /// Marks report `id` handled or dismissed by `by`, with an optional note. The report as it was and as it is now.
    pub fn resolve(&mut self, id: u32, status: Status, by: &str, note: &str) -> Result<(Report, Report), String> {
        if status == Status::Open { return Err("a report is handled or dismissed".into()) }
        let before = self.get(id).map_err(|e| e.to_string())?.ok_or_else(|| format!("no report {id}"))?;
        if before.status != Status::Open { return Err(format!("report {id} is already {}", before.status.text())) }
        let note: String = note.chars().filter(|c| !c.is_control()).take(MAX_NOTE).collect();
        let (at, note) = (now(), Some(note).filter(|n| !n.is_empty()));
        let after = Report { status, handled_by: Some(by.into()), handled_at: Some(at.clone()), note: note.clone(), ..before.clone() };
        match &self.dir {
            Some(dir) => {
                crate::store::hold(dir).map_err(|e| e.to_string())?
                    .execute("UPDATE reports SET status=?2, handled_by=?3, handled_at=?4, note=?5 WHERE id=?1",
                        rusqlite::params![id, status.text(), by, at, note]).map_err(|e| e.to_string())?;
            }
            None => if let Some(r) = self.memory.iter_mut().find(|r| r.id == id) { *r = after.clone() },
        }
        Ok((before, after))
    }

    /// Open reports, for the dashboard's badge.
    pub fn open(&self) -> u32 { self.list(Some(Status::Open), 10_000).map_or(0, |r| r.len() as u32) }
}

/// Deletes the reports made by or about any of `players` (a deleted account's).
pub fn forget(dir: &Path, players: &[u32]) -> io::Result<usize> {
    if !dir.join("server.db").is_file() { return Ok(0) }
    let conn = crate::store::hold(dir)?;
    let mut gone = 0;
    for p in players {
        gone += conn.execute("DELETE FROM reports WHERE reporter_id=?1 OR target_id=?1", [p]).map_err(crate::store::sql)?;
    }
    Ok(gone)
}
