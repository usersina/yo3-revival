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
use std::sync::{Arc, Mutex, RwLock};

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
    Setting { name: "min-launcher", help: "Refuse sign-ins from launchers older than this version (empty: any)", kind: Kind::Version,
        applies: Applies::AtOnce, default: || json!("") },
    Setting { name: "login-regulation", help: "The banlist players log in with, for Auto Duel and NPC duels: an id or a name (empty: Season 16)",
        kind: Kind::Regulation, applies: Applies::NextLogin, default: || json!("") },
    Setting { name: "start-lp", help: "Each player's LP at the start of an Auto Duel or NPC duel (a hosted duel has its own)",
        kind: Kind::Number { min: 100, max: 99_999 }, applies: Applies::NextDuel, default: || json!(8000) },
    Setting { name: "turn-seconds", help: "A turn's clock in duels between players, when the duel sets none (0: off)",
        kind: Kind::Number { min: 0, max: 3600 }, applies: Applies::NextDuel, default: || json!(crate::clock::DEFAULT_TURN) },
    Setting { name: "total-seconds", help: "Each player's total clock in duels between players, when the duel sets none (0: off)",
        kind: Kind::Number { min: 0, max: 36_000 }, applies: Applies::NextDuel, default: || json!(crate::clock::DEFAULT_TOTAL) },
    Setting { name: "news", help: "The game's Information list: \"TITLE | BODY\" for one entry, or a JSON list of {title, body}",
        kind: Kind::News, applies: Applies::NextRequest, default: default_news },
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
    pub turn_seconds: u32,
    pub total_seconds: u32,
    pub news: Vec<(String, String)>,
    pub limits: Limits,
    /// Every setting's value by name, defaults included.
    values: Map<String, Json>,
}

impl Settings {
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
            turn_seconds: number("turn-seconds"),
            total_seconds: number("total-seconds"),
            news,
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
        let mut set = self.set.lock().unwrap_or_else(|e| e.into_inner());
        let mut next = set.clone();
        match value { Some(v) => { next.insert(setting.name.into(), v); } None => { next.remove(setting.name); } }
        let settings = Settings::from_values(with_defaults(&next), &self.banlists)?;
        if let Some(file) = &self.file {
            crate::store::write(file, &Json::Object(next.clone())).map_err(|e| format!("not saved: {e}"))?;
        }
        *set = next;
        let shown = shown(settings.value(setting.name));
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(settings);
        Ok(format!("{} is now {shown} (applies {})", setting.name, setting.applies.describe()))
    }

    /// Every setting with its value, whether it is the default, when it applies and what it does.
    pub fn list(&self) -> String {
        let now = self.now();
        let set = self.set.lock().unwrap_or_else(|e| e.into_inner());
        REGISTRY.iter().map(|s| {
            let state = if set.contains_key(s.name) { "set" } else { "default" };
            format!("{}\t{}\t({state}; applies {})\n  {}", s.name, shown(now.value(s.name)), s.applies.describe(), s.help)
        }).collect::<Vec<_>>().join("\n")
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
