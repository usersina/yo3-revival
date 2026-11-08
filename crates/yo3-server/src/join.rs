//! The join page of the patch HTTP server: a friend opens `http://HOST:8080/join`, picks a player name and gets an
//! account (found by a secret key in the patch URL, see `accounts.rs`) with the `yo3.ini` to put in the game folder.
//! It replaces the host's `add-account` per friend. The key travels in plain HTTP: use it over a VPN or LAN, and
//! turn the page off (`--no-join`) on a server open to the internet.
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::accounts::{Directory, MAX_NAME};
use crate::lock::Locked;

/// What the page needs to know about the server.
#[derive(Clone, Debug)]
pub struct JoinInfo {
    pub gate_port: u16,
    pub http_port: u16,
    /// The host written into the lines when the request has no usable `Host` header.
    pub advertise: Option<String>,
}

/// An HTTP answer.
pub struct Response {
    pub status: &'static str,
    pub content_type: &'static str,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

fn html(status: &'static str, body: String) -> Response {
    Response { status, content_type: "text/html; charset=utf-8", headers: vec![], body: body.into_bytes() }
}

/// `path` (with its query) is the join page's, if so its answer. `host` is the request's `Host` header.
pub fn handle(path: &str, host: Option<&str>, peer: IpAddr, accounts: &Arc<Mutex<Directory>>, info: &JoinInfo) -> Option<Response> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let route = route.trim_end_matches('/');
    if route != "/join" && route != "/join/yo3.ini" {
        return None;
    }
    let params = parse_query(query);
    let param = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let host = host_of(host, info);
    if route == "/join/yo3.ini" {
        let key = param("key").unwrap_or_default();
        if !accounts.locked().has_key(key) {
            return Some(html("404 Not Found", page("Not found", "<p>No account has that key.</p>")));
        }
        return Some(Response {
            status: "200 OK",
            content_type: "text/plain; charset=utf-8",
            headers: vec![("Content-Disposition", "attachment; filename=\"yo3.ini\"".to_string())],
            body: ini(&host, info, key, "\r\n").into_bytes(),
        });
    }
    let now = Instant::now();
    let mut error = None;
    let mut fresh = None;
    {
        let mut d = accounts.locked();
        if let Some(name) = param("name") {
            match d.join(peer, name, now) {
                Ok(key) => fresh = Some(key),
                Err(e) => error = Some(e),
            }
        } else if param("keep").is_some() {
            match d.keep(peer) {
                Ok(key) => fresh = Some(key),
                Err(e) => error = Some(e),
            }
        }
    }
    let state = accounts.locked().join_state(peer);
    let mut body = String::new();
    if let Some(e) = &error {
        body += &format!("<p class=\"err\">{}</p>", esc(e));
    }
    // A repeat visit shows what the address already has; a fresh account comes first.
    let mut shown: Vec<&(String, Vec<String>)> = state.accounts.iter().collect();
    if let Some(key) = &fresh {
        shown.sort_by_key(|(k, _)| k != key);
        body += "<p class=\"ok\">Your account is ready.</p>";
    } else if !shown.is_empty() {
        body += "<p>This address already has an account, shown again below.</p>";
    }
    for (key, players) in shown {
        body += &account_html(&host, info, key, players);
    }
    if !state.accounts.is_empty() || fresh.is_some() {
        body += instructions();
    }
    if state.keepable && fresh.is_none() {
        body += "<h2>Keep my account</h2><p>You played here before keys existed: this address already has an account \
                 (players, cards, decks). Give it a key so it follows you and nothing is lost.</p>\
                 <form method=\"get\" action=\"/join\"><input type=\"hidden\" name=\"keep\" value=\"1\"><button>Keep my account</button></form>";
    }
    if state.can_join {
        let title = if state.accounts.is_empty() { "Get your account" } else { "Another account" };
        body += &format!(
            "<h2>{title}</h2><form method=\"get\" action=\"/join\"><label>Player name (1 to {MAX_NAME} characters) \
             <input name=\"name\" maxlength=\"{MAX_NAME}\" required autofocus></label> <button>Join</button></form>"
        );
    }
    body += "<p class=\"note\">The key in your yo3.ini is your account's secret: it travels in a plain HTTP address, so \
             use this server over a VPN or a LAN and never share the key.</p>";
    let status = if error.is_some() { "400 Bad Request" } else { "200 OK" };
    Some(html(status, page("Join", &body)))
}

fn account_html(host: &str, info: &JoinInfo, key: &str, players: &[String]) -> String {
    let names = players.iter().map(|p| esc(p)).collect::<Vec<_>>().join(", ");
    format!(
        "<h2>Account of {names}</h2><p>Your <code>yo3.ini</code> (the <code>[Yo3]</code> section):</p><pre>{}</pre>\
         <p><a class=\"btn\" href=\"/join/yo3.ini?key={}\">Download yo3.ini</a></p>",
        esc(&ini(host, info, key, "\n")),
        esc(key)
    )
}

fn instructions() -> &'static str {
    "<h2>What to do</h2><ol>\
     <li>Download <code>yo3.ini</code> above.</li>\
     <li>Put it in the game folder, <code>C:\\Program Files (x86)\\Konami\\Yu-Gi-Oh! ONLINE 3</code>, replacing the one \
     there (keep a backup copy first). Windows may refuse the copy: copy it from Explorer and accept the administrator \
     prompt, or change the file's permissions first.</li>\
     <li>Start the game and log in with any name and password.</li>\
     <li>Pick your player, then open Main Menu, Edit Deck once (it saves your starter deck).</li></ol>"
}

/// The five `yo3.ini` lines (the `[Yo3]` section) for `key`, joined by `eol`.
fn ini(host: &str, info: &JoinInfo, key: &str, eol: &str) -> String {
    let (gate, http) = (info.gate_port, info.http_port);
    [
        "[Yo3]".to_string(),
        format!("GateServerAddress={host}"),
        format!("GateServerPort={gate}"),
        format!("PatchUpdateUrl=http://{host}:{http}/yo3/{key}/"),
        format!("PatchDataUrl=http://{host}:{http}/ygoda/patch/"),
        format!("InfoURL=http://{host}:{http}/app/"),
    ]
    .join(eol)
        + eol
}

/// The host the friend typed (the `Host` header without its port), or the advertised one, or a placeholder. Only
/// characters a host name or address has are taken.
fn host_of(header: Option<&str>, info: &JoinInfo) -> String {
    let name = header.map(|h| match h.strip_prefix('[') {
        Some(rest) => rest.split(']').next().map_or(h.to_string(), |a| format!("[{a}]")),
        None => h.rsplit_once(':').filter(|(_, p)| p.chars().all(|c| c.is_ascii_digit())).map_or(h, |(n, _)| n).to_string(),
    });
    name.filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c)))
        .or_else(|| info.advertise.clone())
        .unwrap_or_else(|| "HOST".to_string())
}

/// `a=b&c=d` decoded (`+` is a space, `%XX` bytes as UTF-8).
fn parse_query(q: &str) -> Vec<(String, String)> {
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

fn esc(s: &str) -> String {
    s.chars().fold(String::new(), |mut o, c| {
        match c {
            '&' => o += "&amp;",
            '<' => o += "&lt;",
            '>' => o += "&gt;",
            '"' => o += "&quot;",
            '\'' => o += "&#39;",
            c => o.push(c),
        }
        o
    })
}

fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title} - Yu-Gi-Oh! ONLINE 3 revival</title><style>\
         body{{font:16px/1.5 system-ui,sans-serif;max-width:46rem;margin:2rem auto;padding:0 1rem;color:#222}}\
         pre{{background:#f3f3f3;padding:.8rem;overflow-x:auto}}code{{background:#f3f3f3;padding:0 .2rem}}\
         .err{{color:#a00;font-weight:bold}}.ok{{color:#060;font-weight:bold}}.note{{color:#555;font-size:.9rem}}\
         .btn,button{{font:inherit;padding:.3rem .8rem}}input{{font:inherit}}</style></head><body>\
         <h1>Yu-Gi-Oh! ONLINE 3 revival</h1>{body}</body></html>"
    )
}
