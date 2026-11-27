//! Desktop application model and process control, independent of the windowing toolkit.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use anyhow::{Context, ensure};
use rusqlite::{Connection, OptionalExtension};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use yo3_server::admin;
use yo3_server::api::{self, Op};
pub mod network;
pub mod privileged;
pub mod direct;
pub mod foreign;
pub mod reports;
pub mod system_info;
pub mod bug_report;
pub mod updates;
pub mod preferences;
pub mod server_options;
pub mod game_data;
pub mod instance;
pub mod store;
pub mod setup_check;
pub mod singleplayer;
pub mod games;
pub mod shortcut;
pub mod multiplayer;
pub mod options;
pub mod words;
pub mod hosting;
pub mod pictures;
pub mod anime_cards;
mod game_data_platform;
mod process;
mod github_http;

pub fn server_address(input: &str) -> anyhow::Result<(String, String)> { yo3_tunnel::invite::address(input) }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub advanced_mode: bool,
    pub server_options: std::collections::BTreeMap<String, String>,
    pub game_dir: String,
    pub wine_prefix: String,
    pub host_address: String,
    pub data_dir: String,
    pub bind_address: String,
    pub bind_interface: Option<String>,
    pub listen_port: u16,
    pub player_name: String,
    pub local_account: String,
    /// The connection DLL's higher resolution ("" or "off", "auto", "WIDTHxHEIGHT"); see yo3_launcher::resolution.
    pub resolution: String,
    /// The server's banlist (`--regulation`): "" for the default (the game's unlimited list, `apply_banlist`), else a regulation id, or a
    /// name kept from an older `--regulation` server flag.
    pub banlist: String,
    /// The setup wizard was finished (#28); until then the launcher opens on it.
    pub setup_complete: bool,
    /// `yo3.exe` at the last full check (`setup_check::fingerprint`); a change means checking everything again.
    pub game_fingerprint: String,
    /// Open the game without the launcher (direct start), chosen in setup.
    pub direct_start: bool,
    /// Linux: the Wine program ("" for `wine` on the PATH), handed to the helper as `YO3_WINE`.
    pub wine_binary: String,
    /// Your servers' log level: "" or `options::DETAILED`.
    pub log_level: String,
    /// Host for friends (#37): the port friends connect to.
    pub host_port: u16,
    /// Host for friends: the address friends dial ("" for this PC's network address).
    pub host_share: String,
    /// Host for friends: the server's name in friends' lists ("" for the default).
    pub host_name: String,
    /// When a game breaks, its logs go to the server it played on, for the host (problem reports). Asked in setup; on
    /// unless the player turns it off.
    pub problem_reports: bool,
    /// Card pictures (anime cards' and the cards the game never had, #75) come from ProjectIgnis by themselves: at
    /// setup's end and whenever some are missing at the launcher's start. Asked in setup (ticked); off for a launcher set
    /// up before it, which keeps Settings → Game's Get pictures.
    pub card_pictures: bool,
    /// A new version downloads by itself as soon as the check finds it (Settings → About). Off: the footer says it is
    /// available and the player downloads it from the Update window.
    pub auto_updates: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { advanced_mode: false, server_options: Default::default(), game_dir: yo3_data::default_game_dir().to_string_lossy().into(), wine_prefix: String::new(),
            host_address: "localhost".into(), data_dir: String::new(), bind_address: "0.0.0.0".into(), bind_interface: None, listen_port: 7443,
            player_name: "Duelist".into(), local_account: String::new(), resolution: String::new(),
            banlist: String::new(), setup_complete: false, game_fingerprint: String::new(), direct_start: true,
            wine_binary: String::new(), log_level: String::new(), host_port: 7443, host_share: String::new(), host_name: String::new(), problem_reports: true, card_pictures: false, auto_updates: false }
    }
}

/// How the game's helper signs in to your own server (Singleplayer): the account key of your local account, and the
/// server's identity. Written to a private profile for each Play; never handed to anyone.
#[derive(Clone)]
pub struct KeyConnection {
    pub address: String,
    pub server_name: String,
    pub certificate: String,
    pub identity: Option<String>,
    pub key: String,
}
impl KeyConnection {
    pub fn endpoint(&self) -> anyhow::Result<yo3_tunnel::Endpoint> {
        let identity = self.identity.clone().map(Ok).unwrap_or_else(|| yo3_tunnel::identity_from_pem(self.certificate.as_bytes()))?;
        yo3_tunnel::Endpoint::pinned(self.address.clone(), self.server_name.clone(), &identity)
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        server_address(&self.address)?;
        ensure!(self.key.is_ascii() && self.key.len() == 20, "An account access key has 20 characters.");
        ensure!(self.certificate.contains("-----BEGIN CERTIFICATE-----"), "Your server's certificate is missing.");
        ensure!(!self.server_name.is_empty(), "The certificate name is missing.");
        self.endpoint()?;
        Ok(())
    }
}

pub fn private_write(path: &Path, bytes: &[u8], replace: bool) -> anyhow::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    if replace { options.create(true).truncate(true); } else { options.create_new(true); }
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    options.open(path)?.write_all(bytes)?;
    Ok(())
}

/// The `user_version` every released launcher accepts (see `LauncherDb::migrate`). Raising it locks out every launcher
/// before the change, including an original shortcut's forwarding to the installed update.
pub const COMPATIBLE: u32 = 1;
/// The launcher database's steps in order: step N (from 1) brings the schema from N-1 to N. Append only, never edit a
/// released step. Released launchers recreate their own tables (`settings`, `servers`) if a step drops them.
const MIGRATIONS: &[&str] = &[
    // 1: what v0.0.x launchers made (their `update_state` is created where it is used, `updates::install`).
    "CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);
     CREATE TABLE IF NOT EXISTS servers (id INTEGER PRIMARY KEY, body TEXT NOT NULL);",
    // 2: the Multiplayer servers the player added or played on (#30), by their first identity.
    "CREATE TABLE IF NOT EXISTS remote_servers (key TEXT PRIMARY KEY, body TEXT NOT NULL);",
    // 3: no `servers`, v0.0.21's connections saved with a friend's account key (#23). A released launcher that opens
    // the database again recreates it empty, which is harmless.
    "DROP TABLE IF EXISTS servers;",
];
/// The schema this launcher writes.
pub const LAUNCHER_SCHEMA: u32 = MIGRATIONS.len() as u32;

pub struct LauncherDb { conn: Connection }
impl LauncherDb {
    pub fn open(root: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(root)?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?; }
        let path = root.join("launcher.db");
        if !path.exists() { private_write(&path, &[], false)?; }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }
    /// Brings the database to `LAUNCHER_SCHEMA` and keeps it openable by every released launcher.
    ///
    /// `user_version` is the *compatibility* marker, not the schema: released launchers (v0.0.21 and older) refuse a
    /// database whose `user_version` is above 1, and they open this database on every start (to forward an original
    /// shortcut to the installed update) and to promote an update they launched. A database they refuse strands the
    /// player on the old version, so `user_version` stays `COMPATIBLE` while every change keeps their tables
    /// (`settings`, `servers`, `update_state`) readable. The schema's own revision lives in `launcher_schema`; a step
    /// that drops or reshapes data goes in `MIGRATIONS` without touching `user_version`. Beta development builds wrote
    /// `user_version` 2 for the additive `remote_servers` table; such a database is accepted and set back to 1.
    fn migrate(conn: &Connection) -> anyhow::Result<()> {
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        ensure!(version <= 2, "Launcher database is newer than this application.");
        // Dropping `servers` (step 3) overwrites the old keys' bytes instead of leaving them in free pages.
        conn.execute_batch("PRAGMA secure_delete=ON;")?;
        conn.execute_batch("BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS launcher_schema (id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL);")?;
        let result = (|| -> anyhow::Result<()> {
            let saved: Option<u32> = conn.query_row("SELECT version FROM launcher_schema WHERE id=1", [], |r| r.get(0)).optional()?;
            // Without the table: a new database (0), a released launcher's (1) or a beta development build's (2).
            let schema = saved.unwrap_or(version);
            for (step, sql) in MIGRATIONS.iter().enumerate().skip(schema as usize) { conn.execute_batch(sql).with_context(|| format!("Launcher database step {}", step + 1))?; }
            // Never lower it: a newer launcher that kept the database compatible may have moved it further.
            conn.execute("INSERT INTO launcher_schema VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET version=max(version,excluded.version)", [LAUNCHER_SCHEMA])?;
            conn.execute_batch(&format!("PRAGMA user_version={COMPATIBLE};"))?;
            Ok(())
        })();
        match result {
            Ok(()) => { conn.execute_batch("COMMIT;")?; Ok(()) }
            Err(error) => { let _ = conn.execute_batch("ROLLBACK;"); Err(error) }
        }
    }
    pub fn settings(&self) -> anyhow::Result<Settings> {
        let data: Option<String> = self.conn.query_row("SELECT body FROM settings WHERE id=1", [], |r| r.get(0)).optional()?;
        let Some(body) = data else { return Ok(Settings::default()) };
        let value: Value = serde_json::from_str(&body)?;
        let mut settings: Settings = serde_json::from_value(value.clone())?;
        if value.get("bind_address").is_none() {
            settings.bind_address = if value["allow_friends"].as_bool().unwrap_or(false) { "0.0.0.0" } else { "127.0.0.1" }.into();
            let (address, name) = server_address(&settings.host_address)?;
            settings.listen_port = address.rsplit_once(':').unwrap().1.parse()?;
            settings.host_address = name;
            self.save_settings(&settings)?;
        }
        // A banlist chosen before Hosting had its own control was a --regulation server flag: it becomes the Banlist.
        if let Some(regulation) = settings.server_options.remove("regulation") {
            if settings.banlist.is_empty() { settings.banlist = regulation; }
            self.save_settings(&settings)?;
        }
        Ok(settings)
    }
    pub fn save_settings(&self, settings: &Settings) -> anyhow::Result<()> {
        self.conn.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET body=excluded.body", [serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn save_mode(&self, advanced: bool) -> anyhow::Result<()> {
        let mut saved = self.settings()?; saved.advanced_mode = advanced; self.save_settings(&saved)
    }
    /// The higher resolution for the next Play, stored in its normal form ("off", "auto" or "WIDTHxHEIGHT").
    pub fn save_resolution(&self, value: &str) -> anyhow::Result<String> {
        Ok(self.change(&options::Change::Resolution(value.into()))?.resolution)
    }
    /// The banlist for the next server start: "" for the server's default, else a regulation id (1-255) or a name
    /// (letters, digits, spaces and "/.-()"); the server checks it against the game's lists when it starts.
    pub fn save_banlist(&self, value: &str) -> anyhow::Result<String> {
        Ok(self.change(&options::Change::Banlist(value.into()))?.banlist)
    }
    /// The Multiplayer servers the player added or played on.
    pub fn remote_servers(&self) -> anyhow::Result<Vec<multiplayer::Server>> {
        let mut stmt = self.conn.prepare("SELECT body FROM remote_servers")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|body| Ok(serde_json::from_str(&body?)?)).collect()
    }
    pub fn save_remote_server(&self, server: &multiplayer::Server) -> anyhow::Result<()> {
        ensure!(!server.key().is_empty(), "A server needs its identity.");
        self.conn.execute("INSERT INTO remote_servers VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET body=excluded.body", (server.key(), serde_json::to_string(server)?))?;
        Ok(())
    }
    pub fn delete_remote_server(&self, key: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM remote_servers WHERE key=?1", [key])?;
        Ok(())
    }
}

pub fn default_root() -> PathBuf {
    if let Some(root) = std::env::var_os("YO3_DESKTOP_DATA") { return root.into() }
    #[cfg(windows)] { PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("YO3 Revival") }
    #[cfg(not(windows))] {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")).join("yo3-revival")
    }
}

#[derive(Clone)]
pub struct Runtime { pub server: PathBuf, pub launcher: PathBuf, pub injector: PathBuf, pub dll: PathBuf, pub engine: PathBuf,
    /// The rules engine program (`yo3-engine`) beside the server; `engine` is its data directory.
    pub engine_program: PathBuf }
impl Runtime {
    pub fn discover() -> anyhow::Result<Self> {
        let exe = std::env::current_exe()?;
        let bin = exe.parent().context("Application folder is unavailable.")?;
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let development = yo3_tunnel::BUILD_VERSION.ends_with("-dev") && !bin.join("bundle.json").exists();
        let root = if !development || bin.join(format!("yo3-server{}", std::env::consts::EXE_SUFFIX)).is_file() { bin.to_path_buf() }
            else { project.join("target/release") };
        let mods = if !development || root.join("yo3-auth.dll").is_file() { root.clone() } else { project.join("tools/mod/build") };
        let engine = if !development || root.join("engine").is_dir() { root.join("engine") } else { project.join("engine") };
        Ok(Self { server: root.join(format!("yo3-server{}", std::env::consts::EXE_SUFFIX)),
            launcher: root.join(format!("yo3-launcher{}", std::env::consts::EXE_SUFFIX)), injector: mods.join("yo3-auth-launch.exe"), dll: mods.join("yo3-auth.dll"), engine,
            engine_program: root.join(yo3_server::engine::program_name()) })
    }
}

/// Unlock all cards' numbers (`Backend::collection`): your player's cards out of every card (each artwork counted), how
/// many an unlock would give now, and whether every player's cards were unlocked before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collection { pub owned: u64, pub total: u64, pub new: u64, pub unlocked: bool }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness { Ready, GameProblem(String), ComponentProblem(String), ValidationProblem(String) }
pub struct Backend { pub root: PathBuf, pub runtime: Runtime, pub diagnostics: yo3_tunnel::diagnostics::Session,
    server_session: Mutex<Option<String>>, game: Mutex<Option<Child>>, game_errors: Mutex<Option<process::Errors>>, game_ready: Mutex<Option<PathBuf>>, server: Mutex<Option<Child>>,
    /// The write end of the server's standard input: while this launcher lives it stays open (`--owned`).
    server_owner: Mutex<Option<std::process::ChildStdin>>,
    /// Host for friends' server (#37) and the write end of its standard input (`--owned`), like your own server's.
    hosted: Mutex<Option<Child>>, hosted_owner: Mutex<Option<std::process::ChildStdin>>,
    /// Another program's file or provider stopped the last start (#68), until the window has said so.
    held: Mutex<Option<foreign::Held>>,
    /// The player chose Continue anyway: the next start goes past what it found (#68).
    accept_foreign: std::sync::atomic::AtomicBool,
    /// The server of the next start asks for a clean game (#69): the helper stops it for anything found inside the game.
    clean_game: std::sync::atomic::AtomicBool,
    /// More games from this launcher (#64): the extra ones running, who the main one is for, the counter of their ids and
    /// why one ended badly (said once).
    extras: Mutex<Vec<games::Extra>>, main_label: Mutex<Option<games::GameInfo>>, next_game: std::sync::atomic::AtomicU64, extra_failures: Mutex<Vec<String>>,
    /// Where Singleplayer's account key is kept (#23): the system's credential store.
    vault: Arc<dyn multiplayer::Vault> }
impl Backend {
    pub fn new(root: PathBuf) -> anyhow::Result<Arc<Self>> { Self::with_vault(root, Arc::new(multiplayer::SystemVault)) }
    /// A backend keeping Singleplayer's key in `vault` (tests use one in memory).
    pub fn with_vault(root: PathBuf, vault: Arc<dyn multiplayer::Vault>) -> anyhow::Result<Arc<Self>> {
        yo3_tunnel::diagnostics::private_dir(&root.join("profiles"))?;
        std::fs::create_dir_all(root.join("logs"))?;
        let diagnostics = yo3_tunnel::diagnostics::Session::start(&root.join("logs/sessions"), "app", None)?;
        // This install's id for the servers' linked-account rules (#64), and the Wine prefixes a crash left behind.
        multiplayer::init_device_id(&root);
        Self::clean_stale_prefixes(&root);
        Ok(Arc::new(Self { root, runtime: Runtime::discover()?, diagnostics, server_session: Mutex::new(None), game: Mutex::new(None), game_errors: Mutex::new(None), game_ready: Mutex::new(None), server: Mutex::new(None), server_owner: Mutex::new(None), hosted: Mutex::new(None), hosted_owner: Mutex::new(None), held: Mutex::new(None), accept_foreign: Default::default(), clean_game: Default::default(),
            extras: Default::default(), main_label: Default::default(), next_game: Default::default(), extra_failures: Default::default(), vault }))
    }
    /// The store Singleplayer's key is kept in.
    pub fn vault(&self) -> &dyn multiplayer::Vault { self.vault.as_ref() }
    /// Keeps your own server's account key in the credential store, under the server's identity: the profile fields
    /// that find it (`key_vault`). With no store, the owner-only profile holds the key itself, and the log says so.
    pub(crate) fn key_fields(&self, identity: &str, key: &str) -> Value {
        let entry = yo3_launcher::vault::key_account(identity);
        match self.vault.set(&entry, key).and_then(|()| self.vault.get(&entry)) {
            Ok(Some(kept)) if kept == key => serde_json::json!({ "key_vault": entry }),
            Ok(_) => { self.no_store("it didn't keep the key"); serde_json::json!({ "key": key }) }
            Err(error) => { self.no_store(&format!("{error:#}")); serde_json::json!({ "key": key }) }
        }
    }
    fn no_store(&self, why: &str) {
        self.diagnostics.line(&format!("No credential store for Singleplayer's account key ({why}): it stays in the launcher's owner-only file"));
    }
    /// The first start after an update: a key an older launcher left in `profiles/connection-0.json` or
    /// `profiles/direct.json` moves into the credential store, and out of the file. Direct start's helper reads the
    /// store of the game's own system (under Wine, the prefix's): `mirror` copies the key there first, and direct.json
    /// keeps its copy when that fails.
    pub fn migrate_keys(&self, mirror: &dyn Fn(&str, &str) -> anyhow::Result<()>) {
        for (name, direct) in [("connection-0.json", false), ("direct.json", true)] {
            let file = self.root.join("profiles").join(name);
            let Ok(bytes) = std::fs::read(&file) else { continue };
            let Ok(mut profile) = serde_json::from_slice::<Value>(&bytes) else { continue };
            let (Some(key), Some(identity)) = (profile["key"].as_str().filter(|k| !k.is_empty()).map(str::to_owned), profile["identity"].as_str().map(str::to_owned)) else { continue };
            let fields = self.key_fields(&identity, &key);
            let Some(entry) = fields["key_vault"].as_str() else { return };
            if direct && let Err(error) = mirror(entry, &key) {
                self.diagnostics.warn(&format!("Direct start's copy of the account key stays in its file: {error:#}"));
                continue
            }
            profile.as_object_mut().map(|o| { o.remove("key"); o.insert("key_vault".into(), entry.into()) });
            // A new owner-only file in place of the old one, which may not be (an older launcher's).
            let next = file.with_extension("json.next");
            let _ = std::fs::remove_file(&next);
            match private_write(&next, profile.to_string().as_bytes(), false).and_then(|()| Ok(std::fs::rename(&next, &file)?)) {
                Ok(()) => self.diagnostics.line(&format!("Moved Singleplayer's account key from {name} into the credential store")),
                Err(error) => self.diagnostics.warn(&format!("Could not rewrite {name} without its key: {error:#}")),
            }
        }
    }
    pub fn data_dir(&self, settings: &Settings) -> PathBuf {
        if settings.data_dir.trim().is_empty() { self.root.join("singleplayer") } else { PathBuf::from(&settings.data_dir) }
    }
    pub fn live(&self, settings: &Settings) -> Option<admin::Live> { admin::running(&self.data_dir(settings)) }
    pub fn status(&self, settings: &Settings) -> anyhow::Result<Value> {
        let live = self.live(settings).context("Server stopped")?;
        let status = api::call(&live, &Op::Status).map_err(anyhow::Error::msg)?;
        if let Some(id) = status["diagnostic_id"].as_str().filter(|id| yo3_tunnel::diagnostics::valid_id(id)) { *self.server_session.lock().unwrap() = Some(id.into()); }
        Ok(status)
    }
    pub fn accounts(&self, settings: &Settings) -> anyhow::Result<Value> {
        let live = self.live(settings).context("Start your server to manage accounts.")?;
        api::call(&live, &Op::Accounts).map_err(anyhow::Error::msg)
    }
    pub fn manage(&self, settings: &Settings, op: Op) -> anyhow::Result<Value> {
        let live = self.live(settings).context("Start your server first.")?;
        api::call(&live, &op).map_err(anyhow::Error::msg)
    }
    /// Unlock all cards on your own server, once for every card (Settings → Singleplayer → Your data): every artwork of every card, topped up to
    /// three copies, for every player. Asked of the running server, else done in its stopped data folder under its lock
    /// (the account it would make at its first start made first, so a new player gets them too). The server's sentence.
    pub fn unlock_cards(&self, settings: &Settings) -> anyhow::Result<String> {
        let op = Op::UnlockCards { player: None };
        let done = match self.live(settings) {
            Some(live) => api::call(&live, &op).map_err(anyhow::Error::msg)?,
            None => {
                let data = self.data_dir(settings);
                let game = PathBuf::from(&settings.game_dir);
                let _lock = yo3_server::store::lock(&data)?;
                yo3_server::store::initialize(&data)?;
                yo3_server::store::interrupt_effects(&data)?;
                let mut directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
                if directory.entries().next().is_none() { directory.add_account(std::slice::from_ref(&settings.player_name)).map_err(anyhow::Error::msg)?; }
                let live = yo3_server::settings::Live::open(Some(&data.join("settings.json")), banlists(&game))?;
                let offline = api::Api::new(Arc::new(Mutex::new(directory)), Arc::new(live), Some(data), game, "offline");
                offline.apply_request(&op, "Desktop", "offline", None, &yo3_server::logins::new_token()).map_err(anyhow::Error::msg)?
            }
        };
        Ok(match done["added"].as_u64().unwrap_or(0) {
            0 => "Every card was already unlocked.".to_owned(),
            added => format!("Every card is unlocked: {added} cards added."),
        })
    }
    /// An operation on your own server: asked of the running server, else done in its stopped data folder (under its lock
    /// when it changes something).
    fn own_server(&self, settings: &Settings, op: &Op) -> anyhow::Result<Value> {
        if let Some(live) = self.live(settings) { return api::call(&live, op).map_err(anyhow::Error::msg) }
        let data = self.data_dir(settings);
        let game = PathBuf::from(&settings.game_dir);
        let _lock = if op.changes() { Some(yo3_server::store::lock(&data)?) } else { None };
        if op.changes() { yo3_server::store::initialize(&data)?; yo3_server::store::interrupt_effects(&data)?; }
        let directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
        let live = yo3_server::settings::Live::open(Some(&data.join("settings.json")), banlists(&game))?;
        let offline = api::Api::new(Arc::new(Mutex::new(directory)), Arc::new(live), op.changes().then(|| data.clone()), game, "offline");
        if op.changes() { offline.apply_request(op, "Desktop", "offline", None, &yo3_server::logins::new_token()) } else { offline.apply(op, "Desktop") }
            .map_err(anyhow::Error::msg)
    }
    /// Your player's decks on your own server (Settings → Singleplayer → Your data → EDOPro decks), by name.
    pub fn decks(&self, settings: &Settings) -> anyhow::Result<Vec<String>> {
        let read = self.own_server(settings, &Op::Decks { player: Some(settings.player_name.clone()) })?;
        Ok(read["decks"].as_array().into_iter().flatten().filter_map(|d| d["name"].as_str().map(str::to_owned)).collect())
    }
    /// An EDOPro deck file (`.ydk`) kept as one of your player's decks on your own server: the server's sentence.
    pub fn import_deck(&self, settings: &Settings, file: &Path) -> anyhow::Result<String> {
        let text = std::fs::read_to_string(file).with_context(|| format!("{}", file.display()))?;
        let name = file.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let done = self.own_server(settings, &Op::ImportDeck { player: Some(settings.player_name.clone()), name, ydk: text })?;
        Ok(done["message"].as_str().unwrap_or("Imported.").to_owned())
    }
    /// One of your player's decks on your own server written as an EDOPro deck file (`.ydk`): the server's sentence.
    pub fn export_deck(&self, settings: &Settings, deck: &str, file: &Path) -> anyhow::Result<String> {
        let done = self.own_server(settings, &Op::ExportDeck { player: Some(settings.player_name.clone()), deck: deck.to_owned() })?;
        std::fs::write(file, done["ydk"].as_str().unwrap_or_default()).with_context(|| format!("{}", file.display()))?;
        Ok(done["message"].as_str().unwrap_or("Exported.").to_owned())
    }
    /// Your account's own banlists on your own server (#77, Settings → Singleplayer → Your data → Banlists): a
    /// `player_banlists::operate` request, through the running server, else on its stopped data folder.
    pub fn player_banlist(&self, settings: &Settings, request: Value) -> anyhow::Result<Value> {
        let op = Op::PlayerBanlist { player: Some(settings.player_name.clone()), request: request.clone() };
        if let Some(live) = self.live(settings) { return api::call(&live, &op).map_err(anyhow::Error::msg) }
        let data = self.data_dir(settings);
        let changes = request["op"].as_str().is_some_and(|op| op == "banlist_import" || op == "banlist_delete");
        let _lock = if changes { Some(yo3_server::store::lock(&data)?) } else { None };
        if changes { yo3_server::store::initialize(&data)?; }
        let directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
        let player = directory.player_id_named(&settings.player_name).ok_or_else(|| anyhow::anyhow!("Make your player in the game first (Select Player), then try again."))?;
        let owner = directory.entries().find(|(_, _, a)| a.players.iter().any(|p| p.player_id == player)).map(|(_, _, a)| a.user_id)
            .ok_or_else(|| anyhow::anyhow!("Make your player in the game first (Select Player), then try again."))?;
        drop(directory);
        yo3_server::player_banlists::offline(&data, Path::new(&settings.game_dir), owner, &settings.player_name, &request).map_err(anyhow::Error::msg)
    }
    /// Your banlists' names on your own server.
    pub fn banlists(&self, settings: &Settings) -> anyhow::Result<Vec<String>> {
        let read = self.player_banlist(settings, serde_json::json!({ "op": "banlists" }))?;
        Ok(read["banlists"].as_array().into_iter().flatten().filter_map(|b| b["name"].as_str().map(str::to_owned)).collect())
    }
    /// An EDOPro banlist file (`lflist.conf`) kept as one of your banlists on your own server: the server's sentence.
    pub fn import_banlist(&self, settings: &Settings, file: &Path) -> anyhow::Result<String> {
        let conf = std::fs::read(file).with_context(|| format!("{}", file.display()))?;
        let done = self.player_banlist(settings, serde_json::json!({ "op": "banlist_import", "conf": String::from_utf8_lossy(&conf) }))?;
        Ok(done["message"].as_str().unwrap_or("Imported.").to_owned())
    }
    /// One of your banlists written as an EDOPro `lflist.conf`.
    pub fn export_banlist(&self, settings: &Settings, name: &str, file: &Path) -> anyhow::Result<String> {
        let done = self.player_banlist(settings, serde_json::json!({ "op": "banlist_export", "name": name }))?;
        std::fs::write(file, done["conf"].as_str().unwrap_or_default()).with_context(|| format!("{}", file.display()))?;
        Ok(done["message"].as_str().unwrap_or("Exported.").to_owned())
    }
    /// Deletes one of your banlists: the server's sentence.
    pub fn delete_banlist(&self, settings: &Settings, name: &str) -> anyhow::Result<String> {
        let done = self.player_banlist(settings, serde_json::json!({ "op": "banlist_delete", "name": name }))?;
        Ok(done["message"].as_str().unwrap_or("Deleted.").to_owned())
    }
    /// What Unlock all cards shows (Settings → Singleplayer → Your data): how many cards (each artwork) your player holds,
    /// starting cards included, out of every card, and how many an unlock would give now (all of them before the first
    /// unlock, then only those an update added). Asked of the running server, else read from its stopped data folder.
    pub fn collection(&self, settings: &Settings) -> anyhow::Result<Collection> {
        let op = Op::Collection { player: Some(settings.player_name.clone()) };
        let read = match self.live(settings) {
            Some(live) => api::call(&live, &op).map_err(anyhow::Error::msg)?,
            None => {
                let data = self.data_dir(settings);
                let directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
                let live = yo3_server::settings::Live::open(Some(&data.join("settings.json")), vec![])?;
                let offline = api::Api::new(Arc::new(Mutex::new(directory)), Arc::new(live), None, PathBuf::from(&settings.game_dir), "offline");
                offline.apply(&op, "Desktop").map_err(anyhow::Error::msg)?
            }
        };
        let n = |k: &str| read[k].as_u64().unwrap_or(0);
        Ok(Collection { owned: n("owned"), total: n("total"), new: n("new"), unlocked: read["unlocked"].as_bool().unwrap_or(false) })
    }
    pub fn start_server(&self, settings: &Settings) -> anyhow::Result<Value> {
        let mut owned = self.server.lock().unwrap();
        self.check_server_resources()?;
        if let Some(live) = self.live(settings) { return api::call(&live, &Op::Status).map_err(anyhow::Error::msg) }
        ensure!(owned.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "Server is still starting.");
        ensure!(settings.host_port != settings.listen_port || self.hosting_live().is_none(), "Stop hosting to play Singleplayer: both use the same port.");
        // No live file for this data folder, yet something answers on the port: another server (a Scripts `run.cmd` or
        // another Revival with its own data) or another program. Starting would only fail to bind, with a vaguer error.
        ensure!(!local_port_answers(settings.listen_port), "TCP port {} is already used by another program, such as a \
            Scripts server (run.cmd / run.sh) or another YO3 Revival with a different data folder. Stop it, or choose \
            another TCP port under Advanced → Hosting → Advanced, which also shows this app's server data folder.",
            settings.listen_port);
        validate_game(Path::new(&settings.game_dir))?;
        let (_, name) = server_address(&settings.host_address)?;
        let bind = network::effective_bind(settings, network::resolve_bind(settings, &network::bindings()?)?);
        ensure!(settings.listen_port > 0, "Port must be between 1 and 65535.");
        let mut cmd = self.server_command(settings, bind)?;
        let data = self.data_dir(settings);
        yo3_server::auth::maintain_identity(&data, vec!["localhost".into(), "127.0.0.1".into(), name.clone()])?;
        yo3_server::store::initialize(&data)?;
        apply_banlist(&data, settings)?;
        let mut directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
        if directory.entries().next().is_none() { directory.add_account(std::slice::from_ref(&settings.player_name)).map_err(anyhow::Error::msg)?; }
        drop(directory);
        detach_server(&mut cmd);
        *self.server_session.lock().unwrap() = None;
        // Your own server: it stops itself once this launcher has gone and nobody plays (#29).
        cmd.arg("--owned").stdin(Stdio::piped());
        let (mut child,errors)=process::spawn(&mut cmd,&self.diagnostics,"Server")?;
        *self.server_owner.lock().unwrap() = child.stdin.take();
        *owned = Some(child);
        self.wait_server_ready(settings, &mut owned, errors, std::time::Duration::from_secs(10))
    }
    fn wait_server_ready(&self, settings: &Settings, owned: &mut Option<Child>, errors: process::Errors, timeout: std::time::Duration) -> anyhow::Result<Value> {
        let started = std::time::Instant::now();
        let result = (|| {
            while started.elapsed() < timeout {
                if let Ok(status) = self.status(settings) { return Ok(status) }
                if let Some(status)=owned.as_mut().unwrap().try_wait()? { anyhow::bail!("The server stopped during startup ({status}).") }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            anyhow::bail!("The server did not become ready within {} seconds.",timeout.as_secs())
        })();
        match result {
            Ok(status) => Ok(status),
            Err(error) => {
                // A failed Start must not leave a detached server running and block every retry.
                if let Some(child) = owned.as_mut() {
                    if child.try_wait()?.is_none()
                        && let Err(stop) = child.kill()
                        && child.try_wait()?.is_none() { return Err(stop).context("Could not stop the server after its failed startup") }
                    child.wait().context("Could not wait for the failed server to stop")?;
                }
                *owned = None;
                anyhow::bail!("{error:#}\n{}\nCorrect the reported problem and try Start again. Export logs if you need help.",errors.finish())
            }
        }
    }
    pub fn server_command(&self, settings: &Settings, bind: std::net::IpAddr) -> anyhow::Result<Command> {
        // The banlist is a live setting in the server's data (#24, `apply_banlist`), not a flag.
        let extra = server_options::arguments(settings)?;
        let port = settings.listen_port.to_string();
        let mut cmd = Command::new(&self.runtime.server);
        cmd.args(["--bind", &bind.to_string(), "--auth-port", &port])
            .arg("--data-dir").arg(self.data_dir(settings)).arg("--game-dir").arg(&settings.game_dir).arg("--log-dir").arg(self.root.join("logs"))
            .arg("--diagnostics-dir").arg(self.root.join("logs/sessions")).arg("--diagnostic-parent").arg(self.diagnostics.id())
            .env("YO3_ENGINE_DIR", &self.runtime.engine).env("YO3_ENGINE", &self.runtime.engine_program).env("NO_COLOR", "1").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if !settings.server_options.contains_key("http-port") { cmd.args(["--http-port", "0"]); }
        if !settings.server_options.contains_key("no-packet-log") { cmd.arg("--no-packet-log"); }
        // Settings → Advanced → Detailed logs: the server's own tracing filter, from its next start.
        if options::detailed(settings) { cmd.env("RUST_LOG", "debug"); }
        cmd.args(extra);
        Ok(cmd)
    }
    /// Stops your server when it runs. A server that stops itself between the look and the stop (an owned server ends once
    /// its launcher's game is gone, as after Cancel) is what was asked for, not a failure.
    pub fn stop_server_if_running(&self, settings: &Settings) -> anyhow::Result<()> {
        if self.live(settings).is_none() { return Ok(()) }
        match self.stop_server(settings) { Err(_) if self.live(settings).is_none() => Ok(()), result => result.map(|_| ()) }
    }
    pub fn stop_server(&self, settings: &Settings) -> anyhow::Result<Value> {
        let mut owned = self.server.lock().unwrap();
        let live=self.live(settings).context("Start your server first.")?;
        // Keep the captured endpoint: another app can replace the profile's live file during shutdown.
        let reply=api::stop(&live).map_err(anyhow::Error::msg)?;
        // The management response acknowledges the request before ports and data files are released.
        // Serialize Start with the owned child's graceful exit and original endpoint closure. An old child
        // handle may already have exited while this app attached to a replacement server, so check both.
        // Never terminate an unowned process or send another shutdown to a replacement endpoint.
        // This launcher no longer owns a server that is stopping.
        *self.server_owner.lock().unwrap() = None;
        let address=std::net::SocketAddr::from(([127,0,0,1],live.port));
        let started=std::time::Instant::now();
        let timeout=std::time::Duration::from_secs(10);
        while started.elapsed()<timeout {
            if let Some(child)=owned.as_mut() && child.try_wait()?.is_some() {*owned=None;}
            if owned.is_none() && server_endpoint_closed(&address,timeout.saturating_sub(started.elapsed())) {return Ok(reply)}
            std::thread::sleep(std::time::Duration::from_millis(100).min(timeout.saturating_sub(started.elapsed())));
        }
        anyhow::bail!("The server is still stopping. Wait a few seconds before trying Start again. Export logs if it does not stop.")
    }
    pub fn update_certificate(&self, settings: &Settings) -> anyhow::Result<()> {
        let _owned = self.server.lock().unwrap();
        ensure!(self.live(settings).is_none(), "Stop your server before updating its certificate.");
        let (_, name) = server_address(&settings.host_address)?;
        let names = vec!["localhost".into(), "127.0.0.1".into(), name];
        let data = self.data_dir(settings);
        if data.join("server.key").exists() { yo3_server::auth::reissue_certificate(&data, names)?; }
        else { yo3_server::auth::initialize(&data, names)?; }
        Ok(())
    }
    fn account_connection(&self, settings: &Settings, who: &str) -> anyhow::Result<KeyConnection> {
        let key = self.manage(settings, Op::Key { who: who.into() })?["key"].as_str().context("Missing account key")?.to_string();
        let (address, server_name) = network::advertised(settings)?;
        Ok(KeyConnection { address, server_name,
            certificate: std::fs::read_to_string(settings.server_options.get("auth-cert").map(PathBuf::from)
                .unwrap_or_else(|| self.data_dir(settings).join("server.crt")))?, identity: None, key })
    }
    pub fn game_running(&self) -> bool {
        self.game.lock().unwrap().as_mut().is_some_and(|child| child.try_wait().ok().flatten().is_none())
    }
    pub fn game_ready(&self) -> bool { self.game_ready.lock().unwrap().as_ref().is_some_and(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() <= 65536)) }
    /// The server pins another card profile than the running game loaded (the helper's marker beside the ready file,
    /// `yo3_launcher::decks`): the game shows the old cards until it is restarted.
    pub fn game_profile_stale(&self) -> bool {
        self.game_ready.lock().unwrap().as_ref().is_some_and(|p| std::fs::symlink_metadata(yo3_launcher::decks::stale_file(p)).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink()))
    }
    fn clear_game_ready(&self) {
        if let Some(path) = self.game_ready.lock().unwrap().take() { let _ = std::fs::remove_file(yo3_launcher::decks::stale_file(&path)); let _ = std::fs::remove_file(foreign::survey_file(&path)); let _ = std::fs::remove_file(path); }
    }
    /// Whether the next start is on a server that asks for a clean game (#69).
    pub fn require_clean_game(&self, on: bool) { self.clean_game.store(on, std::sync::atomic::Ordering::SeqCst) }
    /// The next start goes past another program's file or provider (the player's Continue anyway, #68).
    pub fn accept_foreign_once(&self) { self.accept_foreign.store(true, std::sync::atomic::Ordering::SeqCst) }
    /// Why the last start stopped before the game ran, if another program's file or provider (or a redirected network
    /// function) was the reason (#68); said once.
    pub fn take_held(&self) -> Option<foreign::Held> { self.held.lock().unwrap().take() }
    pub fn game_failure(&self) -> Option<String> {
        let mut game = self.game.lock().unwrap();
        let status = game.as_mut()?.try_wait().ok().flatten()?;
        let ready=self.game_ready();
        // What the helper found inside the game (#68): when that stopped the start, it is the whole story.
        let held = self.game_ready.lock().unwrap().as_deref().and_then(|path| foreign::Held::read(&foreign::survey_file(path)));
        *game = None; self.clear_game_ready();
        if let Some(held) = held {
            self.diagnostics.warn(&format!("The game was not started: {} finding(s) inside the game{}", held.findings.len(), if held.blocked { " (blocked)" } else { "" }));
            let _ = self.game_errors.lock().unwrap().take().map(|e| e.finish());
            *self.held.lock().unwrap() = Some(held);
            return None
        }
        let detail=self.game_errors.lock().unwrap().take().map(|e|e.finish()).unwrap_or_default();
        (!status.success()).then(|| format!("{} ({status}).\n{detail}\nTry Play again; it rechecks the required files. If it still fails, export logs for help.",if ready{"The game stopped unexpectedly"}else{"The game could not start"}))
    }
    pub fn local_connection(&self, settings: &Settings, who: &str) -> anyhow::Result<KeyConnection> {
        let account = if who.is_empty() { self.accounts(settings)?[0]["players"][0]["name"].as_str().unwrap_or("Duelist").to_string() } else { who.to_string() };
        let mut server = self.account_connection(settings, &account)?;
        let status = self.status(settings)?;
        let mut address: std::net::SocketAddr = status["listen"].as_str().context("Server does not report its listening address; update the server.")?.parse()?;
        if address.ip().is_unspecified() { address.set_ip(if address.is_ipv6() { "::1".parse()? } else { "127.0.0.1".parse()? }); }
        server.address = address.to_string();
        Ok(server)
    }
    /// Collects absent known files without stopping at the first validation error or reading file contents.
    pub fn missing_required_files(&self, settings: &Settings, diagnostic: Option<&str>) -> Vec<PathBuf> {
        let mut paths=vec![self.runtime.launcher.clone(),self.runtime.injector.clone(),self.runtime.dll.clone()];
        let bundle=self.runtime.injector.parent().unwrap_or(Path::new("."));let manifest=bundle.join("bundle.json");
        let packaged=manifest.exists() || !yo3_tunnel::BUILD_VERSION.ends_with("-dev");
        if packaged {paths.push(manifest);}
        let game=(!settings.game_dir.trim().is_empty()).then(||Path::new(&settings.game_dir));
        if let Some(game)=game {paths.extend(["yo3.exe","yo3.idx","package/data002.ypk"].map(|name|game.join(name)));}
        paths.push(self.runtime.server.clone());
        paths.extend(yo3_server::engine::missing_files(&self.runtime.engine,&self.runtime.engine_program));
        if let Some(diagnostic)=diagnostic {
            let bounded=diagnostic.chars().take(16*1024).collect::<String>();
            for line in bounded.lines() {
                let quoted=line.split_once("Cannot find module ").map(|(_,path)|path)
                    .or_else(||(line.contains("ENOENT") || line.contains("os error 2")).then(||line.split_once("open ").map(|(_,path)|path)).flatten());
                let Some(quoted)=quoted else {continue};
                let Some(quote @ ('\''|'"'))=quoted.chars().next() else {continue};
                let Some((path,_))=quoted[1..].split_once(quote) else {continue};
                let path=Path::new(path);if path.is_absolute() {paths.push(path.to_path_buf());}
            }
        }
        paths.retain(|path|matches!(std::fs::metadata(path),Err(error) if error.kind()==std::io::ErrorKind::NotFound));
        paths.sort();paths.dedup();paths
    }
    pub fn readiness(&self, settings: &Settings) -> Readiness {
        if let Err(error) = validate_game(Path::new(&settings.game_dir)) { return Readiness::GameProblem(format!("{error:#}")) }
        let components=self.check_components(settings).and_then(|()| self.check_server_resources());
        match components { Ok(()) => Readiness::Ready, Err(error) => Readiness::ComponentProblem(format!("{error:#}")) }
    }
    pub fn check_game(&self, settings: &Settings) -> anyhow::Result<()> {
        validate_game(Path::new(&settings.game_dir))?; self.check_components(settings)?;
        self.diagnostics.line("Checking game runtime prerequisites without starting the game");
        let prefix = (!settings.wine_prefix.is_empty()).then(|| Path::new(&settings.wine_prefix));
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(
            yo3_launcher::runtime::check(&Path::new(&settings.game_dir).join("yo3.exe"), &self.runtime.injector, prefix))
    }
    fn check_server_resources(&self) -> anyhow::Result<()> {
        use std::io::Read;
        ensure!(self.runtime.server.is_file(), "Required server file is missing: {}. Extract the complete application bundle.",self.runtime.server.display());
        let manifest=self.runtime.injector.parent().context("Application folder missing")?.join("bundle.json");
        if manifest.exists() {
            let mut bytes=Vec::new();
            std::fs::File::open(&manifest).with_context(||format!("Could not read release manifest: {}",manifest.display()))?
                .take(65537).read_to_end(&mut bytes).with_context(||format!("Could not read release manifest: {}",manifest.display()))?;
            ensure!(bytes.len()<=65536,"Release manifest is too large: {}. Restore the complete bundle.",manifest.display());
            let value:Value=serde_json::from_slice(&bytes).with_context(||format!("Invalid release manifest: {}",manifest.display()))?;
            for (role,path) in [("server",&self.runtime.server),("engine",&self.runtime.engine_program)] {
                ensure!(path.is_file(),"Required {role} file is missing: {}. Restore the complete matching bundle.",path.display());
                let expected=value["components"][role]["sha256"].as_str().filter(|hash|hash.len()==64 && hash.bytes().all(|byte|byte.is_ascii_hexdigit()))
                    .with_context(||format!("Release manifest is missing the {role} SHA-256: {}",manifest.display()))?;
                let mut file=std::fs::File::open(path).with_context(||format!("Could not read required {role} file: {}",path.display()))?.take(256*1024*1024+1);
                let mut digest=ring::digest::Context::new(&ring::digest::SHA256);let mut buffer=[0;16384];let mut size=0;
                loop {
                    let read=file.read(&mut buffer).with_context(||format!("Could not read required {role} file: {}",path.display()))?;
                    if read==0{break}size+=read;
                    ensure!(size<=256*1024*1024,"Required {role} file is unexpectedly large: {}",path.display());digest.update(&buffer[..read]);
                }
                let actual=digest.finish().as_ref().iter().map(|byte|format!("{byte:02x}")).collect::<String>();
                ensure!(actual.eq_ignore_ascii_case(expected),"{} differs from the release manifest. Restore the complete matching bundle.",path.display());
            }
        } else {
            ensure!(yo3_tunnel::BUILD_VERSION.ends_with("-dev"),"Release manifest is missing: {}. Restore the complete matching bundle.",manifest.display());
        }
        if let Some(problem)=yo3_server::engine::problem(Some(&self.runtime.engine),&self.runtime.engine_program) {
            anyhow::bail!("The rules engine is incomplete: {problem}. Extract the complete application bundle again.");
        }
        self.check_engine_start(std::time::Duration::from_secs(10))
    }
    fn check_engine_start(&self,timeout:std::time::Duration)->anyhow::Result<()> {
        // With no requests (stdin closed), the engine loads its card data and exits.
        let program=&self.runtime.engine_program;
        let mut cmd=Command::new(program);cmd.arg(&self.runtime.engine).stdin(Stdio::null()).stdout(Stdio::null());hide_console(&mut cmd);
        self.diagnostics.line("Checking that the rules engine can load its card data without starting a duel");
        let (mut child,errors)=process::spawn(&mut cmd,&self.diagnostics,"Engine check")?;
        let started=std::time::Instant::now();
        let result=(||->anyhow::Result<std::process::ExitStatus>{
            loop {
                if let Some(status)=child.try_wait().context("Could not inspect the engine check process")?{return Ok(status)}
                ensure!(started.elapsed()<timeout,"The rules engine startup check timed out after {:.1} seconds",timeout.as_secs_f32());
                std::thread::sleep(std::time::Duration::from_millis(20).min(timeout.saturating_sub(started.elapsed())));
            }
        })();
        if result.is_err(){let _=child.kill();}
        let waited=child.wait();let detail=errors.finish();
        let cause=detail.lines().find(|line|line.starts_with("yo3-engine")).unwrap_or_else(||detail.lines().find(|line|!line.trim().is_empty()).unwrap_or("No engine diagnostic was returned."));
        match result {
            Ok(status) if status.success()=>{waited.context("Could not reap the completed engine check")?;Ok(())}
            Ok(status)=>anyhow::bail!("The rules engine could not start: {cause}\nEngine: {}\nData: {}\nExit status: {status}\n{detail}\nRestore the complete matching bundle, then Retry check.",program.display(),self.runtime.engine.display()),
            Err(error)=>anyhow::bail!("{error:#}. Engine: {}\n{cause}\n{detail}\nRestore the complete matching bundle or export logs if this repeats.",program.display()),
        }
    }
    pub fn validate_setup(&self, draft: &Settings) -> anyhow::Result<Settings> {
        let saved=LauncherDb::open(&self.root)?.settings()?;
        let next=preferences::Section::Game.merge(&saved,draft);
        self.verify_setup(&next)?;
        self.save_preferences(preferences::Section::Game,&next)
    }
    /// Checks a captured setup without saving preferences, refreshing direct launch, or starting play/hosting.
    pub fn verify_setup(&self, settings: &Settings) -> anyhow::Result<()> {
        self.diagnostics.line("Validating original game files and required connection components");
        self.check_game(settings)?;
        self.check_server_resources()?; server_options::arguments(settings)?;
        Ok(())
    }
    fn check_components(&self, settings: &Settings) -> anyhow::Result<()> {
        ensure!(self.runtime.launcher.is_file(), "Required connection file is missing: {}. Restore the complete application bundle.",self.runtime.launcher.display());
        yo3_launcher::components::preflight(&Path::new(&settings.game_dir).join("yo3.exe"), &self.runtime.injector, &self.runtime.dll)?;
        let manifest = self.runtime.injector.parent().context("Application folder missing")?.join("bundle.json");
        if manifest.is_file() {
            let value: Value = serde_json::from_slice(&std::fs::read(manifest)?)?;
            ensure!(std::fs::metadata(&self.runtime.launcher)?.len() <= 64 * 1024 * 1024, "Unexpected connection launcher size");
            ensure!(value["components"]["launcher"]["sha256"].as_str() == Some(yo3_launcher::components::hash(&std::fs::read(&self.runtime.launcher)?).as_str()), "The connection launcher differs from its release manifest. Restore the complete application bundle.");
        }
        Ok(())
    }
    /// Singleplayer's Play: starts your own server if it isn't running and plays on it with your local account's key.
    pub fn play(&self, settings: &Settings, who: &str) -> anyhow::Result<()> {
        let mut game = self.game.lock().unwrap();
        self.clean_game.store(false, std::sync::atomic::Ordering::SeqCst);
        ensure!(game.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "The game is already running.");
        self.check_game(settings)?;
        self.start_server(settings)?;
        let server = self.local_connection(settings, who)?;
        self.note_main(if who.is_empty() { "Singleplayer" } else { who }, "Singleplayer", "singleplayer");
        server.validate()?;
        self.sync_direct_target(settings, direct::Target::Local)?;
        let profile = self.write_profile(&server)?;
        self.launch(settings, profile, game)
    }
    /// Plays on a Multiplayer server with a session the launcher signed in for (#30): the helper uses it for the game's
    /// connections, through the server's pinned identity. The private profile holds the session, never a password.
    /// Whether direct start needs permission once (a protected game folder), as Singleplayer's Play says.
    /// `clean_game`: what the server's status last said (#69); it is asked once more here, so a host who just turned
    /// the policy on is heard.
    pub fn play_signed_in(&self, settings: &Settings, server: &multiplayer::Server, user: &str, session: &yo3_tunnel::Token, clean_game: bool) -> anyhow::Result<bool> {
        let game = self.game.lock().unwrap();
        self.note_main(user, &server.name, server.key());
        let clean_game = match tokio::runtime::Builder::new_current_thread().enable_all().build().map(|rt| rt.block_on(multiplayer::status(server))) {
            Ok(multiplayer::Status::Online { clean_game, .. }) => clean_game,
            _ => clean_game,
        };
        self.clean_game.store(clean_game, std::sync::atomic::Ordering::SeqCst);
        self.check_game(settings)?;
        let file = self.root.join("profiles").join("multiplayer.json");
        let session: String = session.iter().map(|b| format!("{b:02x}")).collect();
        // The vault entry lets the game's Delete Login Data forget this device's saved sign-in (#33).
        private_write(&file, serde_json::json!({ "server": server.address, "server_name": server.server_name, "identity": server.key(),
            "session": session, "vault": multiplayer::vault_account(server) }).to_string().as_bytes(), true)?;
        // Opening yo3.exe directly plays this server next, with the saved sign-in if the device keeps one (#33).
        let mut needs_permission = false;
        if settings.direct_start {
            match self.auto_direct_target(settings, direct::Target::SignedIn(server.clone())) {
                direct::AutoDirect::Ready => self.mirror_sign_in(settings, server),
                direct::AutoDirect::NeedsPermission => needs_permission = true,
                direct::AutoDirect::Skipped(why) => self.diagnostics.warn(&format!("Direct start not set up: {why}")),
            }
        }
        yo3_launcher::Profile::load(&file)?;
        self.launch(settings, file, game)?;
        Ok(needs_permission)
    }
    /// Starts the game through the helper with the private profile `profile`.
    fn launch(&self, settings: &Settings, profile: PathBuf, mut game: std::sync::MutexGuard<'_, Option<Child>>) -> anyhow::Result<()> {
        ensure!(game.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "The game is already running.");
        ensure!(self.runtime.launcher.is_file() && self.runtime.dll.is_file() && self.runtime.injector.is_file(), "Connection files are missing. Extract the complete application bundle.");
        let ready = self.root.join("profiles").join(format!("game-ready-{}-{}.json", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()));
        let mut cmd = self.launch_command(settings, &profile, &ready)?;
        self.add_client_data(&mut cmd, settings);
        let _ = std::fs::remove_file(foreign::survey_file(&ready));
        let (child,errors)=process::spawn(&mut cmd,&self.diagnostics,"Game helper")?;
        self.accept_foreign.store(false, std::sync::atomic::Ordering::SeqCst);
        *self.game_errors.lock().unwrap()=Some(errors);
        self.clear_game_ready(); *self.game_ready.lock().unwrap() = Some(ready);
        *game = Some(child);
        Ok(())
    }
    /// Tells the helper where the game's Documents folder is, so it can place the player's local decks as the server's card
    /// profile says before the game is released (`yo3_launcher::decks`, issue #60). A folder that can't be found leaves the
    /// decks as they are (the server places them again for every duel). Asked on a thread of its own: under Wine it runs a
    /// small Wine program, and the caller may be inside the async runtime.
    pub(crate) fn add_client_data(&self, cmd: &mut Command, settings: &Settings) {
        match std::thread::scope(|scope| scope.spawn(|| game_data_platform::data_directory(self, settings)).join()) {
            Ok(Ok(folder)) => { cmd.arg("--client-data").arg(folder); }
            Ok(Err(error)) => self.diagnostics.warn(&format!("Local decks are not adapted to the card profile: {error}")),
            Err(_) => {}
        }
    }
    /// Problem reports' switch, where every helper reads it (`yo3_launcher::problems::OFF_FILE` beside the profiles):
    /// the one Play starts, and direct start's, which the game's own loader starts without the launcher.
    pub fn sync_problem_reports(&self, settings: &Settings) {
        let off = self.root.join("profiles").join(yo3_launcher::problems::OFF_FILE);
        let result = if settings.problem_reports { std::fs::remove_file(&off).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }) }
            else { std::fs::create_dir_all(self.root.join("profiles")).and_then(|()| std::fs::write(&off, b"Problem reports are off (Settings, Advanced).\n")) };
        if let Err(error) = result { self.diagnostics.error(&format!("Problem reports' switch could not be saved: {error}")) }
    }
    /// The helper's command for a Play: the settings it reads (screen size, Wine) are taken at each Play.
    pub fn launch_command(&self, settings: &Settings, profile: &Path, ready: &Path) -> anyhow::Result<Command> {
        self.launch_command_with(settings, profile, ready, None)
    }
    /// [`Backend::launch_command`] for an extra game (#64): its own Wine prefix and game data folder.
    pub fn launch_command_with(&self, settings: &Settings, profile: &Path, ready: &Path, extra: Option<&games::ExtraRun>) -> anyhow::Result<Command> {
        self.sync_problem_reports(settings);
        let mut cmd = Command::new(&self.runtime.launcher);
        if !settings.problem_reports { cmd.arg("--no-problem-reports"); }
        cmd.arg("--control-stdin").arg("--profile").arg(profile).arg("--game-dir").arg(&settings.game_dir).arg("--injector").arg(&self.runtime.injector).arg("--dll").arg(&self.runtime.dll);
        match extra.and_then(|e| e.prefix.as_ref()) {
            Some(prefix) => { cmd.arg("--wine-prefix").arg(prefix); }
            None if !settings.wine_prefix.is_empty() => { cmd.arg("--wine-prefix").arg(&settings.wine_prefix); }
            None => {}
        }
        if let Some(extra) = extra { cmd.arg("--data-key").arg(&extra.data_key); }
        if !settings.wine_binary.is_empty() { cmd.env(yo3_launcher::runtime::WINE_ENV, &settings.wine_binary); }
        let resolution = settings.resolution.parse::<yo3_launcher::resolution::Resolution>().map_err(anyhow::Error::msg)?;
        if resolution != yo3_launcher::resolution::Resolution::Off { cmd.arg("--resolution").arg(resolution.to_string()); }
        cmd.arg("--anime-art").arg(yo3_launcher::art::folder_in(&self.root));
        cmd.arg("--diagnostics-dir").arg(self.root.join("logs/sessions")).arg("--diagnostic-parent").arg(self.diagnostics.id());
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        cmd.arg("--ready-file").arg(ready);
        // What else runs inside the game (#68): the helper's report of it is read when a start stops.
        cmd.arg("--survey-file").arg(foreign::survey_file(ready));
        if extra.is_none() && self.accept_foreign.load(std::sync::atomic::Ordering::SeqCst) { cmd.arg("--accept-foreign"); }
        if extra.map_or_else(|| self.clean_game.load(std::sync::atomic::Ordering::SeqCst), |e| e.clean_game) { cmd.arg("--clean-game"); }
        hide_console(&mut cmd);
        Ok(cmd)
    }
    pub fn write_profile(&self, server: &KeyConnection) -> anyhow::Result<PathBuf> {
        let stem = "connection-0";
        private_write(&self.root.join("profiles").join(format!("{stem}.crt")), server.certificate.as_bytes(), true)?;
        let file = self.root.join("profiles").join(format!("{stem}.json"));
        let identity = server.identity.clone().map(Ok).unwrap_or_else(|| yo3_tunnel::identity_from_pem(server.certificate.as_bytes()))?;
        let mut body = serde_json::json!({"server":server.address,"server_name":server.server_name,"certificate":format!("{stem}.crt"), "identity":identity});
        // The key goes in the credential store; the profile names its entry (#23).
        if let (Some(body), Value::Object(fields)) = (body.as_object_mut(), self.key_fields(&identity, &server.key)) { body.extend(fields) }
        private_write(&file, body.to_string().as_bytes(), true)?;
        // Parse through the existing TLS implementation before starting any game process.
        yo3_launcher::Profile::read_with(&file, self.vault())?;
        Ok(file)
    }
    pub fn discover(input: &str) -> anyhow::Result<yo3_tunnel::ObservedServer> {
        let (address, name) = server_address(input)?;
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(yo3_tunnel::discover(&address, &name))
    }
    pub fn stop_game(&self) -> anyhow::Result<()> {
        use std::io::Write;
        let mut game = self.game.lock().unwrap();
        if let Some(child) = game.as_mut() {
            if child.try_wait()?.is_some() { *game = None; self.clear_game_ready(); return Ok(()) }
            if let Some(input) = &mut child.stdin
                && let Err(error) = input.write_all(b"stop\n")
                && error.kind() != std::io::ErrorKind::BrokenPipe { return Err(error.into()) }
        }
        Ok(())
    }
    pub fn logs(&self, server: bool) -> String {
        let root = self.root.join("logs/sessions"); let ids = self.current_sessions();
        let records=match yo3_tunnel::diagnostics::records(&root) {Ok(records)=>records,Err(error)=>return format!("Could not read logs: {error}. Check the data folder permissions, then retry. Copy the on-screen error if export is unavailable.\n")};
        let selected: Vec<_> = records.iter()
            .filter(|r| r.name.ends_with(".log") && ids.contains(&r.id) && r.id.starts_with(if server { "server-" } else { "game-" })).cloned().collect();
        let fallback: Vec<_> = if server { vec![] } else { records.into_iter().filter(|r| r.name.ends_with(".log") && ids.contains(&r.id) && (r.id.starts_with("app-") || r.id.starts_with("direct-"))).collect() };
        let mut result = String::new();
        use std::io::{Read, Seek, SeekFrom};
        for record in fallback.into_iter().chain(selected).rev() {
            let mut f=match std::fs::File::open(root.join(&record.name)) {Ok(f)=>f,Err(error)=>{result.push_str(&format!("Could not open log {}: {error}. Try export or Copy error.\n",record.name));continue}};
            let len = f.metadata().map(|m| m.len()).unwrap_or(0); let remaining = (96 * 1024usize).saturating_sub(result.len()); if remaining == 0 { break }
            let mut bytes = Vec::new();
            if let Err(error)=f.seek(SeekFrom::Start(len.saturating_sub(remaining as u64))).and_then(|_|f.take(remaining as u64).read_to_end(&mut bytes)) {result.push_str(&format!("Could not read log {}: {error}. Try export or Copy error.\n",record.name));continue}
            result = format!("{}{}", String::from_utf8_lossy(&bytes), result);
        }
        result
    }
    pub fn current_sessions(&self) -> Vec<String> {
        let mut ids = vec![self.diagnostics.id()];
        let server = self.server_session.lock().unwrap().clone().or_else(|| {
            let root = self.root.join("logs/sessions");
            yo3_tunnel::diagnostics::records(&root).ok()?.into_iter().rev().filter(|r| r.name.ends_with(".json")).find_map(|r| {
                let meta: yo3_tunnel::diagnostics::Metadata = serde_json::from_slice(&std::fs::read(root.join(r.name)).ok()?).ok()?;
                (meta.kind == "server" && meta.parent.as_deref() == Some(self.diagnostics.id().as_str())).then_some(meta.id)
            })
        });
        if let Some(id) = server { ids.push(id); }
        if let Ok(id) = std::fs::read_to_string(self.root.join("logs/sessions/latest-game"))
            && yo3_tunnel::diagnostics::valid_id(&id) { ids.push(id); }
        ids
    }
    pub fn report(&self, all: bool) -> anyhow::Result<reports::Report> {
        let current = self.current_sessions();
        reports::Report::prepare(&self.root.join("logs/sessions"), if all { None } else { Some(&current) })
    }
}

/// Writes the saved Banlist into the server's data as its `login-regulation` setting (#24), before the server starts:
/// "" is the game's unlimited list (the server's default, Season 16, when the game has none).
pub fn apply_banlist(data: &Path, settings: &Settings) -> anyhow::Result<()> {
    let lists = banlists(Path::new(&settings.game_dir));
    let unlimited = unlimited(&lists);
    let live = yo3_server::settings::Live::open(Some(&data.join("settings.json")), lists)?;
    let done = match (settings.banlist.is_empty(), unlimited) {
        (true, Some(id)) => live.set("login-regulation", &id.to_string()),
        (true, None) => live.reset("login-regulation"),
        (false, _) => live.set("login-regulation", &settings.banlist),
    };
    done.map(|_| ()).map_err(|e| anyhow::anyhow!("The Banlist {:?} can't be used: {e}", settings.banlist))
}

/// Singleplayer's default banlist (the setting left empty): the game's own unlimited list, so a new player can build any
/// deck. The server's default (Season 16) only when the game has no such list. Hosting keeps the server's default.
pub const DEFAULT_BANLIST: &str = "Unlimited (default)";

/// The game's unlimited banlist among `lists`, by its name.
pub fn unlimited(lists: &[(u32, String)]) -> Option<u32> {
    lists.iter().find(|(_, name)| name.trim().eq_ignore_ascii_case("unlimited")).map(|(id, _)| *id)
}

/// The game's banlists (regulation id, name from its text table) for Hosting's Banlist choice; empty without game data.
pub fn banlists(game_dir: &Path) -> Vec<(u32, String)> {
    let Ok(data) = yo3_data::GameData::load(game_dir) else { return Vec::new() };
    data.regulations.iter().filter_map(|r| Some((r.id, data.text(r.name_id)?.to_string()))).filter(|(_, name)| !name.is_empty()).collect()
}
pub fn validate_game(path: &Path) -> anyhow::Result<()> {
    ensure!(path.join("yo3.exe").is_file(), "Choose the installed game folder containing yo3.exe.");
    yo3_launcher::components::inspect_game(&path.join("yo3.exe"))?;
    ensure!(path.join("yo3.idx").is_file(), "The game patch index is missing; choose a complete installation.");
    yo3_data::GameData::load(path).context("The game data is incomplete or unsupported.")?;
    Ok(())
}
fn hide_console(cmd: &mut Command) {
    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
    #[cfg(not(windows))] let _ = cmd;
}
fn detach_server(cmd: &mut Command) {
    hide_console(cmd);
    // A terminal/desktop session may clean up its process group when the manager exits.
    // Hosting owns only file handles and must survive that cleanup.
    #[cfg(unix)] { use std::os::unix::process::CommandExt; cmd.process_group(0); }
}

/// Whether anything accepts connections on this computer's loopback at `port` (IPv4 or IPv6).
fn local_port_answers(port: u16) -> bool {
    let budget = std::time::Duration::from_millis(300);
    [std::net::IpAddr::from([127, 0, 0, 1]), std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST)].into_iter()
        .any(|ip| std::net::TcpStream::connect_timeout(&std::net::SocketAddr::new(ip, port), budget).is_ok())
}
fn server_endpoint_closed(address: &std::net::SocketAddr, budget: std::time::Duration) -> bool {
    if budget.is_zero() {return false}
    // Windows can take about two seconds to report loopback refusal. A shorter connect timeout reports
    // TimedOut even after the listener closes; that is unknown state, never proof that a server stopped.
    matches!(std::net::TcpStream::connect_timeout(address,budget.min(std::time::Duration::from_secs(3))),
        Err(error) if matches!(error.kind(),std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopped_loopback_endpoint_is_detected_with_a_windows_sized_probe_budget() {
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();
        drop(listener);
        assert!(!server_endpoint_closed(&address,std::time::Duration::ZERO),"an expired budget must not be mistaken for endpoint closure");
        assert!(server_endpoint_closed(&address,std::time::Duration::from_secs(3)),"allow Windows enough time to distinguish refusal from a connect timeout");
    }
    #[test]
    #[cfg(unix)]
    fn startup_timeout_reaps_the_owned_server_so_start_can_retry() {
        let root=std::env::temp_dir().join(format!("yo3-server-timeout-{}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap();
        let mut cmd=Command::new("sh"); cmd.args(["-c","printf 'Still loading server resources\\n' >&2; exec sleep 30"]);
        let (child,errors)=process::spawn(&mut cmd,&backend.diagnostics,"Server").unwrap();
        let pid=child.id().to_string(); let mut owned=Some(child);
        let error=backend.wait_server_ready(&Settings::default(),&mut owned,errors,std::time::Duration::from_millis(1)).unwrap_err();
        let released=owned.is_none();
        let still_running=Command::new("kill").args(["-0",&pid]).stderr(Stdio::null()).status().unwrap().success();
        // Clean up on the failing version too: the regression must never leave a server behind.
        if let Some(mut child)=owned.take() { let _=child.kill(); let _=child.wait(); }
        else if still_running { let _=Command::new("kill").args(["-KILL",&pid]).status(); }
        drop(backend); std::fs::remove_dir_all(root).unwrap();
        assert!(released,"a timed-out startup must release its owned child, rather than block every later Start: {error:#}");
        assert!(!still_running,"dropping the process handle must not leave the detached server running");
        assert!(format!("{error:#}").contains("Still loading server resources"),"preserve the server's diagnostic details: {error:#}");
        assert!(format!("{error:#}").contains("try Start again"),"give the player an action they can take: {error:#}");
    }
    fn mock_run_bound_stop(listener:&std::net::TcpListener) {
        use std::io::{Read,Write};
        let run_id="a".repeat(64);
        for op in ["status","shutdown"] {
            let (mut stream,_)=listener.accept().unwrap();stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let mut header=Vec::new();let mut byte=[0];
            while !header.ends_with(b"\r\n\r\n") {stream.read_exact(&mut byte).unwrap();header.push(byte[0]);}
            let header=String::from_utf8(header).unwrap();
            let length:usize=header.lines().find_map(|line|line.strip_prefix("Content-Length: ")).unwrap().parse().unwrap();
            let mut body=vec![0;length];stream.read_exact(&mut body).unwrap();let body:serde_json::Value=serde_json::from_slice(&body).unwrap();
            assert_eq!(body["op"],op);
            let result=if op=="status" {serde_json::json!({"run_id":run_id})} else {assert_eq!(body["run_id"],run_id);serde_json::json!({"stopping":true})};
            let reply=serde_json::json!({"ok":true,"result":result}).to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",reply.len(),reply).unwrap();
        }
    }
    #[test]
    #[cfg(unix)]
    fn stopping_an_owned_server_waits_until_it_has_exited_before_start_can_retry() {
        use std::io::Write;
        let root=std::env::temp_dir().join(format!("yo3-server-shutdown-{}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap(); let settings=Settings::default();
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        admin::write_live(&backend.data_dir(&settings),listener.local_addr().unwrap().port()).unwrap();
        let mut child=Command::new("sh").args(["-c","read finish; sleep 0.15"]).stdin(Stdio::piped()).spawn().unwrap();
        let mut finish=child.stdin.take().unwrap(); *backend.server.lock().unwrap()=Some(child);
        let manager=std::thread::spawn(move || {
            // admin::running makes a TCP liveness probe before the actual management request.
            drop(listener.accept().unwrap().0);
            mock_run_bound_stop(&listener); finish.write_all(b"done\n").unwrap();
        });
        let reply=backend.stop_server(&settings).unwrap();
        let released=backend.server.lock().unwrap().is_none();
        manager.join().unwrap();
        if let Some(mut child)=backend.server.lock().unwrap().take() { child.wait().unwrap(); }
        drop(backend); std::fs::remove_dir_all(root).unwrap();
        assert_eq!(reply["stopping"],true);
        assert!(released,"a shutdown acknowledgement must not enable Start while the owned server is still stopping");
    }
    #[test]
    fn stopping_a_server_that_is_already_gone_is_not_an_error() {
        // Windows 11 (2026-11-20): Cancel killed the game, the owned server saw its launcher's game gone and stopped itself,
        // and Cancel's own stop then failed with "Start your server first.", shown as a red notice.
        let root=std::env::temp_dir().join(format!("yo3-server-gone-{}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap(); let settings=Settings::default();
        assert!(backend.stop_server(&settings).is_err(),"stopping a stopped server on purpose still says so");
        assert!(backend.stop_server_if_running(&settings).is_ok(),"a clean-up that finds the server gone has nothing to report");
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stopping_an_attached_server_waits_until_its_management_port_closes() {
        for stale_owned in [false,true] {
        let root=std::env::temp_dir().join(format!("yo3-attached-shutdown-{}-{stale_owned}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap();let settings=Settings::default();
        if stale_owned {
            let mut child=Command::new(std::env::current_exe().unwrap()).arg("--list").stdout(Stdio::null()).spawn().unwrap();
            child.wait().unwrap();*backend.server.lock().unwrap()=Some(child);
        }
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        admin::write_live(&backend.data_dir(&settings),listener.local_addr().unwrap().port()).unwrap();
        let (finished,closed)=std::sync::mpsc::channel();
        let manager=std::thread::spawn(move || {
            drop(listener.accept().unwrap().0);
            mock_run_bound_stop(&listener);std::thread::sleep(std::time::Duration::from_millis(150));drop(listener);finished.send(()).unwrap();
        });
        let reply=backend.stop_server(&settings).unwrap();let waited=closed.try_recv().is_ok();
        manager.join().unwrap();drop(backend);std::fs::remove_dir_all(root).unwrap();
        assert_eq!(reply["stopping"],true);
        assert!(waited,"an attached server must release its old management port before Stop completes (stale owned child: {stale_owned})");
        }
    }
    #[test]
    fn an_unreadable_log_directory_shows_a_recovery_message_instead_of_empty_logs() {
        let root=std::env::temp_dir().join(format!("yo3-unreadable-log-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut backend=Backend::new(root.clone()).unwrap();
        // Windows cannot rename a directory containing open log files. Point the reader at an independent broken path.
        let blocked=root.join("unreadable");std::fs::create_dir_all(blocked.join("logs")).unwrap();std::fs::write(blocked.join("logs/sessions"),"not a directory").unwrap();
        Arc::get_mut(&mut backend).unwrap().root=blocked;
        assert!(backend.logs(false).contains("Could not read logs"));
        drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn early_helper_errors_reach_the_user_and_export_even_before_game_logging_starts() {
        let root=std::env::temp_dir().join(format!("yo3-early-error-{}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap();
        let mut cmd=Command::new("sh"); cmd.args(["-c","printf 'Cannot load yo3-auth.dll\\nkey=secret-value\\n' >&2; exit 7"]);
        let (mut child,errors)=process::spawn(&mut cmd,&backend.diagnostics,"Game helper").unwrap();
        assert!(!child.wait().unwrap().success());
        let detail=errors.finish(); assert!(detail.contains("Cannot load yo3-auth.dll")); assert!(!detail.contains("secret-value"));
        let logs=backend.logs(false); assert!(logs.contains("Cannot load yo3-auth.dll")); assert!(!logs.contains("secret-value"));
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn explicit_server_options_override_diagnostic_defaults_without_duplicate_flags() {
        let root=std::env::temp_dir().join(format!("yo3-explicit-flags-{}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap(); let mut s=Settings::default();
        s.server_options.extend([("duel-check".into(),"false".into()),("http-port".into(),"18080".into()),("duel-lp".into(),"1000".into())]);
        let cmd=backend.server_command(&s,std::net::Ipv4Addr::LOCALHOST.into()).unwrap();
        let args:Vec<_>=cmd.get_args().map(|s|s.to_string_lossy().to_string()).collect();
        assert!(!args.iter().any(|s|s=="--duel-check")); assert!(!args.iter().any(|s|s=="--http-port"));
        assert_eq!(args.iter().filter(|s|s.starts_with("--http-port=")).count(),1); assert!(args.contains(&"--http-port=18080".into()));
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn the_default_banlist_is_the_games_unlimited_list() {
        // The player's request of 2026-11-22: new players start on the unlimited list. Needs the game (skipped without).
        let game = std::env::var("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(|_| dirs_home().join(".wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3"));
        let lists = banlists(&game);
        let Some(id) = unlimited(&lists) else { return };
        let root = std::env::temp_dir().join(format!("yo3-desktop-unlimited-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("server-data");
        yo3_server::store::initialize(&data).unwrap();
        apply_banlist(&data, &Settings { game_dir: game.to_string_lossy().into(), ..Settings::default() }).unwrap();
        let live = yo3_server::settings::Live::open(Some(&data.join("settings.json")), lists).unwrap();
        assert_eq!(live.now().login_regulation, id);
        let _ = std::fs::remove_dir_all(&root);
    }
    fn dirs_home() -> PathBuf { PathBuf::from(std::env::var_os("HOME").unwrap_or_default()) }

    #[test]
    fn the_banlist_is_saved_reaches_the_server_and_an_old_regulation_flag_moves_to_it() {
        let root=std::env::temp_dir().join(format!("yo3-desktop-banlist-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        let backend=Backend::new(root.clone()).unwrap();
        let db=LauncherDb::open(&root).unwrap();
        // Before the option: nothing passed (the default is applied as a live setting below).
        let old: Settings = serde_json::from_value(serde_json::json!({"game_dir": "saved game", "bind_address": "127.0.0.1"})).unwrap();
        assert_eq!(old.banlist, "");
        let cmd=backend.server_command(&old,std::net::Ipv4Addr::LOCALHOST.into()).unwrap();
        assert!(!cmd.get_args().any(|a| a.to_string_lossy().contains("regulation")),"{cmd:?}");
        // A chosen list: its id, written as the server's login-regulation setting (#24); back to the default with "".
        db.save_settings(&Settings { game_dir: "kept".into(), ..Settings::default() }).unwrap();
        assert_eq!(db.save_banlist(" 3 ").unwrap(), "3");
        assert!(db.save_banlist("0").is_err() && db.save_banlist("--duel-lp=1").is_err() && db.save_banlist("1\n2").is_err());
        let saved=db.settings().unwrap();
        assert_eq!((saved.banlist.as_str(), saved.game_dir.as_str()), ("3", "kept"));
        let cmd=backend.server_command(&saved,std::net::Ipv4Addr::LOCALHOST.into()).unwrap();
        assert!(!cmd.get_args().any(|a| a.to_string_lossy().contains("regulation")),"{cmd:?}");
        let data=root.join("server-data");
        yo3_server::store::initialize(&data).unwrap();
        apply_banlist(&data,&saved).unwrap();
        let login=|| yo3_server::settings::Live::open(Some(&data.join("settings.json")),vec![]).unwrap().now().login_regulation;
        assert_eq!(login(),3);
        assert_eq!(db.save_banlist("").unwrap(), "");
        apply_banlist(&data,&db.settings().unwrap()).unwrap();
        assert_eq!(login(),yo3_server::front::LOGIN_REGULATION, "a game without an unlimited list: the server's default");
        assert_eq!(unlimited(&[(1, "Beta Forbidden/Limited".into()), (4, " unlimited ".into())]), Some(4), "the game's own name, any case");
        assert_eq!(unlimited(&[(1, "Season 16".into())]), None);
        // A host who set --regulation under All server flags keeps it, now as the Banlist.
        db.save_settings(&Settings { server_options: [("regulation".into(), "Season 14".into())].into(), ..Settings::default() }).unwrap();
        let moved=db.settings().unwrap();
        assert_eq!((moved.banlist.as_str(), moved.server_options.contains_key("regulation")), ("Season 14", false));
        assert!(server_options::managed("regulation").is_some());
        drop(db); drop(backend); let _=std::fs::remove_dir_all(&root);
    }
    #[test]
    fn automatic_diagnostics_keeps_expensive_duel_validation_an_explicit_server_option() {
        for check in [false, true] {
            let root=std::env::temp_dir().join(format!("yo3-desktop-duel-check-{}-{check}",std::process::id()));
            let _=std::fs::remove_dir_all(&root);
            let backend=Backend::new(root.clone()).unwrap();
            let mut settings=Settings{listen_port:17443,..Settings::default()};
            if check { settings.server_options.insert("duel-check".into(),"true".into()); }
            let cmd=backend.server_command(&settings,std::net::Ipv4Addr::LOCALHOST.into()).unwrap();
            assert_eq!(cmd.get_args().any(|a| a=="--duel-check"),check,"{cmd:?}");
            assert!(cmd.get_args().any(|a| a=="--no-packet-log"),"raw developer files stay separate from bounded session diagnostics: {cmd:?}");
            drop(backend); std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    #[cfg(unix)]
    fn hosting_survives_the_managers_process_group_ending() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "ps -o pgid= -p $$"]).stdout(Stdio::piped());
        detach_server(&mut cmd);
        let child = cmd.spawn().unwrap();
        let pid = child.id();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let group: u32 = String::from_utf8(output.stdout).unwrap().trim().parse().unwrap();
        assert_eq!(group, pid, "hosting must outlive cleanup of the manager's process group");
    }
    #[test]
    #[cfg(unix)]
    fn closing_a_game_that_already_exited_is_idempotent() {
        let root = std::env::temp_dir().join(format!("yo3-ended-game-{}", std::process::id()));
        let backend = Backend::new(root.clone()).unwrap();
        let mut child = Command::new("sh").args(["-c", "exit 0"]).stdin(Stdio::piped()).spawn().unwrap();
        while child.try_wait().unwrap().is_none() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        *backend.game.lock().unwrap() = Some(child);
        backend.stop_game().unwrap();
        backend.stop_game().unwrap();
        let mut failed = Command::new("sh").args(["-c", "exit 7"]).spawn().unwrap();
        while failed.try_wait().unwrap().is_none() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        *backend.game.lock().unwrap() = Some(failed);
        assert!(backend.game_failure().unwrap().contains("export logs"));
        assert!(backend.game_failure().is_none(), "report an unsuccessful launch only once");
        std::fs::remove_dir_all(root).unwrap();
    }
    /// The helper marks a running game out of date when the server pins another card profile (#60); the marker is the Desktop's
    /// to see only beside the ready file of the game it started, and goes with the game.
    #[test]
    fn a_game_that_loaded_another_card_profile_than_the_servers_is_marked_stale_until_it_closes() {
        let root = std::env::temp_dir().join(format!("yo3-stale-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let backend = Backend::new(root.clone()).unwrap();
        assert!(!backend.game_profile_stale(), "no game, nothing stale");
        let ready = root.join("profiles/game-ready-1.json");
        std::fs::write(&ready, "{}").unwrap();
        *backend.game_ready.lock().unwrap() = Some(ready.clone());
        assert!(!backend.game_profile_stale());
        std::fs::write(yo3_launcher::decks::stale_file(&ready), "anime1").unwrap();
        assert!(backend.game_profile_stale(), "the helper saw the server pin another profile");
        std::fs::remove_file(yo3_launcher::decks::stale_file(&ready)).unwrap();
        assert!(!backend.game_profile_stale(), "the game is current again");
        std::fs::write(yo3_launcher::decks::stale_file(&ready), "anime2").unwrap();
        backend.clear_game_ready();
        assert!(!yo3_launcher::decks::stale_file(&ready).exists() && !backend.game_profile_stale(), "the marker goes with the game");
        std::fs::remove_dir_all(root).unwrap();
    }
    /// Another program's file stopped the start (#68): the helper exits with a failure and has written its decision into the
    /// report; that is said as the dialog's findings, not as the game having failed, and once.
    #[test]
    #[cfg(unix)]
    fn a_start_the_helper_held_for_another_programs_file_is_reported_as_findings_not_a_failure() {
        let root = std::env::temp_dir().join(format!("yo3-held-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let backend = Backend::new(root.clone()).unwrap();
        let ready = root.join("profiles/game-ready-1.json");
        let survey = foreign::survey_file(&ready);
        let report = yo3_launcher::survey::Survey { modules: vec![yo3_launcher::survey::Module { name: "dinput8.dll".into(), path: "C:\\g\\dinput8.dll".into(), game_folder: true, loaded: true, ..Default::default() }], ..Default::default() };
        std::fs::write(&survey, serde_json::to_vec(&report).unwrap()).unwrap();
        yo3_launcher::survey::record_decision(&survey, yo3_launcher::survey::Decision::Held).unwrap();
        let mut helper = Command::new("sh").args(["-c", "exit 1"]).spawn().unwrap();
        while helper.try_wait().unwrap().is_none() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        *backend.game.lock().unwrap() = Some(helper);
        *backend.game_ready.lock().unwrap() = Some(ready);
        assert_eq!(backend.game_failure(), None, "not a failed game: the dialog says what was found");
        let held = backend.take_held().expect("what was found");
        assert_eq!((held.blocked, held.files()), (false, vec!["dinput8.dll".to_string()]));
        assert!(backend.take_held().is_none(), "said once");
        assert!(!survey.exists(), "the helper's report goes with the start");
        // A start that was not held reports its failure as before, and a game that went on leaves nothing to say.
        let ready = root.join("profiles/game-ready-2.json");
        std::fs::write(foreign::survey_file(&ready), serde_json::to_vec(&report).unwrap()).unwrap();
        yo3_launcher::survey::record_decision(&foreign::survey_file(&ready), yo3_launcher::survey::Decision::Went).unwrap();
        let mut played = Command::new("sh").args(["-c", "exit 3"]).spawn().unwrap();
        while played.try_wait().unwrap().is_none() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        *backend.game.lock().unwrap() = Some(played);
        *backend.game_ready.lock().unwrap() = Some(ready);
        assert!(backend.game_failure().unwrap().contains("export logs") && backend.take_held().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    mod component_fixture { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../yo3-launcher/tests/common/mod.rs")); }
    /// A stand-in engine program: answers its version, and otherwise (the startup check) needs `vendor/required.dat`
    /// in the engine directory it is given, as the real one needs its card data.
    #[cfg(unix)]
    const ENGINE_FIXTURE:&str="#!/bin/sh\n[ \"$1\" = --version ] && { echo 'yo3-engine fixture'; exit 0; }\n[ -f \"$1/vendor/required.dat\" ] || { echo \"yo3-engine: open '$1/vendor/required.dat': No such file or directory (os error 2)\" >&2; exit 2; }\nexit 0\n";
    #[cfg(unix)]
    fn server_resource_fixture(name:&str)->Arc<Backend> {
        use std::os::unix::fs::PermissionsExt;
        let root=std::env::temp_dir().join(format!("yo3-resources-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut backend=Backend::new(root.clone()).unwrap();let engine=root.join("engine");
        for name in ["vendor/cardinfo.json","vendor/BabelCDB/cards.cdb","vendor/CardScripts/constant.lua","vendor/CardScripts/utility.lua",
            "vendor/CardScripts/official/c1.lua","vendor/required.dat"] {
            let path=engine.join(name);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,b"fixture").unwrap();
        }
        let engine_program=root.join("yo3-engine");std::fs::write(&engine_program,ENGINE_FIXTURE).unwrap();
        std::fs::set_permissions(&engine_program,std::fs::Permissions::from_mode(0o755)).unwrap();
        let server=root.join("yo3-server");std::fs::write(&server,b"verified server").unwrap();
        std::fs::write(root.join("bundle.json"),serde_json::json!({"components":{"server":{"sha256":yo3_launcher::components::hash(b"verified server")},
            "engine":{"sha256":yo3_launcher::components::hash(ENGINE_FIXTURE.as_bytes())}}}).to_string()).unwrap();
        Arc::get_mut(&mut backend).unwrap().runtime=Runtime {server,engine,launcher:root.join("yo3-launcher"),injector:root.join("yo3-auth-launch.exe"),dll:root.join("yo3-auth.dll"),engine_program};
        backend
    }
    #[test]
    #[cfg(unix)]
    fn packaged_server_resources_reject_a_changed_server_or_engine_and_a_missing_engine() {
        let backend=server_resource_fixture("package-integrity");let root=backend.root.clone();backend.check_server_resources().unwrap();
        std::fs::write(&backend.runtime.server,b"changed server").unwrap();
        let changed=backend.check_server_resources().unwrap_err().to_string();assert!(changed.contains("yo3-server") && changed.contains("manifest"),"{changed}");
        std::fs::write(&backend.runtime.server,b"verified server").unwrap();
        let program=&backend.runtime.engine_program;std::fs::write(program,format!("{ENGINE_FIXTURE}# changed\n")).unwrap();
        let changed=backend.check_server_resources().unwrap_err().to_string();assert!(changed.contains("yo3-engine") && changed.contains("manifest"),"{changed}");
        std::fs::remove_file(program).unwrap();
        let missing=backend.check_server_resources().unwrap_err().to_string();assert!(missing.contains("yo3-engine"),"{missing}");
        drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn missing_required_files_collects_every_absent_path() {
        let backend=server_resource_fixture("missing-files");let root=backend.root.clone();let game=root.join("game");
        std::fs::create_dir_all(game.join("package")).unwrap();std::fs::write(game.join("yo3.exe"),b"").unwrap();std::fs::create_dir(game.join("yo3.idx")).unwrap();
        std::fs::write(&backend.runtime.launcher,b"").unwrap();std::fs::create_dir(&backend.runtime.dll).unwrap();
        for path in [&backend.runtime.server,&backend.runtime.engine_program,&backend.runtime.engine.join("vendor/BabelCDB/cards.cdb"),&backend.runtime.engine.join("vendor/CardScripts/utility.lua")] {std::fs::remove_file(path).unwrap();}
        std::fs::write(root.join("profiles/loader.ini"),b"configured direct launch").unwrap();
        let mut settings=Settings{game_dir:game.to_string_lossy().into(),..Settings::default()};
        let mut expected=vec![backend.runtime.injector.clone(),backend.runtime.server.clone(),game.join("package/data002.ypk"),
            backend.runtime.engine_program.clone(),backend.runtime.engine.join("vendor/BabelCDB/cards.cdb"),backend.runtime.engine.join("vendor/CardScripts/utility.lua")];expected.sort();
        assert_eq!(backend.missing_required_files(&settings,None),expected,"report all ordinary Play requirements even when game preflight fails first; configured optional direct-start files and existing invalid files are separate errors");
        let server=[backend.runtime.server.clone(),backend.runtime.engine_program.clone(),backend.runtime.engine.join("vendor/BabelCDB/cards.cdb"),backend.runtime.engine.join("vendor/CardScripts/utility.lua")];
        settings.game_dir.clear();let mut expected=vec![backend.runtime.injector.clone()];expected.extend(server.clone());expected.sort();
        assert_eq!(backend.missing_required_files(&settings,None),expected,"an unchosen game folder is not a list of missing relative files");
        let opened=root.join("engine/vendor/missing.cdb");let imported=root.join("engine/vendor/missing.json");
        let diagnostic=format!("yo3-engine: open '{}': No such file or directory (os error 2)\nError: ENOENT: no such file or directory, open '{}'\nCannot find module '{}'\nCannot find module 'relative.mjs'\nOther error: open '{}'",opened.display(),imported.display(),backend.runtime.launcher.display(),root.join("unrelated.bin").display());
        let mut expected=vec![backend.runtime.injector.clone(),imported,opened];expected.extend(server);expected.sort();
        assert_eq!(backend.missing_required_files(&settings,Some(&diagnostic)),expected,"only actual absolute missing-file paths supplement the finite inventory; existing files and unrelated text stay excluded");
        drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn a_port_taken_by_another_server_is_reported_before_starting() {
        let backend=server_resource_fixture("port-taken");let root=backend.root.clone();
        let other=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let settings=Settings{listen_port:other.local_addr().unwrap().port(),game_dir:"not reached".into(),..Settings::default()};
        let error=backend.start_server(&settings).unwrap_err().to_string();
        assert!(error.contains("already used by another program"),"{error}");
        assert!(backend.server.lock().unwrap().is_none() && !backend.data_dir(&settings).join("server.db").exists());
        drop(other);drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn engine_data_is_rechecked_after_ready_and_before_attaching_to_a_live_server() {
        use std::io::{Read,Write};use std::sync::atomic::{AtomicBool,Ordering};
        let backend=server_resource_fixture("data-deletion");let root=backend.root.clone();backend.check_server_resources().unwrap();
        std::fs::remove_file(backend.runtime.engine.join("vendor/required.dat")).unwrap();
        let missing=backend.check_server_resources().unwrap_err().to_string();assert!(missing.contains("required.dat"),"{missing}");
        let settings=Settings{game_dir:"an attached server does not need a replacement game path".into(),..Settings::default()};
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();
        admin::write_live(&backend.data_dir(&settings),listener.local_addr().unwrap().port()).unwrap();
        let stop=Arc::new(AtomicBool::new(false));let stopping=stop.clone();let thread=std::thread::spawn(move|| {
            let deadline=std::time::Instant::now()+std::time::Duration::from_secs(3);
            while !stopping.load(Ordering::Relaxed) && std::time::Instant::now()<deadline {
                if let Ok((mut stream,_))=listener.accept(){let _=stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));let mut input=[0;4096];
                    if stream.read(&mut input).unwrap_or(0)>0 {let _=stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\n{\"online\":0}");}}
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        let attached=backend.start_server(&settings);stop.store(true,Ordering::Relaxed);thread.join().unwrap();
        assert!(attached.unwrap_err().to_string().contains("required.dat"));
        assert!(backend.game.lock().unwrap().is_none() && backend.server.lock().unwrap().is_none());
        assert!(!backend.data_dir(&settings).join("server.db").exists());drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn engine_startup_check_has_a_short_budget_and_reaps_its_own_process() {
        use std::os::unix::fs::PermissionsExt;
        let backend=server_resource_fixture("probe-timeout");let root=backend.root.clone();
        // Test the timeout and reaping with an engine that never finishes loading.
        // exec preserves the recorded PID and avoids leaving a descendant with the diagnostic pipe open.
        let program=&backend.runtime.engine_program;
        std::fs::write(program,"#!/bin/sh\nprintf '%s' \"$$\" > \"$1/probe.pid\"\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(program,std::fs::Permissions::from_mode(0o755)).unwrap();
        let started=std::time::Instant::now();let error=backend.check_engine_start(std::time::Duration::from_millis(500)).unwrap_err().to_string();
        assert!(error.contains("timed out") && error.contains("yo3-engine"),"{error}");assert!(started.elapsed()<std::time::Duration::from_secs(2));
        let pid=std::fs::read_to_string(backend.runtime.engine.join("probe.pid")).unwrap();assert!(pid.parse::<u32>().is_ok());
        assert!(!Command::new("kill").args(["-0",pid.trim()]).stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success(),"the engine check must not leave its process running or a zombie");
        assert!(backend.game.lock().unwrap().is_none() && backend.server.lock().unwrap().is_none());assert!(!backend.data_dir(&Settings::default()).exists());
        drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn automatic_setup_checks_do_not_save_drafts_refresh_direct_launch_or_start_processes() {
        let root=std::env::temp_dir().join(format!("yo3-read-only-setup-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let backend=Backend::new(root.clone()).unwrap();
        let saved=Settings{game_dir:"saved installation".into(),local_account:"MyPlayer".into(),..Settings::default()};
        LauncherDb::open(&root).unwrap().save_settings(&saved).unwrap();
        std::fs::write(root.join("profiles/loader.ini"),b"existing direct-start setup").unwrap();
        std::fs::write(root.join("profiles/direct.json"),b"existing private connection").unwrap();
        let draft=Settings{game_dir:root.join("unfinished game").to_string_lossy().into(),..saved.clone()};
        let error=backend.verify_setup(&draft).unwrap_err();
        assert!(error.to_string().contains("Choose the installed game folder containing yo3.exe"),"preserve the actionable game error: {error:#}");
        assert_eq!(LauncherDb::open(&root).unwrap().settings().unwrap(),saved,"an automatic check must never save an unfinished draft");
        assert_eq!(std::fs::read(root.join("profiles/loader.ini")).unwrap(),b"existing direct-start setup");
        assert_eq!(std::fs::read(root.join("profiles/direct.json")).unwrap(),b"existing private connection");
        assert!(backend.game.lock().unwrap().is_none()); assert!(backend.server.lock().unwrap().is_none());
        assert!(!backend.data_dir(&draft).exists(),"a check must never initialize hosting data");
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn automatic_setup_checks_do_not_require_a_launcher_database() {
        let root=std::env::temp_dir().join(format!("yo3-setup-without-db-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let backend=Backend::new(root.clone()).unwrap();
        let settings=Settings{game_dir:root.join("missing installation").to_string_lossy().into(),..Settings::default()};
        let error=backend.verify_setup(&settings).unwrap_err();
        assert!(error.to_string().contains("Choose the installed game folder containing yo3.exe"));
        assert!(!root.join("launcher.db").exists(),"verification belongs to the captured settings, without creating or consulting preference storage");
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_changed_connection_launcher_is_rejected_before_play() {
        let root=std::env::temp_dir().join(format!("yo3-desktop-launcher-hash-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut backend=Backend::new(root.clone()).unwrap(); let binary=root.join("connection-helper"); let injector=root.join("injector.exe"); let dll=root.join("mod.dll");
        std::fs::write(root.join("yo3.exe"),component_fixture::game_fixture()).unwrap();
        std::fs::write(&injector,component_fixture::pe_fixture(yo3_tunnel::MOD_ABI as u32)).unwrap(); std::fs::write(&dll,component_fixture::pe_fixture(yo3_tunnel::MOD_ABI as u32)).unwrap(); std::fs::write(&binary,b"our verified helper").unwrap();
        Arc::get_mut(&mut backend).unwrap().runtime=Runtime{server:root.join("server"),launcher:binary.clone(),injector,dll,engine:root.join("engine"),engine_program:root.join("yo3-engine")};
        let digest=yo3_launcher::components::hash(&component_fixture::pe_fixture(yo3_tunnel::MOD_ABI as u32));
        std::fs::write(root.join("bundle.json"),serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,"components":{
            "injector":{"sha256":digest},"connection_mod":{"sha256":digest},"launcher":{"sha256":yo3_launcher::components::hash(b"our verified helper")}}}).to_string()).unwrap();
        let settings=Settings{game_dir:root.to_string_lossy().into(),..Settings::default()};
        backend.check_components(&settings).unwrap(); std::fs::write(binary,b"changed helper").unwrap();
        assert!(backend.check_components(&settings).is_err()); drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    /// What v0.0.21 (and every release before the beta) does with launcher.db on each start and when it promotes an
    /// update it launched: refuse `user_version` above 1, recreate its tables, read and write `update_state`.
    fn released_launcher_opens(path: &Path) -> anyhow::Result<()> {
        let conn = Connection::open(path)?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        ensure!(version <= 1, "Launcher database is newer than this application.");
        conn.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS servers (id INTEGER PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1; COMMIT;
            CREATE TABLE IF NOT EXISTS update_state (id INTEGER PRIMARY KEY CHECK(id IN (1,2)), body TEXT NOT NULL);
            INSERT INTO update_state VALUES(1,'{}') ON CONFLICT(id) DO UPDATE SET body=excluded.body;")?;
        Ok(())
    }

    #[test]
    fn released_launchers_still_open_the_database_so_their_shortcuts_reach_the_update() {
        let root = std::env::temp_dir().join(format!("yo3-desktop-compatible-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("launcher.db");
        let schema = |path: &Path| -> (u32, u32) { let conn = Connection::open(path).unwrap();
            (conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap(), conn.query_row("SELECT version FROM launcher_schema", [], |r| r.get(0)).unwrap()) };

        // A v0.0.21 database (with a saved server row) opens here, and v0.0.21 still opens it afterwards.
        std::fs::create_dir_all(&root).unwrap();
        released_launcher_opens(&path).unwrap();
        Connection::open(&path).unwrap().execute("INSERT INTO servers(body) VALUES('{}')", []).unwrap();
        LauncherDb::open(&root).unwrap().remote_servers().unwrap();
        assert_eq!(schema(&path), (COMPATIBLE, LAUNCHER_SCHEMA));
        assert_eq!(COMPATIBLE, 1, "raising user_version strands every released launcher: see LauncherDb::migrate");
        released_launcher_opens(&path).expect("a released launcher must still open the database");
        LauncherDb::open(&root).unwrap();
        assert_eq!(schema(&path), (1, LAUNCHER_SCHEMA), "the old launcher's visit changes nothing here");

        // A beta development build's database (user_version 2, no launcher_schema) is set back to 1.
        std::fs::remove_dir_all(&root).unwrap(); std::fs::create_dir_all(&root).unwrap();
        Connection::open(&path).unwrap().execute_batch("CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);
            CREATE TABLE remote_servers (key TEXT PRIMARY KEY, body TEXT NOT NULL); INSERT INTO remote_servers VALUES('k','{}'); PRAGMA user_version=2;").unwrap();
        LauncherDb::open(&root).unwrap();
        assert_eq!(schema(&path), (1, LAUNCHER_SCHEMA));
        assert_eq!(Connection::open(&path).unwrap().query_row("SELECT count(*) FROM remote_servers", [], |r| r.get::<_, u32>(0)).unwrap(), 1, "kept");
        released_launcher_opens(&path).unwrap();

        // A later launcher's schema is never lowered, and an incompatible database is refused.
        Connection::open(&path).unwrap().execute("UPDATE launcher_schema SET version=?1", [LAUNCHER_SCHEMA + 5]).unwrap();
        LauncherDb::open(&root).unwrap();
        assert_eq!(schema(&path), (1, LAUNCHER_SCHEMA + 5));
        Connection::open(&path).unwrap().execute_batch("PRAGMA user_version=3;").unwrap();
        assert!(LauncherDb::open(&root).err().unwrap().to_string().contains("newer than this application"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn addresses_are_explicit_and_do_not_accept_urls_or_missing_hosts() {
        assert_eq!(server_address(" play.example ").unwrap(), ("play.example:7443".into(), "play.example".into()));
        assert_eq!(server_address("192.168.1.5:8000").unwrap(), ("192.168.1.5:8000".into(), "192.168.1.5".into()));
        assert_eq!(server_address("[::1]:7443").unwrap(), ("[::1]:7443".into(), "::1".into()));
        for bad in ["", "https://host", "host:0", "host:no", ":7443", "host/path", "a b"] {
            assert!(server_address(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn resolution_is_saved_in_its_normal_form_and_old_settings_mean_off() {
        let root = std::env::temp_dir().join(format!("yo3-desktop-resolution-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let db = LauncherDb::open(&root).unwrap();
        let old: Settings = serde_json::from_value(serde_json::json!({"game_dir": "saved game", "bind_address": "127.0.0.1"})).unwrap();
        assert_eq!(old.resolution, "", "settings saved before the option exist mean the original size");
        assert_eq!("".parse::<yo3_launcher::resolution::Resolution>(), Ok(yo3_launcher::resolution::Resolution::Off));
        db.save_settings(&Settings { game_dir: "kept".into(), ..Settings::default() }).unwrap();
        assert_eq!(db.save_resolution("1440X1080").unwrap(), "1440x1080");
        assert_eq!(db.save_resolution("Auto").unwrap(), "auto");
        assert!(db.save_resolution("100x100").is_err() && db.save_resolution("banana").is_err());
        let saved = db.settings().unwrap();
        assert_eq!((saved.resolution.as_str(), saved.game_dir.as_str()), ("auto", "kept"), "a refused value keeps the last one and nothing else changes");
        drop(db); let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sqlite_remembers_settings() {
        let root = std::env::temp_dir().join(format!("yo3-desktop-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let db = LauncherDb::open(&root).unwrap();
        let settings = Settings { game_dir: "a game with spaces".into(), local_account: "Duelist".into(), ..Settings::default() };
        db.save_settings(&settings).unwrap(); drop(db);
        let db = LauncherDb::open(&root).unwrap();
        assert_eq!(db.settings().unwrap(), settings);
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
