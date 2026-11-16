//! The Multiplayer tab (#30): the server list, signing in, making an account and recovering one. The main screen
//! (`home.rs`) hosts it: its Play button starts the game with the session signed in here.
use std::collections::HashMap;
use std::sync::Arc;
use iced::widget::{Space, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment::Center, Element, Length::Fill, Task};
use yo3_desktop::multiplayer::{self as mp, Problem, Remedy, Server, SignedIn, Status, Vault};
use yo3_skin::{BODY_BOLD, Kind, color, pill};
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { User, Password, Again, Invite, Code, Address }

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
    Contacted(Result<Server, String>),
    Trust,
    Remove,
    Saved(Result<(), String>),
    Retry,
    /// Handled by the main screen: play on the selected server with its session.
    Play,
    /// Handled by the main screen: close the game Play started.
    CloseGame,
    Remedy(Remedy),
    /// The trust form's identity, behind Details.
    ShowIdentity,
    /// Handled by the window: Host for friends (#37).
    Host,
    /// Your hosted server, added to your own list (#37).
    Hosted(Box<Server>),
    /// Check this server's status again now (your hosted server, just stopped).
    Recheck(String),
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
        Task::batch([self.probe(&server), self.resume()])
    }

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
                    Field::Invite => self.invite = value, Field::Code => self.code = value, Field::Address => self.address = value }
            }
            Message::Toggle(tick, on) => match tick { Tick::SaveUser => self.save_user = on, Tick::Stay => self.stay = on && self.vault.is_some(), Tick::Rules => self.rules = on },
            Message::Show(form) => { self.form = form; self.problem = None; self.note = None; self.password.clear(); self.again.clear(); }
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
                    Ok(code) => { self.password.clear(); self.again.clear(); self.invite.clear(); self.form = Form::Code(code); }
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
                        self.password.clear(); self.again.clear(); self.code.clear();
                        match next {
                            Some(code) => self.form = Form::Code(code),
                            None => { self.form = Form::SignIn; self.note = Some("Your new password is set. Sign in with it.".into()) }
                        }
                    }
                    Err(problem) => self.problem = Some(problem),
                }
            }
            Message::Add => {
                let input = self.address.trim().to_string();
                self.problem = None;
                if input.starts_with("yo3revival://") {
                    // An invite link carries the identity: no first-use question.
                    return match mp::invite_link(&input) {
                        Ok(server) => self.add(ctx, server),
                        Err(error) => {
                            if let Some(backend) = &ctx.backend { backend.diagnostics.line(&format!("Invite link refused: {error:#}")) }
                            self.note = Some("That invite link isn't complete. Ask the host to send it again.".into());
                            Task::none()
                        }
                    };
                }
                self.busy = true;
                return Task::perform(async move { mp::first_contact(&input).await.map_err(|e| format!("{e:#}")) }, Message::Contacted);
            }
            Message::Contacted(result) => {
                self.busy = false;
                match result {
                    Ok(server) => { self.show_identity = false; self.form = Form::Trust(Box::new(server)) }
                    Err(_) => self.problem = Some(Problem::Unreachable),
                }
            }
            Message::Trust => if let Form::Trust(server) = std::mem::take(&mut self.form) { return self.add(ctx, *server) },
            Message::Remove => {
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
            Message::Remedy(Remedy::Update) | Message::Play | Message::CloseGame => {}
            Message::ShowIdentity => self.show_identity = !self.show_identity,
            Message::Host => {}
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
        let Some(token) = token else { return Task::none() };
        Task::perform(async move { mp::sign_out(&server, &token, false).await }, Message::SignedOut)
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
        for account in accounts {
            let state = match (account.signed_in, account.stays) {
                (_, true) => format!("Signed in as {}, and stays signed in on this PC.", account.user),
                _ => format!("Signed in as {}.", account.user),
            };
            let mut row_col = column![
                row![text(account.server.clone()).size(16).font(BODY_BOLD).color(color::NAVY), Space::new().width(Fill),
                    yo3_skin::link("Sign out", Message::AccountSignOut(account.key.clone()), false),
                    yo3_skin::link("Sign out everywhere", Message::AccountForm(account.key.clone(), Some(AccountForm::Everywhere)), false),
                    yo3_skin::link("Change password", Message::AccountForm(account.key.clone(), Some(AccountForm::Password)), false)].spacing(14).align_y(Center),
                text(state).size(14).color(color::INK_SOFT),
            ].spacing(4);
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
            col = col.push(container(row_col).padding([10, 12]).width(Fill).style(|_| container::Style { background: Some(color::ROW.into()),
                border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }));
        }
        if let Some((good, note)) = &self.account_note {
            let shade = if *good { color::GOOD } else { color::BAD };
            col = col.push(row![container(Space::new().width(9).height(9)).style(move |_| dot(shade)), text(note).size(14).font(BODY_BOLD).color(color::INK).width(Fill)]
                .spacing(8).align_y(Center));
        }
        col.into()
    }

    fn add(&mut self, ctx: &Ctx, server: Server) -> Task<Message> {
        if !self.servers.iter().any(|s| s.key() == server.key()) { self.servers.insert(0, server.clone()) }
        self.address.clear();
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

    /// The Play button's label and action for the selected server, and the line above it.
    fn play(&self, playing: bool) -> (&'static str, Option<Message>, String) {
        let Some(server) = self.selected() else { return ("LOG IN", None, "Add a server under Other servers.".into()) };
        let status = self.status.get(server.key());
        if playing { return ("PLAYING", None, "The game is running.".into()) }
        match status {
            Some(Status::Offline) => return ("RETRY", Some(Message::Retry), "We can't reach this server.".into()),
            Some(Status::Maintenance) => return ("RETRY", Some(Message::Retry), "The server is in maintenance.".into()),
            Some(Status::NeedsUpdate { min }) => return ("UPDATE", Some(Message::Remedy(Remedy::Update)), format!("This server needs YO3 Revival {min} or newer.")),
            Some(Status::IdentityChanged) => return ("LOG IN", None, Problem::IdentityChanged.sentence()),
            _ => {}
        }
        if self.signed_in.contains_key(server.key()) { return ("PLAY", Some(Message::Play), String::new()) }
        let ready = !self.busy && !self.user.trim().is_empty() && !self.password.is_empty() && self.form == Form::SignIn;
        ("LOG IN", ready.then_some(Message::SignIn), if self.busy { "Signing in…".into() } else { String::new() })
    }

    pub fn view<'a>(&'a self, playing: bool) -> Element<'a, Message> {
        let choices: Vec<Choice> = self.servers.iter().map(|s| Choice { key: s.key().to_string(),
            label: format!("{}{}  ·  {}", if s.official { "★ " } else { "" }, s.name, Self::row_status(self.status.get(s.key())).1) }).collect();
        let selected = choices.iter().find(|c| Some(&c.key) == self.selected.as_ref()).cloned();
        let picker: Element<'_, Message> = if choices.is_empty() {
            text("No servers yet.").size(15).color(color::INK_SOFT).into()
        } else {
            pick_list(choices, selected, Message::Select).style(yo3_skin::choice).menu_style(yo3_skin::choice_menu).width(Fill).padding([7, 10]).into()
        };
        let (label, action, status) = self.play(playing);
        let body = match &self.selected {
            Some(key) => {
                let (dot_color, line) = Self::row_status(self.status.get(key));
                let state = row![container(Space::new().width(9).height(9)).style(move |_| dot(dot_color)), text(line).size(13).color(color::INK_SOFT)]
                    .spacing(8).align_y(Center);
                column![picker, state, self.form()].spacing(10)
            }
            // Nothing to sign in to yet: only the way to add a server (or the add form itself).
            None if matches!(self.form, Form::Add | Form::Trust(_)) => column![picker, self.form()].spacing(10),
            None => column![picker, self.other_servers()].spacing(10),
        };
        let mut panel = column![body, self.messages(), Space::new().height(Fill)].spacing(10);
        // The forms with their own button (making an account, recovering, adding a server) hide the big one.
        if matches!(self.form, Form::SignIn) || self.selected.is_none() {
            let mut line = row![text(status).size(14).color(color::INK_SOFT)].align_y(Center);
            if playing { line = line.push(Space::new().width(Fill)).push(yo3_skin::link("Close game", Message::CloseGame, false)) }
            panel = panel.push(line).push(pill(label, action, Kind::Play, Fill, 70));
        }
        yo3_skin::panel(Some("Server"), panel)
    }

    fn messages(&self) -> Element<'_, Message> {
        let mut out = column![].spacing(6);
        if let Some(problem) = &self.problem {
            let mut line = row![container(Space::new().width(9).height(9)).style(|_| dot(color::BAD)),
                text(problem.sentence()).size(14).font(BODY_BOLD).color(color::INK).width(Fill)].spacing(8).align_y(Center);
            if let Some(remedy) = problem.remedy() {
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
            let mut field_input = text_input(placeholder, value).on_input(move |v| Message::Edit(field, v)).style(yo3_skin::field).padding(8).size(15);
            if secret { field_input = field_input.secure(true) }
            if !self.busy { field_input = field_input.on_submit(match field { Field::Address => Message::Add, _ => match self.form {
                Form::Register => Message::Register, Form::Recover => Message::Recover, _ => Message::SignIn } }) }
            field_input
        };
        let labelled = |label: &'static str, input: iced::widget::TextInput<'static, Message>| row![text(label).font(BODY_BOLD).size(14).color(color::INK).width(92), input].align_y(Center);
        let back = || yo3_skin::link("Back", Message::Show(Form::SignIn), false);
        let tick = |label: &'static str, on: bool, which: Tick| checkbox(on).label(label).on_toggle(move |v| Message::Toggle(which, v)).style(yo3_skin::tick).size(16).text_size(14);
        let selected = self.selected();
        let signed = selected.and_then(|s| self.signed_in.get(s.key()));
        match &self.form {
            Form::SignIn if signed.is_some() => column![
                row![text(format!("Signed in as {}", signed.map_or("", |s| s.user.as_str()))).size(15).font(BODY_BOLD).color(color::INK),
                    Space::new().width(Fill), yo3_skin::link("Sign out", Message::SignOut, false)].align_y(Center),
                self.other_servers(),
            ].spacing(10).into(),
            Form::SignIn => {
                let mut stay = tick("Stay signed in", self.stay, Tick::Stay);
                if self.vault.is_none() { stay = checkbox(false).label("Stay signed in").style(yo3_skin::tick).size(16).text_size(14) }
                let mut col = column![
                    labelled("User ID", input("", &self.user, Field::User, false)),
                    labelled("Password", input("", &self.password, Field::Password, true)),
                    row![tick("Save User ID", self.save_user, Tick::SaveUser), Space::new().width(16), stay].align_y(Center),
                ].spacing(8);
                if let Some(why) = &self.vault_problem { col = col.push(text(why).size(13).color(color::INK_SOFT)) }
                col.push(row![yo3_skin::link("Create account", Message::Show(Form::Register), false), text("·").color(color::INK_SOFT),
                    yo3_skin::link("Forgot password?", Message::Show(Form::Recover), false)].spacing(8).align_y(Center))
                    .push(self.other_servers()).into()
            }
            Form::Register => {
                let invite = matches!(selected.and_then(|s| self.status.get(s.key())), Some(Status::Online { registration, .. }) if registration != "open");
                let mut col = column![
                    text("Create an account").size(15).font(BODY_BOLD).color(color::INK),
                    labelled("User ID", input("3 to 16 letters or digits", &self.user, Field::User, false)),
                    labelled("Password", input("8 to 16 letters or digits", &self.password, Field::Password, true)),
                    labelled("Again", input("", &self.again, Field::Again, true)),
                ].spacing(8);
                if invite { col = col.push(labelled("Invite code", input("ABCD-EFGH-JKLM-NPQR", &self.invite, Field::Invite, false))) }
                col.push(tick("I accept this server's rules", self.rules, Tick::Rules))
                    .push(row![pill("Create", (!self.busy).then_some(Message::Register), Kind::Primary, 120, 34), Space::new().width(Fill), back()].align_y(Center))
                    .into()
            }
            Form::Recover => column![
                text("A new password").size(15).font(BODY_BOLD).color(color::INK),
                text("With the recovery code you saved at sign-up, or a reset code from the server's moderators.").size(13).color(color::INK_SOFT),
                labelled("User ID", input("", &self.user, Field::User, false)),
                labelled("Code", input("ABCD-EFGH-JKLM-NPQR", &self.code, Field::Code, false)),
                labelled("Password", input("8 to 16 letters or digits", &self.password, Field::Password, true)),
                labelled("Again", input("", &self.again, Field::Again, true)),
                row![pill("Set password", (!self.busy).then_some(Message::Recover), Kind::Primary, 150, 34), Space::new().width(Fill), back()].align_y(Center),
            ].spacing(8).into(),
            Form::Add => column![
                text("Another server").size(15).font(BODY_BOLD).color(color::INK),
                text("Its address (host or host:port), or an invite link a host sent you.").size(13).color(color::INK_SOFT),
                input("play.example.com or yo3revival://…", &self.address, Field::Address, false),
                row![pill("Add", (!self.busy && !self.address.trim().is_empty()).then_some(Message::Add), Kind::Primary, 110, 34), Space::new().width(Fill), back()].align_y(Center),
            ].spacing(8).into(),
            Form::Trust(server) => {
                let mut col = column![
                    text("First connection to this server").size(15).font(BODY_BOLD).color(color::INK),
                    text(format!("{} at {}", server.name, server.address)).size(14).color(color::INK),
                    text("YO3 Revival remembers this server and checks it is the same one each time. If the host gave you an invite link, use it instead.")
                        .size(13).color(color::INK_SOFT),
                    yo3_skin::link(if self.show_identity { "Hide details" } else { "Details" }, Message::ShowIdentity, false),
                ].spacing(8);
                if self.show_identity {
                    col = col.push(text(format!("Identity: {}", fingerprint(server.key()))).size(13).color(color::INK_SOFT));
                }
                col.push(row![pill("Add server", Some(Message::Trust), Kind::Primary, 130, 34), Space::new().width(Fill), back()].align_y(Center)).into()
            }
            Form::Code(code) => column![
                text("Save your recovery code").size(15).font(BODY_BOLD).color(color::INK),
                text("It's the only way back in if you forget your password. It's shown once: write it down or keep it in a password manager.")
                    .size(13).color(color::INK_SOFT),
                container(text(code).size(20).font(BODY_BOLD).color(color::INK)).padding([8, 12]).style(|_| container::Style {
                    background: Some(color::PANEL_TOP.into()), border: iced::Border { color: color::CHROME_DARK, width: 1.0, radius: 3.0.into() }, ..container::Style::default() }),
                row![pill("I saved it", Some(Message::Show(Form::SignIn)), Kind::Primary, 140, 34)],
            ].spacing(10).into(),
        }
    }

    fn other_servers(&self) -> Element<'_, Message> {
        let mut links = row![yo3_skin::link("▸ Other servers", Message::Show(Form::Add), false), yo3_skin::link("Host for friends", Message::Host, false)].spacing(14);
        if self.selected().is_some_and(|s| !s.official) { links = links.push(yo3_skin::link("Remove this server", Message::Remove, false)) }
        links.into()
    }
}

/// An identity as people compare it: groups of four.
fn fingerprint(identity: &str) -> String {
    identity.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or("")).collect::<Vec<_>>().join(" ")
}

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
    fn online() -> Status { Status::Online { online: Some(3), max: None, registration: "invite".into(), news: vec![], ping: std::time::Duration::from_millis(40) } }
    fn signed(user: &str, token: Option<&str>) -> SignedIn { SignedIn { user: user.into(), session: [7; 32], device_token: token.map(str::to_owned) } }

    #[test]
    fn a_stopped_hosted_server_is_checked_again_at_once() {
        let mut state = tab(online(), None);
        let key = state.servers[0].key().to_string();
        let _ = state.update(Message::Recheck(key.clone()), &ctx());
        assert!(!state.status.contains_key(&key), "no last check left showing while the new one runs");
        assert_eq!(State::row_status(state.status.get(&key)).1, State::row_status(None).1);
    }

    #[test]
    fn the_button_says_what_the_server_allows() {
        let label = |state: &State| { let (label, action, line) = state.play(false); (label, action.is_some(), line) };
        assert_eq!(label(&tab(Status::Offline, None)), ("RETRY", true, "We can't reach this server.".into()));
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
        assert_eq!(state.play(true).0, "PLAYING");
        assert!(State::default().play(false).1.is_none(), "no server: nothing to press");
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
        assert_eq!(state.play(false).0, "LOG IN");
        // A saved sign-in whose resume is refused at Play (a ban signs every device out) is forgotten too.
        let vault = Arc::new(MemoryVault::default());
        let mut state = tab(online(), Some(vault.clone()));
        let account = mp::vault_account(&state.servers[0]);
        let _ = state.update(Message::SignedIn(key, Ok(signed("Yugi", Some("device-token")))), &ctx);
        state.play_refused(Problem::SignInAgain);
        assert_eq!(vault.get(&account).unwrap(), None);
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
        let _ = state.update(Message::Show(Form::SignIn), &ctx);
        assert_eq!(state.form, Form::SignIn, "gone once saved");
        let met = server("Friends");
        let _ = state.update(Message::Contacted(Ok(met.clone())), &ctx);
        assert_eq!(state.form, Form::Trust(Box::new(met.clone())));
        let _ = state.update(Message::Trust, &ctx);
        assert_eq!((state.servers.len(), state.selected.as_deref()), (2, Some(met.key())));
        let _ = state.update(Message::Remove, &ctx);
        assert_eq!(state.servers.len(), 1, "a server the player added can be removed");
        let _ = state.update(Message::Contacted(Err("refused".into())), &ctx);
        assert_eq!(state.problem, Some(Problem::Unreachable));
    }

    #[test]
    fn the_information_panel_shows_the_selected_servers_news() {
        let news = vec![yo3_tunnel::News { title: "Maintenance".into(), body: "Sunday".into() }];
        let state = tab(Status::Online { online: Some(3), max: None, registration: "invite".into(), news, ping: std::time::Duration::from_millis(40) }, None);
        assert_eq!(state.news(), [("Maintenance".to_string(), "Sunday".to_string())]);
        assert_eq!(tab(Status::Offline, None).news()[0].1, "Choose a server to see its news.");
    }
}
