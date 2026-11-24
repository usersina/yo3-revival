//! TLS operation 7, problem reports: a launcher sends a game's logs under its session, and the host reads them with
//! the server's own lines about that account beside them (problems.rs).
use serde_json::json;
use yo3_server::logins;
use yo3_server::problems::Problems;
use yo3_server::{Config, admin, api::{self, Op}, start};
use yo3_tunnel::{Endpoint, ReportRefused};

async fn server(test: &str, settings: &[(&str, &str)]) -> (Endpoint, std::path::PathBuf, yo3_server::Running) {
    logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-problem-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = Config::ephemeral(dir);
    cfg.settings.push(("registration".into(), "open".into()));
    cfg.settings.extend(settings.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    (Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap(), data, running)
}

async fn signed_in(endpoint: &Endpoint) -> yo3_tunnel::Token {
    let pass = "CorrectHorse42";
    endpoint.account(&json!({ "op": "register", "user": "Yugi", "password": pass, "rules": true })).await.unwrap();
    let reply = endpoint.account(&json!({ "op": "login", "user": "Yugi", "password": pass })).await.unwrap();
    let hex = reply["session"].as_str().unwrap();
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

fn refusal(error: anyhow::Error) -> ReportRefused { *error.downcast_ref::<ReportRefused>().unwrap_or_else(|| panic!("not a refusal: {error:#}")) }

#[tokio::test]
async fn a_launchers_report_is_kept_under_its_account_with_the_servers_lines() {
    let (endpoint, data, _running) = server("kept", &[]).await;
    let session = signed_in(&endpoint).await;
    let logs = "1 [YO3 MOD] Game network: EasyMultiPlay PacketBase::recv_nonblock 2067 (at 0x535227)\n".repeat(2000);
    let id = endpoint.problem_report(&session, &json!({ "kind": "network", "summary": "The game dropped its front connection",
        "launcher": "0.1.10", "logs": logs })).await.unwrap();
    let report = Problems::new(Some(&data)).get(&id).unwrap();
    assert_eq!((report.account.as_str(), report.kind.as_str()), ("Yugi", "network"));
    assert_eq!(report.logs.lines().count(), 2000, "about 170 KB of logs, sent deflated");
    // The server's own lines about the account come with the next report (a test server has no log writer: one is fed).
    yo3_server::recent::push(&format!("INFO [tls] 127.0.0.1:5 user {} Front ended: closed by the game", report.user_id));
    yo3_server::recent::push("INFO [tls] 127.0.0.1:6 user 999999 Front ended: someone else");
    let id = endpoint.problem_report(&session, &json!({ "kind": "report" })).await.unwrap();
    let report = Problems::new(Some(&data)).get(&id).unwrap();
    assert!(report.server_log.iter().any(|l| l.contains("Front ended: closed by the game")), "{:?}", report.server_log);
    assert!(!report.server_log.iter().any(|l| l.contains("someone else")), "only this account's");
    // The host reads them by account and by the ID a player quotes (`problem-reports`, `problem-report ID`).
    let admin = |op: Op| { let live = admin::running(&data).unwrap(); tokio::task::spawn_blocking(move || api::call(&live, &op)) };
    let listed = admin(Op::ProblemReports { who: Some("yugi".into()), limit: None }).await.unwrap().unwrap();
    assert_eq!(listed["reports"].as_array().unwrap().len(), 2, "{listed}");
    assert!(listed["message"].as_str().unwrap().contains(&id));
    let one = admin(Op::ProblemReport { id: id.to_lowercase() }).await.unwrap().unwrap();
    assert!(one["message"].as_str().unwrap().contains("== The launcher's log =="));
    assert!(admin(Op::ProblemReport { id: "R-0000-0000".into() }).await.unwrap().is_err());
}

#[tokio::test]
async fn a_report_needs_a_live_session_and_the_hosts_setting() {
    let (endpoint, _, _running) = server("refused", &[]).await;
    let report = json!({ "kind": "report" });
    assert_eq!(refusal(endpoint.problem_report(&[7; 32], &report).await.unwrap_err()), ReportRefused::Session);
    let session = signed_in(&endpoint).await;
    assert_eq!(refusal(endpoint.problem_report(&session, &json!({ "kind": "anything" })).await.unwrap_err()), ReportRefused::Invalid);
    endpoint.logout(&session).await.unwrap();
    assert_eq!(refusal(endpoint.problem_report(&session, &report).await.unwrap_err()), ReportRefused::Session, "signed out");
    let (endpoint, data, _running) = server("off", &[("problem-reports", "off")]).await;
    let session = signed_in(&endpoint).await;
    assert_eq!(refusal(endpoint.problem_report(&session, &report).await.unwrap_err()), ReportRefused::Off);
    assert!(Problems::new(Some(&data)).list(None, 10).is_empty());
}

#[tokio::test]
async fn the_projects_server_takes_reports_without_an_account_when_its_host_allows_it() {
    // TLS operation 8: Report a problem from a launcher with no account here (Singleplayer, another server). Players
    // missed attaching their logs to GitHub, so the launcher sends them to the project's server instead. Off unless the
    // host turns on `anonymous-reports`; filed under no account (user 0, "anonymous"), without the server's lines, at
    // most ANONYMOUS_PER_ADDRESS a day from one address.
    let report = json!({ "kind": "report", "summary": "Black Brutdrago closed my game", "launcher": "0.1.22",
        "logs": "== What the player wrote ==\nBlack Brutdrago closed my game\n\n== How to reach them ==\nDiscord: duelist#1\n" });
    let (endpoint, _, _running) = server("anon-off", &[]).await;
    assert_eq!(refusal(endpoint.anonymous_report(&report).await.unwrap_err()), ReportRefused::Off, "off by default");
    let (endpoint, data, _running) = server("anon", &[("anonymous-reports", "on")]).await;
    let id = endpoint.anonymous_report(&report).await.unwrap();
    let kept = Problems::new(Some(&data)).get(&id).unwrap();
    assert_eq!((kept.user_id, kept.account.as_str(), kept.kind.as_str()), (0, "anonymous", "report"));
    assert!(kept.logs.contains("Discord: duelist#1") && kept.server_log.is_empty());
    assert_eq!(refusal(endpoint.anonymous_report(&json!({ "kind": "network" })).await.unwrap_err()), ReportRefused::Invalid,
        "only Report a problem comes without an account");
    for _ in 1..yo3_server::problems::ANONYMOUS_PER_ADDRESS { endpoint.anonymous_report(&report).await.unwrap(); }
    assert_eq!(refusal(endpoint.anonymous_report(&report).await.unwrap_err()), ReportRefused::Limit, "a day's reports from one address");
    // The `problem-reports` switch still turns every report off.
    let (endpoint, _, _running) = server("anon-all-off", &[("anonymous-reports", "on"), ("problem-reports", "off")]).await;
    assert_eq!(refusal(endpoint.anonymous_report(&report).await.unwrap_err()), ReportRefused::Off);
}
