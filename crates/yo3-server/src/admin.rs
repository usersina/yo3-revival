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
            _ => Err("unknown operation".into()),
        }
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
                    format!("{}\t{} BP\tused by {} account(s)\t{left}", c.code, c.bp, c.used_by.len())
                })
                .collect::<Vec<_>>()
                .join("\n")
        }),
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
pub fn handle(path: &str, peer: IpAddr, token: Option<&str>, accounts: &Arc<Mutex<Directory>>) -> Option<Response> {
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
    Some(match Op::from_query(&q).and_then(|op| apply(&mut accounts.locked(), &op)) {
        Ok(done) => {
            tracing::info!("[admin] {done}");
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
