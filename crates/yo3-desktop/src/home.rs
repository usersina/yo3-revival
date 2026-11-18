//! The main screen: Singleplayer (your own server on this computer) and Multiplayer, around one Play button that
//! always says what happens next (#29). Your own server starts with Play and stops a grace period after the game
//! exits; a game opened without the launcher counts as playing.
use std::time::{Duration, Instant};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment::Center, Element, Length::Fill, Subscription, Task};
use yo3_desktop::Readiness;
use yo3_desktop::direct::DirectProblem;
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

    /// While the game is starting, open or closing, the big button on either tab follows it: CLOSE GAME while it is
    /// open, CANCEL once a start has hung for `CANCEL_AFTER`, else a disabled label. `None`: the game isn't busy.
    pub fn busy(self, launching_since: Option<Instant>) -> Option<Busy> {
        let hung = launching_since.is_some_and(|since| since.elapsed() >= CANCEL_AFTER);
        let button = |label, action, line| Some(Busy { label, action, line });
        match self {
            _ if self.cancellable() && hung => button("CANCEL", Some(BusyAction::Cancel), "This is taking longer than usual."),
            Play::Playing | Play::Waiting => button("CLOSE GAME", Some(BusyAction::Close), self.status()),
            Play::StartingServer | Play::StartingGame | Play::Outside | Play::Closing => button(self.label(), None, self.status()),
            _ => None,
        }
    }
}

/// What the big button does while the game is busy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BusyAction { Close, Cancel }

/// The big button while the game is busy: its label, what it does (none: disabled) and the line above it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Busy { pub label: &'static str, pub action: Option<BusyAction>, pub line: &'static str }

/// One plain sentence, with the technical text behind Details and one action: its own (Retry, Fix…) or Report.
#[derive(Clone, Debug)]
pub struct Notice { pub summary: String, pub details: Option<String>, pub open: bool, pub action: Option<(&'static str, Message)>,
    /// Not a problem (a backup put back): no Report, no Help.
    pub good: bool,
    /// Shown on both tabs, not only your own server's panel: a game opened directly may have played either.
    pub everywhere: bool }

impl Notice {
    pub fn good(summary: impl Into<String>) -> Self { Self { summary: summary.into(), details: None, open: false, action: None, good: true, everywhere: false } }
    pub fn problem(summary: impl Into<String>, action: Option<(&'static str, Message)>) -> Self { Self { summary: summary.into(), details: None, open: false, action, good: false, everywhere: false } }
    /// A game opened directly couldn't play (#33): said once, on whichever tab is open.
    pub fn direct(problem: &DirectProblem) -> Self { Self { details: Some(problem.reason.clone()), everywhere: true, ..Self::problem(problem.summary(), None) } }
    /// Another program's `winmm.dll` runs inside the game (#65): said at each Play, in the bell, without stopping it.
    pub fn foreign_winmm(file: &std::path::Path) -> Self {
        Self { details: Some(format!("{}\nWindows loads this file into the game every time it starts. If it's from an older YO3 Revival \
            or another mod, move it out of the game folder while you play with YO3 Revival.", file.display())), everywhere: true,
            ..Self::problem("Another program's file in your game folder runs inside the game beside YO3 Revival and can break it, \
                for example by disconnecting you in the shop.", None) }
    }
    /// The notice for the game folder in `settings`, if another program's `winmm.dll` is there.
    pub fn foreign_winmm_in(settings: &yo3_desktop::Settings) -> Option<Self> {
        yo3_desktop::direct::foreign_winmm(std::path::Path::new(&settings.game_dir)).map(|file| Self::foreign_winmm(&file))
    }
}

/// A banlist choice: the value saved for the server, the name shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banlist { pub value: String, pub name: String }
impl std::fmt::Display for Banlist { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.name) } }

#[derive(Clone, Debug)]
pub struct Live { running: bool, ready: bool, outside: bool, failure: Option<String>, direct: Option<DirectProblem> }

#[derive(Clone, Debug)]
pub enum Message {
    Tab(Tab),
    Play,
    ServerStarted(Result<(), String>),
    Started(Result<bool, String>),
    /// Multiplayer's Play: whether the sign-in's session still plays.
    SessionChecked(Result<yo3_tunnel::Token, yo3_desktop::multiplayer::Problem>),
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
    /// A direct start's problem was shown, and forgotten.
    DirectShown,
    /// The title bar's bell: open or close the notifications.
    Bell(bool),
    /// A notification's Details (its place in the list).
    InboxDetails(usize),
    InboxDismiss(usize),
    Multi(multi::Message),
    /// Handled by the window: a server needs a newer launcher (What's new when the update is known, else the releases page).
    Releases,
    /// Handled by the window: the What's new dialog of the update (#32).
    WhatsNew,
    /// Handled by the window: restart into the downloaded update (#32).
    RestartToInstall,
    /// Handled by the window: Settings at a section.
    Settings(crate::settings::Section),
    /// Handled by the window: Report a problem with this error's text.
    Report(String),
    /// Handled by the window: Help at a topic.
    Help(&'static str),
    /// Handled by the window.
    Copy(String),
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
    /// The last Play couldn't set up direct start: the game folder needs permission once (Settings → Game).
    pub direct_permission: bool,
    /// The Multiplayer server the last Play was for, so its Allow sets direct start up for it (None: Singleplayer).
    allow_for: Option<yo3_desktop::multiplayer::Server>,
    /// A downloaded, checked update (its version): the ribbon above PLAY (#32). Set by the window.
    pub update_ready: Option<String>,
    /// The notifications behind the title bar's bell: notices about both tabs (a game opened directly that couldn't
    /// play, an update that didn't install, a permission to give), newest first.
    pub inbox: Vec<Notice>,
    /// How many came since the bell was last opened.
    pub unread: usize,
    pub bell_open: bool,
}

impl State {
    /// Direct start is set up in the protected game folder (the notice's Allow, or Settings' Fix permission): stop asking.
    pub fn permission_granted(&mut self) {
        self.direct_permission = false;
        let asks = |n: &Notice| matches!(n.action, Some((_, Message::AllowDirect)));
        if self.notice.as_ref().is_some_and(asks) { self.notice = None }
        self.inbox.retain(|n| !asks(n));
        self.unread = self.unread.min(self.inbox.len());
    }

    /// A notice for the bell (one about both tabs), or for your own server's panel.
    fn tell(&mut self, notice: Notice) {
        if notice.everywhere {
            self.inbox.retain(|n| n.summary != notice.summary);
            self.inbox.insert(0, notice);
            self.inbox.truncate(20);
            if !self.bell_open { self.unread += 1 }
        } else {
            self.notice = Some(notice);
        }
    }

    /// Multiplayer's Play, once its sign-in is known to play: the game on the selected server.
    fn play_multiplayer(&mut self, ctx: &mut Ctx, session: yo3_tunnel::Token) -> Task<Message> {
        let (Some(backend), Some((server, _))) = (ctx.backend.clone(), self.multi.session()) else { self.game.starting = None; return Task::none() };
        let settings = ctx.settings.clone();
        if let Some(notice) = Notice::foreign_winmm_in(&settings) { self.tell(notice) }
        let played = self.multi.played(ctx).map(Message::Multi);
        self.allow_for = Some(server.clone());
        let start = job(move || backend.play_signed_in(&settings, &server, &session), Message::Started);
        Task::batch([played, start])
    }

    pub fn update(&mut self, message: Message, ctx: &mut Ctx) -> Task<Message> {
        match message {
            Message::Tab(tab) => {
                self.tab = tab;
                if tab == Tab::Multi { return self.multi.load(ctx).map(Message::Multi) }
            }
            Message::Multi(multi::Message::Play) => {
                let busy = self.game.running || self.game.starting.is_some() || self.game.outside;
                let (Some(_), Some((server, signed)), false) = (ctx.backend.clone(), self.multi.session(), busy) else { return Task::none() };
                self.cancelled = false;
                self.game.starting = Some((Step::Game, Instant::now()));
                self.notice = None;
                // The session to play on, known to play before the game starts: a device that stays signed in gets a
                // fresh one; a plain sign-in's own is checked (a kick, a ban or a newer sign-in elsewhere ends it, and
                // the game would only say it has no connection).
                return Task::perform(async move {
                    use yo3_desktop::multiplayer as mp;
                    match &signed.device_token {
                        Some(token) => mp::resume(&server, token).await.map(|resumed| resumed.session),
                        None => mp::session_alive(&server, &signed.session).await.map(|()| signed.session),
                    }
                }, Message::SessionChecked);
            }
            Message::SessionChecked(result) => {
                if self.cancelled { return self.clean_up(ctx) }
                match result {
                    Ok(session) => return self.play_multiplayer(ctx, session),
                    Err(problem) => { self.game.starting = None; self.multi.play_refused(problem); }
                }
            }
            Message::Multi(multi::Message::Remedy(yo3_desktop::multiplayer::Remedy::Update)) => return Task::done(Message::Releases),
            Message::Multi(multi::Message::CloseGame) => return self.update(Message::CloseGame, ctx),
            Message::Multi(multi::Message::Cancel) => return self.update(Message::Cancel, ctx),
            Message::Multi(multi::Message::Copy(text)) => {
                let task = self.multi.update(multi::Message::Copy(text.clone()), ctx).map(Message::Multi);
                return Task::batch([task, Task::done(Message::Copy(text))]);
            }
            Message::Multi(message) => return self.multi.update(message, ctx).map(Message::Multi),
            Message::Releases | Message::WhatsNew | Message::RestartToInstall => {}
            Message::Play => {
                if Play::of(&self.game) != Play::Ready { return Task::none() }
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.grace += 1;
                self.cancelled = false;
                self.game.starting = Some((Step::Server, Instant::now()));
                self.notice = None;
                self.allow_for = None;
                if let Some(notice) = Notice::foreign_winmm_in(&settings) { self.tell(notice) }
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
                            let needs_permission = settings.direct_start && match backend.auto_direct(&settings) {
                                yo3_desktop::direct::AutoDirect::Ready => false,
                                yo3_desktop::direct::AutoDirect::NeedsPermission => true,
                                yo3_desktop::direct::AutoDirect::Skipped(why) => { backend.diagnostics.line(&format!("Direct start not set up: {why}")); false }
                            };
                            backend.play(&settings, &settings.local_account)?;
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
                        self.direct_permission = needs_permission;
                        if needs_permission {
                            self.tell(Notice { everywhere: self.allow_for.is_some(), ..Notice::problem("Opening the game without the launcher needs permission once, because the game folder is protected.",
                                Some(("Allow", Message::AllowDirect))) });
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
                let server = self.allow_for.clone();
                return job(move || match &server {
                    Some(server) => backend.enable_direct_signed_in(&settings, server),
                    None => backend.enable_direct(&settings),
                }, Message::DirectAllowed);
            }
            Message::DirectAllowed(result) => match result { Ok(()) => self.permission_granted(), Err(error) => self.fail(ctx, error) },
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
                    outside: backend.game_open_elsewhere(&settings), failure: backend.game_failure(), direct: backend.direct_problem() }), Message::Polled);
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
                // A game opened directly that couldn't play (#33): said once, after it closed.
                if let Some(problem) = live.direct && !live.outside && !live.running && let Some(backend) = ctx.backend.clone() {
                    self.tell(Notice::direct(&problem));
                    return job(move || { backend.clear_direct_problem(&problem); Ok(()) }, |_: Result<(), String>| Message::DirectShown);
                }
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
                Ok(()) => self.notice = Some(Notice::good("The backup is back in place. Press PLAY.")),
                Err(error) => self.fail(ctx, error),
            },
            Message::ToggleDetails => if let Some(notice) = &mut self.notice { notice.open = !notice.open },
            Message::Dismiss => self.notice = None,
            Message::DirectShown => {}
            Message::Bell(open) => { self.bell_open = open; if open { self.unread = 0 } }
            Message::InboxDetails(i) => if let Some(n) = self.inbox.get_mut(i) { n.open = !n.open },
            Message::InboxDismiss(i) => if i < self.inbox.len() { self.inbox.remove(i); },
            Message::SetUp | Message::Repair | Message::Restore | Message::Settings(_) | Message::Report(_) | Message::Help(_) | Message::Copy(_) => {}
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
                match backend.verify_setup(&settings) {
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

    /// A notice about an update (one that didn't start, or a failed restart), on both tabs, the reason behind Details.
    pub fn update_notice(&mut self, ctx: &Ctx, summary: String, details: String) {
        if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("{summary} {details}")) }
        let details = error_presentation::present(&details, Hint::General, &crate::app::masks(ctx)).diagnosis.or(Some(details));
        self.tell(Notice { details, everywhere: true, ..Notice::problem(summary, None) });
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
        let action = match hint { Hint::Component => Some(("Repair", Message::Repair)), Hint::GameFolder => Some(("Fix", Message::SetUp)), Hint::General => None };
        self.notice = Some(Notice { details: shown.diagnosis, ..Notice::problem(shown.summary, action) });
    }

    /// Play started a game, or a game opened without the launcher is up.
    pub fn playing(&self) -> bool { self.game.running || self.game.starting.is_some() || self.game.outside }

    /// The banlists to choose from (the server's default first).
    pub fn banlists(&self) -> Vec<Banlist> { banlist_choices(&self.banlists) }

    /// Since when Play has been starting or waiting for the game's window, for Cancel.
    pub fn launching_since(&self) -> Option<Instant> { self.game.launching().or(self.waiting_since) }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick)
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Message> {
        let playing = self.playing();
        let busy = Play::of(&self.game).busy(self.launching_since());
        let side = match self.tab { Tab::Single => self.singleplayer(ctx), Tab::Multi => self.multi.view(busy).map(Message::Multi) };
        let server_news = self.multi.news();
        let news = match self.tab {
            Tab::Single => news(WELCOME.iter().map(|(t, h)| (t.to_string(), h.to_string())).collect()),
            Tab::Multi => news(server_news),
        };
        let panels = row![
            container(yo3_skin::panel(Some("Information"), news)).width(Fill).height(Fill),
            container(column![self.ribbon(playing), side].spacing(8)).width(372).height(Fill),
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
        // Your own server's panel holds its notices; those about both tabs are behind the title bar's bell.
        page.push(panels).into()
    }

    fn singleplayer<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Message> {
        let play = Play::of(&self.game);
        let (label, action, line, kind) = match play.busy(self.launching_since()) {
            Some(busy) => (busy.label, busy.action.map(|a| match a { BusyAction::Close => Message::CloseGame, BusyAction::Cancel => Message::Cancel }), busy.line,
                if busy.action.is_some() { Kind::Primary } else { Kind::Play }),
            None => (play.label(), play.action(), play.status(), Kind::Play),
        };
        let choices = banlist_choices(&self.banlists);
        let selected = choices.iter().find(|c| c.value == ctx.settings.banlist).cloned();
        // The banlist is chosen in Settings → Singleplayer; it is read when your server starts.
        let banlist = text(selected.map_or_else(|| "Season 16 (default)".to_owned(), |b| b.name)).size(15).color(color::INK).width(Fill);
        yo3_skin::panel(Some("Your own server"), column![
            text("Duel the NPCs, open packs and build decks. Cards you earn here stay on this PC.").size(15).color(color::INK),
            row![text("Banlist").font(BODY_BOLD).size(15).color(color::INK).width(74), banlist,
                yo3_skin::link("Change", Message::Settings(crate::settings::Section::Singleplayer), false)].align_y(Center),
            self.notice(),
            Space::new().height(Fill),
            text(line).size(14).color(color::INK_SOFT),
            pill(label, action, kind, Fill, 70),
        ].spacing(14))
    }

    /// "Update X is ready · Restart to install" above PLAY, on both tabs; while the game runs it says when instead.
    fn ribbon(&self, playing: bool) -> Element<'_, Message> {
        let Some(version) = &self.update_ready else { return Space::new().into() };
        let (line, restart) = yo3_desktop::updates::flow::ribbon(version, playing);
        let mut content = row![
            container(Space::new().width(9).height(9)).style(|_| dot(color::WARN)),
            iced::widget::mouse_area(text(line).size(14).font(BODY_BOLD).color(color::ON_DARK)).on_press(Message::WhatsNew)
                .interaction(iced::mouse::Interaction::Pointer),
        ].spacing(8).align_y(Center);
        if restart { content = content.push(yo3_skin::link("Restart to install", Message::RestartToInstall, true)) }
        container(content).padding([7, 12]).width(Fill).style(|_| container::Style {
            background: Some(iced::Background::Gradient(iced::gradient::Linear::new(0.0).add_stop(0.0, color::STRIP_LOW).add_stop(1.0, color::STRIP_TOP).into())),
            border: iced::Border { color: color::WARN, width: 1.5, radius: 4.0.into() }, ..container::Style::default() }).into()
    }

    fn notice(&self) -> Element<'_, Message> {
        let Some(notice) = &self.notice else { return Space::new().into() };
        notice_view(notice, Message::ToggleDetails, Message::Dismiss)
    }

    /// The bell's list: each notice with its links, or a line saying there is nothing.
    pub fn inbox_view(&self) -> Element<'_, Message> {
        let mut list = column![text("Notifications").size(16).font(yo3_skin::DISPLAY).color(color::NAVY)].spacing(10);
        if self.inbox.is_empty() { list = list.push(text("Nothing new.").size(14).color(color::INK_SOFT)) }
        for (i, notice) in self.inbox.iter().enumerate() {
            list = list.push(container(notice_view(notice, Message::InboxDetails(i), Message::InboxDismiss(i))).padding([8, 10]).width(Fill)
                .style(|_| container::Style { background: Some(color::ROW.into()), border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }));
        }
        container(iced::widget::scrollable(list).height(iced::Length::Shrink)).padding(12).width(380).max_height(420).style(|_| container::Style {
            background: Some(color::PANEL_TOP.into()), border: iced::Border { color: color::CHROME_EDGE, width: 1.0, radius: 4.0.into() },
            shadow: iced::Shadow { color: iced::Color { a: 0.45, ..iced::Color::BLACK }, offset: iced::Vector::new(0.0, 4.0), blur_radius: 12.0 },
            ..container::Style::default() }).into()
    }
}

/// A notice: its sentence, then its links: its own action (or Report), Help, Details, Dismiss.
fn notice_view(notice: &Notice, details: Message, dismiss: Message) -> Element<'_, Message> {
    {
        let mut links = row![].spacing(14);
        match &notice.action {
            Some((label, action)) => links = links.push(yo3_skin::link(*label, action.clone(), false)),
            None if !notice.good => links = links.push(yo3_skin::link("Report", Message::Report(notice.details.clone().unwrap_or_else(|| notice.summary.clone())), false)),
            None => {}
        }
        if !notice.good { links = links.push(yo3_skin::link("Help", Message::Help(crate::help::topic_for(&notice.summary)), false)) }
        if notice.details.is_some() {
            links = links.push(yo3_skin::link(if notice.open { "Hide details" } else { "Details" }, details, false));
        }
        links = links.push(yo3_skin::link("Dismiss", dismiss, false));
        let shade = if notice.good { color::GOOD } else { color::BAD };
        let mut body = column![
            row![container(Space::new().width(9).height(9)).style(move |_| dot(shade)), text(&notice.summary).size(14).font(BODY_BOLD).color(color::INK).width(Fill)]
                .spacing(8).align_y(Center),
        ].spacing(6);
        if notice.open && let Some(details) = &notice.details {
            body = body.push(text(details).size(13).color(color::INK_SOFT)).push(yo3_skin::link("Copy", Message::Copy(details.clone()), false));
        }
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

    /// While the game is busy the big button follows it on either tab: CLOSE GAME while it is open, CANCEL once a start
    /// hangs, a disabled label otherwise; never a disabled PLAYING with a small link beside it.
    #[test]
    fn the_big_button_closes_the_game_and_cancels_a_hung_start() {
        let now = Instant::now();
        let long_ago = now.checked_sub(CANCEL_AFTER + Duration::from_secs(1)).unwrap();
        let busy = |play: Play, since| play.busy(since).map(|b| (b.label, b.action));
        assert_eq!(busy(Play::Playing, None), Some(("CLOSE GAME", Some(BusyAction::Close))));
        assert_eq!(busy(Play::Waiting, Some(now)), Some(("CLOSE GAME", Some(BusyAction::Close))));
        assert_eq!(busy(Play::Waiting, Some(long_ago)), Some(("CANCEL", Some(BusyAction::Cancel))));
        assert_eq!(busy(Play::StartingGame, Some(now)), Some(("STARTING…", None)));
        assert_eq!(busy(Play::StartingServer, Some(long_ago)), Some(("CANCEL", Some(BusyAction::Cancel))));
        assert_eq!(busy(Play::Outside, None), Some(("PLAYING", None)), "a game opened elsewhere isn't ours to close");
        assert_eq!(busy(Play::Closing, None), Some(("CLOSING…", None)));
        for idle in [Play::Ready, Play::Checking, Play::SetUp, Play::Repair, Play::Retry] { assert_eq!(busy(idle, None), None) }
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

    /// Settings' Fix permission (or the notice's Allow) sets direct start up: the main screen stops asking for permission.
    /// On Windows 11 a Multiplayer server's refused direct start was said only on the Singleplayer panel, while the player
    /// looked at Multiplayer: shown once there, unseen, and gone. A direct start's problem shows on either tab.
    #[test]
    fn a_direct_starts_problem_shows_on_either_tab() {
        let problem = DirectProblem { refusal: "FailedToConnect".into(), reason: "refused".into(), at: "now".into() };
        let mut state = State::default();
        state.tell(Notice::direct(&problem));
        assert!(state.notice.is_none(), "not on your own server's panel: the game opened directly may have played either");
        assert_eq!((state.inbox.len(), state.unread), (1, 1), "behind the bell, counted");
        assert_eq!(state.inbox[0].details.as_deref(), Some("refused"));
        state.tell(Notice::direct(&problem));
        assert_eq!(state.inbox.len(), 1, "the same notice once");
        let _ = state.update(Message::Bell(true), &mut crate::app::Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() });
        assert_eq!(state.unread, 0, "opening the bell reads them");
        state.tell(Notice::problem("Your singleplayer data couldn't be opened.", None));
        assert!(state.notice.is_some() && state.inbox.len() == 1, "your own server's notices stay on its panel");
    }

    #[test]
    fn multiplayers_play_asks_for_permission_on_its_own_tab_too() {
        let mut ctx = crate::app::Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() };
        let mut state = State { allow_for: Some(yo3_desktop::multiplayer::Server { name: "Friends".into(), address: "play.example.com".into(),
            server_name: "Friends".into(), identities: vec![], official: false, description: String::new(), saved_user: None, last_used: 0 }),
            ..State::default() };
        let _ = state.update(Message::Started(Ok(true)), &mut ctx);
        assert!(state.direct_permission);
        let notice = state.inbox.first().expect("the Allow notice, behind the bell");
        assert!(matches!(notice.action, Some((_, Message::AllowDirect))));
        assert!(state.notice.is_none() && state.unread == 1, "a Multiplayer Play's notice is counted on the bell, seen from either tab");
        state.permission_granted();
        assert!(state.inbox.is_empty() && state.unread == 0, "allowed: the notice goes");
        // Singleplayer's stays on its own panel.
        let mut state = State::default();
        let _ = state.update(Message::Started(Ok(true)), &mut ctx);
        assert!(state.notice.is_some() && state.inbox.is_empty());
    }

    #[test]
    fn granting_permission_ends_the_permission_notice_and_row() {
        let mut state = State { direct_permission: true, ..State::default() };
        state.notice = Some(Notice::problem("needs permission", Some(("Allow", Message::AllowDirect))));
        state.permission_granted();
        assert!(!state.direct_permission);
        assert!(state.notice.is_none(), "the Allow notice goes");
        // Another notice (an error from something else) stays.
        let mut state = State { direct_permission: true, ..State::default() };
        state.notice = Some(Notice::problem("something else", None));
        state.permission_granted();
        assert!(state.notice.is_some());
    }

    #[test]
    fn the_default_banlist_comes_first_and_ids_are_saved() {
        let choices = banlist_choices(&[(4, "Season 13".into())]);
        assert_eq!(choices[0].value, "");
        assert_eq!(choices[1], Banlist { value: "4".into(), name: "Season 13".into() });
    }
}
