//! The main screen: Singleplayer (your own server on this computer) and Multiplayer, around one Play button that
//! always says what happens next (#29 has the full table; this is the part that works without accounts yet).
use std::time::Duration;
use iced::widget::{Space, column, container, pick_list, row, text};
use iced::{Alignment::Center, Element, Length::Fill, Subscription, Task};
use yo3_desktop::Readiness;
use yo3_skin::{BODY_BOLD, Kind, color, pill};
use crate::app::{Ctx, dot, job};
use crate::error_presentation::{self, Hint};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab { #[default] Single, Multi }

/// What the launcher knows about the game right now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    /// The last check of the game folder and our files; `None` while it runs.
    pub readiness: Option<Readiness>,
    /// Play was pressed and the game has not been seen yet.
    pub starting: bool,
    pub running: bool,
    /// The game's window is up (the connection DLL said so).
    pub ready: bool,
    pub closing: bool,
}

/// The Play button's state: its label, what a click does and the line above it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Play { Checking, SetUp, Repair, Retry, Ready, Starting, Waiting, Playing, Closing }

impl Play {
    pub fn of(game: &Game) -> Self {
        if game.closing { return Play::Closing }
        if game.starting { return Play::Starting }
        if game.running { return if game.ready { Play::Playing } else { Play::Waiting } }
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
            Play::Starting | Play::Waiting => "STARTING…", Play::Playing => "PLAYING", Play::Closing => "CLOSING…",
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
            Play::Starting => "Starting your server and the game…",
            Play::Waiting => "Waiting for the game's window…",
            Play::Playing => "The game is running.",
            Play::Closing => "Closing the game…",
        }
    }
}

/// One plain sentence, with the technical text behind Details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice { pub summary: String, pub details: Option<String>, pub open: bool }

/// A banlist choice: the value saved for the server, the name shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banlist { pub value: String, pub name: String }
impl std::fmt::Display for Banlist { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.name) } }

#[derive(Clone, Debug)]
pub struct Live { running: bool, ready: bool, failure: Option<String> }

#[derive(Clone, Debug)]
pub enum Message {
    Tab(Tab),
    Play,
    Started(Result<(), String>),
    CloseGame,
    GameClosed(Result<(), String>),
    Banlist(Banlist),
    BanlistSaved(Result<String, String>),
    Banlists(Vec<(u32, String)>),
    Tick,
    Polled(Result<Live, String>),
    Recheck,
    Checked(Readiness),
    /// Handled by the window: open the game folder screen.
    SetUp,
    /// Handled by the window: explain the repair.
    Repair,
    ToggleDetails,
    Dismiss,
}

#[derive(Default)]
pub struct State {
    pub tab: Tab,
    pub game: Game,
    banlists: Vec<(u32, String)>,
    notice: Option<Notice>,
    polling: bool,
}

impl State {
    pub fn update(&mut self, message: Message, ctx: &mut Ctx) -> Task<Message> {
        match message {
            Message::Tab(tab) => self.tab = tab,
            Message::Play => {
                if Play::of(&self.game) != Play::Ready { return Task::none() }
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let settings = ctx.settings.clone();
                self.game.starting = true;
                self.notice = None;
                return job(move || {
                    backend.start_server(&settings)?;
                    // Opening yo3.exe without the launcher keeps working with your own server.
                    if let yo3_desktop::direct::AutoDirect::Skipped(why) = backend.auto_direct(&settings, None) {
                        backend.diagnostics.line(&format!("Direct start not set up: {why}"));
                    }
                    backend.play(&settings, None, &settings.local_account)
                }, Message::Started);
            }
            Message::Started(result) => {
                self.game.starting = false;
                match result {
                    Ok(()) => { self.game.running = true; self.game.ready = false }
                    Err(error) => { self.fail(ctx, error); return self.check(ctx) }
                }
            }
            Message::CloseGame => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                self.game.closing = true;
                return job(move || backend.stop_game(), Message::GameClosed);
            }
            Message::GameClosed(result) => {
                self.game.closing = false;
                if let Err(error) = result { self.fail(ctx, error) }
            }
            Message::Banlist(choice) => {
                let Some(store) = ctx.store.clone() else { return Task::none() };
                let value = choice.value;
                return Task::perform(async move { store.run(move |db| db.save_banlist(&value)).await.map_err(|e| format!("{e:#}")) }, Message::BanlistSaved);
            }
            Message::BanlistSaved(Ok(value)) => ctx.settings.banlist = value,
            Message::BanlistSaved(Err(error)) => self.fail(ctx, error),
            Message::Banlists(list) => self.banlists = list,
            Message::Tick => {
                if self.polling || self.game.starting { return Task::none() }
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                self.polling = true;
                return job(move || Ok(Live { running: backend.game_running(), ready: backend.game_ready(), failure: backend.game_failure() }), Message::Polled);
            }
            Message::Polled(result) => {
                self.polling = false;
                let Ok(live) = result else { return Task::none() };
                let ended = self.game.running && !live.running;
                if self.game.starting { return Task::none() }
                self.game.running = live.running;
                self.game.ready = live.ready;
                if let Some(failure) = live.failure { self.fail(ctx, failure) }
                // The game folder may have changed while it ran (an update, an antivirus): check again for the next Play.
                if ended { return self.check(ctx) }
            }
            Message::Recheck => return self.check(ctx),
            Message::Checked(readiness) => self.game.readiness = Some(readiness),
            Message::ToggleDetails => if let Some(notice) = &mut self.notice { notice.open = !notice.open },
            Message::Dismiss => self.notice = None,
            Message::SetUp | Message::Repair => {}
        }
        Task::none()
    }

    /// Checks the game folder and YO3 Revival's own files in the background.
    pub fn check(&mut self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        if self.game.running || self.game.starting { return Task::none() }
        self.game.readiness = None;
        let settings = ctx.settings.clone();
        Task::perform(async move {
            tokio::task::spawn_blocking(move || match backend.verify_setup(&settings, None) {
                Ok(()) => Readiness::Ready,
                Err(error) => match backend.readiness(&settings) { Readiness::Ready => Readiness::ValidationProblem(format!("{error:#}")), problem => problem },
            }).await.unwrap_or_else(|error| Readiness::ValidationProblem(error.to_string()))
        }, Message::Checked)
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
        self.notice = Some(Notice { summary: shown.summary, details: shown.diagnosis, open: false });
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick)
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Message> {
        let side = match self.tab { Tab::Single => self.singleplayer(ctx), Tab::Multi => multiplayer() };
        let news = match self.tab { Tab::Single => news(&WELCOME), Tab::Multi => news(&MULTIPLAYER_NEWS) };
        row![
            container(yo3_skin::panel(Some("Information"), news)).width(Fill).height(Fill),
            container(side).width(372).height(Fill),
        ].spacing(16).padding([16, 18]).height(Fill).into()
    }

    fn singleplayer<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Message> {
        let play = Play::of(&self.game);
        let mut status = row![text(play.status()).size(14).color(color::INK_SOFT)].spacing(10).align_y(Center).height(22);
        if play == Play::Playing { status = status.push(Space::new().width(Fill)).push(yo3_skin::link("Close game", Message::CloseGame, false)) }
        let choices = banlist_choices(&self.banlists);
        let selected = choices.iter().find(|c| c.value == ctx.settings.banlist).cloned();
        let editable = !self.game.running && !self.game.starting;
        let banlist = pick_list(choices, selected, Message::Banlist).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu)
            .width(Fill).padding([7, 10]).placeholder("Season 16");
        let banlist: Element<'_, Message> = if editable { banlist.into() } else { container(banlist).into() };
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
        let mut links = row![yo3_skin::link("Dismiss", Message::Dismiss, false)].spacing(14);
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

/// The banlists to choose from: the server's default first, then the game's own lists.
fn banlist_choices(lists: &[(u32, String)]) -> Vec<Banlist> {
    let mut choices = vec![Banlist { value: String::new(), name: "Season 16 (default)".into() }];
    choices.extend(lists.iter().map(|(id, name)| Banlist { value: id.to_string(), name: name.clone() }));
    choices
}

fn multiplayer<'a>() -> Element<'a, Message> {
    yo3_skin::panel(Some("Server"), column![
        text("Multiplayer opens with the beta's accounts: a User ID and password for each server.").size(15).color(color::INK),
        text("Until then, play on your own server in Singleplayer.").size(15).color(color::INK_SOFT),
        Space::new().height(Fill),
        pill("LOG IN", None, Kind::Play, Fill, 70),
    ].spacing(12))
}

const WELCOME: [(&str, &str); 3] = [
    ("Beta", "Welcome to the new YO3 Revival launcher"),
    ("Beta", "Your own server starts when you press Play"),
    ("Beta", "Multiplayer accounts are on their way"),
];
const MULTIPLAYER_NEWS: [(&str, &str); 1] = [("Beta", "The server list and accounts come in a later beta version")];

fn news<'a>(items: &'a [(&'a str, &'a str)]) -> Element<'a, Message> {
    let mut list = column![];
    for (i, (tag, headline)) in items.iter().enumerate() {
        let shade = if i % 2 == 0 { color::ROW } else { color::ROW_ALT };
        list = list.push(container(row![
            text(*tag).font(BODY_BOLD).size(15).color(color::NAVY).width(54),
            text(*headline).size(15).color(color::INK),
        ].align_y(Center)).padding([9, 12]).width(Fill)
            .style(move |_| container::Style { background: Some(shade.into()), ..container::Style::default() }));
    }
    container(list).style(|_| container::Style { border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 2.0.into() }, ..container::Style::default() }).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(readiness: Option<Readiness>) -> Game { Game { readiness, ..Game::default() } }

    #[test]
    fn the_play_button_always_says_what_happens_next() {
        assert_eq!(Play::of(&game(None)), Play::Checking);
        assert_eq!(Play::of(&game(Some(Readiness::Ready))), Play::Ready);
        assert_eq!(Play::of(&game(Some(Readiness::GameProblem("no yo3.exe".into())))), Play::SetUp);
        assert_eq!(Play::of(&game(Some(Readiness::ComponentProblem("no dll".into())))), Play::Repair);
        assert_eq!(Play::of(&game(Some(Readiness::ValidationProblem("timeout".into())))), Play::Retry);
        let mut playing = game(Some(Readiness::Ready));
        playing.starting = true;
        assert_eq!(Play::of(&playing), Play::Starting);
        playing.starting = false; playing.running = true;
        assert_eq!(Play::of(&playing), Play::Waiting);
        playing.ready = true;
        assert_eq!(Play::of(&playing), Play::Playing);
        playing.closing = true;
        assert_eq!(Play::of(&playing), Play::Closing);
    }

    #[test]
    fn only_states_with_a_next_step_are_clickable_and_every_disabled_one_explains_itself() {
        for play in [Play::Checking, Play::SetUp, Play::Repair, Play::Retry, Play::Ready, Play::Starting, Play::Waiting, Play::Playing, Play::Closing] {
            let clickable = play.action().is_some();
            assert_eq!(clickable, matches!(play, Play::SetUp | Play::Repair | Play::Retry | Play::Ready), "{play:?}");
            assert!(play == Play::Ready || !play.status().is_empty(), "{play:?} needs a line saying why");
        }
    }

    #[test]
    fn the_default_banlist_comes_first_and_ids_are_saved() {
        let choices = banlist_choices(&[(4, "Season 13".into())]);
        assert_eq!(choices[0].value, "");
        assert_eq!(choices[1], Banlist { value: "4".into(), name: "Season 13".into() });
    }
}
