//! The Multiplayer tab (#30): the server list, signing in, making an account and recovering one. The main screen
//! (`home.rs`) hosts it: its Play button starts the game with the session signed in here.
use std::collections::HashMap;
use std::sync::Arc;
use iced::widget::{Space, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Task};
use yo3_desktop::games::GameInfo;
use yo3_desktop::multiplayer::{self as mp, Problem, Remedy, Server, SignedIn, Status, Vault};
use yo3_skin::{BODY_BOLD, Icon, Item, Kind, Rule, color, pill};
use crate::app::{Ctx, dot};

/// Which form the server panel shows.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Form {
    #[default]
    SignIn,
    Register,
    Recover,
    /// Another server, by address or invite link.
    Add,
    /// A server met for the first time: its identity, to trust or not.
    Trust(Box<Server>),
    /// A recovery code to save, shown once.
    Code(String),
    /// Another game signed in to a different account on this server (#64).
    PlayAs,
}

/// The address typed under Add a server, as checked so far.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Found {
    #[default]
    Nothing,
    /// Waiting for typing to pause, or for the server to answer.
    Looking,
    /// A server answered there.
    Server(Box<Server>),
    /// An invite link, complete: its server and the invite code it carries.
    Link(Box<Server>, Option<String>),
    /// Not an address or a link, or nobody answered: why, in one sentence.
    No(String),
}

/// How long typing pauses before the address is looked up.
const LOOK_AFTER: std::time::Duration = std::time::Duration::from_millis(600);

/// Whether `text` is a User ID or password as the game's login screen takes it: `min` to 16 letters or digits.
fn typeable(text: &str, min: usize) -> bool { (min..=16).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_alphanumeric()) }

/// A rule for a field: nothing typed yet, met, or broken.
fn rule_for(typed: &str, met: bool) -> Rule { if typed.is_empty() { Rule::Pending } else if met { Rule::Met } else { Rule::Broken } }

/// A new password's rules, as lines: 8 to 16 letters or digits, not the User ID, and typed twice the same.
/// The characters the server picker's line holds (about 290 px of 15 px text) before it cuts the end off.
const PICKER_CHARS: usize = 38;

/// One line of the server picker: the name and the state. A long name never pushes the state out of sight: the ping
/// goes first, then the name is cut short with an ellipsis.
fn choice_label(official: bool, name: &str, status: &str) -> String {
    let star = if official { "★ " } else { "" };
    let fits = |status: &str| star.chars().count() + name.chars().count() + 5 + status.chars().count() <= PICKER_CHARS;
    let short = status.split_once(" · ").map_or(status, |(count, _ping)| count);
    let status = if fits(status) { status } else { short };
    let room = PICKER_CHARS.saturating_sub(star.chars().count() + 5 + status.chars().count()).max(8);
    let name = if name.chars().count() <= room { name.to_owned() } else { format!("{}…", name.chars().take(room - 1).collect::<String>().trim_end()) };
    format!("{star}{name}  ·  {status}")
}

pub fn password_rules(user: &str, password: &str, again: &str) -> Vec<(Rule, &'static str)> {
    vec![
        (rule_for(password, typeable(password, 8)), "Password: 8 to 16 letters or digits"),
        (rule_for(password, !password.eq_ignore_ascii_case(user.trim())), "Not the same as the User ID"),
        (rule_for(again, again == password), "Both passwords match"),
    ]
}

/// Whether Create account asks for an invite code: only a server that said it is open needs none; one that said
/// invite (or closed) needs it; one that hasn't answered yet gets the box anyway, not required (#70: v0.1.5 hid it until
/// the server's status came back, so a player who opened the form early, or came from an invite link, had no box).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InviteAsk { Hidden, Required, Optional }

pub fn invite_ask(status: Option<&Status>) -> InviteAsk {
    match status {
        Some(Status::Online { registration, .. }) if registration == "open" => InviteAsk::Hidden,
        Some(Status::Online { .. }) => InviteAsk::Required,
        _ => InviteAsk::Optional,
    }
}

/// Making an account's rules, as lines that turn green as they are met (the server checks them again).
pub fn register_rules(user: &str, password: &str, again: &str, invite: Option<&str>) -> Vec<(Rule, &'static str)> {
    let mut rules = vec![(rule_for(user.trim(), typeable(user.trim(), 3)), "User ID: 3 to 16 letters or digits")];
    rules.extend(password_rules(user, password, again));
    if let Some(code) = invite {
        let bare: String = code.trim().chars().filter(|c| *c != '-').collect();
        rules.push((rule_for(code.trim(), bare.len() == 16 && bare.bytes().all(|b| b.is_ascii_alphanumeric())), "Invite code from the host"));
    }
    rules
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { User, Password, Again, Invite, Code, Address, Other }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick { SaveUser, Stay, Rules }

/// Settings → Accounts' forms, for one server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountForm { Everywhere, Password }

/// Settings → Accounts' fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountField { Password, New, Again }

/// A sign-in Settings → Accounts lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account { pub key: String, pub server: String, pub user: String, pub signed_in: bool, pub stays: bool }

/// A server in the picker: its key and what the row shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice { pub key: String, pub label: String }
impl std::fmt::Display for Choice { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.label) } }

#[derive(Clone, Debug)]
pub enum Message {
    /// The servers, why the credential store is unavailable (if it is), and the servers this PC stays signed in to.
    Loaded(Vec<Server>, Option<String>, Vec<String>),
    Probed(String, Status),
    Select(Choice),
    Edit(Field, String),
    Toggle(Tick, bool),
    Show(Form),
    SignIn,
    SignedIn(String, Result<SignedIn, Problem>),
    SignOut,
    SignedOut(Result<(), Problem>),
    Register,
    Registered(Result<String, Problem>),
    Recover,
    Recovered(Result<Option<String>, Problem>),
    Add,
    Trust,
    Remove,
    Saved(Result<(), String>),
    Retry,
    /// Handled by the main screen: play on the selected server with its session.
    Play,
    /// Handled by the main screen: close the game Play started.
    CloseGame,
    RestartGame,
    /// Play as…: sign in to the other account the form names (#64), and the game it opens, handled by the main screen.
    PlayAsGo,
    PlayAsSignedIn(String, Result<SignedIn, Problem>),
    PlayAsStart(Box<Server>, SignedIn),
    /// The extra game's start finished (it runs, or said why it didn't): "Starting a game as…" goes.
    StartDone,
    /// The games running now (from the main screen), and one's Close (handled by the main screen).
    Games(Vec<GameInfo>),
    CloseGameId(u64),
    Remedy(Remedy),
    /// The trust form's identity, behind Details.
    ShowIdentity,
    /// Handled by the window: Host for friends (#37).
    Host,
    /// Your hosted server, added to your own list (#37).
    Hosted(Box<Server>),
    /// Check this server's status again now (your hosted server, just stopped).
    Recheck(String),
    /// The server tools' menu beside the picker: open or closed.
    Menu(bool),
    /// The address typed under Add a server is checked once typing pauses (the check's number).
    CheckAddress(u64),
    Checked(u64, Result<Server, String>),
    /// The recovery code: to the clipboard (handled by the window), or saved as a text file.
    Copy(String),
    SaveCode,
    /// I saved it: the account just made (or recovered) signs in.
    CodeKept,
    CodeSaved(Result<Option<String>, String>),
    /// Handled by the main screen: stop a Play that hangs.
    Cancel,
    /// Settings → Accounts: open (or close) a form for a server.
    AccountForm(String, Option<AccountForm>),
    AccountEdit(AccountField, String),
    AccountSignOut(String),
    AccountSubmit,
    AccountDone(String, AccountForm, Result<(), Problem>),
}

#[derive(Default)]
pub struct State {
    servers: Vec<Server>,
    status: HashMap<String, Status>,
    selected: Option<String>,
    form: Form,
    user: String,
    /// Play as…'s User ID (#64); its password is typed in `password`.
    other_user: String,
    /// The games running now, for the list.
    games: Vec<GameInfo>,
    password: String,
    again: String,
    invite: String,
    code: String,
    address: String,
    save_user: bool,
    stay: bool,
    rules: bool,
    busy: bool,
    problem: Option<Problem>,
    note: Option<String>,
    /// The sign-in on each server (by key).
    signed_in: HashMap<String, SignedIn>,
    vault: Option<Arc<dyn Vault>>,
    /// Why Stay signed in is off, when the system's credential store isn't available.
    vault_problem: Option<String>,
    loaded: bool,
    /// The servers this PC stays signed in to (a token in the vault).
    stays: std::collections::HashSet<String>,
    show_identity: bool,
    /// The server tools' menu is open.
    menu: bool,
    /// What the address under Add a server is, as far as it has been checked, and the latest check's number.
    found: Found,
    check: u64,
    /// Where the recovery code went: copied, or saved in a file.
    code_note: Option<String>,
    account: Option<(String, AccountForm)>,
    account_password: String,
    account_new: String,
    account_again: String,
    account_busy: bool,
    /// Settings → Accounts' last result: whether it worked, and its sentence.
    account_note: Option<(bool, String)>,
}

impl State {
    /// Loads the servers (the player's and the official list) and checks the credential store.
    pub fn load(&mut self, ctx: &Ctx) -> Task<Message> {
        if self.loaded { return Task::none() }
        self.loaded = true;
        let (root, store) = (ctx.root.clone(), ctx.store.clone());
        Task::perform(async move {
            let saved = match store { Some(store) => store.run(|db| db.remote_servers()).await.unwrap_or_default(), None => vec![] };
            let official = mp::official_list(&root).await;
            let servers = mp::arrange(&official, &saved);
            let keys: Vec<Server> = servers.clone();
            let (vault, stays) = tokio::task::spawn_blocking(move || {
                let problem = mp::vault_problem(&mp::SystemVault);
                let stays = if problem.is_none() { keys.iter().filter(|s| matches!(mp::SystemVault.get(&mp::vault_account(s)), Ok(Some(_))))
                    .map(|s| s.key().to_string()).collect() } else { vec![] };
                (problem, stays)
            }).await.unwrap_or((Some("unavailable".into()), vec![]));
            (servers, vault, stays)
        }, |(servers, vault, stays)| Message::Loaded(servers, vault, stays))
    }

    fn selected(&self) -> Option<&Server> { self.selected.as_ref().and_then(|k| self.servers.iter().find(|s| s.key() == k)) }

    /// The session for Play on the selected server, if signed in.
    /// Whether the selected server's last status says it asks for a clean game (#69).
    pub fn clean_game(&self, server: &Server) -> bool {
        matches!(self.status.get(server.key()), Some(Status::Online { clean_game: true, .. }))
    }

    /// What Escape does on the Multiplayer tab: close the server tools, or leave a form for the sign-in, like its Back button. A recovery code
    /// shown once stays until the player says they saved it.
    pub fn escape(&self) -> Option<Message> {
        if self.menu { return Some(Message::Menu(false)) }
        (!matches!(self.form, Form::SignIn | Form::Code(_))).then_some(Message::Show(Form::SignIn))
    }

    pub fn session(&self) -> Option<(Server, SignedIn)> {
        let server = self.selected()?.clone();
        let signed = self.signed_in.get(server.key())?.clone();
        Some((server, signed))
    }

    fn probe(&self, server: &Server) -> Task<Message> {
        let server = server.clone();
        Task::perform(async move { let key = server.key().to_string(); (key, mp::status(&server).await) }, |(key, status)| Message::Probed(key, status))
    }

    /// The selected server's saved sign-in, resumed in the background.
    fn resume(&mut self) -> Task<Message> {
        let (Some(server), Some(vault)) = (self.selected().cloned(), self.vault.clone()) else { return Task::none() };
        if self.signed_in.contains_key(server.key()) { return Task::none() }
        let Ok(Some(token)) = vault.get(&mp::vault_account(&server)) else { return Task::none() };
        self.busy = true;
        let key = server.key().to_string();
        Task::perform(async move { mp::resume(&server, &token).await }, move |result| Message::SignedIn(key.clone(), result))
    }

    fn select(&mut self, key: String) -> Task<Message> {
        self.selected = Some(key);
        self.problem = None;
        self.form = Form::SignIn;
        self.user = self.selected().and_then(|s| s.saved_user.clone()).unwrap_or_default();
        self.save_user = !self.user.is_empty();
        let Some(server) = self.selected().cloned() else { return Task::none() };
        Task::batch([self.probe(&server), self.resume(), self.focus_sign_in()])
    }

    /// The sign-in form's first empty field takes the typing, as the other forms' first field does.
    fn focus_sign_in(&self) -> Task<Message> { iced::widget::operation::focus(field_id(sign_in_first_field(&self.user))) }

    fn save(&self, ctx: &Ctx, server: Server) -> Task<Message> {
        let Some(store) = ctx.store.clone() else { return Task::none() };
        Task::perform(async move { store.run(move |db| db.save_remote_server(&server)).await.map_err(|e| format!("{e:#}")) }, Message::Saved)
    }

    pub fn update(&mut self, message: Message, ctx: &Ctx) -> Task<Message> {
        match message {
            Message::Loaded(servers, vault_problem, stays) => {
                self.servers = servers;
                self.stays = stays.into_iter().collect();
                self.vault_problem = vault_problem;
                self.vault = self.vault_problem.is_none().then(|| Arc::new(mp::SystemVault) as Arc<dyn Vault>);
                self.stay = self.vault.is_some();
                let probes: Vec<_> = self.servers.iter().map(|s| self.probe(s)).collect();
                let first = self.servers.first().map(|s| s.key().to_string());
                let select = first.map_or(Task::none(), |key| self.select(key));
                return Task::batch(probes.into_iter().chain([select]));
            }
            Message::Probed(key, status) => { self.status.insert(key, status); }
            Message::Select(choice) => return self.select(choice.key),
            Message::Edit(field, value) => {
                let value = value.chars().filter(|c| !c.is_control()).take(200).collect();
                match field { Field::User => self.user = value, Field::Password => self.password = value, Field::Again => self.again = value,
                    Field::Invite => self.invite = value, Field::Code => self.code = value, Field::Other => self.other_user = value, Field::Address => { self.address = value; return self.look_up() } }
            }
            Message::Menu(open) => self.menu = open,
            Message::CheckAddress(check) => {
                if check != self.check || self.found != Found::Looking { return Task::none() }
                let input = self.address.trim().to_string();
                return Task::perform(async move { mp::first_contact(&input).await.map_err(|e| format!("{e:#}")) }, move |r| Message::Checked(check, r));
            }
            Message::Checked(check, result) => if check == self.check {
                self.found = match result {
                    Ok(server) => Found::Server(Box::new(server)),
                    Err(error) => {
                        if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("No server at {}: {error}", self.address.trim())) }
                        Found::No("No YO3 Revival server answers there.".into())
                    }
                };
            },
            Message::Copy(_) => self.code_note = Some("Copied. Paste it somewhere safe.".into()),
            Message::Cancel => {}
            Message::CodeKept => {
                self.form = Form::SignIn;
                self.code_note = None;
                if !self.user.trim().is_empty() && !self.password.is_empty() { return self.update(Message::SignIn, ctx) }
            }
            Message::SaveCode => {
                let Form::Code(code) = &self.form else { return Task::none() };
                let server = self.selected().map(|s| s.name.clone()).unwrap_or_default();
                let body = recovery_file(&server, self.user.trim(), code);
                let name = format!("YO3 Revival recovery code - {}.txt", sanitize(&server));
                return Task::perform(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().set_title("Save your recovery code").set_file_name(&name).add_filter("Text", &["txt"]).save_file().await
                        else { return Ok(None) };
                    tokio::fs::write(file.path(), body).await.map_err(|e| e.to_string())?;
                    Ok(Some(file.file_name()))
                }, Message::CodeSaved);
            }
            Message::CodeSaved(result) => match result {
                Ok(Some(name)) => self.code_note = Some(format!("Saved as {name}.")),
                Ok(None) => {}
                Err(error) => {
                    if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("Saving the recovery code failed: {error}")) }
                    self.code_note = Some("That file couldn't be saved. Copy the code instead.".into());
                }
            },
            Message::Toggle(tick, on) => match tick { Tick::SaveUser => self.save_user = on, Tick::Stay => self.stay = on && self.vault.is_some(), Tick::Rules => self.rules = on },
            Message::Show(form) => {
                self.menu = false;
                if form == Form::Add { self.address.clear(); self.found = Found::Nothing }
                // The form's first field takes the typing.
                if form == Form::PlayAs { self.other_user.clear() }
                let first = match form { Form::Add => Some(Field::Address), Form::Register | Form::Recover => Some(Field::User), Form::PlayAs => Some(Field::Other), _ => None };
                let form_was_sign_in = form == Form::SignIn;
                self.form = form; self.problem = None; self.note = None; self.code_note = None; self.password.clear(); self.again.clear();
                if form_was_sign_in { return self.focus_sign_in() }
                if let Some(field) = first { return iced::widget::operation::focus(field_id(field)) }
            }
            Message::SignIn => {
                let Some(server) = self.selected().cloned() else { return Task::none() };
                self.busy = true;
                self.problem = None;
                let (user, password, stay) = (self.user.trim().to_string(), std::mem::take(&mut self.password), self.stay);
                let key = server.key().to_string();
                return Task::perform(async move { mp::sign_in(&server, &user, &password, stay).await }, move |r| Message::SignedIn(key.clone(), r));
            }
            Message::SignedIn(key, result) => {
                self.busy = false;
                match result {
                    Ok(signed) => {
                        let Some(server) = self.servers.iter_mut().find(|s| s.key() == key) else { return Task::none() };
                        // The token goes to the system's vault, never to a file; Save User ID keeps only the name.
                        if let (Some(token), Some(vault)) = (&signed.device_token, &self.vault) {
                            if vault.set(&mp::vault_account(server), token).is_err() {
                                self.note = Some("You're signed in, but this PC couldn't keep the sign-in: you'll be asked again next time.".into());
                            } else { self.stays.insert(key.clone()); }
                        }
                        server.saved_user = (self.save_user || signed.device_token.is_some()).then(|| signed.user.clone());
                        let saved = server.clone();
                        self.signed_in.insert(key, signed);
                        self.form = Form::SignIn;
                        return self.save(ctx, saved);
                    }
                    Err(problem) => {
                        // A revoked stay-signed-in: forget it, back to the form with "Please sign in again".
                        if problem == Problem::SignInAgain && let (Some(server), Some(vault)) = (self.servers.iter().find(|s| s.key() == key), &self.vault) {
                            let _ = vault.delete(&mp::vault_account(server));
                            self.stays.remove(&key);
                        }
                        self.problem = Some(problem);
                    }
                }
            }
            Message::PlayAsGo => {
                let Some((server, signed)) = self.session() else { return Task::none() };
                let user = self.other_user.trim().to_string();
                if self.busy || user.is_empty() || self.password.is_empty() { return Task::none() }
                // The same account twice is refused before anything is asked of the server (#64).
                if yo3_desktop::games::same_account(&user, &signed.user) { self.problem = Some(Problem::SameAccount); return Task::none() }
                if self.games.iter().any(|g| g.server_key == server.key() && yo3_desktop::games::same_account(&g.user, &user)) { self.problem = Some(Problem::AlreadyPlaying); return Task::none() }
                self.busy = true;
                self.problem = None;
                let (password, key) = (std::mem::take(&mut self.password), server.key().to_string());
                // A plain sign-in: no device token is kept for an extra account.
                return Task::perform(async move { mp::sign_in(&server, &user, &password, false).await }, move |r| Message::PlayAsSignedIn(key.clone(), r));
            }
            Message::PlayAsSignedIn(key, result) => {
                self.busy = false;
                match result {
                    Ok(signed) => {
                        let Some(server) = self.servers.iter().find(|s| s.key() == key).cloned() else { return Task::none() };
                        self.form = Form::SignIn;
                        self.other_user.clear();
                        self.note = Some(format!("Starting a game as {}…", signed.user));
                        return Task::done(Message::PlayAsStart(Box::new(server), signed));
                    }
                    Err(problem) => self.problem = Some(problem),
                }
            }
            Message::Games(list) => self.games = list,
            Message::PlayAsStart(..) | Message::CloseGameId(_) => {}
            Message::StartDone => self.note = None,
            Message::SignOut => if let Some(key) = self.selected.clone() { return self.sign_out(&key) },
            Message::SignedOut(result) => if let Err(problem) = result && problem != Problem::Unreachable { self.problem = Some(problem) },
            Message::Register => {
                let Some(server) = self.selected().cloned() else { return Task::none() };
                self.busy = true;
                self.problem = None;
                let (user, password, again, invite, rules) = (self.user.trim().to_string(), self.password.clone(), self.again.clone(), self.invite.trim().to_string(), self.rules);
                return Task::perform(async move { mp::register(&server, &user, &password, &again, &invite, rules).await }, Message::Registered);
            }
            Message::Registered(result) => {
                self.busy = false;
                match result {
                    // The password stays until the code is saved: then the new account signs in on its own.
                    Ok(code) => { self.again.clear(); self.invite.clear(); self.form = Form::Code(code); }
                    Err(problem) => self.problem = Some(problem),
                }
            }
            Message::Recover => {
                let Some(server) = self.selected().cloned() else { return Task::none() };
                self.busy = true;
                self.problem = None;
                let (user, code, password, again) = (self.user.trim().to_string(), self.code.trim().to_string(), self.password.clone(), self.again.clone());
                return Task::perform(async move { mp::recover(&server, &user, &code, &password, &again).await }, Message::Recovered);
            }
            Message::Recovered(result) => {
                self.busy = false;
                match result {
                    Ok(next) => {
                        self.again.clear(); self.code.clear();
                        match next {
                            Some(code) => self.form = Form::Code(code),
                            None => { self.form = Form::SignIn; return self.update(Message::SignIn, ctx) }
                        }
                    }
                    Err(problem) => self.problem = Some(problem),
                }
            }
            Message::Add => {
                self.problem = None;
                match std::mem::take(&mut self.found) {
                    // An invite link carries the identity: no first-use question. One made for a friend carries their
                    // invite code too: straight to making the account, the code filled in.
                    Found::Link(server, code) => {
                        let task = self.add(ctx, *server);
                        if let Some(code) = code { self.invite = code; self.form = Form::Register }
                        return task;
                    }
                    Found::Server(server) => { self.show_identity = false; self.form = Form::Trust(server) }
                    other => self.found = other,
                }
            }
            Message::Trust => if let Form::Trust(server) = std::mem::take(&mut self.form) { return self.add(ctx, *server) },
            Message::Remove => {
                self.menu = false;
                let Some(server) = self.selected().cloned() else { return Task::none() };
                if server.official { return Task::none() }
                if let Some(vault) = &self.vault { let _ = vault.delete(&mp::vault_account(&server)); }
                self.servers.retain(|s| s.key() != server.key());
                self.signed_in.remove(server.key());
                self.selected = None;
                let first = self.servers.first().map(|s| s.key().to_string());
                let next = first.map_or(Task::none(), |key| self.select(key));
                let Some(store) = ctx.store.clone() else { return next };
                let key = server.key().to_string();
                return Task::batch([next, Task::perform(async move { store.run(move |db| db.delete_remote_server(&key)).await.map_err(|e| format!("{e:#}")) }, Message::Saved)]);
            }
            Message::Saved(result) => if let Err(error) = result {
                if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("Saving a server failed: {error}")) }
                self.note = Some("YO3 Revival couldn't save this server's details on this PC.".into());
            },
            Message::Retry => {
                self.problem = None;
                if let Some(server) = self.selected().cloned() { return self.probe(&server) }
            }
            Message::Remedy(Remedy::Retry) => return self.update(Message::Retry, ctx),
            Message::Remedy(Remedy::SignInAgain) => { self.problem = None; self.form = Form::SignIn }
            Message::Remedy(Remedy::RemoveServer) => return self.update(Message::Remove, ctx),
            Message::Remedy(Remedy::Update) | Message::Play | Message::CloseGame | Message::RestartGame => {}
            Message::ShowIdentity => self.show_identity = !self.show_identity,
            Message::Host => self.menu = false,
            Message::Recheck(key) => {
                self.status.remove(&key);
                if let Some(server) = self.servers.iter().find(|s| s.key() == key).cloned() { return self.probe(&server) }
            }
            Message::Hosted(server) => {
                // Checked again a few seconds on: a first check that met the server still starting would keep Windows'
                // retry after a refused connection (about 2 s) as its ping.
                let key = server.key().to_string();
                let again = Task::perform(tokio::time::sleep(std::time::Duration::from_secs(4)), move |_| Message::Recheck(key.clone()));
                if self.servers.iter().any(|s| s.key() == server.key()) {
                    // Known already (an earlier hosting, or its link): its address follows this hosting's port.
                    if let Some(known) = self.servers.iter_mut().find(|s| s.key() == server.key()) { known.address = server.address.clone(); known.server_name = server.server_name.clone() }
                    let saved = self.servers.iter().find(|s| s.key() == server.key()).cloned().unwrap_or(*server);
                    return Task::batch([self.probe(&saved), self.save(ctx, saved), again]);
                }
                return Task::batch([self.add(ctx, *server), again]);
            }
            Message::AccountForm(key, form) => {
                self.account = form.map(|f| (key, f));
                self.account_password.clear(); self.account_new.clear(); self.account_again.clear();
                self.account_note = None;
            }
            Message::AccountEdit(field, value) => {
                let value: String = value.chars().filter(|c| !c.is_control()).take(200).collect();
                match field { AccountField::Password => self.account_password = value, AccountField::New => self.account_new = value, AccountField::Again => self.account_again = value }
            }
            Message::AccountSignOut(key) => {
                self.account = None;
                self.account_note = Some((true, "Signed out on this PC.".into()));
                return self.sign_out(&key);
            }
            Message::AccountSubmit => {
                let Some((key, form)) = self.account.clone() else { return Task::none() };
                let Some(server) = self.servers.iter().find(|s| s.key() == key).cloned() else { return Task::none() };
                let user = self.signed_in.get(&key).map(|s| s.user.clone()).or_else(|| server.saved_user.clone()).unwrap_or_default();
                let token = self.signed_in.get(&key).and_then(|s| s.device_token.clone())
                    .or_else(|| self.vault.as_ref().and_then(|v| v.get(&mp::vault_account(&server)).ok().flatten()));
                let (password, new, again) = (std::mem::take(&mut self.account_password), std::mem::take(&mut self.account_new), std::mem::take(&mut self.account_again));
                self.account_busy = true;
                self.account_note = None;
                return Task::perform(async move {
                    match form {
                        AccountForm::Everywhere => mp::sign_out_everywhere(&server, token.as_deref(), &user, &password).await,
                        AccountForm::Password => mp::change_password(&server, &user, &password, &new, &again).await,
                    }
                }, move |result| Message::AccountDone(key.clone(), form, result));
            }
            Message::AccountDone(key, form, result) => {
                self.account_busy = false;
                match result {
                    Ok(()) => {
                        // Either way every device is signed out on the server: this PC forgets its sign-in too.
                        self.forget(&key);
                        self.account = None;
                        self.account_note = Some((true, match form {
                            AccountForm::Everywhere => "Signed out everywhere: every PC must sign in again.".into(),
                            AccountForm::Password => "Your password is changed, and every PC is signed out. Sign in again with the new one.".into(),
                        }));
                    }
                    Err(problem) => self.account_note = Some((false, problem.sentence())),
                }
            }
        }
        Task::none()
    }

    /// Forgets this PC's sign-in on server `key` (the session and a kept token).
    fn forget(&mut self, key: &str) {
        self.signed_in.remove(key);
        self.stays.remove(key);
        if let (Some(server), Some(vault)) = (self.servers.iter().find(|s| s.key() == key), &self.vault) { let _ = vault.delete(&mp::vault_account(server)); }
        if self.selected.as_deref() == Some(key) { self.form = Form::SignIn }
    }

    /// Signs this PC out of server `key`: forgets it here, and revokes a kept token on the server.
    fn sign_out(&mut self, key: &str) -> Task<Message> {
        let Some(server) = self.servers.iter().find(|s| s.key() == key).cloned() else { return Task::none() };
        let signed = self.signed_in.get(key).cloned();
        let token = self.vault.as_ref().and_then(|v| v.get(&mp::vault_account(&server)).ok().flatten()).or_else(|| signed.and_then(|s| s.device_token));
        self.forget(key);
        // Signed out, the sign-in form is back: its first empty field takes the keyboard (live on Windows 11, typing went
        // nowhere).
        let focus = if self.selected.as_deref() == Some(key) { self.focus_sign_in() } else { Task::none() };
        let Some(token) = token else { return focus };
        Task::batch([focus, Task::perform(async move { mp::sign_out(&server, &token, false).await }, Message::SignedOut)])
    }

    /// The sign-ins on this PC, for Settings → Accounts.
    pub fn accounts(&self) -> Vec<Account> {
        self.servers.iter().filter(|s| self.signed_in.contains_key(s.key()) || self.stays.contains(s.key())).map(|s| Account {
            key: s.key().to_string(), server: s.name.clone(),
            user: self.signed_in.get(s.key()).map(|x| x.user.clone()).or_else(|| s.saved_user.clone()).unwrap_or_default(),
            signed_in: self.signed_in.contains_key(s.key()), stays: self.stays.contains(s.key()),
        }).collect()
    }

    /// Settings → Accounts: each sign-in on this PC with Sign out, Sign out everywhere and Change password.
    pub fn accounts_view(&self) -> Element<'_, Message> {
        let accounts = self.accounts();
        let mut col = column![].spacing(12);
        if !self.loaded {
            col = col.push(text("Open the Multiplayer tab once to see your sign-ins here.").size(15).color(color::INK));
        } else if accounts.is_empty() {
            col = col.push(text("You aren't signed in to any server on this PC. Sign in on the Multiplayer tab.").size(15).color(color::INK));
        }
        let field = |placeholder: &'static str, value: &str, which: AccountField| text_input(placeholder, value)
            .on_input(move |v| Message::AccountEdit(which, v)).on_submit(Message::AccountSubmit).secure(true).style(yo3_skin::field).padding(7).size(15);
        let mut rows = Vec::new();
        for account in accounts {
            let state = match (account.signed_in, account.stays) {
                (_, true) => format!("Signed in as {}, and stays signed in on this PC.", account.user),
                _ => format!("Signed in as {}.", account.user),
            };
            // One row per server, as Settings' other rows: Sign out on the right, the account's other actions under it.
            let mut row_col = column![
                row![yo3_skin::link("Sign out everywhere", Message::AccountForm(account.key.clone(), Some(AccountForm::Everywhere)), false),
                    yo3_skin::link("Change password", Message::AccountForm(account.key.clone(), Some(AccountForm::Password)), false)].spacing(16).align_y(Center),
            ].spacing(8);
            if let Some((key, form)) = &self.account && *key == account.key {
                let has_token = account.stays || self.signed_in.get(&account.key).is_some_and(|s| s.device_token.is_some());
                let cancel = yo3_skin::link("Cancel", Message::AccountForm(account.key.clone(), None), false);
                let busy = self.account_busy;
                row_col = row_col.push(match form {
                    AccountForm::Everywhere => {
                        let mut f = column![text("Every PC signed in to this account is signed out, this one too.").size(14).color(color::INK)].spacing(8);
                        if !has_token { f = f.push(row![text("Password").size(14).font(BODY_BOLD).color(color::INK).width(130), field("", &self.account_password, AccountField::Password)].align_y(Center)) }
                        f.push(row![pill("Sign out everywhere", (!busy && (has_token || !self.account_password.is_empty())).then_some(Message::AccountSubmit), Kind::Primary, 200, 34),
                            Space::new().width(Fill), cancel].align_y(Center))
                    }
                    AccountForm::Password => column![
                        text("A new password signs out every PC, this one too.").size(14).color(color::INK),
                        row![text("Current password").size(14).font(BODY_BOLD).color(color::INK).width(130), field("", &self.account_password, AccountField::Password)].align_y(Center),
                        row![text("New password").size(14).font(BODY_BOLD).color(color::INK).width(130), field("8 to 16 letters or digits", &self.account_new, AccountField::New)].align_y(Center),
                        row![text("Again").size(14).font(BODY_BOLD).color(color::INK).width(130), field("", &self.account_again, AccountField::Again)].align_y(Center),
                        row![pill("Change password", (!busy && !self.account_password.is_empty() && !self.account_new.is_empty()).then_some(Message::AccountSubmit), Kind::Primary, 180, 34),
                            Space::new().width(Fill), cancel].align_y(Center),
                    ].spacing(8),
                });
            }
            rows.push(yo3_skin::setting(account.server.clone(), state, Some(pill("Sign out", Some(Message::AccountSignOut(account.key.clone())), Kind::Secondary, 120, 32)),
                Some(row_col.into())));
        }
        if !rows.is_empty() { col = col.push(yo3_skin::setting_group("Signed in on this PC", rows)) }
        if let Some((good, note)) = &self.account_note {
            let shade = if *good { color::GOOD } else { color::BAD };
            col = col.push(row![container(Space::new().width(9).height(9)).style(move |_| dot(shade)), text(note).size(14).font(BODY_BOLD).color(color::INK).width(Fill)]
                .spacing(8).align_y(Center));
        }
        col.into()
    }

    /// What the typed address is: an invite link is read at once; an address is looked up once typing pauses.
    fn look_up(&mut self) -> Task<Message> {
        self.check += 1;
        let input = self.address.trim();
        if input.is_empty() { self.found = Found::Nothing; return Task::none() }
        if input.starts_with("yo3revival://") {
            self.found = match mp::invite_link(input) {
                Ok(server) => Found::Link(Box::new(server), mp::invite_code(input)),
                Err(_) => Found::No("That invite link isn't complete. Ask the host to send it again.".into()),
            };
            return Task::none();
        }
        if yo3_tunnel::invite::address(input).is_err() { self.found = Found::No("That isn't an address: a name or an IP, and a port if it isn't 7443.".into()); return Task::none() }
        self.found = Found::Looking;
        let check = self.check;
        Task::perform(async { tokio::time::sleep(LOOK_AFTER).await }, move |()| Message::CheckAddress(check))
    }

    fn add(&mut self, ctx: &Ctx, server: Server) -> Task<Message> {
        if !self.servers.iter().any(|s| s.key() == server.key()) { self.servers.insert(0, server.clone()) }
        self.address.clear();
        self.found = Found::Nothing;
        self.form = Form::SignIn;
        let key = server.key().to_string();
        Task::batch([self.save(ctx, server), self.select(key)])
    }

    /// Play's session check said no. An ended session, or a revoked saved sign-in, is forgotten (sign in again); a
    /// ban says so; anything else (no answer, maintenance, a full server) keeps the sign-in for Retry.
    pub fn play_refused(&mut self, problem: Problem) {
        let ended = matches!(problem, Problem::SignInAgain | Problem::Banned { .. } | Problem::Restricted { .. });
        if ended && let Some(server) = self.selected().cloned() {
            self.signed_in.remove(server.key());
            if let Some(vault) = &self.vault { let _ = vault.delete(&mp::vault_account(&server)); }
            self.form = Form::SignIn;
        }
        self.problem = Some(if problem == Problem::SignInAgain { Problem::SessionEnded } else { problem });
    }

    /// A Play: the server it is on moves to the top of the list next time.
    pub fn played(&mut self, ctx: &Ctx) -> Task<Message> {
        let Some(server) = self.servers.iter_mut().find(|s| Some(s.key()) == self.selected.as_deref()) else { return Task::none() };
        server.last_used = chrono::Utc::now().timestamp();
        let saved = server.clone();
        self.save(ctx, saved)
    }

    /// The selected server's news, for the Information panel.
    pub fn news(&self) -> Vec<(String, String)> {
        match self.selected.as_ref().and_then(|k| self.status.get(k)) {
            Some(Status::Online { news, .. }) if !news.is_empty() => news.iter().map(|n| (n.title.clone(), n.body.clone())).collect(),
            Some(Status::Online { .. }) => vec![("News".into(), "This server has no news.".into())],
            _ => vec![("News".into(), "Choose a server to see its news.".into())],
        }
    }

    fn row_status(status: Option<&Status>) -> (iced::Color, String) {
        match status {
            None => (color::INK_SOFT, "Checking…".into()),
            Some(Status::Online { online, ping, .. }) => (color::GOOD, format!("{} online · {} ms", online.map_or("?".into(), |n| n.to_string()), ping.as_millis())),
            Some(Status::Maintenance) => (color::WARN, "Maintenance".into()),
            Some(Status::NeedsUpdate { min }) => (color::WARN, format!("Needs YO3 Revival {min}")),
            Some(Status::IdentityChanged) => (color::BAD, "Identity changed".into()),
            Some(Status::Offline) => (color::BAD, "Offline".into()),
        }
    }

    /// The Play button's label and action for the selected server, and the line above it (empty when the button and
    /// the server's row already say it).
    fn play(&self) -> (&'static str, Option<Message>, String) {
        let Some(server) = self.selected() else { return ("LOG IN", None, "Add a server first.".into()) };
        match self.status.get(server.key()) {
            // Your own hosted server (at this PC's address) that isn't running: Retry can't start it, Host for friends can.
            Some(Status::Offline) if is_own_hosted(server) => return ("START HOSTING", Some(Message::Host), "Your hosted server isn't running.".into()),
            Some(Status::Offline) => return ("RETRY", Some(Message::Retry), String::new()),
            Some(Status::Maintenance) => return ("RETRY", Some(Message::Retry), "The server is in maintenance.".into()),
            Some(Status::NeedsUpdate { min }) => return ("UPDATE", Some(Message::Remedy(Remedy::Update)), format!("This server needs YO3 Revival {min} or newer.")),
            Some(Status::IdentityChanged) => return ("LOG IN", None, Problem::IdentityChanged.sentence()),
            _ => {}
        }
        if self.signed_in.contains_key(server.key()) { return ("PLAY", Some(Message::Play), String::new()) }
        let ready = !self.busy && !self.user.trim().is_empty() && !self.password.is_empty() && self.form == Form::SignIn;
        ("LOG IN", ready.then_some(Message::SignIn), if self.busy { "Signing in…".into() } else { String::new() })
    }

    /// The server panel. `busy` is the game's state while it is starting, open or closing: the big button follows it.
    pub fn view<'a>(&'a self, busy: Option<crate::home::Busy>) -> Element<'a, Message> {
        let choices: Vec<Choice> = self.servers.iter().map(|s| Choice { key: s.key().to_string(),
            label: choice_label(s.official, &s.name, &Self::row_status(self.status.get(s.key())).1) }).collect();
        let selected = choices.iter().find(|c| Some(&c.key) == self.selected.as_ref()).cloned();
        let picker: Element<'_, Message> = if choices.is_empty() {
            container(text("No servers yet.").size(15).color(color::INK_SOFT)).padding([7, 0]).width(Fill).into()
        } else {
            // The selected server's state as a coloured dot inside the picker, so it isn't said twice.
            let shade = Self::row_status(self.selected.as_ref().and_then(|k| self.status.get(k))).0;
            iced::widget::stack![
                pick_list(choices, selected, Message::Select).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(Fill)
                    .padding(iced::Padding { top: 7.0, right: 10.0, bottom: 7.0, left: 26.0 }),
                container(container(Space::new().width(9).height(9)).style(move |_| dot(shade))).padding(iced::Padding { left: 10.0, ..iced::Padding::ZERO }).height(34).center_y(34),
            ].into()
        };
        let tools = yo3_skin::icon_button(Icon::More, Some(Message::Menu(!self.menu)), "Server tools", false);
        let top = row![picker, tools].spacing(6).align_y(Center);
        let body = match &self.selected {
            Some(_) => column![top, self.form()].spacing(12),
            None if matches!(self.form, Form::Add | Form::Trust(_)) => column![top, self.form()].spacing(12),
            // Nothing to sign in to yet: the way to add one.
            None => column![top, text("Add a server by its address, or with an invite link a host sent you.").size(14).color(color::INK),
                pill("Add a server", Some(Message::Show(Form::Add)), Kind::Primary, 160, 34)].spacing(12),
        };
        let mut panel = column![body, self.messages(), self.games_list(), Space::new().height(Fill)].spacing(10);
        // The forms with their own button (making an account, recovering, adding a server) hide the big one, and so
        // does an empty list: its way forward is Add a server.
        if matches!(self.form, Form::SignIn) && self.selected.is_some() {
            let (label, action, line, kind) = match busy {
                Some(busy) => (busy.label, busy.action.map(|a| match a { crate::home::BusyAction::Close => Message::CloseGame, crate::home::BusyAction::Cancel => Message::Cancel, crate::home::BusyAction::Restart => Message::RestartGame }),
                    busy.line.to_owned(), if busy.action.is_some() { Kind::Primary } else { Kind::Play }),
                None => { let (label, action, line) = self.play(); (label, action, line, Kind::Play) }
            };
            if !line.is_empty() { panel = panel.push(text(line).size(14).color(color::INK_SOFT)) }
            panel = panel.push(pill(label, action, kind, Fill, 70));
        }
        let mut layers = iced::widget::stack![panel];
        if self.menu {
            // The tools drop down from their button; a click anywhere else closes them.
            let mut items = vec![
                Item { icon: Icon::Plus, label: "Add a server".into(), on_press: Message::Show(Form::Add), danger: false },
                Item { icon: Icon::Server, label: "Host for friends".into(), on_press: Message::Host, danger: false },
            ];
            if self.selected().is_some_and(|s| !s.official) { items.push(Item { icon: Icon::Trash, label: "Remove this server".into(), on_press: Message::Remove, danger: true }) }
            layers = layers.push(iced::widget::mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::Menu(false)))
                .push(column![Space::new().height(36), row![Space::new().width(Fill), yo3_skin::menu(items, 210.0)]]);
        }
        yo3_skin::panel(Some("Server"), layers)
    }

    /// The games running now, each with its account and server and a way to close it (#64). Nothing while none runs.
    fn games_list(&self) -> Element<'_, Message> {
        if self.games.is_empty() { return Space::new().height(0).into() }
        let mut col = column![text("Games running").size(13).font(BODY_BOLD).color(color::INK_SOFT)].spacing(4);
        for game in &self.games {
            col = col.push(row![text(format!("{} on {}", game.user, game.server)).size(14).color(color::INK).width(Fill),
                yo3_skin::link("Close", if game.is_main() { Message::CloseGame } else { Message::CloseGameId(game.id) }, false)].spacing(8).align_y(Center));
        }
        col.into()
    }

    fn messages(&self) -> Element<'_, Message> {
        let mut out = column![].spacing(6);
        // A server that doesn't answer is already said by its row and the RETRY button.
        let offline = self.selected.as_ref().is_some_and(|k| self.status.get(k) == Some(&Status::Offline));
        if let Some(problem) = &self.problem && !(offline && *problem == Problem::Unreachable) {
            let mut line = row![container(Space::new().width(9).height(9)).style(|_| dot(color::BAD)),
                text(problem.sentence()).size(14).font(BODY_BOLD).color(color::INK).width(Fill)].spacing(8).align_y(Center);
            if let Some(remedy) = problem.remedy() && !(remedy == Remedy::Retry && matches!(self.form, Form::SignIn)) {
                let label = match remedy { Remedy::Retry => "Retry", Remedy::Update => "Update", Remedy::SignInAgain => "Sign in", Remedy::RemoveServer => "Remove server" };
                line = line.push(yo3_skin::link(label, Message::Remedy(remedy), false));
            }
            out = out.push(line);
        }
        if let Some(note) = &self.note { out = out.push(text(note).size(14).color(color::INK)) }
        out.into()
    }

    fn form(&self) -> Element<'_, Message> {
        let input = |placeholder: &'static str, value: &str, field: Field, secret: bool| {
            let mut field_input = text_input(placeholder, value).id(field_id(field)).on_input(move |v| Message::Edit(field, v)).style(yo3_skin::field).padding(8).size(15);
            if secret { field_input = field_input.secure(true) }
            if !self.busy { field_input = field_input.on_submit(match field { Field::Address => Message::Add, _ => match self.form {
                Form::Register => Message::Register, Form::Recover => Message::Recover, Form::PlayAs => Message::PlayAsGo, _ => Message::SignIn } }) }
            field_input
        };
        let labelled = |label: &'static str, input: iced::widget::TextInput<'static, Message>| row![text(label).font(BODY_BOLD).size(14).color(color::INK).width(92), input].align_y(Center);
        let heading = |words: &'static str| text(words).size(16).font(yo3_skin::DISPLAY).color(color::NAVY);
        // A form's actions: the way back quiet on the left, the main action filling the rest.
        let actions = |main: &'static str, go: Option<Message>| row![pill("Back", Some(Message::Show(Form::SignIn)), Kind::Secondary, 96, 36),
            pill(main, go, Kind::Primary, Fill, 36)].spacing(8);
        let checklist = |rules: Vec<(Rule, &'static str)>| rules.into_iter().fold(column![].spacing(3), |c, (state, words)| c.push(yo3_skin::rule(state, words)));
        let tick = |label: &'static str, on: bool, which: Tick| checkbox(on).label(label).on_toggle(move |v| Message::Toggle(which, v)).style(yo3_skin::tick).size(16).text_size(14);
        let selected = self.selected();
        let signed = selected.and_then(|s| self.signed_in.get(s.key()));
        match &self.form {
            Form::SignIn if signed.is_some() => column![container(row![
                yo3_skin::icon(Icon::User, 18.0, color::NAVY),
                column![text("Signed in as").size(12).color(color::INK_SOFT), text(signed.map_or("", |s| s.user.as_str())).size(16).font(BODY_BOLD).color(color::INK)],
                Space::new().width(Fill),
                pill("Sign out", Some(Message::SignOut), Kind::Danger, 96, 30),
            ].spacing(10).align_y(Center)).padding([8, 10]).width(Fill).style(|_| container::Style { background: Some(color::ROW.into()),
                border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 4.0.into() }, ..container::Style::default() }),
                row![Space::new().width(Fill), yo3_skin::link("Play as another account…", Message::Show(Form::PlayAs), false)]].spacing(6).into(),
            Form::SignIn => {
                let mut stay = tick("Stay signed in", self.stay, Tick::Stay);
                if self.vault.is_none() { stay = checkbox(false).label("Stay signed in").style(yo3_skin::tick).size(16).text_size(14) }
                let mut col = column![
                    labelled("User ID", input("", &self.user, Field::User, false)),
                    labelled("Password", input("", &self.password, Field::Password, true)),
                    row![tick("Save User ID", self.save_user, Tick::SaveUser), Space::new().width(16), stay].align_y(Center),
                ].spacing(8);
                if let Some(why) = &self.vault_problem { col = col.push(text(why).size(13).color(color::INK_SOFT)) }
                col.push(row![yo3_skin::link("Forgot password?", Message::Show(Form::Recover), false), Space::new().width(Fill),
                    text("New here?").size(14).color(color::INK_SOFT), yo3_skin::link("Create account", Message::Show(Form::Register), false)].spacing(6).align_y(Center))
                    .into()
            }
            Form::Register => {
                let ask = invite_ask(selected.and_then(|s| self.status.get(s.key())));
                let rules = register_rules(&self.user, &self.password, &self.again, (ask == InviteAsk::Required).then_some(self.invite.as_str()));
                let ready = !self.busy && self.rules && rules.iter().all(|(r, _)| *r == Rule::Met);
                let mut col = column![
                    heading("Create an account"),
                    labelled("User ID", input("", &self.user, Field::User, false)),
                    labelled("Password", input("", &self.password, Field::Password, true)),
                    labelled("Again", input("", &self.again, Field::Again, true)),
                ].spacing(8);
                match ask {
                    InviteAsk::Required => col = col.push(labelled("Invite code", input("ABCD-EFGH-JKLM-NPQR", &self.invite, Field::Invite, false))),
                    // The server hasn't said yet (or couldn't be asked): the box is there, filled if the host gave a code.
                    InviteAsk::Optional => col = col.push(column![labelled("Invite code", input("ABCD-EFGH-JKLM-NPQR", &self.invite, Field::Invite, false)),
                        text("If the host gave you one.").size(13).color(color::INK_SOFT)].spacing(2)),
                    InviteAsk::Hidden => {}
                }
                col.push(checklist(rules)).push(tick("I accept this server's rules", self.rules, Tick::Rules))
                    .push(actions(if self.busy { "Creating…" } else { "Create account" }, ready.then_some(Message::Register))).into()
            }
            Form::Recover => {
                let rules = password_rules(&self.user, &self.password, &self.again);
                let ready = !self.busy && !self.user.trim().is_empty() && !self.code.trim().is_empty() && rules.iter().all(|(r, _)| *r == Rule::Met);
                column![
                    heading("Set a new password"),
                    text("With the recovery code you saved at sign-up, or a reset code from the server's moderators.").size(13).color(color::INK_SOFT),
                    labelled("User ID", input("", &self.user, Field::User, false)),
                    labelled("Code", input("ABCD-EFGH-JKLM-NPQR", &self.code, Field::Code, false)),
                    labelled("Password", input("", &self.password, Field::Password, true)),
                    labelled("Again", input("", &self.again, Field::Again, true)),
                    checklist(rules),
                    actions(if self.busy { "Setting…" } else { "Set password" }, ready.then_some(Message::Recover)),
                ].spacing(8).into()
            }
            Form::Add => {
                let (state, words) = match &self.found {
                    Found::Nothing => (Rule::Pending, "A name or an IP (and :port), or an invite link".to_owned()),
                    Found::Looking => (Rule::Pending, "Looking for a server there…".to_owned()),
                    Found::Server(server) if self.servers.iter().any(|s| s.key() == server.key()) => (Rule::Met, format!("“{}” answers: it's already in your list", server.name)),
                    Found::Server(server) => (Rule::Met, format!("“{}” answers there", server.name)),
                    Found::Link(server, Some(_)) => (Rule::Met, format!("Invite to “{}”, with your invite code", server.name)),
                    Found::Link(server, None) => (Rule::Met, format!("Invite to “{}”", server.name)),
                    Found::No(why) => (Rule::Broken, why.clone()),
                };
                let ready = matches!(self.found, Found::Server(_) | Found::Link(..));
                column![
                    heading("Add a server"),
                    input("play.example.com or yo3revival://…", &self.address, Field::Address, false),
                    yo3_skin::rule(state, words),
                    actions("Add", ready.then_some(Message::Add)),
                ].spacing(10).into()
            }
            Form::Trust(server) => {
                let mut col = column![
                    heading("First connection to this server"),
                    text(format!("{} at {}", server.name, server.address)).size(14).font(BODY_BOLD).color(color::INK),
                    text("YO3 Revival remembers this server and checks it is the same one each time. An invite link from the host does this for you.")
                        .size(13).color(color::INK_SOFT),
                    yo3_skin::link(if self.show_identity { "Hide details" } else { "Details" }, Message::ShowIdentity, false),
                ].spacing(8);
                if self.show_identity {
                    col = col.push(text(format!("Identity: {}", fingerprint(server.key()))).size(13).color(color::INK_SOFT));
                }
                col.push(actions("Add server", Some(Message::Trust))).into()
            }
            Form::PlayAs => {
                let ready = !self.busy && !self.other_user.trim().is_empty() && !self.password.is_empty();
                column![
                    heading("Play as another account"),
                    text("Starts another game on this server, signed in as a different account. Each game keeps its own settings and decks.").size(13).color(color::INK_SOFT),
                    labelled("User ID", input("", &self.other_user, Field::Other, false)),
                    labelled("Password", input("", &self.password, Field::Password, true)),
                    actions(if self.busy { "Signing in…" } else { "Play" }, ready.then_some(Message::PlayAsGo)),
                ].spacing(8).into()
            }
            Form::Code(code) => {
                let mut col = column![
                    heading("Save your recovery code"),
                    text("It sets a new password if you forget yours. It's shown only once.").size(13).color(color::INK_SOFT),
                    container(row![text(code).size(18).font(BODY_BOLD).color(color::INK).width(Fill),
                        yo3_skin::icon_button(Icon::Copy, Some(Message::Copy(code.clone())), "Copy", false),
                        yo3_skin::icon_button(Icon::Save, Some(Message::SaveCode), "Save as a text file", false)].spacing(2).align_y(Center))
                        .padding(iced::Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 12.0 }).style(|_| container::Style {
                        background: Some(color::PANEL_TOP.into()), border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }),
                ].spacing(8);
                if let Some(note) = &self.code_note { col = col.push(yo3_skin::rule(Rule::Met, note.clone())) }
                col.push(pill("I saved it", Some(Message::CodeKept), Kind::Primary, Fill, 36)).into()
            }
        }
    }
}

/// A form field's id, to give it the typing.
fn field_id(field: Field) -> iced::widget::Id { iced::widget::Id::new(match field { Field::User => "user", Field::Password => "password", Field::Again => "again",
    Field::Invite => "invite", Field::Code => "code", Field::Address => "address", Field::Other => "other-user" }) }

/// The text file a recovery code is saved in: the server, the User ID and the code, a line each (Windows line ends).
fn recovery_file(server: &str, user: &str, code: &str) -> String {
    let lines = ["YO3 Revival recovery code".to_owned(), String::new(), format!("Server: {server}"), format!("User ID: {user}"),
        format!("Recovery code: {code}"), String::new(), "It sets a new password if you forget yours. Keep it private.".to_owned()];
    lines.join("\r\n") + "\r\n"
}

/// `name` fit for a file name: letters, digits, spaces and dashes.
fn sanitize(name: &str) -> String {
    let kept: String = name.chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')).take(40).collect();
    if kept.trim().is_empty() { "server".into() } else { kept.trim().to_owned() }
}

/// An identity as people compare it: groups of four.
fn fingerprint(identity: &str) -> String {
    identity.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or("")).collect::<Vec<_>>().join(" ")
}

/// The field the sign-in form focuses: User ID, or the password when a saved User ID is already filled in.
fn sign_in_first_field(user: &str) -> Field { if user.trim().is_empty() { Field::User } else { Field::Password } }

/// The server Host for friends made on this PC: in the list at this PC's own address (app.rs adds it as 127.0.0.1).
fn is_own_hosted(server: &Server) -> bool { !server.official && server.address.starts_with("127.0.0.1:") }

#[cfg(test)]
mod tests {
    use super::*;
    use yo3_desktop::multiplayer::MemoryVault;

    fn server(name: &str) -> Server {
        mp::invite_link(&format!("yo3revival://server?address={}.example:7443&identity={}&name={name}", name.to_lowercase(), name.len().to_string().repeat(64))).unwrap()
    }
    fn ctx() -> Ctx { Ctx { root: std::env::temp_dir(), backend: None, store: None, settings: Default::default() } }
    /// A tab with one server, `status`, and a vault (None: the system's isn't available).
    fn tab(status: Status, vault: Option<Arc<MemoryVault>>) -> State {
        let mut state = State::default();
        let ctx = ctx();
        // Loaded as if the system's store were unavailable (tests never touch it), then given the test's vault.
        let _ = state.update(Message::Loaded(vec![server("Beta")], Some("no store".to_string()), vec![]), &ctx);
        if vault.is_some() { state.vault_problem = None }
        state.vault = vault.map(|v| v as Arc<dyn Vault>);
        state.stay = state.vault.is_some();
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::Probed(key, status), &ctx);
        state
    }
    fn online() -> Status { Status::Online { online: Some(3), max: None, registration: "invite".into(), news: vec![], ping: std::time::Duration::from_millis(40), clean_game: false } }
    fn signed(user: &str, token: Option<&str>) -> SignedIn { SignedIn { user: user.into(), session: [7; 32], device_token: token.map(str::to_owned) } }

    #[test]
    fn a_long_server_name_never_hides_its_state_in_the_picker() {
        assert_eq!(choice_label(true, "YO3 Revival", "0 online · 225 ms"), "★ YO3 Revival  ·  0 online · 225 ms");
        // The ping goes first: it is the least useful part, and the name stays whole.
        assert_eq!(choice_label(false, "Friends' server (yours)", "0 online · 0 ms"), "Friends' server (yours)  ·  0 online");
        // A name too long even for that is cut, and the state stays whole.
        let label = choice_label(false, "The Very Long Name Of A Friends' Duel Server", "12 online · 40 ms");
        assert!(label.ends_with("  ·  12 online") && label.contains("…") && label.chars().count() <= PICKER_CHARS + 2, "{label}");
        assert_eq!(choice_label(false, "A server with a pretty long name here", "Offline"), "A server with a pretty lo…  ·  Offline");
    }

    #[test]
    fn escape_leaves_a_form_for_the_sign_in_but_not_a_recovery_code() {
        let mut state = tab(online(), None);
        assert!(state.escape().is_none(), "nothing to leave on the sign-in");
        let _ = state.update(Message::Menu(true), &ctx());
        assert!(matches!(state.escape(), Some(Message::Menu(false))), "Escape closes the server tools first");
        let _ = state.update(Message::Menu(false), &ctx());
        for form in [Form::Add, Form::Register, Form::Recover, Form::PlayAs] {
            let _ = state.update(Message::Show(form), &ctx());
            assert!(matches!(state.escape(), Some(Message::Show(Form::SignIn))));
        }
        let _ = state.update(Message::Show(Form::Code("ABCD".into())), &ctx());
        assert!(state.escape().is_none(), "the recovery code is shown once: the player confirms it");
    }

    #[test]
    fn a_stopped_hosted_server_is_checked_again_at_once() {
        let mut state = tab(online(), None);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::Recheck(key.clone()), &ctx());
        assert!(!state.status.contains_key(&key), "no last check left showing while the new one runs");
        assert_eq!(State::row_status(state.status.get(&key)).1, State::row_status(None).1);
    }

    #[test]
    fn the_sign_in_form_focuses_user_id_first() {
        assert_eq!(sign_in_first_field(""), Field::User);
        assert_eq!(sign_in_first_field("yugi"), Field::Password, "a saved User ID is already there");
    }

    #[test]
    fn your_own_stopped_hosted_server_offers_to_start_it() {
        let mut state = tab(Status::Offline, None);
        state.servers[0] = yo3_desktop::hosting::hosted_server("Friday (yours)", "127.0.0.1", 7443, &"ab".repeat(32)).unwrap();
        let key = state.servers[0].key().to_string();
        state.selected = Some(key.clone());
        state.status.insert(key, Status::Offline);
        let (label, action, _) = state.play();
        assert_eq!(label, "START HOSTING");
        assert!(matches!(action, Some(Message::Host)), "the same action as Host for friends");
    }

    #[test]
    fn the_button_says_what_the_server_allows() {
        let label = |state: &State| { let (label, action, line) = state.play(); (label, action.is_some(), line) };
        assert_eq!(label(&tab(Status::Offline, None)), ("RETRY", true, String::new()), "the row already says Offline");
        assert_eq!(label(&tab(Status::Maintenance, None)).0, "RETRY");
        assert_eq!(label(&tab(Status::NeedsUpdate { min: "0.2.0".into() }, None)), ("UPDATE", true, "This server needs YO3 Revival 0.2.0 or newer.".into()));
        assert_eq!(label(&tab(Status::IdentityChanged, None)), ("LOG IN", false, Problem::IdentityChanged.sentence()));
        let mut state = tab(online(), None);
        assert_eq!(label(&state), ("LOG IN", false, String::new()), "nothing typed yet");
        let ctx = ctx();
        let _ = state.update(Message::Edit(Field::User, "yugi".into()), &ctx);
        let _ = state.update(Message::Edit(Field::Password, "LongPassphrase1".into()), &ctx);
        assert_eq!(label(&state).0, "LOG IN");
        assert!(label(&state).1);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::SignedIn(key, Ok(signed("Yugi", None))), &ctx);
        assert_eq!(label(&state), ("PLAY", true, String::new()), "the form already says who is signed in");
        assert!(State::default().play().1.is_none(), "no server: nothing to press");
    }

    #[test]
    fn a_play_whose_session_ended_asks_to_sign_in_again_and_one_that_cant_reach_the_server_keeps_it() {
        // Play checks a plain sign-in's session first (mp::session_alive): a kick, a ban or a newer sign-in elsewhere
        // ended it, and the game would only say "You will need an active Internet connection" (Windows 11 VM).
        let ctx = ctx();
        let mut state = tab(online(), None);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::SignedIn(key.clone(), Ok(signed("Yugi", None))), &ctx);
        state.play_refused(Problem::Unreachable);
        assert!(state.signed_in.contains_key(&key), "no answer: still signed in, Retry");
        assert_eq!(state.problem, Some(Problem::Unreachable));
        state.play_refused(Problem::SignInAgain);
        assert!(!state.signed_in.contains_key(&key));
        assert_eq!(state.problem, Some(Problem::SessionEnded));
        assert_eq!(Problem::SessionEnded.remedy(), Some(Remedy::SignInAgain));
        assert_eq!(state.play().0, "LOG IN");
        // A saved sign-in whose resume is refused at Play (a ban signs every device out) is forgotten too.
        let vault = Arc::new(MemoryVault::default());
        let mut state = tab(online(), Some(vault.clone()));
        let account = mp::vault_account(&state.servers[0]);
        let _ = state.update(Message::SignedIn(key, Ok(signed("Yugi", Some("device-token")))), &ctx);
        state.play_refused(Problem::SignInAgain);
        assert_eq!(vault.get(&account).unwrap(), None);
    }

    /// Play as… (#64): another account on the server this launcher is signed in to. The same account twice is refused
    /// with a clear sentence and nothing is asked of the server; a different one signs in and a game starts.
    #[test]
    fn play_as_refuses_the_same_account_twice_and_signs_in_a_different_one() {
        let ctx = ctx();
        let mut state = tab(online(), None);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::SignedIn(key.clone(), Ok(signed("Yugi", None))), &ctx);
        let _ = state.update(Message::Show(Form::PlayAs), &ctx);
        let _ = state.update(Message::Edit(Field::Password, "LongPassphrase1".into()), &ctx);
        // The account this launcher is signed in as on the server: PLAY is for that.
        let _ = state.update(Message::Edit(Field::Other, " yugi ".into()), &ctx);
        let _ = state.update(Message::PlayAsGo, &ctx);
        assert_eq!((state.problem.clone(), state.busy), (Some(Problem::SameAccount), false), "nothing was asked of the server");
        assert!(Problem::SameAccount.sentence().contains("PLAY") && yo3_desktop::words::technical(&Problem::SameAccount.sentence()).is_none());
        // An account already in a running game on this server.
        let game = GameInfo { id: 1, user: "Kaiba".into(), server: "Friends".into(), server_key: key.clone() };
        let _ = state.update(Message::Games(vec![game]), &ctx);
        let _ = state.update(Message::Edit(Field::Other, "KAIBA".into()), &ctx);
        let _ = state.update(Message::PlayAsGo, &ctx);
        assert_eq!((state.problem.clone(), state.busy), (Some(Problem::AlreadyPlaying), false));
        // The same User ID on another server is a different account.
        let other_server = GameInfo { id: 2, user: "Joey".into(), server: "Elsewhere".into(), server_key: "another".into() };
        let _ = state.update(Message::Games(vec![other_server]), &ctx);
        let _ = state.update(Message::Edit(Field::Other, "Joey".into()), &ctx);
        let _ = state.update(Message::PlayAsGo, &ctx);
        assert!(state.busy && state.problem.is_none(), "a different account signs in");
        assert!(state.password.is_empty(), "the password is not kept");
        // Its sign-in worked: the game starts, handled by the main screen, and the form closes.
        let _ = state.update(Message::PlayAsSignedIn(key.clone(), Ok(signed("Joey", None))), &ctx);
        assert!(!state.busy && state.form == Form::SignIn && state.note.as_deref().is_some_and(|n| n.contains("Joey")));
        // Once the start is over (the game runs, or said why it didn't) the line goes: it stayed for the whole session before.
        let _ = state.update(Message::StartDone, &ctx);
        assert!(state.note.is_none(), "{:?}", state.note);
        // The server's own refusal (the host's cap of games per PC) is a plain sentence.
        let _ = state.update(Message::PlayAsSignedIn(key, Err(Problem::DeviceFull { limit: 1 })), &ctx);
        assert!(state.problem.as_ref().unwrap().sentence().contains("1 game"), "{:?}", state.problem);
    }

    #[test]
    fn the_running_games_are_listed_with_their_account_and_server_and_the_main_ones_close_is_the_usual() {
        let ctx = ctx();
        let mut state = tab(online(), None);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::SignedIn(key.clone(), Ok(signed("Yugi", None))), &ctx);
        assert!(state.games.is_empty());
        let games = vec![GameInfo { id: 0, user: "Yugi".into(), server: "Friends".into(), server_key: key.clone() }, GameInfo { id: 3, user: "Joey".into(), server: "Friends".into(), server_key: key }];
        let _ = state.update(Message::Games(games), &ctx);
        assert_eq!(state.games.iter().map(|g| (g.user.as_str(), g.is_main())).collect::<Vec<_>>(), [("Yugi", true), ("Joey", false)]);
        // Rendered without panicking, with the list in it.
        let _ = state.view(None);
    }

    #[test]
    fn each_refusal_shows_its_sentence() {
        let ctx = ctx();
        for problem in [Problem::WrongUserOrPassword, Problem::TooManyTries { wait_seconds: 90 }, Problem::Banned { reason: "cheating".into(), until: None },
            Problem::Maintenance { message: String::new() }, Problem::Full, Problem::NeedsUpdate { min: "0.2.0".into() }, Problem::IdentityChanged, Problem::Unreachable, Problem::SignInAgain] {
            let mut state = tab(online(), None);
            let key = state.servers[0].key().to_string();
            let _ = state.update(Message::SignedIn(key, Err(problem.clone())), &ctx);
            assert_eq!(state.problem.as_ref(), Some(&problem));
            assert!(!state.busy && !state.signed_in.contains_key(state.servers[0].key()));
        }
        let mut state = tab(online(), None);
        let _ = state.update(Message::Registered(Err(Problem::InviteInvalid)), &ctx);
        assert_eq!(state.problem, Some(Problem::InviteInvalid));
        let _ = state.update(Message::Recovered(Err(Problem::RecoveryInvalid)), &ctx);
        assert_eq!(state.problem, Some(Problem::RecoveryInvalid));
    }

    #[test]
    fn stay_signed_in_keeps_the_token_in_the_vault_and_a_revoked_one_is_forgotten() {
        let vault = Arc::new(MemoryVault::default());
        let mut state = tab(online(), Some(vault.clone()));
        let ctx = ctx();
        let account = mp::vault_account(&state.servers[0]);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::SignedIn(key.clone(), Ok(signed("Yugi", Some("device-token")))), &ctx);
        assert_eq!(vault.get(&account).unwrap().as_deref(), Some("device-token"));
        assert_eq!(state.servers[0].saved_user.as_deref(), Some("Yugi"), "the User ID, never the password");
        // The token was revoked (a new password, Sign out everywhere, a ban): back to the form, and forgotten.
        state.signed_in.clear();
        let _ = state.update(Message::SignedIn(key, Err(Problem::SignInAgain)), &ctx);
        assert_eq!(state.problem.as_ref().map(Problem::sentence).as_deref(), Some("Please sign in again."));
        assert_eq!(vault.get(&account).unwrap(), None);
    }

    #[test]
    fn without_the_systems_credential_store_stay_signed_in_is_off_with_the_reason() {
        let mut state = tab(online(), None);
        assert!(!state.stay && state.vault_problem.is_some());
        let _ = state.update(Message::Toggle(Tick::Stay, true), &ctx());
        assert!(!state.stay, "it can't be turned on");
    }

    #[test]
    fn a_new_account_shows_its_recovery_code_once_and_a_server_met_first_is_trusted_on_request() {
        let ctx = ctx();
        let mut state = tab(online(), None);
        let _ = state.update(Message::Show(Form::Register), &ctx);
        let _ = state.update(Message::Registered(Ok("ABCD-EFGH-JKLM-NPQR".into())), &ctx);
        assert_eq!(state.form, Form::Code("ABCD-EFGH-JKLM-NPQR".into()));
        let _ = state.update(Message::CodeKept, &ctx);
        assert_eq!(state.form, Form::SignIn, "gone once saved");
        let met = server("Friends");
        let _ = state.update(Message::Show(Form::Add), &ctx);
        let _ = state.update(Message::Edit(Field::Address, "friends.example".into()), &ctx);
        assert_eq!(state.found, Found::Looking, "looked up once typing pauses");
        let _ = state.update(Message::Add, &ctx);
        assert_eq!(state.form, Form::Add, "Add waits for the server to answer");
        let _ = state.update(Message::Checked(state.check, Ok(met.clone())), &ctx);
        let _ = state.update(Message::Add, &ctx);
        assert_eq!(state.form, Form::Trust(Box::new(met.clone())));
        let _ = state.update(Message::Trust, &ctx);
        assert_eq!((state.servers.len(), state.selected.as_deref()), (2, Some(met.key())));
        let _ = state.update(Message::Remove, &ctx);
        assert_eq!(state.servers.len(), 1, "a server the player added can be removed");
        let _ = state.update(Message::Show(Form::Add), &ctx);
        let _ = state.update(Message::Edit(Field::Address, "nobody.example".into()), &ctx);
        let stale = state.check;
        let _ = state.update(Message::Edit(Field::Address, "nobody.example:7444".into()), &ctx);
        let _ = state.update(Message::Checked(stale, Ok(met.clone())), &ctx);
        assert_eq!(state.found, Found::Looking, "an older check's answer is ignored");
        let _ = state.update(Message::Checked(state.check, Err("refused".into())), &ctx);
        assert!(matches!(state.found, Found::No(_)));
        let _ = state.update(Message::Edit(Field::Address, "not an address".into()), &ctx);
        assert!(matches!(state.found, Found::No(_)), "said at once, nothing looked up");
    }

    /// #70: the invite code box is there whenever the server hasn't said it is open, required once it says invite.
    #[test]
    fn create_account_shows_the_invite_box_unless_the_server_said_it_is_open() {
        let with = |registration: &str| Status::Online { online: Some(1), max: None, registration: registration.into(), news: vec![], ping: std::time::Duration::from_millis(5), clean_game: false };
        assert_eq!(invite_ask(None), InviteAsk::Optional, "not asked yet: the box, not required");
        assert_eq!(invite_ask(Some(&Status::Offline)), InviteAsk::Optional);
        assert_eq!(invite_ask(Some(&with("invite"))), InviteAsk::Required);
        assert_eq!(invite_ask(Some(&with(""))), InviteAsk::Required, "an older server that doesn't say: as the server's default, invite");
        assert_eq!(invite_ask(Some(&with("open"))), InviteAsk::Hidden);
    }

    /// An invite link made for one friend adds its server and opens making the account, the invite code filled in.
    #[test]
    fn an_invite_link_for_one_friend_opens_the_sign_up_with_its_code() {
        let ctx = ctx();
        let mut state = tab(online(), None);
        let link = mp::link_with_code(&server("Friends"), "QUPM-Y9GG-BFAS-9WJ6");
        let _ = state.update(Message::Show(Form::Add), &ctx);
        let _ = state.update(Message::Edit(Field::Address, link), &ctx);
        assert!(matches!(&state.found, Found::Link(_, Some(code)) if code == "QUPM-Y9GG-BFAS-9WJ6"), "a link is read at once");
        let _ = state.update(Message::Add, &ctx);
        assert_eq!((state.form.clone(), state.invite.as_str(), state.servers.len()), (Form::Register, "QUPM-Y9GG-BFAS-9WJ6", 2));
    }

    /// Making an account says each rule as the player types, by the game's login screen's rules (the server checks them again).
    #[test]
    fn making_an_account_checks_each_rule_as_it_is_typed() {
        let states = |rules: Vec<(Rule, &str)>| rules.into_iter().map(|(r, _)| r).collect::<Vec<_>>();
        assert_eq!(states(register_rules("", "", "", Some(""))), [Rule::Pending; 5], "nothing typed: nothing red");
        assert_eq!(states(register_rules("Yu", "short", "shor", None)), [Rule::Broken, Rule::Broken, Rule::Met, Rule::Broken]);
        assert_eq!(states(register_rules("Yugi", "Yugi", "", None))[2], Rule::Broken, "not the User ID");
        assert_eq!(states(register_rules("Yugi", "Pass-word1", "", None))[1], Rule::Broken, "letters and digits only");
        assert_eq!(states(register_rules("Yugi", "Seventeen17chars1", "", None))[1], Rule::Broken, "16 at most");
        assert_eq!(states(register_rules("Yugi", "LongPass1", "LongPass1", Some("QUPM-Y9GG-BFAS-9WJ6"))), [Rule::Met; 5]);
        assert_eq!(states(register_rules("Yugi", "LongPass1", "LongPass1", Some("QUPM")))[4], Rule::Broken);
    }

    #[test]
    fn the_information_panel_shows_the_selected_servers_news() {
        let news = vec![yo3_tunnel::News { title: "Maintenance".into(), body: "Sunday".into() }];
        let state = tab(Status::Online { online: Some(3), max: None, registration: "invite".into(), news, ping: std::time::Duration::from_millis(40), clean_game: false }, None);
        assert_eq!(state.news(), [("Maintenance".to_string(), "Sunday".to_string())]);
        assert_eq!(tab(Status::Offline, None).news()[0].1, "Choose a server to see its news.");
    }
}
