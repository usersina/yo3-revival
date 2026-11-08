//! D-Mail (MailWindow 0x4AC6B0, dmail_main.swf): the client keeps its mail in its own files
//! (players\<pname>\mail\<box>\<mail_id>.y3m); the server holds a mail until its recipient's mail window fetches it, and
//! tells an online recipient that one came.
mod common;

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn text(m: &Message, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or_default()
}

/// 0x2580 {mail_type 0, title, body, to_id} (the send path 0x4AE8E0).
fn send(c: &mut Client, to_id: u32, title: &str, body: &str) -> Message {
    let fields = [
        ("mail_type", Value::U32(0)),
        ("title", Value::text(title)),
        ("body", Value::text(body)),
        ("to_id", Value::U32(to_id)),
    ];
    one(c.send(req_with(0x2580, &fields)))
}

/// The mail window opening: 0x2583 -> 0x2588 {count}, then for a count 0x2582 {count} -> 0x2586 with the mail.
fn fetch(c: &mut Client) -> Vec<Message> {
    let count = int(&one(c.send(req(0x2583))), "count").expect("count");
    if count == 0 {
        return vec![];
    }
    let r = one(c.send(req_with(0x2582, &[("count", Value::U32(count))])));
    assert_eq!(r.request(), Some(0x2586));
    r.items
}

fn friends(server: &Server) {
    server.hub.lock().unwrap().befriend((1001, "Yugi"), (1002, "Kaiba"));
}

#[test]
fn a_mail_reaches_an_online_friend() {
    // 0x2584 {mail_id} closes the sender's window ("Your message has been sent."). The recipient gets 0x2581, which
    // sets flag 0x1000 (the mail icon on the bell); their window then fetches it: 0x2586 items {mail_id, mail_type,
    // from_id, from_name, to_id, to_name, title, body, send_date "YYYY-MM-DD HH:MM:SS", team_id, team_name} (0x4AD3C0).
    let server = Server::new();
    friends(&server);
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    kaiba.pushed();
    let sent = send(&mut yugi, 1002, "Duel?", "Tonight at nine");
    assert_eq!(sent.request(), Some(0x2584));
    let mail_id = int(&sent, "mail_id").expect("mail_id");
    assert!(mail_id > 0);
    assert_eq!(one(kaiba.pushed()).request(), Some(0x2581));
    let mail = fetch(&mut kaiba);
    assert_eq!(mail.len(), 1);
    let m = &mail[0];
    assert_eq!((int(m, "mail_id"), int(m, "mail_type"), int(m, "from_id"), int(m, "to_id")), (Some(mail_id), Some(0), Some(1001), Some(1002)));
    assert_eq!((text(m, "from_name"), text(m, "to_name")), ("Yugi".to_string(), "Kaiba".to_string()));
    assert_eq!((text(m, "title"), text(m, "body")), ("Duel?".to_string(), "Tonight at nine".to_string()));
    assert_eq!(text(m, "send_date").len(), "2026-11-07 21:00:00".len());
    assert_eq!((int(m, "team_id"), text(m, "team_name")), (Some(0), String::new()));
    // Fetched mail lives in the client's files from then on.
    assert!(fetch(&mut kaiba).is_empty());
}

#[test]
fn a_mail_waits_for_an_offline_friend() {
    // The send window lists the friend list (slot 6), offline friends too; their mail waits for their next login, where
    // the main screen's startup step (0x4730B0) asks 0x2583 and lights the bell for a count.
    let server = Server::new();
    friends(&server);
    let mut yugi = server.join(1001, "Yugi");
    assert_eq!(send(&mut yugi, 1002, "Hi", "Are you there?").request(), Some(0x2584));
    let mut kaiba = server.join(1002, "Kaiba");
    let mail = fetch(&mut kaiba);
    assert_eq!(mail.len(), 1);
    assert_eq!(text(&mail[0], "body"), "Are you there?");
}

#[test]
fn a_mail_to_nobody_is_refused() {
    // 0x2585 {error_code}: 301 "There are no Players with this name." (0x4AE8E0).
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    let r = send(&mut yugi, 4242, "Hi", "?");
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2585), Some(301)));
}

#[test]
fn waiting_mail_is_kept_across_restarts() {
    let dir = std::env::temp_dir().join(format!("yo3-mail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("mail.json");
    let server = Server::new();
    server.hub.lock().unwrap().keep_mail(&path);
    friends(&server);
    let mut yugi = server.join(1001, "Yugi");
    let first = int(&send(&mut yugi, 1002, "Hi", "Kept?"), "mail_id").unwrap();
    let server = Server::new();
    server.hub.lock().unwrap().keep_mail(&path);
    let mail = fetch(&mut server.join(1002, "Kaiba"));
    assert_eq!(text(&mail[0], "body"), "Kept?");
    // Mail ids go on from where they were (the client files mail by id).
    friends(&server);
    let next = int(&send(&mut server.join(1001, "Yugi"), 1002, "Hi", "again"), "mail_id").unwrap();
    assert!(next > first);
}

#[test]
fn a_mails_title_and_body_are_cut_to_a_sane_size() {
    // The mail waits on the server (and in mail.json) until fetched: a megabyte body would sit there for good.
    let server = Server::new();
    friends(&server);
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    assert_eq!(send(&mut yugi, 1002, &"t".repeat(100_000), &"b".repeat(1_000_000)).request(), Some(0x2584));
    let mail = fetch(&mut kaiba).remove(0);
    assert!(text(&mail, "title").len() <= 128 && text(&mail, "body").len() <= 2048);
}
