//! Desktop application model and process control, independent of the windowing toolkit.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use anyhow::{Context, ensure};
use rusqlite::{Connection, OptionalExtension};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use yo3_server::admin::{self, ManagerOp};
pub mod network;
pub mod privileged;
pub mod direct;
pub mod reports;
pub mod system_info;
pub mod bug_report;
pub mod bug_evidence;
pub mod github_credentials;
pub mod github_report;
pub mod bug_publish;
pub mod updates;
pub mod preferences;
pub mod server_options;
pub mod game_data;
mod game_data_platform;
mod process;

pub fn server_address(input: &str) -> anyhow::Result<(String, String)> {
    let input = input.trim();
    let input = input.strip_prefix("yo3://").unwrap_or(input).trim_end_matches('/');
    ensure!(!input.is_empty() && !input.contains(['/', '\\', '@']) && !input.chars().any(char::is_whitespace), "Enter a hostname or IP address, optionally followed by a port.");
    if let Ok(ip) = input.parse::<std::net::IpAddr>() {
        return Ok((match ip { std::net::IpAddr::V6(_) => format!("[{ip}]:7443"), _ => format!("{ip}:7443") }, ip.to_string()));
    }
    if let Ok(addr) = input.parse::<std::net::SocketAddr>() {
        ensure!(addr.port() != 0, "Port must be between 1 and 65535.");
        return Ok((addr.to_string(), addr.ip().to_string()));
    }
    let (host, port) = input.rsplit_once(':').unwrap_or((input, "7443"));
    let port: u16 = port.parse().context("Port must be between 1 and 65535.")?;
    ensure!(port > 0 && !host.is_empty() && host.bytes().all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b)), "Enter a valid hostname and port.");
    Ok((format!("{host}:{port}"), host.to_string()))
}

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
    pub selected_server: Option<i64>,
    pub local_account: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self { advanced_mode: false, server_options: Default::default(), game_dir: yo3_data::default_game_dir().to_string_lossy().into(), wine_prefix: String::new(),
            host_address: "localhost".into(), data_dir: String::new(), bind_address: "0.0.0.0".into(), bind_interface: None, listen_port: 7443,
            player_name: "Duelist".into(), selected_server: None, local_account: String::new() }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedServer {
    #[serde(default)] pub id: i64,
    #[serde(default)] pub label: String,
    pub address: String,
    pub server_name: String,
    pub certificate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub identity: Option<String>,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerChoice { Local, Saved { id: i64, title: String } }
impl ServerChoice {
    pub fn saved(server: &SavedServer) -> Self { Self::Saved { id: server.id, title: format!("{} · {}", server.label, server.address) } }
    pub fn remote(&self, servers: &[SavedServer]) -> anyhow::Result<Option<SavedServer>> {
        match self {
            Self::Local => Ok(None),
            Self::Saved { id, .. } => servers.iter().find(|s| s.id == *id).cloned().map(Some).context("This saved connection no longer exists."),
        }
    }
}
impl std::fmt::Display for ServerChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Local => f.write_str("This computer"), Self::Saved { title, .. } => f.write_str(title) }
    }
}
impl SavedServer {
    pub fn endpoint(&self) -> anyhow::Result<yo3_tunnel::Endpoint> {
        let identity = self.identity.clone().map(Ok).unwrap_or_else(|| yo3_tunnel::identity_from_pem(self.certificate.as_bytes()))?;
        yo3_tunnel::Endpoint::pinned(self.address.clone(), self.server_name.clone(), &identity)
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        server_address(&self.address)?;
        ensure!(self.key.is_ascii() && self.key.len() == 20, "An account access key has 20 characters.");
        ensure!(self.certificate.contains("-----BEGIN CERTIFICATE-----"), "Select the host's public certificate (.crt), or import their invitation.");
        ensure!(!self.server_name.is_empty(), "The certificate name is missing.");
        self.endpoint()?;
        Ok(())
    }
    pub fn import(path: &Path) -> anyhow::Result<Self> {
        ensure!(std::fs::metadata(path)?.len() <= 65536, "Invitation file is too large.");
        let v: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        // Accept both a desktop invitation and the existing launcher profile + adjacent certificate.
        let certificate = v["certificate"].as_str().context("Invitation is missing the host certificate.")?;
        let certificate = if certificate.contains("-----BEGIN CERTIFICATE-----") { certificate.to_string() }
            else {
                let file = path.parent().unwrap_or(Path::new(".")).join(certificate);
                ensure!(std::fs::metadata(&file)?.len() <= 65536, "Certificate file is too large.");
                std::fs::read_to_string(file)?
            };
        let s = Self { id: 0, label: v["label"].as_str().unwrap_or_else(|| path.file_stem().and_then(|p| p.to_str()).unwrap_or("Server")).into(),
            address: v["address"].as_str().or_else(|| v["server"].as_str()).context("Invitation is missing the address.")?.into(),
            server_name: v["server_name"].as_str().context("Invitation is missing the certificate name.")?.into(),
            key: v["key"].as_str().context("Invitation is missing the account key.")?.into(),
            identity: v["identity"].as_str().map(str::to_string).or_else(|| yo3_tunnel::identity_from_pem(certificate.as_bytes()).ok()), certificate };
        s.validate()?;
        Ok(s)
    }
    pub fn export(&self, path: &Path) -> anyhow::Result<()> {
        self.validate()?;
        private_write(path, &serde_json::to_vec_pretty(self)?, false)
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

pub struct LauncherDb { conn: Connection }
impl LauncherDb {
    pub fn open(root: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(root)?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?; }
        let path = root.join("launcher.db");
        if !path.exists() { private_write(&path, &[], false)?; }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        ensure!(version <= 1, "Launcher database is newer than this application.");
        conn.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS servers (id INTEGER PRIMARY KEY, body TEXT NOT NULL); PRAGMA user_version=1; COMMIT;")?;
        Ok(Self { conn })
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
        Ok(settings)
    }
    pub fn save_settings(&self, settings: &Settings) -> anyhow::Result<()> {
        self.conn.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET body=excluded.body", [serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn save_mode(&self, advanced: bool) -> anyhow::Result<()> {
        let mut saved = self.settings()?; saved.advanced_mode = advanced; self.save_settings(&saved)
    }
    pub fn servers(&self) -> anyhow::Result<Vec<SavedServer>> {
        let mut stmt = self.conn.prepare("SELECT id,body FROM servers ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.map(|row| { let (id, body) = row?; let mut server: SavedServer = serde_json::from_str(&body)?; server.id = id; Ok(server) }).collect()
    }
    pub fn save_server(&self, server: &SavedServer) -> anyhow::Result<i64> {
        let body = serde_json::to_string(server)?;
        if server.id == 0 { self.conn.execute("INSERT INTO servers(body) VALUES(?1)", [body])?; }
        else { ensure!(self.conn.execute("UPDATE servers SET body=?1 WHERE id=?2", (body, server.id))? == 1, "This connection no longer exists."); }
        Ok(if server.id == 0 { self.conn.last_insert_rowid() } else { server.id })
    }
    pub fn delete_server(&mut self, id: i64) -> anyhow::Result<()> {
        let mut settings = self.settings()?; let transaction = self.conn.transaction()?;
        ensure!(transaction.execute("DELETE FROM servers WHERE id=?1", [id])? == 1, "This connection no longer exists.");
        if settings.selected_server == Some(id) {
            settings.selected_server = None;
            transaction.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET body=excluded.body", [serde_json::to_string(&settings)?])?;
        }
        transaction.commit()?; Ok(())
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
pub struct Runtime { pub server: PathBuf, pub launcher: PathBuf, pub injector: PathBuf, pub dll: PathBuf, pub engine: PathBuf }
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
            launcher: root.join(format!("yo3-launcher{}", std::env::consts::EXE_SUFFIX)), injector: mods.join("yo3-auth-launch.exe"), dll: mods.join("yo3-auth.dll"), engine })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness { Ready, GameProblem(String), ComponentProblem(String), ValidationProblem(String) }
pub struct Backend { pub root: PathBuf, pub runtime: Runtime, pub diagnostics: yo3_tunnel::diagnostics::Session,
    server_session: Mutex<Option<String>>, game: Mutex<Option<Child>>, game_errors: Mutex<Option<process::Errors>>, game_ready: Mutex<Option<PathBuf>>, server: Mutex<Option<Child>> }
impl Backend {
    pub fn new(root: PathBuf) -> anyhow::Result<Arc<Self>> {
        yo3_tunnel::diagnostics::private_dir(&root.join("profiles"))?;
        std::fs::create_dir_all(root.join("logs"))?;
        let diagnostics = yo3_tunnel::diagnostics::Session::start(&root.join("logs/sessions"), "app", None)?;
        Ok(Arc::new(Self { root, runtime: Runtime::discover()?, diagnostics, server_session: Mutex::new(None), game: Mutex::new(None), game_errors: Mutex::new(None), game_ready: Mutex::new(None), server: Mutex::new(None) }))
    }
    pub fn data_dir(&self, settings: &Settings) -> PathBuf {
        if settings.data_dir.trim().is_empty() { self.root.join("server") } else { PathBuf::from(&settings.data_dir) }
    }
    pub fn live(&self, settings: &Settings) -> Option<admin::Live> { admin::running(&self.data_dir(settings)) }
    pub fn status(&self, settings: &Settings) -> anyhow::Result<Value> {
        let live = self.live(settings).context("Server stopped")?;
        let status = admin::manage(&live, &ManagerOp::Status).map_err(anyhow::Error::msg)?;
        if let Some(id) = status["diagnostic_id"].as_str().filter(|id| yo3_tunnel::diagnostics::valid_id(id)) { *self.server_session.lock().unwrap() = Some(id.into()); }
        Ok(status)
    }
    pub fn accounts(&self, settings: &Settings) -> anyhow::Result<Value> {
        let live = self.live(settings).context("Start your server to manage accounts.")?;
        admin::manage(&live, &ManagerOp::Accounts).map_err(anyhow::Error::msg)
    }
    pub fn manage(&self, settings: &Settings, op: ManagerOp) -> anyhow::Result<Value> {
        let live = self.live(settings).context("Start your server first.")?;
        admin::manage(&live, &op).map_err(anyhow::Error::msg)
    }
    pub fn start_server(&self, settings: &Settings) -> anyhow::Result<Value> {
        let mut owned = self.server.lock().unwrap();
        self.check_server_resources()?;
        if let Some(live) = self.live(settings) { return admin::manage(&live, &ManagerOp::Status).map_err(anyhow::Error::msg) }
        ensure!(owned.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "Server is still starting.");
        validate_game(Path::new(&settings.game_dir))?;
        let (_, name) = server_address(&settings.host_address)?;
        let bind = network::effective_bind(settings, network::resolve_bind(settings, &network::bindings()?)?);
        ensure!(settings.listen_port > 0, "Port must be between 1 and 65535.");
        let mut cmd = self.server_command(settings, bind)?;
        let data = self.data_dir(settings);
        yo3_server::auth::maintain_identity(&data, vec!["localhost".into(), "127.0.0.1".into(), name.clone()])?;
        yo3_server::store::initialize(&data)?;
        let mut directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
        if directory.entries().next().is_none() { directory.add_account(std::slice::from_ref(&settings.player_name)).map_err(anyhow::Error::msg)?; }
        drop(directory);
        detach_server(&mut cmd);
        *self.server_session.lock().unwrap() = None;
        let (child,errors)=process::spawn(&mut cmd,&self.diagnostics,"Server")?;
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
    fn server_command(&self, settings: &Settings, bind: std::net::IpAddr) -> anyhow::Result<Command> {
        let extra = server_options::arguments(settings)?;
        let port = settings.listen_port.to_string();
        let mut cmd = Command::new(&self.runtime.server);
        cmd.args(["--bind", &bind.to_string(), "--auth-port", &port])
            .arg("--data-dir").arg(self.data_dir(settings)).arg("--game-dir").arg(&settings.game_dir).arg("--log-dir").arg(self.root.join("logs"))
            .arg("--diagnostics-dir").arg(self.root.join("logs/sessions")).arg("--diagnostic-parent").arg(self.diagnostics.id())
            .env("YO3_ENGINE_DIR", &self.runtime.engine).env("NO_COLOR", "1").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if !settings.server_options.contains_key("http-port") { cmd.args(["--http-port", "0"]); }
        if !settings.server_options.contains_key("no-packet-log") { cmd.arg("--no-packet-log"); }
        cmd.args(extra);
        Ok(cmd)
    }
    pub fn stop_server(&self, settings: &Settings) -> anyhow::Result<Value> {
        let mut owned = self.server.lock().unwrap();
        let live=self.live(settings).context("Start your server first.")?;
        // Keep the captured endpoint: another app can replace the profile's live file during shutdown.
        let reply=admin::manage(&live,&ManagerOp::Shutdown).map_err(anyhow::Error::msg)?;
        // The management response acknowledges the request before ports and data files are released.
        // Serialize Start with the owned child's graceful exit and original endpoint closure. An old child
        // handle may already have exited while this app attached to a replacement server, so check both.
        // Never terminate an unowned process or send another shutdown to a replacement endpoint.
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
    pub fn invitation(&self, settings: &Settings, who: &str) -> anyhow::Result<SavedServer> {
        ensure!(network::share_address(settings)?.is_some(), "Save a reachable sharing address under Advanced → Hosting → Advanced before making invitations.");
        self.account_connection(settings, who)
    }
    fn account_connection(&self, settings: &Settings, who: &str) -> anyhow::Result<SavedServer> {
        let key = self.manage(settings, ManagerOp::ExportAccount { who: who.into() })?["key"].as_str().context("Missing account key")?.to_string();
        let (address, server_name) = network::advertised(settings)?;
        Ok(SavedServer { id: 0, label: format!("{}'s server", who), address, server_name,
            certificate: std::fs::read_to_string(settings.server_options.get("auth-cert").map(PathBuf::from)
                .unwrap_or_else(|| self.data_dir(settings).join("server.crt")))?, identity: None, key })
    }
    pub fn game_running(&self) -> bool {
        self.game.lock().unwrap().as_mut().is_some_and(|child| child.try_wait().ok().flatten().is_none())
    }
    pub fn game_ready(&self) -> bool { self.game_ready.lock().unwrap().as_ref().is_some_and(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() <= 65536)) }
    fn clear_game_ready(&self) { if let Some(path) = self.game_ready.lock().unwrap().take() { let _ = std::fs::remove_file(path); } }
    pub fn game_failure(&self) -> Option<String> {
        let mut game = self.game.lock().unwrap();
        let status = game.as_mut()?.try_wait().ok().flatten()?;
        let ready=self.game_ready(); *game = None; self.clear_game_ready();
        let detail=self.game_errors.lock().unwrap().take().map(|e|e.finish()).unwrap_or_default();
        (!status.success()).then(|| format!("{} ({status}).\n{detail}\nTry Play again; it rechecks the required files. If it still fails, export logs for help.",if ready{"The game stopped unexpectedly"}else{"The game could not start"}))
    }
    pub fn local_connection(&self, settings: &Settings, who: &str) -> anyhow::Result<SavedServer> {
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
        if settings.selected_server.is_none() {
            paths.push(self.runtime.server.clone());
            if packaged {paths.push(self.runtime.engine.join("node").join(format!("node{}",std::env::consts::EXE_SUFFIX)));}
            paths.extend(yo3_server::engine::missing_files(&self.runtime.engine));
        }
        if let Some(diagnostic)=diagnostic {
            let bounded=diagnostic.chars().take(16*1024).collect::<String>();
            for line in bounded.lines() {
                let quoted=line.split_once("Cannot find module ").map(|(_,path)|path)
                    .or_else(||line.contains("ENOENT").then(||line.split_once("open ").map(|(_,path)|path)).flatten());
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
        let components=self.check_components(settings).and_then(|()| if settings.selected_server.is_none() { self.check_server_resources() } else { Ok(()) });
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
            let expected=value["components"]["server"]["sha256"].as_str().filter(|hash|hash.len()==64 && hash.bytes().all(|byte|byte.is_ascii_hexdigit()))
                .with_context(||format!("Release manifest is missing the server SHA-256: {}",manifest.display()))?;
            let mut file=std::fs::File::open(&self.runtime.server).with_context(||format!("Could not read required server file: {}",self.runtime.server.display()))?.take(256*1024*1024+1);
            let mut digest=ring::digest::Context::new(&ring::digest::SHA256);let mut buffer=[0;16384];let mut size=0;
            loop {
                let read=file.read(&mut buffer).with_context(||format!("Could not read required server file: {}",self.runtime.server.display()))?;
                if read==0{break}size+=read;
                ensure!(size<=256*1024*1024,"Required server file is unexpectedly large: {}",self.runtime.server.display());digest.update(&buffer[..read]);
            }
            let actual=digest.finish().as_ref().iter().map(|byte|format!("{byte:02x}")).collect::<String>();
            ensure!(actual.eq_ignore_ascii_case(expected),"{} differs from the release manifest. Restore the complete matching bundle.",self.runtime.server.display());
            let node=self.runtime.engine.join("node").join(format!("node{}",std::env::consts::EXE_SUFFIX));
            ensure!(node.is_file(),"Required bundled Node.js file is missing: {}. Restore the complete matching bundle.",node.display());
        } else {
            ensure!(yo3_tunnel::BUILD_VERSION.ends_with("-dev"),"Release manifest is missing: {}. Restore the complete matching bundle.",manifest.display());
        }
        if let Some(problem)=yo3_server::engine::problem(Some(&self.runtime.engine)) {
            anyhow::bail!("The rules engine is incomplete: {problem}. Extract the complete application bundle again.");
        }
        self.check_engine_start(std::time::Duration::from_secs(10))
    }
    fn check_engine_start(&self,timeout:std::time::Duration)->anyhow::Result<()> {
        let sidecar=self.runtime.engine.join("duelcore.ts");let node=yo3_server::engine::node(&self.runtime.engine);
        let mut cmd=Command::new(&node);cmd.arg(&sidecar).stdin(Stdio::null()).stdout(Stdio::null());hide_console(&mut cmd);
        self.diagnostics.line("Checking that the rules engine can load its dependencies without starting a duel");
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
        let cause=detail.lines().find(|line|line.starts_with("Error") || line.starts_with("SyntaxError") || line.starts_with("TypeError")
            || line.contains("Cannot find module") || line.contains("ENOENT")).unwrap_or_else(||detail.lines().find(|line|!line.trim().is_empty()).unwrap_or("No engine diagnostic was returned."));
        match result {
            Ok(status) if status.success()=>{waited.context("Could not reap the completed engine check")?;Ok(())}
            Ok(status)=>anyhow::bail!("The rules engine could not start: {cause}\nSidecar: {}\nNode.js: {}\nExit status: {status}\n{detail}\nRestore the complete matching bundle, then Retry check.",sidecar.display(),node.display()),
            Err(error)=>anyhow::bail!("{error:#}. Sidecar: {}\n{cause}\n{detail}\nRestore the complete matching bundle or export logs if this repeats.",sidecar.display()),
        }
    }
    pub fn validate_setup(&self, draft: &Settings, remote: Option<&SavedServer>) -> anyhow::Result<Settings> {
        let saved=LauncherDb::open(&self.root)?.settings()?;
        let next=preferences::Section::Game.merge(&saved,draft);
        self.verify_setup(&next,remote)?;
        self.save_preferences(preferences::Section::Game,&next)
    }
    /// Checks a captured setup without saving preferences, refreshing direct launch, or starting play/hosting.
    pub fn verify_setup(&self, settings: &Settings, remote: Option<&SavedServer>) -> anyhow::Result<()> {
        self.diagnostics.line("Validating original game files and required connection components");
        self.check_game(settings)?;
        if let Some(remote)=remote {
            let info=self.check_connection(remote)?;
            ensure!(info.ready,"{} Ask the host to repair the server, then choose Retry check.",info.problem.as_deref().unwrap_or("The host's rules engine is unavailable."));
        }
        else { self.check_server_resources()?; server_options::arguments(settings)?; }
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
    pub fn play(&self, settings: &Settings, remote: Option<SavedServer>, who: &str) -> anyhow::Result<()> {
        let mut game = self.game.lock().unwrap();
        ensure!(game.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "The game is already running.");
        self.check_game(settings)?;
        let mut server = match remote {
            Some(s) => s,
            None => {
                self.start_server(settings)?;
                self.local_connection(settings, who)?
            }
        };
        server.id = server.id.max(0);
        server.validate()?;
        self.sync_direct(settings, Some(server.clone()))?;
        let profile = self.write_profile(&server)?;
        ensure!(self.runtime.launcher.is_file() && self.runtime.dll.is_file() && self.runtime.injector.is_file(), "Connection files are missing. Extract the complete application bundle.");
        let mut cmd = Command::new(&self.runtime.launcher);
        cmd.arg("--control-stdin").arg("--profile").arg(profile).arg("--game-dir").arg(&settings.game_dir).arg("--injector").arg(&self.runtime.injector).arg("--dll").arg(&self.runtime.dll);
        if !settings.wine_prefix.is_empty() { cmd.arg("--wine-prefix").arg(&settings.wine_prefix); }
        cmd.arg("--diagnostics-dir").arg(self.root.join("logs/sessions")).arg("--diagnostic-parent").arg(self.diagnostics.id());
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        let ready = self.root.join("profiles").join(format!("game-ready-{}-{}.json", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()));
        cmd.arg("--ready-file").arg(&ready);
        hide_console(&mut cmd);
        let (child,errors)=process::spawn(&mut cmd,&self.diagnostics,"Game helper")?;
        *self.game_errors.lock().unwrap()=Some(errors);
        self.clear_game_ready(); *self.game_ready.lock().unwrap() = Some(ready);
        *game = Some(child);
        Ok(())
    }
    pub fn write_profile(&self, server: &SavedServer) -> anyhow::Result<PathBuf> {
        let stem = format!("connection-{}", server.id);
        private_write(&self.root.join("profiles").join(format!("{stem}.crt")), server.certificate.as_bytes(), true)?;
        let file = self.root.join("profiles").join(format!("{stem}.json"));
        private_write(&file, serde_json::json!({"server":server.address,"server_name":server.server_name,"key":server.key,"certificate":format!("{stem}.crt"), "identity":server.identity.clone().or_else(|| yo3_tunnel::identity_from_pem(server.certificate.as_bytes()).ok())}).to_string().as_bytes(), true)?;
        // Parse through the existing TLS implementation before starting any game process.
        yo3_launcher::Profile::load(&file)?;
        Ok(file)
    }
    pub fn check_connection(&self, server: &SavedServer) -> anyhow::Result<yo3_tunnel::ServerInfo> {
        server.validate()?;
        let endpoint = server.endpoint()?;
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let info = endpoint.info().await?;
            info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI)?;
            let session = endpoint.login(&server.key).await?;
            endpoint.logout(&session).await?;
            Ok::<_, anyhow::Error>(info)
        }).context("Connection check failed; verify the address and the host's invitation")
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
    #[test]
    #[cfg(unix)]
    fn stopping_an_owned_server_waits_until_it_has_exited_before_start_can_retry() {
        use std::io::{Read,Write};
        let root=std::env::temp_dir().join(format!("yo3-server-shutdown-{}",std::process::id()));
        let backend=Backend::new(root.clone()).unwrap(); let settings=Settings::default();
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        admin::write_live(&backend.data_dir(&settings),listener.local_addr().unwrap().port()).unwrap();
        let mut child=Command::new("sh").args(["-c","read finish; sleep 0.15"]).stdin(Stdio::piped()).spawn().unwrap();
        let mut finish=child.stdin.take().unwrap(); *backend.server.lock().unwrap()=Some(child);
        let manager=std::thread::spawn(move || {
            // admin::running makes a TCP liveness probe before the actual management request.
            drop(listener.accept().unwrap().0);
            let (mut stream,_)=listener.accept().unwrap(); stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let mut request=Vec::new(); let mut byte=[0];
            while !request.ends_with(b"\r\n\r\n") { stream.read_exact(&mut byte).unwrap(); request.push(byte[0]); }
            let mut body=vec![0;b"{\"op\":\"shutdown\"}".len()]; stream.read_exact(&mut body).unwrap();
            assert_eq!(body,b"{\"op\":\"shutdown\"}");
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"stopping\":true}").unwrap();
            drop(stream); finish.write_all(b"done\n").unwrap();
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
    fn stopping_an_attached_server_waits_until_its_management_port_closes() {
        use std::io::{Read,Write};
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
            let (mut stream,_)=listener.accept().unwrap();stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let mut request=Vec::new();let mut byte=[0];
            while !request.ends_with(b"\r\n\r\n") {stream.read_exact(&mut byte).unwrap();request.push(byte[0]);}
            let mut body=vec![0;b"{\"op\":\"shutdown\"}".len()];stream.read_exact(&mut body).unwrap();assert_eq!(body,b"{\"op\":\"shutdown\"}");
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"stopping\":true}").unwrap();
            drop(stream);std::thread::sleep(std::time::Duration::from_millis(150));drop(listener);finished.send(()).unwrap();
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
    mod component_fixture { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../yo3-launcher/tests/common/mod.rs")); }
    #[cfg(unix)]
    fn server_resource_fixture(name:&str)->Arc<Backend> {
        let root=std::env::temp_dir().join(format!("yo3-resources-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut backend=Backend::new(root.clone()).unwrap();let engine=root.join("engine");
        for name in ["duelcore.ts","package.json","node_modules/ocgcore-wasm/package.json","node_modules/ocgcore-wasm/dist/index.js",
            "node_modules/ocgcore-wasm/lib/ocgcore.sync.mjs","node_modules/ocgcore-wasm/lib/ocgcore.sync.wasm","vendor/cardinfo.json",
            "vendor/BabelCDB/cards.cdb","vendor/CardScripts/constant.lua","vendor/CardScripts/utility.lua","vendor/CardScripts/official/c1.lua"] {
            let path=engine.join(name);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,b"fixture").unwrap();
        }
        std::fs::write(engine.join("package.json"),br#"{"type":"module"}"#).unwrap();
        std::fs::write(engine.join("duelcore.ts"),"import './required-chunk.mjs'; process.stdin.resume();").unwrap();
        std::fs::write(engine.join("required-chunk.mjs"),"export const ready=true;").unwrap();
        let node=std::env::split_paths(&std::env::var_os("PATH").expect("Node must be on PATH for engine startup fixtures"))
            .map(|path|path.join("node")).find(|path|path.is_file()).expect("Node must be installed for engine startup fixtures");
        std::fs::create_dir_all(engine.join("node")).unwrap();std::os::unix::fs::symlink(node,engine.join("node/node")).unwrap();
        let server=root.join("yo3-server");std::fs::write(&server,b"verified server").unwrap();
        std::fs::write(root.join("bundle.json"),serde_json::json!({"components":{"server":{"sha256":yo3_launcher::components::hash(b"verified server")}}}).to_string()).unwrap();
        Arc::get_mut(&mut backend).unwrap().runtime=Runtime {server,engine,launcher:root.join("yo3-launcher"),injector:root.join("yo3-auth-launch.exe"),dll:root.join("yo3-auth.dll")};
        backend
    }
    #[test]
    #[cfg(unix)]
    fn packaged_server_resources_reject_a_changed_server_and_missing_bundled_node() {
        let backend=server_resource_fixture("package-integrity");let root=backend.root.clone();backend.check_server_resources().unwrap();
        std::fs::write(&backend.runtime.server,b"changed server").unwrap();
        let changed=backend.check_server_resources().unwrap_err().to_string();assert!(changed.contains("yo3-server") && changed.contains("manifest"),"{changed}");
        std::fs::write(&backend.runtime.server,b"verified server").unwrap();std::fs::remove_file(backend.runtime.engine.join("node/node")).unwrap();
        let missing=backend.check_server_resources().unwrap_err().to_string();assert!(missing.contains("engine/node/node"),"packaged checks must not fall back to PATH: {missing}");
        drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn missing_required_files_collects_every_absent_path_for_the_selected_mode() {
        let backend=server_resource_fixture("missing-files");let root=backend.root.clone();let game=root.join("game");
        std::fs::create_dir_all(game.join("package")).unwrap();std::fs::write(game.join("yo3.exe"),b"").unwrap();std::fs::create_dir(game.join("yo3.idx")).unwrap();
        std::fs::write(&backend.runtime.launcher,b"").unwrap();std::fs::create_dir(&backend.runtime.dll).unwrap();
        for path in [&backend.runtime.server,&backend.runtime.engine.join("node/node"),&backend.runtime.engine.join("node_modules/ocgcore-wasm/lib/ocgcore.sync.wasm"),&backend.runtime.engine.join("vendor/CardScripts/utility.lua")] {std::fs::remove_file(path).unwrap();}
        std::fs::write(root.join("profiles/loader.ini"),b"configured direct launch").unwrap();
        let mut settings=Settings{game_dir:game.to_string_lossy().into(),..Settings::default()};
        let mut expected=vec![backend.runtime.injector.clone(),backend.runtime.server.clone(),game.join("package/data002.ypk"),
            backend.runtime.engine.join("node/node"),backend.runtime.engine.join("node_modules/ocgcore-wasm/lib/ocgcore.sync.wasm"),backend.runtime.engine.join("vendor/CardScripts/utility.lua")];expected.sort();
        assert_eq!(backend.missing_required_files(&settings,None),expected,"report all ordinary Play requirements even when game preflight fails first; configured optional direct-start files and existing invalid files are separate errors");
        settings.selected_server=Some(7);std::fs::remove_file(root.join("profiles/loader.ini")).unwrap();
        let mut expected=vec![backend.runtime.injector.clone(),game.join("package/data002.ypk")];expected.sort();
        assert_eq!(backend.missing_required_files(&settings,None),expected,"joining does not need local hosting or disabled direct-start components");
        settings.game_dir.clear();assert_eq!(backend.missing_required_files(&settings,None),vec![backend.runtime.injector.clone()],"an unchosen game folder is not a list of missing relative files");
        let imported=root.join("engine/node_modules/ocgcore-wasm/lib/generated-core.mjs");let opened=root.join("engine/vendor/missing.cdb");
        let diagnostic=format!("Error [ERR_MODULE_NOT_FOUND]: Cannot find module '{}' imported from duelcore.ts\nError: ENOENT: no such file or directory, open '{}'\nCannot find module '{}'\nCannot find module 'relative.mjs'\nOther error: open '{}'",imported.display(),opened.display(),backend.runtime.launcher.display(),root.join("unrelated.bin").display());
        let mut expected=vec![backend.runtime.injector.clone(),imported,opened];expected.sort();
        assert_eq!(backend.missing_required_files(&settings,Some(&diagnostic)),expected,"only actual absolute missing-import/open paths supplement the finite inventory; existing files and unrelated text stay excluded");
        drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn sidecar_imports_are_rechecked_after_ready_and_before_attaching_to_a_live_server() {
        use std::io::{Read,Write};use std::sync::atomic::{AtomicBool,Ordering};
        let backend=server_resource_fixture("import-deletion");let root=backend.root.clone();backend.check_server_resources().unwrap();
        std::fs::remove_file(backend.runtime.engine.join("required-chunk.mjs")).unwrap();
        let missing=backend.check_server_resources().unwrap_err().to_string();assert!(missing.contains("required-chunk.mjs"),"{missing}");
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
        assert!(attached.unwrap_err().to_string().contains("required-chunk.mjs"));
        assert!(backend.game.lock().unwrap().is_none() && backend.server.lock().unwrap().is_none());
        assert!(!backend.data_dir(&settings).join("server.db").exists());drop(backend);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn engine_startup_check_has_a_short_budget_and_reaps_its_own_process() {
        use std::os::unix::fs::PermissionsExt;
        let backend=server_resource_fixture("probe-timeout");let root=backend.root.clone();
        // Test the timeout and reaping, without requiring Node's cold startup to finish within 500 ms.
        // exec preserves the recorded PID and avoids leaving a descendant with the diagnostic pipe open.
        let node=backend.runtime.engine.join("node/node");std::fs::remove_file(&node).unwrap();
        std::fs::write(&node,"#!/bin/sh\nprintf '%s' \"$$\" > \"${1%/*}/probe.pid\"\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(&node,std::fs::Permissions::from_mode(0o755)).unwrap();
        let started=std::time::Instant::now();let error=backend.check_engine_start(std::time::Duration::from_millis(500)).unwrap_err().to_string();
        assert!(error.contains("timed out") && error.contains("duelcore.ts"),"{error}");assert!(started.elapsed()<std::time::Duration::from_secs(2));
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
        let error=backend.verify_setup(&draft,None).unwrap_err();
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
        let error=backend.verify_setup(&settings,None).unwrap_err();
        assert!(error.to_string().contains("Choose the installed game folder containing yo3.exe"));
        assert!(!root.join("launcher.db").exists(),"verification belongs to the captured settings, without creating or consulting preference storage");
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_changed_connection_launcher_is_rejected_before_play() {
        let root=std::env::temp_dir().join(format!("yo3-desktop-launcher-hash-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut backend=Backend::new(root.clone()).unwrap(); let binary=root.join("connection-helper"); let injector=root.join("injector.exe"); let dll=root.join("mod.dll");
        std::fs::write(root.join("yo3.exe"),component_fixture::game_fixture()).unwrap();
        std::fs::write(&injector,component_fixture::pe_fixture(2)).unwrap(); std::fs::write(&dll,component_fixture::pe_fixture(2)).unwrap(); std::fs::write(&binary,b"our verified helper").unwrap();
        Arc::get_mut(&mut backend).unwrap().runtime=Runtime{server:root.join("server"),launcher:binary.clone(),injector,dll,engine:root.join("engine")};
        let digest=yo3_launcher::components::hash(&component_fixture::pe_fixture(2));
        std::fs::write(root.join("bundle.json"),serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":2,"components":{
            "injector":{"sha256":digest},"connection_mod":{"sha256":digest},"launcher":{"sha256":yo3_launcher::components::hash(b"our verified helper")}}}).to_string()).unwrap();
        let settings=Settings{game_dir:root.to_string_lossy().into(),..Settings::default()};
        backend.check_components(&settings).unwrap(); std::fs::write(binary,b"changed helper").unwrap();
        assert!(backend.check_components(&settings).is_err()); drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn equal_connection_labels_still_select_the_right_account() {
        let a = SavedServer { id: 1, label: "Friends".into(), address: "host:7443".into(), server_name: "host".into(), certificate: String::new(), identity: None, key: "a".repeat(20) };
        let b = SavedServer { id: 2, key: "b".repeat(20), ..a.clone() };
        let choice = ServerChoice::saved(&b);
        assert_ne!(ServerChoice::saved(&a), choice);
        assert_eq!(choice.remote(&[a, b]).unwrap().unwrap().key, "b".repeat(20));
        assert!(choice.remote(&[]).is_err(), "a missing saved server must never start a local server instead");
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
    fn sqlite_remembers_settings_and_connections_without_crossing_accounts() {
        let root = std::env::temp_dir().join(format!("yo3-desktop-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let db = LauncherDb::open(&root).unwrap();
        let settings = Settings { game_dir: "a game with spaces".into(), selected_server: Some(2), local_account: "Duelist".into(), ..Settings::default() };
        db.save_settings(&settings).unwrap();
        let a = SavedServer { id: 0, label: "Alice".into(), address: "alice.example:7443".into(), server_name: "alice.example".into(), certificate: "certificate-a".into(), identity: None, key: "a".repeat(20) };
        let mut b = a.clone(); b.label = "Bob".into(); b.address = "bob.example:7443".into(); b.key = "b".repeat(20);
        db.save_server(&a).unwrap(); db.save_server(&b).unwrap(); drop(db);
        let db = LauncherDb::open(&root).unwrap();
        assert_eq!(db.settings().unwrap().game_dir, settings.game_dir);
        assert_eq!(db.settings().unwrap().selected_server, Some(2));
        assert_eq!(db.settings().unwrap().local_account, "Duelist");
        let servers = db.servers().unwrap();
        assert_eq!(servers.len(), 2);
        assert_ne!(servers[0].key, servers[1].key);
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
