//! The window: a title bar, the current screen (setup or the main screen), a footer and at most one dialog.
//! Each screen keeps its own state and messages; this module routes between them and owns what they share.
use std::path::PathBuf;
use std::sync::Arc;
use iced::widget::{Space, column, container, mouse_area, operation, row, stack, text};
use iced::{Alignment::Center, Color, Element, Length::Fill, Subscription, Task, keyboard, window};
use yo3_desktop::{Backend, Settings, instance, store::Store, updates};
use yo3_skin::{Glyph, Kind, color, pill};
use crate::{external, home, setup};

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
pub enum Dialog { Help, Settings, Repair, ConfirmClose }

#[derive(Clone, Debug)]
pub enum Message {
    Home(home::Message),
    Setup(setup::Message),
    Open(Dialog),
    CloseDialog,
    ChangeGame,
    Show,
    Drag,
    Minimize,
    CloseRequested,
    Quit,
    Exit,
    WindowOpened,
    Key(keyboard::Event),
    UpdateChecked(Result<updates::Decision, String>),
    OpenLink(String),
    LinkOpened(Result<(), String>),
}

pub struct App {
    ctx: Ctx,
    home: home::State,
    setup: Option<setup::State>,
    dialog: Option<Dialog>,
    update: Option<updates::Decision>,
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
    pub fn boot(root: PathBuf, startup: updates::activation::Startup, routing_error: Option<String>, claim: Claim) -> (Self, Task<Message>) {
        let mut problem = routing_error;
        let (store, settings) = match Store::open(&root) {
            Ok((store, settings)) => (Some(store), settings),
            Err(error) => { problem = Some(format!("Could not open the launcher's settings: {error:#}")); (None, Settings::default()) }
        };
        let backend = match Backend::new(root.clone()) {
            Ok(backend) => Some(backend),
            Err(error) => { problem.get_or_insert(format!("YO3 Revival needs repair: {error:#}")); None }
        };
        let ctx = Ctx { root, backend, store, settings };
        let mut home = home::State::default();
        if let Some(problem) = problem { home.fail(&ctx, problem) }
        // Until setup has been finished once, the launcher opens on it.
        let mut setup = (!ctx.settings.setup_complete).then(|| setup::State::new(&ctx, setup::Step::Welcome, false));
        let mut tasks = vec![
            home.check(&ctx).map(Message::Home),
            home.load_banlists(&ctx).map(Message::Home),
            Task::perform(async { updates::check().await.map_err(|e| format!("{e:#}")) }, Message::UpdateChecked),
            listen_for_second_starts(&ctx.root, claim),
        ];
        if let Some(setup) = &mut setup { tasks.push(setup.start(&ctx).map(Message::Setup)) }
        (Self { ctx, home, setup, dialog: None, update: None, startup, quitting: false }, Task::batch(tasks))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Home(home::Message::SetUp) => return self.open_setup(setup::Step::Game),
            Message::Home(home::Message::Repair) => self.dialog = Some(Dialog::Repair),
            Message::Home(message) => return self.home.update(message, &mut self.ctx).map(Message::Home),
            Message::Setup(setup::Message::Saved(Ok((settings, note)))) => {
                self.ctx.settings = *settings;
                self.setup = None;
                if let Some(note) = note { self.home.fail(&self.ctx, note) }
                return Task::batch([self.home.check(&self.ctx).map(Message::Home), self.home.load_banlists(&self.ctx).map(Message::Home)]);
            }
            Message::Setup(setup::Message::Open(url)) => return self.update(Message::OpenLink(url.into())),
            Message::Setup(setup::Message::Repair) => self.dialog = Some(Dialog::Repair),
            Message::Setup(setup::Message::Cancel) => self.setup = None,
            Message::Setup(message) => if let Some(setup) = &mut self.setup { return setup.update(message, &self.ctx).map(Message::Setup) },
            Message::Open(dialog) => self.dialog = Some(dialog),
            Message::CloseDialog => self.dialog = None,
            Message::ChangeGame => { self.dialog = None; return self.open_setup(setup::Step::Welcome) }
            Message::Show => return window::latest().and_then(|id| Task::batch([window::minimize(id, false), window::gain_focus(id)])),
            Message::Drag => return window::latest().and_then(window::drag),
            Message::Minimize => return window::latest().and_then(|id| window::minimize(id, true)),
            Message::CloseRequested => {
                if self.home.game.running || self.home.game.starting { self.dialog = Some(Dialog::ConfirmClose) } else { return self.update(Message::Quit) }
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
                    if backend.live(&settings).is_some() { backend.stop_server(&settings)?; }
                    Ok(())
                }, |_| Message::Exit);
            }
            Message::Exit => return iced::exit(),
            Message::WindowOpened => {
                // The updater waits for this before it switches versions.
                if let Some(readiness) = self.startup.readiness.take() && let Err(error) = readiness.write(yo3_tunnel::BUILD_VERSION) {
                    self.home.fail(&self.ctx, format!("Update readiness failed: {error:#}"));
                }
            }
            Message::Key(keyboard::Event::KeyPressed { key: keyboard::Key::Named(named), modifiers, .. }) => match named {
                keyboard::key::Named::Tab if modifiers.shift() => return operation::focus_previous(),
                keyboard::key::Named::Tab => return operation::focus_next(),
                keyboard::key::Named::Escape => self.dialog = None,
                _ => {}
            },
            Message::Key(_) => {}
            Message::UpdateChecked(result) => self.update = result.ok(),
            Message::OpenLink(url) => return job(move || external::open(&url), Message::LinkOpened),
            Message::LinkOpened(result) => if let Err(error) = result { self.home.fail(&self.ctx, error) },
        }
        Task::none()
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
        Subscription::batch(all)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match &self.setup {
            Some(setup) => setup.view().map(Message::Setup),
            None => self.home.view(&self.ctx).map(Message::Home),
        };
        let page = column![self.title_bar(), container(body).height(Fill), self.footer()];
        let screen = stack![yo3_skin::backdrop(), page];
        match self.dialog {
            Some(dialog) => yo3_skin::modal(screen, self.dialog(dialog), Message::CloseDialog),
            None => screen.into(),
        }
    }

    fn title_bar(&self) -> Element<'_, Message> {
        let tabs: Element<'_, Message> = if self.setup.is_none() {
            let tab = |label, which| pill(label, Some(Message::Home(home::Message::Tab(which))), Kind::Tab { active: self.home.tab == which }, 176, 34);
            row![tab("SINGLEPLAYER", home::Tab::Single), tab("MULTIPLAYER", home::Tab::Multi)].spacing(4).into()
        } else {
            Space::new().into()
        };
        let controls = row![
            pill("", (self.setup.is_none()).then_some(Message::Open(Dialog::Settings)), Kind::Window(Glyph::Settings), 34, 28),
            pill("", Some(Message::Minimize), Kind::Window(Glyph::Minimize), 34, 28),
            pill("", Some(Message::CloseRequested), Kind::Window(Glyph::Close), 34, 28),
        ].spacing(3);
        let bar = row![yo3_skin::logo(30.0), Space::new().width(Fill), tabs, Space::new().width(Fill), controls]
            .align_y(Center).padding([0, 12]).height(Fill);
        container(yo3_skin::title_strip(mouse_area(bar).on_press(Message::Drag))).height(50).into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let mut line = row![text(format!("v{}", yo3_tunnel::BUILD_VERSION.trim_start_matches('v'))).size(14).color(color::ON_DARK_SOFT)].spacing(10).align_y(Center);
        if let Some(updates::Decision::Available(release)) = &self.update {
            line = line.push(container(Space::new().width(9).height(9)).style(|_| dot(color::WARN)))
                .push(yo3_skin::link(update_line(&release.tag), Message::OpenLink(release.url.clone()), true));
        }
        let footer = line.push(Space::new().width(Fill)).push(yo3_skin::link("Help", Message::Open(Dialog::Help), true))
            .padding([0, 16]).height(Fill);
        container(yo3_skin::footer_strip(footer)).height(34).into()
    }

    fn dialog(&self, which: Dialog) -> Element<'_, Message> {
        let ok = || pill("OK", Some(Message::CloseDialog), Kind::Primary, 120, 38);
        let (title, body, buttons, height): (&str, String, Element<'_, Message>, f32) = match which {
            Dialog::Help => ("Help", "The full guide, troubleshooting and Report a problem come to this window in a later \
                beta version. Until then, the Desktop guide and the project's issue tracker on GitHub have the answers.".into(),
                row![pill("Guide", Some(Message::OpenLink(GUIDE.into())), Kind::Primary, 120, 38), ok()].spacing(10).into(), 250.0),
            Dialog::Settings => ("Settings", format!("Game folder:\n{}\n\nMore settings come in a later beta version.", self.ctx.settings.game_dir),
                row![pill("Run setup again", Some(Message::ChangeGame), Kind::Primary, 170, 38), ok()].spacing(10).into(), 270.0),
            Dialog::Repair => ("YO3 Revival needs repair", "Some of YO3 Revival's own files are missing or changed, often after an \
                antivirus scan. Download the latest release again and replace this folder with it; your settings are kept.".into(),
                row![pill("Releases", Some(Message::OpenLink(RELEASES.into())), Kind::Primary, 120, 38), ok()].spacing(10).into(), 260.0),
            Dialog::ConfirmClose => ("Close YO3 Revival?", "The game is open. Closing YO3 Revival also closes the game and your server.".into(),
                row![pill("Cancel", Some(Message::CloseDialog), Kind::Primary, 120, 38), pill("Close", Some(Message::Quit), Kind::Primary, 120, 38)].spacing(10).into(), 230.0),
        };
        container(yo3_skin::panel(Some(title), column![
            text(body).size(15).color(color::INK),
            Space::new().height(Fill),
            row![Space::new().width(Fill), buttons],
        ])).width(470).height(height).into()
    }
}

const GUIDE: &str = "https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md";
const RELEASES: &str = "https://github.com/usersina/yo3-revival/releases/latest";

fn update_line(tag: &str) -> String { format!("Update {} is available: see what's new", tag.trim_start_matches('v')) }

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
