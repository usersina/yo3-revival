//! The window: a title bar, the current screen (setup or the main screen), a footer and at most one dialog.
//! Each screen keeps its own state and messages; this module routes between them and owns what they share.
use std::path::PathBuf;
use std::sync::Arc;
use iced::widget::{Space, column, container, mouse_area, operation, row, stack, text};
use iced::{Alignment::Center, Color, Element, Length::Fill, Subscription, Task, keyboard, window};
use yo3_desktop::{Backend, Settings, instance, store::Store, updates};
use yo3_skin::{Glyph, Icon, Kind, color, pill};
use crate::{external, help, home, host, settings, setup};

/// What the window shares with every screen.
pub struct Ctx {
    pub root: PathBuf,
    pub backend: Option<Arc<Backend>>,
    pub store: Option<Store>,
    pub settings: Settings,
}

/// The single-instance claim handed over from `main`.
pub enum Claim { Now(instance::Owner), AfterUpdate }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog { Help, Report, Repair, ConfirmClose, Backups, Host, Support, Contact, WhatsNew, ConfirmRestart, Foreign }

/// Report a problem: what the player says happened, the error it is about, and how far it got.
#[derive(Debug, Default)]
pub struct Report { pub what: iced::widget::text_editor::Content, pub error: String, pub busy: bool,
    /** Ok(true): the logs went to the server, the form carries the report ID; Ok(false): the saved file is to attach. */ pub result: Option<Result<bool, String>> }

/// Another program's file or provider stopped a start (#68): what was found, and where the player is in answering it.
#[derive(Debug)]
pub struct Foreign { held: yo3_desktop::foreign::Held, /** Move it aside was pressed: the player is asked to confirm. */ confirm: bool, details: bool,
    busy: bool, /** The sentence when moving a file failed, and its technical text (behind Details). */ problem: Option<String>, failure: Option<String> }

#[derive(Clone, Debug)]
pub enum ForeignMessage { Move, ConfirmMove, Back, Continue, Details, Moved(Result<Vec<String>, String>) }

#[derive(Clone, Debug)]
pub enum ReportMessage { Edit(iced::widget::text_editor::Action), Send, Sent(Result<bool, String>) }

#[derive(Clone, Debug)]
pub enum Message {
    Home(home::Message),
    Setup(setup::Message),
    Settings(settings::Message),
    OpenSettings(settings::Section),
    /// Settings, Game, scrolled to Card pictures (the footer's download link).
    ShowPictures,
    Help(help::Message),
    OpenHelp(Option<&'static str>),
    OpenReport(String),
    /// The footer's heart: how to support the project.
    OpenSupport,
    /// The footer's Help & contact: the guide, the community, a report.
    OpenContact,
    /// The footer's cog: Settings, or back from it.
    ToggleSettings,
    Report(ReportMessage),
    Foreign(ForeignMessage),
    Copy(String),
    Host(host::Message),
    CloseDialog,
    Show,
    /// A tick while an update downloads, so What's new's bar moves.
    DownloadTick,
    Drag,
    Minimize,
    CloseRequested,
    Quit,
    Exit,
    WindowOpened,
    Key(keyboard::Event),
    /// Every few minutes: checks for updates when `updates::flow::CHECK_EVERY` has passed (#32).
    UpdateTick,
    UpdateChecked(Result<updates::Decision, String>),
    /// The background download of this version finished: staged and waiting for a restart, or refused.
    UpdatePrepared(String, Result<updates::install::Installed, String>),
    /// Restart into the downloaded update (the game is closed; hosting confirmed).
    Restart,
    /// The restart didn't happen: this version stays.
    Restarted(Result<(), String>),
    OpenLink(String),
    LinkOpened(Result<(), String>),
    /// Each second while a good toast shows: it goes by itself after `TOAST_SECONDS`.
    ToastTick,
}

/// How long good news stays in the corner; a problem stays until it is dismissed.
const TOAST_SECONDS: u64 = 6;

/// One second of a good toast's life: the sentence on screen now against the one being timed. A new sentence starts its
/// own time; the same one past `TOAST_SECONDS` has expired (true: dismiss it); none on screen ends the timing.
fn toast_step(timed: Option<(String, std::time::Instant)>, shown: Option<String>, now: std::time::Instant) -> (Option<(String, std::time::Instant)>, bool) {
    match (timed, shown) {
        (_, None) => (None, false),
        (Some((sentence, since)), Some(on_screen)) if sentence == on_screen => {
            if now.duration_since(since) >= std::time::Duration::from_secs(TOAST_SECONDS) { (None, true) } else { (Some((sentence, since)), false) }
        }
        (_, Some(on_screen)) => (Some((on_screen, now)), false),
    }
}

pub struct App {
    ctx: Ctx,
    home: home::State,
    setup: Option<setup::State>,
    settings: Option<settings::State>,
    help: help::State,
    report: Report,
    foreign: Option<Foreign>,
    host: host::State,
    dialog: Option<Dialog>,
    updater: updates::flow::Updater,
    /// The update download in the background: its bytes so far, for What's new's bar.
    transfer: updates::download::Transfer,
    startup: updates::activation::Startup,
    quitting: bool,
    /// The good toast on screen and since when, for its own time out.
    good_toast: Option<(String, std::time::Instant)>,
    /// The anime pictures download, while Settings is closed: it goes on, and its end is a toast on the main screen.
    pictures: Option<settings::Run>,
}

/// Runs blocking work (disk, processes, the network) off the window's thread.
pub fn job<T: Send + 'static, M: Send + 'static>(work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
    done: impl FnOnce(Result<T, String>) -> M + Send + 'static) -> Task<M> {
    Task::perform(async move {
        tokio::task::spawn_blocking(work).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:#}"))
    }, done)
}

impl App {
    pub fn boot(root: PathBuf, startup: updates::activation::Startup, routing_error: Option<String>, update_failed: Option<(String, String)>,
        claim: Claim) -> (Self, Task<Message>) {
        let mut problem = routing_error;
        let (store, settings) = match Store::open(&root) {
            Ok((store, settings)) => (Some(store), settings),
            Err(error) => { problem = Some(format!("Could not open the launcher's settings: {error:#}")); (None, Settings::default()) } // → Details
        };
        let backend = match Backend::new(root.clone()) {
            Ok(backend) => Some(backend),
            Err(error) => { problem.get_or_insert(format!("YO3 Revival needs repair: {error:#}")); None } // → Details
        };
        // Singleplayer's key, left in a profile file by an older launcher, moves into the credential store (#23).
        if let Some(backend) = &backend { let settings = settings.clone(); backend.migrate_keys(&|entry, key| backend.mirror_entry(&settings, entry, Some(key))); }
        let ctx = Ctx { root, backend, store, settings };
        let mut home = home::State::default();
        if let Some(problem) = problem { home.fail(&ctx, problem) }
        // The update downloaded last time didn't start, and this version was kept (#32).
        if let Some((summary, details)) = update_failed { home.update_notice(&ctx, summary, details) }
        let updater = updates::flow::Updater::new(updates::install::failed_start(&ctx.root).ok().flatten());
        // Until setup has been finished once, the launcher opens on it.
        let mut setup = (!ctx.settings.setup_complete).then(|| setup::State::new(&ctx, setup::Step::Welcome, false));
        let mut tasks = vec![
            home.check(&ctx).map(Message::Home),
            home.load_banlists(&ctx).map(Message::Home),
            Task::done(Message::UpdateTick),
            listen_for_second_starts(&ctx.root, claim),
        ];
        if let Some(setup) = &mut setup { tasks.push(setup.start(&ctx).map(Message::Setup)) }
        let mut app = Self { ctx, home, setup, settings: None, help: help::State::at(None), report: Report::default(), foreign: None, host: host::State::default(), dialog: None, updater, transfer: Default::default(), startup, quitting: false, good_toast: None, pictures: None };
        // Card pictures download by themselves (setup's tick): the missing ones, quietly, with their progress in the footer.
        if app.ctx.settings.setup_complete && app.ctx.settings.card_pictures { tasks.push(app.get_missing_pictures()) }
        (app, Task::batch(tasks))
    }

    /// Starts getting the card pictures that are missing, unless a download runs already or none is missing: in Settings
    /// when it is open (its bar), else in the background, shown in the footer and ending in a toast.
    fn get_missing_pictures(&mut self) -> Task<Message> {
        if let Some(state) = &mut self.settings {
            return state.update(settings::Message::GetPictures, &mut self.ctx, self.home.playing()).map(Message::Settings);
        }
        if self.pictures.is_some() { return Task::none() }
        let (have, total) = yo3_desktop::pictures::count(&yo3_launcher::art::folder_in(&self.ctx.root));
        if have >= total { return Task::none() }
        let (run, task) = settings::begin_download(&self.ctx);
        self.pictures = Some(run);
        task.map(Message::Settings)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Home(home::Message::SetUp) => return self.open_setup(setup::Step::Game),
            Message::Home(home::Message::Repair) => self.dialog = Some(Dialog::Repair),
            // A server needs a newer launcher: What's new (with Restart to install) when the update is known.
            Message::Home(home::Message::Releases) => match self.updater.remedy() {
                updates::flow::Remedy::WhatsNew => self.dialog = Some(Dialog::WhatsNew),
                updates::flow::Remedy::Releases => return self.update(Message::OpenLink(RELEASES.into())),
            },
            Message::Home(home::Message::WhatsNew) => self.dialog = Some(Dialog::WhatsNew),
            Message::Home(home::Message::RestartToInstall) => {
                // Never while the game runs; hosting for friends stops, so that is asked first.
                if self.home.playing() || self.updater.ready().is_none() { return Task::none() }
                if self.host.hosted.is_some() { self.dialog = Some(Dialog::ConfirmRestart) } else { return self.update(Message::Restart) }
            }
            Message::Home(home::Message::Restore) => self.dialog = Some(Dialog::Backups),
            Message::Home(home::Message::Foreign(held)) => {
                if let Some(backend) = &self.ctx.backend { backend.diagnostics.line(&format!("Telling the player about {} finding(s) inside the game", held.findings.len())) }
                self.foreign = Some(Foreign { held, confirm: false, details: false, busy: false, problem: None, failure: None });
                self.dialog = Some(Dialog::Foreign);
            }
            Message::Foreign(message) => return self.foreign_update(message),
            Message::Home(home::Message::Settings(section)) => return self.update(Message::OpenSettings(section)),
            Message::Home(home::Message::Report(error)) => return self.update(Message::OpenReport(error)),
            Message::Home(home::Message::Help(topic)) => return self.update(Message::OpenHelp(Some(topic))),
            Message::Home(home::Message::Copy(text)) => return self.update(Message::Copy(text)),
            Message::Home(home::Message::Multi(crate::multi::Message::Host)) => {
                self.host.open(&self.ctx);
                self.dialog = Some(Dialog::Host);
                // The server tools' menu it was chosen from closes.
                return self.home.update(home::Message::Multi(crate::multi::Message::Menu(false)), &mut self.ctx).map(Message::Home);
            }
            Message::Home(home::Message::RestoreBackup(backup)) => {
                self.dialog = None;
                return self.home.update(home::Message::RestoreBackup(backup), &mut self.ctx).map(Message::Home);
            }
            Message::Home(message) => return self.home.update(message, &mut self.ctx).map(Message::Home),
            Message::Setup(setup::Message::Saved(Ok((settings, note)))) => {
                self.ctx.settings = *settings;
                self.setup = None;
                if let Some(note) = note { self.home.fail(&self.ctx, note) }
                let pictures = if self.ctx.settings.card_pictures { self.get_missing_pictures() } else { Task::none() };
                return Task::batch([self.home.check(&self.ctx).map(Message::Home), self.home.load_banlists(&self.ctx).map(Message::Home), pictures]);
            }
            Message::Setup(setup::Message::Open(setup::GETTING_THE_GAME)) => return self.update(Message::OpenHelp(Some("game"))),
            Message::Setup(setup::Message::Open(url)) => return self.update(Message::OpenLink(url.into())),
            Message::Setup(setup::Message::Repair) => self.dialog = Some(Dialog::Repair),
            Message::Setup(setup::Message::Cancel) => self.setup = None,
            Message::Setup(message) => if let Some(setup) = &mut self.setup { return setup.update(message, &self.ctx).map(Message::Setup) },
            Message::Settings(message) => return self.settings_update(message),
            Message::OpenSettings(section) => {
                if self.setup.is_some() { return Task::none() }
                // Already open (the footer's version while in Settings): go to that section, keeping what is typed elsewhere.
                if self.settings.is_some() { return self.settings_update(settings::Message::Section(section)) }
                let mut state = settings::State::new(&self.ctx, section);
                state.resume_pictures(self.pictures.take());
                let start = state.start(&self.ctx).map(Message::Settings);
                // Accounts lists the Multiplayer tab's sign-ins: load its servers if it hasn't yet.
                let load = self.home.multi.load(&self.ctx).map(|m| Message::Home(home::Message::Multi(m)));
                self.settings = Some(state);
                return Task::batch([start, load]);
            }
            Message::ShowPictures => {
                let open = self.update(Message::OpenSettings(settings::Section::Game));
                return Task::batch([open, self.settings_update(settings::Message::ShowPictures)]);
            }
            Message::Help(help::Message::Topic(id)) => self.help.update(&help::Message::Topic(id)),
            Message::Help(help::Message::Report) => return self.update(Message::OpenReport(String::new())),
            Message::Help(help::Message::Close) => self.dialog = None,
            Message::Help(help::Message::Open(url)) => return self.update(Message::OpenLink(url)),
            Message::OpenSupport => self.dialog = Some(Dialog::Support),
            Message::OpenContact => self.dialog = Some(Dialog::Contact),
            Message::ToggleSettings => if self.setup.is_none() {
                return self.update(if self.settings.is_some() { Message::Settings(settings::Message::Close) } else { Message::OpenSettings(settings::Section::Game) });
            },
            Message::OpenHelp(topic) => { self.help = help::State::at(topic); self.dialog = Some(Dialog::Help) }
            Message::OpenReport(error) => { self.report = Report { error, ..Report::default() }; self.dialog = Some(Dialog::Report) }
            Message::Report(ReportMessage::Edit(action)) => {
                // At most 2,000 characters: an edit that would go past it is dropped.
                let edits = action.is_edit();
                let before = self.report.what.text();
                self.report.what.perform(action);
                if edits && self.report.what.text().chars().count() > 2000 { self.report.what = iced::widget::text_editor::Content::with_text(&before) }
            }
            Message::Report(ReportMessage::Send) => {
                let Some(backend) = self.ctx.backend.clone() else { return Task::none() };
                let what: String = self.report.what.text().chars().filter(|c| *c == '\n' || !c.is_control()).collect();
                let (what, error) = (what.trim().to_owned(), self.report.error.clone());
                self.report.busy = true;
                self.report.result = None;
                // Signed in to a server on Multiplayer (and reports on): the logs go to its host and the form gets the
                // report ID. Otherwise (Singleplayer, or turned off) the saved logs file is attached by hand.
                let target = self.home.multi.session().filter(|_| self.ctx.settings.problem_reports);
                return Task::perform(async move {
                    let mut ids = backend.recent_report_ids();
                    if let Some((server, signed)) = target {
                        let logs = backend.problem_logs();
                        match yo3_desktop::multiplayer::problem_report(&server, &signed.session, &what, &error, &logs).await {
                            Ok(id) => ids.insert(0, id),
                            Err(problem) => backend.diagnostics.line(&format!("Report a problem: the logs were not sent to the server: {}", problem.sentence())),
                        }
                    }
                    tokio::task::spawn_blocking(move || {
                        let prepared = backend.report_problem(&what, &error, &ids).map_err(|e| format!("{e:#}"))?;
                        // The saved logs' folder, so the player can drag the file into the form; then the form.
                        if let Some(dir) = prepared.zip.as_ref().and_then(|z| z.parent()) { let _ = external::open(&dir.to_string_lossy()); }
                        external::open(&prepared.url).map(|()| prepared.zip.is_none()).map_err(|e| format!("{e:#}"))
                    }).await.map_err(|e| e.to_string())?
                }, |result| Message::Report(ReportMessage::Sent(result)));
            }
            Message::Report(ReportMessage::Sent(result)) => {
                self.report.busy = false;
                if let Err(error) = &result && let Some(backend) = &self.ctx.backend { backend.diagnostics.line(error) }
                self.report.result = Some(result);
            }
            Message::Copy(text) => return iced::clipboard::write(text),
            Message::Host(host::Message::Copy(text)) => return self.update(Message::Copy(text)),
            Message::Host(host::Message::Open(url)) => return self.update(Message::OpenLink(url)),
            Message::Host(host::Message::Help(topic)) => return self.update(Message::OpenHelp(Some(topic))),
            Message::Host(host::Message::Close) => self.dialog = None,
            Message::Host(message) => {
                // Your hosted server goes in your own Multiplayer list, at this PC's own address: 127.0.0.1, not localhost,
                // which Windows tries as ::1 first, where the server isn't listening (a refused connection there takes
                // Windows about 2 s, so every check of the row did).
                let own = match &message {
                    host::Message::Started(Ok(started)) => yo3_desktop::hosting::hosted_server(&format!("{} (yours)", started.1.server.name), "127.0.0.1",
                        started.0.host_port, started.1.server.key()).ok(),
                    _ => None,
                };
                // Stopped: its row is checked again at once, rather than showing the last check until the next.
                let stopped = match &message {
                    host::Message::Stopped(Ok(())) => self.host.hosted.as_ref().map(|h| h.server.key().to_string()),
                    _ => None,
                };
                let task = self.host.update(message, &mut self.ctx).map(Message::Host);
                if let Some(key) = stopped {
                    return Task::batch([task, self.home.update(home::Message::Multi(crate::multi::Message::Recheck(key)), &mut self.ctx).map(Message::Home)]);
                }
                let Some(own) = own else { return task };
                return Task::batch([task, self.home.update(home::Message::Multi(crate::multi::Message::Hosted(Box::new(own))), &mut self.ctx).map(Message::Home)]);
            }
            Message::CloseDialog => self.dialog = None,
            Message::DownloadTick => {}
            Message::Show => return window::latest().and_then(|id| Task::batch([window::minimize(id, false), window::gain_focus(id)])),
            Message::Drag => return window::latest().and_then(window::drag),
            Message::Minimize => return window::latest().and_then(|id| window::minimize(id, true)),
            Message::CloseRequested => {
                let friends = self.host.hosted.is_some() && self.host.activity.online > 0;
                if self.home.game.running || self.home.game.starting.is_some() || self.home.game.extras > 0 || friends { self.dialog = Some(Dialog::ConfirmClose) } else { return self.update(Message::Quit) }
            }
            Message::Quit => {
                if self.quitting { return Task::none() }
                self.quitting = true;
                self.dialog = None;
                let Some(backend) = self.ctx.backend.clone() else { return iced::exit() };
                let settings = self.ctx.settings.clone();
                // Your own server lives as long as the launcher (until #29 ties it to the game).
                return job(move || {
                    // Every game the launcher started closes with it, the extra ones of Play as… too (#64).
                    backend.close_all_games(&settings)?;
                    let _ = backend.stop_hosting();
                    backend.stop_server_if_running(&settings)?;
                    Ok(())
                }, |_| Message::Exit);
            }
            Message::Exit => return iced::exit(),
            Message::WindowOpened => {
                // The updater waits for this before it switches versions.
                if let Some(readiness) = self.startup.readiness.take() && let Err(error) = readiness.write(yo3_tunnel::BUILD_VERSION) {
                    self.home.fail(&self.ctx, format!("Update readiness failed: {error:#}")); // → Details
                }
            }
            Message::Key(keyboard::Event::KeyPressed { key: keyboard::Key::Named(named), modifiers, .. }) => match named {
                keyboard::key::Named::Tab if modifiers.shift() => return operation::focus_previous(),
                keyboard::key::Named::Tab => return operation::focus_next(),
                keyboard::key::Named::Escape => if self.dialog.is_some() { self.dialog = None }
                // Settings' own dialog (Anime pictures) closes first, not the whole Settings screen behind it.
                else if self.settings.as_ref().is_some_and(|s| s.has_dialog()) { return self.settings_update(settings::Message::PicturesDialogClose) }
                // The bell's list and the server tools close before the screen under them does.
                else if self.setup.is_none() && self.home.bell_open { return self.update(Message::Home(home::Message::Bell(false))) }
                else if self.settings.is_some() { self.close_settings() }
                else if self.setup.is_none() && let Some(message) = self.home.escape() { return self.update(Message::Home(message)) },
                _ => {}
            },
            Message::Key(_) => {}
            Message::UpdateTick => {
                let now = std::time::Instant::now();
                if !self.updater.due(now) { return Task::none() }
                self.updater.checking(now);
                return Task::perform(async { updates::check().await.map_err(|e| format!("{e:#}")) }, Message::UpdateChecked);
            }
            Message::UpdateChecked(result) => {
                // A new version downloads and is checked in the background; it installs only on restart.
                let Some(release) = self.updater.checked(result) else { return Task::none() };
                let (root, tag) = (self.ctx.root.clone(), release.tag.clone());
                self.transfer = Default::default();
                let transfer = self.transfer.clone();
                return Task::perform(async move {
                    let installed = updates::download::prepare(root.clone(), release, transfer).await?;
                    let pending = installed.clone();
                    tokio::task::spawn_blocking(move || updates::install::set_pending(&root, &pending)).await??;
                    anyhow::Ok(installed)
                }, move |result| Message::UpdatePrepared(tag.clone(), result.map_err(|e| format!("{e:#}"))));
            }
            Message::UpdatePrepared(tag, result) => {
                // Refused (a checksum or signature that doesn't match) is reported, and never installed.
                if let Err(error) = &result && let Some(backend) = &self.ctx.backend { backend.diagnostics.line(&format!("Update {tag} not installed: {error}")) }
                self.updater.prepared(&tag, result);
                self.home.update_ready = self.updater.ready().map(|i| i.tag.trim_start_matches('v').to_owned());
            }
            Message::Restart => {
                let (Some(installed), Some(backend)) = (self.updater.ready().cloned(), self.ctx.backend.clone()) else { return Task::none() };
                if self.home.playing() || self.quitting { return Task::none() }
                self.quitting = true;
                self.dialog = None;
                let (settings, root) = (self.ctx.settings.clone(), self.ctx.root.clone());
                return job(move || {
                    anyhow::ensure!(!backend.game_running() && !backend.game_open_elsewhere(&settings), "{GAME_OPEN}");
                    let _ = backend.stop_hosting();
                    backend.stop_server_if_running(&settings)?;
                    // The new version opens its window, then this one exits; if it doesn't, this one stays.
                    match updates::activation::launch(&root, &installed, false, std::time::Duration::from_secs(25)) {
                        Ok(_ready) => { let _ = updates::install::clear_pending(&root, None); Ok(()) }
                        Err(error) => { let _ = updates::install::clear_pending(&root, Some(&installed.tag)); Err(error) }
                    }
                }, Message::Restarted);
            }
            Message::Restarted(Ok(())) => return iced::exit(),
            Message::Restarted(Err(error)) => {
                self.quitting = false;
                let tag = self.updater.ready().map(|i| i.tag.clone()).unwrap_or_default();
                let stopped = self.host.hosted.is_some();
                if error == GAME_OPEN {
                    // A game opened since: the update waits, and installs after it closes.
                    self.home.update_notice(&self.ctx, "Close the game first, then Restart to install.".into(), error);
                } else {
                    self.updater.start_failed(&tag);
                    self.home.update_ready = None;
                    self.home.update_notice(&self.ctx, updates::activation::failed_summary(&tag, yo3_tunnel::BUILD_VERSION), error);
                }
                if stopped { return self.host.update(host::Message::Stopped(Ok(())), &mut self.ctx).map(Message::Host) }
            }
            Message::OpenLink(url) => return job(move || external::open(&url), Message::LinkOpened),
            Message::LinkOpened(result) => if let Err(error) = result { self.home.fail(&self.ctx, error) },
            Message::ToastTick => {
                let now = std::time::Instant::now();
                let (settings, home) = (self.settings.as_ref().and_then(|s| s.good_toast()).map(str::to_owned), self.home.good_toast().map(str::to_owned));
                let shown = settings.clone().or_else(|| home.clone());
                let (timed, expired) = toast_step(self.good_toast.take(), shown.clone(), now);
                self.good_toast = timed;
                if expired {
                    if settings.is_some() && settings == shown { return self.update(Message::Settings(settings::Message::Dismiss)) }
                    return self.update(Message::Home(home::Message::Dismiss));
                }
            }
        }
        Task::none()
    }

    fn foreign_update(&mut self, message: ForeignMessage) -> Task<Message> {
        let Some(foreign) = &mut self.foreign else { return Task::none() };
        match message {
            ForeignMessage::Move => { foreign.confirm = true; foreign.problem = None }
            ForeignMessage::Back => { foreign.confirm = false; foreign.problem = None }
            ForeignMessage::Details => foreign.details = !foreign.details,
            ForeignMessage::ConfirmMove => {
                if foreign.busy { return Task::none() }
                foreign.busy = true; foreign.problem = None;
                let (files, game) = (foreign.held.files(), PathBuf::from(&self.ctx.settings.game_dir));
                return job(move || {
                    let mut moved = Vec::new();
                    for name in files {
                        // A name a finding gave that isn't a plain plugin file in the game folder is skipped, not an error.
                        if yo3_desktop::foreign::movable(&game, &name).is_err() { continue }
                        yo3_desktop::foreign::move_aside_allowed(&game, &name)?;
                        moved.push(name);
                    }
                    anyhow::ensure!(!moved.is_empty(), "No file in the game folder could be moved aside");
                    Ok(moved)
                }, |result| Message::Foreign(ForeignMessage::Moved(result)));
            }
            ForeignMessage::Moved(Ok(names)) => {
                self.dialog = None; self.foreign = None;
                if let Some(backend) = &self.ctx.backend { backend.diagnostics.line(&format!("Moved aside: {}", names.join(", "))) }
                self.home.good(format!("Moved aside: {}. Press PLAY to start the game again. Settings, Advanced can put it back.", names.join(", ")));
            }
            ForeignMessage::Moved(Err(error)) => {
                foreign.busy = false;
                if let Some(backend) = &self.ctx.backend { backend.diagnostics.line(&error) }
                // A declined permission question says what is still true and what to do; any other failure is one plain
                // sentence, its technical text behind Details with the rest.
                foreign.problem = Some(if error.contains("declined") { yo3_desktop::foreign::DECLINED.to_string() } else {
                    let shown = crate::error_presentation::present(&error, crate::error_presentation::Hint::General, &masks(&self.ctx));
                    foreign.failure = shown.diagnosis;
                    shown.summary });
            }
            ForeignMessage::Continue => {
                self.dialog = None; self.foreign = None;
                let Some(backend) = self.ctx.backend.clone() else { return Task::none() };
                backend.accept_foreign_once();
                return self.home.play_again(&mut self.ctx).map(Message::Home);
            }
        }
        Task::none()
    }

    /// Leaves Settings, keeping a pictures download going (see `pictures`).
    fn close_settings(&mut self) {
        if let Some(mut state) = self.settings.take() { self.pictures = state.take_pictures() }
    }

    fn settings_update(&mut self, message: settings::Message) -> Task<Message> {
        match message {
            settings::Message::Close => { self.close_settings(); Task::none() }
            settings::Message::ChangeGame => { self.close_settings(); self.open_setup(setup::Step::Game) }
            // The download's messages while Settings is closed: its bar's numbers, then its end as a toast.
            settings::Message::Pictures(event) if self.settings.is_none() => {
                match event {
                    yo3_desktop::pictures::Event::Progress(done, total) => if let Some(run) = &mut self.pictures { run.0 = done; run.1 = total },
                    yo3_desktop::pictures::Event::Done(result) => {
                        self.pictures = None;
                        match result {
                            Ok(out) => {
                                let (good, sentence) = yo3_desktop::pictures::download_sentence(&out);
                                if let Some(backend) = &self.ctx.backend { backend.diagnostics.line(&format!("Card pictures: {} saved, {} had, {} not fetched", out.saved.len(), out.had, out.failed.len())) }
                                let sentence = format!("Card pictures: {sentence}");
                                if good { self.home.good(sentence) } else { self.home.fail(&self.ctx, sentence) }
                            }
                            Err(error) => self.home.fail(&self.ctx, format!("The card pictures couldn't be saved.\n{error}")), // → Details
                        }
                    }
                }
                Task::none()
            }
            settings::Message::Account(message) => self.home.update(home::Message::Multi(message), &mut self.ctx).map(Message::Home),
            settings::Message::Restore => { self.dialog = Some(Dialog::Backups); Task::none() }
            settings::Message::Open(url) => self.update(Message::OpenLink(url)),
            settings::Message::Copy(text) => self.update(Message::Copy(text)),
            settings::Message::Report(error) => self.update(Message::OpenReport(error)),
            // Settings → About's check counts as one (#32): a new version downloads as after the timed check.
            settings::Message::UpdatesChecked(result) => {
                let task = self.settings.as_mut().map_or_else(Task::none, |state| state.update(settings::Message::UpdatesChecked(result.clone()), &mut self.ctx, false).map(Message::Settings));
                Task::batch([task, self.update(Message::UpdateChecked(result))])
            }
            message => {
                let Some(state) = &mut self.settings else { return Task::none() };
                let playing = self.home.playing();
                let task = state.update(message.clone(), &mut self.ctx, playing).map(Message::Settings);
                // Fix permission set direct start up: the main screen's permission notice and Settings' row go.
                if matches!(message, settings::Message::DirectDone(true, Ok(()))) { self.home.permission_granted() }
                // A change of what the main screen checks (the game folder, Wine) is checked again there.
                if matches!(message, settings::Message::Changed(yo3_desktop::options::Change::WinePrefix(_) | yo3_desktop::options::Change::WineBinary(_), Ok(_))) {
                    return Task::batch([task, self.home.check(&self.ctx).map(Message::Home)]);
                }
                task
            }
        }
    }

    fn open_setup(&mut self, step: setup::Step) -> Task<Message> {
        let mut setup = setup::State::new(&self.ctx, step, true);
        let task = setup.start(&self.ctx).map(Message::Setup);
        self.setup = Some(setup);
        task
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut all = vec![
            window::close_requests().map(|_| Message::CloseRequested),
            window::open_events().map(|_| Message::WindowOpened),
            keyboard::listen().map(Message::Key),
        ];
        if self.setup.is_none() { all.push(self.home.subscription(self.settings.is_none() && self.dialog.is_none()).map(Message::Home)) }
        all.push(self.host.subscription().map(Message::Host));
        all.push(iced::time::every(std::time::Duration::from_secs(10 * 60)).map(|_| Message::UpdateTick));
        if self.home.good_toast().is_some() || self.settings.as_ref().is_some_and(|s| s.good_toast().is_some()) {
            all.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::ToastTick));
        }
        if matches!(self.updater.stage, updates::flow::Stage::Downloading(_)) && self.dialog.is_some() {
            all.push(iced::time::every(std::time::Duration::from_millis(400)).map(|_| Message::DownloadTick));
        }
        Subscription::batch(all)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match (&self.setup, &self.settings) {
            (Some(setup), _) => setup.view().map(Message::Setup),
            (None, Some(settings)) => settings.view(&self.ctx, &self.home).map(Message::Settings),
            (None, None) => self.home.view(&self.ctx).map(Message::Home),
        };
        let page = column![self.title_bar(), container(body).height(Fill), self.footer()];
        let mut screen = stack![yo3_skin::backdrop(), page];
        // Toasts float above the footer: what happens never moves the page (the player's request, 2026-11-20). Settings'
        // own sit bottom right; the main screen's bottom left, over the news panel's empty end, never over PLAY or LOG IN
        // (live on Windows 11 a toast hid the lower half of the button).
        let toast_side = if self.settings.is_some() { iced::alignment::Horizontal::Right } else { iced::alignment::Horizontal::Left };
        if self.setup.is_none() {
            let mut toasts: Vec<Element<'_, Message>> = match &self.settings {
                Some(settings) => settings.toasts().into_iter().map(|t| t.map(Message::Settings)).collect(),
                None => Vec::new(),
            };
            if self.settings.is_none() && let Some(toast) = self.home.toast() { toasts.push(toast.map(Message::Home)) }
            if !toasts.is_empty() {
                screen = screen.push(container(iced::widget::Column::with_children(toasts).spacing(8))
                    .padding(iced::Padding { bottom: 50.0, right: 18.0, left: 22.0, ..iced::Padding::ZERO }).width(Fill).height(Fill)
                    .align_x(toast_side).align_y(iced::alignment::Vertical::Bottom));
            }
        }
        if self.home.bell_open && self.setup.is_none() {
            // The notifications drop down from the bell; a click anywhere else closes them.
            screen = screen.push(mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::Home(home::Message::Bell(false))))
                .push(container(self.home.inbox_view().map(Message::Home)).padding(iced::Padding { top: 46.0, right: 96.0, ..iced::Padding::ZERO })
                    .width(Fill).align_x(iced::alignment::Horizontal::Right));
        }
        match self.dialog {
            Some(dialog) => yo3_skin::modal(screen, self.dialog(dialog), Message::CloseDialog),
            None => screen.into(),
        }
    }

    fn title_bar(&self) -> Element<'_, Message> {
        let tabs: Element<'_, Message> = if self.settings.is_some() {
            text("SETTINGS").font(yo3_skin::DISPLAY_ITALIC).size(22).color(color::ON_DARK).into()
        } else if self.setup.is_none() {
            let tab = |label, which| pill(label, Some(Message::Home(home::Message::Tab(which))), Kind::Tab { active: self.home.tab == which }, 176, 34);
            row![tab("SINGLEPLAYER", home::Tab::Single), tab("MULTIPLAYER", home::Tab::Multi)].spacing(4).into()
        } else {
            Space::new().into()
        };
        // The bell: notices about both tabs, with how many came since it was last opened.
        let bell_open = self.home.bell_open;
        let bell: Element<'_, Message> = if self.setup.is_some() { Space::new().width(34).into() } else if self.home.unread > 0 {
            stack![pill("", Some(Message::Home(home::Message::Bell(!bell_open))), Kind::Window(Glyph::Bell), 34, 28),
                container(yo3_skin::badge(self.home.unread)).width(34).align_x(iced::alignment::Horizontal::Right)].into()
        } else { pill("", Some(Message::Home(home::Message::Bell(!bell_open))), Kind::Window(Glyph::Bell), 34, 28) };
        // Settings moved to the footer's cog (the player's request, 2026-11-20): the title bar keeps the bell and the window.
        let controls = row![
            bell,
            Space::new().width(6),
            pill("", Some(Message::Minimize), Kind::Window(Glyph::Minimize), 34, 28),
            pill("", Some(Message::CloseRequested), Kind::Window(Glyph::Close), 34, 28),
        ].spacing(3);
        let bar = row![yo3_skin::logo(Some(APP_ICON.clone()), 30.0), Space::new().width(Fill), tabs, Space::new().width(Fill), controls]
            .align_y(Center).padding([0, 12]).height(Fill);
        container(yo3_skin::title_strip(mouse_area(bar).on_press(Message::Drag))).height(50).into()
    }

    fn footer(&self) -> Element<'_, Message> {
        // Support on the left; the version and what runs in the middle; Help, then Settings in the far right corner
        // (the player's layout, 2026-11-20). Report a problem is in Help and on every problem toast.
        let version = format!("v{}", yo3_tunnel::BUILD_VERSION.trim_start_matches('v'));
        // The version opens About; a separate update link opens What's new (#32).
        let mut middle = row![yo3_skin::hint(yo3_skin::link(version, Message::OpenSettings(settings::Section::About), true), "About YO3 Revival")].spacing(10).align_y(Center);
        if let Some(offered) = self.updater.offered() {
            middle = middle.push(container(Space::new().width(8).height(8)).style(|_| dot(color::WARN)))
                .push(yo3_skin::link(update_line(offered, self.updater.ready().is_some()), Message::Home(home::Message::WhatsNew), true));
        }
        // A card pictures download running while Settings is closed: its progress, which opens Settings → Game's bar.
        if let Some((done, total, _)) = self.pictures.as_ref().filter(|_| self.settings.is_none()) {
            let words = if *done == 0 { "Card pictures: starting…".to_owned() } else { format!("Card pictures · {done} of {total}") };
            middle = middle.push(container(Space::new().width(8).height(8)).style(|_| dot(color::PLAY_TOP)))
                .push(yo3_skin::hint(yo3_skin::link(words, Message::ShowPictures, true), "Downloading from ProjectIgnis: open Card pictures"));
        }
        if self.host.hosted.is_some() {
            // Hosting for friends runs while the launcher is open: the footer says so and opens its dialog.
            let online = self.host.activity.online;
            middle = middle.push(container(Space::new().width(9).height(9)).style(|_| dot(color::GOOD)))
                .push(yo3_skin::link(format!("Hosting for friends · {online} online"), Message::Home(home::Message::Multi(crate::multi::Message::Host)), true));
        }
        let mut right = row![yo3_skin::dock_button(Icon::Help, "Help", Message::OpenContact, matches!(self.dialog, Some(Dialog::Contact)))].spacing(8).align_y(Center);
        if self.setup.is_none() {
            right = right.push(yo3_skin::dock_button(Icon::Cog, "Settings", Message::ToggleSettings, self.settings.is_some()));
        }
        let footer = row![
            container(yo3_skin::support_link("Support us", Message::OpenSupport)).width(Fill),
            middle,
            container(right).width(Fill).align_x(iced::alignment::Horizontal::Right),
        ].align_y(Center).padding([0, 14]).height(Fill);
        container(yo3_skin::footer_strip(footer)).height(44).into()
    }

    fn dialog(&self, which: Dialog) -> Element<'_, Message> {
        let close = Message::CloseDialog;
        // A short dialog: a sentence or two, then its actions (the main one rightmost); the × closes it.
        let short = |title: &'static str, help: Option<&'static str>, body: String, actions: iced::widget::Row<'static, Message>, height: f32| {
            yo3_skin::dialog(title, help.map(|t| Message::OpenHelp(Some(t))), Message::CloseDialog, 470.0, height, column![
                text(body).size(15).color(color::INK), Space::new().height(Fill), row![Space::new().width(Fill), actions.spacing(10)],
            ])
        };
        match which {
            Dialog::Help => self.help.view().map(Message::Help),
            Dialog::Report => self.report_dialog(),
            Dialog::Host => self.host.view().map(Message::Host),
            Dialog::Foreign => self.foreign_dialog(),
            Dialog::WhatsNew => self.whats_new(),
            Dialog::ConfirmRestart => short("Restart to install?", None, "You are hosting for friends: restarting stops your server. Start hosting again after the update.".into(),
                row![pill("Cancel", Some(close), Kind::Secondary, 110, 38), pill("Restart", Some(Message::Restart), Kind::Primary, 120, 38)], 200.0),
            Dialog::Contact => self.contact(),
            Dialog::Support => short("Support YO3 Revival", None, "YO3 Revival is free, and nothing in it is for sale. If you enjoy it, a donation helps keep it going.".into(),
                row![pill("Discussions", Some(Message::OpenLink(DISCUSSIONS.into())), Kind::Secondary, 140, 38),
                    pill("Donate", Some(Message::OpenLink(DONATE.into())), Kind::Primary, 130, 38)], 190.0),
            Dialog::Repair => short("YO3 Revival needs repair", Some("repair"), "Some of YO3 Revival's own files are missing or changed, often after an antivirus scan. \
                Download the latest release and replace this folder with it: your settings are kept.".into(),
                row![pill("Antivirus help", Some(Message::OpenLink(ANTIVIRUS.into())), Kind::Secondary, 160, 38),
                    pill("Download", Some(Message::OpenLink(RELEASES.into())), Kind::Primary, 130, 38)], 230.0),
            Dialog::Backups => {
                // Newest first; the damaged database is kept beside the restored one, renamed.
                let backups = self.ctx.backend.as_ref().map(|b| b.backups(&self.ctx.settings)).unwrap_or_default();
                let mut list = column![text(if backups.is_empty() { "There is no backup yet: your server makes one each time it starts." }
                    else { "Your server keeps a backup from each of its last starts. Choose one to put back:" }).size(15).color(color::INK)].spacing(8);
                // Tall enough for its rows only: with no backup it is the one sentence.
                let height = 150.0 + 44.0 * backups.len().min(5) as f32;
                for backup in backups.into_iter().take(5) {
                    let label = backup.taken.clone();
                    list = list.push(row![text(label).size(15).color(color::INK).width(Fill),
                        pill("Restore", Some(Message::Home(home::Message::RestoreBackup(backup))), Kind::Primary, 110, 34)].align_y(Center));
                }
                yo3_skin::dialog("Restore a backup", None, close, 470.0, height, list)
            }
            Dialog::ConfirmClose => short("Close YO3 Revival?", None, if self.host.hosted.is_some() && self.host.activity.online > 0 {
                    "Friends are playing on your server. Closing YO3 Revival stops it, and the game too if it is open.".into()
                } else if self.home.game.extras > 0 {
                    "Games are open. Closing YO3 Revival also closes all of them, and your server.".into()
                } else { "The game is open. Closing YO3 Revival also closes the game and your server.".into() },
                row![pill("Cancel", Some(close), Kind::Secondary, 110, 38), pill("Close", Some(Message::Quit), Kind::Danger, 120, 38)], 210.0),
        }
    }
}

impl App {
    /// Another program's file or provider stopped the start (#68): what it is in plain words, what to do about it, and
    /// Continue anyway unless the game can't be kept safe with it there. The technical text is behind Details.
    fn foreign_dialog(&self) -> Element<'_, Message> {
        let Some(foreign) = &self.foreign else { return text("").into() };
        let held = &foreign.held;
        let (files, providers, security) = (held.files(), held.providers(), held.security());
        let (taken, bad) = (held.taken_over(), held.known_bad());
        let say = |words: String| text(words).size(15).color(color::INK);
        let mut body = column![].spacing(10);
        if foreign.confirm {
            body = body.push(say(format!("Move {} aside? It is renamed so that Windows doesn't load it, not deleted. Settings, Advanced can put it back.", files.join(", "))));
        } else {
            if held.clean { body = body.push(say(yo3_desktop::foreign::CLEAN_GAME.into())) }
            if !taken.is_empty() {
                body = body.push(say(format!("{} has taken over the game's network connection, so YO3 Revival can't keep the connection safe and won't start the game. \
                    Close that program, or move its file away, then press PLAY again.", taken.join(", "))));
            }
            if !bad.is_empty() { body = body.push(say(format!("{} is known to break the game's connection. Move it away, then press PLAY again.", bad.join(", ")))) }
            let others: Vec<&String> = files.iter().filter(|f| !taken.contains(f) && !bad.contains(f)).collect();
            let loaded = held.loaded_files();
            let (running, present): (Vec<&String>, Vec<&String>) = others.into_iter().partition(|f| loaded.contains(f));
            let join = |list: &[&String]| list.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", ");
            if !running.is_empty() {
                body = body.push(say(format!("Another program put {} in your game folder: {}. Windows loaded {} into the game, and it can break the game's connection, \\
                    for example by disconnecting you in the shop.", if running.len() == 1 { "a file" } else { "files" }, join(&running), if running.len() == 1 { "it" } else { "them" })));
            }
            if !present.is_empty() {
                body = body.push(say(format!("Your game folder holds {}, which {} not part of the game: {}. The game didn't load {} this time, but other programs may, and a file like this can break the game's connection.",
                    if present.len() == 1 { "a file" } else { "files" }, if present.len() == 1 { "is" } else { "are" }, join(&present), if present.len() == 1 { "it" } else { "them" })));
            }
            if !security.is_empty() {
                let who = security.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>();
                body = body.push(say(format!("{} is attached to the game's network connection. If the game disconnects you, add an exception for your game folder in its settings, \
                    or switch off its web protection while you play.", if who.is_empty() { "Security software".to_string() } else { who.join(", ") })));
            }
            if !providers.is_empty() {
                body = body.push(say("Software on this PC (often an old VPN, ad blocker or security program) sits in Windows' network path for older programs like this game, \
                    and can cut its connection. If you get disconnected, open the Command Prompt as administrator, run the line below, and restart your PC.".into()))
                    .push(row![text(yo3_desktop::foreign::WINSOCK_RESET).size(15).font(iced::Font::MONOSPACE).color(color::NAVY),
                        yo3_skin::link("Copy", Message::Copy(yo3_desktop::foreign::WINSOCK_RESET.into()), false)].spacing(14).align_y(Center));
            }
            if held.blocked && !files.is_empty() { body = body.push(say("Move it aside renames the file so Windows doesn't load it. You can undo that in Settings, Advanced.".into())) }
        }
        if let Some(sentence) = &foreign.problem { body = body.push(text(sentence.clone()).size(14).color(color::BAD)) }
        let mut more = column![yo3_skin::link(if foreign.details { "Hide details" } else { "Details" }, Message::Foreign(ForeignMessage::Details), false)].spacing(6);
        if foreign.details {
            let all = match &foreign.failure { Some(failure) => format!("{}\n\n{failure}", held.details()), None => held.details() };
            more = more.push(text(all.clone()).size(13).color(color::INK_SOFT)).push(yo3_skin::link("Copy", Message::Copy(all), false));
        }
        let idle = !foreign.busy;
        let mut actions = row![Space::new().width(Fill)].spacing(10);
        if foreign.confirm {
            actions = actions.push(pill("Back", idle.then_some(Message::Foreign(ForeignMessage::Back)), Kind::Secondary, 110, 38))
                .push(pill("Yes, move it", idle.then_some(Message::Foreign(ForeignMessage::ConfirmMove)), Kind::Primary, 150, 38));
        } else {
            actions = actions.push(pill("Cancel", Some(Message::CloseDialog), Kind::Secondary, 110, 38));
            let movable = !files.is_empty();
            if !held.blocked { actions = actions.push(pill("Continue anyway", Some(Message::Foreign(ForeignMessage::Continue)), if movable { Kind::Secondary } else { Kind::Primary }, 170, 38)) }
            if movable { actions = actions.push(pill("Move it aside", Some(Message::Foreign(ForeignMessage::Move)), Kind::Primary, 150, 38)) }
        }
        // Only files the game didn't load: nothing is running inside the game, so the title says what was found.
        let loaded = held.loaded_files();
        let only_present = taken.is_empty() && security.is_empty() && providers.is_empty() && files.iter().all(|f| !loaded.contains(f));
        let title = if held.clean { "This server asks for a clean game" }
            else if held.blocked { "YO3 Revival can't start the game safely" }
            else if only_present { "A file in your game folder isn't part of the game" }
            else { "Another program is running in your game" };
        yo3_skin::dialog(title,
            Some(Message::OpenHelp(Some("foreign"))), Message::CloseDialog, 540.0, 430.0,
            column![iced::widget::scrollable(column![body, more].spacing(12)).height(Fill), actions].spacing(12))
    }

    /// What's new in the offered version: its changelog section, where its download is, and Restart to install.
    fn whats_new(&self) -> Element<'_, Message> {
        let Some(release) = self.updater.release() else {
            return yo3_skin::dialog("What's new", None, Message::CloseDialog, 470.0, 170.0, text("You have the latest version.").size(15).color(color::INK));
        };
        let playing = self.home.playing();
        let mut notes = column![].spacing(8);
        for block in updates::flow::blocks(&updates::flow::whats_new(&release.notes)) {
            let spans: Vec<iced::widget::text::Span<'static, (), iced::Font>> = block.runs.into_iter()
                .map(|(words, bold)| iced::widget::span(words).font(if bold { yo3_skin::BODY_BOLD } else { yo3_skin::BODY })).collect();
            let paragraph = iced::widget::rich_text(spans).size(15).color(color::INK).width(Fill);
            notes = notes.push(if block.bullet { row![text("•").size(15).color(color::INK), paragraph].spacing(8).into() } else { Element::from(paragraph) });
        }
        let restart = self.updater.ready().is_some() && !playing && !self.quitting;
        let mut progress = column![].spacing(6);
        if matches!(self.updater.stage, updates::flow::Stage::Downloading(_)) {
            let (fraction, words) = updates::flow::download_line(self.transfer.bytes(), release.asset.bytes, self.transfer.verifying());
            progress = progress.push(yo3_skin::progress_bar(fraction.map_or(yo3_skin::Bar::Starting, yo3_skin::Bar::Fraction), words, 22.0));
        }
        let mut buttons = row![yo3_skin::icon_link(Icon::Open, "Release page", Message::OpenLink(release.url.clone()), false), Space::new().width(Fill)].spacing(10).align_y(Center);
        if self.updater.ready().is_some() { buttons = buttons.push(pill("Restart to install", restart.then_some(Message::Home(home::Message::RestartToInstall)), Kind::Primary, 190, 38)) }
        yo3_skin::dialog(&format!("What's new in {}", release.tag.trim_start_matches('v')), None, Message::CloseDialog, 620.0, 440.0, column![
            iced::widget::scrollable(notes).height(Fill),
            progress,
            text(self.updater.status(playing)).size(14).font(yo3_skin::BODY_BOLD).color(color::INK_SOFT),
            buttons,
        ].spacing(12))
    }

    /// Help & contact: the three ways to get a hand, each one line and one button.
    fn contact(&self) -> Element<'_, Message> {
        let way = |icon: Icon, title: &'static str, line: &'static str, button: &'static str, action: Message| {
            container(row![
                yo3_skin::icon(icon, 26.0, color::NAVY),
                column![text(title).size(15).font(yo3_skin::BODY_BOLD).color(color::INK), text(line).size(14).color(color::INK_SOFT)].spacing(2).width(Fill),
                pill(button, Some(action), Kind::Secondary, 130, 34),
            ].spacing(14).align_y(Center)).padding([10, 12]).width(Fill).style(|_| container::Style { background: Some(color::ROW.into()),
                border: iced::Border { color: color::CHROME, width: 1.0, radius: 4.0.into() }, ..container::Style::default() })
        };
        yo3_skin::dialog("Need a hand?", None, Message::CloseDialog, 600.0, 350.0, column![
            text("YO3 Revival is a small fan project made by players. We read every question and report.").size(15).color(color::INK),
            way(Icon::Book, "Read the guide", "Setup, playing, hosting, and fixes for common problems.", "Open guide", Message::OpenHelp(None)),
            way(Icon::Chat, "Ask the community", "Questions, ideas and servers to join, on GitHub.", "Discussions", Message::OpenLink(DISCUSSIONS.into())),
            way(Icon::Flag, "Report a problem", "Tell us what went wrong. We help you attach your logs.", "Report…", Message::OpenReport(String::new())),
        ].spacing(10))
    }

    fn report_dialog(&self) -> Element<'_, Message> {
        let report = &self.report;
        let help = Some(Message::OpenHelp(Some("report")));
        if let Some(Ok(sent)) = &report.result {
            // Done here: what is left happens in the browser.
            return yo3_skin::dialog("Report a problem", help, Message::CloseDialog, 520.0, 250.0, column![
                yo3_skin::rule(yo3_skin::Rule::Met, "The report form is open in your browser."),
                text(if *sent { "Your logs went to the server's host, and the form has their report ID. Press Submit there." }
                    else { "Drag the logs file from the folder that opened into the form, then press Submit there." }).size(15).color(color::INK),
                Space::new().height(Fill),
                row![Space::new().width(Fill), pill("Done", Some(Message::CloseDialog), Kind::Primary, 120, 38)],
            ].spacing(10));
        }
        let mut col = column![
            text("What happened?").size(15).font(yo3_skin::BODY_BOLD).color(color::INK),
            iced::widget::text_editor(&report.what).placeholder("What were you doing, and what went wrong?").on_action(|a| Message::Report(ReportMessage::Edit(a)))
                .height(110).padding(8).size(15).style(|theme, status| {
                    let field = yo3_skin::field(theme, match status {
                        iced::widget::text_editor::Status::Focused { is_hovered } => iced::widget::text_input::Status::Focused { is_hovered },
                        iced::widget::text_editor::Status::Hovered => iced::widget::text_input::Status::Hovered,
                        iced::widget::text_editor::Status::Disabled => iced::widget::text_input::Status::Disabled,
                        iced::widget::text_editor::Status::Active => iced::widget::text_input::Status::Active,
                    });
                    iced::widget::text_editor::Style { background: field.background, border: field.border, placeholder: field.placeholder, value: field.value, selection: field.selection }
                }),
            text(if report.error.is_empty() { "Your recent logs go with it, personal details removed. You send it yourself, on GitHub." }
                else { "The error you saw and your recent logs go with it, personal details removed. You send it yourself, on GitHub." }).size(13).color(color::INK_SOFT),
        ].spacing(8);
        match &report.result {
            Some(Err(_)) => col = col.push(yo3_skin::rule(yo3_skin::Rule::Broken, "The report couldn't be prepared. Settings, Advanced, Export logs saves the logs alone.")),
            _ if report.busy => col = col.push(text("Saving the logs…").size(14).color(color::INK_SOFT)),
            _ => {}
        }
        let ready = !report.busy && !report.what.text().trim().is_empty();
        let buttons = row![Space::new().width(Fill), pill("Continue on GitHub", ready.then_some(Message::Report(ReportMessage::Send)), Kind::Primary, 200, 38)];
        yo3_skin::dialog("Report a problem", help, Message::CloseDialog, 520.0, 330.0, column![col, Space::new().height(Fill), buttons])
    }
}

/// The app's mark (the Duel Disk with the Monster Reborn badge, without its background), decoded at build time.
static APP_ICON: std::sync::LazyLock<iced::widget::image::Handle> = std::sync::LazyLock::new(|| {
    iced::widget::image::Handle::from_rgba(128, 128, include_bytes!(concat!(env!("OUT_DIR"), "/launcher-mark.rgba")).to_vec())
});

/// Why a restart into the update waited: the game is open.
const GAME_OPEN: &str = "The game is open.";
const RELEASES: &str = "https://github.com/usersina/yo3-revival/releases/latest";
/// The antivirus help page (#38): why our files get flagged, checking a download, restoring a quarantined file.
pub const ANTIVIRUS: &str = "https://github.com/usersina/yo3-revival/blob/main/docs/antivirus.md";
/// The project's discussions on GitHub: questions, ideas, servers to join.
pub const DISCUSSIONS: &str = "https://github.com/usersina/yo3-revival/discussions";
/// Where a donation goes (the footer's Support).
pub const DONATE: &str = "https://donatr.ee/usersina/";

/// Beside the version's dot: which version, and whether it waits for a restart.
fn update_line(version: &str, ready: bool) -> String {
    if ready { format!("Update {version} is ready") } else { format!("Update {version} is available") }
}

pub fn dot(c: Color) -> container::Style {
    container::Style { background: Some(c.into()), border: iced::Border { color: Color { a: 0.5, ..Color::BLACK }, width: 1.0, radius: 5.0.into() },
        ..container::Style::default() }
}

/// Brings this window forward whenever another start of the launcher asks for it.
fn listen_for_second_starts(root: &std::path::Path, claim: Claim) -> Task<Message> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let root = root.to_owned();
    std::thread::spawn(move || {
        let owner = match claim {
            Claim::Now(owner) => owner,
            Claim::AfterUpdate => match instance::claim_when_free(&root, std::time::Duration::from_secs(60)) { Ok(owner) => owner, Err(_) => return },
        };
        owner.serve(move || { let _ = tx.send(()); });
    });
    Task::run(iced::futures::stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|()| ((), rx)) }), |()| Message::Show)
}

/// Paths a player shouldn't have to read, replaced by names (`<game>`, `<app data>`, …) in Details.
pub fn masks(ctx: &Ctx) -> Vec<(String, &'static str)> {
    let settings = &ctx.settings;
    let mut prefixes = vec![(ctx.root.join("profiles").to_string_lossy().into_owned(), "<profile>"),
        (ctx.root.to_string_lossy().into_owned(), "<app data>"), (settings.game_dir.clone(), "<game>")];
    if let Some(backend) = &ctx.backend {
        prefixes.push((backend.data_dir(settings).to_string_lossy().into_owned(), "<server data>"));
        for file in [&backend.runtime.injector, &backend.runtime.dll, &backend.runtime.launcher] {
            if let Some(path) = file.parent() { prefixes.push((path.to_string_lossy().into_owned(), "<app>")) }
        }
    }
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) { prefixes.push((home.to_string_lossy().into_owned(), "<user>")) }
    prefixes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Good news goes by itself after a few seconds, a new sentence gets its own time, and a problem (never timed) stays.
    #[test]
    fn a_good_toast_goes_by_itself_and_a_new_one_gets_its_own_time() {
        let start = Instant::now();
        let (timed, gone) = toast_step(None, Some("Backed up.".into()), start);
        assert!(!gone && timed.as_ref().is_some_and(|(s, _)| s == "Backed up."));
        let (timed, gone) = toast_step(timed, Some("Backed up.".into()), start + Duration::from_secs(TOAST_SECONDS - 1));
        assert!(!gone, "still showing");
        let (renewed, gone) = toast_step(timed.clone(), Some("The shortcut is on your desktop.".into()), start + Duration::from_secs(TOAST_SECONDS));
        assert!(!gone && renewed.as_ref().is_some_and(|(s, at)| s.starts_with("The shortcut") && *at == start + Duration::from_secs(TOAST_SECONDS)), "a new sentence starts again");
        let (timed, gone) = toast_step(timed, Some("Backed up.".into()), start + Duration::from_secs(TOAST_SECONDS));
        assert!(gone && timed.is_none(), "its time is up");
        assert_eq!(toast_step(Some(("x".into(), start)), None, start), (None, false), "dismissed by hand: nothing to time");
    }
}
