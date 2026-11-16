//! The server's newest log lines, kept in memory for the admin panel's Logs page and its dashboard (#36).
//!
//! The server's log writer copies each finished line here ([`Tee`]). Lines are filtered as the diagnostic export is:
//! a line that may hold a credential is replaced, long opaque tokens are masked, and the verbose traces (packets,
//! engine recordings) stay on disk only. The full log is still the diagnostics session in the log folder.
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::Mutex;

use serde::Serialize;

/// How many lines are kept.
pub const KEEP: usize = 2000;

/// One log line: its level word (ERROR, WARN, INFO, DEBUG) and its text as written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Line { pub level: &'static str, pub text: String, #[serde(skip)] pub at: i64 }

static LINES: Mutex<VecDeque<Line>> = Mutex::new(VecDeque::new());

/// Keeps one finished line (without its newline).
pub fn push(text: &str) {
    let text = text.trim_end();
    if text.is_empty() || yo3_tunnel::diagnostics::is_trace(text) { return }
    let level = match yo3_tunnel::diagnostics::level(text) {
        Some(yo3_tunnel::diagnostics::Level::Error) => "ERROR",
        Some(yo3_tunnel::diagnostics::Level::Warn) => "WARN",
        Some(yo3_tunnel::diagnostics::Level::Debug) => "DEBUG",
        _ => "INFO",
    };
    let text = yo3_tunnel::diagnostics::redact_secrets(text, &[]);
    let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    if lines.len() == KEEP { lines.pop_front(); }
    lines.push_back(Line { level, text, at: chrono::Utc::now().timestamp() });
}

/// The newest `limit` lines, newest first; with `problems`, only warnings and errors.
pub fn lines(limit: usize, problems: bool) -> Vec<Line> {
    let lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    lines.iter().rev().filter(|l| !problems || matches!(l.level, "ERROR" | "WARN")).take(limit).cloned().collect()
}

/// How many errors were logged in the last `seconds`.
pub fn errors_within(seconds: i64) -> usize {
    let since = chrono::Utc::now().timestamp() - seconds;
    LINES.lock().unwrap_or_else(|e| e.into_inner()).iter().rev().take_while(|l| l.at >= since).filter(|l| l.level == "ERROR").count()
}

/// A log writer that also keeps each finished line here.
pub struct Tee<W> { inner: W, line: Vec<u8> }

impl<W> Tee<W> {
    pub fn new(inner: W) -> Tee<W> { Tee { inner, line: Vec::new() } }
}

impl<W: Write> Write for Tee<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for &b in bytes {
            if b == b'\n' { push(&String::from_utf8_lossy(&self.line)); self.line.clear(); }
            else if self.line.len() < 4096 { self.line.push(b) }
        }
        self.inner.write_all(bytes)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

impl<W> Drop for Tee<W> {
    fn drop(&mut self) { if !self.line.is_empty() { push(&String::from_utf8_lossy(&self.line)) } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_kept_newest_first_without_traces_or_credentials() {
        let mut tee = Tee::new(io::sink());
        write!(tee, "2026-11-16T10:00:00Z  INFO recent-test one\n2026-11-16T10:00:01Z ERROR recent-test two\n").unwrap();
        writeln!(tee, "2026-11-16T10:00:02Z  INFO [YO3 ENGINE] recent-test trace").unwrap();
        writeln!(tee, "2026-11-16T10:00:03Z  WARN recent-test the password was x").unwrap();
        write!(tee, "2026-11-16T10:00:04Z  INFO recent-test key 0123456789abcdef0123456789abcdef").unwrap();
        drop(tee);
        let ours: Vec<Line> = lines(KEEP, false).into_iter()
            .filter(|l| l.text.contains("recent-test") || l.text.contains("sensitive")).take(4).collect();
        assert_eq!(ours.len(), 4, "{ours:?}");
        assert!(ours[0].text.ends_with("recent-test key [redacted]"), "{:?}", ours[0]);
        assert_eq!(ours[1].text, "[sensitive diagnostic line omitted]");
        assert_eq!((ours[2].level, ours[3].level), ("ERROR", "INFO"));
        assert!(ours.iter().all(|l| !l.text.contains("trace")));
        assert!(lines(KEEP, true).iter().all(|l| matches!(l.level, "ERROR" | "WARN")));
    }
}
