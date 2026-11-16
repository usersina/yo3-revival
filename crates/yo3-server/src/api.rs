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
    /// `count` copies (1 when none) of one artwork of a card (#19), by its English name or Konami id; `art` its first
    /// when none. The player's trunk has them at its next request (Edit Deck, the shop).
    GiveCards { player: String, card: String, #[serde(default)] art: Option<u32>, #[serde(default)] count: Option<u32> },
    /// A deck recipe for Load Deck's Official tab (#19), by its title or preset id; `cards` gives the recipe's cards too.
    /// The game shows it at the player's next login or after their next duel.
    GiveRecipe { player: String, deck: String, #[serde(default)] cards: bool },
    /// The deck recipes the host can give: preset id, title and how many cards.
    Recipes,
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
    /// Moderation (#26). WHO is a User ID or one of its players' names.
    /// Disconnects the account's game: it shows its own disconnect dialog.
    Kick { who: String },
    /// Sign-in refused (for `days`, or until lifted), every device signed out, the game disconnected.
    Ban { who: String, #[serde(default)] reason: String, #[serde(default)] days: Option<u32> },
    /// Sign-in refused (for `days`, or until lifted); a game already signed in plays on until it logs in again.
    Restrict { who: String, #[serde(default)] reason: String, #[serde(default)] days: Option<u32> },
    /// Lifts a ban or a restriction.
    Lift { who: String },
    /// The account's chat lines go nowhere (for `minutes`, or until lifted).
    Mute { who: String, #[serde(default)] reason: String, #[serde(default)] minutes: Option<u32> },
    Unmute { who: String },
    /// Every device signed out and the game disconnected (a moderator's Sign out everywhere).
    SignOut { who: String },
    /// Maintenance in `minutes` (the game's countdown until then) with `message`; `off` calls it off.
    Maintenance { #[serde(default)] minutes: Option<u32>, #[serde(default)] message: Option<String>, #[serde(default)] off: bool },
    /// One account in full (#36): its User ID, role, standing, mute, devices, BP, and each player's collection and record.
    Account { who: String },
    /// The database backups, newest first, with their times and sizes.
    Backups,
    /// A backup now (besides the hourly ones).
    BackUp,
    /// The server's newest log lines (`problems`: only warnings and errors), newest first.
    Logs { #[serde(default)] limit: Option<u32>, #[serde(default)] problems: bool },
}

impl Op {
    /// Its name in the API's path and the audit log.
    pub fn name(&self) -> String {
        serde_json::to_value(self).ok().and_then(|v| v["op"].as_str().map(str::to_owned)).unwrap_or_default()
    }

    /// Whether it changes anything (and so writes an audit row).
    pub fn changes(&self) -> bool {
        !matches!(self, Op::Status | Op::Accounts | Op::Countries | Op::Titles | Op::Recipes | Op::Codes | Op::Invites | Op::Settings | Op::Audit { .. }
            | Op::Account { .. } | Op::Backups | Op::Logs { .. })
    }

    /// Whether a moderator may run it (the admin panel, #36): looking, moderation, invites and reset codes. Settings,
    /// maintenance, BP, codes, keys, roles, backups and stopping the server are an admin's.
    pub fn for_moderators(&self) -> bool {
        matches!(self, Op::Status | Op::Accounts | Op::Account { .. } | Op::Countries | Op::Titles | Op::Codes | Op::Invites
            | Op::Settings | Op::Audit { .. } | Op::Logs { .. } | Op::Backups | Op::Invite { .. } | Op::ResetCode { .. }
            | Op::Rename { .. } | Op::Kick { .. } | Op::Ban { .. } | Op::Restrict { .. } | Op::Lift { .. } | Op::Mute { .. }
            | Op::Unmute { .. } | Op::SignOut { .. })
    }

    /// The account it acts on, if it acts on one (a User ID or a player's name).
    fn account(&self) -> Option<&str> {
        match self {
            Op::ResetCode { user } | Op::SetRole { user, .. } => Some(user),
            Op::Rename { old, .. } => Some(old),
            Op::Kick { who } | Op::Ban { who, .. } | Op::Restrict { who, .. } | Op::Lift { who } | Op::Mute { who, .. }
            | Op::Unmute { who } | Op::SignOut { who } | Op::GiveBp { who, .. } | Op::SetCountry { who, .. } | Op::Key { who }
            | Op::AddPlayer { who, .. } => Some(who),
            Op::GrantTitle { player, .. } | Op::GiveCards { player, .. } | Op::GiveRecipe { player, .. } => Some(player),
            _ => None,
        }
    }

    /// What it acts on, for the audit log.
    fn target(&self) -> String {
        match self {
            Op::AddPlayer { who, .. } | Op::Key { who } | Op::SetCountry { who, .. } | Op::GiveBp { who, .. } => who.clone(),
            Op::Rename { old, .. } => old.clone(),
            Op::GrantTitle { player, .. } | Op::GiveCards { player, .. } | Op::GiveRecipe { player, .. } => player.clone(),
            Op::AddUser { user, .. } | Op::ResetCode { user } | Op::SetRole { user, .. } => user.clone(),
            Op::Kick { who } | Op::Ban { who, .. } | Op::Restrict { who, .. } | Op::Lift { who } | Op::Mute { who, .. }
            | Op::Unmute { who } | Op::SignOut { who } => who.clone(),
            Op::Set { name, .. } | Op::Reset { name } => name.clone(),
            Op::CreateAccount { names } => names.join(", "),
            Op::AddCode { code, .. } | Op::RemoveCode { code } => code.clone(),
            Op::Account { who } => who.clone(),
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
pub struct Status {
    pub engine_problem: Option<String>,
    pub listen: Option<SocketAddr>,
    pub diagnostic_id: Option<String>,
    /// When the server started, for its uptime.
    pub started: Option<std::time::Instant>,
}

/// The server process's resident memory in bytes, where the system says (Linux).
pub fn memory_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kb: u64 = status.lines().find_map(|l| l.strip_prefix("VmRSS:"))?.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb * 1024)
}

/// The newest backup's folder name and its age in seconds.
pub fn last_backup(dir: &std::path::Path) -> Option<(String, u64)> {
    let newest = crate::store::backups(dir).into_iter().next()?;
    let age = std::fs::metadata(newest.join("server.db")).and_then(|m| m.modified()).ok()?.elapsed().map_or(0, |d| d.as_secs());
    Some((newest.file_name()?.to_string_lossy().into_owned(), age))
}

/// What the operations act on: a running server's state, or a stopped server's data folder.
pub struct Api {
    pub accounts: Arc<Mutex<Directory>>,
    pub settings: Arc<crate::settings::Live>,
    /// The online players (none when the server isn't running).
    pub hub: Option<Arc<Mutex<Hub>>>,
    /// The signed-in sessions, which a kick, a ban or maintenance ends (none when the server isn't running).
    pub sessions: Option<Arc<Mutex<crate::auth::Sessions>>>,
    /// Stops the running server.
    pub shutdown: Option<tokio::sync::watch::Sender<bool>>,
    pub status: Status,
    /// Where the audit log is kept (none: in memory, for tests).
    pub data_dir: Option<PathBuf>,
    /// The game, for the country and title tables.
    pub game_dir: PathBuf,
    pub titles: Option<Arc<crate::titles::TitleData>>,
    /// The running server's world, for the collection a gift of cards pushes (none: no push).
    pub world: Option<Arc<crate::front::World>>,
    /// The cards and recipes the host can give, read from the game at the first gift.
    gifts: std::sync::OnceLock<Arc<crate::gifts::Gifts>>,
    /// "server" for a running server's operations, "offline" for the CLI's on a stopped server's folder.
    pub origin: &'static str,
    memory: Mutex<Vec<AuditRow>>,
}

type Done = Result<(Value, Option<Value>, Option<Value>), String>;

impl Api {
    pub fn new(accounts: Arc<Mutex<Directory>>, settings: Arc<crate::settings::Live>, data_dir: Option<PathBuf>, game_dir: PathBuf, origin: &'static str) -> Api {
        Api { accounts, settings, hub: None, sessions: None, shutdown: None, status: Status::default(), data_dir, game_dir, titles: None, world: None, gifts: Default::default(), origin, memory: Mutex::default() }
    }

    /// Runs `op` for `actor` and writes its audit row: its result, or why it was refused.
    pub fn apply(&self, op: &Op, actor: &str) -> Result<Value, String> { self.apply_as(op, actor, self.origin, None) }

    /// Runs `op` for a signed-in staff account of `role` (the admin panel, #36), audited from `origin`. A moderator
    /// runs only [`Op::for_moderators`], and not on a staff account (no moderator resets an admin's password). With no
    /// role, the host's token holder, everything.
    pub fn apply_as(&self, op: &Op, actor: &str, origin: &str, role: Option<crate::logins::Role>) -> Result<Value, String> {
        use crate::logins::Role;
        match role {
            None | Some(Role::Admin) => {}
            Some(Role::Player) => return Err("only admins and moderators may do that".into()),
            Some(Role::Moderator) => {
                if !op.for_moderators() { return Err(format!("only an admin may do that ({})", op.name())) }
                let d = self.accounts.locked();
                let staff = op.account().and_then(|who| d.user_of(who)).and_then(|u| d.login_by_id(u)).is_some_and(|l| l.staff());
                let own = op.account().is_some_and(|who| who.eq_ignore_ascii_case(actor));
                if staff && !own && !matches!(op, Op::Account { .. }) { return Err("a moderator can't act on an admin's or another moderator's account".into()) }
            }
        }
        let (result, before, after) = self.run(op)?;
        if op.changes() {
            let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: actor.into(),
                origin: origin.into(), op: op.name(), target: op.target(), before, after, detail: op.detail() };
            match &self.data_dir {
                Some(dir) => if let Err(e) = crate::store::audit(dir, &row) { tracing::error!("audit row not written ({}): {e}", row.op) },
                None => self.memory.locked().push(row),
            }
        }
        Ok(result)
    }

    /// Writes an audit row for something that isn't an operation (the panel's backup download).
    pub fn note(&self, actor: &str, origin: &str, op: &str, target: &str) {
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: actor.into(),
            origin: origin.into(), op: op.into(), target: target.into(), before: None, after: None, detail: None };
        match &self.data_dir {
            Some(dir) => if let Err(e) = crate::store::audit(dir, &row) { tracing::error!("audit row not written ({}): {e}", row.op) },
            None => self.memory.locked().push(row),
        }
    }

    /// The newest `limit` audit rows.
    pub fn audit(&self, limit: u32) -> Result<Vec<AuditRow>, String> {
        match &self.data_dir {
            Some(dir) => crate::store::audit_rows(dir, limit).map_err(|e| e.to_string()),
            None => Ok(self.memory.locked().iter().rev().take(limit as usize).cloned().collect()),
        }
    }

    /// Ends the sessions of `user_id`: their game shows its own disconnect dialog.
    fn end_sessions(&self, user_id: u32) {
        if let Some(sessions) = &self.sessions { sessions.locked().logout_user(user_id) }
    }

    /// What a changed setting does at once on a running server: the news and the maintenance countdown are pushed to
    /// everyone online (the game takes 0x21FC whenever it arrives: R3, #26).
    fn after_setting(&self, name: &str) {
        let pushed = ["news", "maintenance-at", "maintenance-message"];
        let (Some(hub), true) = (&self.hub, pushed.iter().any(|n| n.eq_ignore_ascii_case(name.trim()))) else { return };
        let info = crate::front::information(&self.settings.now(), self.status.engine_problem.is_some(), crate::logins::now());
        hub.locked().push_everyone(&info);
    }

    /// Uses `gifts` instead of the game's cards and recipes (tests).
    pub fn with_gifts(self, gifts: crate::gifts::Gifts) -> Api {
        let _ = self.gifts.set(Arc::new(gifts));
        self
    }

    fn gifts(&self) -> Result<Arc<crate::gifts::Gifts>, String> {
        if let Some(g) = self.gifts.get() { return Ok(g.clone()) }
        let data = yo3_data::GameData::load(&self.game_dir).map_err(|e| format!("the game's cards: {e}"))?;
        Ok(self.gifts.get_or_init(|| Arc::new(crate::gifts::Gifts::from_game(&data))).clone())
    }

    /// Tells a player of a gift on a running server (`Hub::host_gift`): a D-Mail, and their new collection when the gift
    /// holds cards.
    fn tell_gift(&self, player: Option<crate::front::Player>, title: &str, body: String, cards: bool) {
        let (Some(hub), Some(player)) = (&self.hub, player) else { return };
        let trunk = self.world.as_ref().filter(|_| cards).map(|w| crate::trade::trunk(w, &player));
        hub.locked().host_gift((player.player_id, &player.name), title, body, trunk.as_ref());
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
                let backup = self.data_dir.as_deref().and_then(last_backup);
                Ok((json!({ "running": self.hub.is_some(), "ready": self.status.engine_problem.is_none(), "engine_problem": self.status.engine_problem,
                    "listen": self.status.listen.map(|l| l.to_string()), "release": yo3_tunnel::BUILD_VERSION, "diagnostic_id": self.status.diagnostic_id,
                    "players": players, "online": online, "duels": duels, "engines": crate::engine::running(),
                    "uptime_seconds": self.status.started.map(|t| t.elapsed().as_secs()), "memory_bytes": memory_bytes(),
                    "saving": crate::store::healthy(), "last_backup": backup.as_ref().map(|b| &b.0), "backup_age_seconds": backup.map(|b| b.1),
                    "recent_problems": crate::recent::lines(10, true) }), None, None))
            }
            Op::Backups => {
                let dir = self.data_dir.as_ref().ok_or("this server keeps no data folder")?;
                let rows: Vec<Value> = crate::store::backups(dir).iter().map(|b| {
                    let meta = std::fs::metadata(b.join("server.db")).ok();
                    let at = meta.as_ref().and_then(|m| m.modified().ok()).map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                    json!({ "name": b.file_name().unwrap_or_default().to_string_lossy(), "at": at, "bytes": meta.map(|m| m.len()) })
                }).collect();
                Ok((json!({ "backups": rows }), None, None))
            }
            Op::BackUp => {
                let dir = self.data_dir.as_ref().ok_or("this server keeps no data folder")?;
                crate::store::back_up(dir).map_err(|e| format!("the backup failed: {e}"))?;
                let name = last_backup(dir).map(|b| b.0).unwrap_or_default();
                Ok((json!({ "name": name, "message": format!("backup {name} taken") }), None, Some(json!(name))))
            }
            Op::Logs { limit, problems } => {
                Ok((json!({ "lines": crate::recent::lines(limit.unwrap_or(200).min(crate::recent::KEEP as u32) as usize, *problems) }), None, None))
            }
            Op::Account { who } => {
                let d = self.accounts.locked();
                let user_id = d.user_of(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
                let account = d.account_of(user_id).ok_or("the account is gone")?;
                let login = d.login_by_id(user_id).cloned();
                let has_key = d.entries().any(|(_, key, a)| a.user_id == user_id && key.is_some());
                drop(d);
                let hub = self.hub.as_ref().map(|h| h.locked());
                let now = crate::logins::now();
                let players: Vec<Value> = account.players.iter().map(|p| {
                    let record = hub.as_ref().map(|h| h.record_totals(p.player_id));
                    json!({ "player_id": p.player_id, "name": p.name, "slot": p.index, "level": p.level, "title": p.title,
                        "cards": p.cards.values().sum::<u32>(), "kinds": p.cards.len(), "decks": p.decks.len(), "uploaded_decks": p.uploaded.len(),
                        "items": p.items.len(), "record": record, "online": hub.as_ref().map(|h| h.is_online(p.player_id)) })
                }).collect();
                Ok((json!({ "user_id": user_id, "login": login.as_ref().map(|l| &l.user), "has_key": has_key, "bp": account.dpoint,
                    "country": account.country, "role": login.as_ref().map(|l| l.role),
                    "standing": login.as_ref().map(|l| if l.refusal(now).is_some() { json!(l.standing) } else { json!({ "kind": "active" }) }),
                    "muted": login.as_ref().and_then(|l| l.mute(now)), "devices": login.as_ref().map(|l| l.devices.len()),
                    "reset_pending": login.as_ref().and_then(|l| l.reset.as_ref()).is_some_and(|(_, until)| *until > now),
                    "players": players }), None, None))
            }
            Op::Shutdown => {
                let stop = self.shutdown.as_ref().ok_or("the server isn't running")?;
                stop.send_replace(true);
                Ok((json!({ "stopping": true, "message": "the server is stopping" }), None, None))
            }
            Op::Settings => Ok((json!({ "message": self.settings.list(), "settings": self.settings.rows() }), None, None)),
            Op::Set { name, value } => {
                let before = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                let done = self.settings.set(name, value)?;
                let after = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                self.after_setting(name);
                Ok((message(done), before, after))
            }
            Op::Reset { name } => {
                let before = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                let done = self.settings.reset(name)?;
                let after = crate::settings::find(name).map(|s| self.settings.now().value(s.name).clone());
                self.after_setting(name);
                Ok((message(done), before, after))
            }
            Op::Maintenance { minutes, message: text, off } => {
                let before = self.settings.now().maintenance_at;
                if *off {
                    self.settings.reset("maintenance-at")?;
                    self.after_setting("maintenance-at");
                    // A countdown the game was showing: its wrench stays up (R3), so say why.
                    if let (Some(hub), Some(at)) = (&self.hub, before) && at > crate::logins::now() {
                        hub.locked().push_everyone(&crate::front::chat_note("Moderator", crate::front::MAINTENANCE_CANCELLED));
                    }
                    return Ok((message("maintenance is off".into()), before.map(|t| json!(t)), None));
                }
                let minutes = minutes.ok_or("maintenance needs --in MINUTES (0: now), or --off")?;
                if let Some(text) = text { self.settings.set("maintenance-message", text)?; }
                let at = crate::logins::now() + i64::from(minutes) * 60;
                self.settings.set("maintenance-at", &at.to_string())?;
                self.after_setting("maintenance-at");
                let when = chrono::DateTime::from_timestamp(at, 0).map_or(String::new(), |t| t.format("%Y-%m-%d %H:%M UTC").to_string());
                Ok((message(format!("maintenance starts at {when}: the game counts down until then; then only admins and moderators stay")),
                    before.map(|t| json!(t)), Some(json!(at))))
            }
            Op::Kick { who } => {
                let user_id = self.accounts.locked().user_of(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
                self.end_sessions(user_id);
                Ok((message(format!("{who} was disconnected")), None, None))
            }
            Op::Ban { who, reason, days } | Op::Restrict { who, reason, days } => {
                let ban = matches!(op, Op::Ban { .. });
                let until = days.map(|d| crate::logins::now() + i64::from(d) * 86_400);
                let standing = if ban { crate::logins::Standing::Banned { reason: reason.clone(), until } }
                    else { crate::logins::Standing::Restricted { reason: reason.clone(), until } };
                let mut d = self.accounts.locked();
                let (user_id, login) = d.login_named(who)?;
                d.set_standing_of(user_id, standing.clone())?;
                if ban { d.revoke_devices(user_id)?; }
                drop(d);
                // A ban takes effect at once; a restriction at the account's next sign-in.
                if ban { self.end_sessions(user_id) }
                let until = until.and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("until lifted".to_string(), |t| format!("until {}", t.format("%Y-%m-%d %H:%M UTC")));
                let what = if ban { "banned (devices signed out, game disconnected)" } else { "restricted (refused at the next sign-in)" };
                Ok((message(format!("{} is {what} {until}", login.user)), Some(json!(login.standing)), Some(json!(standing))))
            }
            Op::Lift { who } => {
                let mut d = self.accounts.locked();
                let (user_id, login) = d.login_named(who)?;
                d.set_standing_of(user_id, crate::logins::Standing::Active)?;
                Ok((message(format!("{} may sign in again", login.user)), Some(json!(login.standing)), Some(json!(crate::logins::Standing::Active))))
            }
            Op::Mute { who, reason, minutes } => {
                let until = minutes.map(|m| crate::logins::now() + i64::from(m) * 60);
                let mut d = self.accounts.locked();
                let (user_id, login) = d.login_named(who)?;
                let mute = crate::logins::Mute { reason: reason.clone(), until };
                d.set_mute(user_id, Some(mute.clone()))?;
                let until = until.and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map_or("until unmuted".to_string(), |t| format!("until {}", t.format("%Y-%m-%d %H:%M UTC")));
                Ok((message(format!("{} is muted {until}", login.user)), login.muted.map(|m| json!(m)), Some(json!(mute))))
            }
            Op::Unmute { who } => {
                let mut d = self.accounts.locked();
                let (user_id, login) = d.login_named(who)?;
                d.set_mute(user_id, None)?;
                Ok((message(format!("{} may chat again", login.user)), login.muted.map(|m| json!(m)), None))
            }
            Op::SignOut { who } => {
                let mut d = self.accounts.locked();
                let (user_id, login) = d.login_named(who)?;
                d.revoke_devices(user_id)?;
                drop(d);
                self.end_sessions(user_id);
                Ok((message(format!("{} is signed out everywhere", login.user)), Some(json!(login.devices.len())), Some(json!(0))))
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
            Op::GiveCards { player, card, art, count } => {
                let count = count.unwrap_or(1);
                if !(1..=255).contains(&count) { return Err("give 1 to 255 copies (the trunk counts at most 255 of a card)".into()) }
                let gifts = self.gifts()?;
                let card = gifts.card(card, *art)?;
                let mut d = self.accounts.locked();
                let player_id = d.player_id_named(player).ok_or_else(|| format!("no player named {player:?}"))?;
                let before = d.player(player_id).and_then(|p| p.cards.get(&card.row).copied()).unwrap_or(0);
                d.update_player(player_id, |p| { *p.cards.entry(card.row).or_default() += count; })?;
                let stored = d.player(player_id).cloned();
                drop(d);
                let after = before + count;
                let art = if gifts.cards.iter().filter(|c| c.cid == card.cid).count() > 1 { format!(" (artwork {})", card.art) } else { String::new() };
                self.tell_gift(stored, "Cards received", format!("The host gave you {count} × {}{art}. They are in your cards now.", card.name), true);
                Ok((json!({ "row": card.row, "count": after, "message": format!("gave {count} × {} (artwork {}, Konami id {}) to {player}: {after} beyond \
their starting cards now (a running server mails them, and an online game has them at once)", card.name, card.art, card.cid) }),
                    Some(json!({ "card": card.name, "art": card.art, "count": before })), Some(json!({ "card": card.name, "art": card.art, "count": after }))))
            }
            Op::GiveRecipe { player, deck, cards } => {
                let gifts = self.gifts()?;
                let recipe = gifts.recipe(deck)?;
                let mut d = self.accounts.locked();
                let player_id = d.player_id_named(player).ok_or_else(|| format!("no player named {player:?}"))?;
                let had = d.player(player_id).is_some_and(|p| p.items.contains(&recipe.item));
                if had && !cards { return Err(format!("{player} already has the recipe {}", recipe.title)) }
                d.update_player(player_id, |p| {
                    p.items.insert(recipe.item);
                    if *cards { for &row in &recipe.cards { *p.cards.entry(row).or_default() += 1; } }
                })?;
                let stored = d.player(player_id).cloned();
                drop(d);
                let and = if *cards { format!(" and its {} cards", recipe.cards.len()) } else { String::new() };
                self.tell_gift(stored, "Recipe received", format!("The host gave you the deck recipe {}{and}. It is on Load Deck's \
Official tab from your next login, or after your next duel.", recipe.title), *cards);
                let with = if *cards { format!(" and its {} cards", recipe.cards.len()) } else { String::new() };
                plain(Ok(format!("gave the recipe {} (deck {}){with} to {player} (on Load Deck's Official tab at their next login or after their next duel)",
                    recipe.title, recipe.id)))
            }
            Op::Recipes => {
                let gifts = self.gifts()?;
                let rows: Vec<Value> = gifts.recipes.iter().map(|r| json!({ "id": r.id, "title": r.title, "item": r.item, "cards": r.cards.len() })).collect();
                let lines: Vec<String> = gifts.recipes.iter().map(|r| format!("{}\t{}\t{} cards", r.id, r.title, r.cards.len())).collect();
                Ok((json!({ "recipes": rows, "message": lines.join("\n") }), None, None))
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
                let held = login.and_then(|u| d.login_of(u)).map(|(_, l)| l);
                let now = crate::logins::now();
                let standing = held.as_ref().map(|l| match l.refusal(now) { Some(r) => r["error"].clone(), None => json!("active") });
                let muted = held.as_ref().is_some_and(|l| l.mute(now).is_some());
                json!({ "user_id": account.user_id, "login": login, "has_key": key.is_some(), "role": role, "bp": account.dpoint,
                    "country": account.country, "standing": standing, "muted": muted,
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
                let login = crate::logins::Login::new(user, hash.clone(), crate::logins::digest(&crate::logins::normalize_code(&recovery)));
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
    // Who acts, for the audit log: a moderator's name the tool passes on (`--by`), else the host.
    let actor = head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case("x-yo3-actor").then(|| value.trim().chars().filter(|c| !c.is_control()).take(40).collect::<String>())
    }).filter(|a| !a.is_empty()).unwrap_or_else(|| "host".into());
    // Only the operation's name is logged: answers can hold keys and codes.
    match api.apply(&op, &actor) {
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

/// The answer to `GET /health` on the loopback HTTP port (#37), for the host's monitor: 200 when the server can save
/// and duel, 503 when it can't. No token: it says nothing about players beyond how many are online.
pub fn health(api: &Api) -> Response {
    let now = crate::logins::now();
    let (online, duels) = api.hub.as_ref().map_or((0, 0), |h| { let h = h.locked(); (h.online(), h.tables.len()) });
    let saving = crate::store::healthy();
    let engine = api.status.engine_problem.is_none();
    let backup = api.data_dir.as_deref().and_then(last_backup);
    let body = json!({ "ok": saving && engine, "saving": saving, "engine": engine, "backup_age_seconds": backup.map(|b| b.1),
        "errors_recent": crate::recent::errors_within(600), "online": online, "duels": duels,
        "maintenance": api.settings.now().in_maintenance(now), "release": yo3_tunnel::BUILD_VERSION });
    let status = if saving && engine { "200 OK" } else { "503 Service Unavailable" };
    Response { status, content_type: "application/json", headers: vec![("Cache-Control", "no-store".into())], body: body.to_string().into_bytes() }
}

/// Sends `op` to the running server `live`: its result, or why not.
pub fn call(live: &Live, op: &Op) -> Result<Value, String> { call_as(live, op, "host") }

/// The same, recorded in the audit log as `actor`'s (a moderator's name).
pub fn call_as(live: &Live, op: &Op, actor: &str) -> Result<Value, String> {
    let actor: String = actor.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').take(40).collect();
    let send = || -> anyhow::Result<Value> {
        let mut stream = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let body = serde_json::to_vec(op)?;
        write!(stream, "POST {PREFIX}{} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nX-YO3-Actor: {actor}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
