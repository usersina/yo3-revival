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
pub mod updates;
pub mod preferences;

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
    pub detailed_logs: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { game_dir: yo3_data::default_game_dir().to_string_lossy().into(), wine_prefix: String::new(),
            host_address: "localhost".into(), data_dir: String::new(), bind_address: "0.0.0.0".into(), bind_interface: None, listen_port: 7443,
            player_name: "Duelist".into(), selected_server: None, local_account: String::new(), detailed_logs: false }
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
        match self { Self::Local => f.write_str("My server"), Self::Saved { title, .. } => f.write_str(title) }
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
    #[cfg(windows)] { return PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("YO3 Revival"); }
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
pub enum Readiness { Ready, GameProblem(String), ComponentProblem(String) }
pub struct Backend { pub root: PathBuf, pub runtime: Runtime, pub diagnostics: yo3_tunnel::diagnostics::Session,
    server_session: Mutex<Option<String>>, game: Mutex<Option<Child>>, game_ready: Mutex<Option<PathBuf>>, server: Mutex<Option<Child>> }
impl Backend {
    pub fn new(root: PathBuf) -> anyhow::Result<Arc<Self>> {
        yo3_tunnel::diagnostics::private_dir(&root.join("profiles"))?;
        std::fs::create_dir_all(root.join("logs"))?;
        let diagnostics = yo3_tunnel::diagnostics::Session::start(&root.join("logs/sessions"), "app", None)?;
        Ok(Arc::new(Self { root, runtime: Runtime::discover()?, diagnostics, server_session: Mutex::new(None), game: Mutex::new(None), game_ready: Mutex::new(None), server: Mutex::new(None) }))
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
        if let Some(live) = self.live(settings) { return admin::manage(&live, &ManagerOp::Status).map_err(anyhow::Error::msg) }
        ensure!(owned.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "Server is still starting.");
        validate_game(Path::new(&settings.game_dir))?;
        ensure!(self.runtime.server.is_file(), "The server executable is missing. Extract the complete application bundle.");
        ensure!(yo3_server::engine::problem(Some(&self.runtime.engine)).is_none(), "The rules engine is incomplete. Extract the complete application bundle again.");
        let (_, name) = server_address(&settings.host_address)?;
        let bind = network::resolve_bind(settings, &network::bindings()?)?;
        ensure!(settings.listen_port > 0, "Port must be between 1 and 65535.");
        let port = settings.listen_port.to_string();
        let data = self.data_dir(settings);
        yo3_server::auth::maintain_identity(&data, vec!["localhost".into(), "127.0.0.1".into(), name.clone()])?;
        yo3_server::store::initialize(&data)?;
        let mut directory = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json"))?;
        if directory.entries().next().is_none() { directory.add_account(std::slice::from_ref(&settings.player_name)).map_err(anyhow::Error::msg)?; }
        drop(directory);
        let mut cmd = Command::new(&self.runtime.server);
        cmd.args(["--bind", &bind.to_string(), "--auth-port", &port, "--http-port", "0"])
            .arg("--data-dir").arg(data).arg("--game-dir").arg(&settings.game_dir).arg("--log-dir").arg(self.root.join("logs"))
            .arg("--diagnostics-dir").arg(self.root.join("logs/sessions")).arg("--diagnostic-parent").arg(self.diagnostics.id())
            .env("YO3_ENGINE_DIR", &self.runtime.engine).env("NO_COLOR", "1").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if !settings.detailed_logs { cmd.arg("--no-packet-log"); }
        detach_server(&mut cmd);
        *self.server_session.lock().unwrap() = None;
        *owned = Some(cmd.spawn().context("Start the server")?);
        for _ in 0..100 {
            if let Ok(status) = self.status(settings) { return Ok(status) }
            if owned.as_mut().unwrap().try_wait()?.is_some() { anyhow::bail!("Server exited during startup. Open Activity for details.") }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        anyhow::bail!("Server did not become ready. Open Activity for details.")
    }
    pub fn stop_server(&self, settings: &Settings) -> anyhow::Result<Value> {
        self.manage(settings, ManagerOp::Shutdown)
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
        ensure!(network::share_address(settings)?.is_some(), "Set a sharing address under My server → Advanced before exporting an invitation for friends.");
        self.account_connection(settings, who)
    }
    fn account_connection(&self, settings: &Settings, who: &str) -> anyhow::Result<SavedServer> {
        let key = self.manage(settings, ManagerOp::ExportAccount { who: who.into() })?["key"].as_str().context("Missing account key")?.to_string();
        let (address, server_name) = network::advertised(settings)?;
        Ok(SavedServer { id: 0, label: format!("{}'s server", who), address, server_name,
            certificate: std::fs::read_to_string(self.data_dir(settings).join("server.crt"))?, identity: None, key })
    }
    pub fn game_running(&self) -> bool {
        self.game.lock().unwrap().as_mut().is_some_and(|child| child.try_wait().ok().flatten().is_none())
    }
    pub fn game_ready(&self) -> bool { self.game_ready.lock().unwrap().as_ref().is_some_and(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() <= 65536)) }
    fn clear_game_ready(&self) { if let Some(path) = self.game_ready.lock().unwrap().take() { let _ = std::fs::remove_file(path); } }
    pub fn game_failure(&self) -> Option<String> {
        let mut game = self.game.lock().unwrap();
        let status = game.as_mut()?.try_wait().ok().flatten()?;
        *game = None; self.clear_game_ready();
        (!status.success()).then(|| "The game could not start or exited unexpectedly. Open Activity → Game & launcher for details.".into())
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
    pub fn readiness(&self, settings: &Settings) -> Readiness {
        if let Err(error) = validate_game(Path::new(&settings.game_dir)) { return Readiness::GameProblem(format!("{error:#}")) }
        match self.check_components(settings) { Ok(()) => Readiness::Ready, Err(error) => Readiness::ComponentProblem(format!("{error:#}")) }
    }
    pub fn check_game(&self, settings: &Settings) -> anyhow::Result<()> {
        validate_game(Path::new(&settings.game_dir))?; self.check_components(settings)
    }
    fn check_components(&self, settings: &Settings) -> anyhow::Result<()> {
        ensure!(self.runtime.launcher.is_file(), "The connection launcher is missing. Restore the complete application bundle.");
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
        if settings.detailed_logs { cmd.arg("--detailed-logs"); }
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        let ready = self.root.join("profiles").join(format!("game-ready-{}-{}.json", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()));
        cmd.arg("--ready-file").arg(&ready);
        hide_console(&mut cmd);
        let child = cmd.spawn()?;
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
        let selected: Vec<_> = yo3_tunnel::diagnostics::records(&root).unwrap_or_default().into_iter()
            .filter(|r| r.name.ends_with(".log") && ids.contains(&r.id) && r.id.starts_with(if server { "server-" } else { "game-" })).collect();
        let fallback: Vec<_> = if server { vec![] } else { yo3_tunnel::diagnostics::records(&root).unwrap_or_default().into_iter().filter(|r| r.name.ends_with(".log") && ids.contains(&r.id) && (r.id.starts_with("app-") || r.id.starts_with("direct-"))).collect() };
        let mut result = String::new();
        use std::io::{Read, Seek, SeekFrom};
        for record in fallback.into_iter().chain(selected).rev() {
            let Ok(mut f) = std::fs::File::open(root.join(record.name)) else { continue };
            let len = f.metadata().map(|m| m.len()).unwrap_or(0); let remaining = (96 * 1024usize).saturating_sub(result.len()); if remaining == 0 { break }
            let _ = f.seek(SeekFrom::Start(len.saturating_sub(remaining as u64)));
            let mut bytes = Vec::new(); let _ = f.take(remaining as u64).read_to_end(&mut bytes);
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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(backend.game_failure().unwrap().contains("Activity"));
        assert!(backend.game_failure().is_none(), "report an unsuccessful launch only once");
        std::fs::remove_dir_all(root).unwrap();
    }
    mod component_fixture { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../yo3-launcher/tests/common/mod.rs")); }
    #[test]
    fn a_changed_connection_launcher_is_rejected_before_play() {
        let root=std::env::temp_dir().join(format!("yo3-desktop-launcher-hash-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut backend=Backend::new(root.clone()).unwrap(); let binary=root.join("connection-helper"); let injector=root.join("injector.exe"); let dll=root.join("mod.dll");
        std::fs::write(root.join("yo3.exe"),component_fixture::pe_fixture(2)).unwrap();
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
