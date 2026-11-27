//! Host for friends (#37), the window side: a dialog from the Multiplayer tab that starts your hosted server, shows the
//! address and invite link to share, makes invite codes, opens the admin panel and stops hosting.
use std::time::Duration;
use iced::widget::{Space, column, container, row, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Subscription, Task};
use yo3_desktop::hosting::{self, Activity, Hosted};
use yo3_desktop::options::Change;
use yo3_skin::{BODY_BOLD, Icon, Kind, Rule, color, pill};
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
    /// The invites made this time, newest first: each a code for one friend and the link that carries it.
    invites: Vec<(String, String)>,
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
        if let Some(backend) = &ctx.backend { backend.diagnostics.error(error) }
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
                        self.invites.clear();
                        self.note = None;
                        return Task::done(Message::Tick);
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
                    Ok(()) => { self.hosted = None; self.activity = Activity::default(); self.panel = None; self.invites.clear(); self.good("Your server is stopped.") }
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
            // Each friend gets their own link: it carries a single-use code, and goes to the clipboard at once.
            Message::Invited(Ok(code)) => {
                let Some(hosted) = &self.hosted else { return Task::none() };
                let link = yo3_desktop::multiplayer::link_with_code(&hosted.server, &code);
                self.invites.insert(0, (code, link.clone()));
                self.invites.truncate(3);
                return Task::done(Message::Copy(link));
            }
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
        let action = match &self.hosted {
            Some(_) => pill("Stop hosting", self.busy.is_none().then_some(Message::Stop), Kind::Danger, 150, 38),
            None => pill("Start hosting", self.busy.is_none().then_some(Message::Start), Kind::Primary, 170, 38),
        };
        let height = if self.hosted.is_some() { 500.0 } else { 310.0 };
        yo3_skin::dialog("Host for friends", Some(Message::Help("host")), Message::Close, 600.0, height,
            column![col, Space::new().height(Fill), row![Space::new().width(Fill), action]])
    }

    fn form(&self) -> Element<'_, Message> {
        let input = |label: &'static str, placeholder: &str, value: &str, field: Field, tip: Option<&'static str>| row![
            row![text(label).size(14).font(BODY_BOLD).color(color::INK)].extend(tip.map(yo3_skin::help_tip)).spacing(6).align_y(Center).width(140),
            text_input(placeholder, value).on_input(move |v| Message::Edit(field, v)).on_submit(Message::Start).style(yo3_skin::field).padding(7).size(15),
        ].align_y(Center);
        let lan = hosting::lan_address().map_or_else(|| "this PC's network address".to_owned(), |ip| ip.to_string());
        column![
            text("A server on this PC for your friends, with its own cards and accounts. It runs while YO3 Revival is open.").size(15).color(color::INK),
            input("Server name", hosting::DEFAULT_NAME, &self.name, Field::Name, None),
            input("Port", "7443", &self.port, Field::Port, None),
            input("Address to share", &lan, &self.share, Field::Share, Some("Empty: your network address, for friends on your network. \
                Friends elsewhere need your public address, with the port forwarded on your router.")),
        ].spacing(10).into()
    }

    fn running<'a>(&'a self, hosted: &'a Hosted) -> Element<'a, Message> {
        let heading = |words: &'static str| text(words).size(16).font(yo3_skin::DISPLAY).color(color::NAVY);
        let well = |content: Element<'a, Message>| container(content).padding(iced::Padding { top: 2.0, right: 2.0, bottom: 2.0, left: 10.0 }).width(Fill)
            .style(|_| container::Style { background: Some(color::PANEL_TOP.into()), border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() });
        let online = match self.activity.online { 0 => "Nobody online yet".to_owned(), 1 => "1 player online".to_owned(), n => format!("{n} players online") };
        let mut col = column![
            row![container(Space::new().width(10).height(10)).style(|_| dot(color::GOOD)),
                text(format!("{} is running", hosted.server.name)).size(15).font(BODY_BOLD).color(color::INK), Space::new().width(Fill),
                text(online).size(14).color(color::INK_SOFT)].spacing(8).align_y(Center),
            row![heading("Invite a friend"), yo3_skin::help_tip("Each friend gets their own link. It works once, for a week, and fills in their sign-up: \
                they paste it under Multiplayer, ⋯, Add a server.")].spacing(6).align_y(Center),
            row![pill("Copy an invite link", self.busy.is_none().then_some(Message::Invite), Kind::Primary, 200, 34)],
        ].spacing(10);
        for (i, (code, link)) in self.invites.iter().enumerate() {
            let words = if i == 0 { format!("Copied: {code}. Send it to one friend.") } else { format!("Earlier: {code}") };
            col = col.push(row![yo3_skin::rule(if i == 0 { Rule::Met } else { Rule::Pending }, words), Space::new().width(Fill),
                yo3_skin::icon_button(Icon::Copy, Some(Message::Copy(link.clone())), "Copy this invite again", false)].align_y(Center));
        }
        col = col.push(heading("Your server")).push(row![
            text("Address").size(14).font(BODY_BOLD).color(color::INK).width(80),
            well(row![text(&hosted.server.address).size(14).color(color::INK).width(Fill),
                yo3_skin::icon_button(Icon::Copy, Some(Message::Copy(hosted.server.address.clone())), "Copy the address", false)].align_y(Center).into()),
        ].spacing(10).align_y(Center));
        if hosted.local_network {
            col = col.push(row![text("Works on your network only.").size(13).color(color::INK_SOFT),
                yo3_skin::help_tip("For friends elsewhere, stop hosting and type your public address, with the port forwarded on your router.")].spacing(6).align_y(Center));
        }
        let mut tools = row![pill("Check reachability", self.busy.is_none().then_some(Message::Reach), Kind::Secondary, 170, 32)].spacing(8);
        if let Some(panel) = &self.panel { tools = tools.push(pill("Admin panel", Some(Message::Open(panel.clone())), Kind::Secondary, 130, 32)) }
        col = col.push(tools);
        if self.panel.is_some() {
            // The panel signs in with an admin account: the host's own, made here (it has no terminal on the server).
            col = col.push(row![
                row![text("Admin").size(14).font(BODY_BOLD).color(color::INK), yo3_skin::help_tip("The admin panel signs in with an admin account: make yours one.")]
                    .spacing(6).align_y(Center).width(80),
                text_input("Your User ID on this server", &self.admin).on_input(|v| Message::Edit(Field::Admin, v))
                    .on_submit(Message::MakeAdmin).style(yo3_skin::field).padding(6).size(14).width(Fill),
                pill("Make admin", (self.busy.is_none() && !self.admin.trim().is_empty()).then_some(Message::MakeAdmin), Kind::Primary, 120, 32)]
                .spacing(8).align_y(Center));
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
        // One invite per friend: its link carries the code, for their sign-up.
        let _ = state.update(Message::Invited(Ok("ABCD-EFGH-JKLM-NPQR".into())), &mut ctx);
        assert_eq!(state.invites[0].0, "ABCD-EFGH-JKLM-NPQR");
        assert_eq!(yo3_desktop::multiplayer::invite_code(&state.invites[0].1).as_deref(), Some("ABCD-EFGH-JKLM-NPQR"));
        let _ = state.update(Message::Stopped(Ok(())), &mut ctx);
        assert!(state.hosted.is_none() && state.invites.is_empty());
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
