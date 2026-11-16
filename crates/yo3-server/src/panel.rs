//! The admin panel (#36): server-rendered pages over the admin API (`api.rs`), on the server's loopback HTTP port under
//! [`PREFIX`]. On a VPS the host reaches it through an SSH tunnel (`ssh -L 8080:localhost:8080 HOST`, D7).
//!
//! - **Sign-in** with a password account whose role is admin or moderator (`set-role USER admin`). The same throttle as
//!   the launcher's sign-in: a bucket per address, and a doubling wait per User ID after a wrong password.
//! - **Sessions** are a random cookie (`HttpOnly`, `Secure`, `SameSite=Strict`, path `/panel`), kept as a digest, ending
//!   after 30 idle minutes or 12 hours. Each request checks the account again: a demoted or banned account is out at once.
//! - **Every change** is a form post carrying the session's CSRF token, from this origin (`Origin`, `Sec-Fetch-Site`),
//!   and runs one API operation as the signed-in account ([`Api::apply_as`]): the API refuses a moderator what only an
//!   admin may do, and the audit log names the account, from "panel".
//! - **Every page** is sent with a strict Content-Security-Policy (no inline script or style), no framing, no caching.
//! - **Only this PC:** a peer that isn't loopback, or a `Host` that isn't localhost (DNS rebinding), is refused. The TLS
//!   gate never forwards these paths (`transport.rs`, `public_http`).
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::{Value, json};

use crate::admin::Response;
use crate::api::{Api, Op};
use crate::lock::Locked;
use crate::logins::{Role, Throttle};

/// Where the panel's pages are.
pub const PREFIX: &str = "/panel";
/// The session cookie's name.
pub const COOKIE: &str = "yo3_panel";
/// How long a session lasts without a request, and at most.
const IDLE: i64 = 30 * 60;
const LONGEST: i64 = 12 * 3600;
/// The panel's origin in the audit log.
const ORIGIN: &str = "panel";

const CSS: &str = include_str!("panel/panel.css");
const JS: &str = include_str!("panel/panel.js");

/// A signed-in staff account.
struct Session { user_id: u32, user: String, csrf: String, started: i64, last: i64, flash: Option<(bool, String)> }

/// The panel's state: its sessions and its sign-in throttle.
#[derive(Default)]
pub struct Panel { sessions: Mutex<HashMap<String, Session>>, throttle: Mutex<Throttle> }

/// A parsed request.
struct Request { method: String, path: String, query: HashMap<String, String>, headers: Vec<(String, String)>, form: HashMap<String, String> }

impl Request {
    fn parse(raw: &str) -> Request {
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
        let mut lines = head.lines();
        let mut first = lines.next().unwrap_or_default().split_whitespace();
        let method = first.next().unwrap_or_default().to_string();
        let target = first.next().unwrap_or("/");
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
        Request { method, path: path.to_string(), query: form(query), headers, form: form(body) }
    }

    fn header(&self, name: &str) -> Option<&str> { self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()) }

    fn cookie(&self) -> Option<&str> {
        self.headers.iter().filter(|(k, _)| k == "cookie").flat_map(|(_, v)| v.split(';'))
            .find_map(|c| c.trim().strip_prefix(COOKIE).and_then(|v| v.strip_prefix('=')))
    }

    fn field(&self, name: &str) -> &str { self.form.get(name).map_or("", |v| v.trim()) }

    /// Whether the request came from a page of this origin: `Sec-Fetch-Site` same-origin (or typed: none), and an
    /// `Origin` naming this host. A form post from a browser carries at least one of them.
    /// What a refused request said about where it came from, for the log (no secrets).
    fn whence(&self) -> String {
        format!("Origin {:?}, Sec-Fetch-Site {:?}", self.header("origin").unwrap_or("-"), self.header("sec-fetch-site").unwrap_or("-"))
    }

    fn same_origin(&self) -> bool {
        let site_ok = self.header("sec-fetch-site").map(|s| s == "same-origin" || s == "none");
        let origin_ok = self.header("origin").map(|o| {
            let host = self.header("host").unwrap_or_default();
            o == format!("http://{host}") || o == format!("https://{host}")
        });
        site_ok != Some(false) && origin_ok != Some(false) && (site_ok.is_some() || origin_ok.is_some())
    }

    /// Whether the `Host` names this PC (a page of another site that rebinds its name to 127.0.0.1 is refused).
    fn local_host(&self) -> bool {
        let Some(host) = self.header("host") else { return false };
        let name = if host.starts_with('[') { host.split(']').next().map(|h| format!("{h}]")).unwrap_or_default() }
            else { host.split(':').next().unwrap_or_default().to_string() };
        matches!(name.to_ascii_lowercase().as_str(), "localhost" | "127.0.0.1" | "[::1]")
    }
}

/// A form or query string's fields (`application/x-www-form-urlencoded`).
fn form(text: &str) -> HashMap<String, String> {
    text.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (decode(k), decode(v))
    }).collect()
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                Some(b) => { out.push(b); i += 2 }
                None => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Text for a URL's query.
fn encode(text: &str) -> String {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Text for HTML.
fn esc(text: &str) -> String {
    text.chars().map(|c| match c { '&' => "&amp;".into(), '<' => "&lt;".into(), '>' => "&gt;".into(), '"' => "&quot;".into(), '\'' => "&#39;".into(), c => c.to_string() }).collect()
}

/// A JSON value as text for a page (strings bare, null as empty).
fn text(v: &Value) -> String {
    match v { Value::Null => String::new(), Value::String(s) => s.clone(), other => other.to_string() }
}

/// The headers every answer carries.
fn security_headers() -> Vec<(&'static str, String)> {
    vec![
        ("Content-Security-Policy", "default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'".into()),
        ("X-Frame-Options", "DENY".into()),
        ("X-Content-Type-Options", "nosniff".into()),
        // Not no-referrer: with it a browser sends `Origin: null` on the panel's own form posts, which the origin check
        // must refuse (a sandboxed page sends that too).
        ("Referrer-Policy", "same-origin".into()),
        ("Cross-Origin-Opener-Policy", "same-origin".into()),
        ("Cross-Origin-Resource-Policy", "same-origin".into()),
        ("Cache-Control", "no-store".into()),
    ]
}

fn answer(status: &'static str, content_type: &'static str, body: Vec<u8>, extra: Vec<(&'static str, String)>) -> Response {
    let mut headers = security_headers();
    headers.extend(extra);
    Response { status, content_type, headers, body }
}

fn redirect(to: &str, extra: Vec<(&'static str, String)>) -> Response {
    let mut headers = vec![("Location", to.to_string())];
    headers.extend(extra);
    answer("303 See Other", "text/plain; charset=utf-8", Vec::new(), headers)
}

fn refused(status: &'static str, why: &str) -> Response {
    answer(status, "text/html; charset=utf-8", page_shell("Refused", &format!("<main class=\"narrow\"><h1>Refused</h1><p>{}</p></main>", esc(why))).into_bytes(), vec![])
}

fn page_shell(title: &str, body: &str) -> String {
    format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{} · YO3 admin</title><link rel=\"stylesheet\" href=\"{PREFIX}/panel.css\"><script src=\"{PREFIX}/panel.js\" defer></script></head><body>{body}</body></html>", esc(title))
}

/// The pages in the menu: path, title, and whether only admins see it.
const PAGES: &[(&str, &str)] = &[
    ("/", "Dashboard"), ("/accounts", "Accounts"), ("/invites", "Invites and codes"), ("/news", "News and maintenance"),
    ("/settings", "Settings"), ("/moderation", "Moderation"), ("/audit", "Audit log"), ("/backups", "Backups"), ("/logs", "Logs"),
];

/// Who is looking at a page.
struct Viewer { user: String, role: Role, csrf: String }

impl Viewer {
    fn admin(&self) -> bool { self.role == Role::Admin }
    /// The hidden field every form carries.
    fn token(&self) -> String { format!("<input type=\"hidden\" name=\"csrf\" value=\"{}\">", esc(&self.csrf)) }
}

impl Panel {
    pub fn new() -> Panel { Panel::default() }

    /// Signs `user_id` in: the new session's cookie value.
    fn open(&self, user_id: u32, user: &str) -> String {
        let token = crate::logins::new_token();
        let now = crate::logins::now();
        let mut sessions = self.sessions.locked();
        sessions.retain(|_, s| now - s.last < IDLE && now - s.started < LONGEST);
        sessions.insert(crate::logins::digest(&token), Session { user_id, user: user.into(), csrf: crate::logins::new_token(), started: now, last: now, flash: None });
        token
    }

    fn flash(&self, key: &str, ok: bool, message: impl Into<String>) {
        if let Some(s) = self.sessions.locked().get_mut(key) { s.flash = Some((ok, message.into())) }
    }
}

/// The panel's answer to `raw` (a whole HTTP request) from `peer`, if it is one of its paths.
pub async fn handle(raw: &str, peer: IpAddr, panel: &Panel, api: &Api) -> Option<Response> {
    let r = Request::parse(raw);
    let rest = r.path.strip_prefix(PREFIX)?;
    if !rest.is_empty() && !rest.starts_with('/') { return None }
    if !peer.is_loopback() { return Some(refused("403 Forbidden", "The admin panel answers this PC only.")) }
    if !r.local_host() { return Some(refused("403 Forbidden", "Open the panel at http://localhost (through the SSH tunnel on a server).")) }
    let rest = if rest.is_empty() { "/" } else { rest };
    match (r.method.as_str(), rest) {
        ("GET", "/panel.css") => return Some(answer("200 OK", "text/css; charset=utf-8", CSS.as_bytes().to_vec(), vec![])),
        ("GET", "/panel.js") => return Some(answer("200 OK", "text/javascript; charset=utf-8", JS.as_bytes().to_vec(), vec![])),
        ("GET", "/login") => return Some(login_page(None, "")),
        ("POST", "/login") => return Some(sign_in(&r, peer, panel, api).await),
        _ => {}
    }
    // Everything else needs a session of a staff account in good standing.
    let key = r.cookie().map(crate::logins::digest);
    let now = crate::logins::now();
    let viewer = key.as_ref().and_then(|key| {
        let mut sessions = panel.sessions.locked();
        let s = sessions.get_mut(key)?;
        if now - s.last >= IDLE || now - s.started >= LONGEST { sessions.remove(key); return None }
        let d = api.accounts.locked();
        let login = d.login_by_id(s.user_id).filter(|l| l.staff() && l.refusal(now).is_none() && l.user.eq_ignore_ascii_case(&s.user));
        let Some(login) = login else { drop(d); sessions.remove(key); return None };
        s.last = now;
        Some((Viewer { user: login.user.clone(), role: login.role, csrf: s.csrf.clone() }, s.flash.take()))
    });
    let Some((viewer, flash)) = viewer else {
        return Some(if r.method == "GET" { redirect(&format!("{PREFIX}/login"), vec![]) } else { refused("403 Forbidden", "Your session has ended: sign in again.") });
    };
    let key = key.unwrap_or_default();
    if r.method == "POST" {
        if !r.same_origin() || !crate::admin::same(r.field("csrf"), &viewer.csrf) {
            tracing::warn!("[panel] {} refused: not from the panel's own page ({})", r.path, r.whence());
            return Some(refused("403 Forbidden", "This form didn't come from the panel's own page. Reload the page and try again."));
        }
        return Some(post(rest, &r, &viewer, &key, panel, api));
    }
    if r.method != "GET" { return Some(refused("405 Method Not Allowed", "Use the panel's pages.")) }
    if rest == "/backups/download" { return Some(download(&r, &viewer, api)) }
    let body = match rest {
        "/" => dashboard(&viewer, api),
        "/accounts" => accounts(&r, api),
        "/account" => account(&r, &viewer, api),
        "/invites" => invites(&viewer, api),
        "/news" => news(&viewer, api),
        "/settings" => settings(&viewer, api),
        "/moderation" => moderation(api),
        "/audit" => audit(api),
        "/backups" => backups(&viewer, api),
        "/logs" => logs(&r, api),
        _ => return Some(answer("404 Not Found", "text/html; charset=utf-8", page_shell("Not found", "<main class=\"narrow\"><h1>Not found</h1></main>").into_bytes(), vec![])),
    };
    let title = PAGES.iter().find(|(p, _)| *p == rest).map_or(if rest == "/account" { "Account" } else { "" }, |(_, t)| t);
    Some(answer("200 OK", "text/html; charset=utf-8", layout(title, rest, &viewer, flash, &body).into_bytes(), vec![]))
}

fn layout(title: &str, at: &str, viewer: &Viewer, flash: Option<(bool, String)>, body: &str) -> String {
    let menu: String = PAGES.iter().map(|(path, name)| {
        let current = if *path == at || (at == "/account" && *path == "/accounts") { " aria-current=\"page\"" } else { "" };
        format!("<a href=\"{PREFIX}{path}\"{current}>{}</a>", esc(name))
    }).collect();
    let flash = flash.map_or(String::new(), |(ok, m)| format!("<p class=\"flash {}\" role=\"status\">{}</p>", if ok { "ok" } else { "error" }, esc(&m)));
    let role = if viewer.admin() { "admin" } else { "moderator" };
    page_shell(title, &format!("<header><span class=\"brand\">YO3 Revival · admin</span><nav>{menu}</nav>\
<form method=\"post\" action=\"{PREFIX}/logout\" class=\"who\">{}<span>{} ({role})</span><button>Sign out</button></form></header>\
<main><h1>{}</h1>{flash}{body}</main>", viewer.token(), esc(&viewer.user), esc(title)))
}

fn login_page(error: Option<&str>, user: &str) -> Response {
    let error = error.map_or(String::new(), |e| format!("<p class=\"flash error\" role=\"alert\">{}</p>", esc(e)));
    let body = format!("<main class=\"narrow\"><h1>YO3 Revival admin</h1>{error}<form method=\"post\" action=\"{PREFIX}/login\" class=\"stack\">\
<label>User ID <input name=\"user\" autocomplete=\"username\" required value=\"{}\" autofocus></label>\
<label>Password <input name=\"password\" type=\"password\" autocomplete=\"current-password\" required></label>\
<button>Sign in</button></form><p class=\"muted\">An admin or moderator account. The host makes one with <code>add-user</code> and \
<code>set-role USER admin</code>.</p></main>", esc(user));
    answer(if error.is_empty() { "200 OK" } else { "401 Unauthorized" }, "text/html; charset=utf-8", page_shell("Sign in", &body).into_bytes(), vec![])
}

/// One text for every refused sign-in: an unknown User ID, a wrong password and a player's account look the same.
const WRONG: &str = "Wrong User ID or password, or this account can't use the admin panel.";

async fn sign_in(r: &Request, peer: IpAddr, panel: &Panel, api: &Api) -> Response {
    if !r.same_origin() {
        tracing::warn!("[panel] a sign-in was refused: not from the panel's own page ({})", r.whence());
        return refused("403 Forbidden", "Sign in from the panel's own page.");
    }
    let user = r.field("user").to_string();
    let password = r.form.get("password").cloned().unwrap_or_default();
    {
        let limits = api.settings.now().limits;
        let mut throttle = panel.throttle.locked();
        throttle.set_limits(limits);
        let at = Instant::now();
        let wait = throttle.address(peer, at).err().or_else(|| throttle.user_wait(&user, at));
        if let Some(wait) = wait { return login_page(Some(&format!("Too many tries: wait {} seconds and try again.", wait.as_secs().max(1))), &user) }
    }
    let found = api.accounts.locked().login_of(&user);
    let hash = found.as_ref().map(|(_, l)| l.password.clone()).filter(|h| !h.is_empty());
    let matched = crate::logins::verify_password(password, hash).await;
    let now = crate::logins::now();
    let Some((user_id, login)) = found.filter(|(_, l)| matched && l.staff() && l.refusal(now).is_none()) else {
        panel.throttle.locked().failed(&user, Instant::now());
        tracing::warn!("[panel] a sign-in was refused");
        return login_page(Some(WRONG), &user);
    };
    panel.throttle.locked().succeeded(&user);
    let token = panel.open(user_id, &login.user);
    tracing::info!("[panel] {} signed in", login.user);
    api.note(&login.user, ORIGIN, "panel-sign-in", &login.user);
    redirect(&format!("{PREFIX}/"), vec![("Set-Cookie", format!("{COOKIE}={token}; Path={PREFIX}; HttpOnly; Secure; SameSite=Strict; Max-Age={LONGEST}"))])
}

/// Runs `op` as the viewer: its result, or why not.
fn run(api: &Api, viewer: &Viewer, op: &Op) -> Result<Value, String> { api.apply_as(op, &viewer.user, ORIGIN, Some(viewer.role)) }

/// What an operation's result says to the host.
fn said(result: &Value) -> String {
    result["message"].as_str().map(str::to_string).unwrap_or_else(|| "Done.".into())
}

fn number(text: &str) -> Result<Option<u32>, String> {
    if text.is_empty() { return Ok(None) }
    text.parse().map(Some).map_err(|_| format!("{text:?} is not a whole number"))
}

/// A form post: one operation, then back to the page it came from with what happened.
fn post(at: &str, r: &Request, viewer: &Viewer, key: &str, panel: &Panel, api: &Api) -> Response {
    if at == "/logout" {
        panel.sessions.locked().remove(key);
        return redirect(&format!("{PREFIX}/login"), vec![("Set-Cookie", format!("{COOKIE}=; Path={PREFIX}; HttpOnly; Secure; SameSite=Strict; Max-Age=0"))]);
    }
    let back = |page: &str| format!("{PREFIX}{page}");
    let (page, outcome): (String, Result<String, String>) = match at {
        "/account" => {
            let who = r.field("who").to_string();
            let reason = r.field("reason").to_string();
            let op = (|| Ok::<Op, String>(match r.field("action") {
                "kick" => Op::Kick { who: who.clone() },
                "sign-out" => Op::SignOut { who: who.clone() },
                "ban" => Op::Ban { who: who.clone(), reason, days: number(r.field("days"))? },
                "restrict" => Op::Restrict { who: who.clone(), reason, days: number(r.field("days"))? },
                "lift" => Op::Lift { who: who.clone() },
                "mute" => Op::Mute { who: who.clone(), reason, minutes: number(r.field("minutes"))? },
                "unmute" => Op::Unmute { who: who.clone() },
                "reset-code" => Op::ResetCode { user: who.clone() },
                "rename" => Op::Rename { old: r.field("old").into(), new: r.field("new").into() },
                "give-bp" => Op::GiveBp { who: who.clone(), amount: number(r.field("amount"))?.ok_or("give how many BP?")? },
                "set-role" => Op::SetRole { user: who.clone(), role: r.field("role").into() },
                "set-country" => Op::SetCountry { who: who.clone(), code: r.field("code").into() },
                other => return Err(format!("unknown action {other:?}")),
            }))();
            let outcome = op.and_then(|op| run(api, viewer, &op).map(|v| match &op {
                Op::ResetCode { .. } => format!("Reset code for {who}: {}. Hand it to the player: it works once, for 24 hours, in the launcher's \"Forgot password\".", text(&v["code"])),
                _ => said(&v),
            }));
            // A renamed player's page is under its new name.
            let shown = if r.field("action") == "rename" && outcome.is_ok() && who.eq_ignore_ascii_case(r.field("old")) { r.field("new").to_string() } else { who.clone() };
            (format!("{}?who={}", back("/account"), encode(&shown)), outcome)
        }
        "/invites" => {
            let outcome = match r.field("action") {
                "invite" => number(r.field("uses")).and_then(|uses| Ok((uses, number(r.field("days"))?)))
                    .and_then(|(uses, days)| run(api, viewer, &Op::Invite { uses, days })).map(|v| said(&v)),
                "add-code" => number(r.field("bp")).and_then(|bp| Ok((bp.ok_or("how many BP?")?, number(r.field("uses"))?)))
                    .and_then(|(bp, uses)| run(api, viewer, &Op::AddCode { code: r.field("code").into(), bp, uses })).map(|v| said(&v)),
                "remove-code" => run(api, viewer, &Op::RemoveCode { code: r.field("code").into() }).map(|v| said(&v)),
                other => Err(format!("unknown action {other:?}")),
            };
            (back("/invites"), outcome)
        }
        "/news" => {
            let outcome = match r.field("action") {
                "news" => {
                    let entries: Vec<Value> = (0..10).filter_map(|i| {
                        let (title, body) = (r.field(&format!("title{i}")), r.field(&format!("body{i}")));
                        (!title.is_empty() || !body.is_empty()).then(|| json!({ "title": title, "body": body }))
                    }).collect();
                    run(api, viewer, &Op::Set { name: "news".into(), value: Value::Array(entries).to_string() }).map(|v| said(&v))
                }
                "maintenance" => number(r.field("minutes")).and_then(|minutes| {
                    let message = Some(r.field("message").to_string()).filter(|m| !m.is_empty());
                    run(api, viewer, &Op::Maintenance { minutes: Some(minutes.ok_or("in how many minutes? (0: now)")?), message, off: false })
                }).map(|v| said(&v)),
                "maintenance-off" => run(api, viewer, &Op::Maintenance { minutes: None, message: None, off: true }).map(|v| said(&v)),
                other => Err(format!("unknown action {other:?}")),
            };
            (back("/news"), outcome)
        }
        "/settings" => {
            let name = r.field("name").to_string();
            let op = if r.field("action") == "reset" { Op::Reset { name } } else { Op::Set { name, value: r.field("value").into() } };
            (back("/settings"), run(api, viewer, &op).map(|v| said(&v)))
        }
        "/backups" => (back("/backups"), run(api, viewer, &Op::BackUp).map(|v| said(&v))),
        _ => return refused("404 Not Found", "No such form."),
    };
    match &outcome {
        Ok(m) => panel.flash(key, true, m.clone()),
        Err(e) => panel.flash(key, false, format!("Not done: {e}")),
    }
    redirect(&page, vec![])
}

fn ago(seconds: u64) -> String {
    match seconds {
        s if s < 120 => format!("{s} s"),
        s if s < 7200 => format!("{} min", s / 60),
        s if s < 172_800 => format!("{} h", s / 3600),
        s => format!("{} days", s / 86_400),
    }
}

fn bytes(n: u64) -> String {
    if n >= 1 << 30 { format!("{:.1} GB", n as f64 / f64::from(1u32 << 30)) } else if n >= 1 << 20 { format!("{:.1} MB", n as f64 / f64::from(1u32 << 20)) } else { format!("{:.0} KB", n as f64 / 1024.0) }
}

fn failed(e: &str) -> String { format!("<p class=\"flash error\">{}</p>", esc(e)) }

fn dashboard(viewer: &Viewer, api: &Api) -> String {
    let s = match run(api, viewer, &Op::Status) { Ok(s) => s, Err(e) => return failed(&e) };
    let tile = |label: &str, value: String, warn: bool| format!("<div class=\"tile{}\"><span>{}</span><strong>{}</strong></div>", if warn { " warn" } else { "" }, esc(label), esc(&value));
    let backup_age = s["backup_age_seconds"].as_u64();
    let backup_stale = backup_age.is_none_or(|a| a > 2 * 3600);
    let mut out = String::from("<section class=\"tiles\">");
    out += &tile("Players online", text(&s["online"]), false);
    out += &tile("Duels", text(&s["duels"]), false);
    out += &tile("Rules engines", text(&s["engines"]), false);
    out += &tile("Memory", s["memory_bytes"].as_u64().map_or("unknown".into(), bytes), false);
    out += &tile("Uptime", s["uptime_seconds"].as_u64().map_or("-".into(), ago), false);
    out += &tile("Version", text(&s["release"]), false);
    out += &tile("Last backup", backup_age.map_or("none yet".into(), |a| format!("{} ago", ago(a))), backup_stale);
    out += &tile("Saving", if s["saving"] == true { "OK".into() } else { "FAILING".into() }, s["saving"] != true);
    out += "</section>";
    if let Some(problem) = s["engine_problem"].as_str() { out += &failed(&format!("Duels won't work: {problem}")) }
    out += &format!("<p class=\"muted\">Players connect on {}.</p>", esc(&text(&s["listen"])));
    out += "<h2>Online now</h2>";
    let players = s["players"].as_array().cloned().unwrap_or_default();
    if players.is_empty() { out += "<p class=\"muted\">Nobody is online.</p>" } else {
        out += "<table><thead><tr><th>Player</th><th>Duelling</th></tr></thead><tbody>";
        for p in players {
            out += &format!("<tr><td><a href=\"{PREFIX}/account?who={}\">{}</a></td><td>{}</td></tr>", encode(&text(&p["name"])), esc(&text(&p["name"])), if p["duelling"] == true { "yes" } else { "" });
        }
        out += "</tbody></table>";
    }
    out += &format!("<h2>Recent problems</h2>{}", log_table(s["recent_problems"].as_array().map(Vec::as_slice).unwrap_or_default(), "No warnings or errors since the server started."));
    out += &format!("<p><a href=\"{PREFIX}/logs\">All logs</a></p>");
    out
}

fn log_table(lines: &[Value], empty: &str) -> String {
    if lines.is_empty() { return format!("<p class=\"muted\">{}</p>", esc(empty)) }
    let rows: String = lines.iter().map(|l| format!("<li class=\"log {}\">{}</li>", esc(&text(&l["level"]).to_ascii_lowercase()), esc(&text(&l["text"])))).collect();
    format!("<ul class=\"logs\">{rows}</ul>")
}

fn standing_badge(a: &Value) -> String {
    let mut out = String::new();
    match a["standing"].as_str() {
        Some("banned") => out += "<span class=\"badge bad\">banned</span>",
        Some("restricted") => out += "<span class=\"badge warn\">restricted</span>",
        _ => {}
    }
    if a["muted"] == true { out += "<span class=\"badge warn\">muted</span>" }
    out
}

fn accounts(r: &Request, api: &Api) -> String {
    let q = r.query.get("q").map_or("", |q| q.trim()).to_string();
    let all = match api.apply(&Op::Accounts, "") { Ok(v) => v.as_array().cloned().unwrap_or_default(), Err(e) => return failed(&e) };
    let needle = q.to_lowercase();
    let names = |a: &Value| a["players"].as_array().into_iter().flatten().map(|p| text(&p["name"])).collect::<Vec<_>>();
    let shown: Vec<&Value> = all.iter().filter(|a| needle.is_empty() || text(&a["login"]).to_lowercase().contains(&needle)
        || names(a).iter().any(|n| n.to_lowercase().contains(&needle)) || text(&a["user_id"]) == needle).collect();
    let mut out = format!("<form method=\"get\" action=\"{PREFIX}/accounts\" class=\"inline\"><label>Find <input name=\"q\" value=\"{}\" placeholder=\"User ID or player name\"></label><button>Search</button></form>", esc(&q));
    out += &format!("<p class=\"muted\">{} of {} accounts.</p>", shown.len(), all.len());
    out += "<table><thead><tr><th>Account</th><th>User ID</th><th>Players</th><th>Role</th><th>BP</th><th>Standing</th></tr></thead><tbody>";
    for a in shown.iter().take(500) {
        let who = a["login"].as_str().map(str::to_string).or_else(|| names(a).first().cloned()).unwrap_or_else(|| text(&a["user_id"]));
        let id = if a["login"].is_null() { if a["has_key"] == true { "<span class=\"muted\">key account</span>".to_string() } else { "-".into() } } else { esc(&text(&a["login"])) };
        out += &format!("<tr><td><a href=\"{PREFIX}/account?who={}\">#{}</a></td><td>{id}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            encode(&who), esc(&text(&a["user_id"])), esc(&names(a).join(", ")), esc(&text(&a["role"])), esc(&text(&a["bp"])), standing_badge(a));
    }
    out + "</tbody></table>"
}

fn account(r: &Request, viewer: &Viewer, api: &Api) -> String {
    let who = r.query.get("who").cloned().unwrap_or_default();
    let a = match run(api, viewer, &Op::Account { who: who.clone() }) { Ok(a) => a, Err(e) => return failed(&e) };
    let password = !a["login"].is_null();
    let name = if password { text(&a["login"]) } else { a["players"][0]["name"].as_str().unwrap_or(&who).to_string() };
    let standing = match a["standing"]["kind"].as_str() {
        Some("banned") | Some("restricted") => {
            let until = a["standing"]["until"].as_i64().and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("until lifted".into(), |t| format!("until {}", t.format("%Y-%m-%d %H:%M UTC")));
            format!("{} {until}: {}", text(&a["standing"]["kind"]), text(&a["standing"]["reason"]))
        }
        _ if password => "active".into(),
        _ => "a key account (no sign-in, standing or mute)".into(),
    };
    let muted = if a["muted"].is_null() { "no".to_string() } else {
        let until = a["muted"]["until"].as_i64().and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("until unmuted".into(), |t| format!("until {}", t.format("%Y-%m-%d %H:%M UTC")));
        format!("{until}: {}", text(&a["muted"]["reason"]))
    };
    let mut out = format!("<dl class=\"facts\"><dt>Account</dt><dd>#{}</dd><dt>User ID</dt><dd>{}</dd><dt>Role</dt><dd>{}</dd><dt>BP</dt><dd>{}</dd>\
<dt>Country</dt><dd>{}</dd><dt>Standing</dt><dd>{}</dd><dt>Muted</dt><dd>{}</dd><dt>Signed-in devices</dt><dd>{}</dd><dt>Reset code waiting</dt><dd>{}</dd></dl>",
        esc(&text(&a["user_id"])), if password { esc(&name) } else { "-".into() }, esc(&text(&a["role"])), esc(&text(&a["bp"])),
        esc(&Some(text(&a["country"])).filter(|c| !c.is_empty()).unwrap_or_else(|| "auto".into())), esc(&standing), esc(&muted),
        esc(&text(&a["devices"])), if a["reset_pending"] == true { "yes" } else { "no" });
    out += "<h2>Players</h2><table><thead><tr><th>Player</th><th>Level</th><th>Cards</th><th>Decks bought</th><th>Decks kept</th><th>Items</th><th>Won / lost / drawn</th><th>Online</th></tr></thead><tbody>";
    for p in a["players"].as_array().into_iter().flatten() {
        let record = p["record"].as_array().map_or("-".into(), |r| r.iter().map(text).collect::<Vec<_>>().join(" / "));
        out += &format!("<tr><td>{}</td><td>{}</td><td>{} ({} kinds)</td><td>{}</td><td>{}</td><td>{}</td><td>{record}</td><td>{}</td></tr>",
            esc(&text(&p["name"])), esc(&text(&p["level"])), esc(&text(&p["cards"])), esc(&text(&p["kinds"])), esc(&text(&p["decks"])),
            esc(&text(&p["uploaded_decks"])), esc(&text(&p["items"])), if p["online"] == true { "yes" } else { "" });
    }
    out += "</tbody></table><p class=\"muted\">Cards are those beyond the starting deck.</p>";
    let form = |action: &str, label: &str, fields: &str, confirm: Option<&str>, danger: bool| {
        let confirm = confirm.map_or(String::new(), |c| format!(" data-confirm=\"{}\"", esc(c)));
        format!("<form method=\"post\" action=\"{PREFIX}/account\" class=\"action\"{confirm}>{}<input type=\"hidden\" name=\"who\" value=\"{}\">\
<input type=\"hidden\" name=\"action\" value=\"{action}\">{fields}<button{}>{}</button></form>", viewer.token(), esc(&name), if danger { " class=\"danger\"" } else { "" }, esc(label))
    };
    let reason = "<label>Reason <input name=\"reason\" maxlength=\"200\"></label>";
    out += "<h2>Moderation</h2><div class=\"actions\">";
    out += &form("kick", "Disconnect", "", None, false);
    if password {
        out += &form("mute", "Mute", &format!("<label>Minutes <input name=\"minutes\" inputmode=\"numeric\" placeholder=\"until unmuted\"></label>{reason}"), None, false);
        out += &form("unmute", "Unmute", "", None, false);
        out += &form("restrict", "Restrict", &format!("<label>Days <input name=\"days\" inputmode=\"numeric\" placeholder=\"until lifted\"></label>{reason}"), None, false);
        out += &form("ban", "Ban", &format!("<label>Days <input name=\"days\" inputmode=\"numeric\" placeholder=\"until lifted\"></label>{reason}"), Some(&format!("Ban {name}? Their game is disconnected and every device signed out.")), true);
        out += &form("lift", "Lift ban or restriction", "", None, false);
        out += &form("sign-out", "Sign out everywhere", "", Some(&format!("Sign {name} out on every device?")), false);
        out += &form("reset-code", "Make a reset code", "", Some(&format!("Make a password reset code for {name}?")), false);
    }
    let players: String = a["players"].as_array().into_iter().flatten().map(|p| format!("<option>{}</option>", esc(&text(&p["name"])))).collect();
    if !players.is_empty() { out += &form("rename", "Rename", &format!("<label>Player <select name=\"old\">{players}</select></label><label>New name <input name=\"new\" required maxlength=\"16\"></label>"), None, false); }
    out += "</div>";
    if viewer.admin() {
        out += "<h2>Admin</h2><div class=\"actions\">";
        out += &form("give-bp", "Give BP", "<label>BP <input name=\"amount\" inputmode=\"numeric\" required></label>", None, false);
        out += &form("set-country", "Set country", "<label>Code <input name=\"code\" maxlength=\"4\" placeholder=\"auto\" required></label>", None, false);
        if password {
            out += &form("set-role", "Set role", "<label>Role <select name=\"role\"><option>player</option><option>moderator</option><option>admin</option></select></label>",
                Some(&format!("Change {name}'s role?")), false);
        }
        out += "</div>";
    }
    out
}

fn invites(viewer: &Viewer, api: &Api) -> String {
    let mut out = String::new();
    let field = |label: &str, name: &str, hint: &str| format!("<label>{} <input name=\"{name}\" inputmode=\"numeric\" placeholder=\"{}\"></label>", esc(label), esc(hint));
    out += &format!("<h2>Invite codes</h2><p class=\"muted\">Players enter one when they make their account in the launcher (registration: invite).</p>\
<form method=\"post\" action=\"{PREFIX}/invites\" class=\"inline\">{}<input type=\"hidden\" name=\"action\" value=\"invite\">{}{}<button>Make an invite code</button></form>",
        viewer.token(), field("Sign-ups", "uses", "1"), field("Days", "days", "no end"));
    match run(api, viewer, &Op::Invites) {
        Err(e) => out += &failed(&e),
        Ok(v) => {
            let now = crate::logins::now();
            let list = v["invites"].as_array().cloned().unwrap_or_default();
            if list.is_empty() { out += "<p class=\"muted\">No invite codes yet.</p>" } else {
                out += "<table><thead><tr><th>Code</th><th>State</th><th>Sign-ups left</th><th>Ends</th><th>Issued by</th><th>Used by</th></tr></thead><tbody>";
                for i in list.iter().rev() {
                    let usable = i["uses"].as_u64() != Some(0) && i["expires"].as_i64().is_none_or(|t| t > now);
                    let ends = i["expires"].as_i64().and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("no end".into(), |t| t.format("%Y-%m-%d %H:%M UTC").to_string());
                    out += &format!("<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{} account(s)</td></tr>", esc(&text(&i["code"])),
                        if usable { "usable" } else { "used up or ended" }, esc(&i["uses"].as_u64().map_or("unlimited".into(), |n| n.to_string())), esc(&ends),
                        esc(&text(&i["issued_by"])), i["used_by"].as_array().map_or(0, Vec::len));
                }
                out += "</tbody></table>";
            }
        }
    }
    out += "<h2>BOOSTERPASS codes</h2><p class=\"muted\">Players enter them in the game's Main Menu → BOOSTERPASS for BP, once per account.</p>";
    if viewer.admin() {
        out += &format!("<form method=\"post\" action=\"{PREFIX}/invites\" class=\"inline\">{}<input type=\"hidden\" name=\"action\" value=\"add-code\">\
<label>Code <input name=\"code\" placeholder=\"random\" maxlength=\"32\"></label>{}{}<button>Add a code</button></form>", viewer.token(), field("BP", "bp", "500"), field("Accounts", "uses", "any number"));
    }
    match run(api, viewer, &Op::Codes) {
        Err(e) => out += &failed(&e),
        Ok(v) => {
            let list = v.as_array().cloned().unwrap_or_default();
            if list.is_empty() { out += "<p class=\"muted\">No codes yet.</p>" } else {
                out += "<table><thead><tr><th>Code</th><th>BP</th><th>Claimed</th><th>Left</th><th></th></tr></thead><tbody>";
                for c in list {
                    let used = c["claims"].as_array().map_or(0, Vec::len) as u64;
                    let left = c["uses"].as_u64().map_or("unlimited".into(), |n| n.saturating_sub(used).to_string());
                    let remove = if viewer.admin() { format!("<form method=\"post\" action=\"{PREFIX}/invites\" data-confirm=\"Remove this code?\">{}<input type=\"hidden\" name=\"action\" value=\"remove-code\">\
<input type=\"hidden\" name=\"code\" value=\"{}\"><button class=\"danger small\">Remove</button></form>", viewer.token(), esc(&text(&c["code"]))) } else { String::new() };
                    out += &format!("<tr><td><code>{}</code></td><td>{}</td><td>{used}</td><td>{}</td><td>{remove}</td></tr>", esc(&text(&c["code"])), esc(&text(&c["bp"])), esc(&left));
                }
                out += "</tbody></table>";
            }
        }
    }
    out
}

/// A setting's value from the Settings operation's rows.
fn setting(rows: &[Value], name: &str) -> String {
    rows.iter().find(|r| r["name"] == name).map(|r| text(&r["value"])).unwrap_or_default()
}

fn news(viewer: &Viewer, api: &Api) -> String {
    let rows = match run(api, viewer, &Op::Settings) { Ok(v) => v["settings"].as_array().cloned().unwrap_or_default(), Err(e) => return failed(&e) };
    let entries: Vec<Value> = serde_json::from_str(&setting(&rows, "news")).unwrap_or_default();
    let disabled = if viewer.admin() { "" } else { " disabled" };
    let mut out = format!("<h2>The game's Information list</h2><p class=\"muted\">Up to ten entries; players online see a change at once.</p>\
<form method=\"post\" action=\"{PREFIX}/news\" class=\"stack\">{}<input type=\"hidden\" name=\"action\" value=\"news\"><fieldset{disabled}>", viewer.token());
    for i in 0..10 {
        let e = entries.get(i);
        if e.is_none() && i > entries.len() { break }
        out += &format!("<div class=\"entry\"><label>Title <input name=\"title{i}\" maxlength=\"128\" value=\"{}\"></label><label>Text <textarea name=\"body{i}\" maxlength=\"2048\" rows=\"3\">{}</textarea></label></div>",
            esc(&e.map(|e| text(&e["title"])).unwrap_or_default()), esc(&e.map(|e| text(&e["body"])).unwrap_or_default()));
    }
    out += "<button>Save the news</button></fieldset></form>";
    if !viewer.admin() { out += "<p class=\"muted\">Only an admin changes the news.</p>" }
    let at: i64 = setting(&rows, "maintenance-at").parse().unwrap_or(0);
    let now = crate::logins::now();
    let state = if at == 0 { "No maintenance is planned.".to_string() } else {
        let when = chrono::DateTime::from_timestamp(at, 0).map_or(String::new(), |t| t.format("%Y-%m-%d %H:%M UTC").to_string());
        if at > now { format!("Maintenance starts at {when}, in {}: the game counts down until then.", ago((at - now) as u64)) }
        else { format!("Maintenance since {when}: only admins and moderators can sign in.") }
    };
    out += &format!("<h2>Maintenance</h2><p>{}</p><p class=\"muted\">Message: {}</p>", esc(&state), esc(&setting(&rows, "maintenance-message")));
    out += &format!("<form method=\"post\" action=\"{PREFIX}/news\" class=\"inline\" data-confirm=\"Start the maintenance countdown?\">{}<input type=\"hidden\" name=\"action\" value=\"maintenance\"><fieldset{disabled}>\
<label>In minutes <input name=\"minutes\" inputmode=\"numeric\" required placeholder=\"0: now\"></label><label>Message <input name=\"message\" maxlength=\"200\" placeholder=\"keep the current one\"></label>\
<button>Plan maintenance</button></fieldset></form>", viewer.token());
    if at != 0 {
        out += &format!("<form method=\"post\" action=\"{PREFIX}/news\" class=\"inline\">{}<input type=\"hidden\" name=\"action\" value=\"maintenance-off\"><fieldset{disabled}><button>Call maintenance off</button></fieldset></form>", viewer.token());
    }
    out
}

fn settings(viewer: &Viewer, api: &Api) -> String {
    let rows = match run(api, viewer, &Op::Settings) { Ok(v) => v["settings"].as_array().cloned().unwrap_or_default(), Err(e) => return failed(&e) };
    let disabled = if viewer.admin() { "" } else { " disabled" };
    let mut out = String::from(if viewer.admin() { "<p class=\"muted\">Each change applies without a restart, as its row says, and goes in the audit log.</p>" }
        else { "<p class=\"muted\">Only an admin changes settings.</p>" });
    out += "<table class=\"settings\"><thead><tr><th>Setting</th><th>Value</th><th></th></tr></thead><tbody>";
    for row in rows {
        let name = text(&row["name"]);
        if ["news", "maintenance-at", "maintenance-message"].contains(&name.as_str()) {
            out += &format!("<tr><td><strong>{}</strong><br><span class=\"muted\">{}</span></td><td colspan=\"2\"><a href=\"{PREFIX}/news\">On the News and maintenance page</a></td></tr>", esc(&name), esc(&text(&row["help"])));
            continue;
        }
        let value = text(&row["value"]);
        let input = match row["choices"].as_array() {
            Some(choices) => format!("<select name=\"value\">{}</select>", choices.iter().map(|c| { let c = text(c); format!("<option{}>{}</option>", if c == value { " selected" } else { "" }, esc(&c)) }).collect::<String>()),
            None => format!("<input name=\"value\" value=\"{}\">", esc(&value)),
        };
        let state = if row["set"] == true { "set" } else { "default" };
        let reset = if row["set"] == true { format!("<form method=\"post\" action=\"{PREFIX}/settings\">{}<input type=\"hidden\" name=\"name\" value=\"{}\"><input type=\"hidden\" name=\"action\" value=\"reset\"><fieldset{disabled}><button class=\"small\">Default</button></fieldset></form>", viewer.token(), esc(&name)) } else { String::new() };
        out += &format!("<tr><td><strong>{}</strong> <span class=\"badge\">{state}</span><br><span class=\"muted\">{} (applies {})</span></td>\
<td><form method=\"post\" action=\"{PREFIX}/settings\" class=\"inline\">{}<input type=\"hidden\" name=\"name\" value=\"{}\"><fieldset{disabled}>{input}<button class=\"small\">Save</button></fieldset></form></td><td>{reset}</td></tr>",
            esc(&name), esc(&text(&row["help"])), esc(&text(&row["applies"])), viewer.token(), esc(&name));
    }
    out + "</tbody></table>"
}

const MODERATION_OPS: &[&str] = &["kick", "ban", "restrict", "lift", "mute", "unmute", "sign-out", "reset-code", "rename", "maintenance", "set-role"];

fn audit_table(rows: &[Value]) -> String {
    if rows.is_empty() { return "<p class=\"muted\">Nothing yet.</p>".into() }
    let mut out = String::from("<table><thead><tr><th>When (UTC)</th><th>Who</th><th>From</th><th>What</th><th>On</th><th>Change</th></tr></thead><tbody>");
    for r in rows {
        let value = |v: &Value| if v.is_null() { String::new() } else { v.to_string() };
        let change = if r["before"].is_null() && r["after"].is_null() { value(&r["detail"]) } else { format!("{} → {}", value(&r["before"]), value(&r["after"])) };
        out += &format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"wrap\">{}</td></tr>", esc(&text(&r["at"]).replace('T', " ").replace('Z', "")),
            esc(&text(&r["actor"])), esc(&text(&r["origin"])), esc(&text(&r["op"])), esc(&text(&r["target"])), esc(&change));
    }
    out + "</tbody></table>"
}

fn moderation(api: &Api) -> String {
    let all = match api.apply(&Op::Accounts, "") { Ok(v) => v.as_array().cloned().unwrap_or_default(), Err(e) => return failed(&e) };
    let flagged: Vec<&Value> = all.iter().filter(|a| a["standing"].as_str().is_some_and(|s| s != "active") || a["muted"] == true).collect();
    let mut out = String::from("<h2>Banned, restricted or muted now</h2>");
    if flagged.is_empty() { out += "<p class=\"muted\">Nobody.</p>" } else {
        out += "<table><thead><tr><th>User ID</th><th>Players</th><th>Standing</th></tr></thead><tbody>";
        for a in flagged {
            let names: Vec<String> = a["players"].as_array().into_iter().flatten().map(|p| text(&p["name"])).collect();
            out += &format!("<tr><td><a href=\"{PREFIX}/account?who={}\">{}</a></td><td>{}</td><td>{}</td></tr>", encode(&text(&a["login"])), esc(&text(&a["login"])), esc(&names.join(", ")), standing_badge(a));
        }
        out += "</tbody></table>";
    }
    let rows = api.audit(500).unwrap_or_default();
    let rows: Vec<Value> = rows.into_iter().filter(|r| MODERATION_OPS.contains(&r.op.as_str())).take(50).map(|r| json!(r)).collect();
    out + "<h2>Recent moderation</h2>" + &audit_table(&rows)
}

fn audit(api: &Api) -> String {
    match api.audit(200) {
        Ok(rows) => format!("<p class=\"muted\">The newest 200 changes, from the panel, the command line and the API.</p>{}", audit_table(&rows.into_iter().map(|r| json!(r)).collect::<Vec<_>>())),
        Err(e) => failed(&e),
    }
}

fn backups(viewer: &Viewer, api: &Api) -> String {
    let list = match run(api, viewer, &Op::Backups) { Ok(v) => v["backups"].as_array().cloned().unwrap_or_default(), Err(e) => return failed(&e) };
    let mut out = String::from("<p class=\"muted\">The server backs its database up at start, every hour and at stop, and keeps the newest few. \
Copies off this machine are the host's job (see the hosting guide).</p>");
    if viewer.admin() {
        out += &format!("<form method=\"post\" action=\"{PREFIX}/backups\" class=\"inline\">{}<button>Back up now</button></form>", viewer.token());
    }
    if list.is_empty() { return out + "<p class=\"muted\">No backups yet.</p>" }
    out += "<table><thead><tr><th>Backup</th><th>Taken (UTC)</th><th>Size</th><th></th></tr></thead><tbody>";
    for b in list {
        let name = text(&b["name"]);
        let link = if viewer.admin() { format!("<a href=\"{PREFIX}/backups/download?name={}\" download>Download</a>", encode(&name)) } else { String::new() };
        out += &format!("<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{link}</td></tr>", esc(&name), esc(&text(&b["at"]).replace('T', " ").replace('Z', "")),
            b["bytes"].as_u64().map_or(String::new(), bytes));
    }
    out + "</tbody></table>"
}

/// A backup's database file, for an admin; the download is in the audit log.
fn download(r: &Request, viewer: &Viewer, api: &Api) -> Response {
    if !viewer.admin() { return refused("403 Forbidden", "Only an admin downloads backups.") }
    let name = r.query.get("name").cloned().unwrap_or_default();
    let Some(dir) = &api.data_dir else { return refused("404 Not Found", "This server keeps no data folder.") };
    let Some(found) = crate::store::backups(dir).into_iter().find(|b| b.file_name().is_some_and(|n| n.to_string_lossy() == name)) else {
        return refused("404 Not Found", "No such backup.");
    };
    match std::fs::read(found.join("server.db")) {
        Ok(body) => {
            api.note(&viewer.user, ORIGIN, "download-backup", &name);
            let file: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.').collect();
            answer("200 OK", "application/vnd.sqlite3", body, vec![("Content-Disposition", format!("attachment; filename=\"yo3-server-{file}.db\""))])
        }
        Err(e) => refused("500 Internal Server Error", &format!("The backup can't be read: {e}")),
    }
}

fn logs(r: &Request, api: &Api) -> String {
    let problems = r.query.get("problems").is_some_and(|v| v == "1");
    let lines = api.apply(&Op::Logs { limit: Some(500), problems }, "").map(|v| v["lines"].as_array().cloned().unwrap_or_default());
    let filter = if problems { format!("<a href=\"{PREFIX}/logs\">Show everything</a>") } else { format!("<a href=\"{PREFIX}/logs?problems=1\">Only warnings and errors</a>") };
    match lines {
        Ok(lines) => format!("<p class=\"muted\">The newest 500 lines since the server started, newest first. The whole log is in the server's log folder. {filter}</p>{}",
            log_table(&lines, "No lines yet.")),
        Err(e) => failed(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forms_are_decoded_and_html_escaped() {
        let f = form("a=1+2&b=%3Cscript%3E&c=%E2%9C%93&d=%zz&e");
        assert_eq!((f["a"].as_str(), f["b"].as_str(), f["c"].as_str(), f["d"].as_str(), f["e"].as_str()), ("1 2", "<script>", "✓", "%zz", ""));
        assert_eq!(esc("<a href=\"x\">'&'</a>"), "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;");
        assert_eq!(encode("Yugi Mutō&x"), "Yugi%20Mut%C5%8D%26x");
        assert_eq!(decode("%4"), "%4");
    }

    #[test]
    fn only_a_local_host_name_and_this_origin_pass() {
        let req = |headers: &str| Request::parse(&format!("POST /panel/x HTTP/1.1\r\n{headers}\r\n\r\n"));
        assert!(req("Host: localhost:8080").local_host());
        assert!(req("Host: 127.0.0.1").local_host());
        assert!(req("Host: [::1]:8080").local_host());
        assert!(!req("Host: evil.example:8080").local_host());
        assert!(!req("Host: localhost.evil.example").local_host());
        assert!(!req("").local_host());
        assert!(req("Host: localhost:8080\r\nOrigin: http://localhost:8080").same_origin());
        assert!(req("Host: localhost:8080\r\nSec-Fetch-Site: same-origin").same_origin());
        assert!(!req("Host: localhost:8080\r\nOrigin: http://evil.example").same_origin());
        assert!(!req("Host: localhost:8080\r\nOrigin: http://localhost:8080\r\nSec-Fetch-Site: cross-site").same_origin());
        assert!(!req("Host: localhost:8080").same_origin(), "neither header: not from a browser page of ours");
        assert!(!req("Host: localhost:8080\r\nOrigin: null").same_origin(), "a sandboxed or opaque page");
    }
}
