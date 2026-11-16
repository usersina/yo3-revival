//! Settings (#31): Game, Accounts, Singleplayer, Advanced and About. Each change is saved at once and read again at
//! the next Play or server start, so nothing needs a restart. Results show as one plain sentence; the technical text
//! is behind Details.
use iced::widget::{Space, checkbox, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Task};
use yo3_desktop::options::{self, Change};
use yo3_desktop::{Settings, direct, updates};
use yo3_skin::{BODY_BOLD, DISPLAY, Kind, color, pill};
use crate::app::{Ctx, dot, job};
use crate::{error_presentation, home, multi};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Section { #[default] Game, Accounts, Singleplayer, Advanced, About }

impl Section {
    pub const ALL: [Section; 5] = [Section::Game, Section::Accounts, Section::Singleplayer, Section::Advanced, Section::About];
    pub fn title(self) -> &'static str {
        match self { Section::Game => "Game", Section::Accounts => "Accounts", Section::Singleplayer => "Singleplayer", Section::Advanced => "Advanced", Section::About => "About" }
    }
}

/// A field typed before it is saved (Linux's Wine prefix and Wine program).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { Prefix, Wine }

/// A screen size the player can choose: the value saved, the name shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Size { pub value: String, pub name: String }
impl std::fmt::Display for Size { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.name) } }

/// The screen sizes offered: the game's own, the screen's, then larger 4:3 windows; a size saved elsewhere is kept.
pub fn sizes(current: &str) -> Vec<Size> {
    let mut list = vec![Size { value: "off".into(), name: "Original (1024 × 768)".into() }, Size { value: "auto".into(), name: "Fit my screen".into() }];
    for (w, h) in [(1280, 960), (1440, 1080), (1600, 1200), (1920, 1440)] { list.push(Size { value: format!("{w}x{h}"), name: format!("{w} × {h}") }) }
    let current = if current.is_empty() { "off" } else { current };
    if !list.iter().any(|s| s.value == current) && let Some((w, h)) = current.split_once('x') {
        list.push(Size { value: current.into(), name: format!("{w} × {h}") });
    }
    list
}

/// What opening the game without the launcher is like now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direct { Unknown, Off, On, NextPlay, Conflict(String) }

#[derive(Clone, Debug)]
pub enum Message {
    Section(Section),
    Change(Change),
    Changed(Change, Result<Box<Settings>, String>),
    /// Handled by the window: the setup wizard, to change the game folder.
    ChangeGame,
    Verify,
    Verified(Result<(), String>),
    DirectChecked(Direct),
    Direct(bool),
    DirectDone(bool, Result<(), String>),
    FixPermission,
    Shortcut,
    ShortcutDone(Result<(), String>),
    /// Handled by the window: Settings → Accounts' messages go to the Multiplayer tab.
    Account(multi::Message),
    BackUp,
    BackedUp(Result<(), String>),
    /// Handled by the window: the backups dialog.
    Restore,
    ResetData(bool),
    DataReset(Result<(), String>),
    OpenData,
    Draft(Field, String),
    Browse(Field),
    Picked(Field, Option<String>),
    Save(Field),
    ExportLogs,
    LogsExported(Result<bool, String>),
    OpenLogs,
    ResetGame(Option<bool>),
    GameReset(Result<(), String>),
    CheckUpdates,
    UpdatesChecked(Result<updates::Decision, String>),
    Licences,
    /// Handled by the window: open a web page or a folder.
    Open(String),
    ToggleDetails,
    Dismiss,
    /// Handled by the window.
    Copy(String),
    /// Handled by the window: Report a problem with this error.
    Report(String),
    /// Handled by the window: back to the main screen.
    Close,
}

/// One plain sentence, good or not, with the technical text behind Details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note { pub good: bool, pub sentence: String, pub details: Option<String>, pub open: bool }

pub struct State {
    pub section: Section,
    note: Option<Note>,
    busy: Option<&'static str>,
    direct: Direct,
    prefix: String,
    wine: String,
    confirm_reset: bool,
    confirm_game_reset: bool,
    licences: bool,
    update: Option<String>,
}

pub const LICENCES: &str = "YO3 Revival is free software under the MIT licence. Its rules engine is built on EDOPro's core and ProjectIgnis' \
card scripts, under the GNU Affero General Public License 3.0; its window uses iced (MIT) and the Barlow fonts (SIL Open Font License). \
Yu-Gi-Oh! ONLINE 3 is Konami's game: YO3 Revival is a fan project, not made or endorsed by Konami, and ships none of its files. \
The full licences come with YO3 Revival, in its notices folder.";

impl State {
    pub fn new(ctx: &Ctx, section: Section) -> Self {
        Self { section, note: None, busy: None, direct: Direct::Unknown, prefix: ctx.settings.wine_prefix.clone(), wine: ctx.settings.wine_binary.clone(),
            confirm_reset: false, confirm_game_reset: false, licences: false, update: None }
    }

    /// The first tasks: what direct start is like now.
    pub fn start(&self, ctx: &Ctx) -> Task<Message> { self.check_direct(ctx) }

    fn check_direct(&self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        let settings = ctx.settings.clone();
        Task::perform(async move {
            tokio::task::spawn_blocking(move || match backend.direct_state(&settings) {
                _ if !settings.direct_start => Direct::Off,
                direct::State::Enabled => Direct::On,
                direct::State::Conflict(why) => Direct::Conflict(why),
                direct::State::Disabled | direct::State::NeedsRepair(_) => Direct::NextPlay,
            }).await.unwrap_or(Direct::Unknown)
        }, Message::DirectChecked)
    }

    fn good(&mut self, sentence: impl Into<String>) { self.note = Some(Note { good: true, sentence: sentence.into(), details: None, open: false }) }

    /// An error as one sentence, the technical text (paths masked) behind Details.
    pub fn fail(&mut self, ctx: &Ctx, error: &str) {
        if let Some(backend) = &ctx.backend { backend.diagnostics.line(error) }
        let shown = error_presentation::present(error, error_presentation::Hint::General, &crate::app::masks(ctx));
        self.note = Some(Note { good: false, sentence: shown.summary, details: Some(shown.diagnosis.unwrap_or_default()).filter(|d| !d.is_empty()), open: false });
    }

    fn result(&mut self, ctx: &Ctx, result: Result<(), String>, done: &str) {
        self.busy = None;
        match result { Ok(()) => self.good(done), Err(error) => self.fail(ctx, &error) }
    }

    pub fn update(&mut self, message: Message, ctx: &mut Ctx, playing: bool) -> Task<Message> {
        match message {
            Message::Section(section) => { self.section = section; self.note = None; self.confirm_reset = false; self.confirm_game_reset = false }
            Message::Change(change) => {
                let Some(store) = ctx.store.clone() else { return Task::none() };
                let asked = change.clone();
                return Task::perform(async move { store.run(move |db| db.change(&asked)).await.map(Box::new).map_err(|e| format!("{e:#}")) },
                    move |result| Message::Changed(change.clone(), result));
            }
            Message::Changed(change, Ok(settings)) => {
                ctx.settings = *settings;
                self.note = None;
                match change {
                    Change::WinePrefix(_) => self.good("Saved. The next PLAY uses this Wine prefix."),
                    Change::WineBinary(_) => self.good("Saved. The next PLAY uses this Wine program."),
                    Change::DirectStart(_) => return self.check_direct(ctx),
                    _ => {}
                }
            }
            Message::Changed(_, Err(error)) => {
                // A refused value is the player's to fix: its sentence is already plain.
                self.note = Some(Note { good: false, sentence: error_presentation::plain(error.lines().next().unwrap_or_default()), details: None, open: false });
            }
            Message::Verify => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                if playing { self.note = Some(Note { good: false, sentence: "Close the game first.".into(), details: None, open: false }); return Task::none() }
                let settings = ctx.settings.clone();
                self.busy = Some("Checking your game and YO3 Revival's files…");
                self.note = None;
                return job(move || backend.verify_setup(&settings, None), Message::Verified);
            }
            Message::Verified(result) => self.result(ctx, result, "Your game and YO3 Revival's files are fine."),
            Message::DirectChecked(direct) => self.direct = direct,
            Message::Direct(on) => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                if playing { self.note = Some(Note { good: false, sentence: "Close the game first.".into(), details: None, open: false }); return Task::none() }
                let settings = ctx.settings.clone();
                self.busy = Some(if on { "Turning it on…" } else { "Taking it out of the game folder…" });
                // On: the next Play sets it up (it needs your server running). Off: taken out of the game folder now.
                return job(move || if on { Ok(()) } else { backend.disable_direct(&settings) }, move |result| Message::DirectDone(on, result));
            }
            Message::DirectDone(on, result) => {
                self.busy = None;
                match result {
                    Ok(()) => return self.update(Message::Change(Change::DirectStart(on)), ctx, playing),
                    Err(error) => self.fail(ctx, &error),
                }
            }
            Message::FixPermission => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.busy = Some("Asking for permission…");
                // Direct start points at your server: it runs for the setup and stops after unless it was already up.
                return job(move || {
                    let was_live = backend.live(&settings).is_some();
                    backend.start_server(&settings)?;
                    let done = backend.enable_direct(&settings, None);
                    if !was_live { let _ = backend.stop_server(&settings); }
                    done
                }, |result| Message::DirectDone(true, result));
            }
            Message::Shortcut => {
                self.busy = Some("Adding the shortcut…");
                return job(|| yo3_desktop::shortcut::create(&std::env::current_exe()?).map(|_| ()), Message::ShortcutDone);
            }
            Message::ShortcutDone(result) => self.result(ctx, result, "The shortcut is on your desktop."),
            Message::BackUp => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.busy = Some("Backing up…");
                return job(move || backend.back_up_now(&settings), Message::BackedUp);
            }
            Message::BackedUp(result) => self.result(ctx, result, "Backed up. Restore lists it with the others."),
            Message::ResetData(false) => self.confirm_reset = !self.confirm_reset,
            Message::ResetData(true) => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                if playing { self.note = Some(Note { good: false, sentence: "Close the game first.".into(), details: None, open: false }); return Task::none() }
                let settings = ctx.settings.clone();
                self.confirm_reset = false;
                self.busy = Some("Resetting…");
                return job(move || {
                    if backend.live(&settings).is_some() { backend.stop_server(&settings)?; }
                    backend.reset_singleplayer(&settings).map(|_| ())
                }, Message::DataReset);
            }
            Message::DataReset(result) => self.result(ctx, result, "Your singleplayer data is reset. The old data is kept beside the new, renamed."),
            Message::OpenData => if let Some(backend) = &ctx.backend {
                let dir = backend.data_dir(&ctx.settings);
                let _ = std::fs::create_dir_all(&dir);
                return Task::done(Message::Open(dir.to_string_lossy().into_owned()));
            },
            Message::Draft(field, value) => match field { Field::Prefix => self.prefix = value, Field::Wine => self.wine = value },
            Message::Browse(field) => {
                return Task::perform(async move {
                    let dialog = rfd::AsyncFileDialog::new();
                    let picked = match field { Field::Prefix => dialog.set_title("Choose the Wine prefix").pick_folder().await, Field::Wine => dialog.set_title("Choose the Wine program").pick_file().await };
                    picked.map(|f| f.path().to_string_lossy().into_owned())
                }, move |path| Message::Picked(field, path));
            }
            Message::Picked(field, Some(path)) => { let _ = self.update(Message::Draft(field, path), ctx, playing); return Task::done(Message::Save(field)) }
            Message::Picked(_, None) => {}
            Message::Save(field) => return Task::done(Message::Change(match field { Field::Prefix => Change::WinePrefix(self.prefix.clone()), Field::Wine => Change::WineBinary(self.wine.clone()) })),
            Message::ExportLogs => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let name = format!("yo3-revival-logs-{}.zip", chrono::Local::now().format("%Y%m%d-%H%M%S"));
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().set_title("Save the logs").set_file_name(&name).add_filter("ZIP", &["zip"]).save_file().await else { return Ok(false) };
                    let path = file.path().to_path_buf();
                    tokio::task::spawn_blocking(move || backend.report(true)?.export_with_privacy(&path, true)).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:#}"))?;
                    Ok(true)
                }, Message::LogsExported);
            }
            Message::LogsExported(Ok(true)) => self.good("The logs are saved, with personal details removed."),
            Message::LogsExported(Ok(false)) => {}
            Message::LogsExported(Err(error)) => self.fail(ctx, &error),
            Message::OpenLogs => return Task::done(Message::Open(ctx.root.join("logs").to_string_lossy().into_owned())),
            Message::ResetGame(None) => self.confirm_game_reset = !self.confirm_game_reset,
            Message::ResetGame(Some(full)) => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.confirm_game_reset = false;
                self.busy = Some("Resetting the game's local data…");
                return job(move || { let preview = backend.preview_game_data(&settings)?; backend.reset_game_data(&settings, &preview, full).map(|_| ()) }, Message::GameReset);
            }
            Message::GameReset(result) => self.result(ctx, result, "Done. The old data is kept in a backup folder beside it."),
            Message::CheckUpdates => {
                self.busy = Some("Checking for updates…");
                return Task::perform(async { updates::check().await.map_err(|e| format!("{e:#}")) }, Message::UpdatesChecked);
            }
            Message::UpdatesChecked(result) => {
                self.busy = None;
                match result {
                    Ok(updates::Decision::Available(release)) => { self.update = Some(release.url.clone()); self.good(format!("Version {} is available.", release.tag.trim_start_matches('v'))) }
                    Ok(updates::Decision::Current { .. }) => self.good("You have the latest version."),
                    Ok(updates::Decision::Development { latest } | updates::Decision::Manual { latest }) => self.good(format!("The latest release is {}.", latest.trim_start_matches('v'))),
                    Ok(updates::Decision::NoReleases) => self.good("There is no release yet."),
                    Err(error) => self.fail(ctx, &format!("Couldn't check for updates.\n{error}")), // → Details
                }
            }
            Message::Licences => self.licences = !self.licences,
            Message::ToggleDetails => if let Some(note) = &mut self.note { note.open = !note.open },
            Message::Dismiss => self.note = None,
            Message::ChangeGame | Message::Account(_) | Message::Restore | Message::Open(_) | Message::Copy(_) | Message::Report(_) | Message::Close => {}
        }
        Task::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx, home: &'a home::State) -> Element<'a, Message> {
        let playing = home.playing();
        let mut tabs = column![].spacing(6);
        for section in Section::ALL {
            tabs = tabs.push(pill(section.title(), (section != self.section).then_some(Message::Section(section)), Kind::Tab { active: section == self.section }, Fill, 36));
        }
        tabs = tabs.push(Space::new().height(Fill)).push(pill("Back", Some(Message::Close), Kind::Primary, Fill, 38));
        let body = match self.section {
            Section::Game => self.game(ctx, playing, home.direct_permission),
            Section::Accounts => home.multi.accounts_view().map(Message::Account),
            Section::Singleplayer => self.singleplayer(ctx, home, playing),
            Section::Advanced => self.advanced(ctx),
            Section::About => self.about(),
        };
        // What just happened comes first, where the eye is.
        let mut content = column![].spacing(16);
        if let Some(busy) = self.busy { content = content.push(text(busy).size(14).color(color::INK_SOFT)) }
        if let Some(note) = &self.note { content = content.push(self.note(note)) }
        let content = content.push(body);
        let panel = yo3_skin::panel(Some(self.section.title()), scrollable(content.padding([0, 14])).height(Fill));
        row![container(tabs).width(200).height(Fill), container(panel).width(Fill).height(Fill)].spacing(16).padding([16, 18]).height(Fill).into()
    }

    fn note<'a>(&'a self, note: &'a Note) -> Element<'a, Message> {
        let shade = if note.good { color::GOOD } else { color::BAD };
        let mut links = row![].spacing(14);
        if !note.good { links = links.push(yo3_skin::link("Report", Message::Report(note.details.clone().unwrap_or_else(|| note.sentence.clone())), false)) }
        if note.details.is_some() { links = links.push(yo3_skin::link(if note.open { "Hide details" } else { "Details" }, Message::ToggleDetails, false)) }
        links = links.push(yo3_skin::link("Dismiss", Message::Dismiss, false));
        let mut col = column![row![container(Space::new().width(9).height(9)).style(move |_| dot(shade)), text(&note.sentence).size(14).font(BODY_BOLD).color(color::INK).width(Fill)]
            .spacing(8).align_y(Center)].spacing(6);
        if note.open && let Some(details) = &note.details {
            col = col.push(text(details).size(13).color(color::INK_SOFT)).push(yo3_skin::link("Copy", Message::Copy(details.clone()), false));
        }
        col.push(links).into()
    }

    fn game<'a>(&'a self, ctx: &'a Ctx, playing: bool, needs_permission: bool) -> Element<'a, Message> {
        let folder = std::path::Path::new(&ctx.settings.game_dir).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Not chosen".into());
        let found = std::path::Path::new(&ctx.settings.game_dir).join("yo3.exe").is_file();
        let size = sizes(&ctx.settings.resolution);
        let chosen = size.iter().find(|s| s.value == if ctx.settings.resolution.is_empty() { "off" } else { ctx.settings.resolution.as_str() }).cloned();
        let direct_line = match &self.direct {
            Direct::Unknown => "Checking…".to_owned(),
            Direct::Off => "Off: the game opens only from YO3 Revival.".to_owned(),
            Direct::On => "On: opening the game from its own shortcut signs you in to Singleplayer.".to_owned(),
            Direct::NextPlay => "On: the next PLAY sets it up.".to_owned(),
            Direct::Conflict(_) => "Another program's file is where it would go, so it stays off. Play from YO3 Revival.".to_owned(),
        };
        let mut direct = column![
            checkbox(ctx.settings.direct_start).label("Open the game without the launcher").on_toggle_maybe((!playing && self.busy.is_none()).then_some(Message::Direct))
                .style(yo3_skin::tick).size(16).text_size(15),
            text(direct_line).size(14).color(color::INK_SOFT),
        ].spacing(6);
        if needs_permission && ctx.settings.direct_start {
            direct = direct.push(row![text("The game folder is protected: setting it up needs permission once.").size(14).color(color::INK).width(Fill),
                pill("Fix permission", (self.busy.is_none() && !playing).then_some(Message::FixPermission), Kind::Primary, 150, 32)].spacing(10).align_y(Center));
        }
        column![
            item("Game folder", column![
                row![text(format!("{folder}{}", if found { "" } else { " (we can't find the game there)" })).size(15).color(color::INK).width(Fill),
                    pill("Change…", (!playing).then_some(Message::ChangeGame), Kind::Primary, 120, 32)].spacing(10).align_y(Center),
            ].into()),
            item("Verify and repair", row![
                text("Checks the game and YO3 Revival's own files, and says how to fix what isn't right.").size(14).color(color::INK_SOFT).width(Fill),
                pill("Verify", (self.busy.is_none() && !playing).then_some(Message::Verify), Kind::Primary, 120, 32)].spacing(10).align_y(Center).into()),
            item("Screen size", column![
                pick_list(size, chosen, |s: Size| Message::Change(Change::Resolution(s.value))).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(280).padding([6, 10]),
                text("The game keeps its picture and draws it larger. From the next PLAY.").size(14).color(color::INK_SOFT),
            ].spacing(6).into()),
            item("Without the launcher", direct.into()),
            item("Desktop shortcut", row![text("A shortcut to YO3 Revival on your desktop.").size(14).color(color::INK_SOFT).width(Fill),
                pill("Add", self.busy.is_none().then_some(Message::Shortcut), Kind::Primary, 120, 32)].spacing(10).align_y(Center).into()),
        ].spacing(16).into()
    }

    fn singleplayer<'a>(&'a self, ctx: &'a Ctx, home: &'a home::State, playing: bool) -> Element<'a, Message> {
        let choices = home.banlists();
        let selected = choices.iter().find(|c| c.value == ctx.settings.banlist).cloned();
        let banlist: Element<'_, Message> = if playing {
            text(format!("{} (it can change while the game is closed)", selected.map_or_else(|| "Season 16 (default)".to_owned(), |b| b.name))).size(15).color(color::INK).into()
        } else {
            pick_list(choices, selected, |b: home::Banlist| Message::Change(Change::Banlist(b.value))).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu)
                .width(280).padding([6, 10]).placeholder("Season 16 (default)").into()
        };
        let mut reset = row![text("Starts your singleplayer collection, decks and duel record again.").size(14).color(color::INK_SOFT).width(Fill),
            pill("Reset…", (self.busy.is_none() && !playing).then_some(Message::ResetData(false)), Kind::Primary, 120, 32)].spacing(10).align_y(Center);
        if self.confirm_reset {
            reset = row![text("Reset your singleplayer data? The old data is kept beside it.").size(14).font(BODY_BOLD).color(color::BAD).width(Fill),
                pill("Reset", Some(Message::ResetData(true)), Kind::Primary, 100, 32), yo3_skin::link("Cancel", Message::ResetData(false), false)].spacing(10).align_y(Center);
        }
        column![
            item("Banlist", column![banlist, text("The banlist your own server checks decks against. From the next PLAY.").size(14).color(color::INK_SOFT)].spacing(6).into()),
            item("Backups", column![
                text("Your server keeps a backup from each of its last starts.").size(14).color(color::INK_SOFT),
                row![pill("Back up now", self.busy.is_none().then_some(Message::BackUp), Kind::Primary, 150, 32),
                    pill("Restore…", (!playing).then_some(Message::Restore), Kind::Primary, 130, 32),
                    pill("Open data folder", Some(Message::OpenData), Kind::Primary, 170, 32)].spacing(10),
            ].spacing(8).into()),
            item("Reset", reset.into()),
        ].spacing(16).into()
    }

    fn advanced<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Message> {
        let mut col = column![].spacing(16);
        if cfg!(not(windows)) {
            let field = |label: &'static str, value: &'a str, which: Field, hint: &'static str| column![
                text(label).size(15).font(BODY_BOLD).color(color::INK),
                row![text_input(hint, value).on_input(move |v| Message::Draft(which, v)).on_submit(Message::Save(which)).style(yo3_skin::field).padding(7).size(15),
                    pill("Browse…", Some(Message::Browse(which)), Kind::Primary, 110, 32), pill("Save", Some(Message::Save(which)), Kind::Primary, 90, 32)].spacing(8).align_y(Center),
            ].spacing(6);
            col = col.push(item("Wine", column![
                field("Wine prefix", &self.prefix, Field::Prefix, "Wine's default prefix"),
                field("Wine program", &self.wine, Field::Wine, "wine, from your system"),
                text("Leave them empty for Wine's defaults. From the next PLAY.").size(14).color(color::INK_SOFT),
            ].spacing(10).into()));
        }
        col.push(item("Logs", column![
            checkbox(options::detailed(&_ctx.settings)).label("Detailed logs").on_toggle(|on| Message::Change(Change::DetailedLogs(on))).style(yo3_skin::tick).size(16).text_size(15),
            text("Your servers write more about what they do, for a problem report. From their next start.").size(14).color(color::INK_SOFT),
            row![pill("Export logs…", Some(Message::ExportLogs), Kind::Primary, 150, 32), pill("Open logs folder", Some(Message::OpenLogs), Kind::Primary, 170, 32)].spacing(10),
        ].spacing(8).into()))
        .push(item("Reset local game data", if self.confirm_game_reset {
            column![
                text("Which data? The old data is kept in a backup folder either way.").size(14).font(BODY_BOLD).color(color::INK),
                row![pill("Display and login settings", self.busy.is_none().then_some(Message::ResetGame(Some(false))), Kind::Primary, 240, 32),
                    pill("All local game data", self.busy.is_none().then_some(Message::ResetGame(Some(true))), Kind::Primary, 190, 32),
                    yo3_skin::link("Cancel", Message::ResetGame(None), false)].spacing(10).align_y(Center),
            ].spacing(8).into()
        } else {
            row![text("When the game's screen or saved login settings are stuck, the game can start from fresh ones.").size(14).color(color::INK_SOFT).width(Fill),
                pill("Reset…", Some(Message::ResetGame(None)), Kind::Primary, 120, 32)].spacing(10).align_y(Center).into()
        }))
        .into()
    }

    fn about(&self) -> Element<'_, Message> {
        let mut updates = row![pill("Check for updates", self.busy.is_none().then_some(Message::CheckUpdates), Kind::Primary, 190, 32)].spacing(14).align_y(Center);
        if let Some(url) = &self.update { updates = updates.push(yo3_skin::link("See what's new", Message::Open(url.clone()), false)) }
        let mut col = column![
            text(format!("YO3 Revival {} (beta)", yo3_tunnel::BUILD_VERSION.trim_start_matches('v'))).font(DISPLAY).size(22).color(color::NAVY),
            text("A private server and launcher to play Yu-Gi-Oh! ONLINE 3 again, with your own copy of the game.").size(15).color(color::INK),
            updates,
            row![yo3_skin::link("Release notes", Message::Open(updates::RELEASES.into()), false), yo3_skin::link("The project", Message::Open(updates::REPOSITORY.into()), false),
                yo3_skin::link("Licences and credits", Message::Licences, false)].spacing(18),
        ].spacing(12);
        if self.licences { col = col.push(text(LICENCES).size(14).color(color::INK)) }
        col.into()
    }
}

/// A setting's heading and its controls.
fn item<'a>(title: &'static str, body: Element<'a, Message>) -> Element<'a, Message> {
    column![text(title).font(DISPLAY).size(17).color(color::NAVY), body].spacing(6).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx { Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() } }

    /// Each setting the screen saves (the database round trip is `options`' tests): the window's copy changes at once,
    /// so the next Play reads it without a restart.
    #[test]
    fn a_saved_setting_is_the_one_the_next_play_reads() {
        let mut ctx = ctx();
        let mut state = State::new(&ctx, Section::Game);
        for change in [Change::Resolution("1440x1080".into()), Change::DirectStart(false), Change::Banlist("4".into()), Change::WinePrefix("/p".into()),
            Change::WineBinary("/w".into()), Change::DetailedLogs(true)] {
            let mut expected = ctx.settings.clone();
            match &change { Change::WinePrefix(v) => expected.wine_prefix = v.clone(), Change::WineBinary(v) => expected.wine_binary = v.clone(), other => other.apply(&mut expected).unwrap() }
            let _ = state.update(Message::Changed(change.clone(), Ok(Box::new(expected.clone()))), &mut ctx, false);
            assert_eq!(ctx.settings, expected, "{change:?}");
        }
        assert_eq!((ctx.settings.resolution.as_str(), ctx.settings.direct_start, ctx.settings.banlist.as_str(), options::detailed(&ctx.settings)), ("1440x1080", false, "4", true));
        let _ = state.update(Message::Changed(Change::Resolution("huge".into()), Err("Choose one of the screen sizes.".into())), &mut ctx, false);
        assert_eq!(ctx.settings.resolution, "1440x1080", "a refused value changes nothing");
        assert_eq!(state.note.as_ref().map(|n| (n.good, n.sentence.as_str())), Some((false, "Choose one of the screen sizes.")));
    }

    #[test]
    fn the_screen_sizes_start_with_the_games_own_and_keep_a_custom_one() {
        let list = sizes("");
        assert_eq!((list[0].value.as_str(), list[1].value.as_str()), ("off", "auto"));
        assert!(sizes("1366x1024").iter().any(|s| s.value == "1366x1024" && s.name == "1366 × 1024"));
        assert_eq!(sizes("1440x1080").len(), list.len(), "a listed size isn't added twice");
    }

    #[test]
    fn an_error_is_one_sentence_with_the_technical_text_behind_details() {
        let ctx = ctx();
        let mut state = State::new(&ctx, Section::Game);
        state.fail(&ctx, "The rules engine could not start: yo3-engine: missing cards.cdb\nEngine: /opt/yo3/yo3-engine\nExit status: 1");
        let note = state.note.clone().unwrap();
        assert_eq!(note.sentence, "The rules engine couldn't start: YO3 Revival needs repair.");
        assert!(note.details.unwrap().contains("cards.cdb"));
        assert!(yo3_desktop::words::technical(&note.sentence).is_none());
    }
}
