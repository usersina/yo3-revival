//! The admin API (#25): every administrative operation, one versioned JSON endpoint, and the audit log.
//!
//! `POST /api/admin/v1/<op>` on the server's loopback HTTP port, with `Authorization: Bearer <token>` (the token in
//! `data/admin.json`, readable by the host's user only) and the operation as a JSON object ([`Op`], tagged by `op`).
//! The answer is `{"ok": true, "result": ...}` or `{"ok": false, "error": "..."}`. The CLI, the desktop app and later
//! the admin panel (#36) are its clients.
//!
//! Each operation is one function ([`Api::apply`]): it checks its values, applies the change to the running server's
//! state (in memory and in the database together), then writes an audit row: who, from where, what, on what, the
//! values before and after. Give BP commits the change, mail, audit and retry receipt together before publishing.
//! Invite/code, profile/role, credential, gift and live-setting changes also commit audit and receipt with their change. Other operations are
//! being adapted. With no server running, the CLI runs on the data folder (it
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

pub mod give_bp;
pub mod card_images;
pub mod mutation;
pub mod codes;
pub mod invites;
pub mod profiles;
pub mod credentials;
pub mod keys;
pub mod gifts;
pub mod settings;
pub mod moderation;
pub mod deletion;
pub mod server;
pub mod observability;
pub mod backups;
pub use give_bp::{GiveBpRequest, GiveBpResult};
pub use mutation::{Actor, MutationError, MutationErrorCode};

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
    /// Provision a password account with a caller-generated one-time setup code.
    AddUser { user: String, code: credentials::AccessCode },
    ResetCode { user: String, code: credentials::AccessCode },
    SetRole { user: String, role: String },
    Settings,
    Set { name: String, value: String },
    Reset { name: String },
    /// The newest audit rows (50 when no limit is given).
    Audit { #[serde(default)] limit: Option<u32> },
    Shutdown { run_id: String },
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
    /// Deletes an account (security review #39): its players and everything kept for them, its sign-ins and devices;
    /// its game is disconnected. The audit log keeps only who deleted which User ID and when.
    DeleteAccount { who: String },
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
    /// Operations migrated to a committed change/audit/receipt boundary.
    fn receipted(&self) -> bool { matches!(self, Self::BackUp | Self::Shutdown { .. } | Self::DeleteAccount { .. } | Self::Kick { .. } | Self::Ban { .. } | Self::Restrict { .. } | Self::Lift { .. } | Self::Mute { .. } | Self::Unmute { .. } | Self::SignOut { .. } | Self::Set { .. } | Self::Reset { .. } | Self::Maintenance { .. } | Self::GiveCards { .. } | Self::GiveRecipe { .. } | Self::GrantTitle { .. } | Self::CreateAccount { .. } | Self::Key { .. } | Self::GiveBp { .. } | Self::AddCode { .. } | Self::RemoveCode { .. } | Self::Invite { .. } | Self::AddPlayer { .. } | Self::Rename { .. } | Self::SetCountry { .. } | Self::SetRole { .. } | Self::AddUser { .. } | Self::ResetCode { .. }) }
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
            Op::ResetCode { user, .. } | Op::SetRole { user, .. } => Some(user),
            Op::Rename { old, .. } => Some(old),
            Op::Kick { who } | Op::Ban { who, .. } | Op::Restrict { who, .. } | Op::Lift { who } | Op::Mute { who, .. }
            | Op::Unmute { who } | Op::SignOut { who } | Op::GiveBp { who, .. } | Op::SetCountry { who, .. } | Op::Key { who }
            | Op::AddPlayer { who, .. } | Op::DeleteAccount { who } => Some(who),
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
            Op::AddUser { user, .. } | Op::ResetCode { user, .. } | Op::SetRole { user, .. } => user.clone(),
            Op::Kick { who } | Op::Ban { who, .. } | Op::Restrict { who, .. } | Op::Lift { who } | Op::Mute { who, .. }
            | Op::Unmute { who } | Op::SignOut { who } => who.clone(),
            Op::Set { name, .. } | Op::Reset { name } => name.clone(),
            Op::CreateAccount { names } => names.join(", "),
            Op::AddCode { code, .. } | Op::RemoveCode { code } => code.clone(),
            Op::Account { who } | Op::DeleteAccount { who } => who.clone(),
            _ => String::new(),
        }
    }

    /// Its values for the audit log, without secrets (a password's hash).
    fn detail(&self) -> Option<Value> {
        let mut v = serde_json::to_value(self).ok()?;
        let fields = v.as_object_mut()?;
        fields.remove("op");
        fields.remove("hash");
        if matches!(self, Self::AddUser { .. } | Self::ResetCode { .. }) { fields.remove("code"); }
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
    card_images: std::sync::OnceLock<card_images::CardImages>,
    gifts: std::sync::OnceLock<Arc<crate::gifts::Gifts>>,
    /// "server" for a running server's operations, "offline" for the CLI's on a stopped server's folder.
    pub origin: &'static str,
    run_id: String,
    memory: Mutex<Vec<AuditRow>>,
    effects: Mutex<std::collections::HashMap<(String,String), crate::store::EffectState>>,
    receipts: Mutex<std::collections::HashMap<(String, String), crate::store::Receipt>>,
}

type Done = Result<(Value, Option<Value>, Option<Value>), String>;

impl Api {
    pub fn new(accounts: Arc<Mutex<Directory>>, settings: Arc<crate::settings::Live>, data_dir: Option<PathBuf>, game_dir: PathBuf, origin: &'static str) -> Api {
        Api { accounts, settings, hub: None, sessions: None, shutdown: None, status: Status::default(), data_dir, game_dir, titles: None, world: None, gifts: Default::default(), card_images: Default::default(), origin, run_id: crate::logins::new_token(), memory: Mutex::default(), effects: Mutex::default(), receipts: Mutex::default() }
    }

    /// Runs `op` for `actor` and writes its audit row: its result, or why it was refused.
    pub fn apply(&self, op: &Op, actor: &str) -> Result<Value, String> { self.apply_as(op, actor, self.origin, None) }

    /// Runs `op` for a signed-in staff account of `role` (the admin panel, #36), audited from `origin`. A moderator
    /// runs only [`Op::for_moderators`], and not on a staff account (no moderator resets an admin's password). With no
    /// role, the host's token holder, everything.
    pub fn apply_as(&self, op: &Op, actor: &str, origin: &str, role: Option<crate::logins::Role>) -> Result<Value, String> {
        self.apply_request(op, actor, origin, role, &crate::logins::new_token())
    }

    /// The same operation with a caller's stable retry ID for migrated transactional mutations.
    pub fn apply_request(&self, op: &Op, actor: &str, origin: &str, role: Option<crate::logins::Role>, request_id: &str) -> Result<Value, String> {
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
        if op.receipted() {
            let caller = if role.is_none() { Actor::Host { by: actor } } else {
                let id = self.accounts.locked().login_of(actor).map(|(id, _)| id).ok_or("only an admin may do that")?;
                Actor::Staff { user_id: id }
            };
            return self.typed_change(op, request_id, caller, origin).map_err(|e| e.to_string())
        }
        let (result, before, after) = self.run(op)?;
        if op.changes() {
            let (target, detail) = (op.target(), op.detail());
            let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: actor.into(),
                origin: origin.into(), op: op.name(), target, before, after, detail };
            match &self.data_dir {
                Some(dir) => if let Err(e) = crate::store::audit(dir, &row) { tracing::error!("audit row not written ({}): {e}", row.op) },
                None => self.memory.locked().push(row),
            }
        }
        Ok(result)
    }

    fn typed_change(&self, op: &Op, id: &str, actor: Actor<'_>, origin: &str) -> Result<Value, MutationError> {
        match op {
            Op::BackUp => self.create_backup(&backups::CreateBackupRequest {},id,actor,origin).and_then(|r| {
                if r.backup==crate::store::EffectState::Failed {return Err(MutationError::new(MutationErrorCode::PersistenceFailed,"Backup failed. Inspect the server logs; retrying this ID only checks the saved result."))}
                let mut value=json!(r.result);value["backup"]=json!(r.backup);value["message"]=json!(if r.backup==crate::store::EffectState::Completed {"Backup completed."}else{"Backup outcome unknown. Inspect the backup list and server logs."});Ok(value)
            }),
            Op::Shutdown { run_id } => self.request_stop(&server::StopRequest { run_id: run_id.clone() }, id, actor, origin).map(|r| {
                let mut value=json!(r.result);value["stop"]=json!(r.stop);value["stopping"]=json!(true);value
            }),
            Op::DeleteAccount { .. } => self.named_deletion(op, id, actor, origin),
            Op::Kick { .. } | Op::Ban { .. } | Op::Restrict { .. } | Op::Lift { .. } | Op::Mute { .. } | Op::Unmute { .. } | Op::SignOut { .. } => self.named_moderation(op, id, actor, origin),
            Op::Set { .. } | Op::Reset { .. } | Op::Maintenance { .. } => self.named_setting(op, id, actor, origin),
            Op::GiveBp { who, amount } => self.give_bp(&GiveBpRequest { who: who.clone(), amount: *amount }, id, actor, origin).map(|r| json!(r)),
            Op::AddCode { code, bp, uses } => self.create_bp_code(&codes::CreateBpCodeRequest {
                code: (!code.trim().is_empty()).then(|| code.trim().to_string()), bp: *bp, uses: *uses,
            }, id, actor, origin).map(|r| json!(r)),
            Op::RemoveCode { code } => self.remove_bp_code(&codes::RemoveBpCodeRequest { code: code.trim().to_string() }, id, actor, origin).map(|r| json!(r)),
            Op::Invite { uses, days } => self.create_invite(&invites::CreateInviteRequest { uses: *uses, days: *days }, id, actor, origin).map(|r| json!(r)),
            Op::AddPlayer { .. } | Op::Rename { .. } | Op::SetCountry { .. } | Op::SetRole { .. } => self.edit_named_account(op, id, actor, origin),
            Op::AddUser { .. } | Op::ResetCode { .. } => self.issue_named_code(op, id, actor, origin),
            Op::CreateAccount { .. } | Op::Key { .. } => self.named_key_account(op, id, actor, origin),
            Op::GiveCards { .. } | Op::GiveRecipe { .. } | Op::GrantTitle { .. } => self.named_gift(op, id, actor, origin),
            _ => Err(MutationError::new(MutationErrorCode::InvalidRequest, "This operation has not migrated to the typed mutation boundary.")),
        }
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

    fn titles(&self) -> Result<Arc<crate::titles::TitleData>, String> {
        if let Some(t) = &self.titles { return Ok(t.clone()) }
        let data = yo3_data::GameData::load(&self.game_dir).map_err(|e| format!("the game's titles: {e}"))?;
        Ok(Arc::new(crate::titles::TitleData::from_game(&data)))
    }

    /// The operation itself: its result, and the values before and after the change.
    fn run(&self, op: &Op) -> Done {
        match op {
            Op::Status => {
                let (players, online, duels) = match &self.hub {
                    Some(hub) => { let hub = hub.locked(); (json!(hub.manager_players()), hub.online(), hub.tables.len()) }
                    None => (json!([]), 0, 0),
                };
                let backup = self.data_dir.as_deref().and_then(last_backup);
                Ok((json!({ "run_id": self.run_id, "can_stop": self.shutdown.is_some(), "running": self.hub.is_some(), "ready": self.status.engine_problem.is_none(), "engine_problem": self.status.engine_problem,
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
            Op::BackUp => Err("backups require the typed mutation boundary".into()),
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
            Op::DeleteAccount { .. } => Err("deletion requires the typed mutation boundary".into()),
            Op::Shutdown { .. } => Err("stop requires the typed mutation boundary".into()),
            Op::Settings => Ok((json!({ "message": self.settings.list(), "settings": self.settings.rows() }), None, None)),
            Op::Set { .. } | Op::Reset { .. } | Op::Maintenance { .. } => Err("settings edits require the typed mutation boundary".into()),
            Op::Kick { .. } | Op::Ban { .. } | Op::Restrict { .. } | Op::Lift { .. } | Op::Mute { .. } | Op::Unmute { .. } | Op::SignOut { .. } => Err("moderation requires the typed mutation boundary".into()),
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
            Op::GrantTitle { .. } | Op::GiveCards { .. } | Op::GiveRecipe { .. } => Err("gifts require the typed mutation boundary".into()),
            Op::Recipes => {
                let gifts = self.gifts()?;
                let rows: Vec<Value> = gifts.recipes.iter().map(|r| json!({ "id": r.id, "title": r.title, "item": r.item, "cards": r.cards.len() })).collect();
                let lines: Vec<String> = gifts.recipes.iter().map(|r| format!("{}\t{}\t{} cards", r.id, r.title, r.cards.len())).collect();
                Ok((json!({ "recipes": rows, "message": lines.join("\n") }), None, None))
            }
            Op::Rename { .. } | Op::SetCountry { .. } | Op::SetRole { .. } => Err("profile edits require the typed mutation boundary".into()),
            _ => self.accounts_op(op),
        }
    }

    /// The operations on the accounts alone.
    fn accounts_op(&self, op: &Op) -> Done {
        let d = self.accounts.locked();
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
            Op::CreateAccount { .. } | Op::Key { .. } => Err("key operations require the typed mutation boundary".into()),
            Op::AddPlayer { .. } => Err("player creation requires the typed mutation boundary".into()),
            Op::Codes => done(json!(Self::bp_codes(&d).codes)),
            Op::Invite { .. } => Err("invite creation requires the typed mutation boundary".into()),
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
            Op::ResetCode { .. } | Op::AddUser { .. } => Err("credential operations require the typed mutation boundary".into()),
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
    if op.receipted() {
        let ids: Vec<&str> = head.lines().filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("idempotency-key").then(|| value.trim())
        }).collect();
        if ids.len() != 1 {
            return Some(reply("400 Bad Request", json!({ "ok": false, "code": "invalid_request", "error": "This change requires one Idempotency-Key header." })))
        }
        return Some(match api.typed_change(&op, ids[0], Actor::Host { by: &actor }, api.origin) {
            Ok(result) => reply("200 OK", json!({ "ok": true, "result": result })),
            Err(error) => {
                let status = match error.code {
                    MutationErrorCode::RequestConflict | MutationErrorCode::AlreadyExists => "409 Conflict", MutationErrorCode::Forbidden => "403 Forbidden",
                    MutationErrorCode::NotFound => "404 Not Found", MutationErrorCode::PersistenceFailed => "503 Service Unavailable",
                    MutationErrorCode::InvalidRequest => "400 Bad Request",
                };
                reply(status, json!({ "ok": false, "code": error.code, "error": error.message }))
            }
        })
    }
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

/// Capture this process's identity before requesting a stop; a restarted endpoint must not be stopped accidentally.
pub fn stop(live: &Live) -> Result<Value, String> {
    let status=call(live,&Op::Status)?;
    let run_id=status["run_id"].as_str().ok_or("The server did not provide a run identity")?.to_string();
    call(live,&Op::Shutdown {run_id})
}

/// The same, recorded in the audit log as `actor`'s (a moderator's name).
pub fn call_as(live: &Live, op: &Op, actor: &str) -> Result<Value, String> {
    call_as_request(live, op, actor, &crate::logins::new_token())
}

/// Reuse `request_id` after an uncertain response: migrated mutations return their original committed result.
pub fn call_as_request(live: &Live, op: &Op, actor: &str, request_id: &str) -> Result<Value, String> {
    if !mutation::valid_request_id(request_id) { return Err("invalid request ID (16–128 letters, digits, hyphens or underscores)".into()) }
    let actor: String = actor.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').take(40).collect();
    let send = || -> anyhow::Result<Value> {
        let mut stream = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let body = serde_json::to_vec(op)?;
        write!(stream, "POST {PREFIX}{} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nX-YO3-Actor: {actor}\r\nIdempotency-Key: {request_id}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
