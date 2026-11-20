//! Settings (#31): Game, Accounts, Singleplayer, Advanced and About. Each change is saved at once and read again at
//! the next Play or server start, so nothing needs a restart. Results show as one plain sentence; the technical text
//! is behind Details.
use iced::widget::{Space, checkbox, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Task};
use yo3_desktop::options::{self, Change};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use yo3_desktop::{Settings, direct, pictures, updates};
use yo3_skin::{BODY_BOLD, Bar, DISPLAY, Kind, color, pill};
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

/// What the Settings line says about opening the game without the launcher, from the saved choice and what the game folder holds.
pub fn direct_of(wanted: bool, state: direct::State) -> Direct {
    match state {
        _ if !wanted => Direct::Off,
        direct::State::Enabled => Direct::On,
        direct::State::Conflict(why) => Direct::Conflict(why),
        direct::State::NeedsRepair(_) => Direct::Repair,
        direct::State::Disabled => Direct::NextPlay,
    }
}

/// What opening the game without the launcher is like now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direct {
    Unknown, Off, On, NextPlay,
    /// YO3 Revival's own loader is in the game folder but needs its files put right (#65).
    Repair,
    Conflict(String),
}

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
    /// Fix permission's result (the window also tells the main screen).
    PermissionFixed(Result<(), String>),
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
    /// Put a file moved aside from the game folder back (#68).
    PutBack(String),
    PutBackDone(Result<Vec<String>, String>),
    /// The anime pictures (#60): how many the folder has, Get pictures, its progress and end, Cancel, Add pictures from a file, Save pictures to a file.
    PicturesCounted((usize, usize)),
    GetPictures,
    Pictures(pictures::Event),
    CancelPictures,
    AddPictures,
    PicturesAdded(Result<Option<yo3_launcher::art::pack::Imported>, String>),
    SavePictures,
    PicturesSaved(Result<Option<yo3_launcher::art::pack::Exported>, String>),
    /// The Anime cards switch (#60): the server's value, the player's click, and the server's answer.
    AnimeRead(Result<bool, String>),
    AnimeCards(bool),
    AnimeSet(bool, Result<bool, String>),
    /// Close the pictures dialog (during a download: stops it too).
    PicturesDialogClose,
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

/// The dialog the Anime cards switch opens when pictures are missing: the offer, then (once Get pictures is pressed) the
/// download on the shared bar, then its result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dialog { Offer, Result { good: bool, sentence: String } }

/// How many cards still have no picture, when there are any (the switch offers the download only then).
/// How many cards still lack a picture, as a sentence.
fn missing_words(have: usize, total: usize) -> String {
    match total.saturating_sub(have) {
        0 => "Every card has a picture.".to_owned(),
        missing if missing >= total => format!("None of the {total} cards has a picture yet."),
        1 => format!("1 of the {total} cards has no picture yet."),
        missing => format!("{missing} of the {total} cards have no picture yet."),
    }
}

pub fn offer(pictures: Option<(usize, usize)>) -> Option<usize> {
    pictures.map(|(have, total)| total.saturating_sub(have)).filter(|&missing| missing > 0)
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
    /// The desktop shortcut is already there: its row offers to make it again (after an update moved the launcher).
    shortcut: bool,
    /// The other programs' files YO3 Revival moved aside in the game folder (#68), by their original names.
    moved: Vec<String>,
    /// Anime pictures the folder has, of the cards that have one to get (`None` until counted).
    pictures: Option<(usize, usize)>,
    /// A download in progress: cards done, all cards, the way to cancel it.
    getting: Option<(u32, u32, Arc<AtomicBool>)>,
    /// What the last download left that the bar must keep saying: its words, when it stopped on a problem.
    art_problem: Option<String>,
    /// Adding or saving a zip: the wait is shown on the pictures' own bar.
    art_busy: Option<&'static str>,
    /// Your own server's anime-cards value, as the server says (`None` until read, or when it can't be).
    anime: Option<bool>,
    /// The pictures dialog, opened by turning Anime cards on while pictures are missing.
    dialog: Option<Dialog>,
    /// The switch went on before the pictures were counted: offer them when the count is in.
    offer_when_counted: bool,
}

pub const LICENCES: &str = "YO3 Revival is free software under the MIT licence. Its rules engine is built on EDOPro's core and ProjectIgnis' \
card scripts, under the GNU Affero General Public License 3.0; its window uses iced (MIT) and the Barlow fonts (SIL Open Font License). \
Yu-Gi-Oh! ONLINE 3 is Konami's game: YO3 Revival is a fan project, not made or endorsed by Konami, and ships none of its files. \
The full licences come with YO3 Revival, in its notices folder.";

impl State {
    pub fn new(ctx: &Ctx, section: Section) -> Self {
        Self { section, note: None, busy: None, direct: Direct::Unknown, prefix: ctx.settings.wine_prefix.clone(), wine: ctx.settings.wine_binary.clone(),
            confirm_reset: false, confirm_game_reset: false, licences: false, update: None, shortcut: yo3_desktop::shortcut::exists(),
            moved: yo3_desktop::foreign::moved_aside(std::path::Path::new(&ctx.settings.game_dir)), pictures: None, getting: None, art_problem: None, art_busy: None, anime: None, dialog: None, offer_when_counted: false }
    }

    /// Whether Settings' own dialog is open: Escape closes it before it leaves Settings.
    pub fn has_dialog(&self) -> bool { self.dialog.is_some() }

    /// The first tasks: what direct start is like now, and how many anime pictures there are.
    pub fn start(&self, ctx: &Ctx) -> Task<Message> { Task::batch([self.check_direct(ctx), self.count_pictures(ctx), self.read_anime(ctx)]) }

    fn read_anime(&self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        let settings = ctx.settings.clone();
        Task::perform(async move { tokio::task::spawn_blocking(move || backend.anime_cards(&settings).map_err(|e| format!("{e:#}"))).await.unwrap_or_else(|e| Err(e.to_string())) }, Message::AnimeRead)
    }

    fn count_pictures(&self, ctx: &Ctx) -> Task<Message> {
        let dir = yo3_launcher::art::folder_in(&ctx.root);
        Task::perform(async move { tokio::task::spawn_blocking(move || pictures::count(&dir)).await.unwrap_or((0, 0)) }, Message::PicturesCounted)
    }

    fn check_direct(&self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        let settings = ctx.settings.clone();
        Task::perform(async move {
            tokio::task::spawn_blocking(move || direct_of(settings.direct_start, backend.direct_state(&settings))).await.unwrap_or(Direct::Unknown)
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
                return job(move || backend.verify_setup(&settings), Message::Verified);
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
                    let done = backend.enable_direct(&settings);
                    if !was_live { let _ = backend.stop_server(&settings); }
                    done
                }, Message::PermissionFixed);
            }
            Message::PermissionFixed(result) => return self.update(Message::DirectDone(true, result), ctx, playing),
            Message::Shortcut => {
                self.busy = Some("Adding the shortcut…");
                return job(|| yo3_desktop::shortcut::create(&std::env::current_exe()?).map(|_| ()), Message::ShortcutDone);
            }
            Message::ShortcutDone(result) => {
                if result.is_ok() { self.shortcut = true }
                self.result(ctx, result, "The shortcut is on your desktop.")
            }
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
            Message::PicturesCounted(counts) => {
                self.pictures = Some(counts);
                if std::mem::take(&mut self.offer_when_counted) && offer(self.pictures).is_some() && self.anime == Some(true) { self.dialog = Some(Dialog::Offer) }
            }
            Message::AnimeRead(Ok(on)) => self.anime = Some(on),
            Message::AnimeRead(Err(_)) => self.anime = None,
            Message::AnimeCards(on) => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.busy = Some(if on { "Turning anime cards on…" } else { "Turning anime cards off…" });
                return job(move || backend.set_anime_cards(&settings, on), move |result| Message::AnimeSet(on, result));
            }
            Message::AnimeSet(on, Ok(now)) => {
                self.busy = None;
                self.anime = Some(now);
                if now != on { self.fail(ctx, "The server didn't take the change."); return Task::none() }
                self.good(match (on, playing) {
                    (true, true) => "Anime cards are on. The open game needs a restart to show them: use RESTART THE GAME.",
                    (true, false) => "Anime cards are on. They show from the next game start.",
                    (false, true) => "Anime cards are off. The open game needs a restart to show the game's own cards: use RESTART THE GAME.",
                    (false, false) => "Anime cards are off from the next game start.",
                });
                if on {
                    match (offer(self.pictures), self.pictures) {
                        (Some(_), _) => self.dialog = Some(Dialog::Offer),
                        (None, None) => self.offer_when_counted = true,
                        (None, Some(_)) => {}
                    }
                }
            }
            Message::AnimeSet(_, Err(error)) => { self.busy = None; self.fail(ctx, &error) }
            Message::PicturesDialogClose => {
                if let Some((_, _, cancel)) = &self.getting { pictures::stop(cancel) }
                self.dialog = None;
            }
            Message::GetPictures => {
                if self.getting.is_some() { return Task::none() }
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                let cancel = Arc::new(AtomicBool::new(false));
                let dir = yo3_launcher::art::folder_in(&ctx.root);
                self.getting = Some((0, pictures::count(&dir).1 as u32, cancel.clone()));
                self.note = None;
                self.art_problem = None;
                pictures::start(dir, pictures::SERVER.into(), cancel, tx);
                return Task::run(iced::futures::stream::unfold(Some(rx), |rx| async move {
                    let mut rx = rx?;
                    let event = rx.recv().await?;
                    let next = (!matches!(event, pictures::Event::Done(_))).then_some(rx);
                    Some((event, next))
                }), Message::Pictures);
            }
            Message::Pictures(pictures::Event::Progress(done, total)) => if let Some(run) = &mut self.getting { run.0 = done; run.1 = total },
            Message::Pictures(pictures::Event::Done(result)) => {
                self.getting = None;
                self.art_problem = None;
                match result {
                    Ok(out) => {
                        let (good, sentence) = pictures::download_sentence(&out);
                        if !good { self.art_problem = Some(if out.unreachable.is_some() { "The picture server can't be reached".to_owned() } else { format!("{} not fetched", out.failed.len()) }) }
                        if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("Anime pictures: {} saved, {} had, {} not fetched{}", out.saved.len(), out.had, out.failed.len(),
                            out.unreachable.as_deref().map(|why| format!(" (server not reached: {why})")).unwrap_or_default())) }
                        let mut sentence = sentence;
                        if good || !out.saved.is_empty() { sentence.push_str(if playing { " The open game shows them after you close it and press PLAY again." } else { " They show from the next PLAY." }) }
                        if self.dialog.is_some() { self.dialog = Some(Dialog::Result { good, sentence }) } else { self.note = Some(Note { good, sentence, details: None, open: false }) }
                    }
                    Err(error) => if self.dialog.is_some() { self.dialog = Some(Dialog::Result { good: false, sentence: "The pictures couldn't be saved. Try Get pictures again in Settings.".into() }) } else { self.fail(ctx, &error) },
                }
                return self.count_pictures(ctx);
            }
            Message::CancelPictures => if let Some((_, _, cancel)) = &self.getting { pictures::stop(cancel) },
            Message::AddPictures => {
                let dir = yo3_launcher::art::folder_in(&ctx.root);
                self.busy = Some("Adding the pictures…");
                self.art_busy = self.busy;
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().set_title("Add pictures from a file").add_filter("ZIP", &["zip"]).pick_file().await else { return Ok(None) };
                    let path = file.path().to_path_buf();
                    tokio::task::spawn_blocking(move || yo3_launcher::art::pack::import(&dir, &path)).await.map_err(|e| e.to_string())?.map(Some)
                }, Message::PicturesAdded);
            }
            Message::PicturesAdded(result) => {
                self.busy = None;
                self.art_busy = None;
                match result {
                    Ok(Some(done)) => {
                        let (good, mut sentence) = pictures::import_sentence(&done);
                        if done.faces + done.minis > 0 { sentence.push_str(if playing { " The open game shows them after you close it and press PLAY again." } else { " They show from the next PLAY." }) }
                        self.note = Some(Note { good, sentence, details: None, open: false });
                        return self.count_pictures(ctx);
                    }
                    Ok(None) => {}
                    Err(error) => self.fail(ctx, &error),
                }
            }
            Message::SavePictures => {
                let dir = yo3_launcher::art::folder_in(&ctx.root);
                self.busy = Some("Saving the pictures…");
                self.art_busy = self.busy;
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().set_title("Save pictures to a file").set_file_name("yo3-anime-pictures.zip").add_filter("ZIP", &["zip"]).save_file().await else { return Ok(None) };
                    let path = file.path().to_path_buf();
                    tokio::task::spawn_blocking(move || yo3_launcher::art::pack::export(&dir, &path)).await.map_err(|e| e.to_string())?.map(Some)
                }, Message::PicturesSaved);
            }
            Message::PicturesSaved(result) => {
                self.busy = None;
                self.art_busy = None;
                match result { Ok(Some(done)) => self.good(pictures::export_sentence(&done)), Ok(None) => {}, Err(error) => self.fail(ctx, &error) }
            }
            Message::Licences => self.licences = !self.licences,
            Message::PutBack(name) => {
                if playing { return Task::none() }
                let game = std::path::PathBuf::from(&ctx.settings.game_dir);
                return job(move || { yo3_desktop::foreign::put_back_allowed(&game, &name)?; Ok(yo3_desktop::foreign::moved_aside(&game)) }, Message::PutBackDone);
            }
            Message::PutBackDone(Ok(left)) => { self.moved = left; self.good("Put back: the file is in your game folder again, as before.") }
            Message::PutBackDone(Err(error)) => if error.contains("declined") { self.note = Some(Note { good: false, sentence: yo3_desktop::foreign::DECLINED_PUT_BACK.into(), details: None, open: false }) } else { self.fail(ctx, &error) },
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
        tabs = tabs.push(Space::new().height(Fill)).push(pill("Back", Some(Message::Close), Kind::Secondary, Fill, 38));
        let body = match self.section {
            Section::Game => self.game(ctx, playing, home.direct_permission),
            Section::Accounts => home.multi.accounts_view().map(Message::Account),
            Section::Singleplayer => self.singleplayer(ctx, home, playing),
            Section::Advanced => self.advanced(ctx),
            Section::About => self.about(),
        };
        // What just happened comes first, where the eye is.
        let mut content = column![].spacing(16);
        if let Some(busy) = self.busy.filter(|_| self.art_busy.is_none()) { content = content.push(yo3_skin::progress_bar(Bar::Starting, busy, 22.0)) }
        if let Some(note) = &self.note { content = content.push(self.note(note)) }
        let content = content.push(body);
        let panel = yo3_skin::panel(Some(self.section.title()), scrollable(content.padding([0, 14])).height(Fill));
        let screen: Element<'_, Message> = row![container(tabs).width(200).height(Fill), container(panel).width(Fill).height(Fill)].spacing(16).padding([16, 18]).height(Fill).into();
        match &self.dialog {
            Some(dialog) => yo3_skin::modal(screen, self.pictures_dialog(dialog), Message::PicturesDialogClose),
            None => screen,
        }
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
            Direct::Repair => "On: YO3 Revival's own file in the game folder needs repair. The next PLAY puts it right.".to_owned(),
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
                    pill("Change…", (!playing).then_some(Message::ChangeGame), Kind::Secondary, 120, 32)].spacing(10).align_y(Center),
            ].into()),
            item("Verify and repair", row![
                text("Checks the game and YO3 Revival's own files, and says how to fix what isn't right.").size(14).color(color::INK_SOFT).width(Fill),
                pill("Verify", (self.busy.is_none() && !playing).then_some(Message::Verify), Kind::Secondary, 120, 32)].spacing(10).align_y(Center).into()),
            item("Screen size", column![
                pick_list(size, chosen, |s: Size| Message::Change(Change::Resolution(s.value))).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(280).padding([6, 10]),
                text("The game keeps its picture and draws it larger. From the next PLAY.").size(14).color(color::INK_SOFT),
            ].spacing(6).into()),
            item("Anime pictures", self.pictures_view(playing)),
            item("Without the launcher", direct.into()),
            item("Desktop shortcut", row![text(if self.shortcut { "On your desktop. Make it again if it no longer opens YO3 Revival." }
                    else { "A shortcut to YO3 Revival on your desktop." }).size(14).color(color::INK_SOFT).width(Fill),
                pill(if self.shortcut { "Make again" } else { "Add" }, self.busy.is_none().then_some(Message::Shortcut), Kind::Secondary, 120, 32)]
                .spacing(10).align_y(Center).into()),
        ].spacing(16).into()
    }

    /// The dialog of the Anime cards switch: what is about to be downloaded, the download, its result.
    fn pictures_dialog<'a>(&'a self, dialog: &'a Dialog) -> Element<'a, Message> {
        let close = Message::PicturesDialogClose;
        let (have, total) = self.pictures.unwrap_or((0, 0));
        let body: Element<'_, Message> = match (&self.getting, dialog) {
            (Some(_), _) => {
                let (bar, words) = self.pictures_bar();
                column![text("Getting the pictures. You can cancel at any time: the cards that have one keep it.").size(15).color(color::INK),
                    yo3_skin::progress_bar(bar, words, 24.0), Space::new().height(Fill),
                    row![Space::new().width(Fill), pill("Cancel", Some(Message::CancelPictures), Kind::Secondary, 130, 38)]].spacing(14).into()
            }
            (None, Dialog::Offer) => column![
                text("Anime cards are on. To see their anime artwork, YO3 Revival can get the pictures now.").size(15).color(color::INK),
                text(format!("The download is {} from ProjectIgnis' picture server, one card at a time. {} A card without a picture keeps the game's artwork. \
                    Nothing is downloaded unless you press Get pictures.", pictures::SIZE_NOTE, missing_words(have, total))).size(14).color(color::INK_SOFT),
                Space::new().height(Fill),
                row![Space::new().width(Fill), pill("Not now", Some(close.clone()), Kind::Secondary, 130, 38), pill("Get pictures", Some(Message::GetPictures), Kind::Primary, 160, 38)].spacing(10)].spacing(14).into(),
            (None, Dialog::Result { good, sentence }) => column![
                yo3_skin::rule(if *good { yo3_skin::Rule::Met } else { yo3_skin::Rule::Broken }, sentence.as_str()),
                Space::new().height(Fill),
                row![Space::new().width(Fill), pill("Close", Some(close.clone()), Kind::Primary, 130, 38)]].spacing(14).into(),
        };
        yo3_skin::dialog("Anime pictures", None, close, 520.0, 250.0, body)
    }

    /// The Anime cards switch of Settings → Singleplayer: your own server's value, which the server owns.
    fn anime_cards_view(&self, playing: bool) -> Element<'_, Message> {
        let idle = self.busy.is_none() && self.getting.is_none();
        let mut col = column![
            checkbox(self.anime == Some(true)).label("Anime cards").on_toggle_maybe((idle && self.anime.is_some()).then_some(Message::AnimeCards)).style(yo3_skin::tick).size(16).text_size(15),
            text("Play and see some cards as they were in the anime. Takes effect from the next game start.").size(14).color(color::INK_SOFT),
        ].spacing(6);
        if self.anime.is_none() { col = col.push(text("The setting can't be read right now.").size(14).color(color::INK_SOFT)) }
        if playing && self.anime.is_some() { col = col.push(text("With the game open, changing it makes RESTART THE GAME appear on the main screen.").size(14).color(color::INK_SOFT)) }
        col.into()
    }

    /// The anime card pictures (#60): how many there are and the three ways to get more.
    /// The bar of the anime pictures (#60) and its words: the download's progress, a wait of unknown length while a zip is
    /// read or written, what the last download stopped on, or how many pictures there are.
    pub(crate) fn pictures_bar(&self) -> (Bar, String) {
        let (have, total) = self.pictures.unwrap_or((0, 0));
        if let Some((done, all, cancel)) = &self.getting {
            return match (cancel.load(std::sync::atomic::Ordering::SeqCst), *done) {
                (true, _) => (Bar::Starting, "Stopping…".into()),
                (false, 0) => (Bar::Starting, "Starting…".into()),
                _ => (Bar::of(*done, *all), format!("{done} of {all}")),
            }
        }
        if let Some(busy) = self.art_busy { return (Bar::Starting, busy.to_owned()) }
        let fraction = if total == 0 { 0.0 } else { have as f32 / total as f32 };
        match (&self.art_problem, self.pictures) {
            (_, None) => (Bar::Starting, "Counting…".into()),
            (Some(problem), _) => (Bar::Failed(fraction), format!("{have} of {total} pictures ready: {problem}")),
            (None, Some(_)) if have >= total && total > 0 => (Bar::Done, format!("{have} of {total} pictures ready")),
            (None, Some(_)) => (Bar::Fraction(fraction), format!("{have} of {total} pictures ready")),
        }
    }

    /// The anime card pictures (#60): how many there are and the three ways to get more.
    fn pictures_view(&self, playing: bool) -> Element<'_, Message> {
        let idle = self.getting.is_none() && self.busy.is_none();
        let all = self.pictures.is_some_and(|(have, total)| have >= total);
        let get = match &self.getting {
            Some(_) => pill("Cancel", Some(Message::CancelPictures), Kind::Secondary, 150, 32),
            None => pill("Get pictures", (idle && !all && self.pictures.is_some()).then_some(Message::GetPictures), Kind::Primary, 150, 32),
        };
        let mut hint = format!("Cards with an anime version can show its anime artwork. Get pictures downloads {} from ProjectIgnis' picture server, one card at a time, \
            only when you press it. A card without a picture keeps the game's artwork.", pictures::SIZE_NOTE);
        if playing { hint.push_str(" The game is open: new pictures show after you close it and press PLAY again.") } else { hint.push_str(" New pictures show from the next PLAY.") }
        let (bar, words) = self.pictures_bar();
        column![
            text(hint).size(14).color(color::INK_SOFT),
            yo3_skin::progress_bar(bar, words, 22.0),
            row![get, pill("Add pictures from a file…", idle.then_some(Message::AddPictures), Kind::Secondary, 240, 32),
                pill("Save pictures to a file…", idle.then_some(Message::SavePictures), Kind::Secondary, 230, 32)].spacing(10),
        ].spacing(6).into()
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
            pill("Reset…", (self.busy.is_none() && !playing).then_some(Message::ResetData(false)), Kind::Secondary, 120, 32)].spacing(10).align_y(Center);
        if self.confirm_reset {
            reset = row![text("Reset your singleplayer data? The old data is kept beside it.").size(14).font(BODY_BOLD).color(color::BAD).width(Fill),
                pill("Reset", Some(Message::ResetData(true)), Kind::Danger, 100, 32), yo3_skin::link("Cancel", Message::ResetData(false), false)].spacing(10).align_y(Center);
        }
        column![
            item("Anime cards", self.anime_cards_view(playing)),
            item("Banlist", column![banlist, text("The banlist your own server checks decks against. From the next PLAY.").size(14).color(color::INK_SOFT)].spacing(6).into()),
            item("Backups", column![
                text("Your server keeps a backup from each of its last starts.").size(14).color(color::INK_SOFT),
                row![pill("Back up now", self.busy.is_none().then_some(Message::BackUp), Kind::Secondary, 150, 32),
                    pill("Restore…", (!playing).then_some(Message::Restore), Kind::Secondary, 130, 32),
                    pill("Open data folder", Some(Message::OpenData), Kind::Secondary, 170, 32)].spacing(10),
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
                    pill("Browse…", Some(Message::Browse(which)), Kind::Secondary, 110, 32), pill("Save", Some(Message::Save(which)), Kind::Primary, 90, 32)].spacing(8).align_y(Center),
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
            row![pill("Export logs…", Some(Message::ExportLogs), Kind::Secondary, 150, 32), pill("Open logs folder", Some(Message::OpenLogs), Kind::Secondary, 170, 32)].spacing(10),
        ].spacing(8).into()))
        .extend((!self.moved.is_empty()).then(|| {
            let mut list = column![text("YO3 Revival renamed these files in your game folder, so Windows doesn't load them with the game. Put one back to undo it.").size(14).color(color::INK_SOFT)].spacing(8);
            for name in &self.moved {
                list = list.push(row![text(name.as_str()).size(15).color(color::INK).width(Fill),
                    pill("Put back", Some(Message::PutBack(name.clone())), Kind::Secondary, 120, 32)].spacing(10).align_y(Center));
            }
            item("Files moved aside", list.into())
        }))
        .push(item("Reset local game data", if self.confirm_game_reset {
            column![
                text("Which data? The old data is kept in a backup folder either way.").size(14).font(BODY_BOLD).color(color::INK),
                row![pill("Display and login settings", self.busy.is_none().then_some(Message::ResetGame(Some(false))), Kind::Danger, 240, 32),
                    pill("All local game data", self.busy.is_none().then_some(Message::ResetGame(Some(true))), Kind::Danger, 190, 32),
                    yo3_skin::link("Cancel", Message::ResetGame(None), false)].spacing(10).align_y(Center),
            ].spacing(8).into()
        } else {
            row![text("When the game's screen or saved login settings are stuck, the game can start from fresh ones.").size(14).color(color::INK_SOFT).width(Fill),
                pill("Reset…", Some(Message::ResetGame(None)), Kind::Secondary, 120, 32)].spacing(10).align_y(Center).into()
        }))
        .into()
    }

    fn about(&self) -> Element<'_, Message> {
        let mut updates = row![pill("Check for updates", self.busy.is_none().then_some(Message::CheckUpdates), Kind::Secondary, 190, 32)].spacing(14).align_y(Center);
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

    #[test]
    fn our_own_loader_that_needs_repair_is_said_so_and_never_an_unowned_mod() {
        // #65 on Windows 11: the line read "the next PLAY sets it up" for a loader without its receipt.
        let own = direct_of(true, direct::State::NeedsRepair("receipt missing".into()));
        assert_eq!(own, Direct::Repair);
        assert_eq!(direct_of(true, direct::State::Disabled), Direct::NextPlay);
        assert!(matches!(direct_of(true, direct::State::Conflict("another".into())), Direct::Conflict(_)));
        assert_eq!(direct_of(false, direct::State::NeedsRepair("x".into())), Direct::Off, "the choice is off: nothing to repair");
        assert_eq!(direct_of(true, direct::State::Enabled), Direct::On);
    }

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
    fn the_anime_pictures_line_counts_and_a_finished_download_says_when_the_pictures_show() {
        use yo3_launcher::art::get::Outcome;
        let mut ctx = ctx();
        let mut state = State::new(&ctx, Section::Game);
        let _ = state.update(Message::PicturesCounted((12, 418)), &mut ctx, false);
        assert_eq!(state.pictures, Some((12, 418)));
        // A download in progress shows its count; a refused second click starts nothing.
        state.getting = Some((0, 418, Arc::new(AtomicBool::new(false))));
        let _ = state.update(Message::Pictures(pictures::Event::Progress(40, 418)), &mut ctx, false);
        assert_eq!(state.getting.as_ref().map(|g| (g.0, g.1)), Some((40, 418)));
        let _ = state.update(Message::CancelPictures, &mut ctx, false);
        assert!(state.getting.as_ref().unwrap().2.load(std::sync::atomic::Ordering::SeqCst), "Cancel stops the run at its next card");
        let ended = |playing: bool, out: Outcome| {
            let mut ctx = self::tests::ctx();
            let mut state = State::new(&ctx, Section::Game);
            state.getting = Some((1, 2, Arc::new(AtomicBool::new(false))));
            let _ = state.update(Message::Pictures(pictures::Event::Done(Ok(out))), &mut ctx, playing);
            assert!(state.getting.is_none());
            state.note.unwrap()
        };
        let got = Outcome { saved: vec![1, 2, 3], ..Outcome::default() };
        assert_eq!((ended(false, got.clone()).good, ended(false, got.clone()).sentence), (true, "Got 3 pictures. They show from the next PLAY.".to_owned()));
        assert!(ended(true, got).sentence.contains("open game shows them after you close it"));
        let down = ended(false, Outcome { unreachable: Some("x".into()), ..Outcome::default() });
        assert!(!down.good && down.sentence.contains("Add pictures from a file") && !down.sentence.contains("next PLAY"), "{}", down.sentence);
        assert!(yo3_desktop::words::technical(&down.sentence).is_none());
    }

    #[test]
    fn the_pictures_bar_follows_the_download_the_wait_and_what_it_left() {
        let ctx = ctx();
        let mut state = State::new(&ctx, Section::Game);
        assert_eq!(state.pictures_bar(), (Bar::Starting, "Counting…".to_owned()));
        state.pictures = Some((12, 418));
        assert_eq!(state.pictures_bar(), (Bar::Fraction(12.0 / 418.0), "12 of 418 pictures ready".to_owned()));
        let cancel = Arc::new(AtomicBool::new(false));
        state.getting = Some((0, 418, cancel.clone()));
        assert_eq!(state.pictures_bar(), (Bar::Starting, "Starting…".to_owned()));
        state.getting = Some((209, 418, cancel.clone()));
        assert_eq!(state.pictures_bar(), (Bar::Fraction(0.5), "209 of 418".to_owned()));
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(state.pictures_bar().1, "Stopping…");
        state.getting = None;
        state.busy = Some("Adding the pictures…");
        state.art_busy = state.busy;
        assert_eq!(state.pictures_bar(), (Bar::Starting, "Adding the pictures…".to_owned()), "a zip is read with a wait of unknown length");
        state.busy = None;
        state.art_busy = None;
        state.pictures = Some((400, 418));
        state.art_problem = Some("18 not fetched".into());
        assert_eq!(state.pictures_bar().1, "400 of 418 pictures ready: 18 not fetched");
        assert!(matches!(state.pictures_bar().0, Bar::Failed(f) if (f - 400.0 / 418.0).abs() < 1e-6));
        state.art_problem = None;
        state.pictures = Some((418, 418));
        assert_eq!(state.pictures_bar(), (Bar::Done, "418 of 418 pictures ready".to_owned()));
        // The words on the bar are plain.
        for words in ["Starting…", "Stopping…", "The picture server can't be reached", "400 of 418 pictures ready: 18 not fetched"] { assert_eq!(yo3_desktop::words::technical(words), None) }
    }

    #[test]
    fn the_offer_counts_the_missing_pictures_in_a_sentence() {
        assert_eq!(missing_words(0, 418), "None of the 418 cards has a picture yet.");
        assert_eq!(missing_words(167, 418), "251 of the 418 cards have no picture yet.");
        assert_eq!(missing_words(417, 418), "1 of the 418 cards has no picture yet.");
        assert_eq!(missing_words(418, 418), "Every card has a picture.");
    }

    #[test]
    fn the_anime_cards_switch_offers_the_download_only_when_pictures_are_missing() {
        assert_eq!((offer(None), offer(Some((418, 418))), offer(Some((167, 418))), offer(Some((0, 0)))), (None, None, Some(251), None));
        let mut ctx = ctx();
        let turn_on = |state: &mut State, ctx: &mut Ctx| { let _ = state.update(Message::AnimeSet(true, Ok(true)), ctx, false); };
        // Missing pictures: the dialog opens, the switch is on, and Not now only closes it.
        let mut state = State::new(&ctx, Section::Singleplayer);
        state.pictures = Some((167, 418));
        turn_on(&mut state, &mut ctx);
        assert_eq!((state.anime, state.dialog.clone()), (Some(true), Some(Dialog::Offer)));
        assert!(state.has_dialog(), "Escape has a dialog to close before it leaves Settings");
        let _ = state.update(Message::PicturesDialogClose, &mut ctx, false);
        assert!(!state.has_dialog());
        assert_eq!((state.anime, state.dialog.clone()), (Some(true), None), "Not now keeps the switch on");
        // Every picture there: no dialog.
        let mut full = State::new(&ctx, Section::Singleplayer);
        full.pictures = Some((418, 418));
        turn_on(&mut full, &mut ctx);
        assert_eq!((full.anime, full.dialog.clone()), (Some(true), None));
        // Turning it off never opens one.
        let _ = state.update(Message::AnimeSet(false, Ok(false)), &mut ctx, false);
        assert_eq!((state.anime, state.dialog.clone()), (Some(false), None));
        // Not counted yet: the dialog waits for the count, and only if some are missing.
        let mut early = State::new(&ctx, Section::Singleplayer);
        turn_on(&mut early, &mut ctx);
        assert!(early.dialog.is_none() && early.offer_when_counted);
        let _ = early.update(Message::PicturesCounted((10, 418)), &mut ctx, false);
        assert_eq!(early.dialog, Some(Dialog::Offer));
        let mut early_full = State::new(&ctx, Section::Singleplayer);
        turn_on(&mut early_full, &mut ctx);
        let _ = early_full.update(Message::PicturesCounted((418, 418)), &mut ctx, false);
        assert!(early_full.dialog.is_none());
        // A server that kept the old value is an error, not a silent switch.
        let mut refused = State::new(&ctx, Section::Singleplayer);
        refused.pictures = Some((0, 418));
        let _ = refused.update(Message::AnimeSet(true, Ok(false)), &mut ctx, false);
        assert!(refused.dialog.is_none() && refused.anime == Some(false) && refused.note.as_ref().is_some_and(|n| !n.good));
    }

    #[test]
    fn the_pictures_dialog_goes_from_offer_to_download_to_result_and_the_switch_stays_on() {
        use yo3_launcher::art::get::Outcome;
        let mut ctx = ctx();
        let mut state = State::new(&ctx, Section::Singleplayer);
        state.pictures = Some((0, 418));
        state.anime = Some(true);
        state.dialog = Some(Dialog::Offer);
        // Get pictures starts it inside the dialog: the bar is the shared one, Cancel stops it.
        state.getting = Some((0, 418, Arc::new(AtomicBool::new(false))));
        assert_eq!(state.pictures_bar(), (Bar::Starting, "Starting…".to_owned()));
        let _ = state.update(Message::Pictures(pictures::Event::Progress(100, 418)), &mut ctx, false);
        assert_eq!(state.pictures_bar().1, "100 of 418");
        // It ends with the result in the dialog (no note behind it) and a Close button.
        let _ = state.update(Message::Pictures(pictures::Event::Done(Ok(Outcome { saved: vec![1, 2], ..Outcome::default() }))), &mut ctx, false);
        assert!(state.getting.is_none() && state.note.is_none());
        assert!(matches!(&state.dialog, Some(Dialog::Result { good: true, sentence }) if sentence.starts_with("Got 2 pictures.")), "{:?}", state.dialog);
        let _ = state.update(Message::PicturesDialogClose, &mut ctx, false);
        assert_eq!((state.dialog.clone(), state.anime), (None, Some(true)));
        // Closing during a download cancels it, and the end then lands in a note, not a dialog.
        state.dialog = Some(Dialog::Offer);
        let cancel = Arc::new(AtomicBool::new(false));
        state.getting = Some((5, 418, cancel.clone()));
        let _ = state.update(Message::PicturesDialogClose, &mut ctx, false);
        assert!(cancel.load(std::sync::atomic::Ordering::SeqCst) && state.dialog.is_none());
        let _ = state.update(Message::Pictures(pictures::Event::Done(Ok(Outcome { cancelled: true, saved: vec![1], ..Outcome::default() }))), &mut ctx, false);
        assert!(state.note.is_some());
        // The words are plain, and the open game is told how to see the change.
        let mut open = State::new(&ctx, Section::Singleplayer);
        let _ = open.update(Message::AnimeSet(true, Ok(true)), &mut ctx, true);
        let note = open.note.unwrap();
        assert!(note.sentence.contains("RESTART THE GAME") && yo3_desktop::words::technical(&note.sentence).is_none());
    }

    #[test]
    fn pictures_added_from_a_file_say_what_was_added_and_a_wrong_file_is_one_plain_sentence() {
        let mut ctx = ctx();
        let mut state = State::new(&ctx, Section::Game);
        let done = yo3_launcher::art::pack::Imported { faces: 2, minis: 0, kept: 1, refused: vec![], ignored: 0 };
        let _ = state.update(Message::PicturesAdded(Ok(Some(done))), &mut ctx, false);
        assert_eq!(state.note.as_ref().map(|n| n.sentence.as_str()), Some("Added 2 pictures. 1 of yours was kept. They show from the next PLAY."));
        let _ = state.update(Message::PicturesAdded(Err("That file is not a zip file.".into())), &mut ctx, false);
        assert_eq!(state.note.as_ref().map(|n| (n.good, n.sentence.as_str())), Some((false, "That file is not a zip file.")));
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
