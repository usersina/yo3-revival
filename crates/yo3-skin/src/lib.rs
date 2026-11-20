//! The launcher's skin kit: the look of the game's own menus, drawn from scratch on iced. Brushed-silver chrome frames
//! with bolts, navy pill buttons with silver rims, light-grey panel bodies with dark text, and a deep-blue circuit
//! backdrop with glowing nodes. Nothing here comes from the game's files: the shapes are vector drawings (canvas), so
//! they stay sharp at any display scale, and the fonts are Barlow and Barlow Condensed (SIL OFL, assets/fonts/OFL.txt).
//! The colours were taken from the game's Select Player screen; docs/launcher-style.md is the style guide.
use std::cell::RefCell;
use std::sync::LazyLock;
use iced::font::{Style as FontStyle, Weight};
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke, gradient::Linear};
use iced::widget::{button, center, checkbox, container, image, mouse_area, opaque, pick_list, stack, svg, text, text_input, tooltip};
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
    /// A destructive button's face (Stop hosting, Sign out), from the top of the bevel to the bottom.
    pub const DANGER_TOP: Color = hex(0xe0675c);
    pub const DANGER: Color = hex(0xb4342a);
    pub const DANGER_LOW: Color = hex(0x8f2219);
    pub const DANGER_BASE: Color = hex(0xa83228);
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
    /// The game's navy button with a silver rim: a form's or a dialog's main action.
    Primary,
    /// A way back or out (Back, Cancel, Close): a light silver face with navy text, quieter than the main action.
    Secondary,
    /// An action that ends or removes something (Stop hosting, Sign out): a red face.
    Danger,
    /// The one big action of a screen: PLAY, LOG IN.
    Play,
    /// A tab in the title bar.
    Tab { active: bool },
    /// A window control in the title bar. Close turns red under the pointer.
    Window(Glyph),
}
/// The symbol on a window control, drawn as lines so no font needs the character.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Glyph { Settings, Minimize, Close, Bell, Help }

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
            Kind::Primary | Kind::Secondary | Kind::Danger | Kind::Play => {
                let play = self.kind == Kind::Play;
                let big = play || whole.height >= 50.0;
                let outer = inset(whole, if play { 4.0 } else { 2.0 });
                let cut = outer.height * 0.32;
                if enabled && (hovered || play) {
                    frame.stroke(&chamfer(outer, cut), Stroke::default().with_color(alpha(GLOW, if hovered { 0.55 } else { 0.22 })).with_width(if play { 6.0 } else { 3.0 }));
                }
                // A disabled small button is flat and grey on the panel, so it never reads as a live one; the big action keeps
                // its steel face, since its label says what is happening (STARTING…).
                let flat = !enabled && !play;
                if flat {
                    frame.fill(&chamfer(outer, cut), alpha(CHROME_DARK, 0.45));
                } else {
                    frame.fill(&chamfer(outer, cut), vertical(outer, &[(0.0, CHROME_LIGHT), (0.45, CHROME), (0.55, CHROME_DARK), (1.0, CHROME_LIGHT)]));
                }
                let rim = if play { 3.0 } else if flat { 1.0 } else { 2.0 };
                let face = inset(outer, rim);
                let (top, mid, low, base) = match (enabled, self.kind) {
                    (false, Kind::Play) => (STEEL_TOP, STEEL, STEEL, STEEL_TOP),
                    // A main action not ready yet: navy faded into the panel, flat, so it differs from the light way back.
                    (false, _) => { let faded = mix(NAVY_TOP, PANEL, 0.62); (faded, faded, faded, faded) }
                    (true, Kind::Play) => (PLAY_TOP, PLAY, PLAY_LOW, PLAY_BASE),
                    (true, Kind::Secondary) => (Color::WHITE, PANEL_TOP, ROW_ALT, PANEL_TOP),
                    (true, Kind::Danger) => (DANGER_TOP, DANGER, DANGER_LOW, DANGER_BASE),
                    (true, _) => (NAVY_TOP, NAVY, NAVY_LOW, NAVY_BASE),
                };
                let tint = |c: Color| if pressed { mix(c, Color::BLACK, 0.22) } else if hovered { mix(c, Color::WHITE, 0.14) } else { c };
                frame.fill(&chamfer(face, cut * face.height / outer.height),
                    vertical(face, &[(0.0, tint(top)), (0.49, tint(mid)), (0.51, tint(low)), (1.0, tint(base))]));
                frame.stroke(&chamfer(face, cut * face.height / outer.height), Stroke::default().with_color(alpha(CHROME_EDGE, 0.8)).with_width(1.0));
                let ink = match (enabled, self.kind) { (false, Kind::Play) => STEEL_TEXT, (false, _) => alpha(Color::WHITE, 0.85), (true, Kind::Secondary) => NAVY, _ => Color::WHITE };
                let (size, font) = if big { (outer.height * 0.52, DISPLAY_ITALIC) } else { (outer.height * 0.46, BODY_BOLD) };
                label(frame, &self.label, Point::new(whole.center_x(), whole.center_y() + shift), size, font, ink, enabled && self.kind != Kind::Secondary);
            }
            Kind::Tab { active } => {
                let shape = inset(whole, 1.0);
                let lean = shape.height * 0.45;
                if active {
                    frame.fill(&slant(shape, lean), vertical(shape, &[(0.0, CHROME_LIGHT), (0.5, CHROME), (1.0, CHROME_DARK)]));
                    let face = inset(shape, 2.0);
                    frame.fill(&slant(face, lean * face.height / shape.height), vertical(face, &[(0.0, NAVY_TOP), (0.49, NAVY), (0.51, NAVY_LOW), (1.0, NAVY_BASE)]));
                    // The underline is the tab's own bottom edge, so it leans with the tab instead of poking out.
                    let face_lean = lean * face.height / shape.height;
                    // Along the slanted sides: at height y the left edge is at x + lean·(1 − y/h), the right one at
                    // x + w − lean·y/h.
                    let (top, bottom) = (face.height - 2.5, face.height);
                    let left = |y: f32| face.x + face_lean * (1.0 - y / face.height);
                    let right = |y: f32| face.x + face.width - face_lean * y / face.height;
                    let edge = Path::new(|b| {
                        b.move_to(Point::new(left(top), face.y + top)); b.line_to(Point::new(right(top), face.y + top));
                        b.line_to(Point::new(right(bottom), face.y + bottom)); b.line_to(Point::new(left(bottom), face.y + bottom)); b.close();
                    });
                    frame.fill(&edge, GLOW);
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
                    Glyph::Bell => {
                        // A bell: a rounded body flaring to its rim, a clapper under it and a nub on top.
                        let (top, rim) = (c.y - s * 1.25, c.y + s * 0.8);
                        let body = Path::new(|b| {
                            b.move_to(Point::new(c.x - s * 1.35, rim));
                            b.bezier_curve_to(Point::new(c.x - s * 0.8, rim - s * 0.35), Point::new(c.x - s * 1.05, top), Point::new(c.x, top));
                            b.bezier_curve_to(Point::new(c.x + s * 1.05, top), Point::new(c.x + s * 0.8, rim - s * 0.35), Point::new(c.x + s * 1.35, rim));
                            b.close();
                        });
                        frame.stroke(&body, pen);
                        frame.stroke(&Path::line(Point::new(c.x, top), Point::new(c.x, top - s * 0.3)), pen);
                        frame.fill(&Path::circle(Point::new(c.x, rim + s * 0.42), s * 0.28), if enabled { ON_DARK } else { alpha(ON_DARK_SOFT, 0.35) });
                    }
                    Glyph::Help => label(frame, "?", c, whole.height * 0.6, BODY_BOLD, if enabled { ON_DARK } else { ON_DARK_SOFT }, false),
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

/// The launcher's name, after a mark: the drawn diamond, or nothing when the real icon stands beside it.
struct Logo { mark: bool }
impl<M> canvas::Program<M> for Logo {
    type State = Memo;
    fn draw(&self, state: &Memo, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        vec![state.draw(renderer, bounds.size(), format!("{}", self.mark), |frame| {
        let (cy, r) = (bounds.height / 2.0, bounds.height * 0.3);
        let diamond = |r: f32| Path::new(|b| {
            b.move_to(Point::new(r + 4.0, cy - r)); b.line_to(Point::new(2.0 * r + 4.0, cy)); b.line_to(Point::new(r + 4.0, cy + r));
            b.line_to(Point::new(4.0, cy)); b.close();
        });
        if self.mark {
            frame.stroke(&diamond(r), Stroke::default().with_color(alpha(GLOW, 0.35)).with_width(5.0));
            frame.fill(&diamond(r), vertical(Rectangle::new(Point::new(4.0, cy - r), Size::new(2.0 * r, 2.0 * r)), &[(0.0, Color::WHITE), (0.5, GLOW), (1.0, PLAY)]));
        }
        let at = Point::new(if self.mark { 2.0 * r + 14.0 } else { 2.0 }, cy);
        let t = canvas::Text { content: "YO3 REVIVAL".into(), position: Point::new(at.x + 1.0, at.y + 1.0), color: alpha(Color::BLACK, 0.6), size: Pixels(bounds.height * 0.62),
            font: DISPLAY_ITALIC, align_y: iced::alignment::Vertical::Center, ..canvas::Text::default() };
        frame.fill_text(t.clone());
        frame.fill_text(canvas::Text { position: at, color: Color::WHITE, ..t });
        })]
    }
}
/// The launcher's name with its diamond mark, for the title bar.
/// The launcher's name for the title bar: after the app's own mark when given (the launcher passes its
/// background-less icon, drawn a little larger than the text so the Duel Disk reads), else after a drawn diamond (the
/// examples, which have no icon of their own).
pub fn logo<'a, M: 'a>(icon: Option<image::Handle>, height: f32) -> Element<'a, M> {
    match icon {
        Some(icon) => iced::widget::row![
            image(icon).width(height * 1.25).height(height * 1.25).filter_method(image::FilterMethod::Linear),
            canvas::Canvas::new(Logo { mark: false }).width(height * 4.4).height(height),
        ].spacing(8).align_y(iced::Alignment::Center).into(),
        None => canvas::Canvas::new(Logo { mark: true }).width(height * 5.2).height(height).into(),
    }
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

/// A link on the dark bars led by a heart, like the footer's other links (the footer's Support).
pub fn heart_link<'a, M: Clone + 'a>(label: impl text::IntoFragment<'a>, on_press: M) -> Element<'a, M> { icon_link(Icon::Heart, label, on_press, true) }

/// A text link in red on a panel, for an action that ends something (Sign out).
pub fn danger_link<'a, M: Clone + 'a>(label: impl text::IntoFragment<'a>, on_press: M) -> Element<'a, M> {
    button(text(label).size(14).font(BODY_BOLD)).padding(0).on_press(on_press).style(|_, status| {
        let hover = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style { background: None, text_color: if hover { DANGER_TOP } else { DANGER }, ..button::Style::default() }
    }).into()
}

/// The kit's line icons (24 px grid), drawn in one colour: the widget tints them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon { Copy, Save, Help, Flag, Heart, More, Check, Cross, Pending, Plus, Trash, Server, User, Open, Bell, Link, Refresh }

static ICONS: LazyLock<Vec<svg::Handle>> = LazyLock::new(|| {
    const LINE: &str = "fill='none' stroke='#000' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'";
    let shapes: [String; 17] = [
        format!("<g {LINE}><rect x='9' y='9' width='12' height='12' rx='2'/><path d='M5 15V5a2 2 0 0 1 2-2h10'/></g>"),
        format!("<g {LINE}><path d='M12 3v12M7 10l5 5 5-5M4 20h16'/></g>"),
        format!("<g {LINE}><circle cx='12' cy='12' r='9'/><path d='M9.4 9.3a2.7 2.7 0 1 1 3.8 2.5c-.7.3-1.2 1-1.2 1.8v.4'/></g><circle cx='12' cy='17.3' r='1.2'/>"),
        format!("<g {LINE}><path d='M5 21V4M5 4h12l-2.5 4.5L17 13H5'/></g>"),
        "<path d='M12 21.3l-1.4-1.3C5.4 15.4 2 12.3 2 8.5 2 5.4 4.4 3 7.5 3c1.7 0 3.4.8 4.5 2.1C13.1 3.8 14.8 3 16.5 3 19.6 3 22 5.4 22 8.5c0 3.8-3.4 6.9-8.6 11.5L12 21.3z'/>".into(),
        "<circle cx='5' cy='12' r='2'/><circle cx='12' cy='12' r='2'/><circle cx='19' cy='12' r='2'/>".into(),
        "<g fill='none' stroke='#000' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><path d='M5 12.5l4.5 4.5L19 7'/></g>".into(),
        "<g fill='none' stroke='#000' stroke-width='3' stroke-linecap='round'><path d='M7 7l10 10M17 7L7 17'/></g>".into(),
        format!("<g {LINE}><circle cx='12' cy='12' r='5'/></g>"),
        format!("<g {LINE}><path d='M12 5v14M5 12h14'/></g>"),
        format!("<g {LINE}><path d='M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13'/></g>"),
        format!("<g {LINE}><rect x='4' y='4' width='16' height='7' rx='1.5'/><rect x='4' y='13' width='16' height='7' rx='1.5'/><path d='M8 7.5h.01M8 16.5h.01'/></g>"),
        format!("<g {LINE}><circle cx='12' cy='8' r='4'/><path d='M4 21a8 8 0 0 1 16 0'/></g>"),
        format!("<g {LINE}><path d='M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5'/></g>"),
        format!("<g {LINE}><path d='M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z'/><path d='M10 21h4'/></g>"),
        format!("<g {LINE}><path d='M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1'/></g>"),
        format!("<g {LINE}><path d='M20 11a8 8 0 1 0-2.3 5.7M20 5v6h-6'/></g>"),
    ];
    shapes.iter().map(|shape| svg::Handle::from_memory(format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'>{shape}</svg>").into_bytes())).collect()
});

/// `which` at `size` px, in `tint`.
pub fn icon<'a, M: 'a>(which: Icon, size: f32, tint: Color) -> Element<'a, M> {
    svg(ICONS[which as usize].clone()).width(size).height(size).style(move |_, _| svg::Style { color: Some(tint) }).into()
}

/// A small square button holding an icon, with `tip` shown under the pointer. On a panel unless `on_dark`.
pub fn icon_button<'a, M: Clone + 'a>(which: Icon, on_press: Option<M>, tip: &'a str, on_dark: bool) -> Element<'a, M> {
    let enabled = on_press.is_some();
    let ink = match (on_dark, enabled) { (_, false) => alpha(INK_SOFT, 0.5), (true, true) => ON_DARK_SOFT, (false, true) => LINK };
    let mut b = button(center(icon(which, 17.0, ink))).width(30).height(30).padding(0).style(move |_, status| {
        let hover = enabled && matches!(status, button::Status::Hovered | button::Status::Pressed);
        let face = match (on_dark, hover) { (_, false) => None, (true, true) => Some(alpha(BAR_TOP, 0.9)), (false, true) => Some(alpha(LINK_HOVER, 0.14)) };
        button::Style { background: face.map(Background::Color), border: Border { radius: 4.0.into(), ..Border::default() }, ..button::Style::default() }
    });
    if let Some(m) = on_press { b = b.on_press(m) }
    tooltip(b, container(text(tip).size(13).color(ON_DARK)).padding([3, 8]).style(|_| container::Style {
        background: Some(alpha(BAR_LOW, 0.95).into()), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() }),
        tooltip::Position::Bottom).gap(4).into()
}

/// A text link led by an icon in the same colour: on a panel, or on the dark bars (the footer's links).
pub fn icon_link<'a, M: Clone + 'a>(which: Icon, label: impl text::IntoFragment<'a>, on_press: M, on_dark: bool) -> Element<'a, M> {
    button(iced::widget::row![
        iced::widget::svg(ICONS[which as usize].clone()).width(15).height(15).style(move |_, status| {
            let hover = matches!(status, svg::Status::Hovered);
            svg::Style { color: Some(match (on_dark, hover) { (true, false) => ON_DARK_SOFT, (true, true) => Color::WHITE, (false, false) => LINK, (false, true) => LINK_HOVER }) }
        }),
        text(label).size(14).font(BODY_BOLD),
    ].spacing(6).align_y(iced::Alignment::Center)).padding(0).on_press(on_press).style(move |_, status| {
        let hover = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let ink = match (on_dark, hover) { (true, false) => ON_DARK_SOFT, (true, true) => Color::WHITE, (false, false) => LINK, (false, true) => LINK_HOVER };
        button::Style { background: None, text_color: ink, ..button::Style::default() }
    }).into()
}

/// One entry of a [`menu`]: its icon, its words, what it sends, and whether it ends or removes something (red).
pub struct Item<M> { pub icon: Icon, pub label: String, pub on_press: M, pub danger: bool }

/// A drop-down card of actions, for a popover beside the control that opened it.
pub fn menu<'a, M: Clone + 'a>(items: Vec<Item<M>>, width: f32) -> Element<'a, M> {
    let mut col = iced::widget::column![].spacing(2);
    for item in items {
        let ink = if item.danger { DANGER } else { INK };
        col = col.push(button(iced::widget::row![icon(item.icon, 16.0, if item.danger { DANGER } else { NAVY }), text(item.label).size(14).font(BODY_BOLD).color(ink)]
            .spacing(10).align_y(iced::Alignment::Center)).width(Grow).padding([7, 10]).on_press(item.on_press).style(|_, status| {
                let hover = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style { background: hover.then_some(Background::Color(ROW_ALT)), border: Border { radius: 3.0.into(), ..Border::default() }, ..button::Style::default() }
            }));
    }
    container(col).padding(4).width(width).style(|_| container::Style {
        background: Some(PANEL_TOP.into()), border: Border { color: CHROME_EDGE, width: 1.0, radius: 4.0.into() },
        shadow: iced::Shadow { color: alpha(Color::BLACK, 0.45), offset: iced::Vector::new(0.0, 4.0), blur_radius: 12.0 }, ..container::Style::default() }).into()
}

/// A dialog: a chrome frame `width`×`height` titled `title`, with the help **?** (when given) and the **×** in its title
/// strip. Its content ends with the actions, the main one rightmost.
pub fn dialog<'a, M: Clone + 'a>(title: &str, help: Option<M>, close: M, width: f32, height: f32, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    let mut controls = iced::widget::row![].spacing(3);
    if let Some(help) = help { controls = controls.push(pill("", Some(help), Kind::Window(Glyph::Help), 26, 22)) }
    controls = controls.push(pill("", Some(close), Kind::Window(Glyph::Close), 26, 22));
    container(stack![
        panel(Some(title), content),
        container(iced::widget::row![iced::widget::Space::new().width(Grow), controls]).padding(Padding { top: RIM + 4.0, right: RIM + 6.0, bottom: 0.0, left: 0.0 }),
    ]).width(width).height(height).into()
}

/// Where a form's rule stands: not typed yet, met, or broken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rule { Pending, Met, Broken }

/// One line of a form's checklist: its mark and its words, green once met, red when broken.
pub fn rule<'a, M: 'a>(state: Rule, words: impl text::IntoFragment<'a>) -> Element<'a, M> {
    let (mark, tint) = match state { Rule::Pending => (Icon::Pending, INK_SOFT), Rule::Met => (Icon::Check, GOOD), Rule::Broken => (Icon::Cross, BAD) };
    iced::widget::row![icon(mark, 13.0, tint), text(words).size(13).color(if state == Rule::Pending { INK_SOFT } else { tint })]
        .spacing(7).align_y(iced::Alignment::Center).into()
}

/// What a [`progress_bar`] shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bar {
    /// Waiting for something whose length is not known yet ("Starting…"): a band that travels to and fro.
    Starting,
    /// How far along, from 0 to 1.
    Fraction(f32),
    /// Finished: full, in green.
    Done,
    /// Stopped by a problem after getting this far: in red.
    Failed(f32),
}
impl Bar {
    /// `done` of `total` steps; nothing to do counts as full.
    pub fn of(done: u32, total: u32) -> Self { Bar::Fraction(if total == 0 { 1.0 } else { (done as f32 / total as f32).clamp(0.0, 1.0) }) }
    /// Where the fill ends, 0 to 1, when the length is known.
    pub fn target(self) -> Option<f32> {
        match self { Bar::Starting => None, Bar::Fraction(f) | Bar::Failed(f) => Some(f.clamp(0.0, 1.0)), Bar::Done => Some(1.0) }
    }
}

/// One step of the fill toward its target: it closes most of the gap in a fifth of a second and never overshoots.
fn ease(shown: f32, target: f32, seconds: f32) -> f32 {
    let next = shown + (target - shown) * (1.0 - (-seconds * 12.0).exp());
    if (target - next).abs() < 0.001 { target } else { next }
}
/// The travelling band's left edge, 0 to 1 of the room it has, `seconds` after the start: there and back every 2.4 s.
fn band_at(seconds: f32) -> f32 {
    let t = (seconds / 1.2).rem_euclid(2.0);
    let t = if t > 1.0 { 2.0 - t } else { t };
    t * t * (3.0 - 2.0 * t)
}

struct ProgressBar { bar: Bar, label: String }
#[derive(Default)]
pub struct BarState { shown: f32, last: Option<std::time::Instant>, start: Option<std::time::Instant>, seconds: f32, memo: Memo }

impl<M> canvas::Program<M> for ProgressBar {
    type State = BarState;
    fn update(&self, state: &mut BarState, event: &canvas::Event, _bounds: Rectangle, _cursor: mouse::Cursor) -> Option<canvas::Action<M>> {
        let canvas::Event::Window(iced::window::Event::RedrawRequested(now)) = event else { return None };
        let start = *state.start.get_or_insert(*now);
        let dt = state.last.map_or(0.0, |last| now.saturating_duration_since(last).as_secs_f32());
        state.last = Some(*now);
        match self.bar.target() {
            None => { state.seconds = now.saturating_duration_since(start).as_secs_f32(); Some(canvas::Action::request_redraw()) }
            Some(target) => {
                state.shown = ease(state.shown, target, dt.max(1.0 / 60.0));
                (state.shown != target).then(canvas::Action::request_redraw)
            }
        }
    }
    fn draw(&self, state: &BarState, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let shown = if self.bar.target().is_some() { state.shown } else { band_at(state.seconds) };
        let key = format!("{:?}|{}|{}|{}", std::mem::discriminant(&self.bar), self.label, (shown * 400.0).round(), bounds.width);
        let bar = self.bar;
        vec![state.memo.draw(renderer, bounds.size(), key, |frame| {
            let whole = Rectangle::new(Point::ORIGIN, bounds.size());
            let cut = whole.height / 2.0;
            // The rim, then the dark well the fill runs in.
            frame.fill(&chamfer(whole, cut), vertical(whole, &[(0.0, CHROME_LIGHT), (0.5, CHROME), (1.0, CHROME_DARK)]));
            let well = inset(whole, 2.0);
            frame.fill(&chamfer(well, cut - 2.0), vertical(well, &[(0.0, BAR_LOW), (1.0, BAR)]));
            let room = inset(well, 1.0);
            let (x, width) = match bar.target() {
                None => { let band = room.width * 0.3; (room.x + shown * (room.width - band), band) }
                Some(_) => (room.x, room.width * shown),
            };
            if width > 1.0 {
                let fill = Rectangle { x, y: room.y, width, height: room.height };
                let (top, mid, low) = match bar { Bar::Done => (GOOD_ON_DARK, GOOD, GOOD), Bar::Failed(_) => (DANGER_TOP, DANGER, DANGER_LOW), _ => (PLAY_TOP, PLAY, PLAY_LOW) };
                frame.fill(&chamfer(fill, cut - 3.0), vertical(fill, &[(0.0, top), (0.5, mid), (1.0, low)]));
                // A light line along the top of the bevel, and a glow on the leading edge while it is still moving.
                frame.stroke(&Path::line(Point::new(fill.x + cut * 0.6, fill.y + 1.5), Point::new(fill.x + fill.width - cut * 0.6, fill.y + 1.5)),
                    Stroke::default().with_color(alpha(Color::WHITE, 0.35)).with_width(1.0));
                if matches!(bar, Bar::Fraction(_)) && fill.width < room.width - 2.0 {
                    let edge = fill.x + fill.width;
                    frame.stroke(&Path::line(Point::new(edge - 1.0, fill.y + 2.0), Point::new(edge - 1.0, fill.y + fill.height - 2.0)),
                        Stroke::default().with_color(alpha(GLOW, 0.9)).with_width(2.0));
                }
            }
            label(frame, &self.label, Point::new(whole.width / 2.0, whole.height / 2.0), (whole.height * 0.55).clamp(11.0, 15.0), BODY_BOLD, ON_DARK, true);
        })]
    }
}

/// A progress bar in the skin: a silver rim, a dark well and a navy fill that glides to each new value, `label` on it ("12 of 418").
/// Fills the width it is given. [`Bar::Starting`] shows a travelling band for waits of unknown length, [`Bar::Done`] turns green
/// and [`Bar::Failed`] red; it only animates while there is something to animate.
pub fn progress_bar<'a, M: 'a>(bar: Bar, label: impl Into<String>, height: f32) -> Element<'a, M> {
    canvas::Canvas::new(ProgressBar { bar, label: label.into() }).width(Grow).height(height).into()
}

/// A small count on a corner of a control (unread notices).
pub fn badge<'a, M: 'a>(count: usize) -> Element<'a, M> {
    container(text(if count > 9 { "9+".to_string() } else { count.to_string() }).size(11).font(BODY_BOLD).color(Color::WHITE))
        .padding([0, 4]).height(15).center_y(15).style(|_| container::Style { background: Some(CLOSE.into()),
            border: Border { color: alpha(Color::BLACK, 0.4), width: 1.0, radius: 8.0.into() }, ..container::Style::default() }).into()
}

/// What a toast is about: its accent stripe's colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone { Good, Bad, Busy }

/// A toast: a card floating over the screen's corner (laid out by the window, never pushing the page around), with an
/// accent stripe on its left for good news, a problem, or something in progress, and a soft shadow.
pub fn toast<'a, M: 'a>(content: impl Into<Element<'a, M>>, tone: Tone) -> Element<'a, M> {
    let accent = match tone { Tone::Good => GOOD, Tone::Bad => BAD, Tone::Busy => GLOW };
    // The stripe is the outer card's own colour showing on its left, so it is always as tall as the toast.
    let inner = container(content).padding([10, 12]).width(Grow).style(|_| container::Style { background: Some(PANEL_TOP.into()),
        border: Border { radius: iced::border::Radius { top_right: 3.0, bottom_right: 3.0, ..Default::default() }, ..Border::default() }, ..container::Style::default() });
    container(inner).padding(Padding { left: 5.0, ..Padding::ZERO }).width(380).style(move |_| container::Style {
        background: Some(accent.into()), border: Border { color: CHROME_EDGE, width: 1.0, radius: 4.0.into() },
        shadow: iced::Shadow { color: alpha(Color::BLACK, 0.45), offset: iced::Vector::new(0.0, 4.0), blur_radius: 14.0 },
        ..container::Style::default() }).into()
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
        checkbox::Status::Disabled { is_checked } => {
            // Unavailable: greyed, so it doesn't look like a box that can be ticked.
            return checkbox::Style {
                background: Background::Color(if is_checked { INK_SOFT } else { Color::from_rgb8(0xe4, 0xe7, 0xeb) }),
                icon_color: Color::WHITE,
                border: Border { color: Color::from_rgb8(0xc3, 0xc9, 0xd0), width: 1.0, radius: 2.0.into() },
                text_color: Some(INK_SOFT),
            };
        }
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

    #[test]
    fn a_disabled_tick_looks_unavailable() {
        let active = tick(&Theme::Light, checkbox::Status::Active { is_checked: false });
        let off = tick(&Theme::Light, checkbox::Status::Disabled { is_checked: false });
        assert_ne!(active.background, off.background);
        assert_eq!(off.text_color, Some(INK_SOFT));
    }

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

    #[test]
    fn the_fill_glides_to_its_target_without_overshooting_and_lands_on_it() {
        let (mut shown, mut frames) = (0.0f32, 0);
        while shown != 0.6 { let next = ease(shown, 0.6, 1.0 / 60.0); assert!(next > shown && next <= 0.6); shown = next; frames += 1; assert!(frames < 600) }
        assert!(frames > 10, "it glides, it does not jump ({frames} frames)");
        assert_eq!(ease(0.8, 0.5, 5.0), 0.5, "a long gap lands on the target");
        assert!(ease(0.8, 0.5, 1.0 / 60.0) < 0.8);
    }

    #[test]
    fn the_bars_states_know_where_they_end_and_the_waiting_band_stays_in_its_track() {
        assert_eq!((Bar::of(0, 0), Bar::of(209, 418), Bar::of(9, 3)), (Bar::Fraction(1.0), Bar::Fraction(0.5), Bar::Fraction(1.0)));
        assert_eq!((Bar::Starting.target(), Bar::Done.target(), Bar::Failed(0.25).target(), Bar::Fraction(2.0).target()), (None, Some(1.0), Some(0.25), Some(1.0)));
        for i in 0..400 { let at = band_at(i as f32 * 0.037); assert!((0.0..=1.0).contains(&at), "{at}") }
        assert_eq!((band_at(0.0), band_at(1.2), band_at(2.4)), (0.0, 1.0, 0.0), "there and back");
    }

    /// A bar asks for the next frame only while it moves: gliding or waiting, never when it sits still.
    #[test]
    fn a_bar_redraws_only_while_it_has_something_to_animate() {
        let redraw = |bar: Bar, state: &mut BarState, at: std::time::Instant| canvas::Program::<()>::update(&ProgressBar { bar, label: String::new() }, state,
            &canvas::Event::Window(iced::window::Event::RedrawRequested(at)), BOUNDS, OUTSIDE).is_some();
        let (mut state, t0) = (BarState::default(), std::time::Instant::now());
        assert!(redraw(Bar::Fraction(0.5), &mut state, t0), "it has to glide up from nothing");
        let mut at = t0;
        while redraw(Bar::Fraction(0.5), &mut state, at) { at += std::time::Duration::from_millis(16) }
        assert_eq!(state.shown, 0.5);
        assert!(!redraw(Bar::Fraction(0.5), &mut state, at), "at rest: no more frames");
        assert!(redraw(Bar::Starting, &mut state, at), "waiting never rests");
        assert!(!redraw(Bar::Done, &mut BarState { shown: 1.0, ..BarState::default() }, at));
    }

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
