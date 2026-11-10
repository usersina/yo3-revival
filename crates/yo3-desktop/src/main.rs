#![cfg_attr(windows, windows_subsystem = "windows")]
use std::sync::Arc;
use std::path::PathBuf;
use iced::{Element, Task, Subscription, Length::{Fill, Fixed}, Color, Theme};
use iced::widget::{self, button, checkbox, container, pick_list, scrollable, text, text_input};
use yo3_desktop::{Backend, LauncherDb, Settings, SavedServer, ServerChoice};
use yo3_server::admin::ManagerOp;
use serde_json::Value;
mod views;
mod ui;
use yo3_desktop::{updates, preferences::Section};

fn main() -> iced::Result {
    if let Some(code) = yo3_desktop::privileged::helper_mode() { std::process::exit(code); }
    let root = yo3_desktop::default_root();
    let startup = match updates::activation::Startup::parse(&root, std::env::args().skip(1)) {
        Ok(startup) => startup, Err(error) => { eprintln!("{error:#}"); std::process::exit(1); }
    };
    let mut routing_error = None;
    if startup.readiness.is_none() && root.is_dir() {
        let routed = (|| -> anyhow::Result<bool> {
            let Some(target) = updates::activation::target(&root, yo3_tunnel::BUILD_VERSION, &std::env::current_exe()?)? else { return Ok(false) };
            updates::activation::launch(&root, &target, false, std::time::Duration::from_secs(25))?; Ok(true)
        })();
        match routed { Ok(true) => return Ok(()), Ok(false) => {}, Err(error) => routing_error = Some(format!("Could not open the installed update. Using this version: {error:#}")) }
    }
    iced::application(move || App::boot(startup.clone(), routing_error.clone()), App::update, App::view).title("YO3 Revival").default_font(iced::Font::with_name("Fira Sans"))
        .theme(|_: &App| Theme::custom("Revival", iced::theme::Palette { background: Color::from_rgb8(15, 20, 29),
            text: Color::from_rgb8(229, 235, 244), primary: Color::from_rgb8(75, 155, 224),
            success: Color::from_rgb8(89, 189, 151), warning: Color::from_rgb8(224, 180, 87), danger: Color::from_rgb8(222, 107, 114) }))
        .window(iced::window::Settings { size: iced::Size::new(1080.0, 760.0), min_size: Some(iced::Size::new(900.0, 650.0)), ..Default::default() }).exit_on_close_request(false)
        .subscription(|_| Subscription::batch([iced::time::every(std::time::Duration::from_secs(2)).map(|_| Msg::Tick), iced::window::close_requests().map(|_| Msg::Close), iced::window::open_events().map(|_| Msg::WindowOpened)]))
        .run()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page { Play, Server, Activity, Settings, About }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostTab { Overview, Invitations, Advanced }
#[derive(Clone, Copy)]
enum Disclosure { Game, Diagnostics, Account }
#[derive(Clone)]
enum UpdateStatus { Checking, Checked(updates::Decision), Failed(String) }
#[derive(Clone, Copy)]
enum Field { Game, Wine, Host, Data, Player, Label, Address, Key, Account, NewPlayer, Port }
#[derive(Clone)]
enum Msg {
    Page(Page), HostTab(HostTab), Input(Field, String), Select(ServerChoice), AccountSelect(String), AdminAccountSelect(String),
    Start, Stop, Play, StopGame, TestConnection, Discover, TrustIdentity, RefreshBindings, BindSelect(yo3_desktop::network::BindAddress), Direct(bool),
    SaveSettings(Section), CancelSettings(Section), Browse(Field), Import, AddServer, SaveServer, EditServer, RemoveServer, ConfirmRemoveServer,
    CreateAccount, AddPlayer, Export, Tick, Poll(Result<LiveStatus, String>), Done(Result<Outcome, String>),
    Logs(bool), Pause(bool), LogFilter(String), CopyLogs, ExportLogs, PrepareReport(bool), SaveLogs, Detailed(bool), OpenData, Help, NetworkHelp,
    Close, CancelClose, Quit(bool), Disclosure(Disclosure), NewAccount, GitHub, ExternalOpened(Result<(), String>), DismissNotice,
    CheckSetup, SetupChecked(Settings, yo3_desktop::Readiness, yo3_desktop::direct::State), RepairSetup,
    CheckUpdates, UpdateChecked(Result<updates::Decision, String>), ReleasePage, CopyAddress, CopyVersion,
    ReviewUpdate, ConfirmUpdate, CancelUpdate, UpdatePrepared(Result<updates::install::Installed, String>), UpdateApplied(Result<(), String>), WindowOpened,
}
impl std::fmt::Debug for Msg { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("DesktopEvent") } }
#[derive(Clone)]
enum Outcome {
    Info(String), Status(Value), GameStarted, Accounts(Value), AccountsChanged(Value, String, Option<String>), Path(Field, String), Imported(SavedServer),
    Observed(yo3_tunnel::ObservedServer), Checked(yo3_tunnel::ServerInfo), Bindings(Vec<yo3_desktop::network::BindAddress>),
    SettingsSaved(Section, Settings), SelectionSaved(Settings), ConnectionsSaved(Vec<SavedServer>, i64), ConnectionsRemoved(Vec<SavedServer>, Settings),
    Report(yo3_desktop::reports::Report), Quit,
}
#[derive(Clone)]
struct LiveStatus { server: Option<Value>, game_running: bool, game_ready: bool, logs: String, failure: Option<String> }
struct App {
    root: PathBuf, backend: Option<Arc<Backend>>, settings: Settings, draft: Settings, page: Page, host_tab: HostTab,
    servers: Vec<SavedServer>, selected: ServerChoice, status: Option<Value>, accounts: Value, account: String, admin_account: String,
    new_account: String, new_player: String, editor: bool, editing_id: Option<i64>, remove_server: Option<i64>,
    label: String, address: String, key: String, certificate: String, server_name: String,
    busy: bool, polling: bool, game_running: bool, game_ready: bool, notice: String, failed: bool, feedback_page: Page,
    notice_expires: Option<std::time::Instant>, operation_page: Page, operation_label: String,
    log_server: bool, paused: bool, logs: String, filter: String, closing: bool,
    identity: Option<String>, observed: Option<yo3_tunnel::ObservedServer>, checked: Option<yo3_tunnel::ServerInfo>,
    bindings: Vec<yo3_desktop::network::BindAddress>, port: String, report: Option<yo3_desktop::reports::Report>, report_options: bool, report_all: bool,
    advanced_game: bool, advanced_diagnostics: bool, account_options: bool, account_form: bool,
    setup_result: Option<yo3_desktop::Readiness>, direct_state: yo3_desktop::direct::State, repair_setup: bool, first_setup: bool,
    update_status: UpdateStatus, review_update: bool, transfer: Option<(updates::Release, updates::download::Transfer)>,
    staged: Option<updates::install::Installed>, applying: bool, startup: updates::activation::Startup,
}
fn job(f: impl FnOnce() -> anyhow::Result<Outcome> + Send + 'static) -> Task<Msg> {
    Task::perform(async move { tokio::task::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:#}")) }, Msg::Done)
}
impl App {
    fn boot(startup: updates::activation::Startup, routing_error: Option<String>) -> (Self, Task<Msg>) {
        let mut app = Self::load(yo3_desktop::default_root()); app.startup = startup;
        if let Some(error) = routing_error { app.feedback(Err(error)); }
        let status = app.update(Msg::Tick); let setup = app.inspect_setup();
        (app, Task::batch([status, setup, check_updates()]))
    }
    fn load(root: PathBuf) -> Self {
        let mut error = String::new(); let db = LauncherDb::open(&root).map_err(|e| error = e.to_string()).ok();
        let settings = db.as_ref().and_then(|d| d.settings().map_err(|e| error = e.to_string()).ok()).unwrap_or_default();
        let servers = db.as_ref().and_then(|d| d.servers().map_err(|e| error = e.to_string()).ok()).unwrap_or_default();
        let selected = settings.selected_server.and_then(|id| servers.iter().find(|s| s.id == id)).map(ServerChoice::saved).unwrap_or(ServerChoice::Local);
        let account = settings.local_account.clone(); let port = settings.listen_port.to_string();
        let backend = if error.is_empty() { Backend::new(root.clone()).map_err(|e| error = e.to_string()).ok() } else { None };
        let first_setup = !std::path::Path::new(&settings.game_dir).join("yo3.exe").is_file(); let page = if first_setup { Page::Settings } else { Page::Play };
        Self { root, backend, draft: settings.clone(), settings, page, host_tab: HostTab::Overview, servers, selected, status: None, accounts: Value::Null,
            account, admin_account: String::new(), new_account: String::new(), new_player: String::new(), editor: false, editing_id: None, remove_server: None,
            label: String::new(), address: String::new(), key: String::new(), certificate: String::new(), server_name: String::new(),
            busy: false, polling: false, game_running: false, game_ready: false, failed: !error.is_empty(), notice: error, feedback_page: page,
            notice_expires: None, operation_page: page, operation_label: String::new(), log_server: true, paused: false, logs: String::new(), filter: String::new(), closing: false,
            identity: None, observed: None, checked: None, bindings: yo3_desktop::network::defaults(), port, report: None, report_options: false, report_all: false,
            advanced_game: false, advanced_diagnostics: false, account_options: false, account_form: false, setup_result: None, direct_state: yo3_desktop::direct::State::Disabled, repair_setup: false, first_setup,
            update_status: UpdateStatus::Checking, review_update: false, transfer: None, staged: None, applying: false, startup: Default::default() }
    }
    fn update(&mut self, message: Msg) -> Task<Msg> {
        let background = matches!(message, Msg::Done(_) | Msg::Poll(_) | Msg::Tick | Msg::UpdateChecked(_) | Msg::ExternalOpened(_) | Msg::SetupChecked(..) | Msg::UpdatePrepared(_) | Msg::UpdateApplied(_) | Msg::WindowOpened);
        if self.modal_open() && !background && !matches!(message, Msg::CancelClose | Msg::Close | Msg::Quit(_) | Msg::TrustIdentity | Msg::SaveLogs | Msg::PrepareReport(_) | Msg::ConfirmUpdate | Msg::CancelUpdate | Msg::ConfirmRemoveServer | Msg::CheckSetup | Msg::Browse(Field::Game) | Msg::ReleasePage) { return Task::none() }
        if self.locked() && !background && !matches!(message, Msg::Close | Msg::CancelClose | Msg::Page(_) | Msg::HostTab(_) | Msg::GitHub | Msg::Help | Msg::NetworkHelp | Msg::OpenData | Msg::ReleasePage | Msg::CheckUpdates | Msg::CopyVersion | Msg::Logs(_) | Msg::Pause(_) | Msg::LogFilter(_) | Msg::CopyLogs | Msg::Disclosure(_) | Msg::CancelUpdate | Msg::Stop | Msg::StopGame | Msg::DismissNotice) { return Task::none() }
        match message {
            Msg::WindowOpened => if self.backend.is_some() && let Some(readiness) = self.startup.readiness.take() {
                if let Err(error) = readiness.write(yo3_tunnel::BUILD_VERSION) { self.feedback(Err(format!("Update readiness failed: {error:#}"))); return Task::none() }
                if self.startup.resume_hosting { return self.update(Msg::Start) }
            },
            Msg::CheckSetup => { self.repair_setup = false; return self.inspect_setup() }
            Msg::SetupChecked(settings, result, direct) => {
                if settings != self.settings { return self.inspect_setup() }
                self.setup_result = Some(result); self.direct_state = direct;
            }
            Msg::RepairSetup => self.repair_setup = true,
            Msg::ReviewUpdate => if matches!(self.update_status, UpdateStatus::Checked(updates::Decision::Available(_))) { self.review_update = true; },
            Msg::ConfirmUpdate => {
                if !self.review_update || self.locked() { return Task::none() }
                let UpdateStatus::Checked(updates::Decision::Available(release)) = &self.update_status else { return Task::none() };
                let release = release.clone(); let control = updates::download::Transfer::default(); self.review_update = false; self.transfer = Some((release.clone(), control.clone())); let root = self.root.clone();
                return Task::perform(async move { updates::download::prepare(root, release, control).await.map_err(|e| format!("{e:#}")) }, Msg::UpdatePrepared);
            }
            Msg::CancelUpdate => if let Some((_, transfer)) = &self.transfer { transfer.cancel(); },
            Msg::UpdatePrepared(result) => {
                let cancelled = self.transfer.as_ref().is_some_and(|(_, t)| t.cancelled()); self.transfer = None;
                match result { Ok(installed) if !cancelled => self.staged = Some(installed), Ok(_) => self.feedback_at(Page::About, Ok("Update cancelled.".into())), Err(error) => self.feedback_at(Page::About, if cancelled { Ok("Update cancelled.".into()) } else { Err(error) }) }
                return self.apply_update();
            }
            Msg::UpdateApplied(result) => { self.applying = false; match result { Ok(()) => return iced::exit(), Err(error) => self.feedback_at(Page::About, Err(error)) } }
            Msg::GitHub => return open_external(updates::REPOSITORY.into()),
            Msg::Help => return open_external("https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md".into()),
            Msg::NetworkHelp => return open_external("https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md#connection-help".into()),
            Msg::OpenData => return open_external(self.root.to_string_lossy().into_owned()),
            Msg::ExternalOpened(result) => if let Err(error) = result { self.feedback(Err(error)); },
            Msg::CopyVersion => return iced::clipboard::write(format!("YO3 Revival {} ({}) · {}", yo3_tunnel::BUILD_VERSION, yo3_tunnel::BUILD_COMMIT, std::env::consts::OS)),
            Msg::CopyAddress => if let Ok(Some((address, _))) = yo3_desktop::network::share_address(&self.settings) { return iced::clipboard::write(address) },
            Msg::CheckUpdates => if !matches!(self.update_status, UpdateStatus::Checking) { self.update_status = UpdateStatus::Checking; return check_updates() },
            Msg::UpdateChecked(result) => self.update_status = match result { Ok(result) => UpdateStatus::Checked(result), Err(error) => UpdateStatus::Failed(error) },
            Msg::ReleasePage => return open_external(match &self.update_status { UpdateStatus::Checked(updates::Decision::Available(release)) => release.url.clone(), _ => updates::RELEASES.into() }),
            Msg::DismissNotice => { self.notice.clear(); self.notice_expires = None; }
            Msg::Disclosure(section) => match section { Disclosure::Game => self.advanced_game = !self.advanced_game, Disclosure::Diagnostics => self.advanced_diagnostics = !self.advanced_diagnostics, Disclosure::Account => self.account_options = !self.account_options },
            Msg::NewAccount => self.account_form = !self.account_form,
            Msg::Page(page) => { self.page = page; if page == Page::Play { self.first_setup = false; } }
            Msg::HostTab(tab) => {
                self.host_tab = tab;
                if !self.locked() {
                    if tab == HostTab::Invitations && self.status.is_some() { return self.load_accounts() }
                    if tab == HostTab::Advanced { return self.update(Msg::RefreshBindings) }
                }
            }
            Msg::Input(field, value) => match field {
                Field::Game => self.draft.game_dir = value, Field::Wine => self.draft.wine_prefix = value, Field::Host => self.draft.host_address = value,
                Field::Data => self.draft.data_dir = value, Field::Player => self.draft.player_name = value, Field::Label => self.label = value,
                Field::Address => if self.address != value { self.address = value; self.server_name.clear(); self.identity = None; self.certificate.clear(); self.observed = None; self.checked = None; },
                Field::Key => self.key = value, Field::Account => self.new_account = value, Field::NewPlayer => self.new_player = value, Field::Port => self.port = value,
            },
            Msg::Detailed(value) => self.draft.detailed_logs = value,
            Msg::RefreshBindings => { self.begin("Reading network interfaces…"); return job(|| Ok(Outcome::Bindings(yo3_desktop::network::bindings()?))) }
            Msg::BindSelect(binding) => { self.draft.bind_address = binding.address.to_string(); self.draft.bind_interface = binding.interface_id; }
            Msg::Select(value) => {
                if self.game_running { self.feedback(Err("Close the game before changing its connection.".into())); return Task::none() }
                self.settings.selected_server = match &value { ServerChoice::Local => None, ServerChoice::Saved { id, .. } => Some(*id) };
                self.checked = None; self.reset_editor(); self.selected = value; return self.save_selection();
            }
            Msg::AccountSelect(value) => {
                if self.game_running { self.feedback(Err("Close the game before changing its account.".into())); return Task::none() }
                self.settings.local_account = value.clone(); self.account = value; return self.save_selection();
            }
            Msg::AdminAccountSelect(value) => self.admin_account = value,
            Msg::AddServer => { let open = !self.editor; self.reset_editor(); self.editor = open; }
            Msg::EditServer => {
                if self.game_running { self.feedback(Err("Close the game before editing its connection.".into())); return Task::none() }
                if let Ok(Some(server)) = self.selected.remote(&self.servers) {
                    self.editing_id = Some(server.id); self.label = server.label; self.address = server.address; self.key = server.key;
                    self.identity = server.identity.or_else(|| yo3_tunnel::identity_from_pem(server.certificate.as_bytes()).ok()); self.certificate = server.certificate; self.server_name = server.server_name;
                    self.editor = true;
                }
            }
            Msg::RemoveServer => if !self.game_running && let ServerChoice::Saved { id, .. } = self.selected { self.remove_server = Some(id); },
            Msg::ConfirmRemoveServer => {
                let Some(id) = self.remove_server.take() else { return Task::none() };
                let root = self.root.clone(); let backend = self.backend.clone(); self.begin("Removing saved connection…");
                return job(move || { let mut db = LauncherDb::open(&root)?; db.delete_server(id)?; let settings = db.settings()?;
                    if let Some(backend) = backend { let _ = backend.sync_direct(&settings, None); }
                    for ext in ["json", "crt"] { let _ = std::fs::remove_file(root.join("profiles").join(format!("connection-{id}.{ext}"))); }
                    Ok(Outcome::ConnectionsRemoved(db.servers()?, settings)) });
            }
            Msg::Discover => { let address = self.address.clone(); self.begin("Checking the server…"); return job(move || Ok(Outcome::Observed(Backend::discover(&address)?))) }
            Msg::TrustIdentity => if let Some(observed) = self.observed.take() {
                if let Err(error) = observed.info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI) { self.feedback(Err(error.to_string())); return Task::none() }
                self.identity = Some(observed.identity); self.certificate = observed.certificate; self.checked = Some(observed.info);
            },
            Msg::Start | Msg::Stop | Msg::Play | Msg::StopGame => {
                if self.busy || self.applying { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); let who = self.account.clone();
                match message {
                    Msg::Start => { self.begin("Starting your server…"); return job(move || Ok(Outcome::Status(backend.start_server(&settings)?))) }
                    Msg::Stop => { self.begin("Stopping your server…"); return job(move || { backend.stop_server(&settings)?; Ok(Outcome::Info("Server is stopping.".into())) }) }
                    Msg::StopGame => { self.begin("Closing the game…"); return job(move || { backend.stop_game()?; Ok(Outcome::Info("Closing the game…".into())) }) }
                    _ => {
                        let remote = match self.selected.remote(&self.servers) { Ok(remote) => remote, Err(e) => { self.feedback(Err(e.to_string())); return Task::none() } };
                        self.begin("Checking setup and starting the game…");
                        return job(move || { backend.play(&settings, remote, &who)?; Ok(Outcome::GameStarted) });
                    }
                }
            }
            Msg::SaveSettings(section) => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let mut draft = self.draft.clone();
                if section == Section::Hosting { draft.listen_port = match self.port.parse() { Ok(port) if port > 0 => port, _ => { self.feedback(Err("Port must be between 1 and 65535.".into())); return Task::none() } }; }
                self.begin("Saving changes…"); return job(move || Ok(Outcome::SettingsSaved(section, backend.save_preferences(section, &draft)?)));
            }
            Msg::CancelSettings(section) => { self.draft = section.merge(&self.draft, &self.settings); if section == Section::Hosting { self.port = self.settings.listen_port.to_string(); } }
            Msg::Browse(field) => {
                self.repair_setup = false; self.begin("Choosing a folder…");
                return Task::perform(async move { rfd::AsyncFileDialog::new().set_title("Choose folder").pick_folder().await.map(|f| f.path().to_string_lossy().into_owned()) }, move |path| Msg::Done(Ok(path.map_or(Outcome::Info("Selection cancelled.".into()), |p| Outcome::Path(field, p)))));
            }
            Msg::Import => {
                self.begin("Importing an invitation…");
                return Task::perform(async { rfd::AsyncFileDialog::new().set_title("Import server invitation").add_filter("Invitation", &["json"]).pick_file().await.map(|f| f.path().to_path_buf()) }, |path| Msg::Done(match path { Some(path) => SavedServer::import(&path).map(Outcome::Imported).map_err(|e| e.to_string()), None => Ok(Outcome::Info("Import cancelled.".into())) }));
            }
            Msg::SaveServer => {
                let result = (|| -> anyhow::Result<SavedServer> {
                    let (address, name) = yo3_desktop::server_address(&self.address)?; anyhow::ensure!(self.identity.is_some(), "Check and approve the server identity first.");
                    let server = SavedServer { id: self.editing_id.unwrap_or(0), label: if self.label.trim().is_empty() { name.clone() } else { self.label.trim().into() }, address,
                        server_name: if self.server_name.is_empty() { name } else { self.server_name.clone() }, certificate: self.certificate.clone(), identity: self.identity.clone(), key: self.key.trim().into() };
                    server.validate()?; Ok(server)
                })();
                match result { Ok(server) => { let root = self.root.clone(); self.begin("Saving connection…"); return job(move || { let db = LauncherDb::open(&root)?; let id = db.save_server(&server)?; Ok(Outcome::ConnectionsSaved(db.servers()?, id)) }); }, Err(e) => self.feedback(Err(e.to_string())) }
            }
            Msg::TestConnection => {
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let selected = match self.selected.remote(&self.servers) { Ok(Some(s)) => s, _ => return Task::none() };
                self.begin("Testing connection and account…"); return job(move || Ok(Outcome::Checked(backend.check_connection(&selected)?)));
            }
            Msg::CreateAccount | Msg::AddPlayer => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); let creating = matches!(message, Msg::CreateAccount); let name = self.new_account.trim().to_string();
                let op = if creating { ManagerOp::CreateAccount { name: name.clone() } } else { ManagerOp::AddPlayer { who: self.admin_account.clone(), name: self.new_player.trim().into() } };
                self.begin(if creating { "Creating account…" } else { "Adding player…" });
                return job(move || { backend.manage(&settings, op)?; Ok(Outcome::AccountsChanged(backend.accounts(&settings)?, if creating { "Account created. Export its private invitation." } else { "Player added." }.into(), creating.then_some(name))) });
            }
            Msg::Export => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); let who = self.admin_account.clone(); self.begin("Exporting private invitation…");
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().set_title("Export private invitation").set_file_name("yo3-invitation.json").save_file().await else { return Ok(Outcome::Info("Export cancelled.".into())) };
                    tokio::task::spawn_blocking(move || { backend.invitation(&settings, &who)?.export(file.path())?; Ok(Outcome::Info("Private invitation saved. Send it only to this account's owner.".into())) }).await.map_err(|e| e.to_string())?.map_err(|e: anyhow::Error| e.to_string())
                }, Msg::Done);
            }
            Msg::Direct(enable) => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone();
                let remote = match self.selected.remote(&self.servers) { Ok(s) => s, Err(e) => { self.feedback(Err(e.to_string())); return Task::none() } };
                self.begin(if enable { "Setting up direct launch…" } else { "Disabling direct launch…" });
                return job(move || { if enable { backend.enable_direct(&settings, remote)?; } else { backend.disable_direct(&settings)?; }
                    Ok(Outcome::Info(if enable { "Direct launch is ready for your saved server and account." } else { "Direct launch disabled." }.into())) });
            }
            Msg::Tick => {
                if self.notice_expires.is_some_and(|until| std::time::Instant::now() >= until) { self.notice.clear(); self.notice_expires = None; }
                if self.staged.is_some() && !self.busy && !self.modal_open() { return self.apply_update() }
                if self.polling || self.busy { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() }; self.polling = true; let settings = self.settings.clone(); let log_server = self.log_server; let read_logs = !self.paused && self.page == Page::Activity;
                return Task::perform(async move { tokio::task::spawn_blocking(move || {
                    let status = backend.status(&settings).ok(); let logs = if read_logs { backend.logs(log_server) } else { String::new() };
                    LiveStatus { server: status, game_running: backend.game_running(), game_ready: backend.game_ready(), logs, failure: backend.game_failure() }
                }).await.map_err(|e| e.to_string()) }, Msg::Poll);
            }
            Msg::Poll(result) => {
                self.polling = false;
                if let Ok(LiveStatus { server: status, game_running: running, game_ready: ready, logs, failure }) = result {
                    let attached = self.status.is_none() && status.is_some(); if let Some(error) = failure { self.feedback_at(Page::Play, Err(error)); }
                    self.status = status; self.game_running = running; self.game_ready = ready;
                    if attached && self.accounts.is_null() && !self.locked() { return self.load_accounts() }
                    if !self.paused && !logs.is_empty() { self.logs = logs; return widget::operation::snap_to_end("activity-log") }
                }
            }
            Msg::Done(result) => {
                self.busy = false;
                match result {
                    Ok(Outcome::Info(info)) => { self.completed(Ok(info)); return self.inspect_setup() }
                    Ok(Outcome::Status(status)) => { self.status = Some(status); self.completed(Ok("Server is running on this computer.".into())); self.startup.resume_hosting = false; return self.load_accounts() }
                    Ok(Outcome::GameStarted) => { self.game_running = true; self.game_ready = false; }
                    Ok(Outcome::Accounts(accounts)) => { self.accounts = accounts; self.reconcile_accounts(); }
                    Ok(Outcome::AccountsChanged(accounts, message, selected)) => { self.accounts = accounts; if let Some(selected) = selected { self.admin_account = selected; } self.reconcile_accounts(); self.new_account.clear(); self.new_player.clear(); self.account_form = false; self.completed(Ok(message)); }
                    Ok(Outcome::Path(field, path)) => return self.update(Msg::Input(field, path)),
                    Ok(Outcome::Imported(server)) => { self.reset_editor(); self.identity = server.identity.clone().or_else(|| yo3_tunnel::identity_from_pem(server.certificate.as_bytes()).ok()); self.label = server.label; self.address = server.address; self.key = server.key; self.certificate = server.certificate; self.server_name = server.server_name; self.editor = true; self.page = Page::Play; }
                    Ok(Outcome::Observed(server)) => self.observed = Some(server),
                    Ok(Outcome::Checked(info)) => { self.checked = Some(info); self.completed(Ok("This computer connected successfully and verified the account.".into())); }
                    Ok(Outcome::Bindings(bindings)) => self.bindings = bindings,
                    Ok(Outcome::SettingsSaved(section, settings)) => {
                        if settings.data_dir != self.settings.data_dir { self.accounts = Value::Null; self.account.clear(); self.admin_account.clear(); self.status = None; }
                        self.draft = section.merge(&self.draft, &settings); if section == Section::Hosting { self.port = settings.listen_port.to_string(); }
                        self.draft.selected_server = settings.selected_server; self.draft.local_account = settings.local_account.clone(); self.settings = settings;
                        self.completed(Ok("Changes saved.".into())); return self.inspect_setup();
                    }
                    Ok(Outcome::SelectionSaved(settings)) => { self.draft.selected_server = settings.selected_server; self.draft.local_account = settings.local_account.clone(); self.settings = settings; return self.inspect_setup() }
                    Ok(Outcome::ConnectionsSaved(servers, id)) => { self.selected = ServerChoice::saved(servers.iter().find(|s| s.id == id).unwrap()); self.servers = servers; self.settings.selected_server = Some(id); self.reset_editor(); self.completed(Ok("Connection saved.".into())); return self.save_selection() }
                    Ok(Outcome::ConnectionsRemoved(servers, settings)) => { self.servers = servers; self.settings = settings; self.draft.selected_server = self.settings.selected_server; self.selected = ServerChoice::Local; self.reset_editor(); self.completed(Ok("Saved connection removed.".into())); return self.inspect_setup() }
                    Ok(Outcome::Quit) => return iced::exit(),
                    Ok(Outcome::Report(report)) => if self.report_options { self.report = Some(report); },
                    Err(error) => { self.completed(Err(error)); return self.inspect_setup() }
                }
            }
            Msg::Logs(server) => { self.log_server = server; self.logs.clear(); }
            Msg::Pause(paused) => self.paused = paused,
            Msg::LogFilter(filter) => self.filter = filter,
            Msg::CopyLogs => return iced::clipboard::write(self.shown_logs()),
            Msg::ExportLogs => { self.report_options = true; return self.update(Msg::PrepareReport(false)) }
            Msg::PrepareReport(all) => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); self.report_all = all; self.report = None; self.begin("Preparing diagnostic preview…");
                return job(move || { let _ = backend.status(&settings); Ok(Outcome::Report(backend.report(all)?)) });
            }
            Msg::SaveLogs => {
                let Some(report) = self.report.take() else { return Task::none() }; self.report_options = false; self.begin("Saving diagnostic report…");
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().add_filter("ZIP report", &["zip"]).set_file_name("yo3-diagnostics.zip").save_file().await else { return Ok(Outcome::Info("Report export cancelled.".into())) };
                    tokio::task::spawn_blocking(move || report.export(file.path())).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
                    Ok(Outcome::Info("Diagnostic ZIP saved. Nothing was uploaded.".into()))
                }, Msg::Done);
            }
            Msg::Close => {
                if self.transfer.is_some() || self.applying { self.feedback(Err("Cancel the download or wait for the update restart before closing.".into())); return Task::none() }
                if self.status.is_some() || self.game_running || self.busy { self.closing = true; } else { return iced::exit() }
            }
            Msg::CancelClose => { self.closing = false; self.observed = None; self.report = None; self.report_options = false; self.review_update = false; self.remove_server = None; self.repair_setup = false; }
            Msg::Quit(stop_server) => {
                let Some(backend) = self.backend.clone() else { return iced::exit() }; let settings = self.settings.clone(); self.begin("Closing…");
                return job(move || { backend.stop_game()?; if stop_server && backend.live(&settings).is_some() { backend.stop_server(&settings)?; } Ok(Outcome::Quit) });
            }
        }
        Task::none()
    }
    fn begin(&mut self, label: &str) { self.busy = true; self.operation_page = self.page; self.operation_label = label.into(); self.notice.clear(); }
    fn completed(&mut self, result: Result<String, String>) { self.feedback_at(self.operation_page, result); }
    fn feedback(&mut self, result: Result<String, String>) { self.feedback_at(self.page, result); }
    fn feedback_at(&mut self, page: Page, result: Result<String, String>) {
        self.failed = result.is_err(); self.notice = result.unwrap_or_else(|e| e); self.feedback_page = page;
        self.notice_expires = (!self.failed).then(|| std::time::Instant::now() + std::time::Duration::from_secs(8));
        if let Some(backend) = &self.backend { backend.diagnostics.line(&self.notice); }
    }
    fn reset_editor(&mut self) { self.editor = false; self.editing_id = None; self.label.clear(); self.address.clear(); self.key.clear(); self.identity = None; self.certificate.clear(); self.server_name.clear(); }
    fn reconcile_accounts(&mut self) {
        let names = account_names(&self.accounts);
        if !names.contains(&self.account) { self.account = names.first().cloned().unwrap_or_default(); }
        if !names.contains(&self.admin_account) { self.admin_account = names.first().cloned().unwrap_or_default(); }
    }
    fn inspect_setup(&mut self) -> Task<Msg> {
        let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); self.setup_result = None;
        Task::perform(async move { let checked = settings.clone(); let result = tokio::task::spawn_blocking(move || (backend.readiness(&settings), backend.direct_state(&settings))).await;
            match result { Ok((result, state)) => (checked, result, state), Err(error) => (checked, yo3_desktop::Readiness::ComponentProblem(error.to_string()), yo3_desktop::direct::State::Disabled) }
        }, |(settings, result, state)| Msg::SetupChecked(settings, result, state))
    }
    fn shown_logs(&self) -> String {
        let filter = self.filter.to_lowercase();
        if filter.is_empty() { self.logs.clone() } else { self.logs.lines().filter(|line| line.to_lowercase().contains(&filter)).collect::<Vec<_>>().join("\n") }
    }
    fn can_play(&self) -> bool { matches!(self.setup_result, Some(yo3_desktop::Readiness::Ready)) && !self.locked() && !self.game_running && self.backend.is_some() }
    fn changed(&self, section: Section) -> bool { section.changed(&self.settings, &self.draft) || (section == Section::Hosting && self.port != self.settings.listen_port.to_string()) }
    fn modal_open(&self) -> bool { self.review_update || self.report_options || self.observed.is_some() || self.closing || self.remove_server.is_some() || self.repair_setup }
    fn locked(&self) -> bool { self.busy || self.transfer.is_some() || self.staged.is_some() || self.applying }
    fn save_selection(&mut self) -> Task<Msg> {
        let root = self.root.clone(); let settings = self.settings.clone(); self.begin("Saving connection selection…");
        let backend = self.backend.clone(); let remote = self.selected.remote(&self.servers).ok().flatten();
        job(move || { LauncherDb::open(&root)?.save_settings(&settings)?; if let Some(backend) = backend && let Err(error) = backend.sync_direct(&settings, remote) { backend.diagnostics.line(&format!("Direct-launch selection needs refresh: {error:#}")); } Ok(Outcome::SelectionSaved(settings)) })
    }
    fn load_accounts(&mut self) -> Task<Msg> { let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); self.begin("Loading accounts…"); job(move || Ok(Outcome::Accounts(backend.accounts(&settings)?))) }
    fn apply_update(&mut self) -> Task<Msg> {
        if self.busy || self.modal_open() || self.applying { return Task::none() }
        let Some(installed) = self.staged.take() else { return Task::none() };
        let Some(backend) = self.backend.clone() else { self.feedback(Err("Fix application setup before installing an update.".into())); return Task::none() };
        let settings = self.settings.clone(); let root = self.root.clone(); self.applying = true;
        self.operation_label = "Restarting into the verified update…".into();
        Task::perform(async move { tokio::task::spawn_blocking(move || {
            let hosting = backend.live(&settings).is_some();
            let result = (|| -> anyhow::Result<()> {
                backend.stop_game()?;
                if hosting { backend.stop_server(&settings)?; }
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(25);
                while backend.game_running() || backend.live(&settings).is_some() {
                    anyhow::ensure!(std::time::Instant::now() < deadline, "Game/server did not stop in time. This version is still active.");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                updates::activation::launch(&root, &installed, hosting, std::time::Duration::from_secs(25))?; Ok(())
            })();
            if result.is_err() && hosting && backend.live(&settings).is_none() { let _ = backend.start_server(&settings); }
            result.map_err(|e| format!("{e:#}"))
        }).await.map_err(|e| e.to_string())? }, Msg::UpdateApplied)
    }
}
fn check_updates() -> Task<Msg> { Task::perform(async { updates::check().await.map_err(|e| format!("{e:#}")) }, Msg::UpdateChecked) }
fn open_external(url: String) -> Task<Msg> {
    Task::perform(async move { tokio::task::spawn_blocking(move || open(&url)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string()) }, Msg::ExternalOpened)
}
fn account_names(accounts: &Value) -> Vec<String> { accounts.as_array().into_iter().flatten().filter_map(|a| a["players"][0]["name"].as_str().map(str::to_string)).collect() }
fn open(path: &str) -> anyhow::Result<()> {
    #[cfg(windows)] let mut cmd = { let mut c = std::process::Command::new("explorer.exe"); c.arg(path); c };
    #[cfg(not(windows))] let mut cmd = { let mut c = std::process::Command::new("xdg-open"); c.arg(path); c };
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?;
    Ok(())
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    #[test]
    fn confirmation_blocks_background_forms_and_download_completion_survives_navigation() {
        let root = std::env::temp_dir().join(format!("yo3-ui-update-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
        let mut app = App::load(root.clone()); app.review_update = true;
        let previous = app.draft.game_dir.clone(); let _ = app.update(Msg::Input(Field::Game, "unexpected".into()));
        assert_eq!(app.draft.game_dir, previous);
        let _ = app.update(Msg::Page(Page::Server)); assert!(app.page != Page::Server);
        let _ = app.update(Msg::CancelClose); assert!(!app.review_update);
        app.busy = true; let _ = app.update(Msg::UpdatePrepared(Err("offline".into())));
        assert!(app.busy, "an update result must not clear an unrelated operation"); assert!(app.notice.contains("offline"));
        let _ = app.update(Msg::UpdateChecked(Ok(updates::Decision::NoReleases)));
        assert!(matches!(app.update_status, UpdateStatus::Checked(updates::Decision::NoReleases)));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn account_refresh_preserves_unfinished_forms_and_selection_preserves_settings_drafts() {
        let root = std::env::temp_dir().join(format!("yo3-ui-drafts-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
        let mut app = App::load(root.clone()); app.new_account = "Unfinished".into(); app.draft.game_dir = "unsaved folder".into();
        let _ = app.update(Msg::Done(Ok(Outcome::Accounts(serde_json::json!([])))));
        assert_eq!(app.new_account, "Unfinished");
        let _ = app.update(Msg::Done(Ok(Outcome::SelectionSaved(app.settings.clone()))));
        assert_eq!(app.draft.game_dir, "unsaved folder");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn downloads_require_explicit_review_and_leave_navigation_available() {
        let root = std::env::temp_dir().join(format!("yo3-ui-confirm-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
        let mut app = App::load(root.clone());
        app.update_status = UpdateStatus::Checked(updates::Decision::Available(updates::Release { tag: "v0.3.0".into(), url: String::new(), notes: String::new(), asset: updates::Asset { name: String::new(), url: String::new(), bytes: 100, sha256: String::new() } }));
        let _ = app.update(Msg::ConfirmUpdate); assert!(app.transfer.is_none());
        let _ = app.update(Msg::ReviewUpdate); assert!(app.review_update); assert!(app.transfer.is_none());
        let _ = app.update(Msg::ConfirmUpdate); assert!(!app.review_update); assert!(app.transfer.is_some());
        let _ = app.update(Msg::Page(Page::Activity)); assert!(app.page == Page::Activity);
        let _ = app.update(Msg::Start); assert!(!app.busy, "cannot start another job during an update");
        let _ = app.update(Msg::CancelUpdate); assert!(app.transfer.as_ref().unwrap().1.cancelled());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn host_account_selection_and_unsaved_networking_cannot_change_play_or_start_settings() {
        let root=std::env::temp_dir().join(format!("yo3-ui-separation-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone()); app.account="Mine".into(); app.settings.local_account="Mine".into();
        let _=app.update(Msg::AdminAccountSelect("Friend".into())); assert_eq!(app.account,"Mine"); assert_eq!(app.settings.local_account,"Mine"); assert_eq!(app.admin_account,"Friend");
        let saved=app.settings.clone(); let _=app.update(Msg::BindSelect(yo3_desktop::network::defaults()[1].clone()));
        assert_eq!(app.settings,saved,"binding changes are drafts until Save");
        app.port="18443".into(); app.draft.host_address="unfinished.example".into(); let _=app.update(Msg::Start);
        assert_eq!(app.settings,saved,"Start must use saved settings");
        app.busy=false; let _=app.update(Msg::CancelSettings(yo3_desktop::preferences::Section::Hosting));
        assert_eq!(app.draft.bind_address,saved.bind_address); assert_eq!(app.port,saved.listen_port.to_string());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn navigation_remains_responsive_while_a_background_action_is_running() {
        let root = std::env::temp_dir().join(format!("yo3-ui-navigation-{}", std::process::id())); let _ = std::fs::remove_dir_all(&root);
        let mut app = App::load(root.clone()); app.page = Page::Play; app.busy = true;
        let _ = app.update(Msg::Page(Page::Settings));
        assert!(app.page == Page::Settings, "a clickable navigation item must still work during a connection/server operation");
        assert!(app.busy, "navigation must not pretend the background operation finished");
        std::fs::remove_dir_all(root).unwrap();
    }
}
