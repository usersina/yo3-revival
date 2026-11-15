//! The admin API (#25): every administrative operation, one versioned JSON endpoint, and the audit log.
//!
//! `POST /api/admin/v1/<op>` on the server's loopback HTTP port, with `Authorization: Bearer <token>` (the token in
//! `data/admin.json`, readable by the host's user only) and the operation as a JSON object ([`Op`], tagged by `op`).
//! The answer is `{"ok": true, "result": ...}` or `{"ok": false, "error": "..."}`. The CLI, the desktop app and later
//! the admin panel (#36) are its clients.
//!
//! Each operation is one function ([`Api::apply`]): it checks its values, applies the change to the running server's
//! state (in memory and in the database together), then writes an audit row: who, from where, what, on what, the
//! values before and after. With no server running, the CLI runs the same function on the data folder itself (it
//! holds the folder's lock meanwhile), and those changes are audited too, from "offline".
//!
//! No player connection reaches these routes: the TLS HTTP role forwards only the game's files (`transport.rs`), and
//! a request without the token is refused even from this machine.
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::accounts::Directory;
use crate::admin::{Live, Response, same};
use crate::front::Hub;
use crate::lock::Locked;
use crate::store::AuditRow;

/// The path every operation is under.
pub const PREFIX: &str = "/api/admin/v1/";

/// An administrative operation, as the API takes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Op {
    /// Whether the server is ready, who is online, how many duels run.
    Status,
    Accounts,
    /// A key account with these players (none: the starting players).
    CreateAccount { #[serde(default)] names: Vec<String> },
    AddPlayer { who: String, name: String },
    /// The key of an account (a User ID or a player's name), made if it has none.
    Key { who: String },
    Rename { old: String, new: String },
    SetCountry { who: String, code: String },
    Countries,
    Titles,
    GrantTitle { player: String, title: String },
    GiveBp { who: String, amount: u32 },
    Codes,
    /// A BOOSTERPASS code; a random one when `code` is empty.
    AddCode { #[serde(default)] code: String, bp: u32, #[serde(default)] uses: Option<u32> },
    RemoveCode { code: String },
    Invite { #[serde(default)] uses: Option<u32>, #[serde(default)] days: Option<u32> },
    Invites,
    /// A password account made by the host: `hash` is its first password's PHC string, made by the client.
    AddUser { user: String, hash: String },
    ResetCode { user: String },
    SetRole { user: String, role: String },
    Settings,
    Set { name: String, value: String },
    Reset { name: String },
    /// The newest audit rows (50 when no limit is given).
    Audit { #[serde(default)] limit: Option<u32> },
    Shutdown,
}

impl Op {
    /// Its name in the API's path and the audit log.
    pub fn name(&self) -> String {
        serde_json::to_value(self).ok().and_then(|v| v["op"].as_str().map(str::to_owned)).unwrap_or_default()
    }

    /// Whether it changes anything (and so writes an audit row).
    pub fn changes(&self) -> bool {
        !matches!(self, Op::Status | Op::Accounts | Op::Countries | Op::Titles | Op::Codes | Op::Invites | Op::Settings | Op::Audit { .. })
    }

    /// What it acts on, for the audit log.
    fn target(&self) -> String {
        match self {
            Op::AddPlayer { who, .. } | Op::Key { who } | Op::SetCountry { who, .. } | Op::GiveBp { who, .. } => who.clone(),
            Op::Rename { old, .. } => old.clone(),
            Op::GrantTitle { player, .. } => player.clone(),
            Op::AddUser { user, .. } | Op::ResetCode { user } | Op::SetRole { user, .. } => user.clone(),
            Op::Set { name, .. } | Op::Reset { name } => name.clone(),
            Op::CreateAccount { names } => names.join(", "),
            Op::AddCode { code, .. } | Op::RemoveCode { code } => code.clone(),
            _ => String::new(),
        }
    }

    /// Its values for the audit log, without secrets (a password's hash).
    fn detail(&self) -> Option<Value> {
        let mut v = serde_json::to_value(self).ok()?;
        let fields = v.as_object_mut()?;
        fields.remove("op");
        fields.remove("hash");
        (!fields.is_empty()).then_some(v)
    }
}

/// What the status operation reports besides the players: set by a running server.
#[derive(Clone, Debug, Default)]
pub struct Status { pub engine_problem: Option<String>, pub listen: Option<SocketAddr>, pub diagnostic_id: Option<String> }

/// What the operations act on: a running server's state, or a stopped server's data folder.
pub struct Api {
    pub accounts: Arc<Mutex<Directory>>,
    pub settings: Arc<crate::settings::Live>,
    /// The online players (none when the server isn't running).
    pub hub: Option<Arc<Mutex<Hub>>>,
    /// Stops the running server.
    pub shutdown: Option<tokio::sync::watch::Sender<bool>>,
    pub status: Status,
    /// Where the audit log is kept (none: in memory, for tests).
    pub data_dir: Option<PathBuf>,
    /// The game, for the country and title tables.
    pub game_dir: PathBuf,
    pub titles: Option<Arc<crate::titles::TitleData>>,
    /// "server" for a running server's operations, "offline" for the CLI's on a stopped server's folder.
    pub origin: &'static str,
    memory: Mutex<Vec<AuditRow>>,
}

type Done = Result<(Value, Option<Value>, Option<Value>), String>;

impl Api {
    pub fn new(accounts: Arc<Mutex<Directory>>, settings: Arc<crate::settings::Live>, data_dir: Option<PathBuf>, game_dir: PathBuf, origin: &'static str) -> Api {
        Api { accounts, settings, hub: None, shutdown: None, status: Status::default(), data_dir, game_dir, titles: None, origin, memory: Mutex::default() }
    }

    /// Runs `op` for `actor` and writes its audit row: its result, or why it was refused.
    pub fn apply(&self, op: &Op, actor: &str) -> Result<Value, String> {
        let (result, before, after) = self.run(op)?;
        if op.changes() {
            let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: actor.into(),
                origin: self.origin.into(), op: op.name(), target: op.target(), before, after, detail: op.detail() };
            match &self.data_dir {
                Some(dir) => if let Err(e) = crate::store::audit(dir, &row) { tracing::error!("audit row not written ({}): {e}", row.op) },
                None => self.memory.locked().push(row),
            }
        }
        Ok(result)
    }

    /// The newest `limit` audit rows.
    pub fn audit(&self, limit: u32) -> Result<Vec<AuditRow>, String> {
        match &self.data_dir {
            Some(dir) => crate::store::audit_rows(dir, limit).map_err(|e| e.to_string()),
            None => Ok(self.memory.locked().iter().rev().take(limit as usize).cloned().collect()),
        }
    }

    fn titles(&self) -> Result<Arc<crate::titles::TitleData>, String> {
        if let Some(t) = &self.titles { return Ok(t.clone()) }
        let data = yo3_data::GameData::load(&self.game_dir).map_err(|e| format!("the game's titles: {e}"))?;
        Ok(Arc::new(crate::titles::TitleData::from_game(&data)))
    }

    /// The operation itself: its result, and the values before and after the change.
    fn run(&self, op: &Op) -> Done {
        let message = |text: String| json!({ "message": text });
        let plain = |r: Result<String, String>| r.map(|t| (message(t), None, None));
        match op {
            Op::Status => {
                let (players, online, duels) = match &self.hub {
                    Some(hub) => { let hub = hub.locked(); (json!(hub.manager_players()), hub.online(), hub.tables.len()) }
                    None => (json!([]), 0, 0),
                };
                Ok((json!({ "running": self.hub.is_some(), "ready": self.status.engine_problem.is_none(), "engine_problem": self.status.engine_problem,
                    "listen": self.status.listen.map(|l| l.to_string()), "release": yo3_tunnel::BUILD_VERSION, "diagnostic_id": self.status.diagnostic_id,
                    "players": players, "online": online, "duels": duels }), None, None))
            }
            Op::Shutdown => {
                let stop = self.shutdown.as_ref().ok_or("the server isn't running")?;
                stop.send_replace(true);
                Ok((json!({ "stopping": true, "message": "the server is stopping" }), None, None))
            }
            Op::Settings => Ok((message(self.settings.list()), None, None)),
            Op::Set { name, value } => {
                let before = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                let done = self.settings.set(name, value)?;
                let after = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                Ok((message(done), before, after))
            }
            Op::Reset { name } => {
                let before = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                let done = self.settings.reset(name)?;
                let after = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                Ok((message(done), before, after))
            }
            Op::Audit { limit } => {
                let rows = self.audit(limit.unwrap_or(50).min(10_000))?;
                Ok((json!({ "rows": rows }), None, None))
            }
            Op::Countries => {
                let codes = yo3_data::countries(&self.game_dir).map_err(|e| e.to_string())?;
                Ok((json!({ "countries": codes, "message": codes.join(" ") }), None, None))
            }
            Op::Titles => {
                let titles = self.titles()?;
                let mut names: Vec<_> = titles.by_name.keys().cloned().collect();
                names.sort();
                Ok((json!({ "titles": names, "message": names.join("\n") }), None, None))
            }
            Op::GrantTitle { player, title } => {
                let titles = self.titles()?;
                let id = crate::titles::by_name(&titles, title).ok_or_else(|| format!("no title named {title:?}; `titles` lists them"))?;
                let mut d = self.accounts.locked();
                let player_id = d.player_id_named(player).ok_or_else(|| format!("no player named {player:?}"))?;
                d.update_player(player_id, |p| { p.items.insert(id); })?;
                plain(Ok(format!("{player} has the title {} (they wear it in Avatar Settings)", titles.name(id).unwrap_or(title))))
            }
            Op::GiveBp { who, amount } => {
                let mut d = self.accounts.locked();
                let before = d.user_of(who).and_then(|u| d.dpoint(u));
                let (to, total) = d.give_bp(who, *amount)?;
                let (players, named) = (d.players_of(who), d.named_players_of(who));
                drop(d);
                // A running server shows the gift at once (Hub::push_bp), not only at the player's next login or purchase.
                if let Some(hub) = &self.hub {
                    let mut hub = hub.locked();
                    hub.push_bp(&players, total);
                    hub.mail_bp_gift(&named, *amount, total);
                }
                Ok((json!({ "bp": total, "message": format!("gave {amount} BP to {to}: {total} BP now") }), before.map(|b| json!(b)), Some(json!(total))))
            }
            Op::SetRole { user, role } => {
                let parsed: crate::logins::Role = role.parse()?;
                let mut d = self.accounts.locked();
                let before = d.login_of(user).map(|(_, l)| json!(l.role));
                d.set_role(user, parsed)?;
                Ok((message(format!("{user} is now {}", role.to_ascii_lowercase())), before, Some(json!(parsed))))
            }
            Op::Rename { old, new } => {
                self.accounts.locked().rename(old, new)?;
                Ok((message(format!("{old} is now {new} (online, at the player's next login)")), Some(json!(old)), Some(json!(new))))
            }
            Op::SetCountry { who, code } => {
                let code = if code.eq_ignore_ascii_case("auto") { String::new() } else { code.to_ascii_uppercase() };
                if !code.is_empty() {
                    let known = yo3_data::countries(&self.game_dir).map_err(|e| e.to_string())?;
                    if !known.contains(&code) { return Err(format!("{code} is not in the client's country table; `countries` lists them")) }
                }
                let mut d = self.accounts.locked();
                let user = d.user_of(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
                let before = d.account_of(user).map(|a| json!(a.country));
                d.set_country_of(user, &code)?;
                let shown = if code.is_empty() { "auto (the game's language)".to_string() } else { code.clone() };
                Ok((message(format!("{who}'s country is now {shown}")), before, Some(json!(code))))
            }
            _ => self.accounts_op(op),
        }
    }

    /// The operations on the accounts alone.
    fn accounts_op(&self, op: &Op) -> Done {
        let mut d = self.accounts.locked();
        let done = |v: Value| Ok((v, None, None));
        match op {
            Op::Accounts => done(Value::Array(d.entries().map(|(login, key, account)| {
                let role = login.and_then(|u| d.login_of(u)).map(|(_, l)| json!(l.role));
                json!({ "user_id": account.user_id, "login": login, "has_key": key.is_some(), "role": role, "bp": account.dpoint,
                    "country": account.country,
                    "players": account.players.iter().map(|p| json!({ "player_id": p.player_id, "name": p.name, "slot": p.index })).collect::<Vec<_>>() })
            }).collect())),
            Op::CreateAccount { names } => {
                let key = d.add_account(names)?;
                let made = d.entries().find(|(_, k, _)| *k == Some(key.as_str())).map(|(_, _, a)| a.players.iter().map(|p| p.name.clone()).collect::<Vec<_>>()).unwrap_or_default();
                done(json!({ "created": made.first(), "players": made, "key": key }))
            }
            Op::AddPlayer { who, name } => {
                let user = d.user_of(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
                d.add_player_to(user, name)?;
                done(json!({ "created": name, "message": format!("{who}'s account has a player named {name}") }))
            }
            Op::Key { who } => done(json!({ "key": d.key_of(who)? })),
            Op::Codes => done(Value::Array(d.codes().iter().map(|c| json!({
                "code": c.code, "bp": c.bp, "uses": c.uses,
                "claims": c.used_by.iter().zip(&c.claimed_at).map(|(&user, at)| json!({ "user_id": user, "players": d.player_names(user), "at": at })).collect::<Vec<_>>()
            })).collect())),
            Op::AddCode { code, bp, uses } => {
                let code = if code.trim().is_empty() { d.new_code() } else { code.trim().to_string() };
                d.add_code(&code, *bp, *uses)?;
                let users = uses.map_or("any number of accounts".to_string(), |n| format!("{n} account(s)"));
                done(json!({ "code": code, "message": format!("code {code}: {bp} BP, once per account, for {users}. Players enter it in Main Menu -> BOOSTERPASS.") }))
            }
            Op::RemoveCode { code } => { d.remove_code(code)?; done(json!({ "removed": code, "message": format!("code {code} removed") })) }
            Op::Invite { uses, days } => {
                let expires = days.map(|n| crate::logins::now() + i64::from(n) * 24 * 3600);
                let code = d.add_invite(*uses, expires, "host")?;
                let users = uses.map_or("any number of sign-ups".to_string(), |n| format!("{n} sign-up(s)"));
                let until = days.map_or("with no end date".to_string(), |n| format!("for {n} day(s)"));
                done(json!({ "code": code, "message": format!("invite code {code}: {users}, {until}. Players enter it when they make their account in the launcher.") }))
            }
            Op::Invites => {
                let now = crate::logins::now();
                let text = if d.invites().is_empty() { "no invite codes (make one with invite)".to_string() } else {
                    d.invites().iter().map(|i| {
                        let state = if i.usable(now) { "usable" } else { "used up or expired" };
                        let left = i.uses.map_or("unlimited".to_string(), |n| format!("{n} left"));
                        let until = i.expires.and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("no end".to_string(), |t| format!("until {}", t.format("%Y-%m-%d %H:%M UTC")));
                        format!("{}\t{state}\t{left}\t{until}\tissued by {}\tused by {} account(s)", i.code, i.issued_by, i.used_by.len())
                    }).collect::<Vec<_>>().join("\n")
                };
                done(json!({ "invites": d.invites(), "message": text }))
            }
            Op::ResetCode { user } => {
                let code = crate::logins::new_code();
                let expires = crate::logins::now() + crate::logins::RESET_TTL;
                d.issue_reset(user, crate::logins::digest(&crate::logins::normalize_code(&code)), expires)?;
                done(json!({ "code": code, "message": format!("reset code for {user}: {code}. It works once, for 24 hours, in the launcher's \"Forgot password\".") }))
            }
            Op::AddUser { user, hash } => {
                if !crate::logins::valid_user_id(user) { return Err(format!("{user:?} is not a User ID: 3 to 16 letters, digits, _ or -")) }
                if !hash.starts_with("$argon2id$") { return Err("the password hash is not argon2id".into()) }
                let recovery = crate::logins::new_code();
                let login = crate::logins::Login { user: user.clone(), password: hash.clone(), role: Default::default(), standing: Default::default(),
                    recovery: crate::logins::digest(&crate::logins::normalize_code(&recovery)), reset: None, devices: vec![] };
                d.register(login, None, crate::logins::Registration::Open).map_err(|e| if e == "user_id_taken" { format!("the User ID {user} is taken") } else { e })?;
                done(json!({ "recovery_code": recovery, "message": format!("recovery code: {recovery}") }))
            }
            other => Err(format!("{} is not an accounts operation", other.name())),
        }
    }
}

/// The API's answer to `request` (a whole HTTP request) from `peer`, if it is one of its routes.
pub fn handle(request: &str, peer: IpAddr, token: Option<&str>, api: &Api) -> Option<Response> {
    let (head, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
    let path = head.split_whitespace().nth(1).unwrap_or_default();
    let name = path.strip_prefix(PREFIX)?;
    let reply = |status, value: Value| Response { status, content_type: "application/json", headers: vec![], body: value.to_string().into_bytes() };
    let given = head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case("authorization").then(|| value.trim().strip_prefix("Bearer ")).flatten()
    });
    // The token, always: being on this machine is not enough (#25).
    if !peer.is_loopback() || !matches!((token, given), (Some(a), Some(b)) if same(a, b)) {
        tracing::warn!("[api] {peer}: {name} refused (not from this PC, or a wrong token)");
        return Some(reply("403 Forbidden", json!({ "ok": false, "error": "administrator authentication required" })));
    }
    if !head.starts_with("POST ") { return Some(reply("405 Method Not Allowed", json!({ "ok": false, "error": "use POST" }))) }
    let op: Op = match serde_json::from_str(body) {
        Ok(op) => op,
        Err(e) => return Some(reply("400 Bad Request", json!({ "ok": false, "error": format!("invalid request: {e}") }))),
    };
    if op.name() != name { return Some(reply("400 Bad Request", json!({ "ok": false, "error": "the path and the operation differ" }))) }
    // Only the operation's name is logged: answers can hold keys and codes.
    match api.apply(&op, "host") {
        Ok(result) => {
            if op.changes() { tracing::info!("[api] {name} done") }
            Some(reply("200 OK", json!({ "ok": true, "result": result })))
        }
        Err(error) => {
            tracing::warn!("[api] {name} refused: {error}");
            Some(reply("400 Bad Request", json!({ "ok": false, "error": error })))
        }
    }
}

/// Sends `op` to the running server `live`: its result, or why not.
pub fn call(live: &Live, op: &Op) -> Result<Value, String> {
    let send = || -> anyhow::Result<Value> {
        let mut stream = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let body = serde_json::to_vec(op)?;
        write!(stream, "POST {PREFIX}{} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            op.name(), live.token, body.len())?;
        stream.write_all(&body)?;
        let mut response = String::new();
        stream.take(16 * 1024 * 1024).read_to_string(&mut response)?;
        let (_, body) = response.split_once("\r\n\r\n").ok_or_else(|| anyhow::anyhow!("the server's answer is incomplete"))?;
        Ok(serde_json::from_str(body)?)
    };
    let answer = send().map_err(|e| format!("the server didn't answer: {e}"))?;
    if answer["ok"] == true { Ok(answer["result"].clone()) } else { Err(answer["error"].as_str().unwrap_or("refused").to_string()) }
}
