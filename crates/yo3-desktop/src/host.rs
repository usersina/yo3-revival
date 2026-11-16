//! Host for friends (#37), the window side: a dialog from the Multiplayer tab that starts your hosted server, shows the
//! address and invite link to share, makes invite codes, opens the admin panel and stops hosting.
use std::time::Duration;
use iced::widget::{Space, column, container, row, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Subscription, Task};
use yo3_desktop::hosting::{self, Activity, Hosted};
use yo3_desktop::options::Change;
use yo3_skin::{BODY_BOLD, Kind, color, pill};
use crate::app::{Ctx, dot, job};
use crate::error_presentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { Name, Port, Share, Admin }

#[derive(Clone, Debug)]
pub enum Message {
    Edit(Field, String),
    Start,
    Started(Result<Box<(yo3_desktop::Settings, Hosted)>, String>),
    Stop,
    Stopped(Result<(), String>),
    Invite,
    Invited(Result<String, String>),
    /// Make the typed User ID an admin of your server, for its admin panel.
    MakeAdmin,
    MadeAdmin(Result<(), String>),
    Reach,
    Reached(bool),
    Tick,
    Polled(Result<(Activity, Option<String>), String>),
    ChangePort,
    ToggleDetails,
    /// Handled by the window.
    Copy(String),
    /// Handled by the window: a web page.
    Open(String),
    /// Handled by the window: Help at a topic.
    Help(&'static str),
    /// Handled by the window.
    Close,
}

/// One plain sentence, the technical text behind Details.
#[derive(Clone, Debug, Default)]
struct Note { good: bool, sentence: String, details: Option<String>, open: bool, port: bool }

#[derive(Default)]
pub struct State {
    pub hosted: Option<Hosted>,
    pub activity: Activity,
    name: String,
    port: String,
    share: String,
    busy: Option<&'static str>,
    note: Option<Note>,
    codes: Vec<String>,
    panel: Option<String>,
    polling: bool,
    /// The User ID to make an admin (yours, on your server).
    admin: String,
}

impl State {
    /// The dialog opened: the form shows the saved choices.
    pub fn open(&mut self, ctx: &Ctx) {
        let s = &ctx.settings;
        self.name = if s.host_name.is_empty() { hosting::DEFAULT_NAME.into() } else { s.host_name.clone() };
        self.port = s.host_port.to_string();
        self.share = s.host_share.clone();
        if self.hosted.is_none() { self.note = None }
    }

    fn fail(&mut self, ctx: &Ctx, error: &str) {
        if let Some(backend) = &ctx.backend { backend.diagnostics.line(error) }
        let shown = error_presentation::present(error, error_presentation::Hint::General, &crate::app::masks(ctx));
        let port = shown.summary.contains("port");
        self.note = Some(Note { good: false, sentence: shown.summary, details: shown.diagnosis, open: false, port });
    }

    fn good(&mut self, sentence: &str) { self.note = Some(Note { good: true, sentence: sentence.into(), ..Note::default() }) }

    pub fn update(&mut self, message: Message, ctx: &mut Ctx) -> Task<Message> {
        match message {
            Message::Edit(field, value) => {
                let value: String = value.chars().filter(|c| !c.is_control()).take(64).collect();
                match field { Field::Name => self.name = value, Field::Port => self.port = value.chars().filter(char::is_ascii_digit).take(5).collect(), Field::Share => self.share = value, Field::Admin => self.admin = value }
            }
            Message::Start => {
                let (Some(store), Some(backend)) = (ctx.store.clone(), ctx.backend.clone()) else { return Task::none() };
                let Ok(port) = self.port.parse::<u16>() else { self.good("Choose a port from 1024 to 65535."); if let Some(n) = &mut self.note { n.good = false } return Task::none() };
                let changes = [Change::HostName(self.name.clone()), Change::HostPort(port), Change::HostShare(self.share.clone())];
                self.busy = Some("Starting your server…");
                self.note = None;
                return Task::perform(async move {
                    // The choices are saved first, each checked; a refused one is the player's to fix.
                    let settings = store.run(move |db| { for change in &changes { db.change(change)?; } db.settings() }).await.map_err(|e| format!("{e:#}"))?;
                    let started = settings.clone();
                    let hosted = tokio::task::spawn_blocking(move || backend.start_hosting(&started)).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:#}"))?;
                    Ok(Box::new((settings, hosted)))
                }, Message::Started);
            }
            Message::Started(result) => {
                self.busy = None;
                match result {
                    Ok(started) => {
                        let (settings, hosted) = *started;
                        ctx.settings = settings;
                        self.hosted = Some(hosted);
                        self.codes.clear();
                        self.note = None;
                        return Task::batch([Task::done(Message::Tick), Task::done(Message::Invite)]);
                    }
                    Err(error) => self.fail(ctx, &error),
                }
            }
            Message::Stop => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                self.busy = Some("Stopping your server…");
                return job(move || backend.stop_hosting(), Message::Stopped);
            }
            Message::Stopped(result) => {
                self.busy = None;
                match result {
                    Ok(()) => { self.hosted = None; self.activity = Activity::default(); self.panel = None; self.codes.clear(); self.good("Your server is stopped.") }
                    Err(error) => self.fail(ctx, &error),
                }
            }
            Message::Invite => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                return job(move || backend.hosting_invite(), Message::Invited);
            }
            Message::MakeAdmin => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                let user = self.admin.trim().to_owned();
                if user.is_empty() { return Task::none() }
                self.busy = Some("Making the account an admin…");
                return job(move || backend.hosting_make_admin(&user), Message::MadeAdmin);
            }
            Message::MadeAdmin(result) => {
                self.busy = None;
                match result {
                    Ok(()) => { let line = format!("{} is an admin of your server: sign in to the admin panel with it.", self.admin.trim()); self.good(&line) }
                    Err(error) => self.fail(ctx, &error),
                }
            }
            Message::Invited(Ok(code)) => { self.codes.insert(0, code); self.codes.truncate(3) }
            Message::Invited(Err(error)) => self.fail(ctx, &error),
            Message::Reach => {
                let Some(hosted) = self.hosted.clone() else { return Task::none() };
                self.busy = Some("Checking…");
                return Task::perform(async move { hosting::reachable(&hosted).await }, Message::Reached);
            }
            Message::Reached(ok) => {
                self.busy = None;
                if ok { self.good("Your server answered at the address you share.") } else {
                    self.note = Some(Note { good: false, sentence: "This PC couldn't reach your server at the address you share. Friends outside your network may not either.".into(),
                        details: None, open: false, port: false });
                }
            }
            Message::Tick => {
                let Some(backend) = ctx.backend.clone() else { return Task::none() };
                if self.hosted.is_none() || self.polling { return Task::none() }
                self.polling = true;
                return job(move || Ok((backend.hosting_activity()?, backend.hosting_panel())), Message::Polled);
            }
            Message::Polled(result) => {
                self.polling = false;
                match result {
                    Ok((activity, panel)) => { self.activity = activity; self.panel = panel }
                    // It stopped on its own (a crash, or a stop from elsewhere): say so.
                    Err(_) if ctx.backend.as_ref().is_some_and(|b| b.hosting_live().is_none()) => {
                        self.hosted = None;
                        self.activity = Activity::default();
                        self.note = Some(Note { good: false, sentence: "Your hosted server stopped.".into(), ..Note::default() });
                    }
                    Err(_) => {}
                }
            }
            Message::ChangePort => { self.note = None }
            Message::ToggleDetails => if let Some(note) = &mut self.note { note.open = !note.open },
            Message::Copy(_) | Message::Open(_) | Message::Help(_) | Message::Close => {}
        }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.hosted.is_some() { iced::time::every(Duration::from_secs(5)).map(|_| Message::Tick) } else { Subscription::none() }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = match &self.hosted { Some(hosted) => self.running(hosted), None => self.form() };
        let mut col = column![body].spacing(12);
        if let Some(busy) = self.busy { col = col.push(text(busy).size(14).color(color::INK_SOFT)) }
        if let Some(note) = &self.note { col = col.push(self.note(note)) }
        let close = pill("Close", Some(Message::Close), Kind::Primary, 120, 38);
        let buttons = match &self.hosted {
            Some(_) => row![yo3_skin::link("Help", Message::Help("host"), false), Space::new().width(Fill),
                pill("Stop hosting", self.busy.is_none().then_some(Message::Stop), Kind::Primary, 160, 38), close],
            None => row![yo3_skin::link("Help", Message::Help("host"), false), Space::new().width(Fill),
                pill("Start hosting", self.busy.is_none().then_some(Message::Start), Kind::Play, 190, 42), close],
        }.spacing(10).align_y(Center);
        container(yo3_skin::panel(Some("Host for friends"), column![col, Space::new().height(Fill), buttons])).width(640).height(540).into()
    }

    fn form(&self) -> Element<'_, Message> {
        let input = |label: &'static str, placeholder: &'static str, value: &str, field: Field| row![
            text(label).size(14).font(BODY_BOLD).color(color::INK).width(150),
            text_input(placeholder, value).on_input(move |v| Message::Edit(field, v)).on_submit(Message::Start).style(yo3_skin::field).padding(7).size(15),
        ].align_y(Center);
        let lan = hosting::lan_address().map_or_else(|| "this PC's network address".to_owned(), |ip| ip.to_string());
        column![
            text("Run a server on this PC that your friends join. It has its own cards and accounts, apart from Singleplayer, and runs while YO3 Revival is open.")
                .size(15).color(color::INK),
            input("Server name", hosting::DEFAULT_NAME, &self.name, Field::Name),
            input("Port", "7443", &self.port, Field::Port),
            input("Address to share", "", &self.share, Field::Share),
            text(format!("Leave the address empty for {lan}, which friends on your network reach. Friends elsewhere need your public address and \
                the port forwarded on your router.")).size(13).color(color::INK_SOFT),
        ].spacing(10).into()
    }

    fn running<'a>(&'a self, hosted: &'a Hosted) -> Element<'a, Message> {
        // A long value (the link) is shown cut to one line; Copy copies all of it.
        let copyable = |label: &'static str, value: &'a str| row![
            text(label).size(14).font(BODY_BOLD).color(color::INK).width(110),
            container(text(shorten(value, 52)).size(14).color(color::INK)).padding([5, 8]).width(Fill).style(|_| container::Style { background: Some(color::PANEL_TOP.into()),
                border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }),
            yo3_skin::link("Copy", Message::Copy(value.to_owned()), false),
        ].spacing(10).align_y(Center);
        let online = match self.activity.online { 0 => "Nobody is online yet.".to_owned(), 1 => "1 player online.".to_owned(), n => format!("{n} players online.") };
        let mut col = column![
            row![container(Space::new().width(10).height(10)).style(|_| dot(color::GOOD)),
                text(format!("{} is running. {online}", hosted.server.name)).size(15).font(BODY_BOLD).color(color::INK)].spacing(8).align_y(Center),
            copyable("Address", &hosted.server.address),
            copyable("Invite link", &hosted.link),
        ].spacing(10);
        if hosted.local_network {
            col = col.push(text("This address works for friends on your network. For friends elsewhere, stop hosting and type your public address.").size(13).color(color::INK_SOFT));
        }
        let mut codes = row![text("Invite codes").size(14).font(BODY_BOLD).color(color::INK).width(110)].spacing(10).align_y(Center);
        for code in &self.codes { codes = codes.push(yo3_skin::link(code.clone(), Message::Copy(code.clone()), false)) }
        codes = codes.push(Space::new().width(Fill)).push(pill("New code", self.busy.is_none().then_some(Message::Invite), Kind::Primary, 120, 30));
        col = col.push(codes).push(text("A friend pastes the link under Multiplayer, Other servers, then creates an account with a code. Each code works once, for a week. \
            Your server is in your own list too, so you create your account the same way.").size(13).color(color::INK_SOFT));
        let mut links = row![yo3_skin::link("Check reachability", Message::Reach, false)].spacing(18);
        if let Some(panel) = &self.panel { links = links.push(yo3_skin::link("Admin panel", Message::Open(panel.clone()), false)) }
        col = col.push(links);
        if self.panel.is_some() {
            // The panel signs in with an admin account: the host's own, made here (it has no terminal on the server).
            col = col.push(row![text("Admin").size(14).font(BODY_BOLD).color(color::INK).width(110),
                text_input("Your User ID on this server", &self.admin).on_input(|v| Message::Edit(Field::Admin, v))
                    .on_submit(Message::MakeAdmin).style(yo3_skin::field).padding(6).size(14).width(Fill),
                pill("Make admin", (self.busy.is_none() && !self.admin.trim().is_empty()).then_some(Message::MakeAdmin), Kind::Primary, 130, 30)]
                .spacing(10).align_y(Center));
        }
        col.into()
    }

    fn note<'a>(&'a self, note: &'a Note) -> Element<'a, Message> {
        let shade = if note.good { color::GOOD } else { color::BAD };
        let mut links = row![].spacing(14);
        if note.port && self.hosted.is_none() { links = links.push(yo3_skin::link("Change port", Message::ChangePort, false)) }
        if !note.good { links = links.push(yo3_skin::link("Help", Message::Help(if note.port { "port" } else { "reach" }), false)) }
        if note.details.is_some() { links = links.push(yo3_skin::link(if note.open { "Hide details" } else { "Details" }, Message::ToggleDetails, false)) }
        let mut col = column![row![container(Space::new().width(9).height(9)).style(move |_| dot(shade)), text(&note.sentence).size(14).font(BODY_BOLD).color(color::INK).width(Fill)]
            .spacing(8).align_y(Center)].spacing(6);
        if note.open && let Some(details) = &note.details { col = col.push(text(details).size(13).color(color::INK_SOFT)).push(yo3_skin::link("Copy", Message::Copy(details.clone()), false)) }
        col.push(links).into()
    }
}

/// `value` cut to `limit` characters, with an ellipsis.
fn shorten(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit { return value.to_owned() }
    format!("{}…", value.chars().take(limit - 1).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx { Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() } }
    fn hosted() -> Hosted {
        let server = hosting::hosted_server("Friday duels", "192.168.1.20", 7443, &"ab".repeat(32)).unwrap();
        Hosted { link: yo3_desktop::multiplayer::link_for(&server), server, local_network: true }
    }

    #[test]
    fn a_started_server_shows_what_to_share_and_a_stop_forgets_it() {
        let mut ctx = ctx();
        let mut state = State::default();
        state.open(&ctx);
        assert_eq!((state.name.as_str(), state.port.as_str()), (hosting::DEFAULT_NAME, "7443"));
        let settings = yo3_desktop::Settings { host_name: "Friday duels".into(), ..Default::default() };
        let _ = state.update(Message::Started(Ok(Box::new((settings, hosted())))), &mut ctx);
        assert_eq!(ctx.settings.host_name, "Friday duels", "the choices are the window's at once");
        assert!(state.hosted.as_ref().is_some_and(|h| h.link.starts_with("yo3revival://server?")));
        let _ = state.update(Message::Invited(Ok("ABCD-EFGH-JKLM-NPQR".into())), &mut ctx);
        assert_eq!(state.codes, ["ABCD-EFGH-JKLM-NPQR"]);
        let _ = state.update(Message::Stopped(Ok(())), &mut ctx);
        assert!(state.hosted.is_none() && state.codes.is_empty());
    }

    #[test]
    fn a_port_in_use_says_so_in_one_line_and_offers_change_port() {
        let mut ctx = ctx();
        let mut state = State::default();
        let _ = state.update(Message::Started(Err("Port 7443 is already used by another program. Close it, or change the hosting port.".into())), &mut ctx);
        let note = state.note.clone().unwrap();
        assert!(!note.good && note.port, "{note:?}");
        assert!(yo3_desktop::words::technical(&note.sentence).is_none());
        let _ = state.update(Message::ChangePort, &mut ctx);
        assert!(state.note.is_none());
    }
}
