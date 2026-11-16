//! The window: a title bar, the current screen (setup or the main screen), a footer and at most one dialog.
//! Each screen keeps its own state and messages; this module routes between them and owns what they share.
use std::path::PathBuf;
use std::sync::Arc;
use iced::widget::{Space, column, container, mouse_area, operation, row, stack, text, text_input};
use iced::{Alignment::Center, Color, Element, Length::Fill, Subscription, Task, keyboard, window};
use yo3_desktop::{Backend, Settings, instance, store::Store, updates};
use yo3_skin::{Glyph, Kind, color, pill};
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
pub enum Dialog { Help, Report, Repair, ConfirmClose, Backups, Host, Support, WhatsNew, ConfirmRestart }

/// Report a problem: what the player says happened, the error it is about, and how far it got.
#[derive(Clone, Debug, Default)]
pub struct Report { pub what: String, pub error: String, pub busy: bool, pub result: Option<Result<(), String>> }

#[derive(Clone, Debug)]
pub enum ReportMessage { Edit(String), Send, Sent(Result<(), String>) }

#[derive(Clone, Debug)]
pub enum Message {
    Home(home::Message),
    Setup(setup::Message),
    Settings(settings::Message),
    OpenSettings(settings::Section),
    Help(help::Message),
    OpenHelp(Option<&'static str>),
    OpenReport(String),
    /// The footer's heart: how to support the project.
    OpenSupport,
    Report(ReportMessage),
    Copy(String),
    Host(host::Message),
    CloseDialog,
    Show,
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
}

pub struct App {
    ctx: Ctx,
    home: home::State,
    setup: Option<setup::State>,
    settings: Option<settings::State>,
    help: help::State,
    report: Report,
    host: host::State,
    dialog: Option<Dialog>,
    updater: updates::flow::Updater,
    startup: updates::activation::Startup,
    quitting: bool,
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
        (Self { ctx, home, setup, settings: None, help: help::State::at(None), report: Report::default(), host: host::State::default(), dialog: None, updater, startup, quitting: false },
            Task::batch(tasks))
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
            Message::Home(home::Message::Settings(section)) => return self.update(Message::OpenSettings(section)),
            Message::Home(home::Message::Report(error)) => return self.update(Message::OpenReport(error)),
            Message::Home(home::Message::Help(topic)) => return self.update(Message::OpenHelp(Some(topic))),
            Message::Home(home::Message::Copy(text)) => return self.update(Message::Copy(text)),
            Message::Home(home::Message::Multi(crate::multi::Message::Host)) => { self.host.open(&self.ctx); self.dialog = Some(Dialog::Host) }
            Message::Home(home::Message::RestoreBackup(backup)) => {
                self.dialog = None;
                return self.home.update(home::Message::RestoreBackup(backup), &mut self.ctx).map(Message::Home);
            }
            Message::Home(message) => return self.home.update(message, &mut self.ctx).map(Message::Home),
            Message::Setup(setup::Message::Saved(Ok((settings, note)))) => {
                self.ctx.settings = *settings;
                self.setup = None;
                if let Some(note) = note { self.home.fail(&self.ctx, note) }
                return Task::batch([self.home.check(&self.ctx).map(Message::Home), self.home.load_banlists(&self.ctx).map(Message::Home)]);
            }
            Message::Setup(setup::Message::Open(setup::GETTING_THE_GAME)) => return self.update(Message::OpenHelp(Some("game"))),
            Message::Setup(setup::Message::Open(url)) => return self.update(Message::OpenLink(url.into())),
            Message::Setup(setup::Message::Repair) => self.dialog = Some(Dialog::Repair),
            Message::Setup(setup::Message::Cancel) => self.setup = None,
            Message::Setup(message) => if let Some(setup) = &mut self.setup { return setup.update(message, &self.ctx).map(Message::Setup) },
            Message::Settings(message) => return self.settings_update(message),
            Message::OpenSettings(section) => {
                if self.setup.is_some() { return Task::none() }
                let state = settings::State::new(&self.ctx, section);
                let start = state.start(&self.ctx).map(Message::Settings);
                // Accounts lists the Multiplayer tab's sign-ins: load its servers if it hasn't yet.
                let load = self.home.multi.load(&self.ctx).map(|m| Message::Home(home::Message::Multi(m)));
                self.settings = Some(state);
                return Task::batch([start, load]);
            }
            Message::Help(help::Message::Topic(id)) => self.help.update(&help::Message::Topic(id)),
            Message::Help(help::Message::Report) => return self.update(Message::OpenReport(String::new())),
            Message::Help(help::Message::Close) => self.dialog = None,
            Message::Help(help::Message::Open(url)) => return self.update(Message::OpenLink(url)),
            Message::OpenSupport => self.dialog = Some(Dialog::Support),
            Message::OpenHelp(topic) => { self.help = help::State::at(topic); self.dialog = Some(Dialog::Help) }
            Message::OpenReport(error) => { self.report = Report { error, ..Report::default() }; self.dialog = Some(Dialog::Report) }
            Message::Report(ReportMessage::Edit(what)) => self.report.what = what.chars().filter(|c| *c == '\n' || !c.is_control()).take(2000).collect(),
            Message::Report(ReportMessage::Send) => {
                let Some(backend) = self.ctx.backend.clone() else { return Task::none() };
                let (what, error) = (self.report.what.clone(), self.report.error.clone());
                self.report.busy = true;
                self.report.result = None;
                return job(move || {
                    let prepared = backend.report_problem(&what, &error)?;
                    // The saved logs' folder, so the player can drag the file into the form; then the form.
                    if let Some(dir) = prepared.zip.parent() { let _ = external::open(&dir.to_string_lossy()); }
                    external::open(&prepared.url)
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
            Message::Show => return window::latest().and_then(|id| Task::batch([window::minimize(id, false), window::gain_focus(id)])),
            Message::Drag => return window::latest().and_then(window::drag),
            Message::Minimize => return window::latest().and_then(|id| window::minimize(id, true)),
            Message::CloseRequested => {
                let friends = self.host.hosted.is_some() && self.host.activity.online > 0;
                if self.home.game.running || self.home.game.starting.is_some() || friends { self.dialog = Some(Dialog::ConfirmClose) } else { return self.update(Message::Quit) }
            }
            Message::Quit => {
                if self.quitting { return Task::none() }
                self.quitting = true;
                self.dialog = None;
                let Some(backend) = self.ctx.backend.clone() else { return iced::exit() };
                let settings = self.ctx.settings.clone();
                // Your own server lives as long as the launcher (until #29 ties it to the game).
                return job(move || {
                    backend.stop_game()?;
                    let _ = backend.stop_hosting();
                    if backend.live(&settings).is_some() { backend.stop_server(&settings)?; }
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
                keyboard::key::Named::Escape => if self.dialog.is_some() { self.dialog = None } else { self.settings = None },
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
                return Task::perform(async move {
                    let installed = updates::download::prepare(root.clone(), release, Default::default()).await?;
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
                    if backend.live(&settings).is_some() { backend.stop_server(&settings)?; }
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
        }
        Task::none()
    }

    fn settings_update(&mut self, message: settings::Message) -> Task<Message> {
        match message {
            settings::Message::Close => { self.settings = None; Task::none() }
            settings::Message::ChangeGame => { self.settings = None; self.open_setup(setup::Step::Game) }
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
                if matches!(message, settings::Message::PermissionFixed(Ok(()))) { self.home.permission_granted() }
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
        if self.setup.is_none() { all.push(self.home.subscription().map(Message::Home)) }
        all.push(self.host.subscription().map(Message::Host));
        all.push(iced::time::every(std::time::Duration::from_secs(10 * 60)).map(|_| Message::UpdateTick));
        Subscription::batch(all)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match (&self.setup, &self.settings) {
            (Some(setup), _) => setup.view().map(Message::Setup),
            (None, Some(settings)) => settings.view(&self.ctx, &self.home).map(Message::Settings),
            (None, None) => self.home.view(&self.ctx).map(Message::Home),
        };
        let page = column![self.title_bar(), container(body).height(Fill), self.footer()];
        let screen = stack![yo3_skin::backdrop(), page];
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
        let controls = row![
            pill("", (self.setup.is_none()).then_some(if self.settings.is_some() { Message::Settings(settings::Message::Close) } else { Message::OpenSettings(settings::Section::Game) }),
                Kind::Window(Glyph::Settings), 34, 28),
            pill("", Some(Message::Minimize), Kind::Window(Glyph::Minimize), 34, 28),
            pill("", Some(Message::CloseRequested), Kind::Window(Glyph::Close), 34, 28),
        ].spacing(3);
        let bar = row![yo3_skin::logo(Some(APP_ICON.clone()), 30.0), Space::new().width(Fill), tabs, Space::new().width(Fill), controls]
            .align_y(Center).padding([0, 12]).height(Fill);
        container(yo3_skin::title_strip(mouse_area(bar).on_press(Message::Drag))).height(50).into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let version = format!("v{}", yo3_tunnel::BUILD_VERSION.trim_start_matches('v'));
        // A newer version is known: a dot on the version, which opens What's new (#32).
        let mut line = match self.updater.offered() {
            Some(offered) => row![
                row![yo3_skin::link(version, Message::Home(home::Message::WhatsNew), true),
                    container(container(Space::new().width(8).height(8)).style(|_| dot(color::WARN))).height(20).align_y(iced::alignment::Vertical::Top)].spacing(1),
                yo3_skin::link(update_line(offered, self.updater.ready().is_some()), Message::Home(home::Message::WhatsNew), true),
            ],
            None => row![text(version).size(14).color(color::ON_DARK_SOFT)],
        }.spacing(10).align_y(Center);
        if self.host.hosted.is_some() {
            // Hosting for friends runs while the launcher is open: the footer says so and opens its dialog.
            let online = self.host.activity.online;
            line = line.push(container(Space::new().width(9).height(9)).style(|_| dot(color::GOOD)))
                .push(yo3_skin::link(format!("Hosting for friends · {online} online"), Message::Home(home::Message::Multi(crate::multi::Message::Host)), true));
        }
        let footer = line.push(Space::new().width(Fill)).push(yo3_skin::heart_link("Support", Message::OpenSupport))
            .push(Space::new().width(18)).push(yo3_skin::link("Report a problem", Message::OpenReport(String::new()), true))
            .push(Space::new().width(18)).push(yo3_skin::link("Help", Message::OpenHelp(None), true))
            .padding([0, 16]).height(Fill);
        container(yo3_skin::footer_strip(footer)).height(34).into()
    }

    fn dialog(&self, which: Dialog) -> Element<'_, Message> {
        let ok = || pill("OK", Some(Message::CloseDialog), Kind::Primary, 120, 38);
        let (title, body, buttons, height): (&str, String, Element<'_, Message>, f32) = match which {
            Dialog::Help => return self.help.view().map(Message::Help),
            Dialog::Report => return self.report_dialog(),
            Dialog::Host => return self.host.view().map(Message::Host),
            Dialog::WhatsNew => return self.whats_new(),
            Dialog::ConfirmRestart => ("Restart to install?", "You are hosting for friends. Restarting YO3 Revival stops your server; \
                start hosting again after the update.".into(),
                row![pill("Cancel", Some(Message::CloseDialog), Kind::Primary, 120, 38), pill("Restart", Some(Message::Restart), Kind::Primary, 120, 38)].spacing(10).into(), 210.0),
            Dialog::Support => ("Support YO3 Revival", "YO3 Revival is free, and stays free: nothing in the game is for sale, and \
                giving changes nothing in it. If you enjoy it, a donation helps pay for the time it takes and for the server. Questions, \
                ideas and servers to join are in the project's discussions.".into(),
                row![pill("Discussions", Some(Message::OpenLink(DISCUSSIONS.into())), Kind::Primary, 140, 38),
                    pill("Donate", Some(Message::OpenLink(DONATE.into())), Kind::Primary, 120, 38),
                    pill("Close", Some(Message::CloseDialog), Kind::Primary, 120, 38)].spacing(10).into(), 225.0),
            Dialog::Repair => ("YO3 Revival needs repair", "Some of YO3 Revival's own files are missing or changed, often after an \
                antivirus scan. Download the latest release again, check it, and replace this folder with it; your settings are kept. \
                Antivirus help says how to check the download and get a removed file back.".into(),
                row![pill("Antivirus help", Some(Message::OpenLink(ANTIVIRUS.into())), Kind::Primary, 160, 38),
                    pill("Releases", Some(Message::OpenLink(RELEASES.into())), Kind::Primary, 120, 38), ok()].spacing(10).into(), 280.0),
            Dialog::Backups => {
                // Newest first; the damaged database is kept beside the restored one, renamed.
                let backups = self.ctx.backend.as_ref().map(|b| b.backups(&self.ctx.settings)).unwrap_or_default();
                let mut list = column![text(if backups.is_empty() { "There is no backup yet: your server makes one each time it starts." }
                    else { "Your server keeps a backup from each of its last starts. Choose one to put back:" }).size(15).color(color::INK)].spacing(8);
                for backup in backups.into_iter().take(5) {
                    let label = backup.taken.clone();
                    list = list.push(row![text(label).size(15).color(color::INK).width(Fill),
                        pill("Restore", Some(Message::Home(home::Message::RestoreBackup(backup))), Kind::Primary, 110, 34)].align_y(Center));
                }
                return container(yo3_skin::panel(Some("Restore a backup"), column![list, Space::new().height(Fill),
                    row![Space::new().width(Fill), pill("Cancel", Some(Message::CloseDialog), Kind::Primary, 120, 38)]])).width(470).height(380).into();
            }
            Dialog::ConfirmClose => ("Close YO3 Revival?", if self.host.hosted.is_some() && self.host.activity.online > 0 {
                    "Friends are playing on your server. Closing YO3 Revival stops it, and the game too if it is open.".into()
                } else { "The game is open. Closing YO3 Revival also closes the game and your server.".into() },
                row![pill("Cancel", Some(Message::CloseDialog), Kind::Primary, 120, 38), pill("Close", Some(Message::Quit), Kind::Primary, 120, 38)].spacing(10).into(), 230.0),
        };
        container(yo3_skin::panel(Some(title), column![
            text(body).size(15).color(color::INK),
            Space::new().height(Fill),
            row![Space::new().width(Fill), buttons],
        ])).width(470).height(height).into()
    }
}

impl App {
    /// What's new in the offered version: its changelog section, where its download is, and Restart to install.
    fn whats_new(&self) -> Element<'_, Message> {
        let Some(release) = self.updater.release() else {
            return container(yo3_skin::panel(Some("What's new"), column![text("You have the latest version.").size(15).color(color::INK),
                Space::new().height(Fill), row![Space::new().width(Fill), pill("Close", Some(Message::CloseDialog), Kind::Primary, 120, 38)]])).width(470).height(200).into()
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
        let mut buttons = row![yo3_skin::link("Release page", Message::OpenLink(release.url.clone()), false), Space::new().width(Fill)].spacing(10).align_y(Center);
        if self.updater.ready().is_some() { buttons = buttons.push(pill("Restart to install", restart.then_some(Message::Home(home::Message::RestartToInstall)), Kind::Primary, 190, 38)) }
        buttons = buttons.push(pill("Close", Some(Message::CloseDialog), Kind::Primary, 120, 38));
        container(yo3_skin::panel(Some(&format!("What's new in {}", release.tag.trim_start_matches('v'))), column![
            iced::widget::scrollable(notes).height(Fill),
            text(self.updater.status(playing)).size(14).font(yo3_skin::BODY_BOLD).color(color::INK_SOFT),
            buttons,
        ].spacing(12))).width(620).height(440).into()
    }

    fn report_dialog(&self) -> Element<'_, Message> {
        let report = &self.report;
        let mut col = column![
            text("Tell us what happened. YO3 Revival saves your recent logs in a file (personal details removed) and opens the project's \
                report form in your browser: drag the file into it. Nothing is sent until you submit the form yourself.").size(15).color(color::INK),
            text_input("What were you doing, and what went wrong?", &report.what).on_input(|v| Message::Report(ReportMessage::Edit(v)))
                .on_submit(Message::Report(ReportMessage::Send)).style(yo3_skin::field).padding(8).size(15),
        ].spacing(12);
        if !report.error.is_empty() { col = col.push(text("The error you saw is included.").size(14).color(color::INK_SOFT)) }
        match &report.result {
            Some(Ok(())) => col = col.push(text("Done: the form is open in your browser, and the logs' folder beside it.").size(14).font(yo3_skin::BODY_BOLD).color(color::GOOD)),
            Some(Err(_)) => col = col.push(text("The report couldn't be prepared. Settings, Advanced, Export logs saves the logs on their own.").size(14).font(yo3_skin::BODY_BOLD).color(color::BAD)),
            None if report.busy => col = col.push(text("Saving the logs…").size(14).color(color::INK_SOFT)),
            None => {}
        }
        let buttons = row![Space::new().width(Fill), pill("Open the report form", (!report.busy).then_some(Message::Report(ReportMessage::Send)), Kind::Primary, 210, 38),
            pill("Close", Some(Message::CloseDialog), Kind::Primary, 120, 38)].spacing(10);
        container(yo3_skin::panel(Some("Report a problem"), column![col, Space::new().height(Fill), buttons])).width(560).height(340).into()
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
