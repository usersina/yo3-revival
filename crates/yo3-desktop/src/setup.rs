//! Choosing the game folder: on the first run, and from Settings. #28 turns this into the full setup wizard.
use std::time::Duration;
use iced::widget::{Space, column, container, row, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Task};
use yo3_desktop::{Readiness, Settings, preferences::Section};
use yo3_skin::{BODY_BOLD, Kind, color, pill};
use crate::app::{Ctx, dot};

#[derive(Clone, Debug)]
pub enum Message {
    Path(String),
    Browse,
    Picked(Option<String>),
    Checked(String, Readiness),
    Done,
    Saved(Result<Box<Settings>, String>),
    Cancel,
    ToggleDetails,
}

pub struct State {
    path: String,
    /// The check of the path as it is now; `None` while it runs.
    result: Option<Readiness>,
    checked: String,
    picking: bool,
    saving: bool,
    error: Option<String>,
    /// Opened from Settings, so Cancel goes back to the main screen.
    cancellable: bool,
    /// What the last problem was, paths masked, for Details.
    details: Option<String>,
    details_open: bool,
}

impl State {
    pub fn new(ctx: &Ctx, cancellable: bool) -> Self {
        Self { path: ctx.settings.game_dir.clone(), result: None, checked: String::new(), picking: false, saving: false, error: None, cancellable, details: None, details_open: false }
    }

    pub fn update(&mut self, message: Message, ctx: &Ctx) -> Task<Message> {
        match message {
            Message::ToggleDetails => self.details_open = !self.details_open,
            Message::Path(path) => { self.path = path; self.error = None; return self.check(ctx) }
            Message::Browse => {
                if self.picking { return Task::none() }
                self.picking = true;
                return Task::perform(async {
                    let picked = tokio::time::timeout(Duration::from_secs(300), rfd::AsyncFileDialog::new().set_title("Choose the Yu-Gi-Oh! ONLINE 3 folder").pick_folder()).await;
                    picked.ok().flatten().map(|f| f.path().to_string_lossy().into_owned())
                }, Message::Picked);
            }
            Message::Picked(picked) => {
                self.picking = false;
                if let Some(path) = picked { self.path = path; return self.check(ctx) }
            }
            // Only the answer for the path still in the box counts.
            Message::Checked(path, readiness) => if path == self.path.trim() {
                self.details = match &readiness {
                    Readiness::Ready => None,
                    Readiness::GameProblem(why) | Readiness::ComponentProblem(why) | Readiness::ValidationProblem(why) => {
                        let shown = crate::error_presentation::present(why, crate::error_presentation::Hint::General, &crate::app::masks(ctx));
                        Some(shown.diagnosis.unwrap_or(shown.summary))
                    }
                };
                self.checked = path;
                self.result = Some(readiness);
            },
            Message::Done => {
                if !self.ready() || self.saving { return Task::none() }
                let (Some(backend), Some(store)) = (ctx.backend.clone(), ctx.store.clone()) else { return Task::none() };
                let mut draft = ctx.settings.clone();
                draft.game_dir = self.path.trim().to_owned();
                self.saving = true;
                return Task::perform(async move {
                    store.run(move |_db| backend.save_preferences(Section::Game, &draft).map(Box::new)).await.map_err(|e| format!("{e:#}"))
                }, Message::Saved);
            }
            Message::Saved(Err(error)) => { self.saving = false; self.error = Some(error) }
            Message::Saved(Ok(_)) | Message::Cancel => {}
        }
        Task::none()
    }

    /// Checks the folder in the box: the game, and YO3 Revival's own files.
    pub fn check(&self, ctx: &Ctx) -> Task<Message> {
        let Some(backend) = ctx.backend.clone() else { return Task::none() };
        let path = self.path.trim().to_owned();
        let mut settings = ctx.settings.clone();
        settings.game_dir = path.clone();
        Task::perform(async move {
            let readiness = tokio::task::spawn_blocking(move || backend.readiness(&settings)).await
                .unwrap_or_else(|error| Readiness::ValidationProblem(error.to_string()));
            (path, readiness)
        }, |(path, readiness)| Message::Checked(path, readiness))
    }

    /// The game folder is right. Our own files are the main screen's business (its Repair state), not this choice's.
    fn ready(&self) -> bool {
        self.checked == self.path.trim() && matches!(self.result, Some(Readiness::Ready | Readiness::ComponentProblem(_)))
    }

    pub fn view(&self) -> Element<'_, Message> {
        let (tone, line) = match (&self.result, self.checked == self.path.trim()) {
            (Some(Readiness::Ready), true) => (color::GOOD, "Yu-Gi-Oh! ONLINE 3 found, and YO3 Revival's files are complete."),
            (Some(Readiness::GameProblem(_)), true) => (color::BAD, "This folder doesn't hold the supported Yu-Gi-Oh! ONLINE 3. Choose the folder with yo3.exe."),
            (Some(Readiness::ComponentProblem(_)), true) => (color::WARN, "Yu-Gi-Oh! ONLINE 3 found. Some of YO3 Revival's own files need repair: the main screen shows how."),
            (Some(Readiness::ValidationProblem(_)), true) => (color::WARN, "The check didn't finish. Try again in a moment."),
            _ => (color::CHROME_DARK, "Checking this folder…"),
        };
        let mut buttons = row![Space::new().width(Fill)].spacing(10);
        if self.cancellable { buttons = buttons.push(pill("Cancel", Some(Message::Cancel), Kind::Primary, 120, 38)) }
        buttons = buttons.push(pill(if self.saving { "Saving…" } else { "Done" }, (self.ready() && !self.saving).then_some(Message::Done), Kind::Primary, 120, 38));
        let mut body = column![
            text("Where is Yu-Gi-Oh! ONLINE 3 installed? YO3 Revival needs your own copy of the game.").size(15).color(color::INK),
            text("Game folder").font(BODY_BOLD).size(14).color(color::INK),
            row![
                text_input("The folder with yo3.exe", &self.path).on_input(Message::Path).on_submit(Message::Done).style(yo3_skin::field).padding(8).size(15),
                pill("Browse…", (!self.picking).then_some(Message::Browse), Kind::Primary, 110, 38),
            ].spacing(10).align_y(Center),
            row![container(Space::new().width(9).height(9)).style(move |_| dot(tone)), text(line).size(14).color(color::INK)].spacing(8).align_y(Center),
        ].spacing(10);
        if let Some(details) = &self.details && self.checked == self.path.trim() {
            body = body.push(yo3_skin::link(if self.details_open { "Hide details" } else { "Details" }, Message::ToggleDetails, false));
            if self.details_open { body = body.push(text(details).size(13).color(color::INK_SOFT)) }
        }
        if let Some(error) = &self.error { body = body.push(text(error).size(14).font(BODY_BOLD).color(color::BAD)) }
        body = body.push(Space::new().height(Fill)).push(buttons);
        container(container(yo3_skin::panel(Some("Setup"), body)).width(620).height(330)).center(Fill).into()
    }
}
