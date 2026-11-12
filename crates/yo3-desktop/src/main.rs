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
mod error_presentation;
mod bug_ui;
mod bug_auth;
mod bug_view;
use bug_ui::{BugEvent, BugReport, GitHubAccount};
use yo3_desktop::{updates, preferences::Section};

fn main() -> iced::Result {
    let result=run();
    if let Err(error)=&result { startup_error(&format!("Could not open the application window.\n{error}\n\nKeep the complete Desktop bundle together and use a supported OS. Include this error and your version when asking for help.")); }
    result
}
fn startup_error(message: &str) {
    eprintln!("{message}");
    rfd::MessageDialog::new().set_title("YO3 Revival could not open").set_description(message)
        .set_level(rfd::MessageLevel::Error).set_buttons(rfd::MessageButtons::Ok).show();
}
fn run() -> iced::Result {
    if let Some(code) = yo3_desktop::privileged::helper_mode() { std::process::exit(code); }
    let root = yo3_desktop::default_root();
    let startup = match updates::activation::Startup::parse(&root, std::env::args().skip(1)) {
        Ok(startup) => startup, Err(error) => { startup_error(&format!("Invalid startup request: {error:#}\nOpen the launcher normally from its complete Desktop folder.")); std::process::exit(1); }
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
        .window(iced::window::Settings { size: iced::Size::new(1000.0, 640.0), position: iced::window::Position::Centered, min_size: Some(iced::Size::new(900.0, 640.0)), ..Default::default() }).exit_on_close_request(false)
        .subscription(App::subscription)
        .run()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page { Play, Server, Activity, Settings, About }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostTab { Overview, Invitations, Advanced }
#[derive(Clone, Copy)]
enum Disclosure { Game, Account }
#[derive(Clone)]
enum UpdateStatus { Checking, Checked(updates::Decision), Failed(String) }
#[derive(Clone)]
enum GameDataDialog {
    Checking { settings: Settings },
    Ready { settings: Settings, preview: yo3_desktop::game_data::Preview, keep_players: bool },
    Problem(String), Complete,
}
#[derive(Clone, Copy)]
enum Field { Game, Wine, Host, Data, Player, Label, Address, Key, Account, NewPlayer, Port, GiveBp }
#[derive(Clone)]
enum Msg {
    Mode(bool), ServerOption(String, String), ResetOption(String), FindOption(String),
    Page(Page), HostTab(HostTab), Input(Field, String), Select(ServerChoice), AccountSelect(String), AdminAccountSelect(String),
    Start, Stop, Play, StopGame, TestConnection, Discover, TrustIdentity, RefreshBindings, BindSelect(yo3_desktop::network::BindAddress), Direct(bool),
    SaveSettings(Section), CancelSettings(Section), Browse(Field), UsePath, FolderPicked(u64,Field,Result<Option<String>,String>), Import, AddServer, SaveServer, EditServer, RemoveServer, ConfirmRemoveServer,
    CreateAccount, AddPlayer, GiveBp, Export, Tick, Animate, Poll(Result<LiveStatus, String>), LogsPolled(Result<String,String>), Done(Result<Outcome, String>),
    Logs(bool), LogScrolled(bool), Pause(bool), LogFilter(String), CopyLogs, ExportLogs, PrepareReport(bool), SaveLogs, RemovePersonal(bool), OpenData, Support, Help, NetworkHelp,
    Close, CancelClose, KeepHosting(bool), Quit(bool), Disclosure(Disclosure), NewAccount, GitHub, ExternalOpened(Result<(), String>), DismissNotice,
    RetrySetup, ValidateGame, RunSetupCheck(u64), SetupChecked(u64, Settings, yo3_desktop::Readiness, yo3_desktop::direct::State, Vec<PathBuf>), SetupSaved(u64, Settings, Result<Settings,String>), CopyError,
    CheckInvitation, InvitationChecked(u64, SavedServer, Result<yo3_tunnel::ServerInfo,String>),
    Troubleshoot, GameDataPreviewed(u64,Settings,Result<yo3_desktop::game_data::Preview,String>), KeepGameDataPlayers(bool), ConfirmGameDataReset, OpenGameDataBackup,
    ReportPrepared(u64, Result<yo3_desktop::reports::Report,String>), ReportDestination(u64,Result<Option<PathBuf>,String>), ReportSaved(u64, Result<String,String>), CopyReportError,
    ReportBug, Bug(BugEvent),
    CheckUpdates, UpdateChecked(Result<updates::Decision, String>), ReleasePage, OpenReleaseLink(String), CopyAddress, CopyVersion,
    ReviewUpdate, ConfirmUpdate, CancelUpdate, UpdatePrepared(Result<updates::install::Installed, String>), UpdateApplied(Result<(), String>), WindowOpened,
}
impl std::fmt::Debug for Msg { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("DesktopEvent") } }
#[derive(Clone)]
enum Outcome {
    Info(String), Status(Value), GameStarted, Accounts(Value), AccountsChanged(Value, String, Option<String>), Imported(SavedServer),
    Observed(yo3_tunnel::ObservedServer), Checked(yo3_tunnel::ServerInfo), Bindings(Vec<yo3_desktop::network::BindAddress>),
    SettingsSaved(Section, Settings), SelectionSaved(Settings), ConnectionsSaved(Vec<SavedServer>, i64), ConnectionsRemoved(Vec<SavedServer>, Settings),
    Recovered(Settings, Vec<SavedServer>, Arc<Backend>), GameDataReset(PathBuf), Quit,
}
#[derive(Clone)]
struct LiveStatus { server: Option<Value>, game_running: bool, game_ready: bool, failure: Option<String> }
struct App {
    root: PathBuf, backend: Option<Arc<Backend>>, settings: Settings, draft: Settings, page: Page, host_tab: HostTab,
    servers: Vec<SavedServer>, selected: ServerChoice, status: Option<Value>, accounts: Value, account: String, admin_account: String,
    new_account: String, new_player: String, give_bp: String, editor: bool, editing_id: Option<i64>, remove_server: Option<i64>,
    label: String, address: String, key: String, certificate: String, server_name: String,
    busy: bool, polling: bool, game_running: bool, game_ready: bool, notice: String, failed: bool, feedback_page: Page,
    notice_expires: Option<std::time::Instant>, operation_page: Page, operation_label: String,
    failure_setup: Option<yo3_desktop::Readiness>, monitor_error: Option<String>, logs_polling: bool, pulse: f32,
    setup_revision: u64, setup_checking: bool, setup_due: Option<std::time::Instant>, setup_active: Option<u64>, setup_verified: Option<Settings>, setup_saving: Option<u64>, starting_game: bool, game_start: Option<std::time::Instant>,
    picking: bool, picker_generation: u64,
    log_server: bool, paused: bool, follow_logs: bool, logs: String, filter: String, closing: bool, keep_hosting_on_exit: bool, support: bool,
    identity: Option<String>, observed: Option<yo3_tunnel::ObservedServer>, checked: Option<yo3_tunnel::ServerInfo>,
    invitation_imported: bool, invitation_checking: bool, invitation_verified: bool, invitation_generation: u64,
    bindings: Vec<yo3_desktop::network::BindAddress>, port: String, report: Option<yo3_desktop::reports::Report>, report_options: bool, report_all: bool,
    report_busy: bool, report_generation: u64, report_feedback: Option<Result<String,String>>, report_remove_personal: bool,
    bug_report: Option<BugReport>, github_account: Option<GitHubAccount>,
    advanced_game: bool, account_options: bool, account_form: bool,
    option_filter: String,
    setup_result: Option<yo3_desktop::Readiness>, setup_missing_files: Vec<PathBuf>, direct_state: yo3_desktop::direct::State, first_setup: bool,
    game_data_dialog: Option<GameDataDialog>, game_data_generation: u64, game_data_backup: Option<PathBuf>,
    update_status: UpdateStatus, release_notes: Vec<widget::markdown::Item>, review_update: bool, transfer: Option<(updates::Release, updates::download::Transfer)>,
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
        let first_setup = !std::path::Path::new(&settings.game_dir).join("yo3.exe").is_file(); let page = if settings.advanced_mode && first_setup { Page::Settings } else { Page::Play };
        Self { root, backend, draft: settings.clone(), settings, page, host_tab: HostTab::Overview, servers, selected, status: None, accounts: Value::Null,
            account, admin_account: String::new(), new_account: String::new(), new_player: String::new(), give_bp: String::new(), editor: false, editing_id: None, remove_server: None,
            label: String::new(), address: String::new(), key: String::new(), certificate: String::new(), server_name: String::new(),
            busy: false, polling: false, game_running: false, game_ready: false, failed: !error.is_empty(), notice: error, feedback_page: page,
            notice_expires: None, operation_page: page, operation_label: String::new(), log_server: true, paused: false, follow_logs: true, logs: String::new(), filter: String::new(), closing: false, keep_hosting_on_exit: true, support: false,
            failure_setup: None, monitor_error: None, logs_polling: false, pulse: 0.0, setup_revision: 0, setup_checking: false, setup_due: None, setup_active: None, setup_verified: None, setup_saving: None, starting_game: false, game_start: None,
            picking: false, picker_generation: 0,
            identity: None, observed: None, checked: None, invitation_imported: false, invitation_checking: false, invitation_verified: false, invitation_generation: 0, bindings: yo3_desktop::network::defaults(), port, report: None, report_options: false, report_all: false,
            report_busy: false, report_generation: 0, report_feedback: None, report_remove_personal: false, bug_report: None, github_account: None,
            advanced_game: false, account_options: false, account_form: false, option_filter: String::new(), setup_result: None, setup_missing_files: Vec::new(), direct_state: yo3_desktop::direct::State::Disabled, first_setup,
            game_data_dialog: None, game_data_generation: 0, game_data_backup: None,
            update_status: UpdateStatus::Checking, release_notes: Vec::new(), review_update: false, transfer: None, staged: None, applying: false, startup: Default::default() }
    }
    fn update(&mut self, message: Msg) -> Task<Msg> {
        let background = matches!(message, Msg::Done(_) | Msg::FolderPicked(..) | Msg::Poll(_) | Msg::LogsPolled(_) | Msg::LogScrolled(_) | Msg::Tick | Msg::Animate | Msg::RunSetupCheck(_) | Msg::SetupSaved(..) | Msg::InvitationChecked(..) | Msg::Bug(_) | Msg::ReportPrepared(..) | Msg::ReportDestination(..) | Msg::ReportSaved(..) | Msg::UpdateChecked(_) | Msg::ExternalOpened(_) | Msg::SetupChecked(..) | Msg::GameDataPreviewed(..) | Msg::UpdatePrepared(_) | Msg::UpdateApplied(_) | Msg::WindowOpened);
        let game_data_action=self.game_data_dialog.is_some() && matches!(message,Msg::Troubleshoot | Msg::KeepGameDataPlayers(_) | Msg::ConfirmGameDataReset | Msg::OpenGameDataBackup);
        let support_action=self.support && matches!(message,Msg::Help | Msg::GitHub | Msg::CopyVersion | Msg::CopyLogs | Msg::ExportLogs | Msg::ReportBug | Msg::Troubleshoot | Msg::Page(Page::About));
        let bug_action=self.report_options && self.bug_report.is_some() && matches!(message,Msg::Bug(_));
        let closing_action=self.closing && matches!(message,Msg::KeepHosting(_));
        let release_link_action=self.review_update && matches!(message,Msg::OpenReleaseLink(_));
        if self.modal_open() && !background && !game_data_action && !support_action && !bug_action && !closing_action && !release_link_action && !matches!(message, Msg::CancelClose | Msg::Close | Msg::Quit(_) | Msg::TrustIdentity | Msg::SaveLogs | Msg::PrepareReport(_) | Msg::RemovePersonal(_) | Msg::CopyReportError | Msg::CopyError | Msg::DismissNotice | Msg::CopyLogs | Msg::ConfirmUpdate | Msg::CancelUpdate | Msg::ConfirmRemoveServer | Msg::Browse(Field::Game) | Msg::ReleasePage) { return Task::none() }
        if self.locked() && !background && !matches!(message, Msg::UsePath | Msg::Close | Msg::CancelClose | Msg::Page(_) | Msg::HostTab(_) | Msg::GitHub | Msg::Support | Msg::Help | Msg::NetworkHelp | Msg::OpenData | Msg::OpenGameDataBackup | Msg::ReleasePage | Msg::OpenReleaseLink(_) | Msg::CheckUpdates | Msg::CopyVersion | Msg::CopyError | Msg::ReportBug | Msg::Bug(_) | Msg::Logs(_) | Msg::Pause(_) | Msg::LogFilter(_) | Msg::CopyLogs | Msg::Disclosure(_) | Msg::CancelUpdate | Msg::Stop | Msg::StopGame | Msg::DismissNotice) { return Task::none() }
        match message {
            Msg::ReportBug => return self.open_bug_report(),
            Msg::Bug(event) => return self.update_bug_report(event),
            Msg::PrepareReport(all) if self.bug_report.is_some() => return self.start_bug_preview(all),
            Msg::RemovePersonal(value) if self.bug_report.is_some() => { if !self.report_busy && !self.bug_report.as_ref().unwrap().frozen() {self.report_remove_personal=value; self.invalidate_bug_files();} },
            Msg::Troubleshoot => {
                self.support=false;
                self.game_data_generation+=1;let generation=self.game_data_generation;let settings=self.settings.clone();
                let Some(backend)=self.backend.clone() else {
                    self.game_data_dialog=Some(GameDataDialog::Problem("App setup is unavailable. Close this dialog and choose Retry app setup before resetting game data.".into()));return Task::none()
                };
                self.game_data_dialog=Some(GameDataDialog::Checking {settings:settings.clone()});self.begin("Checking local game data…");
                return Task::perform(async move {
                    let checked=settings.clone();let result=tokio::task::spawn_blocking(move ||backend.preview_game_data(&checked)).await
                        .map_err(|error|error.to_string()).and_then(|result|result.map_err(|error|format!("{error:#}")));
                    (settings,result)
                },move |(settings,result)|Msg::GameDataPreviewed(generation,settings,result));
            }
            Msg::GameDataPreviewed(generation,settings,result) => {
                if generation!=self.game_data_generation || !matches!(&self.game_data_dialog,Some(GameDataDialog::Checking {settings:checked}) if checked==&settings) {return Task::none()}
                self.busy=false;
                if settings!=self.settings {
                    self.game_data_dialog=Some(GameDataDialog::Problem("Game setup changed while checking local data. Choose Check again to review the current folder.".into()));return Task::none()
                }
                self.game_data_dialog=Some(match result {
                    Ok(preview)=>GameDataDialog::Ready {settings,preview,keep_players:true},
                    Err(error)=>{self.completed(Err(error.clone()));GameDataDialog::Problem(error)},
                });
            }
            Msg::KeepGameDataPlayers(keep) => {
                if let Some(GameDataDialog::Ready {keep_players,..})=&mut self.game_data_dialog {*keep_players=keep;}
            }
            Msg::ConfirmGameDataReset => {
                let Some((settings,preview,full))=self.game_data_reset_request() else {return Task::none()};let backend=self.backend.clone().unwrap();
                self.game_data_generation+=1;self.game_data_dialog=None;
                self.setup_revision+=1;self.setup_checking=false;self.setup_due=None;self.setup_verified=None;self.setup_result=None;
                self.begin("Backing up and resetting local game data…");
                return job(move ||Ok(Outcome::GameDataReset(backend.reset_game_data(&settings,&preview,full)?.backup)));
            }
            Msg::OpenGameDataBackup => return match &self.game_data_backup {Some(path)=>open_external(path.to_string_lossy().into_owned()),None=>Task::none()},
            Msg::Animate => self.pulse=(self.pulse+0.6)%std::f32::consts::TAU,
            Msg::RetrySetup => {
                let root=self.root.clone(); self.begin("Opening saved settings and logs…");
                return job(move || {let db=LauncherDb::open(&root)?; Ok(Outcome::Recovered(db.settings()?,db.servers()?,Backend::new(root)?))});
            }
            Msg::Mode(advanced) => {
                if let Err(error) = LauncherDb::open(&self.root).and_then(|db| db.save_mode(advanced)) { self.feedback(Err(error.to_string())); return Task::none() }
                self.settings.advanced_mode = advanced; self.draft.advanced_mode = advanced;

                if !advanced { self.page = Page::Play; self.host_tab = HostTab::Overview; }
                self.follow_logs=true; return widget::operation::snap_to_end("activity-log");
            }
            Msg::ServerOption(name, value) => { if value.is_empty() { self.draft.server_options.remove(&name); } else { self.draft.server_options.insert(name, value); } }
            Msg::ResetOption(name) => { self.draft.server_options.remove(&name); }
            Msg::FindOption(value) => self.option_filter = value,
            Msg::WindowOpened => if self.backend.is_some() && let Some(readiness) = self.startup.readiness.take() {
                if let Err(error) = readiness.write(yo3_tunnel::BUILD_VERSION) { self.feedback(Err(format!("Update readiness failed: {error:#}"))); return Task::none() }
                if self.startup.resume_hosting { return self.update(Msg::Start) }
            },
            Msg::ValidateGame => {
                self.first_setup=false;self.notice.clear();self.failed=false;self.failure_setup=None;
                return self.inspect_setup()
            }
            Msg::RunSetupCheck(revision) => return self.run_setup_check(revision),
            Msg::SetupChecked(revision, settings, result, direct, missing_files) => {
                if self.setup_active!=Some(revision) {return Task::none()}
                self.setup_active=None;
                if revision!=self.setup_revision || !setup_matches(&settings,&self.setup_candidate()) {
                    return self.run_setup_check(self.setup_revision)
                }
                let result=if result==yo3_desktop::Readiness::Ready && !missing_files.is_empty() {
                    let paths=missing_files.iter().map(|path|path.display().to_string()).collect::<Vec<_>>().join("\n");
                    yo3_desktop::Readiness::ComponentProblem(format!("Required files are missing:\n{paths}\nRestore the listed files, then choose Retry check."))
                }else{result};
                if result!=yo3_desktop::Readiness::Ready && let Some(backend)=&self.backend {
                    for path in &missing_files {backend.diagnostics.line(&format!("Missing required file: {}",path.display()));}
                }
                self.setup_checking=false;self.setup_due=None;self.direct_state=direct;self.setup_result=Some(result.clone());self.setup_missing_files=missing_files;
                if result==yo3_desktop::Readiness::Ready {
                    self.setup_verified=Some(settings);
                    return self.finish_setup_check()
                }
                self.setup_verified=None;
                if let yo3_desktop::Readiness::GameProblem(error) | yo3_desktop::Readiness::ComponentProblem(error) | yo3_desktop::Readiness::ValidationProblem(error)=&result
                    && !self.first_setup && !self.draft.game_dir.trim().is_empty() && (!self.failed || self.failure_setup.is_some()) {
                    self.feedback(Err(error.clone()));self.failure_setup=Some(result);
                }
            }
            Msg::SetupSaved(revision, candidate, result) => {
                if self.setup_saving!=Some(revision) {return Task::none()}
                self.setup_saving=None;self.busy=false;
                if revision!=self.setup_revision || !setup_matches(&candidate,&self.setup_candidate()) {return self.inspect_setup()}
                match result {
                    Ok(settings)=>{
                        self.draft=Section::Game.merge(&self.draft,&settings);self.settings=settings;
                        self.setup_due=None;self.setup_checking=false;self.setup_result=Some(yo3_desktop::Readiness::Ready);self.setup_verified=Some(self.setup_candidate());
                        self.first_setup=false;
                        if self.failure_setup.is_some() {self.notice.clear();self.failed=false;self.failure_setup=None;}
                    }
                    Err(error)=>{
                        self.setup_due=None;self.setup_checking=false;self.setup_verified=None;
                        let result=yo3_desktop::Readiness::ValidationProblem(error.clone());self.setup_result=Some(result.clone());
                        if !self.failed || self.failure_setup.is_some(){self.feedback(Err(error));self.failure_setup=Some(result);}
                    }
                }
            }
            Msg::CheckInvitation => return self.check_invitation(),
            Msg::InvitationChecked(generation, server, result) => {
                if generation!=self.invitation_generation || !self.editor || !self.invitation_imported {return Task::none()}
                self.invitation_checking=false;
                if !self.invitation_matches(&server) {return Task::none()}
                match result {
                    Ok(info) if !info.ready=>{
                        self.invitation_verified=false;let problem=info.problem.clone().unwrap_or_else(||"The host's rules engine is unavailable.".into());self.checked=Some(info);
                        self.feedback(Err(format!("{problem}\nAsk the host to repair the server, then choose Retry check.")));
                    }
                    Ok(info)=>{self.checked=Some(info);self.invitation_verified=true;}
                    Err(error)=>{self.invitation_verified=false;self.feedback(Err(error));}
                }
            }
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
            Msg::Support => self.support=true,
            Msg::GitHub => return open_external(updates::REPOSITORY.into()),
            Msg::Help => return open_external("https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md".into()),
            Msg::NetworkHelp => return open_external("https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md#connection-help".into()),
            Msg::OpenData => return open_external(self.root.to_string_lossy().into_owned()),
            Msg::ExternalOpened(result) => if let Err(error) = result { self.feedback(Err(error)); },
            Msg::CopyVersion => return iced::clipboard::write(format!("YO3 Revival {} ({}) · {}", yo3_tunnel::BUILD_VERSION, yo3_tunnel::BUILD_COMMIT, std::env::consts::OS)),
            Msg::CopyAddress => if let Ok(Some((address, _))) = yo3_desktop::network::share_address(&self.settings) { return iced::clipboard::write(address) },
            Msg::CheckUpdates => if !matches!(self.update_status, UpdateStatus::Checking) { self.update_status = UpdateStatus::Checking; return check_updates() },
            Msg::UpdateChecked(result) => {
                self.release_notes=match &result {Ok(updates::Decision::Available(release))=>widget::markdown::parse(&release.notes).collect(),_=>Vec::new()};
                if !matches!(result,Ok(updates::Decision::Available(_))) {self.review_update=false;}
                self.update_status=match result {Ok(result)=>UpdateStatus::Checked(result),Err(error)=>UpdateStatus::Failed(error)};
            }
            Msg::ReleasePage => return open_external(match &self.update_status { UpdateStatus::Checked(updates::Decision::Available(release)) => release.url.clone(), _ => updates::RELEASES.into() }),
            Msg::OpenReleaseLink(link) => match release_web_link(&link) {Ok(link)=>return open_external(link),Err(error)=>self.feedback(Err(error))},
            Msg::CopyError => return iced::clipboard::write(format!("YO3 Revival {} ({})\n{}",yo3_tunnel::BUILD_VERSION,yo3_tunnel::BUILD_COMMIT,self.error_report())),
            Msg::DismissNotice => { self.notice.clear(); self.notice_expires = None; }
            Msg::Disclosure(section) => match section { Disclosure::Game => self.advanced_game = !self.advanced_game, Disclosure::Account => self.account_options = !self.account_options },
            Msg::NewAccount => self.account_form = !self.account_form,
            Msg::Page(page) => {
                self.support=false;
                self.page = page;  if page == Page::Play { self.first_setup = false; }
                if page==Page::Activity || (!self.settings.advanced_mode && page==Page::Play) {
                    self.follow_logs=true; return widget::operation::snap_to_end("activity-log");
                }
            }
            Msg::HostTab(tab) => {
                self.host_tab = tab;
                if !self.locked() {
                    if tab == HostTab::Invitations && self.status.is_some() { return self.load_accounts() }
                    if tab == HostTab::Advanced { return self.update(Msg::RefreshBindings) }
                }
            }
            Msg::Input(field, value) => return self.input(field,value),
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
                if self.invitation_imported {return self.check_invitation()}
            },
            Msg::Start | Msg::Stop | Msg::Play | Msg::StopGame => {
                if self.busy || self.applying { return Task::none() }
                if matches!(message,Msg::Play) && !self.can_play() { return Task::none() }
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); let who = self.account.clone();
                match message {
                    Msg::Start => { self.begin("Starting your server…"); return job(move || Ok(Outcome::Status(backend.start_server(&settings)?))) }
                    Msg::Stop => { self.begin("Stopping your server…"); return job(move || { backend.stop_server(&settings)?; Ok(Outcome::Info("Server is stopping.".into())) }) }
                    Msg::StopGame => { self.begin("Closing the game…"); return job(move || { backend.stop_game()?; Ok(Outcome::Info("Closing the game…".into())) }) }
                    _ => {
                        let remote = match self.selected.remote(&self.servers) { Ok(remote) => remote, Err(e) => { self.feedback(Err(e.to_string())); return Task::none() } };
                        self.starting_game=true;self.begin("Rechecking files and preparing the game…");
                        return job(move || { backend.play(&settings, remote, &who)?; Ok(Outcome::GameStarted) });
                    }
                }
            }
            Msg::SaveSettings(section) => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let mut draft = self.draft.clone();
                if section == Section::Hosting { draft.listen_port = match self.port.parse() { Ok(port) if port > 0 => port, _ => { self.feedback(Err("Port must be between 1 and 65535.".into())); return Task::none() } }; }
                self.begin("Saving changes…"); return job(move || Ok(Outcome::SettingsSaved(section, backend.save_preferences(section, &draft)?)));
            }
            Msg::CancelSettings(section) => {
                self.draft=section.merge(&self.draft,&self.settings);
                if section==Section::Hosting {self.port=self.settings.listen_port.to_string();}else{return self.inspect_setup()}
            }
            Msg::Browse(field) => {
                self.begin("Choose a folder in the file dialog…");self.picking=true;self.picker_generation+=1;let generation=self.picker_generation;
                return Task::perform(async move {tokio::time::timeout(std::time::Duration::from_secs(120),rfd::AsyncFileDialog::new().set_title("Choose folder").pick_folder()).await
                    .map(|file|file.map(|f|f.path().to_string_lossy().into_owned())).map_err(|_|"Folder selection took too long. Close the folder dialog if it is open, then paste the path into the game folder box or Browse again.".into())},move|path|Msg::FolderPicked(generation,field,path));
            }
            Msg::UsePath => if self.picking {self.picking=false;self.picker_generation+=1;self.busy=false;self.feedback(Ok("Paste the path into the folder box. Close the file dialog if it is still open; its selection will be ignored.".into()));},
            Msg::FolderPicked(generation,field,result) => {
                if !self.picking || generation!=self.picker_generation {return Task::none()}
                self.picking=false;self.busy=false;
                match result {Ok(Some(path))=>return self.input_immediately(field,path),Ok(None)=>self.completed(Ok(format!("No folder selected. You can Browse again or paste the {} path into the box.",match field {Field::Data=>"data folder",Field::Wine=>"Wine prefix",_=>"game folder"}))),Err(error)=>self.completed(Err(error))}
            }
            Msg::Import => {
                self.begin("Importing an invitation…");
                return Task::perform(async {
                    let file=tokio::time::timeout(std::time::Duration::from_secs(120),rfd::AsyncFileDialog::new().set_title("Import server invitation").add_filter("Invitation", &["json"]).pick_file()).await
                        .map_err(|_|"Invitation selection took too long. Close the file dialog if it is still open, then choose Import invitation again.".to_string())?;
                    match file {Some(file)=>SavedServer::import(file.path()).map(Outcome::Imported).map_err(|e|e.to_string()),None=>Ok(Outcome::Info("Import cancelled.".into()))}
                },Msg::Done);
            }
            Msg::SaveServer => {
                if self.invitation_imported && (!self.invitation_verified || self.invitation_checking) {return Task::none()}
                let result = (|| -> anyhow::Result<SavedServer> {
                    let (address, name) = yo3_desktop::server_address(&self.address)?; anyhow::ensure!(self.identity.is_some(), "Check and approve the server identity first.");
                    let server = SavedServer { id: self.editing_id.unwrap_or(0), label: if self.label.trim().is_empty() { name.clone() } else { self.label.trim().into() }, address,
                        server_name: if self.server_name.is_empty() { name } else { self.server_name.clone() }, certificate: self.certificate.clone(), identity: self.identity.clone(), key: self.key.trim().into() };
                    server.validate()?; Ok(server)
                })();
                match result { Ok(server) => {
                    let Some(backend)=self.backend.clone() else {return Task::none()};let root=self.root.clone();self.begin("Checking and saving connection…");
                    return job(move || {backend.check_connection(&server)?;let db=LauncherDb::open(&root)?;let id=db.save_server(&server)?;Ok(Outcome::ConnectionsSaved(db.servers()?,id))})
                }, Err(e) => self.feedback(Err(e.to_string())) }
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
            Msg::GiveBp => {
                let Some(backend) = self.backend.clone() else { return Task::none() };
                let Some(amount) = give_bp_amount(&self.give_bp) else { return Task::none() };
                let (settings, who) = (self.settings.clone(), self.admin_account.clone());
                self.begin("Giving BP…");
                return job(move || {
                    let total = backend.manage(&settings, ManagerOp::GiveBp { who: who.clone(), amount })?["bp"].as_u64().unwrap_or_default();
                    Ok(Outcome::AccountsChanged(backend.accounts(&settings)?, format!("Gave {amount} BP to {who}: {total} BP now. The game shows it after the next purchase or login."), None))
                });
            }
            Msg::Export => {
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); let who = self.admin_account.clone(); self.begin("Exporting private invitation…");
                return Task::perform(async move {
                    let file=tokio::time::timeout(std::time::Duration::from_secs(120),rfd::AsyncFileDialog::new().set_title("Export private invitation").set_file_name("yo3-invitation.json").save_file()).await
                        .map_err(|_|"Invitation export took too long. Close the file dialog if it is still open, then choose Export private invitation again.".to_string())?;
                    let Some(file)=file else {return Ok(Outcome::Info("Export cancelled.".into()))};
                    tokio::task::spawn_blocking(move || { backend.invitation(&settings, &who)?.export(file.path())?; Ok(Outcome::Info("Private invitation saved. Send it only to this account's owner.".into())) }).await.map_err(|e| e.to_string())?.map_err(|e: anyhow::Error| e.to_string())
                }, Msg::Done);
            }
            Msg::Direct(enable) => {
                if !self.can_change_direct(enable){return Task::none()}
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone();
                let remote=if enable {match self.selected.remote(&self.servers) {Ok(server)=>server,Err(error)=>{self.feedback(Err(error.to_string()));return Task::none()}}}else{None};
                self.begin(if enable { "Setting up direct launch…" } else { "Disabling direct launch…" });
                return job(move || { if enable { backend.enable_direct(&settings, remote)?; } else { backend.disable_direct(&settings)?; }
                    Ok(Outcome::Info(if enable { "Direct launch is ready for your saved server and account." } else { "Direct launch disabled." }.into())) });
            }
            Msg::Tick => {
                if self.notice_expires.is_some_and(|until| std::time::Instant::now() >= until) { self.notice.clear(); self.notice_expires = None; }
                if self.staged.is_some() && !self.busy && !self.modal_open() { return self.apply_update() }
                let Some(backend) = self.backend.clone() else { return Task::none() }; let settings = self.settings.clone(); let log_server = self.log_server;
                let simple=!self.settings.advanced_mode; let read_logs=simple || (!self.paused && self.page == Page::Activity);
                let mut tasks=vec![self.run_setup_check(self.setup_revision)];
                if read_logs && !self.logs_polling {
                    self.logs_polling=true; let backend=backend.clone();
                    tasks.push(Task::perform(async move {tokio::task::spawn_blocking(move || if simple {format!("{}{}",backend.logs(true),backend.logs(false))}else{backend.logs(log_server)}).await.map_err(|e|e.to_string())},Msg::LogsPolled));
                }
                // Lifecycle checks take the game mutex; Play holds it while starting a local server.
                // Log reads use their own task so startup never freezes the visible activity stream.
                if !self.polling && !self.busy {
                    self.polling=true;
                    tasks.push(Task::perform(async move {tokio::task::spawn_blocking(move || {
                        let status=if backend.live(&settings).is_some() {Some(backend.status(&settings).map_err(|e|format!("Could not read connection status: {e:#}"))?)}else{None};
                        let failure=backend.game_failure();
                        Ok(LiveStatus {server:status,game_running:backend.game_running(),game_ready:backend.game_ready(),failure})
                    }).await.map_err(|e|e.to_string())?},Msg::Poll));
                }
                return Task::batch(tasks);
            }
            Msg::LogsPolled(result) => {
                self.logs_polling=false;
                match result {
                    Ok(logs) if (!self.settings.advanced_mode || !self.paused) && self.logs!=logs => {
                        self.logs=logs;
                        if self.follow_logs { return widget::operation::snap_to_end("activity-log") }
                    }
                    Err(error)=>self.monitor_failure(format!("Could not refresh logs: {error}")), _=>{}
                }
            }
            Msg::Poll(result) => {
                self.polling = false;
                if let Ok(LiveStatus { server: status, game_running: running, game_ready: ready, failure }) = result {
                    if self.monitor_error.as_ref().is_some_and(|e|e==&self.notice) {self.feedback_at(Page::Play,Ok("Status monitoring recovered.".into()));}
                    self.monitor_error=None;
                    if self.status.is_some() && status.is_none() && running && self.selected==ServerChoice::Local {self.feedback_at(Page::Play,Err("The local connection stopped while your game was open. Close the game, then Play again. Export logs if this repeats.".into()));}
                    let attached = self.status.is_none() && status.is_some(); if let Some(error) = failure { self.feedback_at(Page::Play, Err(error)); }
                    self.status = status; self.game_running = running; self.game_ready = ready;
                    if !running || ready {self.game_start=None;}
                    if attached && self.settings.advanced_mode && self.accounts.is_null() && !self.locked() && !self.failed { return self.load_accounts() }
                } else if let Err(error)=result { self.monitor_failure(error); }
            }
            Msg::Done(result) => {
                self.busy = false;self.starting_game=false;
                match result {
                    Ok(Outcome::Info(info)) => { self.completed(Ok(info)); return self.inspect_setup() }
                    Ok(Outcome::Status(status)) => { self.status = Some(status); self.completed(Ok("Server is running on this computer.".into())); self.startup.resume_hosting = false; return self.load_accounts() }
                    Ok(Outcome::GameStarted) => { self.game_running = true; self.game_ready = false; self.game_start=Some(std::time::Instant::now()); }
                    Ok(Outcome::Accounts(accounts)) => { self.accounts = accounts; self.reconcile_accounts(); }
                    Ok(Outcome::AccountsChanged(accounts, message, selected)) => { let invite = !self.settings.advanced_mode && selected.is_some(); self.accounts = accounts; if let Some(selected) = selected { self.admin_account = selected; } self.reconcile_accounts(); self.new_account.clear(); self.new_player.clear(); self.give_bp.clear(); self.account_form = false; self.completed(Ok(message)); if invite { return self.update(Msg::Export) } }
                    Ok(Outcome::Imported(server)) => { self.reset_editor(); self.identity = server.identity.clone().or_else(|| yo3_tunnel::identity_from_pem(server.certificate.as_bytes()).ok()); self.label = server.label; self.address = server.address; self.key = server.key; self.certificate = server.certificate; self.server_name = server.server_name; self.editor = true; self.page = Page::Play; self.invitation_imported=true; return self.check_invitation() }
                    Ok(Outcome::Observed(server)) => {self.support=false; self.observed = Some(server);},
                    Ok(Outcome::Checked(info)) => { self.checked = Some(info); self.completed(Ok("This computer connected successfully and verified the account.".into())); }
                    Ok(Outcome::Bindings(bindings)) => self.bindings = bindings,
                    Ok(Outcome::SettingsSaved(section, settings)) => {
                        if settings.data_dir != self.settings.data_dir { self.accounts = Value::Null; self.account.clear(); self.admin_account.clear(); self.status = None; }
                        self.draft = section.merge(&self.draft, &settings); if section == Section::Hosting { self.port = settings.listen_port.to_string(); }
                        self.draft.selected_server = settings.selected_server; self.draft.local_account = settings.local_account.clone(); self.settings = settings;
                        self.completed(Ok(if !self.settings.advanced_mode && section==Section::Game { "Game setup saved. Checking it automatically…" } else { "Changes saved." }.into())); return self.inspect_setup();
                    }
                    Ok(Outcome::SelectionSaved(settings)) => { self.draft.selected_server = settings.selected_server; self.draft.local_account = settings.local_account.clone(); self.settings = settings; return self.inspect_setup() }
                    Ok(Outcome::ConnectionsSaved(servers, id)) => { self.selected = ServerChoice::saved(servers.iter().find(|s| s.id == id).unwrap()); self.servers = servers; self.settings.selected_server = Some(id); self.reset_editor(); self.completed(Ok("Connection saved.".into())); return self.save_selection() }
                    Ok(Outcome::ConnectionsRemoved(servers, settings)) => { self.servers = servers; self.settings = settings; self.draft.selected_server = self.settings.selected_server; self.selected = ServerChoice::Local; self.reset_editor(); self.completed(Ok("Saved connection removed.".into())); return self.inspect_setup() }
                    Ok(Outcome::GameDataReset(backup)) => {
                        self.support=false;
                        self.completed(Ok(format!("Local game data reset. Backup saved to {}.",views::display_path(&backup))));
                        self.game_data_backup=Some(backup);self.game_data_dialog=Some(GameDataDialog::Complete);
                        return self.inspect_setup()
                    }
                    Ok(Outcome::Quit) => return iced::exit(),
                    Ok(Outcome::Recovered(settings,servers,backend)) => {
                        self.backend=Some(backend); self.settings=settings; self.draft=self.settings.clone(); self.servers=servers;
                        self.selected=self.settings.selected_server.and_then(|id|self.servers.iter().find(|s|s.id==id)).map(ServerChoice::saved).unwrap_or(ServerChoice::Local);
                        self.account=self.settings.local_account.clone();self.port=self.settings.listen_port.to_string();
                        self.completed(Ok("Saved settings are available. Checking your game automatically…".into())); return self.inspect_setup();
                    }
                    Err(error) => { self.completed(Err(error)); return self.inspect_setup() }
                }
            }
            Msg::Logs(server) => { self.log_server = server; self.logs.clear(); self.follow_logs=true; return widget::operation::snap_to_end("activity-log") }
            Msg::LogScrolled(at_bottom) => self.follow_logs=at_bottom,
            Msg::Pause(paused) => self.paused = paused,
            Msg::LogFilter(filter) => { self.filter=filter; self.follow_logs=true; return widget::operation::snap_to_end("activity-log") }
            Msg::CopyLogs => return iced::clipboard::write(self.shown_logs()),
            Msg::ExportLogs => { self.cancel_bug_task(); self.bug_report=None; self.support=false; self.report_options = true; self.report_remove_personal = false; return self.update(Msg::PrepareReport(false)) }
            Msg::RemovePersonal(value) => { self.report_remove_personal = value; self.report_feedback = None; }
            Msg::PrepareReport(all) => {
                if self.report_busy {return Task::none()}
                let Some(backend) = self.backend.clone() else {self.report_feedback=Some(Err("Logs are unavailable until app setup is repaired. Copy the error, check the data folder permissions and choose Retry app setup.".into())); return Task::none()};
                self.report_all=all;self.report=None;self.report_feedback=None;self.report_busy=true;self.report_generation+=1;let generation=self.report_generation;
                return Task::perform(async move {tokio::task::spawn_blocking(move || backend.report(all).map_err(|e|format!("Could not prepare logs: {e:#}. Try again, or close this dialog and Copy logs."))).await.map_err(|e|e.to_string())?},move|r|Msg::ReportPrepared(generation,r));
            }
            Msg::ReportPrepared(generation,result) => if generation==self.report_generation && self.report_options {
                self.report_busy=false;match result {Ok(report)=>self.report=Some(report),Err(error)=>self.report_feedback=Some(Err(error))}
            },
            Msg::ReportSaved(generation,result) => if generation==self.report_generation && self.report_options {self.report_busy=false;self.report_feedback=Some(result);},
            Msg::ReportDestination(generation,result) => {
                if generation!=self.report_generation || !self.report_options {return Task::none()}
                match result {
                    Ok(Some(path))=>if let Some(report)=self.report.clone() {let remove_personal = self.report_remove_personal; return Task::perform(async move {
                        tokio::task::spawn_blocking(move || report.export_with_privacy(&path, remove_personal)).await.map_err(|e|e.to_string())?.map_err(|e|format!("Could not save ZIP: {e:#}. Choose a writable folder and try Save ZIP again, or Copy logs."))?;
                        Ok("Diagnostic ZIP saved. Nothing was uploaded.".into())
                    },move|r|Msg::ReportSaved(generation,r));},
                    Ok(None)=>{self.report_busy=false;self.report_feedback=Some(Ok("No ZIP selected. Logs have not been saved. Try Save ZIP again, or close this dialog and Copy logs.".into()));}
                    Err(error)=>{self.report_busy=false;self.report_feedback=Some(Err(error));}
                }
            }
            Msg::CopyReportError => if let Some(result)=&self.report_feedback {return iced::clipboard::write(result.as_ref().unwrap_or_else(|e|e).clone())},
            Msg::SaveLogs => {
                if self.report.is_none() {return Task::none()} self.report_busy=true;self.report_feedback=None;let generation=self.report_generation;
                return Task::perform(async move {
                    tokio::time::timeout(std::time::Duration::from_secs(120),rfd::AsyncFileDialog::new().add_filter("ZIP report", &["zip"]).set_file_name("yo3-diagnostics.zip").save_file()).await
                        .map(|file|file.map(|f|f.path().to_path_buf())).map_err(|_|"No ZIP location received within two minutes. Close the file dialog if it is open. Try Save ZIP again, or close this dialog and Copy logs.".into())
                }, move|r|Msg::ReportDestination(generation,r));
            }
            Msg::Close => {
                self.support=false;
                if self.report_options {
                    self.cancel_bug_task(); self.report_options=false;self.bug_report=None;self.report=None;self.report_busy=false;self.report_feedback=None;self.report_generation+=1;
                }
                self.close_game_data_dialog();
                if self.picking {let _=self.update(Msg::UsePath);}
                if self.transfer.is_some() || self.applying { self.feedback(Err("Cancel the download or wait for the update restart before closing.".into())); return Task::none() }
                if self.status.is_some() || self.game_running || self.busy { if !self.closing {self.keep_hosting_on_exit=self.settings.advanced_mode;} self.closing = true; } else { return iced::exit() }
            }
            Msg::CancelClose => { self.cancel_bug_task(); self.close_game_data_dialog(); self.support=false; self.closing = false; self.observed = None; self.report = None; self.bug_report=None; self.report_options = false; self.report_generation+=1;self.report_busy=false; self.review_update = false; self.remove_server = None; }
            Msg::KeepHosting(keep) => if self.closing && self.settings.advanced_mode {self.keep_hosting_on_exit=keep;},
            Msg::Quit(stop_server) => {
                let Some(backend) = self.backend.clone() else { return iced::exit() }; let settings = self.settings.clone(); self.begin("Closing…");
                return job(move || { backend.stop_game()?; if stop_server && backend.live(&settings).is_some() { backend.stop_server(&settings)?; } Ok(Outcome::Quit) });
            }
        }
        Task::none()
    }
    fn begin(&mut self, label: &str) { if let Some(backend) = &self.backend { backend.diagnostics.line(&format!("Operation started: {label}")); } self.busy = true; self.operation_page = self.page; self.operation_label = label.into(); self.notice.clear(); self.failed=false; self.failure_setup=None; }
    fn completed(&mut self, result: Result<String, String>) { self.feedback_at(self.operation_page, result); }
    fn feedback(&mut self, result: Result<String, String>) { self.feedback_at(self.page, result); }
    fn feedback_at(&mut self, page: Page, result: Result<String, String>) {
        self.failure_setup=None;
        self.failed = result.is_err(); self.notice = result.unwrap_or_else(|e| e); self.feedback_page = page;
        self.notice_expires = (!self.failed).then(|| std::time::Instant::now() + std::time::Duration::from_secs(8));
        if let Some(backend) = &self.backend { backend.diagnostics.line(&self.notice); }
    }
    fn monitor_failure(&mut self,error:String) {
        let message=format!("Status could not be refreshed. Retrying automatically.\n{error}\nYour last known game state is shown. Copy or export logs if this continues.");
        if self.monitor_error.as_ref()!=Some(&message) && let Some(backend)=&self.backend {backend.diagnostics.line(&message);}
        self.monitor_error=Some(message.clone());
        if !self.failed {self.feedback_at(Page::Play,Err(message));}
    }
    fn feedback_visible(&self)->bool { !self.notice.is_empty() && (self.failed || !self.settings.advanced_mode || self.feedback_page==self.page) }
    fn present_error(&self,message:&str,hint:error_presentation::Hint)->error_presentation::Presentation {
        let mut prefixes=vec![(self.root.join("profiles").to_string_lossy().into_owned(),"<profile>"),
            (self.root.to_string_lossy().into_owned(),"<app data>"),(self.draft.game_dir.clone(),"<game>"),(self.settings.game_dir.clone(),"<game>")];
        if let Some(backend)=&self.backend {
            prefixes.push((backend.data_dir(&self.settings).to_string_lossy().into_owned(),"<server data>"));
            for file in [&backend.runtime.injector,&backend.runtime.dll,&backend.runtime.launcher] {
                if let Some(path)=file.parent(){prefixes.push((path.to_string_lossy().into_owned(),"<app>"));}
            }
        }
        if let Some(home)=std::env::var_os(if cfg!(windows){"USERPROFILE"}else{"HOME"}) {prefixes.push((home.to_string_lossy().into_owned(),"<user>"));}
        error_presentation::present(message,hint,&prefixes)
    }
    fn failure_hint(&self)->error_presentation::Hint {
        match self.failure_setup {Some(yo3_desktop::Readiness::ComponentProblem(_))=>error_presentation::Hint::Component,
            Some(yo3_desktop::Readiness::GameProblem(_))=>error_presentation::Hint::GameFolder,_=>error_presentation::Hint::General}
    }
    fn error_summary(&self)->String {
        if self.failed {self.present_error(&self.notice,self.failure_hint()).summary}else{self.notice.clone()}
    }
    fn error_diagnosis(&self)->Option<String> {
        if self.failed {self.present_error(&self.notice,self.failure_hint()).diagnosis}else{None}
    }
    fn animating(&self)->bool {self.busy || self.setup_in_progress() || self.invitation_checking || self.report_busy || self.bug_report.as_ref().is_some_and(|bug|bug.auth_busy) || (self.game_running && !self.game_ready) || self.applying || self.transfer.is_some() || (self.page==Page::About && matches!(self.update_status,UpdateStatus::Checking))}
    fn subscription(&self)->Subscription<Msg> {
        let mut subscriptions=vec![iced::time::every(std::time::Duration::from_secs(2)).map(|_|Msg::Tick),iced::window::close_requests().map(|_|Msg::Close),iced::window::open_events().map(|_|Msg::WindowOpened)];
        if self.animating() {subscriptions.push(iced::time::every(std::time::Duration::from_millis(80)).map(|_|Msg::Animate));}
        Subscription::batch(subscriptions)
    }
    fn input(&mut self, field: Field, value: String) -> Task<Msg> {
        let first_game_input=self.first_setup && matches!(field,Field::Game|Field::Wine);
        let game_changed=match field {Field::Game=>self.draft.game_dir!=value,Field::Wine=>self.draft.wine_prefix!=value,_=>false};
        match field {
            Field::Game=>self.draft.game_dir=value,Field::Wine=>self.draft.wine_prefix=value,Field::Host=>self.draft.host_address=value,
            Field::Data=>self.draft.data_dir=value,Field::Player=>self.draft.player_name=value,Field::Label=>self.label=value,
            Field::Address=>if self.address!=value {
                self.address=value;self.server_name.clear();self.identity=None;self.certificate.clear();self.observed=None;self.checked=None;self.invalidate_invitation();
            },
            Field::Key=>if self.key!=value {self.key=value;self.checked=None;self.invalidate_invitation();},
            Field::Account=>self.new_account=value,Field::NewPlayer=>self.new_player=value,Field::GiveBp=>self.give_bp=value,Field::Port=>self.port=value,
        }
        if game_changed || first_game_input {
            self.first_setup=false;self.notice.clear();self.failed=false;self.failure_setup=None;
            return self.queue_setup_check(std::time::Duration::from_millis(650))
        }
        Task::none()
    }
    fn input_immediately(&mut self, field: Field, value: String) -> Task<Msg> {
        let _=self.input(field,value);
        if matches!(field,Field::Game|Field::Wine) {return self.inspect_setup()}
        Task::none()
    }
    fn invalidate_invitation(&mut self) {self.invitation_generation+=1;self.invitation_checking=false;self.invitation_verified=false;}
    fn reset_editor(&mut self) {
        self.invalidate_invitation();self.invitation_imported=false;
        self.editor=false;self.editing_id=None;self.label.clear();self.address.clear();self.key.clear();self.identity=None;self.certificate.clear();self.server_name.clear();self.checked=None;
    }
    fn invitation_server(&self) -> anyhow::Result<SavedServer> {
        let (address,name)=yo3_desktop::server_address(&self.address)?;
        anyhow::ensure!(self.identity.is_some(),"Check the server and confirm its identity before checking the account.");
        let server=SavedServer {id:self.editing_id.unwrap_or(0),label:if self.label.trim().is_empty(){name.clone()}else{self.label.trim().into()},address,
            server_name:if self.server_name.is_empty(){name}else{self.server_name.clone()},certificate:self.certificate.clone(),identity:self.identity.clone(),key:self.key.trim().into()};
        server.validate()?;Ok(server)
    }
    fn invitation_matches(&self, server: &SavedServer) -> bool {
        self.invitation_server().is_ok_and(|current|current.address==server.address && current.server_name==server.server_name
            && current.identity==server.identity && current.certificate==server.certificate && current.key==server.key)
    }
    fn check_invitation(&mut self) -> Task<Msg> {
        if !self.editor || !self.invitation_imported || self.invitation_checking {return Task::none()}
        let Some(backend)=self.backend.clone() else {self.feedback(Err("Repair app setup before checking an invitation.".into()));return Task::none()};
        let server=match self.invitation_server() {Ok(server)=>server,Err(error)=>{self.feedback(Err(error.to_string()));return Task::none()}};
        self.invitation_generation+=1;let generation=self.invitation_generation;self.invitation_checking=true;self.invitation_verified=false;
        self.notice.clear();self.failed=false;self.failure_setup=None;
        Task::perform(async move {
            let checked=server.clone();let result=tokio::task::spawn_blocking(move ||backend.check_connection(&checked)).await
                .map_err(|error|error.to_string()).and_then(|result|result.map_err(|error|format!("{error:#}")));
            (server,result)
        },move |(server,result)|Msg::InvitationChecked(generation,server,result))
    }
    fn reconcile_accounts(&mut self) {
        let names = account_names(&self.accounts);
        if !names.contains(&self.account) { self.account = names.first().cloned().unwrap_or_default(); }
        if !names.contains(&self.admin_account) { self.admin_account = names.first().cloned().unwrap_or_default(); }
    }
    fn setup_candidate(&self) -> Settings {Section::Game.merge(&self.settings,&self.draft)}
    fn error_report(&self)->String {
        let mut report=self.notice.clone();
        if !self.setup_missing_files.is_empty() {
            report.push_str("\n\nMissing required files:\n");
            report.push_str(&self.setup_missing_files.iter().map(|path|path.display().to_string()).collect::<Vec<_>>().join("\n"));
        }
        report
    }
    fn inspect_setup(&mut self) -> Task<Msg> {self.queue_setup_check(std::time::Duration::ZERO)}
    fn queue_setup_check(&mut self, delay: std::time::Duration) -> Task<Msg> {
        self.setup_revision+=1;self.setup_verified=None;self.setup_result=None;self.setup_missing_files.clear();
        if self.failure_setup.is_some() {self.notice.clear();self.failed=false;self.failure_setup=None;}
        if self.backend.is_none() {
            self.setup_checking=false;self.setup_due=None;return Task::none()
        }
        if self.draft.game_dir.trim().is_empty() {
            self.setup_checking=false;self.setup_due=None;
            self.setup_result=Some(yo3_desktop::Readiness::GameProblem("Choose the folder containing yo3.exe.".into()));return Task::none()
        }
        self.setup_checking=true;self.setup_due=Some(std::time::Instant::now()+delay);let revision=self.setup_revision;
        if delay.is_zero() {return self.run_setup_check(revision)}
        Task::perform(async move {tokio::time::sleep(delay).await;revision},Msg::RunSetupCheck)
    }
    fn run_setup_check(&mut self, revision: u64) -> Task<Msg> {
        if revision!=self.setup_revision || self.setup_due.is_none_or(|due|due>std::time::Instant::now())
            || self.setup_active.is_some() || self.locked() || self.modal_open() || self.editor || self.game_running {return Task::none()}
        if self.setup_verified.as_ref().is_some_and(|checked|setup_matches(checked,&self.setup_candidate())) {return self.finish_setup_check()}
        let Some(backend)=self.backend.clone() else {return Task::none()};let settings=self.setup_candidate();
        let remote=match self.selected.remote(&self.servers) {Ok(remote)=>remote,Err(error)=>{
            let result=yo3_desktop::Readiness::ValidationProblem(error.to_string());self.setup_due=None;self.setup_checking=false;self.setup_result=Some(result.clone());
            if !self.failed || self.failure_setup.is_some(){self.feedback(Err(error.to_string()));self.failure_setup=Some(result);}return Task::none()
        }};
        self.setup_active=Some(revision);self.setup_due=None;self.setup_checking=true;
        Task::perform(async move {
            let checked=settings.clone();let result=tokio::task::spawn_blocking(move || {
                let result=match backend.verify_setup(&settings,remote.as_ref()) {
                    Ok(())=>yo3_desktop::Readiness::Ready,
                    Err(error)=>match backend.readiness(&settings) {yo3_desktop::Readiness::Ready=>yo3_desktop::Readiness::ValidationProblem(format!("{error:#}")),problem=>problem},
                };
                let diagnostic=match &result {yo3_desktop::Readiness::Ready=>None,yo3_desktop::Readiness::GameProblem(error)|yo3_desktop::Readiness::ComponentProblem(error)|yo3_desktop::Readiness::ValidationProblem(error)=>Some(error.as_str())};
                let missing_files=backend.missing_required_files(&settings,diagnostic);
                (result,backend.direct_state(&settings),missing_files)
            }).await;
            match result {Ok((result,direct,missing_files))=>(checked,result,direct,missing_files),Err(error)=>(checked,yo3_desktop::Readiness::ValidationProblem(error.to_string()),yo3_desktop::direct::State::Disabled,Vec::new())}
        },move |(settings,result,direct,missing_files)|Msg::SetupChecked(revision,settings,result,direct,missing_files))
    }
    fn finish_setup_check(&mut self) -> Task<Msg> {
        if self.setup_verified.as_ref().is_none_or(|settings|!setup_matches(settings,&self.setup_candidate())) {return Task::none()}
        if !self.changed(Section::Game) {self.setup_checking=false;self.setup_due=None;return Task::none()}
        self.setup_due=Some(std::time::Instant::now());
        if self.locked() || self.modal_open() || self.game_running {return Task::none()}
        let Some(backend)=self.backend.clone() else {return Task::none()};let candidate=self.setup_candidate();let settings=candidate.clone();let revision=self.setup_revision;
        self.setup_saving=Some(revision);self.begin("Saving verified game setup…");
        Task::perform(async move {
            tokio::task::spawn_blocking(move ||backend.save_preferences(Section::Game,&settings)).await
                .map_err(|error|error.to_string()).and_then(|result|result.map_err(|error|format!("{error:#}")))
        },move |result|Msg::SetupSaved(revision,candidate.clone(),result))
    }
    fn setup_in_progress(&self) -> bool {self.setup_checking || self.setup_due.is_some() || self.setup_saving.is_some()}
    fn game_start_stalled(&self) -> bool {self.game_running && !self.game_ready && self.game_start.is_some_and(|started|started.elapsed().as_secs()>=30)}
    fn play_loading(&self) -> bool {self.starting_game || self.setup_in_progress() || (self.game_running && !self.game_ready && !self.game_start_stalled())}
    fn play_label(&self) -> &'static str {
        if self.game_running {if self.game_ready || self.game_start_stalled() {"Close game"}else{"Starting…"}}
        else if self.starting_game {"Starting…"}
        else if self.setup_in_progress() {"Checking…"}
        else if !self.first_setup && !self.draft.game_dir.trim().is_empty() && self.setup_result.as_ref().is_some_and(|result|*result!=yo3_desktop::Readiness::Ready) {"Retry check"}
        else {"Play"}
    }
    fn play_message(&self) -> Option<Msg> {
        if self.game_running {return (!self.busy && !self.applying && (self.game_ready || self.game_start_stalled())).then_some(Msg::StopGame)}
        if self.locked() || self.setup_in_progress() || self.editor || self.backend.is_none() {return None}
        if self.can_play() {Some(Msg::Play)}else if self.play_label()=="Retry check" {Some(Msg::ValidateGame)}else{None}
    }
    fn play_status(&self) -> &str {
        if self.backend.is_none() {return "App setup needs attention. Check the error below, then retry."}
        if self.starting_game {return "Rechecking required files and preparing your game…"}
        if self.game_running {return if self.game_ready {"Game running"}else if self.game_start_stalled(){"Still starting. Choose Close game and retry if it stays here; logs have the details."}else{"Opening the game and waiting for the connection mod…"}}
        if self.picking {return "Choose your game folder in the file dialog, or paste its path instead."}
        if self.setup_saving.is_some() {return "Checks passed. Saving your game folder…"}
        if self.setup_in_progress() {return if self.setup_verified.is_some(){"Checks passed. Waiting to finish saving your setup…"}else{"Checking your game files, runtime and connection…"}}
        if self.editor {return "Finish or cancel Add/Edit server in Advanced before playing."}
        if self.draft.game_dir.trim().is_empty() || self.first_setup {return "Choose your original game folder to get started."}
        if self.busy {return &self.operation_label}
        match &self.setup_result {
            Some(yo3_desktop::Readiness::Ready)=>"Ready to play",
            Some(yo3_desktop::Readiness::GameProblem(_))=>"Check the highlighted game folder.",
            Some(yo3_desktop::Readiness::ComponentProblem(_))=>"Restore the required Revival files, then retry.",
            Some(yo3_desktop::Readiness::ValidationProblem(_))=>"Setup check failed. Review the error below, then retry.",
            None=>"Choose your game folder. It will be checked automatically.",
        }
    }
    fn game_field_error(&self) -> Option<String> {
        if self.backend.is_none() || self.setup_in_progress() || self.first_setup || self.draft.game_dir.trim().is_empty() {return None}
        match &self.setup_result {Some(yo3_desktop::Readiness::GameProblem(error))=>{
            let display=self.present_error(error,error_presentation::Hint::GameFolder);Some(display.diagnosis.unwrap_or(display.summary))},_=>None}
    }
    fn play_tone(&self) -> ui::Tone {
        if self.backend.is_none() {return ui::Tone::Danger}
        if self.starting_game || self.setup_in_progress() || (self.game_running && !self.game_ready && !self.game_start_stalled()) {return ui::Tone::Info}
        if self.game_start_stalled() || self.editor {return ui::Tone::Warning}
        if self.game_running && self.game_ready {return ui::Tone::Success}
        if self.first_setup || self.draft.game_dir.trim().is_empty() {return ui::Tone::Neutral}
        if self.busy {return ui::Tone::Info}
        match &self.setup_result {Some(yo3_desktop::Readiness::Ready)=>ui::Tone::Success,Some(_)=>ui::Tone::Danger,None=>ui::Tone::Neutral}
    }
    fn shown_logs(&self) -> String {
        let filter = if self.settings.advanced_mode { self.filter.to_lowercase() } else { String::new() };
        self.logs.lines().filter(|line| if filter.is_empty() { !yo3_tunnel::diagnostics::is_trace(line) } else { line.to_lowercase().contains(&filter) }).collect::<Vec<_>>().join("\n")
    }
    fn can_play(&self) -> bool { matches!(self.setup_result, Some(yo3_desktop::Readiness::Ready)) && !self.locked() && !self.game_running && !self.editor && self.backend.is_some()
        && !self.setup_in_progress() && !Section::Game.changed(&self.settings,&self.draft) }
    fn direct_checked(&self)->bool {
        matches!(self.direct_state,yo3_desktop::direct::State::Enabled|yo3_desktop::direct::State::NeedsRepair(_))
    }
    fn can_change_direct(&self,enable:bool)->bool {
        if self.backend.is_none() || self.locked() || self.game_running || self.changed(Section::Game) || self.setup_in_progress(){return false}
        if !enable{return self.direct_checked()}
        matches!(self.setup_result,Some(yo3_desktop::Readiness::Ready))
            && matches!(self.direct_state,yo3_desktop::direct::State::Disabled|yo3_desktop::direct::State::NeedsRepair(_))
            && match self.selected.remote(&self.servers){Ok(None)=>self.status.is_some(),Ok(Some(_))=>true,Err(_)=>false}
    }
    fn game_data_reset_request(&self)->Option<(Settings,yo3_desktop::game_data::Preview,bool)> {
        let Some(GameDataDialog::Ready {settings,preview,keep_players})=&self.game_data_dialog else {return None};
        if self.backend.is_none() || self.locked() || self.game_running || settings!=&self.settings || self.changed(Section::Game)
            || (if *keep_players {!preview.settings_present}else{!preview.full_present}) {return None}
        Some((settings.clone(),preview.clone(),!*keep_players))
    }
    fn close_game_data_dialog(&mut self) {
        if matches!(&self.game_data_dialog,Some(GameDataDialog::Checking {..})) {self.busy=false;}
        self.game_data_generation+=1;self.game_data_dialog=None;
    }
    fn changed(&self, section: Section) -> bool { section.changed(&self.settings, &self.draft) || (section == Section::Hosting && self.port != self.settings.listen_port.to_string()) }
    fn modal_open(&self) -> bool { self.support || self.review_update || self.report_options || self.observed.is_some() || self.closing || self.remove_server.is_some() || self.game_data_dialog.is_some() }
    fn locked(&self) -> bool { self.busy || self.report_busy || self.transfer.is_some() || self.staged.is_some() || self.applying }
    fn save_selection(&mut self) -> Task<Msg> {
        self.setup_revision+=1;self.setup_verified=None;self.setup_result=None;self.setup_due=None;self.setup_checking=false;
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
            if let Err(error)=&result && hosting && backend.live(&settings).is_none() && let Err(restart)=backend.start_server(&settings) {return Err(format!("Update failed: {error:#}\nYour server could not restart: {restart:#}. This launcher is still active. Try starting again or export logs."));}
            result.map_err(|e| format!("{e:#}"))
        }).await.map_err(|e| e.to_string())? }, Msg::UpdateApplied)
    }
}
fn setup_matches(a: &Settings, b: &Settings) -> bool {let mut comparable=a.clone();comparable.advanced_mode=b.advanced_mode;comparable==*b}
fn check_updates() -> Task<Msg> { Task::perform(async { updates::check().await.map_err(|e| format!("{e:#}")) }, Msg::UpdateChecked) }
fn release_web_link(link: &str) -> Result<String,String> {
    let url=reqwest::Url::parse(link).map_err(|_|"This release link is not a complete web address. Open the release page to review it.".to_string())?;
    if !matches!(url.scheme(),"http"|"https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("This release link cannot be opened. Only HTTP or HTTPS links without embedded credentials are supported. Open the release page to review it.".into())
    }
    Ok(url.into())
}
fn open_external(url: String) -> Task<Msg> {
    Task::perform(async move { tokio::task::spawn_blocking(move || open(&url)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string()) }, Msg::ExternalOpened)
}
/// A Give BP amount the host typed: a whole number from 1 to 1,000,000.
fn give_bp_amount(input: &str) -> Option<u32> { input.trim().parse::<u32>().ok().filter(|n| (1..=1_000_000).contains(n)) }
fn account_names(accounts: &Value) -> Vec<String> { accounts.as_array().into_iter().flatten().filter_map(|a| a["players"][0]["name"].as_str().map(str::to_string)).collect() }
fn open(path: &str) -> anyhow::Result<()> {
    let result:anyhow::Result<()>= {
        #[cfg(windows)] {
            use std::os::windows::ffi::OsStrExt;
            let wide:Vec<u16>=std::ffi::OsStr::new(path).encode_wide().chain(Some(0)).collect();
            let result=unsafe {windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(),std::ptr::null(),wide.as_ptr(),std::ptr::null(),std::ptr::null(),windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL)};
            if result as isize>32 {Ok(())}else{Err(anyhow::anyhow!("No application could open this location (Windows error {})",result as isize))}
        }
        #[cfg(not(windows))] {wait_for_opener(std::process::Command::new("xdg-open").arg(path),std::time::Duration::from_secs(15))}
    };
    result.map_err(|e|anyhow::anyhow!("Could not open the browser or file manager. Copy this location and open it manually:\n{path}\n{e:#}"))
}
#[cfg(not(windows))]
fn wait_for_opener(cmd:&mut std::process::Command,limit:std::time::Duration)->anyhow::Result<()> {
    let mut child=cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?;
    let deadline=std::time::Instant::now()+limit;
    loop {
        if let Some(status)=child.try_wait()? {anyhow::ensure!(status.success(),"The system opener failed ({status})");return Ok(())}
        if std::time::Instant::now()>=deadline {let _=child.kill();let _=child.wait();anyhow::bail!("The system opener did not confirm within {} seconds",limit.as_secs());}
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    #[test]
    fn direct_launch_changes_require_a_ready_saved_idle_setup_and_our_own_loader() {
        use yo3_desktop::{Readiness,direct::State};
        let root=std::env::temp_dir().join(format!("yo3-ui-direct-guards-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        for (state,enable,ready,dirty,running,checking) in [
            (State::Disabled,true,false,false,false,false),(State::Enabled,false,false,true,false,false),
            (State::Disabled,true,true,true,false,false),(State::Enabled,false,true,false,true,false),
            (State::Disabled,true,true,false,false,true),(State::Enabled,false,true,false,false,true),
            (State::Conflict("Another mod owns WINMM".into()),true,true,false,false,false),
            (State::Conflict("Another mod owns WINMM".into()),false,true,false,false,false),
            (State::Disabled,false,true,false,false,false),(State::Enabled,true,true,false,false,false),
            (State::Disabled,true,true,false,false,false),
        ] {
            let mut app=App::load(root.clone());app.direct_state=state;app.game_running=running;app.setup_checking=checking;
            app.setup_result=Some(if ready{Readiness::Ready}else{Readiness::ComponentProblem("Missing connection component".into())});
            if dirty{app.draft.game_dir="Unsaved game folder".into();}
            let draft=app.draft.clone();let saved=app.settings.clone();let _=app.update(Msg::Direct(enable));
            assert!(!app.busy,"direct-launch change should be blocked for unsaved, running, unchecked, unready or unowned state");
            assert_eq!(app.draft,draft);assert_eq!(app.settings,saved);
        }
        for state in [State::Disabled,State::NeedsRepair("Refresh owned loader".into())] {
            let mut app=App::load(root.clone());app.direct_state=state;app.setup_result=Some(Readiness::Ready);app.status=Some(serde_json::json!({}));
            let checked=app.direct_checked();assert!(app.can_change_direct(true));let _=app.update(Msg::Direct(true));assert!(app.busy,"valid enable and repair operations should queue their work");
            assert_eq!(app.direct_checked(),checked,"the checkbox must retain the installed state while the change is pending");
            assert!(!app.can_change_direct(true) && !app.can_change_direct(false),"pending changes must block a second toggle");
        }
        let mut unavailable=App::load(root.clone());unavailable.backend=None;unavailable.direct_state=State::Enabled;unavailable.setup_result=Some(Readiness::Ready);
        assert!(!unavailable.can_change_direct(false));assert!(!unavailable.can_change_direct(true));let _=unavailable.update(Msg::Direct(false));assert!(!unavailable.busy);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn direct_launch_can_be_removed_with_failed_checks_and_a_deleted_saved_connection() {
        use yo3_desktop::{Readiness,direct::State};
        let root=std::env::temp_dir().join(format!("yo3-ui-direct-removal-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        for state in [State::Enabled,State::NeedsRepair("The matching loader needs refresh".into())] {
            let mut app=App::load(root.clone());app.direct_state=state;app.setup_result=Some(Readiness::ComponentProblem("Missing DLL".into()));
            app.selected=ServerChoice::Saved {id:999,title:"Removed connection".into()};let saved=app.settings.clone();let draft=app.draft.clone();
            assert!(app.direct_checked() && app.can_change_direct(false));assert!(!app.can_change_direct(true));
            let _=app.update(Msg::Direct(false));assert!(app.busy,"removing our loader must not require a working server or connection");
            assert!(app.direct_checked(),"the checkbox must stay checked until removal completes");
            assert!(!app.failed);assert_eq!(app.operation_label,"Disabling direct launch…");assert_eq!(app.settings,saved);assert_eq!(app.draft,draft);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn direct_launch_enable_needs_a_running_local_server_but_remote_selection_does_not() {
        use yo3_desktop::{Readiness,direct::State};
        let root=std::env::temp_dir().join(format!("yo3-ui-direct-connections-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.direct_state=State::Disabled;app.setup_result=Some(Readiness::Ready);
        assert!(!app.direct_checked() && !app.can_change_direct(true));let _=app.update(Msg::Direct(true));assert!(!app.busy);
        app.status=Some(serde_json::json!({}));assert!(app.can_change_direct(true));app.status=None;
        let mut remote=invitation_fixture(&root);remote.id=7;app.servers.push(remote.clone());app.selected=ServerChoice::saved(&remote);
        assert!(app.can_change_direct(true));let _=app.update(Msg::Direct(true));assert!(app.busy);
        app.busy=false;app.selected=ServerChoice::Saved{id:99,title:"Deleted connection".into()};assert!(!app.can_change_direct(true));
        app.direct_state=State::Enabled;assert!(app.direct_checked() && app.can_change_direct(false));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn automatic_setup_failures_do_not_replace_a_native_failure_or_mislabel_its_context() {
        let root=std::env::temp_dir().join(format!("yo3-ui-native-evidence-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.first_setup=false;app.settings.game_dir="Game".into();app.draft=app.settings.clone();
        let native="The game could not start (exit status: 1).\nstage=game-exit process_exit_code=0xc0000005: Game exited with an error.";
        let _=app.update(Msg::Done(Err(native.into())));let revision=app.setup_revision;app.setup_active=Some(revision);
        let _=app.update(Msg::SetupChecked(revision,app.setup_candidate(),yo3_desktop::Readiness::ComponentProblem("Required component not found: yo3-auth.dll".into()),yo3_desktop::direct::State::Disabled,Vec::new()));
        assert_eq!(app.notice,native,"background preflight must retain the primary native failure for the screenshot and exported logs");
        assert!(app.failure_setup.is_none(),"an unrelated setup result must not reclassify the native crash");
        assert!(matches!(app.setup_result,Some(yo3_desktop::Readiness::ComponentProblem(_))));assert!(!app.can_play());
        assert_eq!(app.error_summary(),"The game crashed.");assert!(app.error_diagnosis().unwrap().contains("0xC0000005"));let _=app.view();
        let _=app.update(Msg::ValidateGame);let revision=app.setup_revision;app.setup_active=Some(revision);
        let _=app.update(Msg::SetupChecked(revision,app.setup_candidate(),yo3_desktop::Readiness::ComponentProblem("Required component not found: yo3-auth.dll".into()),yo3_desktop::direct::State::Disabled,Vec::new()));
        assert!(app.notice.contains("yo3-auth.dll") && app.failure_setup.is_some(),"an explicit retry begins a new attempt and can show its own setup failure");
        app.feedback(Err("Could not save a ZIP".into()));assert_eq!(app.error_summary(),"Could not save a ZIP");assert!(app.error_diagnosis().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn release_notes_parse_once_per_response_and_clear_when_no_release_is_available() {
        use widget::markdown::{Item,HeadingLevel};
        let root=std::env::temp_dir().join(format!("yo3-ui-release-notes-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());
        let release=|notes:&str|updates::Decision::Available(updates::Release {tag:"v0.0.7".into(),url:updates::RELEASES.into(),notes:notes.into(),
            asset:updates::Asset {name:"desktop".into(),url:String::new(),bytes:100,sha256:String::new()}});
        let _=app.update(Msg::UpdateChecked(Ok(release("## Fixed\n\n- **Decks** keep `saved` edits.\n- [Details](https://github.com/usersina/yo3-revival)\n\n```text\nexample\n```\n\n![Image description](https://example.com/large.png)"))));
        assert!(matches!(app.release_notes.first(),Some(Item::Heading(HeadingLevel::H2,_))),"release headings must be parsed instead of displayed as raw Markdown");
        assert!(app.release_notes.iter().any(|item|matches!(item,Item::List {bullets,..} if bullets.len()==2)));
        assert!(app.release_notes.iter().any(|item|matches!(item,Item::CodeBlock {code,..} if code.trim()=="example")));
        assert!(app.release_notes.iter().any(|item|matches!(item,Item::Image {..})),"native image-alt handling must remain available without downloads");
        app.page=Page::About;let cached=app.release_notes.as_ptr();let _=app.view();let _=app.update(Msg::Animate);let _=app.view();
        let _=app.update(Msg::ReviewUpdate);let _=app.view();assert!(app.review_update);assert_eq!(cached,app.release_notes.as_ptr(),"both views and redraws must reuse the parsed notes");
        let _=app.update(Msg::CancelClose);
        let _=app.update(Msg::UpdateChecked(Ok(release("Replacement notes"))));assert_eq!(app.release_notes.len(),1);assert!(matches!(app.release_notes[0],Item::Paragraph(_)));
        let _=app.update(Msg::UpdateChecked(Err("offline".into())));assert!(app.release_notes.is_empty());
        let _=app.update(Msg::UpdateChecked(Ok(release("## Another release"))));assert!(!app.release_notes.is_empty());
        app.review_update=true;let _=app.update(Msg::UpdateChecked(Ok(updates::Decision::NoReleases)));assert!(app.release_notes.is_empty());
        assert!(!app.review_update,"a late non-release response must not leave an invisible modal blocking the app");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn release_note_links_accept_only_web_urls_and_fail_visibly_during_review() {
        assert_eq!(release_web_link("https://github.com/usersina/yo3-revival/issues/4").unwrap(),"https://github.com/usersina/yo3-revival/issues/4");
        assert_eq!(release_web_link("http://localhost:7443/help").unwrap(),"http://localhost:7443/help");
        for link in ["file:///tmp/game.exe","javascript:alert(1)","data:text/html,test","mailto:test@example.com","yo3://localhost", "relative/help", "https://user:secret@example.com/help"] {
            assert!(release_web_link(link).is_err(),"non-web or credential-bearing link accepted: {link}");
        }
        let root=std::env::temp_dir().join(format!("yo3-ui-release-links-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.page=Page::About;
        let _=app.update(Msg::UpdateChecked(Ok(updates::Decision::Available(updates::Release {tag:"v0.0.7".into(),url:updates::RELEASES.into(),notes:"## Fixed\n\nNotes".into(),
            asset:updates::Asset {name:"desktop".into(),url:String::new(),bytes:100,sha256:String::new()}}))));
        let _=app.update(Msg::ReviewUpdate);let _=app.update(Msg::OpenReleaseLink("file:///tmp/blocked.exe".into()));
        assert!(app.review_update && app.failed && app.feedback_visible());assert!(app.notice.contains("Only HTTP or HTTPS"));
        let _=app.update(Msg::ExternalOpened(Err("Could not open the browser. Copy this location and open it manually.\nhttps://github.com/usersina/yo3-revival".into())));
        assert!(app.review_update && app.failed);assert!(app.notice.contains("Copy this location"));let _=app.view();
        std::fs::remove_dir_all(root).unwrap();
    }
    fn invitation_fixture(root: &std::path::Path) -> SavedServer {
        let identity=yo3_server::auth::initialize(&root.join("identity"),vec!["localhost".into()]).unwrap();
        let certificate=std::fs::read_to_string(identity.certificate).unwrap();
        SavedServer {id:0,label:"Friends".into(),address:"127.0.0.1:27499".into(),server_name:"localhost".into(),
            identity:Some(yo3_tunnel::identity_from_pem(certificate.as_bytes()).unwrap()),certificate,key:"x".repeat(20)}
    }
    #[test]
    fn invitations_check_on_import_and_stale_success_cannot_approve_an_edited_key() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-invite-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let server=invitation_fixture(&root);let _=app.update(Msg::Done(Ok(Outcome::Imported(server.clone()))));
        let old=app.invitation_generation;assert!(app.invitation_imported && app.invitation_checking);assert!(!app.invitation_verified);
        let _=app.update(Msg::SaveServer);assert!(!app.busy);assert!(LauncherDb::open(&root).unwrap().servers().unwrap().is_empty());
        let _=app.update(Msg::Input(Field::Key,"y".repeat(20)));let _=app.update(Msg::CheckInvitation);let current=app.invitation_generation;
        let _=app.update(Msg::InvitationChecked(old,server,Ok(yo3_tunnel::ServerInfo::current(true))));assert!(app.invitation_checking);assert!(!app.invitation_verified);
        let checked=app.invitation_server().unwrap();let _=app.update(Msg::InvitationChecked(current,checked,Ok(yo3_tunnel::ServerInfo::current(true))));assert!(app.invitation_verified && !app.invitation_checking);
        let _=app.view();let _=app.update(Msg::AddServer);assert!(!app.editor);assert!(app.checked.is_none(),"a cancelled invitation cannot label the selected server as verified");std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cancelled_invitation_checks_cannot_replace_the_current_error_or_reopen_the_form() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-invite-cancel-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let server=invitation_fixture(&root);let _=app.update(Msg::Done(Ok(Outcome::Imported(server.clone()))));let generation=app.invitation_generation;
        let _=app.update(Msg::AddServer);app.feedback(Err("Current unrelated error".into()));
        let _=app.update(Msg::InvitationChecked(generation,server,Err("Old failed check".into())));assert!(!app.editor && !app.invitation_checking);assert_eq!(app.notice,"Current unrelated error");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_reachable_invitation_with_an_unavailable_engine_is_not_labelled_ready() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-invite-unready-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let server=invitation_fixture(&root);let _=app.update(Msg::Done(Ok(Outcome::Imported(server.clone()))));
        let generation=app.invitation_generation;let _=app.update(Msg::InvitationChecked(generation,server,Ok(yo3_tunnel::ServerInfo::current(false))));
        assert!(!app.invitation_verified);assert!(app.failed);let _=app.update(Msg::SaveServer);assert!(!app.busy);let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn starting_button_prevents_double_launch_and_only_offers_close_after_a_stall() {
        let root=std::env::temp_dir().join(format!("yo3-ui-dynamic-button-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.starting_game=true;app.busy=true;assert_eq!(app.play_label(),"Starting…");assert!(app.play_message().is_none());assert!(app.play_loading());
        app.starting_game=false;app.busy=false;app.game_running=true;app.game_start=Some(std::time::Instant::now());assert!(app.play_message().is_none());
        app.game_start=Some(std::time::Instant::now()-std::time::Duration::from_secs(31));assert_eq!(app.play_label(),"Close game");assert!(matches!(app.play_message(),Some(Msg::StopGame)));assert!(!app.play_loading());
        let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn current_setup_check_retains_every_missing_path_without_replacing_the_launch_failure() {
        let root=std::env::temp_dir().join(format!("yo3-ui-missing-files-current-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let native="Game startup failed at injection: Windows error 126 (0x7e).";
        app.feedback(Err(native.into()));let revision=app.setup_revision;app.setup_active=Some(revision);
        let missing=vec![PathBuf::from("application/yo3-auth-launch.exe"),PathBuf::from("application/yo3-auth.dll"),PathBuf::from("game/yo3.idx")];
        let _=app.update(Msg::SetupChecked(revision,app.setup_candidate(),yo3_desktop::Readiness::ComponentProblem("Required files are missing".into()),yo3_desktop::direct::State::Disabled,missing.clone()));
        assert_eq!(app.setup_missing_files,missing,"cache the complete exact-path list even when an earlier preflight check failed");
        assert_eq!(app.notice,native,"background file discovery must preserve the original startup failure");assert!(app.failed);
        let report=app.error_report();assert!(report.starts_with(native),"copied diagnostics must keep the original failure first");
        let logs=app.backend.as_ref().unwrap().logs(false);
        for path in &missing {let path=path.display().to_string();assert!(report.contains(&path),"Copy error lost a missing path");assert!(logs.contains(&path),"exportable diagnostics lost a missing path");}
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_paths_found_after_verification_revoke_ready_before_saving_or_playing() {
        let root=std::env::temp_dir().join(format!("yo3-ui-missing-files-ready-race-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let saved=app.settings.clone();let _=app.update(Msg::Input(Field::Game,"verified then changed folder".into()));
        let revision=app.setup_revision;app.setup_active=Some(revision);let missing=vec![PathBuf::from("application/yo3-auth.dll")];
        let _=app.update(Msg::SetupChecked(revision,app.setup_candidate(),yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,missing.clone()));
        assert_eq!(app.setup_missing_files,missing);assert!(!app.can_play(),"a missing file discovered after preflight must revoke readiness");
        assert!(app.setup_verified.is_none() && app.setup_saving.is_none(),"a missing setup must not enter automatic saving");assert!(!app.busy);
        assert_eq!(app.settings,saved);assert!(app.notice.contains("yo3-auth.dll"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stale_setup_results_cannot_replace_the_current_missing_path_list() {
        let root=std::env::temp_dir().join(format!("yo3-ui-missing-files-stale-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let current=vec![PathBuf::from("current/yo3-auth.dll")];app.setup_missing_files=current.clone();
        app.setup_revision=2;app.setup_active=Some(1);let candidate=app.setup_candidate();
        let _=app.update(Msg::SetupChecked(1,candidate.clone(),yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,vec![PathBuf::from("old/yo3.exe")]));
        assert_eq!(app.setup_missing_files,current,"superseded active checks must not publish old paths");
        app.setup_active=Some(2);let mut wrong_folder=candidate.clone();wrong_folder.game_dir="different folder".into();
        let _=app.update(Msg::SetupChecked(2,wrong_folder,yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,vec![PathBuf::from("wrong/yo3.exe")]));
        assert_eq!(app.setup_missing_files,current,"a result for a different candidate must not publish paths");
        let _=app.update(Msg::SetupChecked(1,candidate,yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));
        assert_eq!(app.setup_missing_files,current,"late duplicate results must not clear current evidence");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn changing_the_game_folder_or_wine_prefix_clears_old_missing_paths_immediately() {
        let root=std::env::temp_dir().join(format!("yo3-ui-missing-files-input-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());
        for (field,value) in [(Field::Game,"different game folder"),(Field::Wine,"different Wine prefix")] {
            app.setup_missing_files=vec![PathBuf::from("previous/yo3.exe")];let _=app.update(Msg::Input(field,value.into()));
            assert!(app.setup_missing_files.is_empty(),"changing the candidate must immediately clear previous missing-path evidence");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn automatic_game_checks_debounce_typing_and_ignore_superseded_results() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-debounce-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.setup_result=Some(yo3_desktop::Readiness::Ready);
        let saved=app.settings.clone();let _=app.update(Msg::Input(Field::Game,"first folder".into()));let old=app.setup_revision;
        assert!(!app.can_play());assert!(app.setup_in_progress());assert!(app.setup_active.is_none());
        let _=app.update(Msg::Input(Field::Game,"second folder".into()));let latest=app.setup_revision;
        let _=app.update(Msg::RunSetupCheck(old));assert!(app.setup_active.is_none());
        app.setup_due=Some(std::time::Instant::now());let _=app.update(Msg::RunSetupCheck(latest));assert_eq!(app.setup_active,Some(latest));
        let candidate=app.setup_candidate();let _=app.update(Msg::Input(Field::Game,"third folder".into()));app.begin("Unrelated operation…");
        let _=app.update(Msg::SetupChecked(latest,candidate,yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));
        assert!(app.busy,"an old check cannot complete another operation");assert_eq!(app.settings,saved);assert!(!app.can_play());assert!(app.setup_verified.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_current_successful_check_saves_only_the_game_section_before_enabling_play() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-save-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let old=app.settings.game_dir.clone();
        app.draft.host_address="unfinished hosting change".into();let _=app.update(Msg::Input(Field::Game,"verified folder".into()));
        app.setup_due=Some(std::time::Instant::now());let revision=app.setup_revision;let _=app.update(Msg::RunSetupCheck(revision));let candidate=app.setup_candidate();
        let _=app.update(Msg::SetupChecked(revision,candidate.clone(),yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));
        assert!(app.busy);assert!(!app.can_play());assert_eq!(app.settings.game_dir,old,"read-only completion precedes the save");
        let _=app.update(Msg::SetupSaved(revision,candidate.clone(),Ok(candidate)));
        assert!(!app.busy);assert!(app.can_play());assert_eq!(app.settings.game_dir,"verified folder");assert_eq!(app.draft.host_address,"unfinished hosting change");
        assert_ne!(app.settings.host_address,"unfinished hosting change");let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn automatic_check_failure_preserves_saved_folder_and_exposes_retry() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-failure-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let saved=app.settings.clone();let _=app.update(Msg::Input(Field::Game,"broken folder".into()));
        app.setup_due=Some(std::time::Instant::now());let revision=app.setup_revision;let _=app.update(Msg::RunSetupCheck(revision));let candidate=app.setup_candidate();
        let _=app.update(Msg::SetupChecked(revision,candidate,yo3_desktop::Readiness::GameProblem("Missing yo3.exe".into()),yo3_desktop::direct::State::Disabled,Vec::new()));
        assert_eq!(app.settings,saved);assert!(!app.busy && !app.can_play());assert_eq!(app.play_label(),"Retry check");assert!(app.failed);
        let _=app.update(Msg::ValidateGame);assert!(app.setup_in_progress());assert!(!app.failed);let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn restoring_the_saved_folder_invalidates_an_unfinished_check_and_preserves_hosting_drafts() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-restore-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let saved=app.settings.game_dir.clone();app.draft.host_address="unfinished sharing address".into();
        let _=app.update(Msg::Input(Field::Game,"unsaved folder".into()));app.setup_due=Some(std::time::Instant::now());let old=app.setup_revision;let _=app.update(Msg::RunSetupCheck(old));let candidate=app.setup_candidate();
        let _=app.update(Msg::CancelSettings(Section::Game));assert_eq!(app.draft.game_dir,saved);assert_eq!(app.draft.host_address,"unfinished sharing address");
        let _=app.update(Msg::SetupChecked(old,candidate,yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));assert!(!app.busy);assert!(app.setup_saving.is_none());assert!(!app.can_play());assert!(app.setup_in_progress());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn switching_modes_during_a_check_keeps_it_current_and_empty_paths_never_launch_checks() {
        let root=std::env::temp_dir().join(format!("yo3-ui-auto-mode-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.inspect_setup();let revision=app.setup_revision;let candidate=app.setup_candidate();
        let _=app.update(Msg::Mode(true));let _=app.update(Msg::SetupChecked(revision,candidate,yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));assert!(app.can_play());
        let _=app.update(Msg::Input(Field::Game,String::new()));assert!(!app.setup_in_progress());assert!(app.setup_active.is_none());assert!(!app.can_play());assert_eq!(app.play_label(),"Play");
        let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn explicitly_reselecting_the_same_missing_game_folder_exposes_the_error_and_retry() {
        for paste in [false,true] {
            let root=std::env::temp_dir().join(format!("yo3-ui-same-missing-folder-{}-{paste}",std::process::id()));let _=std::fs::remove_dir_all(&root);
            let folder=root.join("game");std::fs::create_dir_all(&folder).unwrap();
            let settings=Settings{game_dir:folder.to_string_lossy().into(),..Settings::default()};
            LauncherDb::open(&root).unwrap().save_settings(&settings).unwrap();
            let mut app=App::load(root.clone());assert!(app.first_setup);
            let _=app.update(Msg::Browse(Field::Game));let generation=app.picker_generation;
            if paste {
                let _=app.update(Msg::UsePath);let _=app.update(Msg::Input(Field::Game,settings.game_dir.clone()));
                app.setup_due=Some(std::time::Instant::now());let _=app.update(Msg::RunSetupCheck(app.setup_revision));
            } else {let _=app.update(Msg::FolderPicked(generation,Field::Game,Ok(Some(settings.game_dir.clone()))));}
            assert!(!app.first_setup,"explicitly choosing the same path ends pristine setup (paste: {paste})");
            assert_eq!(app.setup_active,Some(app.setup_revision));
            let candidate=app.setup_candidate();let revision=app.setup_revision;
            let _=app.update(Msg::SetupChecked(revision,candidate,yo3_desktop::Readiness::GameProblem("Choose the installed game folder containing yo3.exe.".into()),yo3_desktop::direct::State::Disabled,Vec::new()));
            assert!(app.failed && app.feedback_visible());assert!(app.game_field_error().unwrap().contains("yo3.exe"));
            assert_eq!(app.play_label(),"Retry check");assert!(matches!(app.play_message(),Some(Msg::ValidateGame)));
            assert_eq!(app.settings,settings,"a failed explicit check must preserve saved preferences");
            drop(app);std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn support_switches_to_recovery_and_export_without_stacking_dialogs() {
        let root=std::env::temp_dir().join(format!("yo3-ui-support-routing-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Support);assert!(app.support && app.modal_open());
        let original=app.draft.game_dir.clone();let _=app.update(Msg::Input(Field::Game,"hidden edit".into()));assert_eq!(app.draft.game_dir,original);
        let _=app.update(Msg::ExportLogs);assert!(!app.support && app.report_options);let _=app.view();
        let _=app.update(Msg::CancelClose);let _=app.update(Msg::Support);let _=app.update(Msg::Troubleshoot);
        assert!(!app.support && app.game_data_dialog.is_some());let _=app.update(Msg::CancelClose);
        let _=app.update(Msg::Support);let _=app.update(Msg::Page(Page::About));assert!(!app.support);assert_eq!(app.page,Page::About);
        let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn support_remains_available_during_startup_with_copy_but_cannot_reset_or_export() {
        let root=std::env::temp_dir().join(format!("yo3-ui-support-busy-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.logs="Starting connection…".into();app.begin("Starting game…");
        let _=app.update(Msg::Support);assert!(app.support);assert_eq!(app.update(Msg::CopyLogs).units(),1);
        let _=app.update(Msg::Troubleshoot);assert!(app.game_data_dialog.is_none());assert!(app.support && app.busy);
        let _=app.update(Msg::ExportLogs);assert!(!app.report_options);assert!(app.support);
        let _=app.view();let _=app.update(Msg::CancelClose);assert!(!app.support && app.busy,"closing Help must not cancel startup");
        app.report_options=true;let _=app.update(Msg::Support);assert!(!app.support,"Help must not cover another modal");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn folder_selection_is_applied_when_help_is_opened_during_the_native_picker() {
        let root=std::env::temp_dir().join(format!("yo3-ui-support-picker-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Browse(Field::Game));let generation=app.picker_generation;
        let _=app.update(Msg::Support);assert!(app.support && app.picking);
        let _=app.update(Msg::FolderPicked(generation,Field::Game,Ok(Some("chosen game folder".into()))));
        assert_eq!(app.draft.game_dir,"chosen game folder","Help must not silently discard a completed folder choice");
        assert!(app.support);assert!(!app.picking && !app.busy);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn background_reset_completion_replaces_help_with_the_backup_confirmation() {
        let root=std::env::temp_dir().join(format!("yo3-ui-support-result-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.begin("Resetting local game data…");let _=app.update(Msg::Support);
        let _=app.update(Msg::Done(Ok(Outcome::GameDataReset(root.join("backup")))));
        assert!(!app.support);assert!(matches!(app.game_data_dialog,Some(GameDataDialog::Complete)));let _=app.view();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn advanced_exit_has_an_explicit_keep_hosting_choice_and_normal_exit_stops_hosting() {
        let root=std::env::temp_dir().join(format!("yo3-ui-exit-choice-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.settings.advanced_mode=true;app.status=Some(serde_json::json!({}));
        let _=app.update(Msg::KeepHosting(false));assert!(app.keep_hosting_on_exit,"a hidden checkbox must not change exit intent");
        let _=app.update(Msg::Close);assert!(app.closing && app.keep_hosting_on_exit);
        let _=app.update(Msg::KeepHosting(false));assert!(!app.keep_hosting_on_exit);let _=app.view();
        let _=app.update(Msg::Close);assert!(!app.keep_hosting_on_exit,"repeated window close must preserve the reviewed choice");
        let _=app.update(Msg::CancelClose);app.settings.advanced_mode=false;let _=app.update(Msg::Close);
        assert!(!app.keep_hosting_on_exit);let _=app.update(Msg::KeepHosting(true));assert!(!app.keep_hosting_on_exit);
        let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    fn game_data_preview() -> yo3_desktop::game_data::Preview {
        yo3_desktop::game_data::Preview {directory:PathBuf::from("local-game-data"),settings_present:true,full_present:true}
    }
    #[test]
    fn game_data_confirmation_preserves_players_by_default_and_allows_explicit_full_reset() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-scope-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        let _=app.update(Msg::GameDataPreviewed(generation,settings,Ok(game_data_preview())));
        assert!(app.modal_open());assert!(!app.busy);assert!(!app.game_data_reset_request().unwrap().2,"default reset must leave all player files and decks intact");
        let _=app.update(Msg::KeepGameDataPlayers(false));assert!(app.game_data_reset_request().unwrap().2,"full reset requires explicitly clearing the preserve checkbox");
        let _=app.view();let _=app.update(Msg::CancelClose);assert!(!app.modal_open());std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_preview_cancellation_and_stale_results_cannot_unlock_other_work() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-stale-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        assert!(app.busy);let _=app.update(Msg::CancelClose);assert!(!app.busy);assert!(!app.modal_open());
        app.begin("Unrelated operation…");let _=app.update(Msg::GameDataPreviewed(generation,settings,Err("Old preview failure".into())));
        assert!(app.busy);assert!(app.game_data_dialog.is_none());assert!(app.notice.is_empty());std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_reset_requires_current_confirmed_folder_idle_game_and_no_unsaved_changes() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-guard-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::ConfirmGameDataReset);assert!(!app.busy,"reset cannot run without its reviewed preview");
        let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        let _=app.update(Msg::GameDataPreviewed(generation,settings,Ok(game_data_preview())));
        app.game_running=true;assert!(app.game_data_reset_request().is_none());app.game_running=false;
        app.draft.game_dir.push_str("-unsaved");assert!(app.game_data_reset_request().is_none());app.draft=app.settings.clone();
        app.settings.wine_prefix="different prefix".into();app.draft=app.settings.clone();assert!(app.game_data_reset_request().is_none(),"an old preview cannot authorize a different target");
        let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_changed_settings_reject_async_preview_and_missing_backend_stays_readable() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-unavailable-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        app.settings.wine_prefix="changed".into();app.draft=app.settings.clone();
        let _=app.update(Msg::GameDataPreviewed(generation,settings,Ok(game_data_preview())));assert!(!app.busy);assert!(app.game_data_reset_request().is_none());
        let _=app.update(Msg::CancelClose);app.backend=None;let _=app.update(Msg::Troubleshoot);assert!(app.modal_open());assert!(!app.busy);assert!(app.game_data_reset_request().is_none());
        let _=app.view();app.settings.advanced_mode=true;let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_reset_success_keeps_backup_action_and_rechecks_readiness_without_starting() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-result-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        let _=app.update(Msg::GameDataPreviewed(generation,settings,Ok(game_data_preview())));app.setup_result=Some(yo3_desktop::Readiness::Ready);let revision=app.setup_revision;
        let _=app.update(Msg::ConfirmGameDataReset);assert!(app.busy);assert!(!app.modal_open());assert!(app.setup_result.is_none());assert!(app.setup_revision>revision);
        let backup=root.join("backup");let _=app.update(Msg::Done(Ok(Outcome::GameDataReset(backup.clone()))));
        assert_eq!(app.game_data_backup,Some(backup));assert!(app.modal_open());assert!(app.setup_checking);assert!(!app.game_running);assert!(app.status.is_none());
        let _=app.update(Msg::DismissNotice);assert!(app.game_data_backup.is_some(),"backup access must survive notice dismissal");let _=app.view();std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_modal_guards_block_background_forms_and_duplicate_confirmations() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-modal-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.review_update=true;let _=app.update(Msg::Troubleshoot);assert!(app.game_data_dialog.is_none());
        let _=app.update(Msg::CancelClose);let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        let _=app.update(Msg::GameDataPreviewed(generation,settings,Ok(game_data_preview())));let path=app.draft.game_dir.clone();
        let _=app.update(Msg::Input(Field::Game,"Unexpected folder".into()));let _=app.update(Msg::Mode(true));assert_eq!(app.draft.game_dir,path);assert!(!app.settings.advanced_mode);
        let _=app.update(Msg::ConfirmGameDataReset);let generation=app.game_data_generation;assert!(app.busy);
        let _=app.update(Msg::ConfirmGameDataReset);assert_eq!(app.game_data_generation,generation);assert!(app.busy);assert!(app.game_data_dialog.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_empty_default_scope_never_erases_player_files_and_window_close_cancels_preview() {
        let root=std::env::temp_dir().join(format!("yo3-ui-game-data-empty-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Troubleshoot);let generation=app.game_data_generation;let settings=app.settings.clone();
        let mut preview=game_data_preview();preview.settings_present=false;
        let _=app.update(Msg::GameDataPreviewed(generation,settings,Ok(preview)));assert!(app.game_data_reset_request().is_none(),"default reset must not expand its scope when only player files exist");
        let _=app.view();let _=app.update(Msg::KeepGameDataPlayers(false));assert!(app.game_data_reset_request().unwrap().2);
        let _=app.update(Msg::CancelClose);let _=app.update(Msg::Troubleshoot);assert!(app.busy);let _=app.update(Msg::Close);assert!(!app.busy);assert!(app.game_data_dialog.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn game_data_paths_are_friendly_without_changing_the_operation_path() {
        let path=PathBuf::from(r"\\?\C:\Users\Duelist\Documents\Yu-Gi-Oh! ONLINE 3");let original=path.clone();
        assert_eq!(views::display_path(&path),r"C:\Users\Duelist\Documents\Yu-Gi-Oh! ONLINE 3");assert_eq!(path,original);
        assert_eq!(views::display_path(std::path::Path::new(r"\\?\UNC\host\share\backup")),r"\\host\share\backup");
    }
    #[test]
    fn unchanged_log_refresh_does_not_move_the_scrollbar() {
        let root=std::env::temp_dir().join(format!("yo3-ui-log-refresh-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());
        assert_eq!(app.update(Msg::LogsPolled(Ok("Latest log line".into()))).units(),1);
        assert_eq!(app.update(Msg::LogsPolled(Ok("Latest log line".into()))).units(),0,"polling unchanged logs must not force a scroll");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn logs_follow_only_at_the_bottom_in_both_modes_even_during_startup() {
        let root=std::env::temp_dir().join(format!("yo3-ui-log-follow-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());
        for advanced in [false,true] {
            app.settings.advanced_mode=advanced;app.logs.clear();app.follow_logs=true;app.begin("Starting game…");
            assert_eq!(app.update(Msg::LogsPolled(Ok("First line".into()))).units(),1,"start at the newest line");
            let _=app.update(Msg::LogScrolled(false));
            assert_eq!(app.update(Msg::LogsPolled(Ok("First line\nSecond line".into()))).units(),0,"reading older lines must not jump to the bottom");
            assert_eq!(app.logs,"First line\nSecond line","scrolling up must not stop capture or refresh");
            let _=app.update(Msg::LogScrolled(true));
            assert_eq!(app.update(Msg::LogsPolled(Ok("First line\nSecond line\nThird line".into()))).units(),1,"scrolling back to the bottom resumes following");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn paused_activity_keeps_its_logs_and_scroll_position_until_resumed() {
        let root=std::env::temp_dir().join(format!("yo3-ui-log-pause-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.settings.advanced_mode=true;
        let _=app.update(Msg::LogsPolled(Ok("First line".into())));let _=app.update(Msg::LogScrolled(false));let _=app.update(Msg::Pause(true));
        assert_eq!(app.update(Msg::LogsPolled(Ok("Second line".into()))).units(),0);assert_eq!(app.logs,"First line");
        let _=app.update(Msg::Pause(false));assert_eq!(app.update(Msg::LogsPolled(Ok("Second line".into()))).units(),0);assert_eq!(app.logs,"Second line");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn opening_a_log_view_or_changing_its_source_or_filter_starts_at_the_latest_line() {
        let root=std::env::temp_dir().join(format!("yo3-ui-log-view-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.settings.advanced_mode=true;
        for message in [Msg::Page(Page::Activity),Msg::Logs(false),Msg::LogFilter("ERROR".into()),Msg::Mode(false)] {
            let _=app.update(Msg::LogScrolled(false));assert!(!app.follow_logs);
            assert_eq!(app.update(message).units(),1);assert!(app.follow_logs,"a newly opened log view must start at the bottom");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn closing_during_folder_selection_does_not_trap_the_user_in_a_busy_dialog() {
        let root=std::env::temp_dir().join(format!("yo3-ui-picker-close-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Browse(Field::Game));assert!(app.busy);
        let _=app.update(Msg::Close);assert!(!app.busy,"a native folder dialog must not prevent closing the app");std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn abandoning_a_folder_dialog_ignores_its_late_selection() {
        let root=std::env::temp_dir().join(format!("yo3-ui-picker-late-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.update(Msg::Browse(Field::Game));let generation=app.picker_generation;
        let _=app.update(Msg::UsePath);assert!(!app.busy);let _=app.update(Msg::Input(Field::Game,"Pasted path".into()));
        let _=app.update(Msg::FolderPicked(generation,Field::Game,Ok(Some("Late dialog path".into()))));assert_eq!(app.draft.game_dir,"Pasted path");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn normal_feedback_follows_navigation_and_uses_the_actual_failure_context() {
        let root=std::env::temp_dir().join(format!("yo3-ui-error-context-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.setup_result=Some(yo3_desktop::Readiness::GameProblem("Missing game".into()));
        app.feedback_at(Page::About,Err("Could not open the browser".into()));
        assert!(app.feedback_visible());assert_eq!(app.error_summary(),"Could not open the browser");
        app.failure_setup=Some(yo3_desktop::Readiness::ComponentProblem("Missing DLL".into()));assert!(app.error_summary().contains("Required Revival files"));
        app.feedback(Err("Could not save a ZIP".into()));assert_eq!(app.error_summary(),"Could not save a ZIP");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_stale_setup_check_cannot_enable_play_after_the_current_check_failed() {
        let root=std::env::temp_dir().join(format!("yo3-ui-setup-race-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let _=app.inspect_setup();let previous=app.setup_revision;let candidate=app.setup_candidate();
        let _=app.update(Msg::ValidateGame);let current=app.setup_revision;
        let _=app.update(Msg::SetupChecked(previous,candidate.clone(),yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));
        assert_eq!(app.setup_active,Some(current));
        let _=app.update(Msg::SetupChecked(current,candidate.clone(),yo3_desktop::Readiness::ValidationProblem("Connection refused".into()),yo3_desktop::direct::State::Disabled,Vec::new()));
        let _=app.update(Msg::SetupChecked(previous,candidate,yo3_desktop::Readiness::Ready,yo3_desktop::direct::State::Disabled,Vec::new()));
        assert!(!app.can_play());assert!(!app.setup_checking);assert!(app.failed);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn report_failures_allow_retry_and_stale_results_cannot_replace_the_new_scope() {
        let root=std::env::temp_dir().join(format!("yo3-ui-report-race-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.feedback(Err("Original launch error".into()));let _=app.update(Msg::ExportLogs);let previous=app.report_generation;
        let _=app.update(Msg::ReportPrepared(previous,Err("Read denied. Try Copy logs.".into())));
        assert!(!app.report_busy);assert!(app.report_feedback.as_ref().unwrap().is_err());assert_eq!(app.notice,"Original launch error");let _=app.view();
        let _=app.update(Msg::PrepareReport(true));let latest=app.report_generation;assert!(latest>previous);
        let _=app.update(Msg::ReportPrepared(previous,Err("Old scope failed".into())));assert!(app.report_busy);assert!(app.report_feedback.is_none());
        let _=app.update(Msg::ReportPrepared(latest,Err("Latest scope failed".into())));assert!(!app.report_busy);
        let _=app.update(Msg::CancelClose);let _=app.update(Msg::ReportSaved(latest,Ok("Old save".into())));assert_eq!(app.notice,"Original launch error");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn loading_keeps_logs_live_and_close_errors_readable() {
        let root=std::env::temp_dir().join(format!("yo3-ui-live-loading-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.begin("Starting game…");let _=app.update(Msg::Tick);
        assert!(app.logs_polling);assert!(!app.polling,"lifecycle polling must not wait on the Play mutex");assert!(app.animating());
        let _=app.update(Msg::Animate);assert!(app.pulse>0.0);
        let _=app.update(Msg::LogsPolled(Ok("Live startup detail".into())));assert_eq!(app.logs,"Live startup detail");assert!(app.busy);
        app.closing=true;let _=app.update(Msg::Done(Err("Could not close game. Keep open and retry.".into())));
        assert!(app.closing);assert!(app.feedback_visible());assert!(!app.busy);let _=app.view();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_initialization_can_be_retried_without_deleting_saved_data() {
        let root=std::env::temp_dir().join(format!("yo3-ui-retry-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);std::fs::write(&root,"not a directory").unwrap();
        let mut app=App::load(root.clone());assert!(app.backend.is_none());assert!(!app.can_play());let _=app.view();
        std::fs::remove_file(&root).unwrap();let recovered=App::load(root.clone());
        let _=app.update(Msg::Done(Ok(Outcome::Recovered(recovered.settings,recovered.servers,recovered.backend.unwrap()))));
        assert!(app.backend.is_some());assert!(app.setup_checking);assert!(!app.can_play());std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn system_opener_failures_and_stalls_are_reported() {
        assert!(wait_for_opener(std::process::Command::new("sh").args(["-c","exit 7"]),std::time::Duration::from_secs(1)).unwrap_err().to_string().contains("exit status"));
        let start=std::time::Instant::now();assert!(wait_for_opener(std::process::Command::new("sleep").arg("30"),std::time::Duration::from_millis(50)).is_err());assert!(start.elapsed().as_secs()<1);
    }
    #[test]
    fn exporting_diagnostics_preserves_the_problem_the_user_is_reporting() {
        let root=std::env::temp_dir().join(format!("yo3-ui-report-errors-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone()); app.feedback(Err("The game could not start: missing helper".into()));
        let original=app.notice.clone(); let _=app.update(Msg::ExportLogs);
        assert_eq!(app.notice,original,"opening diagnostics must preserve the original error");
        assert_eq!(app.update(Msg::CopyError).units(),1,"the complete error must remain copyable inside a dialog");
        let _=app.update(Msg::DismissNotice);assert!(app.notice.is_empty(),"a visible dismiss control must work inside the dialog");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn export_privacy_is_a_per_report_choice_and_does_not_change_log_capture_or_scope() {
        let root=std::env::temp_dir().join(format!("yo3-ui-report-privacy-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());let saved=app.settings.clone();let _=app.update(Msg::ExportLogs);
        assert!(!app.report_remove_personal);let generation=app.report_generation;
        let report=app.backend.as_ref().unwrap().report(false).unwrap();let _=app.update(Msg::ReportPrepared(generation,Ok(report)));
        let _=app.update(Msg::RemovePersonal(true));assert!(app.report_remove_personal);assert_eq!(app.settings,saved);let _=app.view();
        let _=app.update(Msg::PrepareReport(true));assert!(app.report_all && app.report_remove_personal);
        let _=app.update(Msg::CancelClose);let _=app.update(Msg::ExportLogs);assert!(!app.report_remove_personal);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn verbose_payloads_stay_out_of_the_default_view_but_can_be_searched_in_advanced_mode() {
        let root=std::env::temp_dir().join(format!("yo3-ui-trace-view-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());app.logs="1 Runtime validation failed\n2 [YO3 CLIENT] raw packet\n3 [YO3 PACKET] nested fields".into();
        assert_eq!(app.shown_logs(),"1 Runtime validation failed");
        app.settings.advanced_mode=true;app.filter="YO3 PACKET".into();assert!(app.shown_logs().contains("nested fields"));
        app.settings.advanced_mode=false;assert_eq!(app.shown_logs(),"1 Runtime validation failed");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn monitoring_errors_are_visible_and_do_not_pretend_the_game_stopped() {
        let root=std::env::temp_dir().join(format!("yo3-ui-monitor-errors-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone()); app.game_running=true; app.polling=true;
        let _=app.update(Msg::Poll(Err("Could not read process status".into())));
        assert!(!app.polling); assert!(app.game_running); assert!(app.failed);
        assert!(app.notice.contains("Could not read process status"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn normal_logs_show_readable_times_and_preserve_unstamped_error_details() {
        let shown=views::friendly_logs("1790000000 Validation passed\nError: required file is missing");
        assert!(!shown.starts_with("1790000000")); assert!(shown.contains("Validation passed"));
        assert!(shown.ends_with("Error: required file is missing"));
        let merged=views::friendly_logs("1790000002 Later\n1790000001 2026-11-11T06:00:00Z INFO Earlier");
        assert!(merged.find("Earlier").unwrap()<merged.find("Later").unwrap()); assert!(!merged.contains("2026-11-11T"));
    }
    #[test]
    fn compact_logs_group_repeats_and_wine_diagnostics_but_keep_actual_failures() {
        let raw="1790000000 Checking required files\n1790000001 Checking required files\n1790000002 0164:fixme:d3dx:Unsupported filter\n1790000003 Game helper: 00ac:fixme:d3d:Diagnostic output\n1790000004 0164:err:module:Cannot load d3dx9_40.dll\n1790000005 stage=game-exit process_exit_code=0xc0000005\n1790000006 User message: fixme: keep this text\n\nMissing required file: yo3-auth.dll";
        let shown=views::friendly_logs(raw);
        assert!(shown.contains("Checking required files (×2)"),"consecutive repeats should occupy one line: {shown}");
        assert_eq!(shown.matches("Checking required files").count(),1);
        assert!(!shown.contains("Unsupported filter") && !shown.contains("Diagnostic output"));
        assert!(shown.contains("2 Wine diagnostic lines hidden"),"the display should explain its filtering");
        for evidence in ["err:module:Cannot load d3dx9_40.dll","process_exit_code=0xc0000005","User message: fixme:","yo3-auth.dll"] {
            assert!(shown.contains(evidence),"lost useful evidence: {evidence}");
        }
        assert!(!shown.contains("\n\n"));
    }
    #[test]
    fn compact_log_display_and_dismissing_errors_preserve_full_exported_diagnostics() {
        use std::io::Read;
        let root=std::env::temp_dir().join(format!("yo3-ui-compact-export-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone());
        let diagnostic=format!("The game could not start.\n{}\nstage=game-exit process_exit_code=0xc0000005\nFull diagnostic: missing native dependency d3dx9_40.dll", "Additional diagnostic context. ".repeat(25));
        app.feedback(Err(diagnostic.clone()));
        let backend=app.backend.clone().unwrap();
        backend.diagnostics.line("0164:fixme:d3dx:Original Wine diagnostic retained");
        app.logs=backend.logs(false);
        let shown=views::friendly_logs(&app.shown_logs());
        assert!(shown.contains("process_exit_code=0xc0000005") && shown.contains("d3dx9_40.dll"));
        let _=app.update(Msg::DismissNotice);assert!(app.notice.is_empty());
        app.feedback(Ok("Another operation finished.".into()));
        let archive=root.join("report.zip");backend.report(false).unwrap().export(&archive).unwrap();
        let mut zip=zip::ZipArchive::new(std::fs::File::open(archive).unwrap()).unwrap();let mut text=String::new();
        for index in 0..zip.len() {let mut entry=zip.by_index(index).unwrap();if entry.name().unwrap().ends_with(".log"){entry.read_to_string(&mut text).unwrap();}}
        assert!(text.contains(&diagnostic),"the complete error must survive dismissal and export, including context beyond the banner");
        assert!(text.contains("Original Wine diagnostic retained"),"display filtering must not change the ZIP");
        drop(zip);drop(backend);drop(app);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn normal_mode_has_one_page_and_cannot_start_an_unvalidated_game_path() {
        let root=std::env::temp_dir().join(format!("yo3-single-page-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone()); assert_eq!(app.page,Page::Play);
        app.setup_result=Some(yo3_desktop::Readiness::Ready); app.settings.game_dir="saved game".into();app.draft=app.settings.clone();
        assert!(app.can_play());
        let _=app.update(Msg::Input(Field::Game,"different game".into())); assert!(!app.can_play());
        app.settings.advanced_mode=true; assert!(!app.can_play(),"both modes must wait for the chosen folder to pass its check");
        app.page=Page::Server; let _=app.update(Msg::Mode(false)); assert_eq!(app.page,Page::Play);
        app.draft=app.settings.clone();let _=app.inspect_setup();let revision=app.setup_revision;let candidate=app.setup_candidate();
        let _=app.update(Msg::SetupChecked(revision,candidate,yo3_desktop::Readiness::ValidationProblem("Saved connection could not be verified".into()),yo3_desktop::direct::State::Disabled,Vec::new()));
        assert!(!app.can_play(),"a failed complete validation must revoke earlier file-only readiness");
        assert!(matches!(app.setup_result,Some(yo3_desktop::Readiness::ValidationProblem(_))),"a connection/validation failure must not claim a DLL is missing");
        let _=app.view(); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failures_stay_visible_and_can_be_copied_without_changing_modes() {
        let root=std::env::temp_dir().join(format!("yo3-ui-errors-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone()); app.page=Page::Play;
        app.feedback(Err("Could not start the helper.\nMissing yo3-auth-launch.exe".into()));
        assert!(app.failed); assert!(app.notice_expires.is_none()); assert!(!app.settings.advanced_mode);
        assert_eq!(app.update(Msg::CopyError).units(),1);let _=app.view();
        app.setup_result=Some(yo3_desktop::Readiness::ComponentProblem("Missing helper".into())); let _=app.view();
        assert!(!app.modal_open(),"the error must remain available without an extra recovery dialog");
        let _=app.update(Msg::DismissNotice); assert!(app.notice.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
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
    #[test]
    fn switching_modes_hides_technical_pages_without_discarding_configuration_drafts() {
        let root=std::env::temp_dir().join(format!("yo3-ui-modes-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let mut app=App::load(root.clone()); assert!(!app.settings.advanced_mode);
        app.draft.host_address="unfinished.example".into();
        let _=app.update(Msg::Mode(true)); assert!(app.settings.advanced_mode);
        app.page=Page::Activity; let _=app.update(Msg::Mode(false)); assert_eq!(app.page,Page::Play);
        assert_eq!(app.draft.host_address,"unfinished.example"); assert!(!LauncherDb::open(&root).unwrap().settings().unwrap().advanced_mode);
        let _=app.view(); std::fs::remove_dir_all(root).unwrap();
    }

}
