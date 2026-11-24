//! Live settings (#24): what a host changes while the server runs, without a restart.
//!
//! Each setting in [`REGISTRY`] has a name, a kind (which checks a new value), a default and the moment it applies. The
//! values a host set are kept in the database (`settings`); the others are their defaults. Readers take the current
//! [`Settings`] with [`Live::now`], an immutable snapshot, at their natural boundary: a login keeps its banlist until the
//! player logs in again, a duel its LP and clocks until it ends, and the account requests read theirs at each request.
//! A change makes a new snapshot and swaps it in whole.
//!
//! What can't change while running (listeners, the TLS identity, folders, the engine) is in the TOML file instead
//! (`config.rs`), and the testing aids stay command-line flags.
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use serde::Serialize;
use serde_json::{Map, Value as Json, json};

use crate::logins::{Limits, Registration};

/// When a change reaches the players.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Applies {
    /// At each player's next login: players already online keep the value they logged in with.
    NextLogin,
    /// At the next duel: duels already running keep theirs.
    NextDuel,
    /// At the game's next request for it.
    NextRequest,
    /// At once, for every request from then on.
    AtOnce,
}

impl Applies {
    pub fn describe(self) -> &'static str {
        match self {
            Applies::NextLogin => "at each player's next login",
            Applies::NextDuel => "at the next duel",
            Applies::NextRequest => "at the game's next request",
            Applies::AtOnce => "at once",
        }
    }
}

/// What a setting holds, which says how a typed value is read and checked.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Text { max: usize },
    Number { min: u32, max: u32 },
    Choice(&'static [&'static str]),
    /// A launcher version such as 0.2.0, or empty for any.
    Version,
    /// A banlist of the client's: an id or a name as the game shows it, or empty for the default (Season 16).
    Regulation,
    /// The news on the game's Information list: entries of a title and a body.
    News,
}

pub struct Setting {
    pub name: &'static str,
    pub help: &'static str,
    pub kind: Kind,
    pub applies: Applies,
    pub default: fn() -> Json,
}

/// The news a new server shows.
fn default_news() -> Json {
    json!([{ "title": "Welcome back to Yu-Gi-Oh! ONLINE", "body": "This is a private revival server. Have fun duelling with your friends!" }])
}

/// Every live setting.
pub const REGISTRY: &[Setting] = &[
    Setting { name: "server-name", help: "The server's name in the launcher's server list", kind: Kind::Text { max: 60 },
        applies: Applies::AtOnce, default: || json!("Yu-Gi-Oh! ONLINE 3 server") },
    Setting { name: "headline", help: "A line of news under the name in the launcher's server list", kind: Kind::Text { max: 200 },
        applies: Applies::AtOnce, default: || json!("") },
    Setting { name: "registration", help: "How accounts are made: invite (with an invite code), open (anyone) or closed (only the host's add-user)",
        kind: Kind::Choice(&["invite", "open", "closed"]), applies: Applies::AtOnce, default: || json!("invite") },
    Setting { name: "min-launcher", help: "Require at least this launcher version (empty: the server's release; cannot lower that floor)", kind: Kind::Version,
        applies: Applies::AtOnce, default: || json!("") },
    Setting { name: "login-regulation", help: "The banlist players log in with, for Auto Duel and NPC duels: an id or a name (empty: Season 16)",
        kind: Kind::Regulation, applies: Applies::NextLogin, default: || json!("") },
    Setting { name: "start-lp", help: "Each player's LP at the start of an Auto Duel or NPC duel (a hosted duel has its own)",
        kind: Kind::Number { min: 100, max: 99_999 }, applies: Applies::NextDuel, default: || json!(8000) },
    Setting { name: "turn-seconds", help: "A turn's clock in duels between players, when the duel sets none (0: off)",
        kind: Kind::Number { min: 0, max: 3600 }, applies: Applies::NextDuel, default: || json!(crate::clock::DEFAULT_TURN) },
    Setting { name: "total-seconds", help: "Each player's total clock in duels between players, when the duel sets none (0: off)",
        kind: Kind::Number { min: 0, max: 36_000 }, applies: Applies::NextDuel, default: || json!(crate::clock::DEFAULT_TOTAL) },
    Setting { name: "anime-cards", help: "Duels play the cards with a reviewed anime version by their anime effects (off: the game's own; the cards \
        the client could not show right keep theirs: docs/playing.md). A game that started before a change cannot duel \
        until it is restarted (Play again)", kind: Kind::Choice(&["off", "on"]), applies: Applies::NextDuel,
        default: || json!("off") },
    Setting { name: "destiny-draw", help: "Casual duels against the computer (NPC duels): once per duel, at the start of your Draw Phase with \
        1000 LP or less, you may pick any card left in your Deck as the card you draw (off: no)",
        kind: Kind::Choice(&["off", "on"]), applies: Applies::NextDuel, default: || json!("off") },
    Setting { name: "pack-cost", help: "BP a booster pack costs in Blister's shop (the game shows it when you pick the pack)",
        kind: Kind::Number { min: 1, max: 100_000 }, applies: Applies::NextRequest, default: || json!(crate::shop::PACK_COST) },
    Setting { name: "deck-cost", help: "BP a structure deck costs in Blister's shop",
        kind: Kind::Number { min: 1, max: 100_000 }, applies: Applies::NextRequest, default: || json!(crate::shop::DECK_COST) },
    Setting { name: "trade-cards", help: "Cards of their own a player gives for one card in Blister's Trade Card",
        kind: Kind::Number { min: 1, max: 40 }, applies: Applies::NextRequest, default: || json!(crate::shop::EXCHANGE_COST) },
    Setting { name: "rental-cards", help: "Cards the shop's Rental offers at once (\"You can borrow N cards for BP for D days\")",
        kind: Kind::Number { min: 1, max: 40 }, applies: Applies::NextRequest, default: || json!(crate::shop::RENTAL_CARDS) },
    Setting { name: "rental-bp", help: "BP the shop's Rental costs",
        kind: Kind::Number { min: 0, max: 100_000 }, applies: Applies::NextRequest, default: || json!(crate::shop::RENTAL_BP) },
    Setting { name: "rental-days", help: "Days the shop's Rental lasts",
        kind: Kind::Number { min: 1, max: 365 }, applies: Applies::NextRequest, default: || json!(crate::shop::RENTAL_DAYS) },
    Setting { name: "report-limit", help: "Player reports (`/report` in the chat) one player may send in an hour (0: reports are off)",
        kind: Kind::Number { min: 0, max: 1000 }, applies: Applies::NextRequest, default: || json!(5) },
    Setting { name: "games-per-device", help: "Games one PC may run at once on this server (the launcher's Play as another account; staff is exempt). \
        1 keeps one game per PC: raise it to let friends sharing a PC play together", kind: Kind::Number { min: 1, max: 16 },
        applies: Applies::AtOnce, default: || json!(1) },
    Setting { name: "linked-accounts", help: "Accounts seen on one PC or address: fair (a duel between them pays no BP and counts no record or title, \
        and they can't trade or rent to each other) or off (they are treated as strangers)",
        kind: Kind::Choice(&["fair", "off"]), applies: Applies::AtOnce, default: || json!("fair") },
    Setting { name: "clean-game", help: "on: the launcher refuses to start the game here while another program's .dll or .asi is in the game folder, \
        a Winsock provider isn't Microsoft's or a network function is detoured (a tidiness policy, not anti-cheat: the launcher reports it)",
        kind: Kind::Choice(&["off", "on"]), applies: Applies::AtOnce, default: || json!("off") },
    Setting { name: "problem-reports", help: "on: players' launchers send their logs here when a game breaks (its connection dropped, an error) \
        and with Report a problem; read them with `reports` and `report ID`, or on the dashboard. off: nothing is sent or kept",
        kind: Kind::Choice(&["on", "off"]), applies: Applies::AtOnce, default: || json!("on") },
    Setting { name: "news", help: "The game's Information list: \"TITLE | BODY\" for one entry, or a JSON list of {title, body}",
        kind: Kind::News, applies: Applies::NextRequest, default: default_news },
    Setting { name: "maintenance-at", help: "When maintenance starts, in Unix seconds (0: none); `maintenance --in MINUTES` sets it. \
        Before: the game's countdown. From then: only admins and moderators sign in, the others are disconnected",
        kind: Kind::Number { min: 0, max: u32::MAX }, applies: Applies::AtOnce, default: || json!(0) },
    Setting { name: "maintenance-message", help: "What the game shows with the maintenance countdown", kind: Kind::Text { max: 200 },
        applies: Applies::AtOnce, default: || json!("The server goes down for maintenance.") },
    Setting { name: "max-players", help: "Players signed in at once; one more is told the server is full (the staff still comes in)",
        kind: Kind::Number { min: 1, max: 100_000 }, applies: Applies::AtOnce, default: || json!(500) },
    // 30: an NPC duel's engine peaks near 100 MB and a player duel's near 50 MB (measured, #27): about 3 GB at most.
    Setting { name: "max-duels", help: "Duels running at once, each with its rules engine; beyond it NPC duels and Create Duel are refused \
        (\"Duels are busy\") and Auto Duel waits", kind: Kind::Number { min: 1, max: 2000 }, applies: Applies::AtOnce, default: || json!(30) },
    // The client calls the Ultimate Match "PvP or PvCPU" (text 15261) and says nothing of when the computer plays: a guess.
    Setting { name: "ultimate-cpu-wait", help: "Seconds an Ultimate Match entrant waits for another player before the computer plays them \
        (0: only players)", kind: Kind::Number { min: 0, max: 3600 }, applies: Applies::AtOnce, default: || json!(30) },
    Setting { name: "max-connections", help: "TLS connections at once (a player's game holds several)",
        kind: Kind::Number { min: 16, max: 100_000 }, applies: Applies::AtOnce, default: || json!(2048) },
    // Real play peaks near 32 requests a second on the front and 6 on a duel connection (packet logs, #39).
    Setting { name: "request-burst", help: "Requests one game connection may make at once before they are paced (a connection keeps \
        the values it opened with)", kind: Kind::Number { min: 10, max: 100_000 }, applies: Applies::NextLogin, default: || json!(200) },
    Setting { name: "requests-per-second", help: "How fast a game connection's requests are answered past its burst (they wait, never dropped)",
        kind: Kind::Number { min: 1, max: 100_000 }, applies: Applies::NextLogin, default: || json!(20) },
    Setting { name: "outbox-max", help: "Messages that may wait for a game that reads nothing before its connection ends",
        kind: Kind::Number { min: 64, max: 1_000_000 }, applies: Applies::NextLogin, default: || json!(4096) },
    // A year: the log names players and moderation reasons, so it isn't kept for ever by default (#39), and a year of
    // moderation history still shows a repeat offender. 0 keeps every row.
    Setting { name: "audit-keep-days", help: "Days the admin audit log keeps a row, pruned at start, daily and when this changes (0: for ever)",
        kind: Kind::Number { min: 0, max: 36_500 }, applies: Applies::AtOnce, default: || json!(365) },
    // Two weeks: long enough for a player to mention a duel that went wrong; a duel's file is tens of kilobytes.
    Setting { name: "recording-days", help: "Days every duel's rules-engine recording is kept, for replaying a reported problem \
        (pinned ones, from the `bug` chat command, stay); pruned at start and daily (0: duels aren't recorded)",
        kind: Kind::Number { min: 0, max: 365 }, applies: Applies::AtOnce, default: || json!(14) },
    Setting { name: "handshake-burst", help: "TLS connections one address may open at once before it waits (this PC's are not counted)",
        kind: Kind::Number { min: 1, max: 10_000 }, applies: Applies::AtOnce, default: || json!(120) },
    Setting { name: "handshakes-per-second", help: "How fast an address gets its TLS connections back",
        kind: Kind::Number { min: 1, max: 10_000 }, applies: Applies::AtOnce, default: || json!(10) },
    Setting { name: "address-burst", help: "Account requests one address may make at once before it waits",
        kind: Kind::Number { min: 1, max: 1000 }, applies: Applies::AtOnce, default: || json!(10) },
    Setting { name: "address-refill-seconds", help: "Seconds that give an address back one account request",
        kind: Kind::Number { min: 1, max: 3600 }, applies: Applies::AtOnce, default: || json!(3) },
    Setting { name: "backoff-max-seconds", help: "The longest wait after failed passwords or codes for one User ID",
        kind: Kind::Number { min: 1, max: 86_400 }, applies: Applies::AtOnce, default: || json!(900) },
];

pub fn find(name: &str) -> Option<&'static Setting> {
    REGISTRY.iter().find(|s| s.name.eq_ignore_ascii_case(name.trim()))
}

/// What the shop asks (#16): BP for a pack and a structure deck, the cards a Trade Card swap takes, and the Rental's
/// offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shop {
    pub pack_cost: u32,
    pub deck_cost: u32,
    pub trade_cards: u32,
    pub rental_cards: u32,
    pub rental_bp: u32,
    pub rental_days: u32,
}

/// The settings at one moment, typed.
#[derive(Clone, Debug)]
pub struct Settings {
    pub server_name: String,
    pub headline: String,
    pub registration: Registration,
    pub min_launcher: Option<String>,
    /// The login banlist's id.
    pub login_regulation: u32,
    pub start_lp: u32,
    /// Duels play the reviewed anime card versions (issue #60).
    pub anime_cards: bool,
    /// Destiny Draw in NPC duels (issue #71).
    pub destiny_draw: bool,
    pub turn_seconds: u32,
    pub total_seconds: u32,
    pub news: Vec<(String, String)>,
    /// When maintenance starts (Unix seconds), if one is planned or running.
    pub maintenance_at: Option<i64>,
    pub maintenance_message: String,
    pub limits: Limits,
    /// The caps (#27).
    pub max_players: usize,
    pub max_duels: usize,
    /// Seconds before the computer plays an Ultimate Match entrant nobody joined (0: never).
    pub ultimate_cpu_wait: u32,
    pub max_connections: usize,
    pub handshake_burst: u32,
    pub handshakes_per_second: u32,
    /// Each game connection's request pace and outbox (#39).
    pub pace: crate::net::Pace,
    /// Days the audit log keeps a row (0: for ever).
    pub audit_keep_days: u32,
    /// Days a duel's recording is kept (0: duels aren't recorded).
    pub recording_days: u32,
    /// The shop's prices (#16) and the reports a player may send in an hour.
    pub shop: Shop,
    pub report_limit: u32,
    /// Games one PC may run at once, and whether accounts seen together are treated as linked (#64).
    pub games_per_device: u32,
    pub linked_fair: bool,
    /// The launcher refuses a game with other programs inside (#69).
    pub clean_game: bool,
    /// Players' launchers may send problem reports (`problems.rs`).
    pub problem_reports: bool,
    /// Every setting's value by name, defaults included.
    values: Map<String, Json>,
}

impl Settings {
    /// Whether maintenance is on at `now` (it started and wasn't called off).
    pub fn in_maintenance(&self, now: i64) -> bool { self.maintenance_at.is_some_and(|t| t <= now) }

    /// Seconds until a planned maintenance starts, for the game's countdown.
    pub fn maintenance_in(&self, now: i64) -> Option<i64> { self.maintenance_at.map(|t| t - now).filter(|&s| s > 0) }

    /// The value of `name` (its default unless set).
    pub fn value(&self, name: &str) -> &Json { &self.values[name] }

    fn from_values(values: Map<String, Json>, banlists: &[(u32, String)]) -> Result<Settings, String> {
        let text = |n: &str| values[n].as_str().unwrap_or_default().to_string();
        let number = |n: &str| values[n].as_u64().unwrap_or_default() as u32;
        let registration = match text("registration").as_str() { "open" => Registration::Open, "closed" => Registration::Closed, _ => Registration::Invite };
        let news = values["news"].as_array().into_iter().flatten()
            .map(|e| (e["title"].as_str().unwrap_or_default().to_string(), e["body"].as_str().unwrap_or_default().to_string())).collect();
        Ok(Settings {
            server_name: text("server-name"),
            headline: text("headline"),
            registration,
            min_launcher: Some(text("min-launcher")).filter(|v| !v.is_empty()),
            login_regulation: regulation(&text("login-regulation"), banlists)?,
            start_lp: number("start-lp"),
            anime_cards: text("anime-cards") == "on",
            destiny_draw: text("destiny-draw") == "on",
            turn_seconds: number("turn-seconds"),
            total_seconds: number("total-seconds"),
            news,
            maintenance_at: Some(i64::from(number("maintenance-at"))).filter(|&t| t > 0),
            maintenance_message: text("maintenance-message"),
            max_players: number("max-players") as usize,
            max_duels: number("max-duels") as usize,
            ultimate_cpu_wait: number("ultimate-cpu-wait"),
            max_connections: number("max-connections") as usize,
            handshake_burst: number("handshake-burst"),
            handshakes_per_second: number("handshakes-per-second"),
            audit_keep_days: number("audit-keep-days"),
            recording_days: number("recording-days"),
            shop: Shop { pack_cost: number("pack-cost"), deck_cost: number("deck-cost"), trade_cards: number("trade-cards"),
                rental_cards: number("rental-cards"), rental_bp: number("rental-bp"), rental_days: number("rental-days") },
            report_limit: number("report-limit"),
            games_per_device: number("games-per-device"),
            linked_fair: text("linked-accounts") != "off",
            clean_game: text("clean-game") == "on",
            problem_reports: text("problem-reports") != "off",
            pace: crate::net::Pace { burst: number("request-burst"), per_second: number("requests-per-second"), outbox_max: number("outbox-max") as usize },
            limits: Limits { burst: number("address-burst"), refill_seconds: number("address-refill-seconds"), max_backoff_seconds: number("backoff-max-seconds") },
            values,
        })
    }
}

/// The banlist id `value` names: an id or a name of the client's, or empty for the default.
fn regulation(value: &str, banlists: &[(u32, String)]) -> Result<u32, String> {
    if value.trim().is_empty() { return Ok(crate::front::LOGIN_REGULATION) }
    if banlists.is_empty() {
        // Without the game's data the names are unknown: an id is taken as it is.
        return value.trim().parse().map_err(|_| "a banlist by name needs the game's data: give its id".to_string());
    }
    crate::regulation_by(value, banlists)
}

/// Reads a typed value for `setting` from what a host wrote, and checks it.
pub fn parse(setting: &Setting, input: &str, banlists: &[(u32, String)]) -> Result<Json, String> {
    let input = input.trim();
    let name = setting.name;
    match setting.kind {
        Kind::Text { max } => {
            if input.chars().count() > max || input.chars().any(char::is_control) { return Err(format!("{name} is at most {max} characters, on one line")) }
            Ok(json!(input))
        }
        Kind::Number { min, max } => match input.parse::<u32>() {
            Ok(n) if (min..=max).contains(&n) => Ok(json!(n)),
            _ => Err(format!("{name} is a whole number from {min} to {max}")),
        },
        Kind::Choice(choices) => choices.iter().find(|c| c.eq_ignore_ascii_case(input)).map(|c| json!(c))
            .ok_or_else(|| format!("{name} is one of: {}", choices.join(", "))),
        Kind::Version => {
            let bare = input.trim_start_matches(['v', 'V']);
            let ok = bare.is_empty() || bare.split('.').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
            if ok { Ok(json!(bare)) } else { Err(format!("{name} is a version such as 0.2.0, or empty")) }
        }
        Kind::Regulation => regulation(input, banlists).map(|_| json!(input)),
        Kind::News => {
            let entries: Vec<(String, String)> = if input.starts_with('[') {
                let list: Vec<Json> = serde_json::from_str(input).map_err(|e| format!("news as JSON: {e}"))?;
                list.iter().map(|e| (e["title"].as_str().unwrap_or_default().to_string(), e["body"].as_str().unwrap_or_default().to_string())).collect()
            } else {
                let (title, body) = input.split_once('|').unwrap_or((input, ""));
                vec![(title.trim().to_string(), body.trim().to_string())]
            };
            // The game's list shows a short title and a body (128 and 2048, the sizes mail keeps), at most ten entries.
            // At least one: the client indexes the list's first entry, and an empty list it indexes crashes it.
            if entries.is_empty() || entries.len() > 10 || entries.iter().any(|(t, b)| t.is_empty() || t.chars().count() > 128 || b.chars().count() > 2048) {
                return Err("news is 1 to 10 entries, each with a title (up to 128 characters) and a body (up to 2048)".into());
            }
            Ok(Json::Array(entries.into_iter().map(|(title, body)| json!({ "title": title, "body": body })).collect()))
        }
    }
}

/// The settings of a running server (or of a data folder, for the admin commands).
pub struct Live {
    current: RwLock<Arc<Settings>>,
    /// The values a host set (not their defaults), as saved.
    set: Mutex<Map<String, Json>>,
    /// Where they are saved (none: in memory only).
    file: Option<PathBuf>,
    /// The client's banlists (id and name), to read a banlist by name; empty without the game's data.
    banlists: Vec<(u32, String)>,
}

/// Every setting's value: those set, else the defaults.
fn with_defaults(set: &Map<String, Json>) -> Map<String, Json> {
    REGISTRY.iter().map(|s| (s.name.to_string(), set.get(s.name).cloned().unwrap_or_else(s.default))).collect()
}

impl Live {
    /// The settings saved at `file` (none: in memory only). A saved value that no longer passes its check (an unknown
    /// name, a banlist the game lost) is left out with a warning, so the server still starts.
    pub fn open(file: Option<&Path>, banlists: Vec<(u32, String)>) -> io::Result<Live> {
        let saved = match file { Some(f) => crate::store::read(f)?, None => None };
        let mut set = Map::new();
        for (name, value) in saved.and_then(|s| s.as_object().cloned()).unwrap_or_default() {
            let Some(setting) = find(&name) else { tracing::warn!("setting {name} is not known: ignored"); continue };
            let input = match &value { Json::String(s) => s.clone(), other => other.to_string() };
            match parse(setting, &input, &banlists) {
                Ok(value) => { set.insert(setting.name.to_string(), value); }
                Err(e) => tracing::warn!("setting {name} ignored, its default is used: {e}"),
            }
        }
        let settings = Settings::from_values(with_defaults(&set), &banlists).map_err(io::Error::other)?;
        Ok(Live { current: RwLock::new(Arc::new(settings)), set: Mutex::new(set), file: file.map(Path::to_path_buf), banlists })
    }

    /// The settings now: a snapshot that a later change doesn't touch.
    pub fn now(&self) -> Arc<Settings> { self.current.read().unwrap_or_else(|e| e.into_inner()).clone() }

    /// Sets `name` from a host's text, saves it, and swaps the new settings in; what to tell the host.
    pub fn set(&self, name: &str, input: &str) -> Result<String, String> {
        let setting = find(name).ok_or_else(|| format!("no setting {name:?}; `settings` lists them"))?;
        let value = parse(setting, input, &self.banlists)?;
        self.change(setting, Some(value))
    }

    /// Puts `name` back to its default.
    pub fn reset(&self, name: &str) -> Result<String, String> {
        let setting = find(name).ok_or_else(|| format!("no setting {name:?}; `settings` lists them"))?;
        self.change(setting, None)
    }

    fn change(&self, setting: &Setting, value: Option<Json>) -> Result<String, String> {
        let input = value.map(|v| match v { Json::String(s) => s, other => other.to_string() });
        let next = self.prepare(&[(setting.name.to_string(), input)])?;
        let message = next.message(setting.name);
        next.persist().map_err(|e| format!("not saved: {e}"))?;
        next.publish();
        Ok(message)
    }

    /// Serialize all writers, validate the complete edit, then let the caller commit it with audit and receipt.
    pub(crate) fn prepare(&self, edits: &[(String, Option<String>)]) -> Result<Prepared<'_>, String> {
        let set = self.set.lock().unwrap_or_else(|e| e.into_inner());
        let before = self.now(); let mut next = set.clone();
        for (name, input) in edits {
            let setting = find(name).ok_or_else(|| format!("no setting {name:?}; `settings` lists them"))?;
            match input {
                Some(input) => { next.insert(setting.name.into(), parse(setting, input, &self.banlists)?); }
                None => { next.remove(setting.name); }
            }
        }
        let settings = Arc::new(Settings::from_values(with_defaults(&next), &self.banlists)?);
        Ok(Prepared { live: self, set, next, before, settings })
    }

    pub(crate) fn snapshot(&self) -> (Arc<Settings>, Map<String,Json>) {
        let set = self.set.lock().unwrap_or_else(|e| e.into_inner());
        (self.now(), set.clone())
    }
    pub(crate) fn banlists(&self) -> &[(u32, String)] { &self.banlists }

    /// Every setting as a row (the admin panel, #36): its name, value as typed, whether it was set, when it applies, what it
    /// does, and its choices for a choice.
    pub fn rows(&self) -> Vec<Json> {
        let set = self.set.lock().unwrap_or_else(|e| e.into_inner());
        let now = self.now();
        REGISTRY.iter().map(|s| {
            let value = match now.value(s.name) { Json::String(v) => v.clone(), other => other.to_string() };
            let choices = match s.kind { Kind::Choice(c) => json!(c), _ => Json::Null };
            json!({ "name": s.name, "value": value, "set": set.contains_key(s.name), "applies": s.applies.describe(), "help": s.help,
                "choices": choices })
        }).collect()
    }

    /// Every setting with its value, whether it is the default, when it applies and what it does.
    pub fn list(&self) -> String {
        let set = self.set.lock().unwrap_or_else(|e| e.into_inner());
        let now = self.now();
        REGISTRY.iter().map(|s| {
            let state = if set.contains_key(s.name) { "set" } else { "default" };
            format!("{}\t{}\t({state}; applies {})\n  {}", s.name, shown(now.value(s.name)), s.applies.describe(), s.help)
        }).collect::<Vec<_>>().join("\n")
    }
}

/// Holding the override lock prevents a second edit from preparing against stale settings. Publish only after COMMIT.
pub(crate) struct Prepared<'a> {
    live: &'a Live, set: MutexGuard<'a, Map<String, Json>>, next: Map<String, Json>,
    pub before: Arc<Settings>, pub settings: Arc<Settings>,
}
impl Prepared<'_> {
    pub fn persist(&self) -> io::Result<()> {
        if let Some(file) = &self.live.file { crate::store::write(file, &Json::Object(self.next.clone()))?; }
        Ok(())
    }
    pub fn message(&self, name: &str) -> String {
        let setting = find(name).expect("prepared setting");
        format!("{} is now {} (applies {})", setting.name, shown(self.settings.value(setting.name)), setting.applies.describe())
    }
    pub fn publish(mut self) {
        *self.set = self.next;
        *self.live.current.write().unwrap_or_else(|e| e.into_inner()) = self.settings;
    }
}

/// A value as the host reads it: text bare, empty text as (none), the rest as JSON.
fn shown(value: &Json) -> String {
    match value {
        Json::String(s) if s.is_empty() => "(none)".into(),
        Json::String(s) => s.clone(),
        Json::Array(entries) => entries.iter().map(|e| format!("{} | {}", e["title"].as_str().unwrap_or_default(), e["body"].as_str().unwrap_or_default()))
            .collect::<Vec<_>>().join(" / "),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn banlists() -> Vec<(u32, String)> { vec![(1, "Beta".into()), (20, "Season 16".into())] }

    #[test]
    fn every_setting_has_a_valid_default_and_the_defaults_are_todays_behaviour() {
        let live = Live::open(None, banlists()).unwrap();
        let now = live.now();
        assert_eq!((now.registration, now.login_regulation, now.start_lp), (Registration::Invite, crate::front::LOGIN_REGULATION, 8000));
        assert_eq!((now.turn_seconds, now.total_seconds), (crate::clock::DEFAULT_TURN, crate::clock::DEFAULT_TOTAL));
        assert_eq!(now.limits, Limits::default());
        assert!(!now.anime_cards, "the anime card versions are off until a host turns them on");
        for s in REGISTRY {
            let input = match (s.default)() { Json::String(v) => v, other => other.to_string() };
            parse(s, &input, &banlists()).unwrap_or_else(|e| panic!("{}: {e}", s.name));
        }
    }

    #[test]
    fn a_value_is_checked_by_its_kind() {
        let live = Live::open(None, banlists()).unwrap();
        assert!(live.set("start-lp", "50").unwrap_err().contains("100 to 99999"));
        assert!(live.set("start-lp", "lots").is_err());
        assert!(live.set("registration", "sometimes").unwrap_err().contains("invite, open, closed"));
        assert!(live.set("min-launcher", "soon").is_err());
        assert!(live.set("login-regulation", "Season 99").is_err());
        assert!(live.set("server-name", "two\nlines").is_err());
        assert!(live.set("news", "| no title").is_err());
        assert!(live.set("news", "[]").is_err(), "the game's list needs an entry");
        assert!(live.set("nothing", "1").unwrap_err().contains("no setting"));
        assert!(live.set("anime-cards", "sometimes").unwrap_err().contains("off, on"));
        assert_eq!(live.set("anime-cards", "ON").unwrap(), "anime-cards is now on (applies at the next duel)");
        assert!(live.now().anime_cards);
        live.reset("anime-cards").unwrap();
        assert!(!live.now().anime_cards);
        assert!(!live.now().destiny_draw, "Destiny Draw is off until a host turns it on");
        live.set("destiny-draw", "on").unwrap();
        assert!(live.now().destiny_draw);
        live.reset("destiny-draw").unwrap();
        assert!(!live.now().destiny_draw);
        assert_eq!(live.set("Registration", "OPEN").unwrap(), "registration is now open (applies at once)");
        assert_eq!(live.set("login-regulation", "season 16").unwrap(), "login-regulation is now season 16 (applies at each player's next login)");
        assert_eq!(live.now().login_regulation, 20);
        assert_eq!(live.set("min-launcher", "v0.2.0").unwrap(), "min-launcher is now 0.2.0 (applies at once)");
        live.set("news", "Maintenance | Sunday at 10:00").unwrap();
        assert_eq!(live.now().news, [("Maintenance".to_string(), "Sunday at 10:00".to_string())]);
    }

    #[test]
    fn a_snapshot_taken_before_a_change_keeps_its_values() {
        let live = Live::open(None, banlists()).unwrap();
        let before = live.now();
        live.set("start-lp", "4000").unwrap();
        assert_eq!((before.start_lp, live.now().start_lp), (8000, 4000));
        live.reset("start-lp").unwrap();
        assert_eq!(live.now().start_lp, 8000);
    }

    #[test]
    fn values_are_saved_and_a_bad_saved_one_falls_back_to_its_default() {
        let dir = std::env::temp_dir().join(format!("yo3-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("settings.json");
        Live::open(Some(&file), banlists()).unwrap().set("turn-seconds", "90").unwrap();
        assert_eq!(Live::open(Some(&file), banlists()).unwrap().now().turn_seconds, 90);
        crate::store::write(&file, &json!({ "turn-seconds": 90, "start-lp": 5, "gone": 1 })).unwrap();
        let live = Live::open(Some(&file), banlists()).unwrap();
        assert_eq!((live.now().turn_seconds, live.now().start_lp), (90, 8000));
        assert!(live.list().contains("turn-seconds\t90\t(set; applies at the next duel)"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
