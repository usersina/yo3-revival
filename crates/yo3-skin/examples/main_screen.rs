//! The look spike for issue #21: the launcher's main screen, Singleplayer and Multiplayer, built only from the skin
//! kit. Nothing here is wired to a server or the game; the states are simulated so every part of the screen can be
//! seen and clicked. `cargo run -p yo3-skin --example main_screen [-- --system-frame]`.
use std::time::Duration;
use iced::widget::{Space, column, container, mouse_area, operation, pick_list, row, stack, text, text_input, checkbox};
use iced::{Alignment::Center, Color, Element, Length::Fill, Subscription, Task, keyboard, window};
use yo3_skin::{BODY, BODY_BOLD, Glyph, Kind, color, pill};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Tab { #[default] Single, Multi }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Play { #[default] Ready, Starting(u8), Playing }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dialog { Help, WhatsNew, Settings, Discord, CreateAccount, Forgot }

const BANLISTS: [&str; 5] = ["Season 16", "Season 15", "Season 14", "Season 13", "No banlist"];
const SERVERS: [&str; 2] = ["YO3 Revival Beta  ·  42 online", "Friends' server  ·  3 online"];
const STEPS: [&str; 3] = ["Checking files…", "Starting your server…", "Starting the game…"];

#[derive(Default)]
struct Spike {
    tab: Tab, play: Play, banlist: Option<&'static str>, server: Option<&'static str>,
    user: String, password: String, save_id: bool, stay: bool, signed_in: Option<String>, error: Option<String>,
    adding: bool, address: String, dialog: Option<Dialog>, draft: [String; 4],
}

#[derive(Clone, Debug)]
enum Msg {
    Tab(Tab), Banlist(&'static str), Server(&'static str), Play, Step, CloseGame,
    User(String), Password(String), SaveId(bool), Stay(bool), LogIn, SignOut, ToggleAdd, Address(String),
    Open(Dialog), CloseDialog, Drag, Minimize, Exit, Key(keyboard::Event), Draft(usize, String),
}

impl Spike {
    fn boot() -> Self { Spike { banlist: Some(BANLISTS[0]), server: Some(SERVERS[0]), save_id: true, stay: true, ..Spike::default() } }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Tab(tab) => self.tab = tab,
            Msg::Banlist(b) => self.banlist = Some(b),
            Msg::Server(s) => { self.server = Some(s); self.signed_in = None; self.error = None; }
            Msg::Play => if self.play == Play::Ready { self.play = Play::Starting(0) },
            Msg::Step => self.play = match self.play { Play::Starting(n) if usize::from(n) + 1 < STEPS.len() => Play::Starting(n + 1), Play::Starting(_) => Play::Playing, other => other },
            Msg::CloseGame => self.play = Play::Ready,
            Msg::User(v) => self.user = v,
            Msg::Password(v) => self.password = v,
            Msg::SaveId(v) => self.save_id = v,
            Msg::Stay(v) => self.stay = v,
            Msg::LogIn => {
                self.error = None;
                if self.user.trim().is_empty() || self.password.is_empty() { self.error = Some("Enter your User ID and password.".into()) }
                else if self.password != "beta" { self.error = Some("Wrong User ID or password.".into()); self.password.clear() }
                else { self.signed_in = Some(self.user.trim().to_owned()); self.password.clear() }
            }
            Msg::SignOut => self.signed_in = None,
            Msg::ToggleAdd => self.adding = !self.adding,
            Msg::Address(v) => self.address = v,
            Msg::Open(d) => { self.dialog = Some(d); self.draft = Default::default(); }
            Msg::CloseDialog => self.dialog = None,
            Msg::Drag => return window::latest().and_then(window::drag),
            Msg::Minimize => return window::latest().and_then(|id| window::minimize(id, true)),
            Msg::Exit => return iced::exit(),
            Msg::Key(keyboard::Event::KeyPressed { key: keyboard::Key::Named(named), modifiers, .. }) => match named {
                keyboard::key::Named::Tab if modifiers.shift() => return operation::focus_previous(),
                keyboard::key::Named::Tab => return operation::focus_next(),
                keyboard::key::Named::Escape => self.dialog = None,
                _ => {}
            },
            Msg::Key(_) => {}
            Msg::Draft(i, v) => self.draft[i] = v,
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Msg> {
        let keys = keyboard::listen().map(Msg::Key);
        match self.play {
            Play::Starting(_) => Subscription::batch([keys, iced::time::every(Duration::from_millis(900)).map(|_| Msg::Step)]),
            _ => keys,
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        let tabs = row![
            pill("SINGLEPLAYER", Some(Msg::Tab(Tab::Single)), Kind::Tab { active: self.tab == Tab::Single }, 176, 34),
            pill("MULTIPLAYER", Some(Msg::Tab(Tab::Multi)), Kind::Tab { active: self.tab == Tab::Multi }, 176, 34),
        ].spacing(4);
        let controls = row![
            pill("", Some(Msg::Open(Dialog::Settings)), Kind::Window(Glyph::Settings), 34, 28),
            pill("", Some(Msg::Minimize), Kind::Window(Glyph::Minimize), 34, 28),
            pill("", Some(Msg::Exit), Kind::Window(Glyph::Close), 34, 28),
        ].spacing(3);
        let bar = row![yo3_skin::logo(30.0), Space::new().width(Fill), tabs, Space::new().width(Fill), controls]
            .align_y(Center).padding([0, 12]).height(Fill);
        let title = container(yo3_skin::title_strip(mouse_area(bar).on_press(Msg::Drag))).height(50);

        let (news, side) = match self.tab { Tab::Single => (self.news(&SINGLE_NEWS), self.singleplayer()), Tab::Multi => (self.news(&SERVER_NEWS), self.multiplayer()) };
        let body = row![
            container(yo3_skin::panel(Some("Information"), news)).width(Fill).height(Fill),
            container(side).width(372).height(Fill),
        ].spacing(16).padding([16, 18]).height(Fill);

        let footer = row![
            text("v0.2.0").size(14).color(color::ON_DARK_SOFT),
            container(Space::new().width(9).height(9)).style(|_| dot(color::WARN)),
            text("Update 0.2.1 is ready: restart to install").size(14).color(color::ON_DARK),
            Space::new().width(Fill),
            yo3_skin::link("Help", Msg::Open(Dialog::Help), true),
            yo3_skin::link("Discord", Msg::Open(Dialog::Discord), true),
        ].spacing(10).align_y(Center).padding([0, 16]).height(Fill);
        let page = column![title, body, container(yo3_skin::footer_strip(footer)).height(34)];
        let screen = stack![yo3_skin::backdrop(), page];
        match self.dialog {
            Some(d) => yo3_skin::modal(screen, self.dialog(d), Msg::CloseDialog),
            None => screen.into(),
        }
    }

    fn news<'a>(&self, items: &'a [(&'a str, &'a str, Option<Dialog>)]) -> Element<'a, Msg> {
        let mut list = column![].spacing(0);
        for (i, (date, headline, opens)) in items.iter().enumerate() {
            let shade = if i % 2 == 0 { color::ROW } else { color::ROW_ALT };
            let line = container(row![
                text(*date).font(BODY_BOLD).size(15).color(color::NAVY).width(54),
                text(*headline).size(15).color(color::INK),
            ].align_y(Center)).padding([9, 12]).width(Fill)
                .style(move |_| container::Style { background: Some(shade.into()), ..container::Style::default() });
            list = list.push(match opens {
                Some(d) => Element::from(mouse_area(line).on_press(Msg::Open(*d)).interaction(iced::mouse::Interaction::Pointer)),
                None => line.into(),
            });
        }
        container(list).style(|_| container::Style { border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 2.0.into() }, ..container::Style::default() }).into()
    }

    fn singleplayer(&self) -> Element<'_, Msg> {
        let (label, action, status) = match self.play {
            Play::Ready => ("PLAY", Some(Msg::Play), None),
            Play::Starting(n) => ("STARTING…", None, Some(STEPS[usize::from(n)])),
            Play::Playing => ("PLAYING", None, Some("The game is running.")),
        };
        let mut status_row = row![text(status.unwrap_or_default()).size(14).color(color::INK_SOFT)].spacing(10).align_y(Center).height(22);
        if self.play == Play::Playing { status_row = status_row.push(Space::new().width(Fill)).push(yo3_skin::link("Close game", Msg::CloseGame, false)) }
        yo3_skin::panel(Some("Your own server"), column![
            text("Duel the NPCs, open packs and build decks. Cards you earn here stay on this PC.").size(15).color(color::INK),
            row![
                text("Banlist").font(BODY_BOLD).size(15).color(color::INK).width(74),
                pick_list(BANLISTS, self.banlist, Msg::Banlist).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(Fill).padding([7, 10]),
            ].align_y(Center),
            Space::new().height(Fill),
            status_row,
            pill(label, action, Kind::Play, Fill, 70),
        ].spacing(16))
    }

    fn multiplayer(&self) -> Element<'_, Msg> {
        let server = row![
            container(Space::new().width(10).height(10)).style(|_| dot(color::GOOD)),
            pick_list(SERVERS, self.server, Msg::Server).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(Fill).padding([7, 10]),
        ].spacing(8).align_y(Center);
        let account: Element<'_, Msg> = match &self.signed_in {
            Some(name) => column![
                row![text("Signed in as ").size(15).color(color::INK), text(name.as_str()).font(BODY_BOLD).size(15).color(color::INK)],
                yo3_skin::link("Sign out", Msg::SignOut, false),
            ].spacing(6).into(),
            None => column![
                caption("User ID"),
                text_input("", &self.user).on_input(Msg::User).on_submit(Msg::LogIn).style(yo3_skin::field).padding(8).size(15),
                caption("Password"),
                text_input("", &self.password).secure(true).on_input(Msg::Password).on_submit(Msg::LogIn).style(yo3_skin::field).padding(8).size(15),
                row![
                    checkbox(self.save_id).label("Save User ID").on_toggle(Msg::SaveId).style(yo3_skin::tick).size(16).text_size(14),
                    checkbox(self.stay).label("Stay signed in").on_toggle(Msg::Stay).style(yo3_skin::tick).size(16).text_size(14),
                ].spacing(18),
                row![
                    yo3_skin::link("Create account", Msg::Open(Dialog::CreateAccount), false),
                    text("·").color(color::INK_SOFT),
                    yo3_skin::link("Forgot password?", Msg::Open(Dialog::Forgot), false),
                ].spacing(8),
            ].spacing(6).into(),
        };
        let error = text(self.error.clone().unwrap_or_default()).size(14).font(BODY_BOLD).color(color::BAD);
        let mut add = column![yo3_skin::link(if self.adding { "Hide other servers" } else { "Add a server by address" }, Msg::ToggleAdd, false)].spacing(6);
        if self.adding {
            add = add.push(row![
                text_input("host:port or invite link", &self.address).on_input(Msg::Address).style(yo3_skin::field).padding(7).size(14),
                pill("Add", None, Kind::Primary, 72, 34),
            ].spacing(8).align_y(Center));
        }
        let (label, action) = match &self.signed_in { Some(_) => ("PLAY", Some(Msg::Tab(Tab::Multi))), None => ("LOG IN", Some(Msg::LogIn)) };
        yo3_skin::panel(Some("Server"), column![server, account, error, Space::new().height(Fill), add, pill(label, action, Kind::Play, Fill, 70)].spacing(10))
    }

    fn dialog(&self, which: Dialog) -> Element<'_, Msg> {
        let (title, body, height) = match which {
            Dialog::Help => ("Help", "The guide, troubleshooting for the current error and Report a problem will live here (#31).", 240.0),
            Dialog::WhatsNew => ("What's new in 0.2.0", "• A new launcher in the game's own style.\n• Log in with a User ID and password.\n• Your own server starts with the game and stops after it.", 270.0),
            Dialog::Settings => ("Settings", "Game, Accounts, Singleplayer, Advanced and About will be here (#31).", 240.0),
            Dialog::Discord => ("Discord", "Opens the project's Discord invite in your browser.", 220.0),
            Dialog::Forgot => ("Forgot password?", "Enter the recovery code you saved when you created the account, then a new password. No code? Ask a moderator on Discord.", 260.0),
            Dialog::CreateAccount => return self.create_account(),
        };
        container(yo3_skin::panel(Some(title), column![
            text(body).size(15).color(color::INK),
            Space::new().height(Fill),
            row![Space::new().width(Fill), pill("OK", Some(Msg::CloseDialog), Kind::Primary, 120, 38)],
        ])).width(470).height(height).into()
    }

    fn create_account(&self) -> Element<'_, Msg> {
        let input = |i: usize, placeholder: &str, secure: bool| text_input(placeholder, &self.draft[i]).secure(secure)
            .on_input(move |v| Msg::Draft(i, v)).style(yo3_skin::field).padding(8).size(15);
        container(yo3_skin::panel(Some("Create account"), column![
            caption("User ID"), input(0, "3–16 letters, digits, _ or -", false),
            caption("Password"), input(1, "8 characters or more", true),
            caption("Password again"), input(2, "", true),
            caption("Invite code"), input(3, "from the beta's Discord", false),
            Space::new().height(Fill),
            row![Space::new().width(Fill), pill("Cancel", Some(Msg::CloseDialog), Kind::Primary, 120, 38), pill("Create", Some(Msg::CloseDialog), Kind::Primary, 120, 38)].spacing(10),
        ].spacing(6))).width(470).height(440).into()
    }
}

fn caption(label: &str) -> Element<'_, Msg> { text(label).font(BODY_BOLD).size(14).color(color::INK).into() }
fn dot(c: Color) -> container::Style {
    container::Style { background: Some(c.into()), border: iced::Border { color: Color { a: 0.5, ..Color::BLACK }, width: 1.0, radius: 5.0.into() }, ..container::Style::default() }
}

const SINGLE_NEWS: [(&str, &str, Option<Dialog>); 3] = [
    ("11/20", "YO3 Revival 0.2.0 is out", Some(Dialog::WhatsNew)),
    ("11/18", "New: Host for friends, from the launcher", None),
    ("11/15", "Welcome to YO3 Revival", None),
];
const SERVER_NEWS: [(&str, &str, Option<Dialog>); 3] = [
    ("11/21", "Maintenance on 11/22 from 10:00 to 11:00 (JST)", None),
    ("11/20", "The closed beta has started", None),
    ("11/19", "Invite codes: ask on our Discord", None),
];

fn main() -> iced::Result {
    let system_frame = std::env::args().any(|a| a == "--system-frame");
    let mut app = iced::application(Spike::boot, Spike::update, Spike::view).title("YO3 Revival")
        .subscription(Spike::subscription).default_font(BODY)
        .window(window::Settings { size: iced::Size::new(1024.0, 640.0), resizable: false, decorations: system_frame,
            position: window::Position::Centered, ..window::Settings::default() });
    for font in yo3_skin::FONTS { app = app.font(font) }
    app.run()
}
