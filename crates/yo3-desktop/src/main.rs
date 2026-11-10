#![cfg_attr(windows, windows_subsystem = "windows")]
use std::sync::Arc;
use std::path::PathBuf;
use iced::{Element, Task, Subscription, Length::{Fill, Fixed}, Color, Theme};
use iced::widget::{self, button, checkbox, column, container, pick_list, row, scrollable, text, text_input};
use yo3_desktop::{Backend, LauncherDb, Settings, SavedServer, ServerChoice};
use yo3_server::admin::ManagerOp;
use serde_json::Value;

fn main() -> iced::Result {
    if let Some(code) = yo3_desktop::privileged::helper_mode() { std::process::exit(code); }
    iced::application(App::boot, App::update, App::view).title("YO3 Revival").default_font(iced::Font::with_name("Fira Sans"))
        .theme(|_: &App| Theme::custom("Revival", iced::theme::Palette { background: Color::from_rgb8(15, 20, 29),
            text: Color::from_rgb8(229, 235, 244), primary: Color::from_rgb8(75, 155, 224),
            success: Color::from_rgb8(89, 189, 151), warning: Color::from_rgb8(224, 180, 87), danger: Color::from_rgb8(222, 107, 114) }))
        .window(iced::window::Settings { size: iced::Size::new(1080.0, 760.0), min_size: Some(iced::Size::new(900.0, 650.0)), ..Default::default() }).exit_on_close_request(false)
        .subscription(|_| Subscription::batch([iced::time::every(std::time::Duration::from_secs(2)).map(|_| Msg::Tick), iced::window::close_requests().map(|_| Msg::Close)]))
        .run()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page { Play, Server, Accounts, Activity, Settings }
#[derive(Clone, Copy)]
enum Field { Game, Wine, Host, Data, Player, Label, Address, Key, Account, NewPlayer, Port }
#[derive(Clone)]
enum Msg {
    Page(Page), Input(Field, String), Select(ServerChoice), AccountSelect(String), Start, Stop, Play, StopGame, TestConnection, Discover, TrustIdentity, AdvancedConnection, AdvancedNetwork, RefreshBindings, BindSelect(yo3_desktop::network::BindAddress), Firewall, Direct(bool),
    SaveSettings, Browse(Field), Import, AddServer, SaveServer, Certificate, CreateAccount, AddPlayer, Export,
    Tick, Poll(Result<(Option<Value>, bool, String, Option<String>), String>), Done(Result<Outcome, String>), Logs(bool), Pause(bool),
    LogFilter(String), CopyLogs, PrepareReport(bool), SaveLogs, Detailed(bool), OpenData, Help, Close, CancelClose, Quit(bool), ConfirmCertificate, RenewCertificate,
}
impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("DesktopEvent") }
}
#[derive(Clone)]
enum Outcome { Info(String), Status(Value), Accounts(Value), Path(Field, String), Imported(SavedServer), Observed(yo3_tunnel::ObservedServer), Checked(yo3_tunnel::ServerInfo), Bindings(Vec<yo3_desktop::network::BindAddress>), Certificate(String), SettingsSaved(Settings), ConnectionsSaved(Vec<SavedServer>), Report(yo3_desktop::reports::Report), Quit }

struct App {
    root: PathBuf, backend: Option<Arc<Backend>>, settings: Settings, draft: Settings,
    page: Page, servers: Vec<SavedServer>, selected: ServerChoice, status: Option<Value>, accounts: Value,
    account: String, new_account: String, new_player: String, editor: bool, label: String, address: String, key: String,
    certificate: String, server_name: String, busy: bool, polling: bool, game_running: bool, notice: String,
    failed: bool, log_server: bool, paused: bool, logs: String, filter: String, closing: bool, confirm_certificate: bool,
    identity: Option<String>, observed: Option<yo3_tunnel::ObservedServer>, checked: Option<yo3_tunnel::ServerInfo>, advanced_connection: bool,
    bindings: Vec<yo3_desktop::network::BindAddress>, advanced_network: bool, port: String,
    report: Option<yo3_desktop::reports::Report>,
}

fn job(f: impl FnOnce() -> anyhow::Result<Outcome> + Send + 'static) -> Task<Msg> {
    Task::perform(async move { tokio::task::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:#}")) }, Msg::Done)
}
impl App {
    fn boot() -> Self {
        let root = yo3_desktop::default_root();
        let mut error = String::new();
        let db = LauncherDb::open(&root).map_err(|e| error = e.to_string()).ok();
        let settings = db.as_ref().and_then(|d| d.settings().map_err(|e| error = e.to_string()).ok()).unwrap_or_default();
        let servers = db.as_ref().and_then(|d| d.servers().map_err(|e| error = e.to_string()).ok()).unwrap_or_default();
        let selected = settings.selected_server.and_then(|id| servers.iter().find(|s| s.id == id)).map(ServerChoice::saved).unwrap_or(ServerChoice::Local);
        let account = settings.local_account.clone();
        let backend = if error.is_empty() { Backend::new(root.clone()).map_err(|e| error = e.to_string()).ok() } else { None };
        let page = if std::path::Path::new(&settings.game_dir).join("yo3.exe").is_file() { Page::Play } else { Page::Settings };
        let port = settings.listen_port.to_string();
        Self { root, backend, draft: settings.clone(), settings, page, servers, selected,
            status: None, accounts: Value::Null, account, new_account: String::new(), new_player: String::new(),
            editor: false, label: String::new(), address: String::new(), key: String::new(), certificate: String::new(), server_name: String::new(),
            busy: false, polling: false, game_running: false, failed: !error.is_empty(), notice: if error.is_empty() { "Welcome. Choose where you want to play.".into() } else { error },
            log_server: true, paused: false, logs: String::new(), filter: String::new(), closing: false, confirm_certificate: false,
            identity: None, observed: None, checked: None, advanced_connection: false,
            bindings: yo3_desktop::network::defaults(), advanced_network: false, port, report: None }
    }
    fn update(&mut self, message: Msg) -> Task<Msg> {
        if self.busy && !matches!(message, Msg::Done(_) | Msg::Poll(_) | Msg::Tick | Msg::Close | Msg::CancelClose) { return Task::none() }
        match message {
            Msg::Detailed(value) => self.draft.detailed_logs = value,
            Msg::PrepareReport(all) => {
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let settings = self.settings.clone(); self.busy = true;
                return job(move || { let _ = backend.status(&settings); Ok(Outcome::Report(backend.report(all)?)) });
            }
            Msg::Direct(enable) => {
                if self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let settings = self.settings.clone();
                let remote = match self.selected.remote(&self.servers) { Ok(s) => s, Err(e) => { self.feedback(Err(e.to_string())); return Task::none() } };
                self.busy = true;
                return job(move || { if enable { backend.enable_direct(&settings, remote)?; } else { backend.disable_direct(&settings)?; }
                    Ok(Outcome::Info(if enable { "Direct start enabled for the saved game, server and account. You can now open yo3.exe directly. Restart any open game." } else { "Direct-start loader removed. Open the game through the launcher." }.into())) });
            }
            Msg::Page(page) => { self.page = page; if page == Page::Accounts { return self.load_accounts() } else if page == Page::Server { return self.update(Msg::RefreshBindings) } }
            Msg::Input(field, value) => match field {
                Field::Game => self.draft.game_dir = value, Field::Wine => self.draft.wine_prefix = value,
                Field::Host => self.draft.host_address = value, Field::Data => self.draft.data_dir = value,
                Field::Player => self.draft.player_name = value, Field::Label => self.label = value,
                Field::Address => { self.address = value; self.server_name.clear(); self.identity = None; self.certificate.clear(); self.observed = None; }, Field::Key => self.key = value,
                Field::Account => self.new_account = value, Field::NewPlayer => self.new_player = value, Field::Port => self.port = value,
            },
            Msg::AdvancedNetwork => { self.advanced_network = !self.advanced_network; if self.advanced_network { return self.update(Msg::RefreshBindings) } }
            Msg::RefreshBindings => { self.busy = true; return job(|| Ok(Outcome::Bindings(yo3_desktop::network::bindings()?))) }
            Msg::BindSelect(binding) => { self.settings.bind_address = binding.address.to_string(); self.settings.bind_interface = binding.interface_id.clone(); self.draft.bind_address = binding.address.to_string(); self.draft.bind_interface = binding.interface_id; }
            Msg::Firewall => {
                let port = self.settings.listen_port; self.busy = true; self.notice = "Waiting for firewall permission. Normal server startup does not require elevation.".into();
                return job(move || Ok(Outcome::Info(match yo3_desktop::privileged::request_firewall(port)? {
                    yo3_desktop::privileged::PermissionResult::Applied => "Firewall rule configured for the server's TCP port.".into(),
                    yo3_desktop::privileged::PermissionResult::Cancelled => "Firewall setup cancelled. Your server and settings were left running as before.".into(),
                })));
            }
            Msg::Select(value) => {
                if self.game_running { self.feedback(Err("Close the game before changing its connection.".into())); return Task::none() }
                self.settings.selected_server = match &value { ServerChoice::Local => None, ServerChoice::Saved { id, .. } => Some(*id) };
                self.checked = None;
                self.selected = value; return self.save_selection();
            }
            Msg::AccountSelect(value) => { self.settings.local_account = value.clone(); self.account = value; return self.save_selection(); }
            Msg::AddServer => self.editor = !self.editor,
            Msg::AdvancedConnection => self.advanced_connection = !self.advanced_connection,
            Msg::Discover => {
                let address = self.address.clone(); self.busy = true;
                return job(move || Ok(Outcome::Observed(Backend::discover(&address)?)));
            }
            Msg::TrustIdentity => {
                if let Some(observed) = self.observed.take() {
                    if let Err(error) = observed.info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI) { self.feedback(Err(error.to_string())); return Task::none() }
                    self.identity = Some(observed.identity); self.certificate = observed.certificate; self.checked = Some(observed.info);
                    self.feedback(Ok("Server identity approved. Enter your account key and save the connection.".into()));
                }
            }
            Msg::Start | Msg::Stop | Msg::Play | Msg::StopGame => {
                if self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() };
                if matches!(message, Msg::Start) {
                    let port: u16 = match self.port.parse() { Ok(port) if port > 0 => port, _ => { self.feedback(Err("Port must be between 1 and 65535.".into())); return Task::none() } };
                    self.settings.listen_port = port; self.draft.listen_port = port; self.settings.host_address = self.draft.host_address.clone();
                }
                self.busy = true;
                let settings = self.settings.clone();
                let who = self.account.clone();
                match message {
                    Msg::Start => { let root = self.root.clone(); self.notice = "Starting your server…".into(); return job(move || { LauncherDb::open(&root)?.save_settings(&settings)?; Ok(Outcome::Status(backend.start_server(&settings)?)) }) }
                    Msg::Stop => { self.notice = "Stopping your server…".into(); return job(move || { backend.stop_server(&settings)?; Ok(Outcome::Info("Server is stopping.".into())) }) }
                    Msg::StopGame => return job(move || { backend.stop_game()?; Ok(Outcome::Info("Closing the game…".into())) }),
                    _ => {
                        let remote = match self.selected.remote(&self.servers) { Ok(remote) => remote, Err(e) => { self.busy = false; self.feedback(Err(e.to_string())); return Task::none() } };
                        self.notice = "Checking setup and starting the game…".into();
                        return job(move || { backend.play(&settings, remote, &who)?; Ok(Outcome::Info("Launcher started. Connection progress appears in Activity.".into())) });
                    }
                }
            }
            Msg::SaveSettings => {
                if self.busy { return Task::none() }
                let root = self.root.clone(); let mut draft = self.draft.clone(); draft.selected_server = self.settings.selected_server; draft.local_account = if draft.data_dir != self.settings.data_dir { String::new() } else { self.account.clone() }; let current = self.settings.clone(); let backend = self.backend.clone();
                let port = self.port.clone();
                self.busy = true;
                return job(move || {
                    yo3_desktop::server_address(&draft.host_address)?;
                    draft.listen_port = port.parse().map_err(|_| anyhow::anyhow!("Port must be between 1 and 65535."))?;
                    anyhow::ensure!(draft.listen_port > 0, "Port must be between 1 and 65535.");
                    yo3_desktop::network::resolve_bind(&draft, &yo3_desktop::network::bindings()?)?;
                    if backend.as_ref().is_some_and(|b| b.live(&current).is_some()) {
                        anyhow::ensure!(current.bind_address == draft.bind_address && current.bind_interface == draft.bind_interface && current.listen_port == draft.listen_port && current.data_dir == draft.data_dir && current.game_dir == draft.game_dir,
                            "Stop your server before changing its setup.");
                    }
                    LauncherDb::open(&root)?.save_settings(&draft)?;
                    if let Some(backend) = backend {
                        let remote = draft.selected_server.and_then(|id| LauncherDb::open(&root).ok()?.servers().ok()?.into_iter().find(|s| s.id == id));
                        if let Err(error) = backend.sync_direct(&draft, remote) { backend.diagnostics.line(&format!("Direct-start selection cleared; enable it again after fixing setup: {error:#}")); }
                    }
                    Ok(Outcome::SettingsSaved(draft))
                });
            }
            Msg::Browse(field) => {
                self.busy = true; return Task::perform(async move { rfd::AsyncFileDialog::new().set_title("Choose folder").pick_folder().await.map(|f| f.path().to_string_lossy().into_owned()) }, move |path| Msg::Done(Ok(path.map_or(Outcome::Info("Selection cancelled.".into()), |p| Outcome::Path(field, p)))));
            }
            Msg::Import => {
                self.busy = true; return Task::perform(async { rfd::AsyncFileDialog::new().set_title("Import server invitation or client profile").add_filter("Invitation", &["json"]).pick_file().await.map(|f| f.path().to_path_buf()) }, |path| {
                    Msg::Done(match path { Some(path) => SavedServer::import(&path).map(Outcome::Imported).map_err(|e| e.to_string()), None => Ok(Outcome::Info("Import cancelled.".into())) })
                });
            }
            Msg::Certificate => { self.busy = true; return Task::perform(async { rfd::AsyncFileDialog::new().set_title("Choose the host's public certificate").add_filter("Certificate", &["crt", "pem"]).pick_file().await.map(|f| f.path().to_path_buf()) }, |path| {
                Msg::Done(match path { Some(p) => std::fs::read_to_string(p).map(Outcome::Certificate).map_err(|e| e.to_string()), None => Ok(Outcome::Info("Selection cancelled.".into())) })
            }); },
            Msg::SaveServer => {
                let result = (|| -> anyhow::Result<SavedServer> {
                    let (address, name) = yo3_desktop::server_address(&self.address)?;
                    anyhow::ensure!(self.identity.is_some(), "Discover and approve the server identity, or import an invitation first.");
                    let server = SavedServer { id: 0, label: if self.label.trim().is_empty() { name.clone() } else { self.label.trim().into() }, address,
                        server_name: if self.server_name.is_empty() { name } else { self.server_name.clone() }, certificate: self.certificate.clone(), identity: self.identity.clone(), key: self.key.trim().into() };
                    server.validate()?;
                    Ok(server)
                })();
                match result {
                    Ok(server) => { let root = self.root.clone(); self.busy = true; return job(move || { let db = LauncherDb::open(&root)?; db.save_server(&server)?; Ok(Outcome::ConnectionsSaved(db.servers()?)) }); }
                    Err(e) => self.feedback(Err(e.to_string())),
                }
            }
            Msg::TestConnection => {
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let selected = match self.selected.remote(&self.servers) { Ok(Some(s)) => s, _ => { self.feedback(Err("Choose a saved remote connection first.".into())); return Task::none() } };
                self.busy = true; return job(move || Ok(Outcome::Checked(backend.check_connection(&selected)?)));
            }
            Msg::CreateAccount | Msg::AddPlayer => {
                if self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let settings = self.settings.clone();
                let op = if matches!(message, Msg::CreateAccount) { ManagerOp::CreateAccount { name: self.new_account.trim().into() } }
                    else { ManagerOp::AddPlayer { who: self.account.clone(), name: self.new_player.trim().into() } };
                self.busy = true;
                return job(move || { backend.manage(&settings, op)?; Ok(Outcome::Accounts(backend.accounts(&settings)?)) });
            }
            Msg::Export => {
                if self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let settings = self.settings.clone(); let who = self.account.clone(); self.busy = true;
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().set_title("Export private invitation").set_file_name("yo3-invitation.json").save_file().await else { return Ok(Outcome::Info("Export cancelled.".into())) };
                    tokio::task::spawn_blocking(move || { backend.invitation(&settings, &who)?.export(file.path())?; Ok(Outcome::Info("Private invitation exported. Send it only to this account's owner.".into())) }).await.map_err(|e| e.to_string())?.map_err(|e: anyhow::Error| e.to_string())
                }, Msg::Done);
            }
            Msg::Tick => {
                if self.polling || self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() };
                self.polling = true; let settings = self.settings.clone(); let log_server = self.log_server;
                let read_logs = !self.paused && self.page == Page::Activity;
                return Task::perform(async move { tokio::task::spawn_blocking(move || {
                    let status = backend.status(&settings).ok();
                    let logs = if read_logs { backend.logs(log_server) } else { String::new() };
                    (status, backend.game_running(), logs, backend.game_failure())
                }).await.map_err(|e| e.to_string()) }, Msg::Poll);
            }
            Msg::Poll(result) => { self.polling = false; if let Ok((status, running, logs, failure)) = result { if let Some(error) = failure { self.feedback(Err(error)); } self.status = status; self.game_running = running; if !self.paused && !logs.is_empty() { self.logs = logs; return widget::operation::snap_to_end("activity-log"); } } }
            Msg::Done(result) => {
                self.busy = false;
                match result {
                    Ok(Outcome::Info(info)) => self.feedback(Ok(info)),
                    Ok(Outcome::Status(status)) => { self.status = Some(status); self.feedback(Ok("Server is running.".into())); return self.load_accounts() }
                    Ok(Outcome::Accounts(accounts)) => {
                        self.accounts = accounts; self.new_account.clear(); self.new_player.clear();
                        if self.account.is_empty() { self.account = account_names(&self.accounts).first().cloned().unwrap_or_default(); }
                        self.feedback(Ok("Accounts updated.".into()));
                    }
                    Ok(Outcome::Path(field, path)) => return self.update(Msg::Input(field, path)),
                    Ok(Outcome::Imported(server)) => { self.identity = server.identity.clone().or_else(|| yo3_tunnel::identity_from_pem(server.certificate.as_bytes()).ok()); self.label = server.label; self.address = server.address; self.key = server.key; self.certificate = server.certificate; self.server_name = server.server_name; self.editor = true; self.page = Page::Play; self.feedback(Ok("Invitation loaded. Save this connection to continue.".into())); }
                    Ok(Outcome::Observed(server)) => { self.observed = Some(server); self.feedback(Ok("Compare this identity with the host before approving it. No account credentials were sent.".into())); }
                    Ok(Outcome::Checked(info)) => { self.checked = Some(info); self.feedback(Ok("Connection and account verified.".into())); }
                    Ok(Outcome::Bindings(bindings)) => { self.bindings = bindings; }
                    Ok(Outcome::SettingsSaved(settings)) => { if settings.data_dir != self.settings.data_dir { self.accounts = Value::Null; self.account.clear(); self.status = None; } self.settings = settings; self.feedback(Ok("Settings saved.".into())); }
                    Ok(Outcome::ConnectionsSaved(servers)) => { self.selected = ServerChoice::saved(servers.last().unwrap()); self.servers = servers; self.editor = false; self.key.clear(); self.certificate.clear(); self.settings.selected_server = self.servers.last().map(|s| s.id); self.feedback(Ok("Connection saved.".into())); return self.save_selection(); }
                    Ok(Outcome::Certificate(certificate)) => { self.identity = yo3_tunnel::identity_from_pem(certificate.as_bytes()).ok(); self.certificate = certificate; self.feedback(Ok("Host certificate selected.".into())); }
                    Ok(Outcome::Quit) => return iced::exit(),
                    Ok(Outcome::Report(report)) => self.report = Some(report),
                    Err(error) => self.feedback(Err(error)),
                }
            }
            Msg::Logs(server) => { self.log_server = server; self.logs.clear(); }
            Msg::Pause(paused) => self.paused = paused,
            Msg::LogFilter(filter) => self.filter = filter,
            Msg::CopyLogs => return iced::clipboard::write(self.logs.clone()),
            Msg::SaveLogs => {
                let Some(report) = self.report.take() else { return Task::none() }; self.busy = true;
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().add_filter("ZIP report", &["zip"]).set_file_name("yo3-diagnostics.zip").save_file().await else { return Ok(Outcome::Info("Report export cancelled.".into())) };
                    let path = file.path().to_path_buf();
                    tokio::task::spawn_blocking(move || report.export(&path)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
                    Ok(Outcome::Info("Diagnostic ZIP saved. Review it before sharing; nothing was uploaded.".into()))
                }, Msg::Done);
            }
            Msg::OpenData => { let path = self.root.to_string_lossy().to_string(); return job(move || { open(&path)?; Ok(Outcome::Info("Application data folder opened.".into())) }) }
            Msg::Help => return job(|| { open("https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md")?; Ok(Outcome::Info("Setup guide opened.".into())) }),
            Msg::Close => { if self.status.is_some() || self.game_running || self.busy { self.closing = true; } else { return iced::exit() } }
            Msg::CancelClose => { self.closing = false; self.confirm_certificate = false; self.observed = None; self.report = None; },
            Msg::ConfirmCertificate => self.confirm_certificate = true,
            Msg::RenewCertificate => {
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let settings = self.settings.clone(); self.busy = true; self.confirm_certificate = false;
                return job(move || { backend.update_certificate(&settings)?; Ok(Outcome::Info("Certificate updated. Export invitations using the new address.".into())) });
            }
            Msg::Quit(stop_server) => {
                if self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return iced::exit() };
                let settings = self.settings.clone(); self.busy = true;
                return job(move || { backend.stop_game()?; if stop_server && backend.live(&settings).is_some() { backend.stop_server(&settings)?; } Ok(Outcome::Quit) });
            }
        }
        Task::none()
    }
    fn feedback(&mut self, result: Result<String, String>) { self.failed = result.is_err(); self.notice = result.unwrap_or_else(|e| e); if let Some(backend) = &self.backend { backend.diagnostics.line(&self.notice); } }
    fn save_selection(&mut self) -> Task<Msg> {
        let root = self.root.clone(); let settings = self.settings.clone(); self.busy = true;
        let backend = self.backend.clone(); let remote = self.selected.remote(&self.servers).ok().flatten();
        job(move || { LauncherDb::open(&root)?.save_settings(&settings)?; if let Some(backend) = backend
            && let Err(error) = backend.sync_direct(&settings, remote) { backend.diagnostics.line(&format!("Direct-start selection cleared; enable it again after fixing setup: {error:#}")); }
            Ok(Outcome::SettingsSaved(settings)) })
    }
    fn load_accounts(&mut self) -> Task<Msg> {
        let Some(backend) = self.backend.clone() else { return Task::none() };
        let settings = self.settings.clone(); self.busy = true;
        job(move || Ok(Outcome::Accounts(backend.accounts(&settings)?)))
    }
    fn view(&self) -> Element<'_, Msg> {
        if let Some(report) = &self.report {
            let p = &report.preview;
            let size = if p.bytes < 1_000_000 { format!("{:.1} KB", p.bytes as f64 / 1000.0) } else { format!("{:.1} MB", p.bytes as f64 / 1_000_000.0) };
            let date = |n| chrono::DateTime::from_timestamp(n as i64, 0).map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| "No sessions".into());
            return container(column![text("Export diagnostic report").size(28),
                text(format!("{} files · {} before compression\n{} to {}", p.files, size, date(p.oldest), date(p.newest))),
                text("Includes application/server/game session logs and build/compatibility details. Credentials, databases, keys and game files are excluded. Detailed tracing may contain player/chat information; review before sharing."),
                text("Nothing is uploaded. Retention: 14 days or 100 MiB; active sessions keep their current log."),
                row![button("Cancel").on_press(Msg::CancelClose), button("Save ZIP…").on_press(Msg::SaveLogs)].spacing(14)].spacing(24).padding(30).max_width(860)).center(Fill).into();
        }
        if let Some(observed) = &self.observed {
            let compatible = observed.info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI);
            return container(column![text("Confirm server identity").size(28), text(&self.address),
                text(format!("Server reports {} · protocol {} · DLL interface {}", observed.info.release, observed.info.protocol, observed.info.mod_abi)),
                text("This is a first-use identity, not independently verified. Compare the full code with the host or import their private invitation."),
                text(&observed.identity).font(iced::Font::MONOSPACE).size(16),
                text(compatible.as_ref().map(|_| "Compatible with this application.".to_string()).unwrap_or_else(|e| e.to_string())),
                row![button("Cancel").on_press(Msg::CancelClose), button("Trust this identity").on_press_maybe(compatible.is_ok().then_some(Msg::TrustIdentity))].spacing(14)].spacing(24).padding(30).max_width(860)).center(Fill).into();
        }
        let mut navigation = column![text("YO3").size(38), text("REVIVAL").size(17), text("Play. Host. Reconnect.").size(12), widget::rule::horizontal(1)].spacing(14);
        for (label, page) in [("Play", Page::Play), ("My server", Page::Server), ("Accounts", Page::Accounts), ("Activity", Page::Activity), ("Settings", Page::Settings)] {
            navigation = navigation.push(button(text(label)).width(Fill).padding([12, 16]).on_press(Msg::Page(page)).style(if self.page == page { button::primary } else { button::text }));
        }
        navigation = navigation.push(widget::Space::new().height(Fill)).push(button("Setup guide").on_press(Msg::Help).style(button::text))
            .push(text(format!("YO3 Revival {}\nYu-Gi-Oh! ONLINE 3 · 1.153", yo3_tunnel::BUILD_VERSION)).size(11));
        let content = match self.page { Page::Play => self.play_view(), Page::Server => self.server_view(), Page::Accounts => self.accounts_view(), Page::Activity => self.activity_view(), Page::Settings => self.settings_view() };
        let notice = container(text(&self.notice).size(13).color(if self.failed { Color::from_rgb8(240, 143, 148) } else { Color::from_rgb8(166, 187, 210) })).padding([12, 16]).style(container::rounded_box).width(Fill);
        let main = column![content, notice].spacing(18).padding(28).width(Fill);
        if self.confirm_certificate {
            return container(column![text("Update the server certificate?").size(28), text("Save the new address first. The account data and private key stay intact, and the previous certificate is backed up. Export updated invitations for friends afterward."),
                row![button("Cancel").on_press(Msg::CancelClose), button("Update certificate").on_press(Msg::RenewCertificate)].spacing(12)].spacing(24).padding(30)).center(Fill).into();
        }
        if self.closing {
            let dialog = column![text("Close YO3 Revival?").size(28), text("The game will close. Choose what happens to your hosted server."),
                row![button("Keep open").on_press(Msg::CancelClose), button("Keep hosting & exit").on_press_maybe((!self.busy).then_some(Msg::Quit(false))),
                    button("Stop server & exit").on_press_maybe((!self.busy).then_some(Msg::Quit(true))).style(button::danger)].spacing(12)].spacing(24).padding(30);
            return container(dialog).center(Fill).into();
        }
        row![container(navigation).width(Fixed(198.0)).padding(22).height(Fill).style(container::rounded_box), main].into()
    }
    fn play_view(&self) -> Element<'_, Msg> {
        let mut servers = vec![ServerChoice::Local]; servers.extend(self.servers.iter().map(ServerChoice::saved));
        let choice_key = match self.selected { ServerChoice::Local => 0, ServerChoice::Saved { id, .. } => id };
        let chooser = widget::keyed_column([(choice_key, pick_list(servers, Some(self.selected.clone()), Msg::Select).width(Fill).padding(12).into())]).width(Fill);
        let mut body = column![text("Your next duel starts here").size(30), text("Choose your own server or connect to a friend's address."),
            row![chooser, button("Add by address").padding(12).on_press(Msg::AddServer).style(button::secondary), button("Import invitation").padding(12).on_press(Msg::Import).style(button::secondary)].spacing(12),
            text(if self.selected == ServerChoice::Local { "Your server starts automatically when you press Play. Manage hosting in My server." } else { "This connection uses the saved account credential and host certificate." }).size(13)].spacing(20);
        if self.editor {
            body = body.push(container(column![text("Add a connection").size(20),
                input("Name", &self.label, Field::Label), input("Server address — hostname or IP, optional :port", &self.address, Field::Address),
                text_input("Account access key from the host", &self.key).secure(true).on_input(|v| Msg::Input(Field::Key, v)).padding(12),
                row![button(if self.identity.is_some() { "Check server again" } else { "Discover server" }).on_press(Msg::Discover), button("Save connection").on_press(Msg::SaveServer), button("Advanced…").on_press(Msg::AdvancedConnection)].spacing(12),
                text(if self.identity.is_some() { "Server identity approved. Invitations also include the identity and account key." } else { "Discover the server before sending credentials. The default port is 7443." }).size(12)].spacing(12)).padding(20).style(container::rounded_box));
            if self.advanced_connection { body = body.push(button("Import a trusted certificate manually").on_press(Msg::Certificate).style(button::secondary)); }
        }
        if let Some(info) = &self.checked { body = body.push(text(format!("Server {} · protocol {} · DLL interface {} · {}", info.release, info.protocol, info.mod_abi, if info.ready { "ready" } else { "rules engine unavailable" })).size(13)); }
        if self.selected != ServerChoice::Local { body = body.push(button("Test connection").on_press_maybe((!self.busy).then_some(Msg::TestConnection)).style(button::secondary)); }
        if self.selected == ServerChoice::Local && !account_names(&self.accounts).is_empty() { body = body.push(pick_list(account_names(&self.accounts), (!self.account.is_empty()).then_some(self.account.clone()), Msg::AccountSelect).placeholder("Choose your player account")); }
        body = body.push(widget::rule::horizontal(1)).push(row![button(text(if self.game_running { "Game is running" } else if self.busy { "Please wait…" } else { "Play" }).size(20)).width(Fixed(246.0)).padding([14, 46]).on_press_maybe((!self.busy && !self.game_running && self.backend.is_some()).then_some(Msg::Play)),
            button("Close game").padding(14).on_press_maybe(self.game_running.then_some(Msg::StopGame)).style(button::secondary)].spacing(14));
        scrollable(body.width(Fill)).spacing(16).width(Fill).height(Fill).into()
    }
    fn server_view(&self) -> Element<'_, Msg> {
        let online = self.status.as_ref().and_then(|v| v["online"].as_u64()).unwrap_or(0);
        let duels = self.status.as_ref().and_then(|v| v["duels"].as_u64()).unwrap_or(0);
        let listening = self.status.as_ref().and_then(|v| v["listen"].as_str()).map(str::to_string)
            .unwrap_or_else(|| format!("{}:{}", self.settings.bind_address, self.settings.listen_port));
        let friends = yo3_desktop::network::advertised(&self.settings).map(|v| v.0).unwrap_or_else(|_| self.settings.host_address.clone());
        let mut body = column![text("My server").size(30), text("Hosting is independent of the server selected on Play."),
            container(column![text(if self.status.is_some() { "RUNNING" } else { "STOPPED" }).size(14), text(format!("{} {listening}", if self.status.is_some() { "Listening on" } else { "Will listen on" })).size(24),
                text(format!("{online} players online · {duels} active duels")), text(format!("Friends use {friends} · one public TCP port")).size(13)].spacing(12)).padding(22).width(Fill).style(container::rounded_box),
            row![button("Start server").padding(12).on_press_maybe((!self.busy && self.status.is_none()).then_some(Msg::Start)),
                button("Stop server").padding(12).on_press_maybe((!self.busy && self.status.is_some()).then_some(Msg::Stop)).style(button::danger),
                button("View activity").padding(12).on_press(Msg::Page(Page::Activity)).style(button::secondary)].spacing(12),
            text("Stopping the server disconnects its players. Closing your game keeps hosting active.").size(13),
            row![button("Advanced networking…").on_press(Msg::AdvancedNetwork).style(button::secondary), button("Allow through firewall…").on_press(Msg::Firewall).style(button::secondary)].spacing(12)].spacing(22);
        if self.advanced_network {
            if self.status.is_none() {
                let resolved = yo3_desktop::network::resolve_bind(&self.settings, &self.bindings).ok();
                let selected = self.bindings.iter().find(|b| b.interface_id == self.settings.bind_interface && Some(b.address) == resolved).cloned();
                body = body.push(text("Listen on")).push(pick_list(self.bindings.clone(), selected, Msg::BindSelect).placeholder("Saved interface unavailable — refresh and select").width(Fill))
                    .push(row![text("TCP port"), text_input("7443", &self.port).on_input(|v| Msg::Input(Field::Port, v)).padding(12).width(Fixed(120.0)), button("Refresh interfaces").on_press(Msg::RefreshBindings)].spacing(12))
                    .push(input("Address friends use (hostname/IP, optional external :port)", &self.draft.host_address, Field::Host))
                    .push(text("Start server saves these networking choices. 0.0.0.0 listens on all IPv4 interfaces; friends need an actual reachable address. An explicit address port supports router/VPN port mapping.").size(12))
                    .push(button("Reissue certificate for saved address").on_press(Msg::ConfirmCertificate).style(button::secondary));
            } else { body = body.push(text("Stop your server to change its listening interface or port. The address above reports its active listener.").size(13)); }
        }
        if let Some(status) = &self.status {
            if let Some(problem) = status["engine_problem"].as_str() { body = body.push(text(problem).color(Color::from_rgb8(240, 143, 148))); }
            if let Some(players) = status["players"].as_array() { for p in players { body = body.push(text(format!("{}  ·  {}", p["name"].as_str().unwrap_or("Player"), if p["duelling"].as_bool().unwrap_or(false) { "Duelling" } else { "Online" }))); } }
        }
        scrollable(body.width(Fill)).spacing(16).width(Fill).height(Fill).into()
    }
    fn accounts_view(&self) -> Element<'_, Msg> {
        let mut body = column![text("Accounts on my server").size(30), text("Create accounts and share private invitations while your server keeps running."),
            row![input("New account's first player name", &self.new_account, Field::Account), button("Create account").padding(12).on_press_maybe((!self.busy && self.status.is_some() && !self.new_account.trim().is_empty()).then_some(Msg::CreateAccount))].spacing(12),
            pick_list(account_names(&self.accounts), (!self.account.is_empty()).then_some(self.account.clone()), Msg::AccountSelect).placeholder("Select an account"),
            row![input("Additional player name", &self.new_player, Field::NewPlayer), button("Add player").padding(12).on_press_maybe((!self.busy && !self.account.is_empty() && !self.new_player.trim().is_empty()).then_some(Msg::AddPlayer))].spacing(12),
            button("Export private invitation").padding(12).on_press_maybe((!self.busy && !self.account.is_empty()).then_some(Msg::Export)),
            text("Invitations include an account key. Give each friend their own account, and send its invitation privately.").size(13), widget::rule::horizontal(1)].spacing(20);
        if let Some(accounts) = self.accounts.as_array() { for a in accounts {
            let names = a["players"].as_array().map(|p| p.iter().filter_map(|p| p["name"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            body = body.push(container(row![text(names).width(Fill), text(format!("{} BP", a["bp"]))].spacing(16)).padding(14).width(Fill).style(container::rounded_box));
        } }
        scrollable(body.width(Fill)).spacing(16).width(Fill).height(Fill).into()
    }
    fn activity_view(&self) -> Element<'_, Msg> {
        let filter = self.filter.to_lowercase();
        let shown = if filter.is_empty() { self.logs.clone() } else { self.logs.lines().filter(|line| line.to_lowercase().contains(&filter)).collect::<Vec<_>>().join("\n") };
        column![text("Activity").size(30), row![button("Server").on_press(Msg::Logs(true)), button("Launcher & game").on_press(Msg::Logs(false)), checkbox(self.paused).label("Pause updates").on_toggle(Msg::Pause), button("Copy").on_press(Msg::CopyLogs)].spacing(12),
            row![button("Export current session…").on_press(Msg::PrepareReport(false)), button("Export all retained…").on_press(Msg::PrepareReport(true))].spacing(12),
            text_input("Filter activity…", &self.filter).on_input(Msg::LogFilter).padding(10),
            text(if self.log_server { "Your local server's activity" } else { "Game startup and connection diagnostics" }).size(13),
            container(scrollable(text(if shown.is_empty() { "No matching activity.".to_string() } else { shown }).font(iced::Font::MONOSPACE).size(12)).id("activity-log").height(Fill)).padding(16).width(Fill).height(Fill).style(container::rounded_box)].spacing(18).into()
    }
    fn settings_view(&self) -> Element<'_, Msg> {
        let mut body = column![text("Make yourself at home").size(30), text("Choose a complete game installation. We keep the game executable and INI unchanged."),
            text("Game folder").size(15), row![input("Folder containing yo3.exe", &self.draft.game_dir, Field::Game), button("Browse…").padding(12).on_press(Msg::Browse(Field::Game))].spacing(12)].spacing(16);
        if !cfg!(windows) { body = body.push(text("Wine prefix (optional)")).push(row![input("Default Wine prefix", &self.draft.wine_prefix, Field::Wine), button("Browse…").padding(12).on_press(Msg::Browse(Field::Wine))].spacing(12)); }
        body = body.push(text("Direct start").size(22))
            .push(text("After saving settings, install the optional automatic loader to open yo3.exe directly with your selected server/account. Setup may request permission for a protected game folder. An existing foreign WINMM mod is never replaced.").size(13))
            .push(row![button("Enable direct start").on_press_maybe((!self.busy && !self.game_running).then_some(Msg::Direct(true))), button("Disable direct start").on_press_maybe((!self.busy && !self.game_running).then_some(Msg::Direct(false)))].spacing(12))
            .push(widget::rule::horizontal(1)).push(text("Diagnostics").size(22))
            .push(checkbox(self.draft.detailed_logs).label("Detailed client and protocol tracing (next game/server start)").on_toggle(Msg::Detailed))
            .push(text("Normally only operational logs are kept. Detailed tracing can include player/chat data; use it when investigating a problem, then turn it off. Export ZIP reports from Activity.").size(13))
            .push(widget::rule::horizontal(1)).push(text("Hosting").size(22)).push(text("Choose the listening interface and port in My server → Advanced networking.").size(13))
            .push(input("Address friends will reach — e.g. 192.168.1.50:7443", &self.draft.host_address, Field::Host))
            .push(input("Your first player's name", &self.draft.player_name, Field::Player))
            .push(row![input("Server data folder (empty uses application storage)", &self.draft.data_dir, Field::Data), button("Browse…").padding(12).on_press(Msg::Browse(Field::Data))].spacing(12))
            .push(text("Server identities are managed by the application. Existing data folders are imported into SQLite once, with the originals retained.").size(12))
            .push(row![button("Save settings").padding(12).on_press_maybe((!self.busy).then_some(Msg::SaveSettings)), button("Open application data").padding(12).on_press(Msg::OpenData)].spacing(12));
        scrollable(body.width(Fill)).spacing(16).width(Fill).height(Fill).into()
    }
}
fn input(value_hint: &str, value: &str, field: Field) -> Element<'static, Msg> {
    text_input(value_hint, value).on_input(move |v| Msg::Input(field, v)).padding(12).into()
}
fn account_names(accounts: &Value) -> Vec<String> { accounts.as_array().into_iter().flatten().filter_map(|a| a["players"][0]["name"].as_str().map(str::to_string)).collect() }
fn open(path: &str) -> anyhow::Result<()> {
    #[cfg(windows)] let mut cmd = { let mut c = std::process::Command::new("explorer.exe"); c.arg(path); c };
    #[cfg(not(windows))] let mut cmd = { let mut c = std::process::Command::new("xdg-open"); c.arg(path); c };
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?;
    Ok(())
}
