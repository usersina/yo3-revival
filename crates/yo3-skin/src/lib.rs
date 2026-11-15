//! The launcher's skin kit: the look of the game's own menus, drawn from scratch on iced. Brushed-silver chrome frames
//! with bolts, navy pill buttons with silver rims, light-grey panel bodies with dark text, and a deep-blue circuit
//! backdrop with glowing nodes. Nothing here comes from the game's files: the shapes are vector drawings (canvas), so
//! they stay sharp at any display scale, and the fonts are Barlow and Barlow Condensed (SIL OFL, assets/fonts/OFL.txt).
//! The colours were taken from the game's Select Player screen; docs/launcher-style.md is the style guide.
use std::cell::RefCell;
use std::sync::LazyLock;
use iced::font::{Style as FontStyle, Weight};
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke, gradient::Linear};
use iced::widget::{button, center, checkbox, container, mouse_area, opaque, pick_list, stack, svg, text, text_input};
use iced::{Background, Border, Color, ContentFit, Element, Font, Length::Fill as Grow, Padding, Pixels, Point, Rectangle,
    Renderer, Size, Theme, mouse};

/// The four font files, for `iced::application(..).font(..)`.
pub const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Barlow-Regular.ttf"),
    include_bytes!("../assets/fonts/Barlow-SemiBold.ttf"),
    include_bytes!("../assets/fonts/BarlowCondensed-Bold.ttf"),
    include_bytes!("../assets/fonts/BarlowCondensed-ExtraBoldItalic.ttf"),
];
/// Body text, the application's default font.
pub const BODY: Font = Font::with_name("Barlow");
/// Labels, list headlines and small buttons.
pub const BODY_BOLD: Font = Font { weight: Weight::Semibold, ..BODY };
/// Headings and the title strip of a frame.
pub const DISPLAY: Font = Font { weight: Weight::Bold, ..Font::with_name("Barlow Condensed") };
/// The big buttons, tabs and the logo: bold, condensed and slanted, like the game's location banners.
pub const DISPLAY_ITALIC: Font = Font { weight: Weight::ExtraBold, style: FontStyle::Italic, ..Font::with_name("Barlow Condensed") };

/// The palette. Names say where a colour is used, not what it looks like.
pub mod color {
    use iced::Color;
    const fn hex(value: u32) -> Color { Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8) }
    /// Navy button faces, from the top of the bevel to the bottom (game: #354c94 to #22295d).
    pub const NAVY_TOP: Color = hex(0x4a63b5);
    pub const NAVY: Color = hex(0x26306a);
    pub const NAVY_LOW: Color = hex(0x1b2357);
    pub const NAVY_BASE: Color = hex(0x2b3779);
    /// The PLAY button: brighter, so the one action that matters stands out.
    pub const PLAY_TOP: Color = hex(0x4c93ff);
    pub const PLAY: Color = hex(0x2160d2);
    pub const PLAY_LOW: Color = hex(0x17449f);
    pub const PLAY_BASE: Color = hex(0x2a63c9);
    /// Silver rims and chrome (game: #a9b3bd).
    pub const CHROME_LIGHT: Color = hex(0xf4f6f8);
    pub const CHROME: Color = hex(0xaab3bd);
    pub const CHROME_DARK: Color = hex(0x7b848f);
    pub const CHROME_EDGE: Color = hex(0x4c545e);
    /// Panel bodies (game: #eeeeee to #bebebe) and the text on them.
    pub const PANEL_TOP: Color = hex(0xf1f1f1);
    pub const PANEL: Color = hex(0xd4d4d4);
    pub const PANEL_LOW: Color = hex(0xbebebe);
    pub const ROW: Color = hex(0xe6e6e6);
    pub const ROW_ALT: Color = hex(0xd9d9d9);
    pub const INK: Color = hex(0x1b2130);
    pub const INK_SOFT: Color = hex(0x4d5566);
    /// A frame's title strip (game: #052d52) and the cyan line under it.
    pub const STRIP_TOP: Color = hex(0x11507f);
    pub const STRIP: Color = hex(0x063058);
    pub const STRIP_LOW: Color = hex(0x021a33);
    pub const GLOW: Color = hex(0x43b6ff);
    /// Disabled buttons (game: #36373a) and their text.
    pub const STEEL_TOP: Color = hex(0x5d5f63);
    pub const STEEL: Color = hex(0x3a3b3e);
    pub const STEEL_TEXT: Color = hex(0x8c8e93);
    /// The title bar and footer.
    pub const BAR_TOP: Color = hex(0x22345c);
    pub const BAR: Color = hex(0x0d1834);
    pub const BAR_LOW: Color = hex(0x070e22);
    pub const ON_DARK: Color = hex(0xeef3fb);
    pub const ON_DARK_SOFT: Color = hex(0x9fb3d4);
    /// States.
    pub const GOOD: Color = hex(0x2f9e4f);
    pub const GOOD_ON_DARK: Color = hex(0x6fdc8c);
    pub const WARN: Color = hex(0xf0b43c);
    pub const BAD: Color = hex(0xc0392b);
    pub const LINK: Color = hex(0x1f4fb4);
    pub const LINK_HOVER: Color = hex(0x3a78f0);
    pub const CLOSE: Color = hex(0xc4383a);
}
use color::*;

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }
}
fn alpha(c: Color, a: f32) -> Color { Color { a, ..c } }
fn inset(r: Rectangle, d: f32) -> Rectangle { Rectangle { x: r.x + d, y: r.y + d, width: r.width - 2.0 * d, height: r.height - 2.0 * d } }
/// A top-to-bottom gradient across `r`.
fn vertical(r: Rectangle, stops: &[(f32, Color)]) -> Fill {
    let mut linear = Linear::new(Point::new(r.x, r.y), Point::new(r.x, r.y + r.height));
    for &(offset, c) in stops { linear = linear.add_stop(offset, c); }
    Fill { style: canvas::Style::Gradient(linear.into()), ..Fill::default() }
}
/// The game's button outline: a bar with chamfered ends.
fn chamfer(r: Rectangle, cut: f32) -> Path {
    // A button squeezed narrower than its own ends stays a hexagon instead of folding over itself.
    let cut = cut.min((r.width / 2.0 - 1.0).max(0.0));
    Path::new(|b| {
        let (mid, right, bottom) = (r.y + r.height / 2.0, r.x + r.width, r.y + r.height);
        b.move_to(Point::new(r.x + cut, r.y)); b.line_to(Point::new(right - cut, r.y)); b.line_to(Point::new(right, mid));
        b.line_to(Point::new(right - cut, bottom)); b.line_to(Point::new(r.x + cut, bottom)); b.line_to(Point::new(r.x, mid));
        b.close();
    })
}
/// A tab: a parallelogram leaning right, like the game's location banners.
fn slant(r: Rectangle, lean: f32) -> Path {
    Path::new(|b| {
        b.move_to(Point::new(r.x + lean, r.y)); b.line_to(Point::new(r.x + r.width, r.y));
        b.line_to(Point::new(r.x + r.width - lean, r.y + r.height)); b.line_to(Point::new(r.x, r.y + r.height)); b.close();
    })
}
fn label(frame: &mut Frame, content: &str, at: Point, size: f32, font: Font, ink: Color, shadow: bool) {
    let mut t = canvas::Text { content: content.to_owned(), position: at, color: ink, size: Pixels(size), font,
        align_x: text::Alignment::Center, align_y: iced::alignment::Vertical::Center, ..canvas::Text::default() };
    if shadow {
        let mut s = t.clone();
        s.position = Point::new(at.x + 1.0, at.y + 1.0); s.color = alpha(Color::BLACK, 0.55);
        frame.fill_text(s);
    }
    t.position = at;
    frame.fill_text(t);
}

/// Geometry drawn once and reused until its key changes. tiny-skia repaints only the regions whose primitives changed,
/// and it can only tell for cached geometry: an uncached canvas makes it repaint the whole window on every frame.
#[derive(Default)]
pub struct Memo { cache: canvas::Cache, key: RefCell<String> }
impl Memo {
    fn draw(&self, renderer: &Renderer, size: Size, key: String, paint: impl FnOnce(&mut Frame)) -> Geometry {
        if *self.key.borrow() != key { self.cache.clear(); *self.key.borrow_mut() = key; }
        self.cache.draw(renderer, size, paint)
    }
}

/// What a [`pill`] looks like.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The game's navy button with a silver rim.
    Primary,
    /// The one big action of a screen: PLAY, LOG IN.
    Play,
    /// A tab in the title bar.
    Tab { active: bool },
    /// A window control in the title bar. Close turns red under the pointer.
    Window(Glyph),
}
/// The symbol on a window control, drawn as lines so no font needs the character.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Glyph { Settings, Minimize, Close }

struct Pill<M> { label: String, on_press: Option<M>, kind: Kind }
/// Whether the left button went down on this pill and is still held.
#[derive(Default)]
pub struct PillState { hovered: bool, pressed: bool, memo: Memo }

impl<M: Clone> canvas::Program<M> for Pill<M> {
    type State = PillState;
    fn update(&self, state: &mut PillState, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<M>> {
        let over = cursor.is_over(bounds);
        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) if over != state.hovered => {
                state.hovered = over;
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over && self.on_press.is_some() => {
                state.pressed = true;
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.pressed => {
                state.pressed = false;
                match (&self.on_press, over) {
                    (Some(message), true) => Some(canvas::Action::publish(message.clone()).and_capture()),
                    _ => Some(canvas::Action::request_redraw()),
                }
            }
            _ => None,
        }
    }
    fn draw(&self, state: &PillState, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let enabled = self.on_press.is_some();
        let hovered = enabled && cursor.is_over(bounds);
        let pressed = hovered && state.pressed;
        let key = format!("{:?}|{}|{enabled}|{hovered}|{pressed}", self.kind, self.label);
        vec![state.memo.draw(renderer, bounds.size(), key, |frame| {
        let whole = Rectangle::new(Point::ORIGIN, bounds.size());
        let shift = if pressed { 1.0 } else { 0.0 };
        match self.kind {
            Kind::Primary | Kind::Play => {
                let play = self.kind == Kind::Play;
                let outer = inset(whole, if play { 4.0 } else { 2.0 });
                let cut = outer.height * 0.32;
                if enabled && (hovered || play) {
                    frame.stroke(&chamfer(outer, cut), Stroke::default().with_color(alpha(GLOW, if hovered { 0.55 } else { 0.22 })).with_width(if play { 6.0 } else { 3.0 }));
                }
                frame.fill(&chamfer(outer, cut), vertical(outer, &[(0.0, CHROME_LIGHT), (0.45, CHROME), (0.55, CHROME_DARK), (1.0, CHROME_LIGHT)]));
                let rim = if play { 3.0 } else { 2.0 };
                let face = inset(outer, rim);
                let (top, mid, low, base) = match (enabled, play) {
                    (false, _) => (STEEL_TOP, STEEL, STEEL, STEEL_TOP),
                    (true, true) => (PLAY_TOP, PLAY, PLAY_LOW, PLAY_BASE),
                    (true, false) => (NAVY_TOP, NAVY, NAVY_LOW, NAVY_BASE),
                };
                let tint = |c: Color| if pressed { mix(c, Color::BLACK, 0.22) } else if hovered { mix(c, Color::WHITE, 0.14) } else { c };
                frame.fill(&chamfer(face, cut * face.height / outer.height),
                    vertical(face, &[(0.0, tint(top)), (0.49, tint(mid)), (0.51, tint(low)), (1.0, tint(base))]));
                frame.stroke(&chamfer(face, cut * face.height / outer.height), Stroke::default().with_color(alpha(CHROME_EDGE, 0.8)).with_width(1.0));
                let ink = if enabled { Color::WHITE } else { STEEL_TEXT };
                let (size, font) = if play { (outer.height * 0.52, DISPLAY_ITALIC) } else { (outer.height * 0.5, BODY_BOLD) };
                label(frame, &self.label, Point::new(whole.center_x(), whole.center_y() + shift), size, font, ink, enabled);
            }
            Kind::Tab { active } => {
                let shape = inset(whole, 1.0);
                let lean = shape.height * 0.45;
                if active {
                    frame.fill(&slant(shape, lean), vertical(shape, &[(0.0, CHROME_LIGHT), (0.5, CHROME), (1.0, CHROME_DARK)]));
                    let face = inset(shape, 2.0);
                    frame.fill(&slant(face, lean * face.height / shape.height), vertical(face, &[(0.0, NAVY_TOP), (0.49, NAVY), (0.51, NAVY_LOW), (1.0, NAVY_BASE)]));
                    frame.fill_rectangle(Point::new(face.x + lean * 0.3, face.y + face.height - 2.5), Size::new(face.width - lean * 0.6, 2.5), GLOW);
                } else {
                    let face = alpha(if hovered { BAR_TOP } else { BAR }, 0.9);
                    frame.fill(&slant(shape, lean), face);
                    frame.stroke(&slant(shape, lean), Stroke::default().with_color(alpha(CHROME, if hovered { 0.6 } else { 0.3 })).with_width(1.0));
                }
                let ink = if active || hovered { Color::WHITE } else { ON_DARK_SOFT };
                label(frame, &self.label, Point::new(whole.center_x(), whole.center_y() + shift), whole.height * 0.52, DISPLAY_ITALIC, ink, true);
            }
            Kind::Window(glyph) => {
                let shape = inset(whole, 2.0);
                let face = match (glyph, hovered) {
                    (Glyph::Close, true) => (mix(CLOSE, Color::WHITE, 0.1), CLOSE),
                    (_, true) => (BAR_TOP, mix(BAR_TOP, Color::BLACK, 0.3)),
                    _ => (alpha(BAR_TOP, 0.6), alpha(BAR, 0.6)),
                };
                let path = Path::rounded_rectangle(shape.position(), shape.size(), 3.0.into());
                frame.fill(&path, vertical(shape, &[(0.0, face.0), (1.0, face.1)]));
                frame.stroke(&path, Stroke::default().with_color(alpha(CHROME, if hovered { 0.8 } else { 0.35 })).with_width(1.0));
                let c = Point::new(whole.center_x(), whole.center_y() + shift);
                let s = whole.height.min(whole.width) * 0.2;
                let pen = Stroke::default().with_color(if enabled { ON_DARK } else { alpha(ON_DARK_SOFT, 0.35) }).with_width(1.8).with_line_cap(canvas::LineCap::Round);
                match glyph {
                    Glyph::Minimize => frame.stroke(&Path::line(Point::new(c.x - s, c.y + s * 0.6), Point::new(c.x + s, c.y + s * 0.6)), pen),
                    Glyph::Close => {
                        frame.stroke(&Path::line(Point::new(c.x - s, c.y - s), Point::new(c.x + s, c.y + s)), pen);
                        frame.stroke(&Path::line(Point::new(c.x + s, c.y - s), Point::new(c.x - s, c.y + s)), pen);
                    }
                    Glyph::Settings => {
                        frame.stroke(&Path::circle(c, s * 0.75), pen);
                        for i in 0..8u8 {
                            let a = f32::from(i) * std::f32::consts::FRAC_PI_4;
                            let (dx, dy) = (a.cos(), a.sin());
                            frame.stroke(&Path::line(Point::new(c.x + dx * s * 1.05, c.y + dy * s * 1.05), Point::new(c.x + dx * s * 1.5, c.y + dy * s * 1.5)), pen);
                        }
                    }
                }
            }
        }
        })]
    }
    fn mouse_interaction(&self, _state: &PillState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if self.on_press.is_some() && cursor.is_over(bounds) { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
}

/// A button in the game's style. `None` draws it disabled. Give it a size: the shape fills it.
pub fn pill<'a, M: Clone + 'a>(label: impl Into<String>, on_press: Option<M>, kind: Kind, width: impl Into<iced::Length>, height: impl Into<iced::Length>) -> Element<'a, M> {
    canvas::Canvas::new(Pill { label: label.into(), on_press, kind }).width(width).height(height).into()
}

struct Logo;
impl<M> canvas::Program<M> for Logo {
    type State = Memo;
    fn draw(&self, state: &Memo, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        vec![state.draw(renderer, bounds.size(), String::new(), |frame| {
        let (cy, r) = (bounds.height / 2.0, bounds.height * 0.3);
        let diamond = |r: f32| Path::new(|b| {
            b.move_to(Point::new(r + 4.0, cy - r)); b.line_to(Point::new(2.0 * r + 4.0, cy)); b.line_to(Point::new(r + 4.0, cy + r));
            b.line_to(Point::new(4.0, cy)); b.close();
        });
        frame.stroke(&diamond(r), Stroke::default().with_color(alpha(GLOW, 0.35)).with_width(5.0));
        frame.fill(&diamond(r), vertical(Rectangle::new(Point::new(4.0, cy - r), Size::new(2.0 * r, 2.0 * r)), &[(0.0, Color::WHITE), (0.5, GLOW), (1.0, PLAY)]));
        let at = Point::new(2.0 * r + 14.0, cy);
        let t = canvas::Text { content: "YO3 REVIVAL".into(), position: Point::new(at.x + 1.0, at.y + 1.0), color: alpha(Color::BLACK, 0.6), size: Pixels(bounds.height * 0.62),
            font: DISPLAY_ITALIC, align_y: iced::alignment::Vertical::Center, ..canvas::Text::default() };
        frame.fill_text(t.clone());
        frame.fill_text(canvas::Text { position: at, color: Color::WHITE, ..t });
        })]
    }
}
/// The launcher's name with its diamond mark, for the title bar.
pub fn logo<'a, M: 'a>(height: f32) -> Element<'a, M> {
    canvas::Canvas::new(Logo).width(height * 5.2).height(height).into()
}

/// The parts of the window that are not panels: the title bar strip, the footer strip.
#[derive(Clone, Copy, Debug)]
enum Strip { Title, Footer }
impl<M> canvas::Program<M> for Strip {
    type State = Memo;
    fn draw(&self, state: &Memo, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        vec![state.draw(renderer, bounds.size(), format!("{self:?}"), |frame| {
        let r = Rectangle::new(Point::ORIGIN, bounds.size());
        match self {
            Strip::Title => {
                frame.fill_rectangle(r.position(), r.size(), vertical(r, &[(0.0, BAR_TOP), (0.5, BAR), (1.0, BAR_LOW)]));
                frame.fill_rectangle(Point::new(0.0, r.height - 2.0), Size::new(r.width, 1.0), alpha(CHROME, 0.7));
                frame.fill_rectangle(Point::new(0.0, r.height - 1.0), Size::new(r.width, 1.0), alpha(GLOW, 0.5));
            }
            Strip::Footer => {
                frame.fill_rectangle(r.position(), r.size(), vertical(r, &[(0.0, alpha(BAR_TOP, 0.92)), (1.0, alpha(BAR_LOW, 0.96))]));
                frame.fill_rectangle(Point::ORIGIN, Size::new(r.width, 1.0), alpha(CHROME, 0.55));
            }
        }
        })]
    }
}

/// A chrome frame: silver rim with bolts, an optional navy title strip, a light-grey body.
struct Panel { title: Option<String> }
/// Height of a frame's title strip, and the rim's width.
const STRIP_HEIGHT: f32 = 30.0;
const RIM: f32 = 6.0;
impl<M> canvas::Program<M> for Panel {
    type State = Memo;
    fn draw(&self, state: &Memo, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        vec![state.draw(renderer, bounds.size(), self.title.clone().unwrap_or_default(), |frame| {
        let outer = inset(Rectangle::new(Point::ORIGIN, bounds.size()), 1.0);
        let shadow = Rectangle { y: outer.y + 3.0, ..outer };
        frame.fill(&Path::rounded_rectangle(shadow.position(), shadow.size(), 8.0.into()), alpha(Color::BLACK, 0.45));
        let rim = Path::rounded_rectangle(outer.position(), Size::new(outer.width, outer.height - 2.0), 8.0.into());
        frame.fill(&rim, vertical(outer, &[(0.0, CHROME_LIGHT), (0.06, CHROME), (0.5, CHROME_DARK), (0.94, CHROME), (1.0, CHROME_EDGE)]));
        frame.stroke(&rim, Stroke::default().with_color(alpha(Color::WHITE, 0.55)).with_width(1.0));
        let body = Rectangle { height: outer.height - 2.0 - 2.0 * RIM, ..inset(outer, RIM) };
        let body_path = Path::rounded_rectangle(body.position(), body.size(), 3.0.into());
        frame.fill(&body_path, vertical(body, &[(0.0, PANEL_TOP), (0.55, PANEL), (1.0, PANEL_LOW)]));
        if let Some(title) = &self.title {
            let strip = Rectangle { height: STRIP_HEIGHT, ..body };
            frame.fill(&Path::rounded_rectangle(strip.position(), strip.size(), 3.0.into()), vertical(strip, &[(0.0, STRIP_TOP), (0.5, STRIP), (1.0, STRIP_LOW)]));
            for i in 0..7u8 {
                let x = strip.x + strip.width - 18.0 - f32::from(i) * 9.0;
                let stripe = Path::line(Point::new(x, strip.y + 5.0), Point::new(x - 9.0, strip.y + strip.height - 5.0));
                frame.stroke(&stripe, Stroke::default().with_color(alpha(GLOW, 0.12 + 0.05 * f32::from(i))).with_width(3.0));
            }
            frame.fill_rectangle(Point::new(strip.x, strip.y + strip.height - 1.5), Size::new(strip.width, 1.5), alpha(GLOW, 0.9));
            let mut t = canvas::Text { content: title.to_uppercase(), position: Point::new(strip.x + 13.0, strip.center_y()), color: alpha(Color::BLACK, 0.6),
                size: Pixels(19.0), font: DISPLAY_ITALIC, align_y: iced::alignment::Vertical::Center, ..canvas::Text::default() };
            frame.fill_text(canvas::Text { position: Point::new(strip.x + 14.0, strip.center_y() + 1.0), ..t.clone() });
            t.color = Color::WHITE;
            frame.fill_text(t);
        }
        frame.stroke(&body_path, Stroke::default().with_color(CHROME_EDGE).with_width(1.0));
        let (top, bottom) = (outer.y + RIM / 2.0, outer.y + outer.height - 2.0 - RIM / 2.0);
        for (x, y) in [(outer.x + 16.0, top), (outer.x + outer.width - 16.0, top), (outer.x + 16.0, bottom), (outer.x + outer.width - 16.0, bottom)] {
            let bolt = Path::circle(Point::new(x, y), 2.3);
            frame.fill(&bolt, vertical(Rectangle::new(Point::new(x - 2.3, y - 2.3), Size::new(4.6, 4.6)), &[(0.0, Color::WHITE), (1.0, CHROME_EDGE)]));
            frame.stroke(&bolt, Stroke::default().with_color(alpha(CHROME_EDGE, 0.9)).with_width(0.8));
        }
        })]
    }
}

/// A chrome frame around `content`. It fills the space it is given; size it with a container.
pub fn panel<'a, M: 'a>(title: Option<&str>, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    let top = if title.is_some() { RIM + STRIP_HEIGHT + 12.0 } else { RIM + 14.0 };
    stack![
        canvas::Canvas::new(Panel { title: title.map(str::to_owned) }).width(Grow).height(Grow),
        container(content).padding(Padding { top, right: RIM + 14.0, bottom: RIM + 16.0, left: RIM + 14.0 }).width(Grow).height(Grow),
    ].into()
}

/// The title bar's background, behind its content.
pub fn title_strip<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    stack![canvas::Canvas::new(Strip::Title).width(Grow).height(Grow), content.into()].into()
}
/// The footer's background, behind its content.
pub fn footer_strip<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    stack![canvas::Canvas::new(Strip::Footer).width(Grow).height(Grow), content.into()].into()
}

/// One transparent rectangle over the whole window, drawn fresh every frame. tiny-skia then repaints the whole window
/// instead of only the regions it believes changed: in iced 0.14 that partial repaint can leave a newly shown text
/// unpainted until the next frame (seen on X11 with a status line). The GPU renderer repaints everything anyway.
struct Repaint;
impl<M> canvas::Program<M> for Repaint {
    type State = ();
    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);
        vec![frame.into_geometry()]
    }
}

static BACKDROP: LazyLock<svg::Handle> = LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/backdrop.svg").as_slice()));
/// The window's backdrop: the deep-blue circuit board, cropped to fill. It also makes every frame a full repaint (see
/// [`Repaint`]), so put it at the bottom of the window's stack.
pub fn backdrop<'a, M: 'a>() -> Element<'a, M> {
    stack![svg(BACKDROP.clone()).content_fit(ContentFit::Cover).width(Grow).height(Grow), canvas::Canvas::new(Repaint).width(Grow).height(Grow)].into()
}

/// A dark overlay over `base` with `dialog` centred on it. A click outside the dialog sends `on_blur`.
pub fn modal<'a, M: Clone + 'a>(base: impl Into<Element<'a, M>>, dialog: impl Into<Element<'a, M>>, on_blur: M) -> Element<'a, M> {
    let shade = container(center(opaque(dialog))).style(|_| container::Style { background: Some(alpha(Color::BLACK, 0.6).into()), ..container::Style::default() });
    stack![base.into(), opaque(mouse_area(shade).on_press(on_blur))].into()
}

/// A text link: on a panel (`on_dark` false) or on the dark bars.
pub fn link<'a, M: Clone + 'a>(label: impl text::IntoFragment<'a>, on_press: M, on_dark: bool) -> Element<'a, M> {
    button(text(label).size(14).font(BODY_BOLD)).padding(0).on_press(on_press).style(move |_, status| {
        let hover = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let ink = match (on_dark, hover) { (true, false) => ON_DARK_SOFT, (true, true) => Color::WHITE, (false, false) => LINK, (false, true) => LINK_HOVER };
        button::Style { background: None, text_color: ink, ..button::Style::default() }
    }).into()
}

/// A text field on a panel: white well, dark text, a blue ring when focused.
pub fn field(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    let hovered = matches!(status, text_input::Status::Hovered | text_input::Status::Focused { is_hovered: true });
    text_input::Style {
        background: Background::Color(if matches!(status, text_input::Status::Disabled) { ROW_ALT } else { Color::from_rgb8(0xfb, 0xfc, 0xfd) }),
        border: Border { color: if focused { LINK_HOVER } else if hovered { CHROME_EDGE } else { CHROME_DARK }, width: if focused { 2.0 } else { 1.0 }, radius: 3.0.into() },
        icon: NAVY, placeholder: Color::from_rgb8(0x8a, 0x90, 0x99), value: INK, selection: Color::from_rgb8(0xa8, 0xcc, 0xff),
    }
}

/// A checkbox on a panel.
pub fn tick(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (checked, hovered) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true),
        checkbox::Status::Disabled { is_checked } => (is_checked, false),
    };
    checkbox::Style {
        background: Background::Color(if checked { NAVY } else { Color::WHITE }),
        icon_color: Color::WHITE,
        border: Border { color: if hovered { LINK_HOVER } else { CHROME_EDGE }, width: 1.0, radius: 2.0.into() },
        text_color: Some(INK),
    }
}

/// A drop-down on a panel.
pub fn choice(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let open = matches!(status, pick_list::Status::Hovered | pick_list::Status::Opened { .. });
    pick_list::Style {
        text_color: INK, placeholder_color: INK_SOFT, handle_color: NAVY,
        background: Background::Color(Color::from_rgb8(0xfb, 0xfc, 0xfd)),
        border: Border { color: if open { LINK_HOVER } else { CHROME_DARK }, width: 1.0, radius: 3.0.into() },
    }
}
/// The open list of a drop-down.
pub fn choice_menu(_theme: &Theme) -> iced::overlay::menu::Style {
    iced::overlay::menu::Style {
        background: Background::Color(PANEL_TOP), border: Border { color: CHROME_EDGE, width: 1.0, radius: 3.0.into() },
        text_color: INK, selected_text_color: Color::WHITE, selected_background: Background::Color(NAVY),
        shadow: iced::Shadow { color: alpha(Color::BLACK, 0.4), offset: iced::Vector::new(0.0, 3.0), blur_radius: 8.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::canvas::Program;

    /// iced picks a font by family name: a name that matches no loaded face silently falls back to another font.
    #[test]
    fn font_constants_name_the_bundled_families() {
        let utf16 = |name: &str| name.encode_utf16().flat_map(u16::to_be_bytes).collect::<Vec<u8>>();
        let has = |font: &[u8], name: &str| font.windows(utf16(name).len()).any(|w| w == utf16(name).as_slice());
        let family = |font: Font| match font.family { iced::font::Family::Name(name) => name, other => panic!("{other:?}") };
        assert!(FONTS[..2].iter().all(|f| has(f, family(BODY))));
        assert!(FONTS[2..].iter().all(|f| has(f, family(DISPLAY)) && has(f, family(DISPLAY_ITALIC))));
    }

    fn press(button: mouse::Event) -> canvas::Event { canvas::Event::Mouse(button) }
    const BOUNDS: Rectangle = Rectangle { x: 10.0, y: 10.0, width: 100.0, height: 40.0 };
    const INSIDE: mouse::Cursor = mouse::Cursor::Available(Point::new(50.0, 30.0));
    const OUTSIDE: mouse::Cursor = mouse::Cursor::Available(Point::new(500.0, 30.0));

    /// A pill sends its message on a release over it after a press over it, like a native button.
    #[test]
    fn pill_clicks_only_on_press_and_release_inside() {
        let pill = Pill { label: "PLAY".into(), on_press: Some(7), kind: Kind::Play };
        let mut state = PillState::default();
        let down = pill.update(&mut state, &press(mouse::Event::ButtonPressed(mouse::Button::Left)), BOUNDS, INSIDE);
        assert!(down.is_some());
        let up = pill.update(&mut state, &press(mouse::Event::ButtonReleased(mouse::Button::Left)), BOUNDS, INSIDE).map(canvas::Action::into_inner);
        assert!(matches!(up, Some((Some(7), _, iced::event::Status::Captured))));

        pill.update(&mut state, &press(mouse::Event::ButtonPressed(mouse::Button::Left)), BOUNDS, INSIDE);
        let dragged_off = pill.update(&mut state, &press(mouse::Event::ButtonReleased(mouse::Button::Left)), BOUNDS, OUTSIDE).map(canvas::Action::into_inner);
        assert!(matches!(dragged_off, Some((None, _, _))), "releasing outside cancels the click");
    }

    /// A disabled pill (no message) ignores presses and leaves them to the widgets around it.
    #[test]
    fn disabled_pill_ignores_presses() {
        let pill: Pill<u8> = Pill { label: "STARTING…".into(), on_press: None, kind: Kind::Play };
        let mut state = PillState::default();
        assert!(pill.update(&mut state, &press(mouse::Event::ButtonPressed(mouse::Button::Left)), BOUNDS, INSIDE).is_none());
        assert!(pill.update(&mut state, &press(mouse::Event::ButtonReleased(mouse::Button::Left)), BOUNDS, INSIDE).is_none());
    }
}
