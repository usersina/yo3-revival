//! The host's BP commands (`give-bp`, `add-code`, `remove-code`, `codes`), on a running server too.
//!
//! A running server keeps the accounts in memory and rewrites `accounts.json` on every change, so a command that edited
//! the file would be overwritten. Instead the server writes [`LIVE_FILE`] into its data directory at start: its HTTP port
//! and a random token, readable by the host's user only. A command finds it, and when the server answers on that port
//! it sends the operation there (`GET /admin?…&token=…`, from this PC only: a loopback peer with the token); else it
//! edits the file itself. The other admin commands still need the server stopped: [`running`] tells them it isn't.
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use crate::accounts::Directory;
pub use crate::manager::{ManagerOp, manage};
pub struct Response {
    pub status: &'static str,
    pub content_type: &'static str,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}
use crate::lock::Locked;

/// The running server's port and token, in its data directory.
pub const LIVE_FILE: &str = "admin.json";

/// An admin operation on the accounts.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// BP for the account at an address or with a player of that name.
    GiveBp { who: String, amount: u32 },
    AddCode { code: String, bp: u32, uses: Option<u32> },
    RemoveCode { code: String },
    Codes,
    /// An invite code for `uses` sign-ups (any number if none), working for `days` (for good if none).
    Invite { uses: Option<u32>, days: Option<u32> },
    Invites,
    /// A reset code for a player who lost their password and recovery code (checked with them first).
    ResetCode { user: String },
    SetRole { user: String, role: String },
    /// A password account made by the host: `hash` is its first password's PHC string, made by the command.
    AddUser { user: String, hash: String },
    /// The live settings (#24): every one with its value and when it applies.
    Settings,
    Set { name: String, value: String },
    Reset { name: String },
}

impl Op {
    fn query(&self) -> String {
        match self {
            Op::GiveBp { who, amount } => format!("op=give-bp&who={}&amount={amount}", encode(who)),
            Op::AddCode { code, bp, uses } => {
                format!("op=add-code&code={}&bp={bp}", encode(code)) + &uses.map(|n| format!("&uses={n}")).unwrap_or_default()
            }
            Op::RemoveCode { code } => format!("op=remove-code&code={}", encode(code)),
            Op::Codes => "op=codes".into(),
            Op::Invite { uses, days } => "op=invite".to_string() + &uses.map(|n| format!("&uses={n}")).unwrap_or_default()
                + &days.map(|n| format!("&days={n}")).unwrap_or_default(),
            Op::Invites => "op=invites".into(),
            Op::ResetCode { user } => format!("op=reset-code&user={}", encode(user)),
            Op::SetRole { user, role } => format!("op=set-role&user={}&role={}", encode(user), encode(role)),
            Op::AddUser { user, hash } => format!("op=add-user&user={}&hash={}", encode(user), encode(hash)),
            Op::Settings => "op=settings".into(),
            Op::Set { name, value } => format!("op=set&name={}&value={}", encode(name), encode(value)),
            Op::Reset { name } => format!("op=reset&name={}", encode(name)),
        }
    }

    fn from_query(q: &[(String, String)]) -> Result<Op, String> {
        let get = |k: &str| q.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        let num = |k: &str| get(k).ok_or(format!("{k} missing"))?.parse::<u32>().map_err(|_| format!("{k} is not a number"));
        match get("op").as_deref() {
            Some("give-bp") => Ok(Op::GiveBp { who: get("who").ok_or("who missing")?, amount: num("amount")? }),
            Some("add-code") => Ok(Op::AddCode { code: get("code").ok_or("code missing")?, bp: num("bp")?, uses: num("uses").ok() }),
            Some("remove-code") => Ok(Op::RemoveCode { code: get("code").ok_or("code missing")? }),
            Some("codes") => Ok(Op::Codes),
            Some("invite") => Ok(Op::Invite { uses: num("uses").ok(), days: num("days").ok() }),
            Some("invites") => Ok(Op::Invites),
            Some("reset-code") => Ok(Op::ResetCode { user: get("user").ok_or("user missing")? }),
            Some("set-role") => Ok(Op::SetRole { user: get("user").ok_or("user missing")?, role: get("role").ok_or("role missing")? }),
            Some("add-user") => Ok(Op::AddUser { user: get("user").ok_or("user missing")?, hash: get("hash").ok_or("hash missing")? }),
            Some("settings") => Ok(Op::Settings),
            Some("set") => Ok(Op::Set { name: get("name").ok_or("name missing")?, value: get("value").unwrap_or_default() }),
            Some("reset") => Ok(Op::Reset { name: get("name").ok_or("name missing")? }),
            _ => Err("unknown operation".into()),
        }
    }
}

/// Applies a settings operation (`Settings`, `Set`, `Reset`); None for the others, which go to [`apply`].
pub fn apply_settings(live: &crate::settings::Live, op: &Op) -> Option<Result<String, String>> {
    match op {
        Op::Settings => Some(Ok(live.list())),
        Op::Set { name, value } => Some(live.set(name, value)),
        Op::Reset { name } => Some(live.reset(name)),
        _ => None,
    }
}

/// Applies `op` to the accounts: what to print, or why not.
pub fn apply(d: &mut Directory, op: &Op) -> Result<String, String> {
    match op {
        Op::GiveBp { who, amount } => d.give_bp(who, *amount).map(|(to, total)| format!("gave {amount} BP to {to}: {total} BP now")),
        Op::AddCode { code, bp, uses } => d.add_code(code, *bp, *uses).map(|()| {
            let uses = uses.map_or("any number of accounts".to_string(), |n| format!("{n} account(s)"));
            format!("code {code}: {bp} BP, once per account, for {uses}. Players enter it in Main Menu -> BOOSTERPASS.")
        }),
        Op::RemoveCode { code } => d.remove_code(code).map(|()| format!("code {code} removed")),
        Op::Codes => Ok(if d.codes().is_empty() {
            "no codes (add one with add-code)".into()
        } else {
            d.codes()
                .iter()
                .map(|c| {
                    let left = c.uses.map_or("unlimited".to_string(), |n| format!("{} of {n} left", n.saturating_sub(c.used_by.len() as u32)));
                    let claims: String = c.used_by.iter().zip(&c.claimed_at).map(|(&user, at)| {
                        format!("\n  claimed by user {user} ({}) at {}", d.player_names(user).join(", "), at.as_deref().unwrap_or("an unrecorded time"))
                    }).collect();
                    format!("{}\t{} BP\tused by {} account(s)\t{left}{claims}", c.code, c.bp, c.used_by.len())
                })
                .collect::<Vec<_>>()
                .join("\n")
        }),
        Op::Invite { uses, days } => {
            let expires = days.map(|n| crate::logins::now() + i64::from(n) * 24 * 3600);
            let code = d.add_invite(*uses, expires, "host")?;
            let uses = uses.map_or("any number of sign-ups".to_string(), |n| format!("{n} sign-up(s)"));
            let until = days.map_or("with no end date".to_string(), |n| format!("for {n} day(s)"));
            Ok(format!("invite code {code}: {uses}, {until}. Players enter it when they make their account in the launcher."))
        }
        Op::Invites => {
            let now = crate::logins::now();
            Ok(if d.invites().is_empty() { "no invite codes (make one with invite)".into() } else {
                d.invites().iter().map(|i| {
                    let state = if i.usable(now) { "usable" } else { "used up or expired" };
                    let left = i.uses.map_or("unlimited".to_string(), |n| format!("{n} left"));
                    let until = i.expires.and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("no end".to_string(), |t| format!("until {}", t.format("%Y-%m-%d %H:%M UTC")));
                    format!("{}\t{state}\t{left}\t{until}\tissued by {}\tused by {} account(s)", i.code, i.issued_by, i.used_by.len())
                }).collect::<Vec<_>>().join("\n")
            })
        }
        Op::ResetCode { user } => {
            let code = crate::logins::new_code();
            let expires = crate::logins::now() + crate::logins::RESET_TTL;
            d.issue_reset(user, crate::logins::digest(&crate::logins::normalize_code(&code)), expires)?;
            Ok(format!("reset code for {user}: {code}. It works once, for 24 hours, in the launcher's \"Forgot password\"."))
        }
        Op::SetRole { user, role } => {
            let parsed: crate::logins::Role = role.parse()?;
            d.set_role(user, parsed).map(|()| format!("{user} is now {}", role.to_ascii_lowercase()))
        }
        Op::AddUser { user, hash } => {
            if !crate::logins::valid_user_id(user) { return Err(format!("{user:?} is not a User ID: 3 to 16 letters, digits, _ or -")) }
            if !hash.starts_with("$argon2id$") { return Err("the password hash is not argon2id".into()) }
            let recovery = crate::logins::new_code();
            let login = crate::logins::Login { user: user.clone(), password: hash.clone(), role: Default::default(), standing: Default::default(),
                recovery: crate::logins::digest(&crate::logins::normalize_code(&recovery)), reset: None, devices: vec![] };
            d.register(login, None, crate::logins::Registration::Open).map_err(|e| if e == "user_id_taken" { format!("the User ID {user} is taken") } else { e })?;
            Ok(format!("recovery code: {recovery}"))
        }
        Op::Settings | Op::Set { .. } | Op::Reset { .. } => Err("a settings operation (apply_settings)".into()),
    }
}

/// Where a running server takes admin operations.
#[derive(Clone, Debug, PartialEq)]
pub struct Live {
    pub port: u16,
    pub token: String,
}

/// Writes [`LIVE_FILE`] for a server on `port` (readable by the host's user only); its token.
pub fn write_live(data_dir: &Path, port: u16) -> io::Result<String> {
    let token = crate::accounts::new_key();
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join(LIVE_FILE);
    let _ = std::fs::remove_file(&path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(&path)?.write_all(json!({ "port": port, "token": token }).to_string().as_bytes())?;
    Ok(token)
}

/// The running server of `data_dir`, if its [`LIVE_FILE`] is there and the server answers on its port.
pub fn running(data_dir: &Path) -> Option<Live> {
    let text = std::fs::read_to_string(data_dir.join(LIVE_FILE)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let live = Live { port: v["port"].as_u64()? as u16, token: v["token"].as_str()?.to_string() };
    TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(2)).ok()?;
    Some(live)
}

/// Sends `op` to a running server: what it printed, or why not.
pub fn send(live: &Live, op: &Op) -> Result<String, String> {
    let fail = |e: io::Error| format!("the server didn't answer: {e}");
    let mut s = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(5)).map_err(fail)?;
    s.set_read_timeout(Some(Duration::from_secs(10))).map_err(fail)?;
    let request = format!("GET /admin?{}&token={} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", op.query(), live.token);
    s.write_all(request.as_bytes()).map_err(fail)?;
    let mut response = String::new();
    s.read_to_string(&mut response).map_err(fail)?;
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
    if head.starts_with("HTTP/1.1 200") { Ok(body.trim().to_string()) } else { Err(body.trim().to_string()) }
}

/// The patch HTTP server's `/admin`: `path` (with its query) is an admin request, if so its answer. Only from this PC
/// (a loopback peer) with the token; the server logs the route, never the query.
pub fn handle(path: &str, peer: IpAddr, token: Option<&str>, accounts: &Arc<Mutex<Directory>>, hub: &Arc<Mutex<crate::front::Hub>>,
    settings: &crate::settings::Live) -> Option<Response> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    if route.trim_end_matches('/') != "/admin" {
        return None;
    }
    let text = |status, body: String| Response { status, content_type: "text/plain; charset=utf-8", headers: vec![], body: body.into_bytes() };
    let q = parse_query(query);
    let given = q.iter().find(|(k, _)| k == "token").map(|(_, v)| v.as_str());
    let allowed = peer.is_loopback() && matches!((token, given), (Some(t), Some(g)) if same(g, t));
    if !allowed {
        tracing::warn!("[http] {peer}: an admin request refused (not from this PC, or a wrong token)");
        return Some(text("403 Forbidden", "refused: admin requests come from the server's own PC, with its token".into()));
    }
    let op = Op::from_query(&q);
    let applied = op.clone().and_then(|op| {
        if let Some(done) = apply_settings(settings, &op) { return done }
        let mut d = accounts.locked();
        let done = apply(&mut d, &op)?;
        // A running server shows the gift at once (Hub::push_bp), not only at the player's next login or purchase.
        if let Op::GiveBp { who, amount } = &op {
            let (players, named) = (d.players_of(who), d.named_players_of(who));
            if let Some(bp) = players.first().and_then(|&p| d.player_bp(p)) {
                drop(d);
                let mut hub = hub.locked();
                hub.push_bp(&players, bp);
                hub.mail_bp_gift(&named, *amount, bp);
            }
        }
        Ok(done)
    });
    Some(match applied {
        Ok(done) => {
            // Codes and recovery codes are secrets: their answers stay out of the log.
            match op {
                Ok(Op::Invite { .. } | Op::ResetCode { .. } | Op::AddUser { .. } | Op::Invites) => tracing::info!("[admin] {} done", q.iter().find(|(k, _)| k == "op").map_or("", |(_, v)| v.as_str())),
                _ => tracing::info!("[admin] {done}"),
            }
            text("200 OK", done)
        }
        Err(why) => text("400 Bad Request", why),
    })
}

/// Compares two tokens in time that doesn't depend on where they differ.
pub(crate) fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Percent-encodes a query value (names may hold spaces and other characters).
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}
pub(crate) fn parse_query(q: &str) -> Vec<(String, String)> {
    q.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (decode(k), decode(v))
    }).collect()
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if b.get(i + 1..i + 3).is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) => {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'?'));
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
