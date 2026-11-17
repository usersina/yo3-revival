//! Deleting an account (security review #39, `delete-account`): the account, its players and everything kept for them
//! go, its sessions and devices end, and the audit log keeps only who deleted which User ID and when.
mod common;

use std::sync::{Arc, Mutex};

use common::{Server, one, req, req_with};
use serde_json::json;
use yo3_proto::Value;
use yo3_server::accounts::Directory;
use yo3_server::api::{self, Op};
use yo3_server::logins::{self, Device, Login, Registration};
use yo3_server::{Config, start};
use yo3_tunnel::{Endpoint, Role, Token};

/// Two password accounts with a player each: (directory, yugi's user id, yugi's player, kaiba's user id, kaiba's player).
fn directory() -> (Arc<Mutex<Directory>>, u32, u32, u32, u32) {
    let mut d = Directory::new(vec![]);
    let mut make = |user: &str, player: &str| {
        let id = d.register(Login::new(user, String::new(), String::new()), None, Registration::Open).unwrap();
        d.add_player_to(id, player).unwrap();
        let pid = d.account_of(id).unwrap().players[0].player_id;
        (id, pid)
    };
    let ((yugi, yp), (kaiba, kp)) = (make("yugi", "Yugi"), make("kaiba", "Kaiba"));
    (Arc::new(Mutex::new(d)), yugi, yp, kaiba, kp)
}

fn u(n: u32) -> Value { Value::U32(n) }

#[test]
fn deleting_an_account_removes_its_players_and_everything_kept_for_them() {
    let (d, yugi, yp, kaiba, kp) = directory();
    d.lock().unwrap().add_device(yugi, Device { token: "d".repeat(64), label: "PC".into(), created: logins::now(), last_used: logins::now(), expires: logins::now() + 3600 }).unwrap();
    let s = Server::new();
    s.hub.lock().unwrap().set_directory(d.clone());
    let (mut a, mut b) = (s.join_saved(&d, yugi), s.join_saved(&d, kaiba));
    // Friends, mail both ways, a team Yugi leads with Kaiba in it, and a duel record each way.
    one(a.send(req_with(0x24B9, &[("to_id", u(kp))])));
    one(b.send(req_with(0x24BB, &[("to_id", u(yp)), ("answer", u(2))])));
    let mail = |c: &mut common::Client, to: u32| one(c.send(req_with(0x2580, &[("mail_type", u(0)), ("title", Value::text("hi")),
        ("body", Value::text("hello")), ("to_id", u(to))])));
    assert_eq!(mail(&mut a, kp).request(), Some(0x2584));
    assert_eq!(mail(&mut b, yp).request(), Some(0x2584));
    one(a.send(req_with(0x2AF9, &[("team_name", Value::text("Millennium"))])));
    let team = one(a.send(req(0x2AFB))).get("team_id").and_then(Value::as_u32).unwrap();
    a.send(req_with(0x2AFE, &[("to_id", u(kp))]));
    b.send(req_with(0x2B00, &[("to_id", u(yp)), ("team_id", u(team)), ("answer", u(2))]));
    s.hub.lock().unwrap().record_result(yp, kp, 1);
    s.hub.lock().unwrap().record_result(kp, yp, 2);
    a.pushed();
    b.pushed();

    let settings = Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let mut api = api::Api::new(d.clone(), settings, None, std::path::PathBuf::new(), "server");
    api.hub = Some(s.hub.clone());
    // A running server supplies its session manager even if this fixture has no authenticated sockets.
    api.sessions = Some(Arc::new(Mutex::new(yo3_server::auth::Sessions::default())));
    api.apply(&Op::GiveBp { who: "Yugi".into(), amount: 10 }, "host").unwrap();
    api.apply(&Op::Kick { who: "kaiba".into() }, "host").unwrap();
    // Only an admin deletes: a moderator is refused.
    assert!(api.apply_as(&Op::DeleteAccount { who: "yugi".into() }, "Anna", "panel", Some(logins::Role::Moderator)).is_err());
    let done = api.apply(&Op::DeleteAccount { who: "Yugi".into() }, "host").unwrap();
    assert_eq!(done["deleted"], json!("yugi"), "{done}");

    let dir = d.lock().unwrap();
    assert!(dir.account_of(yugi).is_none() && dir.login_of("yugi").is_none() && dir.user_of("Yugi").is_none(), "account, User ID, player");
    assert!(dir.player(yp).is_none());
    assert_eq!(dir.device_user(&"d".repeat(64), logins::now()), None, "its devices");
    assert!(dir.account_of(kaiba).is_some(), "the other account stays");
    drop(dir);
    let hub = s.hub.lock().unwrap();
    assert_eq!(hub.friends_of(kp), vec![], "gone from Kaiba's friends");
    assert_eq!(hub.record_totals(yp), [0; 3], "its records");
    drop(hub);
    assert_eq!(one(b.send(req(0x2583))).get("count"), Some(&Value::U32(0)), "the mail it sent Kaiba");
    assert_eq!(one(b.send(req(0x2AFB))).request(), Some(0x2B14), "the team it led is disbanded");
    assert!(b.pushed().iter().any(|m| m.request() == Some(0x2B07)), "its members are told");
    // The audit log: the deletion alone names the account, by User ID, with no other values.
    let rows = api.audit(50).unwrap();
    let row = &rows[0];
    assert_eq!((row.actor.as_str(), row.op.as_str(), row.target.as_str()), ("host", "delete-account", "yugi"));
    assert!(row.before.is_none() && row.after.is_none() && row.detail.is_none(), "{row:?}");
    assert!(rows[1..].iter().all(|r| !r.target.eq_ignore_ascii_case("yugi")), "earlier rows about it go: {rows:?}");
    assert!(rows.iter().any(|r| r.op == "kick" && r.target == "kaiba"), "rows about others stay");
    assert!(api.apply(&Op::DeleteAccount { who: "Yugi".into() }, "host").is_err(), "nothing left to delete");
}

struct Live { endpoint: Endpoint, admin: yo3_server::admin::Live, data: std::path::PathBuf, _running: yo3_server::Running }

async fn server(test: &str) -> Live {
    logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-deletion-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config::ephemeral(dir);
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap();
    let admin = yo3_server::admin::running(&data).unwrap();
    Live { endpoint, admin, data, _running: running }
}

fn session(reply: &serde_json::Value) -> Token {
    let hex = reply["session"].as_str().unwrap_or_else(|| panic!("no session in {reply}"));
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

#[tokio::test]
async fn a_deleted_account_is_signed_out_everywhere_and_its_user_id_signs_in_no_more() {
    let s = server("live").await;
    let account = |r: serde_json::Value| { let e = s.endpoint.clone(); async move { e.account(&r).await.unwrap() } };
    assert_eq!(account(json!({ "op": "register", "user": "Yugi", "password": "LongPassphrase1", "rules": true })).await["ok"], true);
    let signed = account(json!({ "op": "login", "user": "Yugi", "password": "LongPassphrase1", "stay": true })).await;
    let (game, token) = (session(&signed), signed["device_token"].clone());
    let admin = s.admin.clone();
    let done = tokio::task::spawn_blocking(move || api::call_as(&admin, &Op::DeleteAccount { who: "yugi".into() }, "host")).await.unwrap();
    assert!(done.is_ok(), "{done:?}");
    assert!(s.endpoint.ticket(&game, Role::Front).await.is_err(), "the game's session ended");
    assert_ne!(account(json!({ "op": "resume", "device_token": token })).await["ok"], true, "the device is signed out");
    assert_ne!(account(json!({ "op": "login", "user": "Yugi", "password": "LongPassphrase1" })).await["ok"], true, "no such User ID");
    let rows = yo3_server::store::audit_rows(&s.data, 10).unwrap();
    assert_eq!((rows[0].op.as_str(), rows[0].target.as_str()), ("delete-account", "Yugi"), "the User ID as it was made");
}

#[test]
fn the_cli_deletes_an_account_of_a_stopped_server_only_when_told_yes() {
    let dir = std::env::temp_dir().join(format!("yo3-deletion-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cli = |args: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).args(args).arg("--data-dir").arg(&dir).output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
    };
    assert!(cli(&["add-account", "Yugi"]).0);
    assert!(cli(&["add-account", "Kaiba"]).0);
    let (ok, out) = cli(&["delete-account", "Yugi"]);
    assert!(!ok && out.contains("--yes"), "{out}");
    let (ok, out) = cli(&["delete-account", "Yugi", "--yes"]);
    assert!(ok, "{out}");
    let (_, accounts) = cli(&["accounts"]);
    assert!(!accounts.contains("Yugi") && accounts.contains("Kaiba"), "{accounts}");
    let (_, audit) = cli(&["audit"]);
    assert!(audit.contains("delete-account") && !audit.contains("Yugi"), "a key account is named by its number: {audit}");
    let _ = std::fs::remove_dir_all(&dir);
}
