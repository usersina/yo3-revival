//! Problem reports from the helper (#65's follow-up): when a game ends after something broke, its session log goes to
//! the server it played on (TLS operation 7, `yo3_server::problems`), so the host has the player's side of a disconnect
//! without asking them to attach anything. Nothing is sent after a game that ended well, or when the player turned it
//! off (Desktop's setting writes [`OFF_FILE`] beside the profile; Scripts: the same file or `--no-problem-reports`).
use std::path::Path;
use std::time::Duration;

use yo3_tunnel::{Endpoint, Token};

/// A file beside the helper's profile that turns problem reports off.
pub const OFF_FILE: &str = "problem-reports.off";
/// How much of the session log a report carries: its newest bytes. Deflated, it stays well under the server's limit.
pub const LOG_BUDGET: usize = 768 * 1024;
/// How long sending may hold the end of the helper.
pub const SEND_WAIT: Duration = Duration::from_secs(15);

/// Whether reports are off for the helper whose profile is `profile`.
pub fn off(profile: &Path) -> bool { profile.parent().is_some_and(|dir| dir.join(OFF_FILE).exists()) }

/// What broke, from the session log and how the game ended: the report's kind and its one-line summary. None: nothing
/// to report.
pub fn incident(log: &str, failure: Option<&str>) -> Option<(&'static str, String)> {
    let last = |test: &dyn Fn(&str) -> bool| log.lines().rev().find(|l| test(l)).map(|l| strip_time(l).to_string());
    if let Some(failure) = failure { return Some(("game-error", failure.lines().next().unwrap_or("The game exited with an error").to_string())) }
    if let Some(line) = last(&|l| l.contains("Front connection: ") || l.contains("Duel connection: ")) { return Some(("relay", line)) }
    if let Some(line) = last(&|l| l.contains("closing the game's connections")) { return Some(("server-lost", line)) }
    // The game's own network code (the connection DLL's EasyMultiPlay lines, #65); waiting for a frame's rest is normal.
    if let Some(line) = last(&|l| l.contains("Game network: ") && !l.contains("waiting for the rest")) { return Some(("network", line)) }
    None
}

/// The report's JSON.
pub fn report(kind: &str, summary: &str, logs: &str) -> serde_json::Value {
    serde_json::json!({ "kind": kind, "summary": summary, "launcher": yo3_tunnel::BUILD_VERSION, "logs": logs, "os": std::env::consts::OS })
}

/// Sends a report of what broke, if anything did, under `session`; logs the outcome in the session log.
pub async fn send_if_broken(endpoint: &Endpoint, session: &Token, log: &yo3_tunnel::diagnostics::Session, failure: Option<&str>) {
    let mut text = log.tail(LOG_BUDGET);
    let Some((kind, summary)) = incident(&text, failure) else { return };
    if let Some(failure) = failure { text.push_str(&format!("\n(the helper's error) {failure}")); }
    match tokio::time::timeout(SEND_WAIT, endpoint.problem_report(session, &report(kind, &summary, &text))).await {
        Ok(Ok(id)) => log.line(&format!("Problem report {id} sent to the server ({kind}): quote it when you report this problem")),
        Ok(Err(error)) => log.line(&format!("Problem report not sent: {error:#}")),
        Err(_) => log.line("Problem report not sent: the server didn't answer in time"),
    }
}

/// A session log line without its leading Unix time.
fn strip_time(line: &str) -> &str {
    match line.split_once(' ') { Some((time, rest)) if time.bytes().all(|b| b.is_ascii_digit()) => rest, _ => line }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_that_ended_well_sends_nothing() {
        let log = "1 Authenticated; game resumed\n2 [YO3 MOD] Game network: EasyMultiPlay recv 2106, waiting for the rest of a frame (at 0x535227)\n3 Game/helper session ended";
        assert_eq!(incident(log, None), None);
    }

    #[test]
    fn what_broke_is_named_by_its_strongest_evidence() {
        let network = "1 [YO3 MOD] Game network: EasyMultiPlay PacketBase::recv_nonblock 2067 (at 0x535227)\n2 [YO3 MOD] The game closed its front connection";
        assert_eq!(incident(network, None), Some(("network", "[YO3 MOD] Game network: EasyMultiPlay PacketBase::recv_nonblock 2067 (at 0x535227)".into())));
        let relay = format!("{network}\n3 Front connection: the game's connection failed while reading");
        assert_eq!(incident(&relay, None).unwrap().0, "relay");
        let lost = "1 the server did not answer 3 checks in a row; closing the game's connections";
        assert_eq!(incident(lost, None).unwrap().0, "server-lost");
        assert_eq!(incident(&relay, Some("Game or injector exited with an error (exit code: 3)\ndetail")).unwrap(),
            ("game-error", "Game or injector exited with an error (exit code: 3)".to_string()));
    }

    #[test]
    fn reports_are_off_with_the_file_beside_the_profile() {
        let dir = std::env::temp_dir().join(format!("yo3-problems-off-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let profile = dir.join("connection-0.json");
        assert!(!off(&profile));
        std::fs::write(dir.join(OFF_FILE), "").unwrap();
        assert!(off(&profile));
        let _ = std::fs::remove_dir_all(dir);
    }
}
