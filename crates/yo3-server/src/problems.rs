//! Problem reports (#65's follow-up): a player's launcher logs, sent to the server the game played on (TLS operation 7)
//! when a game broke (the game's own network errors, a relay error, an exit with an error) or with Report a problem.
//! Players used to be asked to attach their logs to a GitHub issue, and most didn't.
//!
//! A report is filed under the session's account with the server's own log lines about that account from the minutes
//! before, so both sides of one disconnect are read together. Each is a file in `data/reports/` (diagnostics, not game
//! state: no backup, pruned by age, count and size). The host reads them with `reports` and `report ID`, or on the
//! dashboard. The launcher scrubs credentials before sending; the server scrubs again.
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use yo3_tunnel::ReportRefused;

/// Reports one account may send in a day.
pub const PER_DAY: usize = 20;
/// Reports kept: the newest this many, and none older than [`KEEP_DAYS`] or past [`KEEP_BYTES`] in all.
pub const KEEP_FILES: usize = 2000;
pub const KEEP_DAYS: i64 = 30;
pub const KEEP_BYTES: u64 = 256 * 1024 * 1024;
/// The server's lines added to a report: about its account, from this many seconds before it, at most [`SERVER_LINES`].
pub const SERVER_WINDOW: i64 = 15 * 60;
pub const SERVER_LINES: usize = 300;

/// What a report says happened. The launcher picks one; any other is refused.
pub const KINDS: &[&str] = &[
    // The game's own network code gave up on a connection (its EasyMultiPlay errors, #65).
    "network",
    // The helper's relay between the game and the server failed on one side.
    "relay",
    // The helper stopped hearing from the server and closed the game's connections.
    "server-lost",
    // The game or its injector exited with an error.
    "game-error",
    // The player's Report a problem.
    "report",
];

/// One report as kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    /// When it came, in Unix seconds.
    pub at: i64,
    pub user_id: u32,
    /// The account's User ID, or its players' names for a key account.
    pub account: String,
    pub kind: String,
    /// The launcher's version.
    pub launcher: String,
    /// One line on what happened, from the launcher.
    pub summary: String,
    /// The launcher's session log (the helper's, with the game's and the connection DLL's lines), credentials scrubbed.
    pub logs: String,
    /// The server's own lines about this account from the minutes before.
    pub server_log: Vec<String>,
}

/// A report in a list: everything but its logs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listed { pub id: String, pub at: i64, pub user_id: u32, pub account: String, pub kind: String, pub launcher: String, pub summary: String,
    /// The size of its logs in bytes.
    pub bytes: u64 }

/// The reports' folder, and how many each account sent today.
pub struct Problems { dir: Option<PathBuf>, sent: Mutex<HashMap<u32, Vec<i64>>> }

impl Problems {
    /// Reports in `data_dir/reports/` (none kept without a data folder: they are refused as off).
    pub fn new(data_dir: Option<&Path>) -> Problems { Problems { dir: data_dir.map(|d| d.join("reports")), sent: Mutex::default() } }

    /// Files `request` (the launcher's JSON, inflated) under `user_id`; its new id.
    pub fn accept(&self, user_id: u32, account: &str, request: &Json, server_log: Vec<String>, now: i64) -> Result<String, ReportRefused> {
        let dir = self.dir.as_ref().ok_or(ReportRefused::Off)?;
        let kind = request["kind"].as_str().filter(|k| KINDS.contains(k)).ok_or(ReportRefused::Invalid)?;
        let text = |field: &str, max: usize| clip(&yo3_tunnel::diagnostics::redact_secrets(request[field].as_str().unwrap_or_default(), &[]), max);
        let logs = request["logs"].as_str().unwrap_or_default().lines()
            .map(|line| yo3_tunnel::diagnostics::redact_secrets(line, &[])).collect::<Vec<_>>().join("\n");
        {
            let mut sent = self.sent.lock().unwrap_or_else(|e| e.into_inner());
            let times = sent.entry(user_id).or_default();
            times.retain(|t| now - t < 86_400);
            if times.len() >= PER_DAY { return Err(ReportRefused::Limit) }
            times.push(now);
        }
        let report = Report { id: new_id(), at: now, user_id, account: clip(account, 64), kind: kind.into(), launcher: text("launcher", 32),
            summary: text("summary", 300), logs, server_log };
        std::fs::create_dir_all(dir).map_err(|_| ReportRefused::Off)?;
        let body = serde_json::to_vec(&report).map_err(|_| ReportRefused::Invalid)?;
        let temporary = dir.join(format!(".{}.tmp", report.id));
        std::fs::write(&temporary, &body).and_then(|_| std::fs::rename(&temporary, dir.join(file_name(&report.listed())))).map_err(|_| ReportRefused::Off)?;
        self.prune(now);
        Ok(report.id)
    }

    /// The newest reports first, of `user_id` only when given.
    pub fn list(&self, user_id: Option<u32>, limit: usize) -> Vec<Listed> {
        let mut found: Vec<Listed> = self.files().into_iter().filter_map(|(path, bytes)| parse_name(&path, bytes))
            .filter(|l| user_id.is_none_or(|u| l.user_id == u)).collect();
        found.sort_by(|a, b| b.at.cmp(&a.at).then(b.id.cmp(&a.id)));
        found.truncate(limit);
        // The summary and account are in the file only: read for the few listed.
        for listed in &mut found { if let Some(report) = self.get(&listed.id) { *listed = report.listed() } }
        found
    }

    /// Report `id` (`R-XXXX-XXXX`, in any case).
    pub fn get(&self, id: &str) -> Option<Report> {
        let id = id.trim().to_ascii_uppercase();
        let (path, _) = self.files().into_iter().find(|(p, _)| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(&format!("_{id}.json"))))?;
        let mut text = String::new();
        std::fs::File::open(path).ok()?.take(4 * yo3_tunnel::MAX_PROBLEM_PLAIN as u64).read_to_string(&mut text).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn files(&self) -> Vec<(PathBuf, u64)> {
        let Some(dir) = &self.dir else { return vec![] };
        std::fs::read_dir(dir).into_iter().flatten().flatten()
            .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(".json") && !n.starts_with('.')))
            .map(|e| (e.path(), e.metadata().map_or(0, |m| m.len()))).collect()
    }

    /// Removes the oldest reports past the count, the age and the total size kept.
    fn prune(&self, now: i64) {
        let mut files: Vec<(PathBuf, u64, i64)> = self.files().into_iter()
            .filter_map(|(path, bytes)| parse_name(&path, bytes).map(|l| (path, bytes, l.at))).collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.2));
        let mut total = 0;
        for (i, (path, bytes, at)) in files.iter().enumerate() {
            total += bytes;
            if i >= KEEP_FILES || now - at > KEEP_DAYS * 86_400 || total > KEEP_BYTES { let _ = std::fs::remove_file(path); }
        }
    }
}

impl Report {
    pub fn listed(&self) -> Listed {
        Listed { id: self.id.clone(), at: self.at, user_id: self.user_id, account: self.account.clone(), kind: self.kind.clone(),
            launcher: self.launcher.clone(), summary: self.summary.clone(), bytes: self.logs.len() as u64 }
    }

    /// The report as one text file: its details, the server's lines, then the launcher's logs.
    pub fn text(&self) -> String {
        let when = chrono::DateTime::from_timestamp(self.at, 0).map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string()).unwrap_or_default();
        format!("Problem report {}\nWhen: {when}\nAccount: {} (user {})\nKind: {}\nLauncher: {}\nSummary: {}\n\n== The server's log about this account ==\n{}\n\n== The launcher's log ==\n{}\n",
            self.id, self.account, self.user_id, self.kind, self.launcher, self.summary, self.server_log.join("\n"), self.readable_logs())
    }

    /// The launcher's log for a person: its yo3-log/1 lines as `time LEVEL src message key=value`, older lines as they
    /// are (docs/logs.md). The stored report keeps the lines as sent, for `tools/logs/yo3log.py`.
    pub fn readable_logs(&self) -> String {
        self.logs.lines().map(yo3_tunnel::diagnostics::human).collect::<Vec<_>>().join("\n")
    }
}

/// Inflates operation 7's body: at most [`yo3_tunnel::MAX_PROBLEM_PLAIN`] bytes of a JSON object.
pub fn inflate(body: &[u8]) -> Result<Json, ReportRefused> {
    let mut plain = Vec::new();
    flate2::read::DeflateDecoder::new(body).take(yo3_tunnel::MAX_PROBLEM_PLAIN as u64 + 1).read_to_end(&mut plain).map_err(|_| ReportRefused::Invalid)?;
    if plain.len() > yo3_tunnel::MAX_PROBLEM_PLAIN { return Err(ReportRefused::Invalid) }
    serde_json::from_slice::<Json>(&plain).ok().filter(Json::is_object).ok_or(ReportRefused::Invalid)
}

/// The server's newest lines about `user_id` (its `user N` lines, and those naming one of `names`), oldest first.
pub fn server_lines(user_id: u32, names: &[String], now: i64) -> Vec<String> {
    // The account as a field (`user=1001`, yo3-log's console form) or in a message's words (`user 1001 `).
    let (tag, field) = (format!("user {user_id} "), format!("user={user_id}"));
    let mut lines: Vec<String> = crate::recent::lines(crate::recent::KEEP, false).into_iter()
        .take_while(|l| now - l.at <= SERVER_WINDOW)
        .filter(|l| l.text.contains(&tag) || l.text.split_whitespace().any(|w| w == field)
            || names.iter().any(|n| !n.is_empty() && l.text.contains(n.as_str())))
        .take(SERVER_LINES).map(|l| l.text).collect();
    lines.reverse();
    lines
}

/// A file per report, named so a list needs no reading: `<at>_<user>_<kind>_<id>.json`.
fn file_name(l: &Listed) -> String { format!("{}_{}_{}_{}.json", l.at, l.user_id, l.kind, l.id) }

fn parse_name(path: &Path, bytes: u64) -> Option<Listed> {
    let name = path.file_name()?.to_str()?.strip_suffix(".json")?;
    let mut parts = name.splitn(4, '_');
    let (at, user_id, kind, id) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?, parts.next()?);
    Some(Listed { id: id.into(), at, user_id, account: String::new(), kind: kind.into(), launcher: String::new(), summary: String::new(), bytes })
}

/// `R-XXXX-XXXX` in Crockford's base 32 (no I, L, O or U: read aloud and typed without mistakes).
fn new_id() -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).expect("OS randomness unavailable");
    let digit: Vec<char> = random.iter().map(|b| DIGITS[(*b & 31) as usize] as char).collect();
    format!("R-{}-{}", digit[..4].iter().collect::<String>(), digit[4..].iter().collect::<String>())
}

fn clip(text: &str, max: usize) -> String { text.lines().next().unwrap_or_default().chars().take(max).collect() }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn problems(test: &str) -> (Problems, PathBuf) {
        let dir = std::env::temp_dir().join(format!("yo3-problems-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Problems::new(Some(&dir)), dir)
    }

    #[test]
    fn a_report_is_kept_with_its_logs_and_found_by_its_id_in_any_case() {
        let (p, _) = problems("kept");
        let id = p.accept(7, "Yugi", &json!({ "kind": "network", "summary": "The game dropped its front connection", "launcher": "0.1.10",
            "logs": "1 [YO3 MOD] Game network: EasyMultiPlay PacketBase::recv_nonblock 2067" }), vec!["[tls] user 7 Front ended".into()], 1_000).unwrap();
        assert!(id.len() == 11 && id.starts_with("R-"), "{id}");
        let report = p.get(&id.to_lowercase()).unwrap();
        assert_eq!((report.user_id, report.kind.as_str(), report.account.as_str()), (7, "network", "Yugi"));
        assert!(report.logs.contains("recv_nonblock 2067"));
        assert!(report.text().contains("== The server's log about this account ==\n[tls] user 7 Front ended"));
        let listed = p.list(None, 10);
        assert_eq!((listed.len(), listed[0].summary.as_str()), (1, "The game dropped its front connection"));
        assert!(p.list(Some(8), 10).is_empty(), "another account's");
    }

    #[test]
    fn an_unknown_kind_is_refused_and_an_account_sends_twenty_a_day() {
        let (p, _) = problems("limit");
        assert_eq!(p.accept(1, "A", &json!({ "kind": "../../etc" }), vec![], 0), Err(ReportRefused::Invalid));
        for i in 0..PER_DAY { p.accept(1, "A", &json!({ "kind": "report" }), vec![], i as i64).unwrap(); }
        assert_eq!(p.accept(1, "A", &json!({ "kind": "report" }), vec![], 100), Err(ReportRefused::Limit));
        p.accept(2, "B", &json!({ "kind": "report" }), vec![], 100).expect("another account has its own");
        p.accept(1, "A", &json!({ "kind": "report" }), vec![], 86_400 + 1).expect("a day later");
    }

    #[test]
    fn credentials_in_a_report_are_scrubbed_again_on_the_server() {
        let (p, _) = problems("scrub");
        let token = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2";
        let id = p.accept(1, "A", &json!({ "kind": "relay", "logs": format!("session token {token}") }), vec![], 0).unwrap();
        assert!(!p.get(&id).unwrap().logs.contains(token));
    }

    #[test]
    fn old_reports_go_first_past_the_age_kept() {
        let (p, _) = problems("prune");
        let old = p.accept(1, "A", &json!({ "kind": "report" }), vec![], 0).unwrap();
        let new = p.accept(1, "A", &json!({ "kind": "report" }), vec![], KEEP_DAYS * 86_400 + 10).unwrap();
        assert!(p.get(&old).is_none() && p.get(&new).is_some());
    }

    #[test]
    fn a_body_that_inflates_past_the_limit_or_is_not_an_object_is_refused() {
        use std::io::Write;
        let pack = |bytes: &[u8]| { let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default()); e.write_all(bytes).unwrap(); e.finish().unwrap() };
        assert!(inflate(&pack(br#"{"kind":"report"}"#)).is_ok());
        assert_eq!(inflate(&pack(b"[1,2]")), Err(ReportRefused::Invalid));
        let bomb = format!(r#"{{"logs":"{}"}}"#, "a".repeat(yo3_tunnel::MAX_PROBLEM_PLAIN));
        assert_eq!(inflate(&pack(bomb.as_bytes())), Err(ReportRefused::Invalid));
        assert_eq!(inflate(b"not deflate"), Err(ReportRefused::Invalid));
    }
}
