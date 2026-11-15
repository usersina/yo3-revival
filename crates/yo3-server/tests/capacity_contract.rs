//! Capacity (#27): a server at its caps says so, and never hangs. The caps are live settings: `max-players` (signed
//! in), `max-duels` (running and matched duels, an engine each), `max-connections` and the per-address handshakes.
mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{Client, Server, one, req_with};
use serde_json::json;
use yo3_proto::{Message, Value};
use yo3_server::{Config, start};

/// 0x1784 {event_id 2} then 0x1798 (Auto Duel): the replies.
fn auto_match(c: &mut Client) -> Vec<Message> {
    let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
    let join_id = join.get("join_id").cloned().unwrap();
    c.send(req_with(0x1798, &[("event_id", Value::U32(2)), ("join_id", join_id), ("match_type", Value::U32(1)), ("level", Value::U32(0))]))
}

fn matched(c: &mut Client) -> bool {
    std::thread::sleep(Duration::from_millis(400));
    c.pushed().iter().any(|m| m.request() == Some(0x1978))
}

/// A server whose hub counts at most `duels` duels.
fn capped(duels: u32) -> Server {
    let server = Server::new();
    let live = yo3_server::settings::Live::open(None, vec![]).unwrap();
    live.set("max-duels", &duels.to_string()).unwrap();
    server.hub.lock().unwrap().set_settings(Arc::new(live));
    server
}

#[test]
fn at_the_duel_cap_auto_duel_waits_and_is_matched_when_a_duel_ends() {
    let server = capped(1);
    let (mut a, mut b, mut c, mut d) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"), server.join(1004, "Mai"));
    auto_match(&mut a);
    auto_match(&mut b);
    assert!(matched(&mut a) || matched(&mut b), "the first pair: one duel, the cap");
    // A matched duel counts before its engine starts: the next pair waits, as players with no opponent do.
    assert_eq!(auto_match(&mut c).iter().map(Message::request).collect::<Vec<_>>(), [Some(0x1799)]);
    auto_match(&mut d);
    assert!(!matched(&mut c) && !matched(&mut d));
    // The first duel ends: the clock's next pass pairs them.
    let first = server.hub.lock().unwrap().match_ids()[0];
    server.hub.lock().unwrap().end_match(first, None);
    server.hub.lock().unwrap().match_waiting();
    assert!(matched(&mut c) && matched(&mut d));
}

#[test]
fn at_the_duel_cap_an_npc_duel_is_refused_with_the_reason_in_chat() {
    // 0x17AE (agent step 0x4A1790) shows the game's text 10508 "A transmission error has occurred." whatever its
    // code, and goes back to the map: the reason comes first, in the player's chat window.
    let server = capped(1);
    let (mut a, mut b, mut c) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    auto_match(&mut a);
    auto_match(&mut b);
    let rule = Message::record().with("cpu_id", Value::U32(124)).with("match_type", Value::U32(4));
    let npc = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(rule);
    let replies = c.send(yo3_proto::Packet::Message(npc.clone()));
    assert_eq!(one(replies).request(), Some(0x17AE));
    let note = c.pushed().into_iter().find(|m| m.request() == Some(0x2455)).expect("a chat line");
    assert_eq!(note.items[0].get("chat_str"), Some(&Value::text("Duels are busy: try again in a moment.")));
    // Below the cap it goes ahead.
    let first = server.hub.lock().unwrap().match_ids()[0];
    server.hub.lock().unwrap().end_match(first, None);
    assert_eq!(one(c.send(yo3_proto::Packet::Message(npc))).request(), Some(0x17AD));
}

#[tokio::test]
async fn a_full_server_says_so_and_lets_the_staff_and_a_second_sign_in_through() {
    yo3_server::logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-capacity-full-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = Config::ephemeral(dir);
    cfg.settings.push(("max-players".into(), "1".into()));
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    let e = yo3_tunnel::Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap();
    let ask = |request: serde_json::Value| { let e = e.clone(); async move { e.account(&request).await.unwrap() } };
    for user in ["Yugi", "Kaiba", "Pegasus"] {
        assert_eq!(ask(json!({ "op": "register", "user": user, "password": "a long passphrase", "rules": true })).await["ok"], true);
    }
    let live = yo3_server::admin::running(&data).unwrap();
    tokio::task::spawn_blocking(move || yo3_server::api::call(&live, &yo3_server::api::Op::SetRole { user: "Pegasus".into(), role: "moderator".into() }))
        .await.unwrap().unwrap();
    let login = |user: &str| ask(json!({ "op": "login", "user": user, "password": "a long passphrase" }));
    assert_eq!(login("Yugi").await["ok"], true);
    assert_eq!(login("Kaiba").await["error"], "server_full", "the launcher shows \"The server is full\" with Retry");
    assert_eq!(login("Yugi").await["ok"], true, "signing in again takes no other place");
    assert_eq!(login("Pegasus").await["ok"], true, "the staff comes in");
    // A key account is refused the same way.
    let key = std::fs::read_to_string(running_key_dir(&certificate)).unwrap();
    assert!(e.login(&key).await.is_err(), "full");
    assert_eq!(e.info().await.unwrap().max_online, Some(1), "the server list shows the cap");
}

/// The test account's key, beside the test certificate (`auth::test_config`).
fn running_key_dir(certificate: &std::path::Path) -> std::path::PathBuf { certificate.with_file_name("client.key") }
