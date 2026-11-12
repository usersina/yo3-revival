//! Small compositions of standard Iced widgets; the library owns layout, focus and interaction.
//! The look borrows from the game's own screens: steel-rimmed navy panels, bright blue title plates ("Edit Deck"),
//! dark section bars (Main/Extra/Side), gunmetal consoles and the black chat log with green text.
use iced::{Element, Color, Theme, Length::Fill};
use iced::widget::{button, container, row, text, text_input, svg, markdown};
use crate::Msg;
pub const BACKGROUND: Color = Color::from_rgb8(9, 20, 52);
pub const SIDEBAR: Color = Color::from_rgb8(28, 32, 40);
pub const SURFACE: Color = Color::from_rgb8(14, 31, 74);
pub const BORDER: Color = Color::from_rgb8(86, 102, 130);
pub const STEEL: Color = Color::from_rgb8(176, 188, 205);
pub const WELL: Color = Color::from_rgb8(4, 6, 10);
pub const TEXT: Color = Color::from_rgb8(242, 245, 250);
pub const PRIMARY: Color = Color::from_rgb8(52, 100, 200);
pub const ACCENT: Color = Color::from_rgb8(104, 186, 250);
pub const MUTED: Color = Color::from_rgb8(168, 182, 208);
pub const GOOD: Color = Color::from_rgb8(92, 222, 110);
pub const WARN: Color = Color::from_rgb8(246, 200, 80);
pub const BAD: Color = Color::from_rgb8(255, 112, 112);
/// The chat log's green.
pub const CONSOLE: Color = Color::from_rgb8(78, 230, 96);
/// The application theme, shared by the window and the Markdown viewer.
pub fn theme() -> Theme {
    Theme::custom_with_fn("Revival", iced::theme::Palette { background: BACKGROUND, text: TEXT, primary: PRIMARY,
        success: GOOD, warning: WARN, danger: BAD }, |palette| {
        // Scroll rails: a dark track with a steel scroller instead of the generated bright blue.
        let mut extended = iced::theme::palette::Extended::generate(palette);
        extended.background.weak.color = Color::from_rgb8(16, 30, 66);
        extended.background.strongest.color = Color::from_rgb8(118, 130, 150);
        extended
    })
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone { Neutral, Info, Success, Warning, Danger }
pub fn tone_color(tone: Tone) -> Color {
    match tone { Tone::Neutral => MUTED, Tone::Info => ACCENT, Tone::Success => GOOD, Tone::Warning => WARN, Tone::Danger => BAD }
}
fn tone_icon(tone: Tone) -> Icon {
    match tone { Tone::Neutral => Icon::Info, Tone::Info => Icon::Info, Tone::Success => Icon::Check,
        Tone::Warning => Icon::Warning, Tone::Danger => Icon::Alert }
}
#[derive(Clone, Copy)]
pub enum Icon { Play, Server, Users, Activity, Settings, Folder, External, Download, Down, Right, Check, Plus, Upload, Copy, Close, Refresh, Book, Shield, Stop, Info, Warning, Alert }
pub fn icon<'a>(which: Icon) -> svg::Svg<'a> {
    let bytes: &'static [u8] = match which {
        Icon::Play => include_bytes!("../assets/icons/play.svg"),
        Icon::Server => include_bytes!("../assets/icons/server.svg"), Icon::Users => include_bytes!("../assets/icons/users.svg"),
        Icon::Activity => include_bytes!("../assets/icons/activity.svg"), Icon::Settings => include_bytes!("../assets/icons/settings-2.svg"),
        Icon::Folder => include_bytes!("../assets/icons/folder-open.svg"), Icon::External => include_bytes!("../assets/icons/external-link.svg"),
        Icon::Download => include_bytes!("../assets/icons/download.svg"), Icon::Down => include_bytes!("../assets/icons/chevron-down.svg"),
        Icon::Right => include_bytes!("../assets/icons/chevron-right.svg"), Icon::Check => include_bytes!("../assets/icons/circle-check.svg"),
        Icon::Plus => include_bytes!("../assets/icons/plus.svg"), Icon::Upload => include_bytes!("../assets/icons/file-up.svg"),
        Icon::Copy => include_bytes!("../assets/icons/copy.svg"), Icon::Close => include_bytes!("../assets/icons/x.svg"),
        Icon::Refresh => include_bytes!("../assets/icons/refresh-cw.svg"), Icon::Book => include_bytes!("../assets/icons/book-open.svg"),
        Icon::Shield => include_bytes!("../assets/icons/shield-check.svg"), Icon::Stop => include_bytes!("../assets/icons/square.svg"),
        Icon::Info => include_bytes!("../assets/icons/info.svg"), Icon::Warning => include_bytes!("../assets/icons/alert-triangle.svg"),
        Icon::Alert => include_bytes!("../assets/icons/alert-circle.svg"),
    };
    svg(svg::Handle::from_memory(bytes)).width(18).height(18).style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}
pub fn action<'a>(which: Icon, label: &'static str, message: Option<Msg>) -> button::Button<'a, Msg> {
    let opacity: f32 = if message.is_some() { 1.0 } else { 0.35 };
    button(row![icon(which).opacity(opacity), text(label).size(14)].spacing(9).align_y(iced::Alignment::Center))
        .padding([10, 14]).on_press_maybe(message).style(secondary)
}
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }
}
fn radius(value: f32) -> iced::border::Radius { value.into() }
/// Top to bottom; Iced measures gradient angles from the positive y axis.
fn vertical(top: Color, bottom: Color) -> iced::Background {
    iced::Gradient::Linear(iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI)).add_stop(0.0, top).add_stop(1.0, bottom)).into()
}
fn horizontal(stops: &[(f32, Color)]) -> iced::Background {
    let mut gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2));
    for (offset, color) in stops { gradient = gradient.add_stop(*offset, *color); }
    iced::Gradient::Linear(gradient).into()
}
/// A bevelled button: lit at the top, darker below, with a steel rim, like the game's menu buttons.
fn bevel(top: Color, bottom: Color, status: button::Status) -> button::Style {
    let (top, bottom) = match status {
        button::Status::Hovered => (mix(top, Color::WHITE, 0.14), mix(bottom, Color::WHITE, 0.10)),
        button::Status::Pressed => (bottom, top),
        button::Status::Disabled => (mix(top, SIDEBAR, 0.7), mix(bottom, SIDEBAR, 0.7)),
        button::Status::Active => (top, bottom),
    };
    let enabled = !matches!(status, button::Status::Disabled);
    button::Style { background: Some(vertical(top, bottom)), text_color: if enabled { Color::WHITE } else { Color { a: 0.45, ..TEXT } },
        border: iced::Border { color: if enabled { STEEL } else { Color { a: 0.4, ..BORDER } }, width: 1.0, radius: radius(4.0) },
        ..Default::default() }
}
/// The main call to action on a page: the game's navy menu button.
pub fn primary(_: &Theme, status: button::Status) -> button::Style { bevel(Color::from_rgb8(70, 112, 204), Color::from_rgb8(24, 44, 116), status) }
/// The Play button: the red of the Duel Disk's markers and the Monster Reborn gem.
pub fn duel(_: &Theme, status: button::Status) -> button::Style { bevel(Color::from_rgb8(232, 70, 66), Color::from_rgb8(140, 18, 30), status) }
/// Stopping, removing and resetting.
pub fn danger(_: &Theme, status: button::Status) -> button::Style { bevel(Color::from_rgb8(200, 58, 64), Color::from_rgb8(112, 18, 28), status) }
/// A gunmetal button for secondary actions, like the game's Delete and Back buttons.
pub fn secondary(_: &Theme, status: button::Status) -> button::Style { bevel(Color::from_rgb8(78, 86, 100), Color::from_rgb8(34, 38, 46), status) }
/// A borderless action that shows a soft highlight on hover.
pub fn ghost(_: &Theme, status: button::Status) -> button::Style {
    let background = match status { button::Status::Hovered => Some(Color { a: 0.08, ..Color::WHITE }.into()),
        button::Status::Pressed => Some(Color { a: 0.14, ..Color::WHITE }.into()), _ => None };
    button::Style { background, text_color: if matches!(status, button::Status::Disabled) { Color { a: 0.45, ..TEXT } } else { TEXT },
        border: iced::Border { radius: radius(4.0), ..Default::default() }, ..Default::default() }
}
pub fn disclosure<'a>(label: &'static str, expanded: bool, message: Msg) -> button::Button<'a, Msg> {
    action(if expanded { Icon::Down } else { Icon::Right }, label, Some(message)).width(Fill).style(ghost)
}
fn panel(_: &Theme) -> container::Style {
    container::Style { background: Some(vertical(Color::from_rgb8(18, 38, 88), SURFACE)), text_color: Some(TEXT),
        border: iced::Border { color: BORDER, width: 1.0, radius: radius(4.0) }, ..Default::default() }
}
pub fn card<'a>(content: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    container(content).width(Fill).padding(18).style(panel)
}
/// A section bar like the deck editor's Main/Extra/Side: a dark plate that fades out to the right.
/// A section bar with a small action at its right end.
pub fn bar_with<'a>(title: &'static str, trailing: Option<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    let mut content = row![text(title).size(15).font(semibold()).color(Color::WHITE).width(Fill)].align_y(iced::Alignment::Center);
    if let Some(trailing) = trailing { content = content.push(trailing); }
    container(content).width(Fill).padding([6, 14]).style(|_: &Theme| container::Style {
        background: Some(horizontal(&[(0.0, Color::from_rgb8(44, 84, 172)), (0.45, Color::from_rgb8(26, 52, 118)), (1.0, Color::from_rgba8(26, 52, 118, 0.0))])),
        border: iced::Border { radius: iced::border::Radius { top_left: 4.0, ..Default::default() }, ..Default::default() }, ..Default::default() })
}
/// A titled panel: one group of related settings or actions.
pub fn section<'a>(title: &'static str, content: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    section_with(title, None, content)
}
pub fn section_with<'a>(title: &'static str, trailing: Option<Element<'a, Msg>>, content: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    container(iced::widget::column![bar_with(title, trailing), container(content).width(Fill).padding(iced::Padding { top: 14.0, right: 18.0, bottom: 18.0, left: 18.0 })])
        .width(Fill).style(panel)
}
/// A page title on a bright blue plate, like the game's screen names.
pub fn plate<'a>(title: &'a str, size: u32) -> container::Container<'a, Msg> {
    container(text(title).size(size).font(semibold()).color(Color::WHITE)).padding([5, 18]).style(|_: &Theme| container::Style {
        background: Some(horizontal(&[(0.0, Color::from_rgb8(96, 182, 250)), (0.5, Color::from_rgb8(44, 118, 222)), (1.0, Color::from_rgb8(30, 74, 176))])),
        border: iced::Border { color: Color::from_rgb8(214, 236, 255), width: 1.0, radius: radius(3.0) }, ..Default::default() })
}
/// A gunmetal console, like the game's bottom chat frame; holds the launch controls.
pub fn console<'a>(content: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    container(content).width(Fill).padding(18).style(|_: &Theme| container::Style {
        background: Some(vertical(Color::from_rgb8(64, 72, 86), Color::from_rgb8(24, 28, 35))), text_color: Some(TEXT),
        border: iced::Border { color: STEEL, width: 1.0, radius: radius(5.0) }, ..Default::default() })
}
/// The navigation column: a gunmetal frame like the game's bottom console.
pub fn sidebar(_: &Theme) -> container::Style {
    container::Style { background: Some(vertical(Color::from_rgb8(44, 50, 61), Color::from_rgb8(20, 23, 29))), text_color: Some(TEXT),
        border: iced::Border { color: Color::from_rgb8(92, 102, 118), width: 1.0, ..Default::default() }, ..Default::default() }
}
/// The page behind the panels: the game's deep blue, a little lighter at the top.
pub fn backdrop(_: &Theme) -> container::Style {
    container::Style { background: Some(vertical(Color::from_rgb8(14, 32, 84), Color::from_rgb8(6, 14, 38))), text_color: Some(TEXT), ..Default::default() }
}
/// Drop-down lists look like the game's list rows: navy with a steel rim.
pub fn pick(_: &Theme, status: iced::widget::pick_list::Status) -> iced::widget::pick_list::Style {
    let open = !matches!(status, iced::widget::pick_list::Status::Active);
    iced::widget::pick_list::Style { text_color: TEXT, placeholder_color: MUTED, handle_color: if open { ACCENT } else { STEEL },
        background: vertical(Color::from_rgb8(38, 66, 140), Color::from_rgb8(18, 36, 90)),
        border: iced::Border { color: if open { ACCENT } else { STEEL }, width: 1.0, radius: radius(4.0) } }
}
/// A black box for code, identities and other monospace text.
pub fn well(_: &Theme) -> container::Style {
    container::Style { background: Some(WELL.into()), text_color: Some(TEXT),
        border: iced::Border { color: BORDER, width: 1.0, radius: radius(3.0) }, ..Default::default() }
}
/// A counter like the deck editor's card totals: white figures in a black box.
pub fn counter<'a>(value: String) -> container::Container<'a, Msg> {
    container(text(value).size(26).font(iced::Font::MONOSPACE).color(Color::WHITE)).padding([4, 14]).style(well)
}
/// Tooltips and other floating text.
pub fn bubble(_: &Theme) -> container::Style {
    container::Style { background: Some(Color::from_rgb8(236, 241, 248).into()), text_color: Some(Color::from_rgb8(20, 28, 44)),
        border: iced::Border { color: Color::from_rgb8(70, 90, 130), width: 1.0, radius: radius(3.0) }, ..Default::default() }
}
pub fn semibold() -> iced::Font { iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::with_name("Fira Sans") } }
/// The launcher icon, decoded at build time.
pub fn brand<'a>(size: f32) -> container::Container<'a, Msg> {
    static HANDLE: std::sync::OnceLock<iced::widget::image::Handle> = std::sync::OnceLock::new();
    let handle = HANDLE.get_or_init(|| iced::widget::image::Handle::from_rgba(256, 256,
        include_bytes!(concat!(env!("OUT_DIR"), "/launcher-icon.rgba")).to_vec())).clone();
    container(iced::widget::image(handle).width(size).height(size).border_radius(4.0)).padding(1)
        .style(|_: &Theme| container::Style { border: iced::Border { color: STEEL, width: 1.0, radius: radius(5.0) }, ..Default::default() })
}
/// The colour of a log line by its level: errors red, warnings amber, everything else the default text colour.
pub fn log_color(line: &str) -> Option<Color> {
    use yo3_tunnel::diagnostics::{Level, level};
    match level(line) { Some(Level::Error) => Some(tone_color(Tone::Danger)), Some(Level::Warn) => Some(tone_color(Tone::Warning)), _ => None }
}
pub fn log_panel<'a>(logs: String) -> container::Container<'a, Msg> {
    // Consecutive lines of one colour share a span, so long logs stay a handful of spans.
    let mut spans: Vec<iced::widget::text::Span<'a>> = Vec::new();
    let (mut run, mut color) = (String::new(), None);
    for line in logs.lines() {
        let next = log_color(line);
        if next != color && !run.is_empty() { spans.push(iced::widget::span(std::mem::take(&mut run)).color_maybe(color)); }
        color = next; run.push_str(line); run.push('\n');
    }
    if run.ends_with('\n') { run.pop(); }
    if !run.is_empty() { spans.push(iced::widget::span(run).color_maybe(color)); }
    let body = iced::widget::rich_text(spans).size(12).font(iced::Font::MONOSPACE).width(Fill);
    // Fit short output naturally; the parent bounds the viewport for long logs.
    container(iced::widget::scrollable(body)
        .id("activity-log").width(Fill).height(iced::Length::Shrink).spacing(10)
        .on_scroll(|viewport|Msg::LogScrolled(viewport.absolute_offset_reversed().y<=1.0)))
        .padding(14).width(Fill).height(iced::Length::Shrink).style(|theme: &Theme| container::Style { text_color: Some(CONSOLE), ..well(theme) })
}
pub fn muted<'a>(label: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(label).size(13).color(MUTED).width(Fill)
}
pub fn report_filter<'a>(checked:bool,enabled:bool)->Element<'a,Msg> {
    let control=iced::widget::checkbox(checked).label("Exclude chat, decks, paths & addresses")
        .size(14).text_size(13).spacing(8).on_toggle_maybe(enabled.then_some(Msg::RemovePersonal));
    iced::widget::tooltip(control,container(text("Filters player activity and detailed game traces, and masks common paths and network/email addresses in this report. Original logs stay unchanged. Review before sharing.").size(12))
        .padding(10).max_width(340).style(bubble),iced::widget::tooltip::Position::Top).into()
}
pub fn label_help<'a>(label: &'static str, help: &'static str) -> Element<'a, Msg> {
    use iced::widget::tooltip;
    let hint = container(text("?").size(11).color(MUTED)).width(18).height(18).center_x(18).center_y(18)
        .style(|_: &Theme| container::Style { border: iced::Border { color: BORDER, width: 1.0, radius: radius(9.0) }, ..Default::default() });
    row![text(label).size(15), tooltip(hint, container(text(help).size(13)).padding(12).max_width(300)
        .style(bubble), tooltip::Position::Top).gap(8).snap_within_viewport(true)]
        .spacing(8).align_y(iced::Alignment::Center).into()
}
pub fn status_badge<'a>(label: String, tone: Tone) -> Element<'a, Msg> {
    let color = tone_color(tone);
    container(row![icon(tone_icon(tone)).style(move |_,_| svg::Style { color: Some(color) }), text(label).size(12).color(color)].spacing(7).align_y(iced::Alignment::Center))
        .padding([6, 10]).style(move |_| container::Style { background: Some(Color { a: 0.08, ..color }.into()), border: iced::Border { color: Color { a: 0.22, ..color }, width: 1.0, radius: 3.0.into() }, ..Default::default() }).into()
}
pub fn inline_status<'a>(label: impl iced::widget::text::IntoFragment<'a>, tone: Tone) -> Element<'a, Msg> {
    let color = tone_color(tone);
    row![icon(tone_icon(tone)).style(move |_,_| svg::Style { color: Some(color) }),
        text(label).size(13).color(color).width(Fill)].spacing(9).align_y(iced::Alignment::Center).into()
}
pub fn notice<'a>(content: impl Into<Element<'a, Msg>>, tone: Tone) -> container::Container<'a, Msg> {
    let color = tone_color(tone);
    container(content).padding(14).width(Fill).style(move |theme: &Theme| container::Style {
        background: Some(Color { a: 0.07, ..color }.into()), text_color: Some(theme.palette().text),
        border: iced::Border { color: Color { a: 0.45, ..color }, width: 1.0, radius: 3.0.into() }, ..Default::default()
    })
}
pub fn input_style(theme: &Theme, status: text_input::Status, invalid: bool) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.background = WELL.into(); style.border.radius = 3.0.into();
    style.border.color = match status { text_input::Status::Focused { .. } => ACCENT, text_input::Status::Hovered => mix(BORDER, ACCENT, 0.3), _ => BORDER };
    style.placeholder = Color { a: 0.75, ..MUTED }; style.selection = Color { a: 0.35, ..PRIMARY };
    if invalid {
        style.border.color = if matches!(status,text_input::Status::Disabled) { Color { a: 0.65, ..BAD } } else { BAD };
        style.border.width = if matches!(status,text_input::Status::Focused {..}) { 2.0 } else { 1.0 };
    }
    style
}
pub fn release_markdown_settings() -> markdown::Settings {
    let mut style=markdown::Style::from(theme());
    style.font=iced::Font::with_name("Fira Sans");style.link_color=tone_color(Tone::Info);
    style.inline_code_color=TEXT;
    let mut settings=markdown::Settings::with_text_size(14,style);
    settings.h1_size=22.into();settings.h2_size=19.into();settings.h3_size=17.into();
    settings.h4_size=15.into();settings.h5_size=14.into();settings.h6_size=14.into();
    settings.code_size=12.into();settings.spacing=10.into();settings
}
struct ReleaseNotesViewer;
impl<'a> markdown::Viewer<'a,Msg> for ReleaseNotesViewer {
    fn on_link_click(link: markdown::Uri) -> Msg {Msg::OpenReleaseLink(link)}
    fn paragraph(&self,settings:markdown::Settings,body:&markdown::Text)->Element<'a,Msg> {
        // Native tables measure cells under an unbounded horizontal scroller; keep intrinsic width.
        iced::widget::rich_text(body.spans(settings.style)).size(settings.text_size)
            .wrapping(text::Wrapping::WordOrGlyph).on_link_click(Self::on_link_click).into()
    }
    fn heading(&self,mut settings:markdown::Settings,level:&'a markdown::HeadingLevel,body:&'a markdown::Text,index:usize)->Element<'a,Msg> {
        settings.style.font.weight=iced::font::Weight::Semibold;
        let size=match level {markdown::HeadingLevel::H1=>settings.h1_size,markdown::HeadingLevel::H2=>settings.h2_size,
            markdown::HeadingLevel::H3=>settings.h3_size,markdown::HeadingLevel::H4=>settings.h4_size,
            markdown::HeadingLevel::H5=>settings.h5_size,markdown::HeadingLevel::H6=>settings.h6_size};
        container(iced::widget::rich_text(body.spans(settings.style)).size(size)
            .wrapping(text::Wrapping::WordOrGlyph).on_link_click(Self::on_link_click))
            .padding(iced::Padding {top:if index>0{6.0}else{0.0},..Default::default()}).into()
    }
}
pub fn release_notes_view(items:&[markdown::Item])->Element<'_,Msg> {
    if items.is_empty(){return muted("No release notes provided.").into()}
    // Native Markdown handles lists, code/table scrollers and image alt text without downloading images.
    markdown::view_with(items,release_markdown_settings(),&ReleaseNotesViewer)
}
pub fn release_notes_panel(items:&[markdown::Item],height:f32)->Element<'_,Msg> {
    container(iced::widget::scrollable(container(release_notes_view(items))
        .width(Fill).padding(iced::Padding {right:16.0,..Default::default()})).width(Fill).height(iced::Length::Shrink))
        .padding(12).width(Fill).max_height(height).style(well).into()
}
pub fn update_review<'a>(base:Element<'a,Msg>,header:Element<'a,Msg>,notes:Element<'a,Msg>,consequences:Element<'a,Msg>,footer:Element<'a,Msg>)->Element<'a,Msg> {
    // Only release notes scroll: version, restart consequences and confirmation stay visible.
    let body=iced::widget::column![header,notes,consequences,iced::widget::rule::horizontal(1),
        row![iced::widget::Space::new().width(Fill),footer].align_y(iced::Alignment::Center)].spacing(12);
    iced::widget::stack![base,iced::widget::opaque(container(container(card(body)).max_width(760))
        .padding(28).center(Fill).style(|_|container::Style {background:Some(Color::from_rgba8(4,8,18,0.82).into()),..Default::default()}))].into()
}
pub fn modal_actions<'a>(base: Element<'a, Msg>, body: Element<'a, Msg>, footer: Element<'a, Msg>) -> Element<'a, Msg> {
    // The app's minimum height is 640px. Reserve room for the action row while
    // allowing long details to scroll independently with standard Iced widgets.
    let content = iced::widget::column![
        container(iced::widget::scrollable(container(body).padding(iced::Padding { right: 16.0, ..Default::default() })).height(iced::Length::Shrink)).max_height(400),
        iced::widget::rule::horizontal(1),
        container(footer).width(Fill).align_x(iced::alignment::Horizontal::Right),
    ].spacing(16);
    iced::widget::stack![base, iced::widget::opaque(container(container(card(content)).max_width(660)).padding(28).center(Fill)
        .style(|_| container::Style { background: Some(Color::from_rgba8(4, 8, 18, 0.82).into()), ..Default::default() }))].into()
}
pub fn tab<'a>(label: &'static str, message: Msg, selected: bool) -> button::Button<'a, Msg> {
    tab_content(text(label).size(14),message,selected)
}
fn tab_content<'a>(content: impl Into<Element<'a,Msg>>, message: Msg, selected: bool) -> button::Button<'a, Msg> {
    button(content).padding([8,14]).on_press(message).style(move |theme, status| {
        if selected { primary(theme, status) } else {
            let mut style = ghost(theme, status); style.text_color = MUTED;
            style.border = iced::Border { color: Color { a: 0.5, ..BORDER }, width: 1.0, radius: radius(4.0) }; style }
    })

}

pub fn mode_switch<'a>(advanced: bool, enabled: bool) -> Element<'a, Msg> {
    use iced::widget::tooltip;
    let mode = |label: &'static str, value: bool, help: &'static str| tooltip(
        tab_content(text(label).size(14).width(Fill).align_x(iced::Alignment::Center), Msg::Mode(value), advanced == value)
            .width(Fill).padding([7,8]).on_press_maybe(enabled.then_some(Msg::Mode(value))),
        container(text(help).size(13)).padding(12).max_width(260).style(bubble),
        tooltip::Position::Bottom,
    ).gap(8).delay(std::time::Duration::from_millis(500)).snap_within_viewport(true);
    container(row![mode("Normal", false, "Simple play, your game folder and logs."),
        mode("Advanced", true, "Server connections, hosting and game options.")].spacing(4)).padding(4).width(170)
        .style(|_: &Theme| container::Style { background: Some(WELL.into()), border: iced::Border { color: BORDER, width: 1.0, radius: radius(5.0) }, ..Default::default() }).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_note_tables_keep_finite_geometry_inside_the_bounded_notes_viewport() {
        use iced::advanced::{layout,widget::Tree};
        fn finite(node:&layout::Node) {
            let bounds=node.bounds();
            assert!([bounds.x,bounds.y,bounds.width,bounds.height].into_iter().all(f32::is_finite),
                "Markdown produced non-finite widget geometry: {bounds:?}");
            assert!(bounds.width>=0.0 && bounds.height>=0.0,"Markdown produced negative widget geometry: {bounds:?}");
            for child in node.children(){finite(child);}
        }
        let notes=markdown::parse("## Downloads\n\n| Edition | Windows | Linux |\n|---|---|---|\n| Desktop | [Download ZIP](https://example.com/desktop.zip) | [Download tar.gz](https://example.com/desktop.tar.gz) |\n| Scripts | `scripts.zip` | `scripts.tar.gz` |\n\n```text\na-long-checksum-and-filename\n```\n\n> Keep the complete bundle together.").collect::<Vec<_>>();
        let renderer=iced::Renderer::new(iced::Font::default(),14.into());
        for height in [100.0,190.0,260.0] {
            let mut content=release_notes_panel(&notes,height);let mut tree=Tree::new(content.as_widget());
            let layout=content.as_widget_mut().layout(&mut tree,&renderer,&layout::Limits::new(iced::Size::ZERO,iced::Size::new(680.0,height)));
            finite(&layout);
            assert!(layout.bounds().height<=height && layout.bounds().width<=680.0,"notes must stay in their viewport");
        }
    }
}
