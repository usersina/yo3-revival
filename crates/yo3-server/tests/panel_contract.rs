//! The admin panel (#36): each page through a running server's loopback HTTP port, as a browser reaches it through the
//! SSH tunnel. Sign-in with a staff account, the session cookie, CSRF and origin checks, and the roles: a moderator
//! moderates but can't change settings, maintenance, BP or roles, or act on a staff account.
use std::collections::HashMap;
use std::net::SocketAddr;

use serde_json::{Value as Json, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yo3_server::api::{self, Op};
use yo3_server::logins;
use yo3_server::{Config, Running, start};
use yo3_tunnel::Endpoint;

const PASSWORD: &str = "LongPassphrase1";

struct Live { http: SocketAddr, endpoint: Endpoint, admin: yo3_server::admin::Live, data: std::path::PathBuf, _running: Running }

/// An HTTP answer: status code, headers (lower-case names) and body.
struct Answer { status: u16, headers: Vec<(String, String)>, body: String }

impl Answer {
    fn header(&self, name: &str) -> Option<&str> { self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()) }
    fn location(&self) -> &str { self.header("location").unwrap_or_default() }
    /// The CSRF token of the page's forms.
    fn csrf(&self) -> String {
        let at = self.body.find("name=\"csrf\" value=\"").unwrap_or_else(|| panic!("no form on the page: {}", self.body)) + 19;
        self.body[at..].split('"').next().unwrap().to_string()
    }
}

/// A browser: its session cookie, and the origin headers a page of the panel sends.
struct Browser { http: SocketAddr, cookie: Option<String> }

fn urlencode(fields: &[(&str, &str)]) -> String {
    fields.iter().map(|(k, v)| format!("{k}={}", v.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>()))
        .collect::<Vec<_>>().join("&")
}

async fn send(http: SocketAddr, request: String) -> Answer {
    let mut stream = tokio::net::TcpStream::connect(http).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let raw = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let mut lines = head.lines();
    let status = lines.next().unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
    let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    Answer { status, headers, body: body.to_string() }
}

impl Browser {
    fn new(http: SocketAddr) -> Browser { Browser { http, cookie: None } }

    fn head(&self, method: &str, path: &str, extra: &str) -> String {
        let cookie = self.cookie.as_ref().map_or(String::new(), |c| format!("Cookie: yo3_panel={c}\r\n"));
        format!("{method} {path} HTTP/1.1\r\nHost: localhost:{}\r\n{cookie}{extra}Connection: close\r\n", self.http.port())
    }

    async fn get(&self, path: &str) -> Answer { send(self.http, format!("{}\r\n", self.head("GET", path, ""))).await }

    /// A form post as the panel's own page sends it (Origin and Sec-Fetch-Site of this origin).
    async fn post(&self, path: &str, fields: &[(&str, &str)]) -> Answer {
        let origin = format!("Origin: http://localhost:{}\r\nSec-Fetch-Site: same-origin\r\n", self.http.port());
        self.post_with(path, fields, &origin).await
    }

    async fn post_with(&self, path: &str, fields: &[(&str, &str)], extra: &str) -> Answer {
        let body = urlencode(fields);
        send(self.http, format!("{}Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}", self.head("POST", path, extra), body.len())).await
    }

    /// Signs in; the answer to the sign-in form.
    async fn sign_in(&mut self, user: &str, password: &str) -> Answer {
        let answer = self.post("/panel/login", &[("user", user), ("password", password)]).await;
        if let Some(cookie) = answer.header("set-cookie") {
            self.cookie = cookie.split(';').next().and_then(|c| c.strip_prefix("yo3_panel=")).map(str::to_string);
        }
        answer
    }

    /// Posts a form of `page` with that page's CSRF token; the page shown after it (its flash message).
    async fn act(&self, page: &str, to: &str, fields: &[(&str, &str)]) -> Answer {
        let csrf = self.get(page).await.csrf();
        let mut all = vec![("csrf", csrf.as_str())];
        all.extend_from_slice(fields);
        let done = self.post(to, &all).await;
        assert_eq!(done.status, 303, "{}", done.body);
        self.get(done.location()).await
    }
}

async fn server(test: &str) -> Live {
    logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-panel-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config::ephemeral(dir);
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap();
    let admin = yo3_server::admin::running(&data).unwrap();
    Live { http: running.http, endpoint, admin, data, _running: running }
}

impl Live {
    async fn sign_up(&self, user: &str) {
        let made = self.endpoint.account(&json!({ "op": "register", "user": user, "password": PASSWORD, "rules": true })).await.unwrap();
        assert_eq!(made["ok"], true, "{made}");
    }
    async fn host(&self, op: Op) -> Json {
        let admin = self.admin.clone();
        tokio::task::spawn_blocking(move || api::call(&admin, &op)).await.unwrap().unwrap()
    }
    /// Pegasus (admin), Croquet (moderator), Yugi and Kaiba (players).
    async fn staffed(test: &str) -> Live {
        let s = server(test).await;
        for user in ["Pegasus", "Croquet", "Yugi", "Kaiba"] { s.sign_up(user).await }
        s.host(Op::SetRole { user: "Pegasus".into(), role: "admin".into() }).await;
        s.host(Op::SetRole { user: "Croquet".into(), role: "moderator".into() }).await;
        // A registered account gets its players in the game; these are made as the host's add-player does.
        for user in ["Yugi", "Kaiba"] { s.host(Op::AddPlayer { who: user.into(), name: user.into() }).await; }
        s
    }
    async fn browser(&self, user: &str) -> Browser {
        let mut b = Browser::new(self.http);
        let signed = b.sign_in(user, PASSWORD).await;
        assert_eq!((signed.status, signed.location()), (303, "/panel/"), "{}", signed.body);
        b
    }
    async fn setting(&self, name: &str) -> String {
        let rows = self.host(Op::Settings).await["settings"].clone();
        rows.as_array().unwrap().iter().find(|r| r["name"] == name).unwrap()["value"].as_str().unwrap().to_string()
    }
    fn audit(&self) -> Vec<yo3_server::store::AuditRow> { yo3_server::store::audit_rows(&self.data, 50).unwrap() }
}

const PAGES: [&str; 10] = ["/panel/", "/panel/accounts", "/panel/account?who=Yugi", "/panel/invites", "/panel/news", "/panel/settings",
    "/panel/moderation", "/panel/audit", "/panel/backups", "/panel/logs"];

#[tokio::test]
async fn every_page_needs_a_staff_sign_in_and_a_players_password_is_not_enough() {
    let s = Live::staffed("sign-in").await;
    let mut b = Browser::new(s.http);
    for page in PAGES {
        let a = b.get(page).await;
        assert_eq!((a.status, a.location()), (303, "/panel/login"), "{page} without a session");
        assert!(!a.body.contains("Yugi"), "{page} leaks nothing");
    }
    // A player's right password, a wrong password and an unknown User ID get the same refusal.
    let refused = [b.sign_in("Yugi", PASSWORD).await, b.sign_in("Croquet", "not it at all").await, b.sign_in("Nobody", PASSWORD).await];
    for r in &refused {
        assert_eq!(r.status, 401);
        assert!(r.body.contains("Wrong User ID or password, or this account can&#39;t use the admin panel."), "{}", r.body);
        assert!(r.header("set-cookie").is_none());
    }
    let signed = b.sign_in("Pegasus", PASSWORD).await;
    let cookie = signed.header("set-cookie").unwrap();
    for part in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/panel"] { assert!(cookie.contains(part), "{cookie}") }
    assert!(s.audit().iter().any(|r| r.op == "panel-sign-in" && r.actor == "Pegasus" && r.origin == "panel"));
}

#[tokio::test]
async fn every_page_shows_for_an_admin_with_the_strict_headers() {
    let s = Live::staffed("pages").await;
    s.host(Op::Invite { uses: Some(3), days: None }).await;
    s.host(Op::GiveBp { who: "Yugi".into(), amount: 1234 }).await;
    let b = s.browser("Pegasus").await;
    let expect: HashMap<&str, &str> = HashMap::from([
        ("/panel/", "Players online"), ("/panel/accounts", "Croquet"), ("/panel/account?who=Yugi", "Won / lost / drawn"), ("/panel/invites", "Sign-ups left"),
        ("/panel/news", "Welcome back to Yu-Gi-Oh! ONLINE"), ("/panel/settings", "server-name"), ("/panel/moderation", "Recent moderation"),
        ("/panel/audit", "give-bp"), ("/panel/backups", "Back up now"), ("/panel/logs", "newest 500 lines"),
    ]);
    for page in PAGES {
        let a = b.get(page).await;
        assert_eq!(a.status, 200, "{page}");
        assert!(a.body.contains(expect[page]), "{page} shows {:?}: {}", expect[page], a.body);
        let csp = a.header("content-security-policy").unwrap();
        for rule in ["default-src 'none'", "script-src 'self'", "frame-ancestors 'none'", "form-action 'self'"] { assert!(csp.contains(rule), "{csp}") }
        assert_eq!((a.header("x-frame-options"), a.header("cache-control")), (Some("DENY"), Some("no-store")));
        // `no-referrer` would make a browser send `Origin: null` on the panel's own forms (found live, Chromium).
        assert_eq!(a.header("referrer-policy"), Some("same-origin"));
        assert!(!a.body.contains("<script>") && !a.body.contains("style=\""), "{page}: no inline script or style");
    }
    assert_eq!(b.get("/panel/panel.css").await.status, 200);
    assert_eq!(b.get("/panel/panel.js").await.status, 200);
}

#[tokio::test]
async fn a_form_without_its_token_or_from_another_site_changes_nothing() {
    let s = Live::staffed("csrf").await;
    let b = s.browser("Pegasus").await;
    let csrf = b.get("/panel/settings").await.csrf();
    let change = [("csrf", csrf.as_str()), ("name", "server-name"), ("value", "Hijacked")];
    assert_eq!(b.post("/panel/settings", &change[1..]).await.status, 403, "no token");
    assert_eq!(b.post("/panel/settings", &[("csrf", "0".repeat(64).as_str()), change[1], change[2]]).await.status, 403, "a wrong token");
    let other = "Origin: http://evil.example\r\nSec-Fetch-Site: cross-site\r\n";
    assert_eq!(b.post_with("/panel/settings", &change, other).await.status, 403, "another site's page, even with the token");
    assert_eq!(b.post_with("/panel/settings", &change, "").await.status, 403, "no origin at all");
    assert_eq!(s.setting("server-name").await, "Test server");
    // From the panel's own page it applies, named in the audit log.
    let page = b.act("/panel/settings", "/panel/settings", &[("name", "server-name"), ("value", "Beta server")]).await;
    assert!(page.body.contains("server-name is now Beta server"), "{}", page.body);
    assert_eq!(s.setting("server-name").await, "Beta server");
    let row = &s.audit()[0];
    assert_eq!((row.actor.as_str(), row.origin.as_str(), row.op.as_str()), ("Pegasus", "panel", "set"));
}

#[tokio::test]
async fn a_rebound_host_name_or_a_session_from_elsewhere_is_refused() {
    let s = Live::staffed("host").await;
    let b = s.browser("Pegasus").await;
    let rebound = send(s.http, format!("GET /panel/ HTTP/1.1\r\nHost: evil.example:{}\r\nCookie: yo3_panel={}\r\nConnection: close\r\n\r\n", s.http.port(), b.cookie.clone().unwrap())).await;
    assert_eq!(rebound.status, 403);
    assert!(!rebound.body.contains("Players online"));
    let forged = Browser { http: s.http, cookie: Some("1".repeat(64)) };
    assert_eq!(forged.get("/panel/").await.location(), "/panel/login");
}

#[tokio::test]
async fn a_moderator_moderates_but_cant_change_settings_or_act_on_staff() {
    let s = Live::staffed("roles").await;
    let m = s.browser("Croquet").await;
    // The settings page shows, its forms disabled; a post anyway is refused by the API.
    let page = m.get("/panel/settings").await;
    assert!(page.body.contains("Only an admin changes settings.") && page.body.contains("<fieldset disabled>"));
    let refused = m.act("/panel/settings", "/panel/settings", &[("name", "registration"), ("value", "closed")]).await;
    assert!(refused.body.contains("Not done: only an admin may do that (set)"), "{}", refused.body);
    assert_eq!(s.setting("registration").await, "open");
    for (page, to, fields) in [
        ("/panel/news", "/panel/news", vec![("action", "maintenance"), ("minutes", "5")]),
        ("/panel/news", "/panel/news", vec![("action", "news"), ("title0", "Hi"), ("body0", "there")]),
        ("/panel/backups", "/panel/backups", vec![]),
        ("/panel/account?who=Yugi", "/panel/account", vec![("who", "Yugi"), ("action", "give-bp"), ("amount", "100")]),
        ("/panel/account?who=Yugi", "/panel/account", vec![("who", "Yugi"), ("action", "set-role"), ("role", "admin")]),
        ("/panel/account?who=Yugi", "/panel/account", vec![("who", "Yugi"), ("action", "give-cards"), ("player", "Yugi"), ("card", "4007")]),
        ("/panel/account?who=Yugi", "/panel/account", vec![("who", "Yugi"), ("action", "give-recipe"), ("player", "Yugi"), ("deck", "7")]),
    ] {
        let after = m.act(page, to, &fields).await;
        assert!(after.body.contains("Not done: only an admin may do that"), "{fields:?}: {}", after.body);
    }
    assert!(m.get("/panel/backups/download?name=x").await.status == 403);
    // Moderation works on a player, and not on the admin (no moderator resets an admin's password).
    let banned = m.act("/panel/account?who=Kaiba", "/panel/account", &[("who", "Kaiba"), ("action", "ban"), ("days", "3"), ("reason", "cheating")]).await;
    assert!(banned.body.contains("Kaiba is banned"), "{}", banned.body);
    assert!(m.get("/panel/moderation").await.body.contains("banned"));
    for action in ["reset-code", "ban", "sign-out", "mute"] {
        let refused = m.act("/panel/account?who=Pegasus", "/panel/account", &[("who", "Pegasus"), ("action", action)]).await;
        assert!(refused.body.contains("Not done: a moderator can&#39;t act on an admin&#39;s or another moderator&#39;s account"), "{action}: {}", refused.body);
    }
    let code = m.act("/panel/account?who=Yugi", "/panel/account", &[("who", "Yugi"), ("action", "reset-code")]).await;
    assert!(code.body.contains("Reset code for Yugi: "), "{}", code.body);
    let invite = m.act("/panel/invites", "/panel/invites", &[("action", "invite"), ("uses", "2")]).await;
    assert!(invite.body.contains("invite code "), "{}", invite.body);
    let row = s.audit().into_iter().find(|r| r.op == "ban").unwrap();
    assert_eq!((row.actor.as_str(), row.origin.as_str(), row.target.as_str()), ("Croquet", "panel", "Kaiba"));
    let refused = s.endpoint.account(&json!({ "op": "login", "user": "Kaiba", "password": PASSWORD })).await.unwrap();
    assert_eq!(refused["error"], "banned");
}

#[tokio::test]
async fn an_admin_runs_every_pages_forms() {
    let s = Live::staffed("admin").await;
    let b = s.browser("Pegasus").await;
    let account = "/panel/account?who=Yugi";
    let said = |a: &Answer, text: &str| assert!(a.body.contains(text), "{text}: {}", a.body);
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "give-bp"), ("amount", "500")]).await, "gave 500 BP to");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "mute"), ("minutes", "10"), ("reason", "spam")]).await, "Yugi is muted until");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "unmute")]).await, "Yugi may chat again");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "restrict")]).await, "restricted");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "lift")]).await, "Yugi may sign in again");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "kick")]).await, "Yugi was disconnected");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "sign-out")]).await, "signed out everywhere");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "set-role"), ("role", "moderator")]).await, "Yugi is now moderator");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "set-role"), ("role", "player")]).await, "Yugi is now player");
    said(&b.act("/panel/invites", "/panel/invites", &[("action", "add-code"), ("code", "SUMMER2026BETA01"), ("bp", "300")]).await, "code SUMMER2026BETA01: 300 BP");
    said(&b.act("/panel/invites", "/panel/invites", &[("action", "remove-code"), ("code", "SUMMER2026BETA01")]).await, "code SUMMER2026BETA01 removed");
    said(&b.act("/panel/news", "/panel/news", &[("action", "news"), ("title0", "Beta"), ("body0", "Welcome & enjoy <3"), ("title1", "Rules"), ("body1", "Be kind")]).await,
        "news is now Beta | Welcome &amp; enjoy &lt;3 / Rules | Be kind");
    said(&b.act("/panel/news", "/panel/news", &[("action", "maintenance"), ("minutes", "30"), ("message", "Back soon")]).await, "maintenance starts at");
    said(&b.get("/panel/news").await, "the game counts down until then");
    said(&b.act("/panel/news", "/panel/news", &[("action", "maintenance-off")]).await, "maintenance is off");
    said(&b.act("/panel/settings", "/panel/settings", &[("name", "start-lp"), ("value", "4000")]).await, "start-lp is now 4000");
    said(&b.act("/panel/settings", "/panel/settings", &[("name", "start-lp"), ("action", "reset")]).await, "start-lp is now 8000");
    said(&b.act("/panel/settings", "/panel/settings", &[("name", "start-lp"), ("value", "lots")]).await, "Not done: start-lp is a whole number");
    let renamed = b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "rename"), ("old", "Yugi"), ("new", "Atem")]).await;
    said(&renamed, "Yugi is now Atem");
    // Backups: one now, listed, downloaded as the SQLite file, and the download audited.
    let backups = b.act("/panel/backups", "/panel/backups", &[]).await;
    said(&backups, "backup ");
    let name = s.host(Op::Backups).await["backups"][0]["name"].as_str().unwrap().to_string();
    let file = b.get(&format!("/panel/backups/download?name={name}")).await;
    assert_eq!((file.status, file.header("content-type")), (200, Some("application/vnd.sqlite3")));
    assert!(file.body.starts_with("SQLite format 3"));
    assert_eq!(b.get("/panel/backups/download?name=../../server.key").await.status, 404);
    let ops: Vec<String> = s.audit().into_iter().filter(|r| r.origin == "panel").map(|r| r.op).collect();
    for op in ["download-backup", "back-up", "set", "reset", "maintenance", "rename", "give-bp", "set-role", "add-code", "remove-code", "mute"] {
        assert!(ops.iter().any(|o| o == op), "{op} in {ops:?}");
    }
}

#[tokio::test]
async fn a_demoted_or_banned_account_is_out_at_its_next_request_and_sign_out_ends_the_session() {
    let s = Live::staffed("demote").await;
    let m = s.browser("Croquet").await;
    assert_eq!(m.get("/panel/").await.status, 200);
    s.host(Op::SetRole { user: "Croquet".into(), role: "player".into() }).await;
    assert_eq!(m.get("/panel/").await.location(), "/panel/login");
    s.host(Op::SetRole { user: "Croquet".into(), role: "moderator".into() }).await;
    assert_eq!(m.get("/panel/").await.location(), "/panel/login", "a new sign-in, not the old session");
    let again = s.browser("Croquet").await;
    s.host(Op::Ban { who: "Croquet".into(), reason: String::new(), days: None }).await;
    assert_eq!(again.get("/panel/").await.location(), "/panel/login", "banned: out");
    let b = s.browser("Pegasus").await;
    let csrf = b.get("/panel/").await.csrf();
    let out = b.post("/panel/logout", &[("csrf", &csrf)]).await;
    assert_eq!((out.status, out.location()), (303, "/panel/login"));
    assert!(out.header("set-cookie").unwrap().contains("Max-Age=0"));
    assert_eq!(b.get("/panel/").await.location(), "/panel/login");
}

#[tokio::test]
async fn wrong_passwords_wait_longer_each_time() {
    let s = Live::staffed("throttle").await;
    let mut b = Browser::new(s.http);
    assert_eq!(b.sign_in("Pegasus", "wrong password 1").await.status, 401);
    let waits = b.sign_in("Pegasus", PASSWORD).await;
    assert!(waits.body.contains("Too many tries: wait"), "the right password waits too: {}", waits.body);
}

#[tokio::test]
async fn the_api_refuses_a_moderator_what_only_an_admin_may_do() {
    // The panel's role checks live in the API (Api::apply_as), so a future client can't skip them.
    let settings = std::sync::Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let accounts = std::sync::Arc::new(std::sync::Mutex::new(yo3_server::accounts::Directory::new(vec![])));
    let a = api::Api::new(accounts, settings, None, std::path::PathBuf::new(), "server");
    let moderator = Some(logins::Role::Moderator);
    for op in [Op::Set { name: "start-lp".into(), value: "100".into() }, Op::Shutdown, Op::Key { who: "x".into() }, Op::BackUp,
        Op::Maintenance { minutes: Some(1), message: None, off: false }, Op::AddUser { user: "x".into(), hash: String::new() }] {
        assert!(!op.for_moderators(), "{op:?}");
        assert!(a.apply_as(&op, "Croquet", "panel", moderator).unwrap_err().starts_with("only an admin"), "{op:?}");
    }
    assert!(a.apply_as(&Op::Settings, "Croquet", "panel", moderator).is_ok());
    assert!(a.apply_as(&Op::Settings, "Yugi", "panel", Some(logins::Role::Player)).is_err());
    assert!(a.apply_as(&Op::Set { name: "start-lp".into(), value: "100".into() }, "Pegasus", "panel", Some(logins::Role::Admin)).is_ok());
}

#[tokio::test]
async fn the_health_check_answers_on_loopback_without_a_token() {
    // The host's monitor (#37, tools/host/yo3-monitor.sh) reads these fields.
    let s = server("health").await;
    let a = send(s.http, "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n".into()).await;
    let health: Json = serde_json::from_str(&a.body).unwrap();
    // 200 when it can save and duel; a checkout without the rules engine fetched answers 503 and says why.
    assert_eq!(health["saving"], true);
    assert_eq!(a.status == 200, health["engine"] == true, "{}", a.body);
    assert!(health["backup_age_seconds"].as_u64().is_some_and(|age| age < 60), "the start's backup: {health}");
    assert_eq!((&health["online"], &health["errors_recent"]), (&json!(0), &json!(0)));
    // The monitor's parsing: bare values, no spaces.
    assert!(a.body.contains("\"saving\":true") && a.body.contains("\"engine\":"), "{}", a.body);
}

#[tokio::test]
async fn an_admin_gives_cards_and_recipes_from_a_players_page_and_a_moderator_sees_no_such_form() {
    // #19: the forms are on the player's page, each with the CSRF token and a confirm prompt.
    let s = Live::staffed("gifts").await;
    let b = s.browser("Pegasus").await;
    let account = "/panel/account?who=Yugi";
    let page = b.get(account).await;
    for action in ["give-cards", "give-recipe"] {
        let at = page.body.find(&format!("name=\"action\" value=\"{action}\"")).unwrap_or_else(|| panic!("no {action} form: {}", page.body));
        let form = &page.body[page.body[..at].rfind("<form").unwrap()..at];
        assert!(form.contains("data-confirm=") && form.contains("name=\"csrf\""), "{action}: {form}");
    }
    let m = s.browser("Croquet").await;
    let seen = m.get(account).await;
    assert!(!seen.body.contains("give-cards") && !seen.body.contains("give-recipe"), "{}", seen.body);
    // Without its token the form does nothing.
    let forged = b.post("/panel/account", &[("who", "Yugi"), ("action", "give-cards"), ("player", "Yugi"), ("card", "4007")]).await;
    assert_ne!(forged.status, 303);
    if !yo3_data::default_game_dir().join("package/data002.ypk").is_file() {
        eprintln!("game not installed; the gifts themselves are checked in api_contract.rs");
        return;
    }
    let said = |a: &Answer, text: &str| assert!(a.body.contains(text), "{text}: {}", a.body);
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "give-cards"), ("player", "Yugi"), ("card", "Blue-Eyes White Dragon"),
        ("art", "2"), ("count", "3")]).await, "gave 3 × Blue-Eyes White Dragon (artwork 2, Konami id 4007) to Yugi");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "give-cards"), ("player", "Yugi"), ("card", "Dark Magician")]).await,
        "Not done: 2 cards are named");
    said(&b.act(account, "/panel/account", &[("who", "Yugi"), ("action", "give-recipe"), ("player", "Yugi"), ("deck", "E-HERO EVOLUTION DECK"),
        ("cards", "on")]).await, "gave the recipe E-HERO EVOLUTION DECK");
    let rows: Vec<_> = s.audit().into_iter().filter(|r| r.op.starts_with("give-") && r.origin == "panel").collect();
    assert_eq!(rows.iter().map(|r| (r.op.as_str(), r.actor.as_str(), r.target.as_str())).collect::<Vec<_>>(),
        [("give-recipe", "Pegasus", "Yugi"), ("give-cards", "Pegasus", "Yugi")]);
}
