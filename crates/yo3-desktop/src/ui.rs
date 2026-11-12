//! Small compositions of standard Iced widgets; the library owns layout, focus and interaction.
use iced::{Element, Color, Theme, Length::Fill};
use iced::widget::{button, container, row, text, text_input, svg, markdown};
use crate::Msg;
pub const MUTED: Color = Color::from_rgb8(151, 165, 184);
pub const GOLD: Color = Color::from_rgb8(216, 182, 117);
pub const GOOD: Color = Color::from_rgb8(110, 204, 170);
pub const BAD: Color = Color::from_rgb8(241, 153, 163);
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone { Neutral, Info, Success, Warning, Danger }
pub fn tone_color(tone: Tone) -> Color {
    match tone { Tone::Neutral => MUTED, Tone::Info => Color::from_rgb8(130, 190, 242),
        Tone::Success => GOOD, Tone::Warning => Color::from_rgb8(235, 192, 112), Tone::Danger => BAD }
}
fn tone_icon(tone: Tone) -> Icon {
    match tone { Tone::Neutral => Icon::Stop, Tone::Info => Icon::Info, Tone::Success => Icon::Check,
        Tone::Warning => Icon::Warning, Tone::Danger => Icon::Alert }
}
#[derive(Clone, Copy)]
pub enum Icon { Layers, Play, Server, Users, Activity, Settings, Folder, External, Download, Down, Right, Check, Plus, Upload, Copy, Close, Refresh, Book, Shield, Stop, Info, Warning, Alert }
pub fn icon<'a>(which: Icon) -> svg::Svg<'a> {
    let bytes: &'static [u8] = match which {
        Icon::Layers => include_bytes!("../assets/icons/layers.svg"), Icon::Play => include_bytes!("../assets/icons/play.svg"),
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
fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::text(theme, status);
    style.border = iced::Border { color: Color::from_rgb8(61, 76, 95), width: 1.0, radius: 6.0.into() };
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background = Some(Color::from_rgb8(32, 45, 62).into());
    }
    style
}
pub fn disclosure<'a>(label: &'static str, expanded: bool, message: Msg) -> button::Button<'a, Msg> {
    action(if expanded { Icon::Down } else { Icon::Right }, label, Some(message)).width(Fill).style(button::text)
}
pub fn card<'a>(content: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    container(content).width(Fill).padding(20).style(|theme: &Theme| container::Style {
        background: Some(Color::from_rgb8(25, 33, 45).into()), text_color: Some(theme.palette().text),
        border: iced::Border { color: Color::from_rgb8(42, 53, 69), width: 1.0, radius: 12.0.into() }, ..Default::default()
    })
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
        .padding(14).width(Fill).height(iced::Length::Shrink).style(container::rounded_box)
}
pub fn muted<'a>(label: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(label).size(13).color(MUTED).width(Fill)
}
pub fn report_filter<'a>(checked:bool,enabled:bool)->Element<'a,Msg> {
    let control=iced::widget::checkbox(checked).label("Exclude chat, decks, paths & addresses")
        .size(14).text_size(13).spacing(8).on_toggle_maybe(enabled.then_some(Msg::RemovePersonal));
    iced::widget::tooltip(control,container(text("Filters player activity and detailed game traces, and masks common paths and network/email addresses in this report. Original logs stay unchanged. Review before sharing.").size(12))
        .padding(10).max_width(340).style(container::rounded_box),iced::widget::tooltip::Position::Top).into()
}
pub fn label_help<'a>(label: &'static str, help: &'static str) -> Element<'a, Msg> {
    use iced::widget::tooltip;
    let hint = container(text("?").size(12).color(MUTED)).width(20).height(20).center_x(20).center_y(20)
        .style(container::rounded_box);
    row![text(label).size(15), tooltip(hint, container(text(help).size(13)).padding(12).max_width(300)
        .style(container::rounded_box), tooltip::Position::Top).gap(8).snap_within_viewport(true)]
        .spacing(8).align_y(iced::Alignment::Center).into()
}
pub fn status_badge<'a>(label: String, tone: Tone) -> Element<'a, Msg> {
    let color = tone_color(tone);
    container(row![icon(tone_icon(tone)).style(move |_,_| svg::Style { color: Some(color) }), text(label).size(12).color(color)].spacing(7).align_y(iced::Alignment::Center))
        .padding([6, 10]).style(move |_| container::Style { background: Some(Color { a: 0.08, ..color }.into()), border: iced::Border { color: Color { a: 0.22, ..color }, width: 1.0, radius: 20.0.into() }, ..Default::default() }).into()
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
        border: iced::Border { color: Color { a: 0.35, ..color }, width: 1.0, radius: 8.0.into() }, ..Default::default()
    })
}
pub fn input_style(theme: &Theme, status: text_input::Status, invalid: bool) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.border.radius = 6.0.into();
    if invalid {
        style.border.color = if matches!(status,text_input::Status::Disabled) { Color { a: 0.65, ..BAD } } else { BAD };
        style.border.width = if matches!(status,text_input::Status::Focused {..}) { 2.0 } else { 1.0 };
    }
    style
}
pub fn release_markdown_settings() -> markdown::Settings {
    let mut style=markdown::Style::from(Theme::Dark);
    style.font=iced::Font::with_name("Fira Sans");style.link_color=tone_color(Tone::Info);
    style.inline_code_color=Color::from_rgb8(212,222,237);
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
        .padding(12).width(Fill).max_height(height).style(container::rounded_box).into()
}
pub fn update_review<'a>(base:Element<'a,Msg>,header:Element<'a,Msg>,notes:Element<'a,Msg>,consequences:Element<'a,Msg>,footer:Element<'a,Msg>)->Element<'a,Msg> {
    // Only release notes scroll: version, restart consequences and confirmation stay visible.
    let body=iced::widget::column![header,notes,consequences,iced::widget::rule::horizontal(1),
        row![iced::widget::Space::new().width(Fill),footer].align_y(iced::Alignment::Center)].spacing(12);
    iced::widget::stack![base,iced::widget::opaque(container(container(card(body)).max_width(760))
        .padding(28).center(Fill).style(|_|container::Style {background:Some(Color::from_rgba8(5,8,14,0.85).into()),..Default::default()}))].into()
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
        .style(|_| container::Style { background: Some(Color::from_rgba8(5, 8, 14, 0.85).into()), ..Default::default() }))].into()
}
pub fn tab<'a>(label: &'static str, message: Msg, selected: bool) -> button::Button<'a, Msg> {
    tab_content(text(label).size(14),message,selected)
}
fn tab_content<'a>(content: impl Into<Element<'a,Msg>>, message: Msg, selected: bool) -> button::Button<'a, Msg> {
    button(content).padding([10,14]).on_press(message).style(move |theme, status| {
        let mut style = button::text(theme, status);
        if selected { style.background = Some(Color::from_rgb8(36,57,79).into()); }
        style.border.radius = 8.0.into(); style
    })

}

pub fn mode_switch<'a>(advanced: bool, enabled: bool) -> Element<'a, Msg> {
    use iced::widget::tooltip;
    let mode = |label: &'static str, value: bool, help: &'static str| tooltip(
        tab_content(text(label).size(14).width(Fill).align_x(iced::Alignment::Center), Msg::Mode(value), advanced == value)
            .width(Fill).padding([10,8]).on_press_maybe(enabled.then_some(Msg::Mode(value))),
        container(text(help).size(13)).padding(12).max_width(260).style(container::rounded_box),
        tooltip::Position::Bottom,
    ).gap(8).delay(std::time::Duration::from_millis(500)).snap_within_viewport(true);
    row![mode("Normal", false, "Simple play, your game folder and logs."),
        mode("Advanced", true, "Server connections, hosting and game options.")].spacing(4).width(162).into()
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
