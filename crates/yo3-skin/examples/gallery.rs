//! Every skin kit component on one screen, in each state that can be shown without the pointer (hover and pressed
//! are live: point at anything). Use it to check a change to the kit at a glance, on every platform.
//! `cargo run -p yo3-skin --example gallery`.
#![cfg_attr(windows, windows_subsystem = "windows")]
use iced::widget::{Space, checkbox, column, container, pick_list, row, scrollable, stack, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Task, window};
use yo3_skin::{BODY_BOLD, DISPLAY, Glyph, Kind, color, pill};

#[derive(Default)]
struct Gallery { field: String, secret: String, ticked: bool, choice: Option<&'static str>, dialog: bool, clicks: u32 }

#[derive(Clone, Debug)]
enum Msg { Field(String), Secret(String), Tick(bool), Choice(&'static str), Dialog(bool), Click }

const CHOICES: [&str; 3] = ["Season 16 (default)", "Season 15", "No banlist"];

impl Gallery {
    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Field(v) => self.field = v,
            Msg::Secret(v) => self.secret = v,
            Msg::Tick(v) => self.ticked = v,
            Msg::Choice(v) => self.choice = Some(v),
            Msg::Dialog(open) => self.dialog = open,
            Msg::Click => self.clicks += 1,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Msg> {
        let title = container(yo3_skin::title_strip(row![
            yo3_skin::logo(None, 30.0), Space::new().width(Fill),
            pill("ACTIVE TAB", Some(Msg::Click), Kind::Tab { active: true }, 176, 34),
            pill("TAB", Some(Msg::Click), Kind::Tab { active: false }, 176, 34),
            Space::new().width(Fill),
            pill("", Some(Msg::Click), Kind::Window(Glyph::Settings), 34, 28),
            pill("", None, Kind::Window(Glyph::Settings), 34, 28),
            pill("", Some(Msg::Click), Kind::Window(Glyph::Minimize), 34, 28),
            pill("", Some(Msg::Click), Kind::Window(Glyph::Close), 34, 28),
        ].spacing(4).align_y(Center).padding([0, 12]).height(Fill))).height(50);

        let buttons = yo3_skin::panel(Some("Buttons"), column![
            label("Navy button: enabled, disabled"),
            row![pill("Enabled", Some(Msg::Click), Kind::Primary, 130, 38), pill("Disabled", None, Kind::Primary, 130, 38)].spacing(10),
            row![pill("Back", Some(Msg::Click), Kind::Secondary, 130, 38), pill("Stop hosting", Some(Msg::Click), Kind::Danger, 150, 38)].spacing(10),
            row![yo3_skin::rule(yo3_skin::Rule::Pending, "Not typed yet"), yo3_skin::rule(yo3_skin::Rule::Met, "Met"), yo3_skin::rule(yo3_skin::Rule::Broken, "Broken")].spacing(16),
            row![yo3_skin::icon_button(yo3_skin::Icon::Copy, Some(Msg::Click), "Copy", false), yo3_skin::icon_button(yo3_skin::Icon::Save, Some(Msg::Click), "Save", false),
                yo3_skin::icon_button(yo3_skin::Icon::More, Some(Msg::Click), "More", false), yo3_skin::badge(3)].spacing(6).align_y(iced::Alignment::Center),
            label("Big action: enabled, disabled"),
            pill("PLAY", Some(Msg::Click), Kind::Play, Fill, 64),
            pill("PLAYING", None, Kind::Play, Fill, 64),
            label("Progress bar: starting, 40%, done, failed"),
            yo3_skin::progress_bar(yo3_skin::Bar::Starting, "Starting…", 22.0),
            yo3_skin::progress_bar(yo3_skin::Bar::of(167, 418), "167 of 418", 22.0),
            yo3_skin::progress_bar(yo3_skin::Bar::Done, "418 of 418 ready", 22.0),
            yo3_skin::progress_bar(yo3_skin::Bar::Failed(0.25), "Stopped at 105 of 418", 22.0),
            label("Links on a panel"),
            row![yo3_skin::link("Create account", Msg::Click, false), yo3_skin::link("Forgot password?", Msg::Click, false)].spacing(14),
            label("Dialog"),
            pill("Open a dialog", Some(Msg::Dialog(true)), Kind::Primary, 180, 38),
            text(format!("Clicks so far: {}", self.clicks)).size(14).color(color::INK_SOFT),
        ].spacing(10));

        let inputs = yo3_skin::panel(Some("Inputs"), column![
            label("Text field (focus it to see the ring)"),
            text_input("Placeholder", &self.field).on_input(Msg::Field).style(yo3_skin::field).padding(8).size(15),
            label("Password field"),
            text_input("", &self.secret).secure(true).on_input(Msg::Secret).style(yo3_skin::field).padding(8).size(15),
            label("Disabled field"),
            text_input("Not editable", "").style(yo3_skin::field).padding(8).size(15),
            row![
                checkbox(self.ticked).label("Checkbox").on_toggle(Msg::Tick).style(yo3_skin::tick).size(16).text_size(14),
                checkbox(true).label("Ticked, disabled").style(yo3_skin::tick).size(16).text_size(14),
            ].spacing(18),
            label("Drop-down"),
            pick_list(CHOICES, self.choice, Msg::Choice).placeholder("Choose…").style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(Fill).padding([7, 10]),
        ].spacing(8));

        let text_styles = yo3_skin::panel(Some("Text and colours"), column![
            text("Heading (DISPLAY)").font(DISPLAY).size(22).color(color::INK),
            text("Body text on a panel, 15 px.").size(15).color(color::INK),
            text("Soft text for status lines, 14 px.").size(14).color(color::INK_SOFT),
            text("An error in one plain sentence.").size(14).font(BODY_BOLD).color(color::BAD),
            row![swatch(color::NAVY), swatch(color::PLAY), swatch(color::CHROME), swatch(color::STRIP), swatch(color::GLOW),
                swatch(color::GOOD), swatch(color::WARN), swatch(color::BAD)].spacing(6),
            Space::new().height(8),
            panel_without_title(),
        ].spacing(8));

        let body = scrollable(row![
            container(buttons).width(Fill).height(520),
            container(inputs).width(Fill).height(520),
            container(text_styles).width(Fill).height(520),
        ].spacing(16).padding([16, 18])).height(Fill);
        let footer = container(yo3_skin::footer_strip(row![
            text("Footer text on the dark bar").size(14).color(color::ON_DARK_SOFT), Space::new().width(Fill),
            yo3_skin::link("Link on dark", Msg::Click, true),
        ].align_y(Center).padding([0, 16]).height(Fill))).height(34);
        let screen = stack![yo3_skin::backdrop(), column![title, body, footer]];
        if self.dialog {
            let dialog = container(yo3_skin::panel(Some("A dialog"), column![
                text("The screen darkens behind it; a click outside closes it.").size(15).color(color::INK),
                Space::new().height(Fill),
                row![Space::new().width(Fill), pill("OK", Some(Msg::Dialog(false)), Kind::Primary, 120, 38)],
            ])).width(470).height(220);
            yo3_skin::modal(screen, dialog, Msg::Dialog(false))
        } else {
            screen.into()
        }
    }
}

fn label(s: &str) -> Element<'_, Msg> { text(s).font(BODY_BOLD).size(14).color(color::INK).into() }
fn swatch<'a>(c: iced::Color) -> Element<'a, Msg> {
    container(Space::new().width(26).height(26)).style(move |_| container::Style {
        background: Some(c.into()), border: iced::Border { color: color::CHROME_EDGE, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }).into()
}
fn panel_without_title<'a>() -> Element<'a, Msg> {
    container(yo3_skin::panel(None, text("A frame without a title strip.").size(14).color(color::INK))).height(90).into()
}

fn main() -> iced::Result {
    let mut app = iced::application(Gallery::default, Gallery::update, Gallery::view).title("YO3 Revival skin gallery")
        .default_font(yo3_skin::BODY)
        .window(window::Settings { size: iced::Size::new(1024.0, 640.0), position: window::Position::Centered, ..window::Settings::default() });
    for font in yo3_skin::FONTS { app = app.font(font) }
    app.run()
}
