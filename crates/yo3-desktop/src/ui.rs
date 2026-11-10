//! Small compositions of standard Iced widgets; the library owns layout, focus and interaction.
use iced::{Element, Color, Theme, Length::Fill};
use iced::widget::{button, container, row, text, svg};
use crate::Msg;
pub const MUTED: Color = Color::from_rgb8(151, 165, 184);
pub const GOLD: Color = Color::from_rgb8(216, 182, 117);
pub const GOOD: Color = Color::from_rgb8(110, 204, 170);
pub const BAD: Color = Color::from_rgb8(241, 153, 163);
#[derive(Clone, Copy)]
pub enum Icon { Layers, Play, Server, Users, Activity, Settings, Folder, External, Download, Down, Right, Check, Plus, Upload, Copy, Close, Refresh, Book, Shield, Stop }
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
    };
    svg(svg::Handle::from_memory(bytes)).width(18).height(18).style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}
pub fn action<'a>(which: Icon, label: &'static str, message: Option<Msg>) -> button::Button<'a, Msg> {
    let opacity: f32 = if message.is_some() { 1.0 } else { 0.35 };
    button(row![icon(which).opacity(opacity), text(label).size(14)].spacing(9).align_y(iced::Alignment::Center))
        .padding([10, 14]).on_press_maybe(message).style(button::secondary)
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
pub fn muted<'a>(label: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(label).size(13).color(MUTED).width(Fill)
}
pub fn label_help<'a>(label: &'static str, help: &'static str) -> Element<'a, Msg> {
    use iced::widget::tooltip;
    let hint = container(text("?").size(12).color(MUTED)).width(20).height(20).center_x(20).center_y(20)
        .style(container::rounded_box);
    row![text(label).size(15), tooltip(hint, container(text(help).size(13)).padding(12).max_width(300)
        .style(container::rounded_box), tooltip::Position::Top).gap(8).snap_within_viewport(true)]
        .spacing(8).align_y(iced::Alignment::Center).into()
}
pub fn badge<'a>(label: String, good: bool) -> Element<'a, Msg> {
    let color = if good { GOOD } else { MUTED };
    container(row![icon(if good { Icon::Check } else { Icon::Stop }).style(move |_,_| svg::Style { color: Some(color) }), text(label).size(12).color(color)].spacing(7).align_y(iced::Alignment::Center))
        .padding([6, 10]).style(move |_| container::Style { background: Some(Color { a: 0.08, ..color }.into()), border: iced::Border { radius: 20.0.into(), ..Default::default() }, ..Default::default() }).into()
}
pub fn modal<'a>(base: Element<'a, Msg>, content: Element<'a, Msg>) -> Element<'a, Msg> {
    iced::widget::stack![base, iced::widget::opaque(container(container(iced::widget::scrollable(card(content)).height(iced::Length::Shrink)).max_width(660)).padding(28).center(Fill)
        .style(|_| container::Style { background: Some(Color::from_rgba8(5, 8, 14, 0.85).into()), ..Default::default() }))].into()
}
pub fn tab<'a>(label: &'static str, message: Msg, selected: bool) -> button::Button<'a, Msg> {
    button(text(label).size(14)).padding([10,14]).on_press(message).style(move |theme, status| {
        let mut style = button::text(theme, status);
        if selected { style.background = Some(Color::from_rgb8(36,57,79).into()); }
        style.border.radius = 8.0.into(); style
    })

}
