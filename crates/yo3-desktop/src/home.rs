//! The main screen: Singleplayer (your own server on this computer) and Multiplayer, around one Play button that
//! always says what happens next (#29). Your own server starts with Play and stops a grace period after the game
//! exits; a game opened without the launcher counts as playing.
use std::time::{Duration, Instant};
use iced::widget::{Space, column, container, pick_list, row, text};
use iced::{Alignment::Center, Element, Length::Fill, Subscription, Task};
use yo3_desktop::Readiness;
use yo3_desktop::singleplayer::{self, Backup};
use yo3_skin::{BODY_BOLD, Kind, color, pill};
use crate::app::{Ctx, dot, job};
use crate::multi;
use crate::error_presentation::{self, Hint};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab { #[default] Single, Multi }

/// After this long without the game, Starting offers Cancel.
pub const CANCEL_AFTER: Duration = Duration::from_secs(30);

/// Where a Play is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step { Server, Game }

/// What the launcher knows about the game right now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    /// The last check of the game folder and our files; `None` while it runs.
    pub readiness: Option<Readiness>,
    /// Play was pressed: the step it is on and since when.
    pub starting: Option<(Step, Instant)>,
    /// The game Play started is running.
    pub running: bool,
    /// The game's window is up (the connection DLL said so).
    pub ready: bool,
    /// When Close game (or Cancel) was asked for; forced after `singleplayer::CLOSE_TIMEOUT`.
    pub closing: Option<Instant>,
    /// A game of this prefix opened without the launcher.
    pub outside: bool,
}

impl Game {
    /// Play was pressed and the game hasn't shown its window yet: Starting, or Waiting.
    pub fn launching(&self) -> Option<Instant> {
        self.starting.map(|(_, since)| since)
    }
}

/// The Play button's state: its label, what a click does and the line above it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Play { Checking, SetUp, Repair, Retry, Ready, StartingServer, StartingGame, Waiting, Playing, Outside, Closing }

impl Play {
    pub fn of(game: &Game) -> Self {
        if game.closing.is_some() { return Play::Closing }
        if let Some((step, _)) = game.starting { return match step { Step::Server => Play::StartingServer, Step::Game => Play::StartingGame } }
        if game.running { return if game.ready { Play::Playing } else { Play::Waiting } }
        if game.outside { return Play::Outside }
        match &game.readiness {
            None => Play::Checking,
            Some(Readiness::Ready) => Play::Ready,
            Some(Readiness::GameProblem(_)) => Play::SetUp,
            Some(Readiness::ComponentProblem(_)) => Play::Repair,
            Some(Readiness::ValidationProblem(_)) => Play::Retry,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Play::Checking => "CHECKING…", Play::SetUp => "SET UP", Play::Repair => "REPAIR", Play::Retry => "RETRY", Play::Ready => "PLAY",
            Play::StartingServer | Play::StartingGame | Play::Waiting => "STARTING…", Play::Playing | Play::Outside => "PLAYING",
            Play::Closing => "CLOSING…",
        }
    }
    /// What a click sends; `None` draws the button disabled.
    pub fn action(self) -> Option<Message> {
        match self {
            Play::SetUp => Some(Message::SetUp), Play::Repair => Some(Message::Repair), Play::Retry => Some(Message::Recheck),
            Play::Ready => Some(Message::Play), _ => None,
        }
    }
    pub fn status(self) -> &'static str {
        match self {
            Play::Checking => "Checking your game and YO3 Revival's files…",
            Play::SetUp => "We can't find your game.",
            Play::Repair => "Some of YO3 Revival's files need repair.",
            Play::Retry => "The last check didn't finish.",
            Play::Ready => "",
            Play::StartingServer => "Starting your server…",
            Play::StartingGame => "Starting the game…",
            Play::Waiting => "Waiting for the game's window…",
            Play::Playing => "The game is running.",
            Play::Outside => "The game is open (started without the launcher).",
            Play::Closing => "Closing the game…",
        }
    }
    /// Starting states, where Cancel appears after `CANCEL_AFTER`.
    pub fn cancellable(self) -> bool { matches!(self, Play::StartingServer | Play::StartingGame | Play::Waiting) }
}

/// One plain sentence, with the technical text behind Details and at most one action.
#[derive(Clone, Debug)]
pub struct Notice { pub summary: String, pub details: Option<String>, pub open: bool, pub action: Option<(&'static str, Message)> }

/// A banlist choice: the value saved for the server, the name shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banlist { pub value: String, pub name: String }
impl std::fmt::Display for Banlist { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.name) } }

#[derive(Clone, Debug)]
pub struct Live { running: bool, ready: bool, outside: bool, failure: Option<String> }

#[derive(Clone, Debug)]
pub enum Message {
    Tab(Tab),
    Play,
    ServerStarted(Result<(), String>),
    Started(Result<bool, String>),
    Cancel,
    Cancelled(Result<(), String>),
    /// Set up direct start in a protected game folder (asks for permission once).
    AllowDirect,
    DirectAllowed(Result<(), String>),
    CloseGame,
    GameClosed(Result<(), String>),
    /// The grace period after the game exited is over (its generation).
    GraceOver(u64),
    ServerStopped(Result<(), String>),
    Banlist(Banlist),
    BanlistSaved(Result<String, String>),
    Banlists(Vec<(u32, String)>),
    Tick,
    Polled(Result<Live, String>),
    Recheck,
    Checked(Readiness, Option<String>),
    FingerprintSaved(Result<(), String>),
    /// Handled by the window: open setup at the Game step (Set up, or the missing-game banner's Fix).
    SetUp,
    /// Handled by the window: explain the repair.
    Repair,
    /// Handled by the window: list the server's backups to restore one.
    Restore,
    RestoreBackup(Backup),
    Restored(Result<(), String>),
    ToggleDetails,
    Dismiss,
    Multi(multi::Message),
    /// Handled by the window: the releases page, for a server that needs a newer launcher.
    Releases,
}

#[derive(Default)]
pub struct State {
    pub tab: Tab,
    pub game: Game,
    banlists: Vec<(u32, String)>,
    notice: Option<Notice>,
    polling: bool,
    /// Bumped by each Play and each game exit: only the latest grace period may stop your server.
    grace: u64,
    /// Cancel was asked for while a start job was still running; its result is cleaned up when it arrives.
    cancelled: bool,
    /// When the Play that is now waiting for the game's window was pressed.
    waiting_since: Option<Instant>,
    /// The Multiplayer tab (#30).
    pub multi: multi::State,
}

impl State {
    pub fn update(&mut self, message: Message, ctx: &mut Ctx) -> Task<Message> {
        match message {
            Message::Tab(tab) => {
                self.tab = tab;
                if tab == Tab::Multi { return self.multi.load(ctx).map(Message::Multi) }
            }
            Message::Multi(multi::Message::Play) => {
                let busy = self.game.running || self.game.starting.is_some() || self.game.outside;
                let (Some(backend), Some((server, signed)), false) = (ctx.backend.clone(), self.multi.session(), busy) else { return Task::none() };
                let settings = ctx.settings.clone();
                self.cancelled = false;
                self.game.starting = Some((Step::Game, Instant::now()));
                self.notice = None;
                let played = self.multi.played(ctx).map(Message::Multi);
                let start = job(move || {
                    // A device that stays signed in gets a fresh session for each Play; otherwise the sign-in's own.
                    let session = match &signed.device_token {
                        Some(token) => tokio::runtime::Builder::new_current_thread().enable_all().build()?
                            .block_on(yo3_desktop::multiplayer::resume(&server, token)).map_err(|p| anyhow::anyhow!(p.sentence()))?.session,
                        None => signed.session,
                    };
                    backend.play_signed_in(&settings, &server, &session)?;
                    Ok(false)
                }, Message::Started);
                return Task::batch([played, start]);
            }
            Message::Multi(multi::Message::Remedy(yo3_desktop::multiplayer::Remedy::Update)) => return Task::done(Message::Releases),
            Message::Multi(multi::Message::CloseGame) => return self.update(Message::CloseGame, ctx),
            Message::Multi(message) => return self.multi.update(message, ctx).map(Message::Multi),
            Message::Releases => {}
            Message::Play => {
                if Play::of(&self.game) != Play::Ready { return Task::none() }
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.grace += 1;
                self.cancelled = false;
                self.game.starting = Some((Step::Server, Instant::now()));
                self.notice = None;
                // Your own server first (reused when it is still running from the last game).
                return job(move || backend.start_server(&settings).map(|_| ()), Message::ServerStarted);
            }
            Message::ServerStarted(result) => {
                if self.cancelled { return self.clean_up(ctx) }
                match result {
                    Ok(()) => {
                        let Some(backend) = ctx.backend.clone() else { return Task::none() };
                        let settings = ctx.settings.clone();
                        let since = self.game.launching().unwrap_or_else(Instant::now);
                        self.game.starting = Some((Step::Game, since));
                        return job(move || {
                            // Opening yo3.exe without the launcher keeps working with your own server, when setup chose it.
                            let needs_permission = settings.direct_start && match backend.auto_direct(&settings, None) {
                                yo3_desktop::direct::AutoDirect::Ready => false,
                                yo3_desktop::direct::AutoDirect::NeedsPermission => true,
                                yo3_desktop::direct::AutoDirect::Skipped(why) => { backend.diagnostics.line(&format!("Direct start not set up: {why}")); false }
                            };
                            backend.play(&settings, None, &settings.local_account)?;
                            Ok(needs_permission)
                        }, Message::Started);
                    }
                    Err(error) => {
                        self.game.starting = None;
                        let damaged = singleplayer::damaged_database(&error);
                        self.fail(ctx, error);
                        if damaged && let Some(notice) = &mut self.notice {
                            notice.summary = "Your singleplayer data couldn't be opened. A backup from an earlier start can replace it.".into();
                            notice.action = Some(("Restore a backup", Message::Restore));
                        }
                        return self.check(ctx);
                    }
                }
            }
            Message::Started(result) => {
                if self.cancelled { return self.clean_up(ctx) }
                let since = self.game.launching();
                self.game.starting = None;
                match result {
                    Ok(needs_permission) => {
                        self.game.running = true;
                        self.game.ready = false;
                        // Waiting for the window keeps the Play's start time, for Cancel.
                        self.waiting_since = since;
                        if needs_permission {
                            self.notice = Some(Notice { summary: "Opening the game without the launcher needs permission once, because the game folder is protected.".into(),
                                details: None, open: false, action: Some(("Allow", Message::AllowDirect)) });
                        }
                    }
                    Err(error) => { self.fail(ctx, error); self.start_grace(); return Task::batch([self.check(ctx), self.grace_timer()]) }
                }
            }
            Message::Cancel => {
                self.cancelled = true;
                return self.clean_up(ctx);
            }
            Message::Cancelled(result) => {
                self.game = Game { readiness: self.game.readiness.clone(), ..Game::default() };
                self.waiting_since = None;
                self.cancelled = false;
                if let Err(error) = result { self.fail(ctx, error) }
                return self.check(ctx);
            }
            Message::AllowDirect => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.notice = None;
                return job(move || backend.enable_direct(&settings, None), Message::DirectAllowed);
            }
            Message::DirectAllowed(result) => if let Err(error) = result { self.fail(ctx, error) },
            Message::CloseGame => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                self.game.closing = Some(Instant::now());
                return job(move || backend.stop_game(), Message::GameClosed);
            }
            Message::GameClosed(result) => if let Err(error) = result { self.fail(ctx, error) },
            Message::GraceOver(generation) => {
                if generation != self.grace || self.game.running || self.game.starting.is_some() || self.game.outside { return Task::none() }
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                return job(move || { if backend.live(&settings).is_some() { backend.stop_server(&settings)?; } Ok(()) }, Message::ServerStopped);
            }
            Message::ServerStopped(result) => if let Err(error) = result { self.fail(ctx, error) },
            Message::Banlist(choice) => {
                let Some(store) = ctx.store.clone() else { return Task::none() };
                let value = choice.value;
                return Task::perform(async move { store.run(move |db| db.save_banlist(&value)).await.map_err(|e| format!("{e:#}")) }, Message::BanlistSaved);
            }
            Message::BanlistSaved(Ok(value)) => ctx.settings.banlist = value,
            Message::BanlistSaved(Err(error)) => self.fail(ctx, error),
            Message::Banlists(list) => self.banlists = list,
            Message::Tick => {
                // Close game that the game ignored: force it.
                if let Some(since) = self.game.closing && since.elapsed() >= singleplayer::CLOSE_TIMEOUT && let Some(backend) = ctx.backend.clone() {
                    let settings = ctx.settings.clone();
                    self.game.closing = Some(Instant::now());
                    return job(move || backend.force_stop_game(&settings), Message::GameClosed);
                }
                if self.polling || self.game.starting.is_some() { return Task::none() }
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.polling = true;
                return job(move || Ok(Live { running: backend.game_running(), ready: backend.game_ready(),
                    outside: backend.game_open_elsewhere(&settings), failure: backend.game_failure() }), Message::Polled);
            }
            Message::Polled(result) => {
                self.polling = false;
                let Ok(live) = result else { return Task::none() };
                if self.game.starting.is_some() { return Task::none() }
                let ended = self.game.running && !live.running;
                self.game.running = live.running;
                self.game.ready = live.ready;
                self.game.outside = live.outside;
                if live.ready { self.waiting_since = None }
                if ended {
                    self.game.closing = None;
                    self.waiting_since = None;
                    if let Some(failure) = live.failure { self.fail(ctx, failure) }
                    // Your server waits a little, so a quick relaunch reuses it; the game folder may have changed
                    // while it ran (an update, an antivirus), so check again for the next Play.
                    self.start_grace();
                    return Task::batch([self.check(ctx), self.grace_timer()]);
                }
            }
            Message::Recheck => return self.check(ctx),
            Message::Checked(readiness, fingerprint) => {
                self.game.readiness = Some(readiness);
                // The full check passed for a changed game: remember it, so the next start is quick again.
                if let (Some(fingerprint), Some(store)) = (fingerprint, ctx.store.clone()) && fingerprint != ctx.settings.game_fingerprint {
                    ctx.settings.game_fingerprint = fingerprint.clone();
                    let settings = ctx.settings.clone();
                    return Task::perform(async move { store.run(move |db| db.save_settings(&settings)).await.map_err(|e| format!("{e:#}")) }, Message::FingerprintSaved);
                }
            }
            Message::FingerprintSaved(result) => if let Err(error) = result { self.fail(ctx, error) },
            Message::RestoreBackup(backup) => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.notice = None;
                return job(move || backend.restore_backup(&settings, &backup), Message::Restored);
            }
            Message::Restored(result) => match result {
                Ok(()) => self.notice = Some(Notice { summary: "The backup is back in place. Press PLAY.".into(), details: None, open: false, action: None }),
                Err(error) => self.fail(ctx, error),
            },
            Message::ToggleDetails => if let Some(notice) = &mut self.notice { notice.open = !notice.open },
            Message::Dismiss => self.notice = None,
            Message::SetUp | Message::Repair | Message::Restore => {}
        }
        Task::none()
    }

    /// Stops what a cancelled Play started: the game by force, then your server.
    fn clean_up(&mut self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        let settings = ctx.settings.clone();
        self.game.closing = Some(Instant::now());
        job(move || {
            backend.force_stop_game(&settings)?;
            if backend.live(&settings).is_some() { backend.stop_server(&settings)?; }
            Ok(())
        }, Message::Cancelled)
    }

    fn start_grace(&mut self) { self.grace += 1 }

    fn grace_timer(&self) -> Task<Message> {
        let generation = self.grace;
        Task::perform(tokio::time::sleep(singleplayer::GRACE), move |()| Message::GraceOver(generation))
    }

    /// Checks the game folder and YO3 Revival's own files in the background. When `yo3.exe` is unchanged since the last
    /// full check, only YO3 Revival's own files are looked at again (#28); Play checks the game itself anyway.
    pub fn check(&mut self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        if self.game.running || self.game.starting.is_some() { return Task::none() }
        self.game.readiness = None;
        let settings = ctx.settings.clone();
        Task::perform(async move {
            tokio::task::spawn_blocking(move || {
                if let Some(quick) = backend.quick_readiness(&settings) { return (quick, None) }
                match backend.verify_setup(&settings, None) {
                    Ok(()) => (Readiness::Ready, yo3_desktop::setup_check::fingerprint(std::path::Path::new(&settings.game_dir)).ok()),
                    Err(error) => (match backend.readiness(&settings) { Readiness::Ready => Readiness::ValidationProblem(format!("{error:#}")), problem => problem }, None),
                }
            }).await.unwrap_or_else(|error| (Readiness::ValidationProblem(error.to_string()), None))
        }, |(readiness, fingerprint)| Message::Checked(readiness, fingerprint))
    }

    /// The game's banlist names, read in the background (the game's data takes a moment).
    pub fn load_banlists(&self, ctx: &Ctx) -> Task<Message> {
        let game_dir = std::path::PathBuf::from(&ctx.settings.game_dir);
        Task::perform(async move { tokio::task::spawn_blocking(move || yo3_desktop::banlists(&game_dir)).await.unwrap_or_default() }, Message::Banlists)
    }

    /// Shows one plain sentence for `error`, with the technical text (paths masked) behind Details.
    pub fn fail(&mut self, ctx: &Ctx, error: String) {
        if let Some(backend) = &ctx.backend { backend.diagnostics.line(&error) }
        let hint = match &self.game.readiness {
            Some(Readiness::ComponentProblem(_)) => Hint::Component,
            Some(Readiness::GameProblem(_)) => Hint::GameFolder,
            _ => Hint::General,
        };
        let shown = error_presentation::present(&error, hint, &crate::app::masks(ctx));
        self.notice = Some(Notice { summary: shown.summary, details: shown.diagnosis, open: false, action: None });
    }

    /// Since when Play has been starting or waiting for the game's window, for Cancel.
    pub fn launching_since(&self) -> Option<Instant> { self.game.launching().or(self.waiting_since) }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick)
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Message> {
        let playing = self.game.running || self.game.starting.is_some() || self.game.outside;
        let side = match self.tab { Tab::Single => self.singleplayer(ctx), Tab::Multi => self.multi.view(playing).map(Message::Multi) };
        let server_news = self.multi.news();
        let news = match self.tab {
            Tab::Single => news(WELCOME.iter().map(|(t, h)| (t.to_string(), h.to_string())).collect()),
            Tab::Multi => news(server_news),
        };
        let panels = row![
            container(yo3_skin::panel(Some("Information"), news)).width(Fill).height(Fill),
            container(side).width(372).height(Fill),
        ].spacing(16).height(Fill);
        let mut page = column![].spacing(10).padding([16, 18]).height(Fill);
        // The game folder moved or was removed since setup: say so plainly, with the fix, instead of a broken button.
        if game_missing(&ctx.settings) {
            page = page.push(container(row![
                container(Space::new().width(10).height(10)).style(|_| dot(color::BAD)),
                text("We can't find your game.").size(15).font(BODY_BOLD).color(color::INK),
                Space::new().width(Fill),
                pill("Fix", Some(Message::SetUp), Kind::Primary, 90, 32),
            ].spacing(10).align_y(Center)).padding([6, 12]).style(|_| container::Style {
                background: Some(color::PANEL_TOP.into()), border: iced::Border { color: color::BAD, width: 1.5, radius: 4.0.into() }, ..container::Style::default() }));
        }
        page.push(panels).into()
    }

    fn singleplayer<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Message> {
        let play = Play::of(&self.game);
        let mut status = row![text(play.status()).size(14).color(color::INK_SOFT)].spacing(10).align_y(Center).height(22);
        if matches!(play, Play::Playing | Play::Waiting) && self.launching_since().is_none_or(|since| since.elapsed() < CANCEL_AFTER) {
            status = status.push(Space::new().width(Fill)).push(yo3_skin::link("Close game", Message::CloseGame, false));
        } else if play.cancellable() && self.launching_since().is_some_and(|since| since.elapsed() >= CANCEL_AFTER) {
            status = status.push(Space::new().width(Fill)).push(yo3_skin::link("Cancel", Message::Cancel, false));
        }
        let choices = banlist_choices(&self.banlists);
        let selected = choices.iter().find(|c| c.value == ctx.settings.banlist).cloned();
        // The banlist is read when your server starts: it can change only while nothing is starting or playing.
        let editable = !self.game.running && self.game.starting.is_none() && !self.game.outside;
        let banlist: Element<'_, Message> = if editable {
            pick_list(choices, selected, Message::Banlist).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu)
                .width(Fill).padding([7, 10]).placeholder("Season 16").into()
        } else {
            text(selected.map_or_else(|| "Season 16 (default)".to_owned(), |b| b.name)).size(15).color(color::INK).into()
        };
        yo3_skin::panel(Some("Your own server"), column![
            text("Duel the NPCs, open packs and build decks. Cards you earn here stay on this PC.").size(15).color(color::INK),
            row![text("Banlist").font(BODY_BOLD).size(15).color(color::INK).width(74), banlist].align_y(Center),
            self.notice(),
            Space::new().height(Fill),
            status,
            pill(play.label(), play.action(), Kind::Play, Fill, 70),
        ].spacing(14))
    }

    fn notice(&self) -> Element<'_, Message> {
        let Some(notice) = &self.notice else { return Space::new().into() };
        let mut links = row![].spacing(14);
        if let Some((label, action)) = &notice.action { links = links.push(yo3_skin::link(*label, action.clone(), false)) }
        links = links.push(yo3_skin::link("Dismiss", Message::Dismiss, false));
        if notice.details.is_some() {
            links = links.push(yo3_skin::link(if notice.open { "Hide details" } else { "Details" }, Message::ToggleDetails, false));
        }
        let mut body = column![
            row![container(Space::new().width(9).height(9)).style(|_| dot(color::BAD)), text(&notice.summary).size(14).font(BODY_BOLD).color(color::INK)]
                .spacing(8).align_y(Center),
        ].spacing(6);
        if notice.open && let Some(details) = &notice.details { body = body.push(text(details).size(13).color(color::INK_SOFT)) }
        body.push(links).into()
    }
}

/// Setup was finished, but `yo3.exe` isn't where it was.
pub fn game_missing(settings: &yo3_desktop::Settings) -> bool {
    settings.setup_complete && !std::path::Path::new(&settings.game_dir).join("yo3.exe").is_file()
}

/// The banlists to choose from: the server's default first, then the game's own lists.
fn banlist_choices(lists: &[(u32, String)]) -> Vec<Banlist> {
    let mut choices = vec![Banlist { value: String::new(), name: "Season 16 (default)".into() }];
    choices.extend(lists.iter().map(|(id, name)| Banlist { value: id.to_string(), name: name.clone() }));
    choices
}

const WELCOME: [(&str, &str); 3] = [
    ("Beta", "Welcome to the new YO3 Revival launcher"),
    ("Beta", "Your own server starts when you press Play"),
    ("Beta", "Multiplayer: sign in to a server with your User ID"),
];

/// The Information panel: a title and a line per entry (Singleplayer's welcome, or the selected server's news).
fn news<'a>(items: Vec<(String, String)>) -> Element<'a, Message> {
    let mut list = column![];
    for (i, (title, body)) in items.into_iter().enumerate() {
        let shade = if i % 2 == 0 { color::ROW } else { color::ROW_ALT };
        list = list.push(container(column![
            text(title).font(BODY_BOLD).size(15).color(color::NAVY),
            text(body).size(15).color(color::INK),
        ].spacing(2)).padding([9, 12]).width(Fill)
            .style(move |_| container::Style { background: Some(shade.into()), ..container::Style::default() }));
    }
    container(list).style(|_| container::Style { border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 2.0.into() }, ..container::Style::default() }).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(readiness: Option<Readiness>) -> Game { Game { readiness, ..Game::default() } }
    const ALL: [Play; 11] = [Play::Checking, Play::SetUp, Play::Repair, Play::Retry, Play::Ready, Play::StartingServer, Play::StartingGame,
        Play::Waiting, Play::Playing, Play::Outside, Play::Closing];

    /// Every Play state, from the facts that lead to it (#29's table without the Multiplayer rows, which need #30).
    #[test]
    fn every_play_state_is_reached_from_its_facts() {
        let now = Instant::now();
        let ready = || game(Some(Readiness::Ready));
        let cases: [(Game, Play); 11] = [
            (game(None), Play::Checking),
            (game(Some(Readiness::GameProblem("no yo3.exe".into()))), Play::SetUp),
            (game(Some(Readiness::ComponentProblem("no dll".into()))), Play::Repair),
            (game(Some(Readiness::ValidationProblem("timeout".into()))), Play::Retry),
            (ready(), Play::Ready),
            (Game { starting: Some((Step::Server, now)), ..ready() }, Play::StartingServer),
            (Game { starting: Some((Step::Game, now)), ..ready() }, Play::StartingGame),
            (Game { running: true, ..ready() }, Play::Waiting),
            (Game { running: true, ready: true, ..ready() }, Play::Playing),
            (Game { outside: true, ..ready() }, Play::Outside),
            (Game { running: true, ready: true, closing: Some(now), ..ready() }, Play::Closing),
        ];
        let reached: Vec<Play> = cases.iter().map(|(facts, expected)| { assert_eq!(Play::of(facts), *expected, "{facts:?}"); *expected }).collect();
        assert_eq!(reached, ALL, "every state has a case");
    }

    #[test]
    fn only_states_with_a_next_step_are_clickable_and_every_disabled_one_explains_itself() {
        for play in ALL {
            assert_eq!(play.action().is_some(), matches!(play, Play::SetUp | Play::Repair | Play::Retry | Play::Ready), "{play:?}");
            assert!(play == Play::Ready || !play.status().is_empty(), "{play:?} needs a line saying why");
        }
        assert!(ALL.iter().filter(|p| p.cancellable()).eq([Play::StartingServer, Play::StartingGame, Play::Waiting].iter()));
    }

    #[test]
    fn a_game_open_without_the_launcher_blocks_a_second_one() {
        let outside = Game { outside: true, ..game(Some(Readiness::Ready)) };
        assert_eq!(Play::of(&outside), Play::Outside);
        assert!(Play::of(&outside).action().is_none(), "a second game can't be started");
    }

    #[test]
    fn only_the_latest_grace_period_may_stop_your_server() {
        let mut state = State::default();
        state.start_grace();
        let first = state.grace;
        state.start_grace(); // Play again, or the game exited again
        assert_ne!(first, state.grace);
        let mut ctx = crate::app::Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() };
        // An old timer does nothing (and without a backend nothing could be stopped anyway).
        let _ = state.update(Message::GraceOver(first), &mut ctx);
        assert_eq!(state.game, Game::default());
    }

    #[test]
    fn the_default_banlist_comes_first_and_ids_are_saved() {
        let choices = banlist_choices(&[(4, "Season 13".into())]);
        assert_eq!(choices[0].value, "");
        assert_eq!(choices[1], Banlist { value: "4".into(), name: "Season 13".into() });
    }
}
