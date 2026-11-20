//! The setup wizard (#28): Welcome, Game, Ready. It opens on the first run and again from Settings, or from the main
//! screen's "We can't find your game" banner (then straight at the Game step).
use std::time::Duration;
use iced::widget::{Space, checkbox, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Alignment::Center, Color, Element, Length::Fill, Task};
use yo3_desktop::{Settings, direct};
use yo3_desktop::setup_check::{self, Candidate, Check, Line, State as Outcome};
use yo3_skin::{BODY_BOLD, DISPLAY_ITALIC, Kind, color, pill};
use crate::app::{Ctx, dot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step { Welcome, Game, Ready }

#[derive(Clone, Debug)]
pub enum Message {
    Next,
    Back,
    Candidates(Vec<Candidate>),
    Pick(usize),
    Path(String),
    Prefix(String),
    Browse { prefix: bool },
    Picked { prefix: bool, path: Option<String> },
    MoreOptions,
    CheckDue(u64),
    Retry,
    Checked(u64, Vec<Line>),
    Details(Check),
    Direct(bool),
    DirectDone(bool, Result<(), String>),
    Shortcut(bool),
    Done,
    Saved(Result<(Box<Settings>, Option<String>), String>),
    Cancel,
    /// Handled by the window: open a web page.
    Open(&'static str),
    /// Handled by the window: explain the repair.
    Repair,
    ErrorDetails,
}

pub struct State {
    step: Step,
    candidates: Vec<Candidate>,
    path: String,
    prefix: String,
    more_options: bool,
    revision: u64,
    lines: Option<Vec<Line>>,
    details: Option<Check>,
    direct: bool,
    /// The tick is being set up (Windows may be asking for permission).
    asking: bool,
    /// What the last press of the tick came to, when it isn't on: refused, or why it couldn't.
    direct_note: Option<String>,
    shortcut: bool,
    picking: bool,
    saving: bool,
    error: Option<String>,
    error_details: Option<String>,
    error_open: bool,
    /// Opened from the main screen, so Cancel goes back to it.
    cancellable: bool,
}

pub const GETTING_THE_GAME: &str = "https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md#getting-the-original-game";

impl State {
    /// A wizard starting at `step`, with the game folder and prefix from `ctx`'s settings.
    pub fn new(ctx: &Ctx, step: Step, cancellable: bool) -> Self {
        Self { step, candidates: Vec::new(), path: ctx.settings.game_dir.clone(), prefix: ctx.settings.wine_prefix.clone(),
            more_options: !ctx.settings.wine_prefix.is_empty(), revision: 0, lines: None, details: None,
            direct: ctx.settings.direct_start && ctx.settings.setup_complete, asking: false, direct_note: None, shortcut: !ctx.settings.setup_complete,
            picking: false, saving: false, error: None, error_details: None, error_open: false, cancellable }
    }

    /// The first tasks: find installed copies, and check the folder when the wizard opens at the Game step.
    pub fn start(&mut self, ctx: &Ctx) -> Task<Message> {
        let find = Task::perform(async { tokio::task::spawn_blocking(setup_check::candidates).await.unwrap_or_default() }, Message::Candidates);
        if self.step == Step::Game { Task::batch([find, self.check(ctx, Duration::ZERO)]) } else { find }
    }

    pub fn update(&mut self, message: Message, ctx: &Ctx) -> Task<Message> {
        match message {
            Message::Next => match self.step {
                Step::Welcome => { self.step = Step::Game; return self.check(ctx, Duration::ZERO) }
                Step::Game if self.can_continue() => self.step = Step::Ready,
                _ => {}
            },
            Message::Back => self.step = match self.step { Step::Ready => Step::Game, _ => Step::Welcome },
            Message::Candidates(found) => {
                // Nothing typed yet, or the saved folder is gone: take the first copy found.
                let current_missing = !std::path::Path::new(self.path.trim()).join("yo3.exe").is_file();
                self.candidates = found;
                if current_missing && let Some(first) = self.candidates.first().cloned() {
                    self.use_candidate(&first);
                    if self.step == Step::Game { return self.check(ctx, Duration::ZERO) }
                }
            }
            Message::Pick(i) => if let Some(candidate) = self.candidates.get(i).cloned() { self.use_candidate(&candidate); return self.check(ctx, Duration::ZERO) },
            Message::Path(path) => { self.path = path; return self.check(ctx, Duration::from_millis(600)) }
            Message::Prefix(prefix) => { self.prefix = prefix; return self.check(ctx, Duration::from_millis(600)) }
            Message::Browse { prefix } => {
                if self.picking { return Task::none() }
                self.picking = true;
                let title = if prefix { "Choose the Wine prefix" } else { "Choose the Yu-Gi-Oh! ONLINE 3 folder" };
                return Task::perform(async move {
                    let picked = tokio::time::timeout(Duration::from_secs(300), rfd::AsyncFileDialog::new().set_title(title).pick_folder()).await;
                    picked.ok().flatten().map(|f| f.path().to_string_lossy().into_owned())
                }, move |path| Message::Picked { prefix, path });
            }
            Message::Picked { prefix, path } => {
                self.picking = false;
                if let Some(path) = path {
                    if prefix { self.prefix = path } else { self.path = path }
                    return self.check(ctx, Duration::ZERO);
                }
            }
            Message::MoreOptions => self.more_options = !self.more_options,
            Message::CheckDue(revision) => if revision == self.revision { return self.run_check(ctx) },
            Message::Retry => return self.check(ctx, Duration::ZERO),
            Message::Checked(revision, lines) => if revision == self.revision {
                if read_only(&lines) { self.direct = false }
                self.lines = Some(lines);
            },
            Message::Details(check) => self.details = if self.details == Some(check) { None } else { Some(check) },
            Message::Direct(on) => return self.set_direct(ctx, on),
            Message::DirectDone(on, result) => {
                self.asking = false;
                match result {
                    Ok(()) => { self.direct = on; self.direct_note = None }
                    Err(error) => {
                        if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("Direct start in setup: {error}")) }
                        self.direct_note = Some(if direct::was_refused(&error) { "Windows' permission wasn't given, so it stays off. You can turn it on later in Settings → Game.".into() }
                            else { format!("It couldn't be set up, so it stays off: {}", crate::error_presentation::plain(error.lines().next().unwrap_or_default())) });
                    }
                }
            }
            Message::Shortcut(on) => self.shortcut = on,
            Message::Done => return self.finish(ctx),
            Message::Saved(Err(error)) => {
                self.saving = false;
                if let Some(backend) = &ctx.backend { backend.diagnostics.line(&error) }
                let shown = crate::error_presentation::present(&error, crate::error_presentation::Hint::General, &crate::app::masks(ctx));
                self.error = Some(shown.summary);
                self.error_details = shown.diagnosis;
                self.error_open = false;
            }
            Message::ErrorDetails => self.error_open = !self.error_open,
            Message::Saved(Ok(_)) | Message::Cancel | Message::Open(_) | Message::Repair => {}
        }
        Task::none()
    }

    fn use_candidate(&mut self, candidate: &Candidate) {
        self.path = candidate.game_dir.to_string_lossy().into_owned();
        // A copy found in a non-default prefix brings its prefix along (Linux).
        let default = std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".wine"));
        self.prefix = candidate.wine_prefix.as_ref().filter(|p| Some(p.as_path()) != default.as_deref())
            .map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        if !self.prefix.is_empty() { self.more_options = true }
    }

    fn settings(&self, ctx: &Ctx) -> Settings {
        let mut settings = ctx.settings.clone();
        settings.game_dir = self.path.trim().to_owned();
        settings.wine_prefix = self.prefix.trim().to_owned();
        settings
    }

    /// Checks the folder after `delay`; typing more within the delay starts over, and only the latest answer counts.
    fn check(&mut self, ctx: &Ctx, delay: Duration) -> Task<Message> {
        self.revision += 1;
        self.lines = None;
        self.details = None;
        let revision = self.revision;
        if delay.is_zero() { return self.run_check(ctx) }
        Task::perform(tokio::time::sleep(delay), move |()| Message::CheckDue(revision))
    }

    fn run_check(&self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        let settings = self.settings(ctx);
        let revision = self.revision;
        Task::perform(async move {
            tokio::task::spawn_blocking(move || backend.setup_checklist(&settings)).await.unwrap_or_default()
        }, move |lines| Message::Checked(revision, lines))
    }

    fn can_continue(&self) -> bool { self.lines.as_deref().is_some_and(setup_check::passes) }

    /// The tick "Open the game without the launcher": pressing it sets it up right away, with Windows' permission prompt
    /// when the game folder is protected (the player's request: asked once, here, never again at each Play). The folder
    /// is saved first, so Done finds nothing to move. Unticking takes it out again.
    fn set_direct(&mut self, ctx: &Ctx, on: bool) -> Task<Message> {
        if self.asking { return Task::none() }
        let Some(backend) = ctx.backend.clone() else { self.direct = on; return Task::none() };
        let draft = self.settings(ctx);
        self.asking = true;
        self.direct_note = None;
        crate::app::job(move || {
            let saved = backend.save_preferences(yo3_desktop::preferences::Section::Game, &draft)?;
            if on { backend.turn_on_direct(&saved) }
            else if backend.direct_state(&saved) != direct::State::Disabled { backend.disable_direct(&saved) } else { Ok(()) }
        }, move |result| Message::DirectDone(on, result))
    }

    /// Saves the choices, then sets up what was ticked. Direct start was set up when its tick was pressed.
    fn finish(&mut self, ctx: &Ctx) -> Task<Message> {
        if self.saving { return Task::none() }
        let (Some(backend), Some(store)) = (ctx.backend.clone(), ctx.store.clone()) else { return Task::none() };
        let draft = self.settings(ctx);
        let (direct, shortcut) = (self.direct, self.shortcut);
        self.saving = true;
        self.error = None;
        Task::perform(async move {
            store.run(move |db| {
                let mut settings = backend.save_preferences(yo3_desktop::preferences::Section::Game, &draft)?;
                settings.setup_complete = true;
                settings.direct_start = direct;
                settings.game_fingerprint = setup_check::fingerprint(std::path::Path::new(&settings.game_dir))?;
                db.save_settings(&settings)?;
                if !direct && backend.direct_state(&settings) != yo3_desktop::direct::State::Disabled {
                    backend.disable_direct(&settings)?;
                }
                // A shortcut that couldn't be made doesn't undo setup; the main screen says so.
                let note = if shortcut {
                    std::env::current_exe().map_err(anyhow::Error::from).and_then(|exe| yo3_desktop::shortcut::create(&exe)).err()
                        .map(|e| format!("The desktop shortcut couldn't be made: {e:#}")) // → Details
                } else { None };
                Ok((Box::new(settings), note))
            }).await.map_err(|e| format!("{e:#}"))
        }, Message::Saved)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = match self.step { Step::Welcome => self.welcome(), Step::Game => self.game(), Step::Ready => self.ready() };
        let mut buttons = row![].spacing(10).align_y(Center);
        if self.cancellable { buttons = buttons.push(pill("Cancel", Some(Message::Cancel), Kind::Secondary, 110, 38)) }
        buttons = buttons.push(Space::new().width(Fill));
        if self.step != Step::Welcome { buttons = buttons.push(pill("Back", Some(Message::Back), Kind::Secondary, 110, 38)) }
        buttons = buttons.push(match self.step {
            Step::Welcome => pill("Next", Some(Message::Next), Kind::Primary, 110, 38),
            Step::Game => pill("Next", self.can_continue().then_some(Message::Next), Kind::Primary, 110, 38),
            Step::Ready => pill(if self.saving { "Saving…" } else { "Done" }, (!self.saving && !self.asking).then_some(Message::Done), Kind::Primary, 110, 38),
        });
        let header = row![Space::new().width(Fill), self.steps()].align_y(Center);
        let panel = yo3_skin::panel(Some("Setup"), column![header, container(body).height(Fill), buttons].spacing(10));
        container(container(panel).width(700).height(500)).center(Fill).into()
    }

    fn steps(&self) -> Element<'_, Message> {
        let mark = |n: &'static str, label: &'static str, step: Step| {
            let (fill, ink) = if self.step == step { (color::NAVY, Color::WHITE) } else if self.step > step { (color::CHROME_DARK, Color::WHITE) } else { (color::ROW_ALT, color::INK_SOFT) };
            row![
                container(text(n).size(13).font(BODY_BOLD).color(ink)).center_x(22).center_y(22)
                    .style(move |_| container::Style { background: Some(fill.into()), border: iced::Border { radius: 11.0.into(), ..iced::Border::default() }, ..container::Style::default() }),
                text(label).size(14).font(BODY_BOLD).color(if self.step == step { color::INK } else { color::INK_SOFT }),
            ].spacing(6).align_y(Center)
        };
        row![mark("1", "Welcome", Step::Welcome), mark("2", "Game", Step::Game), mark("3", "Ready", Step::Ready)].spacing(18).into()
    }

    fn welcome(&self) -> Element<'_, Message> {
        column![
            text("Welcome to YO3 Revival").font(DISPLAY_ITALIC).size(28).color(color::NAVY),
            text("YO3 Revival brings Yu-Gi-Oh! ONLINE 3 back: your own server for duels against the NPCs, and servers to play with friends.").size(15).color(color::INK),
            text("You need your own copy of the game, installed on this computer. YO3 Revival never ships or downloads it.").size(15).color(color::INK),
            yo3_skin::link("Where to get the original game", Message::Open(GETTING_THE_GAME), false),
        ].spacing(14).padding([10, 0]).into()
    }

    fn game(&self) -> Element<'_, Message> {
        let mut found = column![].spacing(4);
        for (i, candidate) in self.candidates.iter().enumerate() {
            let chosen = candidate.game_dir.to_string_lossy() == self.path.trim();
            found = found.push(mouse_area(container(row![
                container(Space::new().width(10).height(10)).style(move |_| dot(if chosen { color::NAVY } else { Color::WHITE })),
                text(candidate.game_dir.to_string_lossy().into_owned()).size(14).color(color::INK),
            ].spacing(8).align_y(Center)).padding([5, 8]).width(Fill)
                .style(move |_| container::Style { background: Some((if chosen { color::ROW } else { color::PANEL_TOP }).into()), ..container::Style::default() }))
                .on_press(Message::Pick(i)).interaction(iced::mouse::Interaction::Pointer));
        }
        let folder = row![
            text_input("The folder with yo3.exe", &self.path).on_input(Message::Path).style(yo3_skin::field).padding(8).size(14),
            pill("Browse…", (!self.picking).then_some(Message::Browse { prefix: false }), Kind::Secondary, 110, 36),
        ].spacing(10).align_y(Center);
        let mut column = column![text("Where is Yu-Gi-Oh! ONLINE 3 installed?").size(15).font(BODY_BOLD).color(color::INK)].spacing(8);
        if self.candidates.is_empty() {
            column = column.push(row![text("No installed copy was found automatically.").size(14).color(color::INK_SOFT),
                yo3_skin::link("Getting the game", Message::Open(GETTING_THE_GAME), false)].spacing(10));
        } else {
            column = column.push(found);
        }
        column = column.push(folder);
        if cfg!(not(windows)) {
            column = column.push(yo3_skin::link(if self.more_options { "Fewer options" } else { "More options" }, Message::MoreOptions, false));
            if self.more_options {
                column = column.push(row![
                    text("Wine prefix").size(14).font(BODY_BOLD).color(color::INK).width(90),
                    text_input("Wine's default prefix", &self.prefix).on_input(Message::Prefix).style(yo3_skin::field).padding(7).size(14),
                    pill("Browse…", (!self.picking).then_some(Message::Browse { prefix: true }), Kind::Secondary, 110, 34),
                ].spacing(10).align_y(Center));
            }
        }
        column = column.push(self.checklist());
        scrollable(column.padding(iced::Padding { right: 12.0, ..iced::Padding::ZERO })).height(Fill).into()
    }

    fn checklist(&self) -> Element<'_, Message> {
        let Some(lines) = &self.lines else {
            return yo3_skin::progress_bar(yo3_skin::Bar::Starting, "Checking this folder…", 22.0);
        };
        let mut list = column![].spacing(6);
        for line in lines {
            // YO3 Revival's own files are checked silently: shown only when they need repair.
            if line.check == Check::Ours && line.state != Outcome::Fail { continue }
            let tone = match line.state { Outcome::Pass => color::GOOD, Outcome::Warn => color::WARN, Outcome::Fail => color::BAD, Outcome::Skipped => color::CHROME_DARK };
            let title = if line.check == Check::Ours { "YO3 Revival needs repair" } else { line.check.title() };
            let mut head = row![container(Space::new().width(9).height(9)).style(move |_| dot(tone)), text(title).size(14).font(BODY_BOLD).color(color::INK)].spacing(8).align_y(Center);
            if line.details.is_some() && line.state != Outcome::Pass {
                head = head.push(yo3_skin::link(if self.details == Some(line.check) { "Hide details" } else { "Details" }, Message::Details(line.check), false));
            }
            if line.check == Check::Ours { head = head.push(yo3_skin::link("Repair", Message::Repair, false)) }
            let mut entry = column![head].spacing(2);
            if !line.sentence.is_empty() && line.state != Outcome::Pass && line.check != Check::Ours {
                entry = entry.push(text(&line.sentence).size(13).color(color::INK_SOFT));
            }
            if self.details == Some(line.check) && let Some(details) = &line.details { entry = entry.push(text(details).size(12).color(color::INK_SOFT)) }
            list = list.push(entry);
        }
        if lines.iter().any(|l| l.state == Outcome::Fail) { list = list.push(pill("Retry", Some(Message::Retry), Kind::Primary, 110, 34)) }
        list.into()
    }

    fn ready(&self) -> Element<'_, Message> {
        let read_only = self.lines.as_deref().is_some_and(read_only);
        let direct = checkbox(self.direct).label("Open the game without the launcher").style(yo3_skin::tick).size(16).text_size(15);
        let direct = if read_only || self.asking || self.saving { direct } else { direct.on_toggle(Message::Direct) };
        let explain = if read_only { "The game folder is read-only, so the game opens only from YO3 Revival." }
            else if self.asking { "Setting it up… Windows may ask for permission." }
            else { "Start yo3.exe on its own and it still signs you in. Optional: ticking it may ask Windows for permission once." };
        let mut list = column![
            text("YO3 Revival is ready.").font(DISPLAY_ITALIC).size(26).color(color::NAVY),
            direct,
            text(explain).size(13).color(color::INK_SOFT),
        ].spacing(10).padding([10, 0]);
        if let Some(note) = &self.direct_note { list = list.push(text(note).size(13).color(color::INK)) }
        list = list.push(checkbox(self.shortcut).label("Put a YO3 Revival shortcut on the desktop").on_toggle(Message::Shortcut).style(yo3_skin::tick).size(16).text_size(15));
        if let Some(error) = &self.error {
            list = list.push(text(error).size(14).font(BODY_BOLD).color(color::BAD));
            if let Some(details) = &self.error_details {
                list = list.push(yo3_skin::link(if self.error_open { "Hide details" } else { "Details" }, Message::ErrorDetails, false));
                if self.error_open { list = list.push(text(details).size(13).color(color::INK_SOFT)) }
            }
        }
        list.into()
    }
}

/// The checklist found a folder that can't be written to at all: direct start stays off.
fn read_only(lines: &[Line]) -> bool {
    lines.iter().any(|l| l.check == Check::DirectStart && l.state == Outcome::Warn && l.sentence.contains("read-only"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State { step: Step::Game, candidates: vec![], path: "/games/yo3".into(), prefix: String::new(), more_options: false, revision: 3,
            lines: None, details: None, direct: true, asking: false, direct_note: None, shortcut: true, picking: false, saving: false, error: None, error_details: None, error_open: false, cancellable: false }
    }
    fn line(check: Check, state: Outcome) -> Line { Line { check, state, sentence: String::new(), details: None } }

    #[test]
    fn next_waits_for_a_passing_check() {
        let mut wizard = state();
        assert!(!wizard.can_continue(), "nothing checked yet");
        wizard.lines = Some(vec![line(Check::Game, Outcome::Pass), line(Check::Data, Outcome::Fail)]);
        assert!(!wizard.can_continue());
        wizard.lines = Some(vec![line(Check::Game, Outcome::Pass), line(Check::DirectStart, Outcome::Warn), line(Check::Ours, Outcome::Fail)]);
        assert!(wizard.can_continue(), "a warning and our own files don't block the game folder");
    }

    #[test]
    fn a_found_copy_in_another_prefix_brings_its_prefix() {
        let mut wizard = state();
        wizard.use_candidate(&Candidate { game_dir: "/home/u/Games/yo3/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3".into(), wine_prefix: Some("/home/u/Games/yo3".into()) });
        assert_eq!(wizard.prefix, "/home/u/Games/yo3");
        assert!(wizard.more_options, "the prefix is shown, not hidden");
    }

    /// The player's request (2026-11-20): the tick is asked once, in setup, off until pressed; pressing it sets it up at
    /// once (Windows' permission prompt then); a refused prompt leaves it off with a quiet line, never an error.
    #[test]
    fn the_direct_start_tick_is_off_until_pressed_and_a_refused_permission_leaves_it_off() {
        let ctx = crate::app::Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() };
        assert!(!State::new(&ctx, Step::Welcome, false).direct, "a new player starts with it off");
        let mut wizard = State { direct: false, ..state() };
        let _ = wizard.update(Message::DirectDone(true, Err("Direct-start installation cancelled".into())), &ctx);
        assert!(!wizard.direct && !wizard.asking);
        assert!(wizard.direct_note.as_deref().is_some_and(|n| n.contains("permission wasn't given")), "{:?}", wizard.direct_note);
        assert!(wizard.error.is_none(), "not an error");
        let _ = wizard.update(Message::DirectDone(true, Ok(())), &ctx);
        assert!(wizard.direct && wizard.direct_note.is_none(), "allowed: on");
        let _ = wizard.update(Message::DirectDone(false, Ok(())), &ctx);
        assert!(!wizard.direct, "unticked: taken out again");
    }

    #[test]
    fn a_read_only_folder_turns_direct_start_off() {
        let mut lines = vec![line(Check::Game, Outcome::Pass)];
        assert!(!read_only(&lines));
        lines.push(Line { sentence: "This folder is read-only, so…".into(), ..line(Check::DirectStart, Outcome::Warn) });
        assert!(read_only(&lines));
    }
}
