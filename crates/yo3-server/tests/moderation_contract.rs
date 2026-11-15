//! Moderation and maintenance (#26): what the game shows (the restricted login, the muted chat, the countdown) and what
//! a moderator's action does on a running server (kick, ban, restrict, maintenance), each in the audit log by name.
mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{Server, one, req, req_with};
use serde_json::{Value as Json, json};
use yo3_proto::{Message, Packet, Value};
use yo3_server::accounts::Directory;
use yo3_server::api::{self, Op};
use yo3_server::logins::{self, Login, Mute, Registration, Standing};
use yo3_server::{Config, Running, start};
use yo3_tunnel::{Endpoint, Role, Token};

/// Two password accounts with a player each (Yugi 1001's account, Kaiba's), in memory.
fn directory() -> (Arc<Mutex<Directory>>, u32, u32) {
    let mut d = Directory::new(vec![]);
    let mut make = |user: &str, player: &str| {
        let id = d.register(Login::new(user, String::new(), String::new()), None, Registration::Open).unwrap();
        d.add_player_to(id, player).unwrap();
        id
    };
    let (yugi, kaiba) = (make("yugi", "Yugi"), make("kaiba", "Kaiba"));
    (Arc::new(Mutex::new(d)), yugi, kaiba)
}

#[test]
fn a_restricted_account_is_refused_with_the_games_own_text_at_its_next_login() {
    // LoginWindow 0x469C00: 0x219E opens the login error dialog with kind 6, text 10600 "This User ID is currently
    // restricted." (R2). The game already in plays on; the refusal comes at its next ONLINE.
    let (d, yugi, _) = directory();
    let s = Server::new();
    let mut c = s.join_saved(&d, yugi);
    d.lock().unwrap().set_standing_of(yugi, Standing::Restricted { reason: "spam".into(), until: None }).unwrap();
    assert_eq!(one(c.send(req(0x1F47))).request(), Some(0x219E));
    // An ended restriction lets it in again.
    d.lock().unwrap().set_standing_of(yugi, Standing::Restricted { reason: "spam".into(), until: Some(logins::now() - 1) }).unwrap();
    assert_eq!(one(c.send(req(0x1F47))).request(), Some(0x219A));
}

fn say(room: u32, text: &str) -> Packet {
    let body = Message { tag: 10, fields: vec![("chat_str".into(), Value::text(text)), ("chat_type".into(), Value::U32(1)), ("to_id".into(), Value::U32(room))], items: vec![] };
    Packet::Message(Message::reply(0x2134).item(body))
}

#[test]
fn a_muted_players_lines_go_nowhere_and_they_are_told_why() {
    // The game has no muted reply or text (R6): the line is dropped, and the sender's own chat window says why.
    let (d, yugi, kaiba) = directory();
    let s = Server::new();
    let (mut a, mut b) = (s.join_saved(&d, yugi), s.join_saved(&d, kaiba));
    for c in [&mut a, &mut b] { one(c.send(req_with(0x2136, &[("room_id", Value::U32(1))]))); }
    d.lock().unwrap().set_mute(yugi, Some(Mute { reason: "flooding".into(), until: None })).unwrap();
    let replies = a.send(say(1, "spam spam"));
    assert_eq!(replies[0].request(), Some(0x2456));
    let note = replies[1].items[0].get("chat_str").and_then(Value::as_str).unwrap().to_string();
    assert_eq!(note, "You are muted until a moderator lifts it: flooding");
    assert!(b.pushed().iter().all(|m| m.request() != Some(0x2455)), "nothing reached the room");
    d.lock().unwrap().set_mute(yugi, None).unwrap();
    a.send(say(1, "sorry"));
    assert!(b.pushed().iter().any(|m| m.request() == Some(0x2455)), "unmuted: the room hears it");
}

#[test]
fn the_information_list_carries_the_countdown_before_maintenance() {
    // An entry of the {lang} item with lefttime (seconds) and maintenance (a text, not "-1") is the maintenance notice:
    // InfoIcon 0x44E130 keeps the soonest one, and its icon's help 0x451700 writes the time left into the chat.
    // Without a planned maintenance no entry carries them.
    let live = yo3_server::settings::Live::open(None, vec![]).unwrap();
    let now = logins::now();
    let lang = |info: Message| info.items[0].clone();
    let calm = lang(yo3_server::front::information(&live.now(), false, now));
    assert!(calm.items.iter().all(|e| e.get("lefttime").is_none() && e.get("maintenance").is_none()));
    live.set("maintenance-at", &(now + 600).to_string()).unwrap();
    live.set("maintenance-message", "Back at 10:00").unwrap();
    let soon = lang(yo3_server::front::information(&live.now(), false, now));
    let notice = &soon.items[0];
    assert_eq!(notice.get("lefttime"), Some(&Value::U32(600)));
    assert_eq!(notice.get("maintenance"), Some(&Value::text("Back at 10:00")));
    assert_eq!((notice.get("title"), notice.get("body")), (Some(&Value::text("Maintenance")), Some(&Value::text("Back at 10:00"))));
    assert_eq!(soon.items.len(), 2, "the news is still there");
    let started = lang(yo3_server::front::information(&live.now(), false, now + 601));
    assert!(started.items.iter().all(|e| e.get("lefttime").is_none()), "no countdown once it started");
}

struct Live { endpoint: Endpoint, admin: yo3_server::admin::Live, data: std::path::PathBuf, _running: Running }

async fn server(test: &str) -> Live {
    logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-moderation-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config::ephemeral(dir);
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap();
    let admin = yo3_server::admin::running(&data).unwrap();
    Live { endpoint, admin, data, _running: running }
}

impl Live {
    async fn account(&self, request: Json) -> Json { self.endpoint.account(&request).await.unwrap() }
    async fn sign_up(&self, user: &str) { assert_eq!(self.account(json!({ "op": "register", "user": user, "password": "a long passphrase", "rules": true })).await["ok"], true) }
    async fn sign_in(&self, user: &str, stay: bool) -> Json { self.account(json!({ "op": "login", "user": user, "password": "a long passphrase", "stay": stay })).await }
    /// A moderator's action, as `yo3-server --by Anna …` sends it.
    async fn moderate(&self, op: Op) -> Result<Json, String> {
        let admin = self.admin.clone();
        tokio::task::spawn_blocking(move || api::call_as(&admin, &op, "Anna")).await.unwrap()
    }
    async fn playing(&self, session: &Token) -> bool { self.endpoint.ticket(session, Role::Front).await.is_ok() }
    fn audit(&self) -> Vec<yo3_server::store::AuditRow> { yo3_server::store::audit_rows(&self.data, 10).unwrap() }
}

fn session(reply: &Json) -> Token {
    let hex = reply["session"].as_str().unwrap_or_else(|| panic!("no session in {reply}"));
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

#[tokio::test]
async fn a_kick_ends_the_game_and_names_the_moderator() {
    let s = server("kick").await;
    s.sign_up("Yugi").await;
    let game = session(&s.sign_in("Yugi", false).await);
    s.moderate(Op::Kick { who: "yugi".into() }).await.unwrap();
    assert!(!s.playing(&game).await, "the game's session ended");
    let row = &s.audit()[0];
    assert_eq!((row.actor.as_str(), row.op.as_str(), row.target.as_str()), ("Anna", "kick", "yugi"));
}

#[tokio::test]
async fn a_ban_refuses_sign_in_with_its_reason_and_signs_every_device_out() {
    let s = server("ban").await;
    s.sign_up("Yugi").await;
    let signed = s.sign_in("Yugi", true).await;
    let (game, token) = (session(&signed), signed["device_token"].clone());
    s.moderate(Op::Ban { who: "Yugi".into(), reason: "cheating".into(), days: Some(7) }).await.unwrap();
    assert!(!s.playing(&game).await, "disconnected at once");
    assert_eq!(s.account(json!({ "op": "resume", "device_token": token })).await["error"], "signed_out", "the device too");
    let refused = s.sign_in("Yugi", false).await;
    assert_eq!((&refused["error"], &refused["reason"]), (&json!("banned"), &json!("cheating")));
    let week = refused["until"].as_i64().unwrap() - logins::now();
    assert!((7 * 86_400 - 60..=7 * 86_400).contains(&week), "{week}");
    let row = &s.audit()[0];
    assert_eq!((row.op.as_str(), row.after.as_ref().unwrap()["kind"].as_str()), ("ban", Some("banned")));
    s.moderate(Op::Lift { who: "Yugi".into() }).await.unwrap();
    assert_eq!(s.sign_in("Yugi", false).await["ok"], true, "lifted");
}

#[tokio::test]
async fn a_restriction_lets_the_game_play_on_and_refuses_the_next_sign_in() {
    let s = server("restrict").await;
    s.sign_up("Yugi").await;
    let game = session(&s.sign_in("Yugi", false).await);
    s.moderate(Op::Restrict { who: "Yugi".into(), reason: "spam".into(), days: None }).await.unwrap();
    assert!(s.playing(&game).await, "still in");
    assert_eq!(s.sign_in("Yugi", false).await["error"], "restricted");
}

#[tokio::test]
async fn maintenance_counts_down_then_lets_only_the_staff_in() {
    let s = server("maintenance").await;
    for user in ["Yugi", "Pegasus"] { s.sign_up(user).await }
    s.moderate(Op::SetRole { user: "Pegasus".into(), role: "admin".into() }).await.unwrap();
    let (player, admin) = (session(&s.sign_in("Yugi", false).await), session(&s.sign_in("Pegasus", false).await));
    s.moderate(Op::Maintenance { minutes: Some(10), message: Some("Back at 10:00".into()), off: false }).await.unwrap();
    assert!(!s.endpoint.info().await.unwrap().maintenance, "only planned");
    assert!(s.playing(&player).await);
    // Now: the watcher (once a second) disconnects everyone but the staff.
    s.moderate(Op::Maintenance { minutes: Some(0), message: None, off: false }).await.unwrap();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert!(s.endpoint.info().await.unwrap().maintenance, "discovery says so (the launcher shows Maintenance)");
    assert!(!s.playing(&player).await && s.playing(&admin).await);
    let refused = s.sign_in("Yugi", false).await;
    assert_eq!((&refused["error"], &refused["message"]), (&json!("maintenance"), &json!("Back at 10:00")));
    assert_eq!(s.sign_in("Pegasus", false).await["ok"], true, "the staff comes in");
    s.moderate(Op::Maintenance { minutes: None, message: None, off: true }).await.unwrap();
    assert_eq!(s.sign_in("Yugi", false).await["ok"], true, "over");
    assert!(s.audit().iter().any(|r| r.op == "maintenance" && r.actor == "Anna"));
}

#[tokio::test]
async fn an_accounts_key_does_not_get_past_its_ban() {
    // An account can have a User ID and a key (the host's `key USER`): the key login refuses what a password would.
    let s = server("key-ban").await;
    s.sign_up("Yugi").await;
    let key = s.moderate(Op::Key { who: "Yugi".into() }).await.unwrap()["key"].as_str().unwrap().to_string();
    s.endpoint.login(&key).await.expect("a key signs in");
    s.moderate(Op::Ban { who: "Yugi".into(), reason: String::new(), days: None }).await.unwrap();
    assert!(s.endpoint.login(&key).await.is_err(), "banned: the key is refused too");
    s.moderate(Op::Lift { who: "Yugi".into() }).await.unwrap();
    s.moderate(Op::Maintenance { minutes: Some(0), message: None, off: false }).await.unwrap();
    assert!(s.endpoint.login(&key).await.is_err(), "maintenance: a player's key waits");
}

#[tokio::test]
async fn a_muted_or_banned_key_account_is_refused_by_name() {
    // Standing and mutes belong to password accounts; a key account (Singleplayer, Scripts) is named in the refusal.
    let s = server("key").await;
    let refused = s.moderate(Op::Ban { who: "test".into(), reason: String::new(), days: None }).await.unwrap_err();
    assert!(refused.contains("key account"), "{refused}");
}
