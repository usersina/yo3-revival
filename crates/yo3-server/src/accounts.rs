//! Persistent accounts: password accounts (a User ID and password, #23) and key accounts (the launcher's own
//! Singleplayer account and the host's tools), each with up to four players.
//!
//! Launchers sign in through the TLS transport: by password or a device's token (`account_ops`), or with a key
//! (`authenticate`). The address a client connects from never selects an account.
use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;


use serde_json::{Value as Json, json};

use crate::front::{Account, COMMENTS, Deck, Player, SPEECHES};
use crate::store;

/// The client shows four player slots.
pub const MAX_PLAYERS: usize = 4;
/// The longest player name the server takes, in characters (**guess**: the client's name field has its own limit).
pub const MAX_NAME: usize = 16;
/// The BOOSTERPASS Points (the shop's BP, the account's dpoint) a new account starts with: 50 packs (**guess**).
pub const START_BP: u32 = 5000;

/// A change to one player (`Directory::update_players`).
pub type PlayerChange<'a> = (u32, Box<dyn FnOnce(&mut Player) + 'a>);

/// The characters of a key (URL-safe).
const KEY_CHARS: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// The characters in a key.
pub const KEY_LEN: usize = 20;

/// The characters of a BOOSTERPASS code: the game's window (BpInputWindow 0x40F7B0) takes 16 letters and digits.
pub const CODE_LEN: usize = 16;

/// A BOOSTERPASS code the host made (`add-code`): the BP it gives, how many accounts may use it (any number when none),
/// and the accounts (user ids) that did, each once.
#[derive(Clone, Debug, PartialEq)]
pub struct BpCode {
    pub code: String,
    pub bp: u32,
    pub uses: Option<u32>,
    pub used_by: Vec<u32>,
    /// When each account in `used_by` claimed it (UTC, RFC 3339), at the same position; `None` for claims saved before
    /// claim times were kept.
    pub claimed_at: Vec<Option<String>>,
}

/// An account with how a launcher finds it: by its key, or by its User ID and password.
#[derive(Clone)]
struct Entry {
    key: Option<String>,
    /// A password account's User ID, password, role, standing, recovery code and signed-in devices (#23). Accounts
    /// with only a key are the launcher's own (Singleplayer) and the host's tools'.
    login: Option<crate::logins::Login>,
    /// Shared until changed (copy-on-write): a change gives this entry a new `Arc`, which is how a save finds the
    /// accounts that changed without serializing the others (#22).
    account: Arc<Account>,
}

/// A uniformly sampled account credential from the operating system's cryptographic RNG.
pub(crate) fn new_key() -> String {
    let mut key = String::with_capacity(KEY_LEN);
    while key.len() < KEY_LEN {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).expect("OS randomness unavailable");
        for b in bytes {
            if (b as usize) < 256 / KEY_CHARS.len() * KEY_CHARS.len() {
                key.push(KEY_CHARS[b as usize % KEY_CHARS.len()] as char);
                if key.len() == KEY_LEN { break }
            }
        }
    }
    key
}

/// The key of the test fixture account named by `addr` (`Directory::account_for`).
fn fixture_key(addr: IpAddr) -> String { crate::logins::digest(&format!("fixture {addr}"))[..KEY_LEN].to_string() }

#[derive(Clone)]
pub struct Directory {
    player_names: Vec<String>,
    accounts: Vec<Entry>,
    next_user: u32,
    next_player: u32,
    /// Where the accounts are kept (none: in memory only).
    file: Option<PathBuf>,
    /// The host's BOOSTERPASS codes.
    codes: Vec<BpCode>,
    /// Invite codes for new password accounts (#23).
    invites: Vec<crate::logins::Invite>,
    /// Each account as last saved (by user id): its `Arc` and the entry's key and login. A save writes
    /// the accounts whose `Arc` or fields differ, and drops the ones that went.
    saved: HashMap<u32, Saved>,
}

/// An account as last saved: compared by pointer, so an unchanged account costs nothing to check.
#[derive(Clone)]
struct Saved { account: Arc<Account>, key: Option<String>, login: Option<crate::logins::Login> }

impl Saved {
    fn of(e: &Entry) -> Saved { Saved { account: e.account.clone(), key: e.key.clone(), login: e.login.clone() } }
    fn same(&self, e: &Entry) -> bool { Arc::ptr_eq(&self.account, &e.account) && self.key == e.key && self.login == e.login }
}

impl Directory {
    /// The account `pred` picks, to change: only that account is copied (copy-on-write), the others stay shared.
    fn account_mut_where(&mut self, pred: impl Fn(&Account) -> bool) -> Option<&mut Account> {
        self.accounts.iter_mut().find(|e| pred(&e.account)).map(|e| Arc::make_mut(&mut e.account))
    }

    /// The player with `player_id`, to change (copying only their account).
    fn player_mut(&mut self, player_id: u32) -> Option<&mut Player> {
        self.account_mut_where(|a| a.players.iter().any(|p| p.player_id == player_id))?.players.iter_mut().find(|p| p.player_id == player_id)
    }

    /// The entry of the password account with User ID `user` (ignoring case).
    fn login_at(&self, user: &str) -> Option<usize> {
        self.accounts.iter().position(|e| e.login.as_ref().is_some_and(|l| l.user.eq_ignore_ascii_case(user)))
    }

    fn login_mut(&mut self, user_id: u32) -> Option<&mut crate::logins::Login> {
        self.accounts.iter_mut().find(|e| e.account.user_id == user_id).and_then(|e| e.login.as_mut())
    }

    /// The password account with User ID `user` (ignoring case): its user id and login.
    pub fn login_of(&self, user: &str) -> Option<(u32, crate::logins::Login)> {
        let e = &self.accounts[self.login_at(user)?];
        Some((e.account.user_id, e.login.clone()?))
    }

    /// The login of the account `user_id`, if it is a password account.
    pub fn login_by_id(&self, user_id: u32) -> Option<&crate::logins::Login> {
        self.accounts.iter().find(|e| e.account.user_id == user_id).and_then(|e| e.login.as_ref())
    }

    /// Makes a password account with no players yet (they are made in the game's Select Player), under the server's
    /// registration mode: an invite code is used up when one is needed. Its user id.
    pub fn register(&mut self, login: crate::logins::Login, invite: Option<&str>, mode: crate::logins::Registration) -> Result<u32, String> {
        let invite_at = self.admission(&login.user, invite, mode)?;
        self.atomic(|d| {
            let account = d.create_account(&[]);
            let user_id = account.user_id;
            if let Some(at) = invite_at {
                let invite = &mut d.invites[at];
                invite.uses = invite.uses.map(|n| n.saturating_sub(1));
                invite.used_by.push(user_id);
            }
            d.accounts.push(Entry { key: None, login: Some(login), account: Arc::new(account) });
            Ok(user_id)
        })
    }

    /// A host-created account waits for its owner to set a password with this one-time code.
    pub(crate) fn prepare_password_account(&self, user: &str, digest: String, expires: i64) -> Result<(Self, u32), String> {
        self.can_register(user, None, crate::logins::Registration::Open)?;
        self.next_user.checked_add(1).ok_or("The account number limit has been reached.")?;
        let mut next = self.clone();
        let account = next.create_account(&[]);
        let user_id = account.user_id;
        let mut login = crate::logins::Login::new(user, String::new(), crate::logins::digest(&crate::logins::new_token()));
        login.reset = Some((digest, expires));
        next.accounts.push(Entry { key: None, login: Some(login), account: Arc::new(account) });
        Ok((next, user_id))
    }

    pub(crate) fn prepare_reset(&self, user_id: u32, digest: String, expires: i64) -> Result<Self, String> {
        let mut next = self.clone();
        next.login_mut(user_id).ok_or("No such password account.")?.reset = Some((digest, expires));
        Ok(next)
    }

    /// Whether `user` could sign up now with `invite` under `mode`, without using the invite: the account requests
    /// ask before hashing the password (security review, #39), and [`Directory::register`] asks again.
    pub fn can_register(&self, user: &str, invite: Option<&str>, mode: crate::logins::Registration) -> Result<(), String> {
        self.admission(user, invite, mode).map(|_| ())
    }

    /// The invite a sign-up would use (its place), or why it can't sign up.
    fn admission(&self, user: &str, invite: Option<&str>, mode: crate::logins::Registration) -> Result<Option<usize>, String> {
        use crate::logins::Registration;
        if self.login_at(user).is_some() { return Err("user_id_taken".into()) }
        let now = crate::logins::now();
        match mode {
            Registration::Closed => Err("registration_closed".into()),
            Registration::Open => Ok(None),
            Registration::Invite => {
                let code = crate::logins::normalize_code(invite.ok_or("invite_required")?);
                Ok(Some(self.invites.iter().position(|i| crate::logins::normalize_code(&i.code) == code && i.usable(now)).ok_or("invite_invalid")?))
            }
        }
    }

    /// The account a device's sign-in belongs to (`digest` of its token), if it is still valid at `now`.
    pub fn device_user(&self, digest: &str, now: i64) -> Option<u32> {
        self.accounts.iter().find(|e| e.login.as_ref().is_some_and(|l| l.devices.iter().any(|d| d.token == digest && d.expires > now)))
            .map(|e| e.account.user_id)
    }

    /// Keeps a device signed in to `user_id` (the oldest goes when there are too many).
    pub fn add_device(&mut self, user_id: u32, device: crate::logins::Device) -> Result<(), String> {
        self.atomic(|d| {
            let login = d.login_mut(user_id).ok_or("no such account")?;
            login.devices.retain(|x| x.expires > device.created);
            while login.devices.len() >= crate::logins::MAX_DEVICES { login.devices.remove(0); }
            login.devices.push(device);
            Ok(())
        })
    }

    /// A device's sign-in was used: it lasts another [`crate::logins::DEVICE_TTL`] from `now`.
    pub fn touch_device(&mut self, user_id: u32, digest: &str, now: i64) -> Result<(), String> {
        self.atomic(|d| {
            let device = d.login_mut(user_id).and_then(|l| l.devices.iter_mut().find(|x| x.token == digest)).ok_or("no such device")?;
            device.last_used = now;
            device.expires = now + crate::logins::DEVICE_TTL;
            Ok(())
        })
    }

    /// Signs one device out (by its token's digest); whether it was signed in.
    pub fn revoke_device(&mut self, digest: &str) -> Result<bool, String> {
        let Some(user_id) = self.device_user(digest, i64::MIN) else { return Ok(false) };
        self.atomic(|d| { d.login_mut(user_id).ok_or("no such account")?.devices.retain(|x| x.token != digest); Ok(true) })
    }

    /// Signs every device of `user_id` out.
    pub fn revoke_devices(&mut self, user_id: u32) -> Result<(), String> {
        self.atomic(|d| { let login = d.login_mut(user_id).ok_or("no such account")?;
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("session revision exhausted")?; login.devices.clear(); Ok(()) })
    }

    /// A new password (its PHC hash): every device is signed out and any reset code is used up.
    pub fn set_password(&mut self, user_id: u32, hash: String) -> Result<(), String> {
        self.atomic(|d| {
            let login = d.login_mut(user_id).ok_or("no such account")?;
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("session revision exhausted")?;
            login.password = hash;
            login.devices.clear();
            login.reset = None;
            Ok(())
        })
    }

    /// The password verified outside the lock must still be current when its replacement commits.
    pub fn change_password_if_current(&mut self, user_id: u32, expected: &crate::logins::Login, hash: String) -> Result<(), String> {
        let login = self.login_by_id(user_id).ok_or("invalid_credentials")?;
        if login.password != expected.password || login.recovery != expected.recovery || login.staff_epoch != expected.staff_epoch { return Err("invalid_credentials".into()) }
        self.set_password(user_id, hash)
    }

    /// Consume a still-current recovery/reset code with the password change in one save. Hashing happens outside the lock.
    pub fn complete_recovery(&mut self, user_id: u32, expected: &crate::logins::Login, code: &str, hash: String, recovery: String, now: i64) -> Result<bool, String> {
        self.atomic(|d| {
            let login = d.login_mut(user_id).ok_or("invalid_recovery")?;
            if login.password != expected.password || login.recovery != expected.recovery || login.reset != expected.reset || login.staff_epoch != expected.staff_epoch {
                return Err("invalid_recovery".into())
            }
            let by_recovery = login.recovery == code;
            if !by_recovery && !login.reset.as_ref().is_some_and(|(digest, until)| digest == code && *until > now) { return Err("invalid_recovery".into()) }
            let rotate = by_recovery || login.password.is_empty();
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("session revision exhausted")?;
            login.password = hash; login.devices.clear(); login.reset = None;
            if rotate { login.recovery = recovery; }
            Ok(rotate)
        })
    }

    /// A new recovery code (its digest), after the old one was used.
    pub fn set_recovery(&mut self, user_id: u32, digest: String) -> Result<(), String> {
        self.atomic(|d| { d.login_mut(user_id).ok_or("no such account")?.recovery = digest; Ok(()) })
    }

    /// A moderator's reset code for User ID `user`: a new code, working until `expires`.
    pub fn issue_reset(&mut self, user: &str, digest: String, expires: i64) -> Result<u32, String> {
        let at = self.login_at(user).ok_or("no such User ID")?;
        let user_id = self.accounts[at].account.user_id;
        self.atomic(|d| { d.login_mut(user_id).ok_or("no such account")?.reset = Some((digest, expires)); Ok(user_id) })
    }

    /// Sets the role of User ID `user`.
    pub fn set_role(&mut self, user: &str, role: crate::logins::Role) -> Result<(), String> {
        let at = self.login_at(user).ok_or("no such User ID")?;
        let user_id = self.accounts[at].account.user_id;
        let mut next = self.prepare_role(user_id, role)?;
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(())
    }

    pub(crate) fn prepare_role(&self, user_id: u32, role: crate::logins::Role) -> Result<Self, String> {
        let mut next = self.clone();
        let login = next.login_mut(user_id).ok_or("Only password accounts can have a staff role.")?;
        if login.password.is_empty() && role != crate::logins::Role::Player { return Err("Finish password setup before assigning a staff role.".into()) }
        if login.role != role {
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("The staff session revision limit has been reached.")?;
            login.role = role;
            login.reset = None;
        }
        Ok(next)
    }

    /// Sets the standing of User ID `user` (active, restricted or banned; #26 makes the moderation around it).
    pub fn set_standing(&mut self, user: &str, standing: crate::logins::Standing) -> Result<u32, String> {
        let at = self.login_at(user).ok_or("no such User ID")?;
        let user_id = self.accounts[at].account.user_id;
        self.atomic(|d| { d.login_mut(user_id).ok_or("no such account")?.standing = standing; Ok(user_id) })
    }

    /// The password account a moderator names (a User ID, or one of its players' names): its user id and login.
    pub fn login_named(&self, who: &str) -> Result<(u32, crate::logins::Login), String> {
        let user_id = self.user_of(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
        let login = self.login_by_id(user_id).cloned().ok_or_else(|| format!("{who:?} is a key account: only password accounts have a standing"))?;
        Ok((user_id, login))
    }

    /// Sets the standing of account `user_id` (#26).
    pub fn set_standing_of(&mut self, user_id: u32, standing: crate::logins::Standing) -> Result<(), String> {
        self.atomic(|d| { d.login_mut(user_id).ok_or("no such account")?.standing = standing; Ok(()) })
    }

    /// Mutes account `user_id`, or lifts its mute (`None`).
    pub fn set_mute(&mut self, user_id: u32, mute: Option<crate::logins::Mute>) -> Result<(), String> {
        self.atomic(|d| { d.login_mut(user_id).ok_or("no such account")?.muted = mute; Ok(()) })
    }

    /// The mute of the account holding `player_id` that is still running at `now`, if any.
    pub fn mute_of_player(&self, player_id: u32, now: i64) -> Option<crate::logins::Mute> {
        self.accounts.iter().find(|e| e.account.players.iter().any(|p| p.player_id == player_id))
            .and_then(|e| e.login.as_ref()).and_then(|l| l.mute(now)).cloned()
    }

    /// Makes an invite code: for `uses` accounts (any number if `None`), until `expires`, recorded as `issued_by`'s.
    pub fn add_invite(&mut self, uses: Option<u32>, expires: Option<i64>, issued_by: &str) -> Result<String, String> {
        let (mut next, code) = self.prepare_invite(uses, expires, issued_by);
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(code)
    }

    pub(crate) fn prepare_invite(&self, uses: Option<u32>, expires: Option<i64>, issued_by: &str) -> (Self, String) {
        let mut code = crate::logins::new_code();
        while self.invites.iter().any(|i| i.code == code) { code = crate::logins::new_code(); }
        let mut next = self.clone();
        next.invites.push(crate::logins::Invite { code: code.clone(), uses, expires, issued_by: issued_by.into(), used_by: vec![] });
        (next, code)
    }

    /// The invite codes, oldest first.
    pub fn invites(&self) -> &[crate::logins::Invite] { &self.invites }

    /// Publish a live administration change only after its entire snapshot commits. Failed writes restore memory.
    pub fn atomic<T>(&mut self, change: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        let before = self.clone();
        self.file = None;
        let result = change(self);
        self.file.clone_from(&before.file);
        match result {
            Ok(value) => match self.persist() {
                Ok(()) => Ok(value),
                Err(error) => { *self = before; Err(format!("change was not saved: {error}")) },
            },
            Err(error) => { *self = before; Err(error) },
        }
    }

    /// `player_names`: the players every new account starts with (at most four are used).
    pub fn new(player_names: Vec<String>) -> Directory {
        Directory { player_names, accounts: Vec::new(), next_user: 1001, next_player: 1001, file: None, codes: vec![], invites: vec![], saved: HashMap::new() }
    }

    /// The accounts kept in `file` (none yet if it doesn't exist); changes are written back to it.
    pub fn open(player_names: Vec<String>, file: &Path) -> io::Result<Directory> {
        Self::from_document(player_names, file, store::read(file)?)
    }

    pub(crate) fn from_document(player_names: Vec<String>, file: &Path, saved: Option<Json>) -> io::Result<Directory> {
        let mut d = Directory { file: Some(file.to_path_buf()), ..Directory::new(player_names) };
        let Some(saved) = saved else { return Ok(d) };
        // The counters are kept: ids are never handed out twice, even after the newest player was deleted (the
        // friends, mail and rentals of the old one would else belong to the new).
        d.next_player = d.next_player.max(saved["next_player"].as_u64().unwrap_or(0) as u32);
        d.next_user = d.next_user.max(saved["next_user"].as_u64().unwrap_or(0) as u32);
        for a in saved["accounts"].as_array().into_iter().flatten() {
            let bad = || io::Error::new(io::ErrorKind::InvalidData, format!("{}: invalid account record (check its ids and identity)", file.display()));
            let key = a["key"].as_str().filter(|k| !k.is_empty()).map(str::to_string);
            let login: Option<crate::logins::Login> = match a.get("login") { Some(l) => Some(serde_json::from_value(l.clone()).map_err(|_| bad())?), None => None };
            if key.is_none() && login.is_none() {
                // Found only by the address it connected from: that login is gone (#23), and so is the account.
                tracing::warn!("account {} has neither a key nor a password and is no longer loaded", a["user_id"]);
                continue;
            }
            let user_id = a["user_id"].as_u64().ok_or_else(bad)? as u32;
            let mut players = vec![];
            for (slot, p) in a["players"].as_array().into_iter().flatten().take(MAX_PLAYERS).enumerate() {
                let mut player = Player::new(p["player_id"].as_u64().ok_or_else(bad)? as u32, p["name"].as_str().ok_or_else(bad)?);
                player.index = p["index"].as_u64().filter(|&i| i < MAX_PLAYERS as u64).map_or(slot as u32, |i| i as u32);
                player.level = p["level"].as_u64().unwrap_or(1) as u32;
                if let Some(avatar) = p["avatar"].as_str().and_then(from_hex) {
                    player.avatar = avatar;
                }
                let lines = |k: &str, n: usize| -> Vec<String> {
                    let saved: Vec<String> = p[k].as_array().into_iter().flatten().map(|l| l.as_str().unwrap_or_default().to_string()).collect();
                    (0..n).map(|i| saved.get(i).cloned().unwrap_or_default()).collect()
                };
                player.speeches = lines("speeches", SPEECHES);
                player.comments = lines("comments", COMMENTS);
                player.title = p["title"].as_u64().unwrap_or(0) as u32;
                player.decks = p["decks"].as_array().into_iter().flatten().filter_map(|d| d.as_u64()).map(|d| d as u32).collect();
                for deck in p["uploaded"].as_array().into_iter().flatten() {
                    let rows = |k: &str| -> Vec<u16> { deck[k].as_array().into_iter().flatten().filter_map(|r| r.as_u64()).map(|r| r as u16).collect() };
                    player.uploaded.push(Deck {
                        id: deck["id"].as_u64().unwrap_or(0) as u32,
                        name: deck["name"].as_str().unwrap_or_default().to_string(),
                        main: rows("main"),
                        extra: rows("extra"),
                        side: rows("side"),
                    });
                }
                player.items = p["items"].as_array().into_iter().flatten().filter_map(|i| i.as_u64()).map(|i| i as u16).collect();
                for pair in p["given"].as_array().into_iter().flatten() {
                    if let (Some(row), Some(count)) = (pair[0].as_u64(), pair[1].as_u64()) {
                        player.given.insert(row as u16, count as u32);
                    }
                }
                for pair in p["cards"].as_array().into_iter().flatten() {
                    if let (Some(row), Some(count)) = (pair[0].as_u64(), pair[1].as_u64()) {
                        player.cards.insert(row as u16, count as u32);
                    }
                }
                player.eval_num = p["eval_num"].as_u64().unwrap_or(0) as u32;
                player.eval_total = p["eval_total"].as_i64().unwrap_or(0) as i32;
                if let Some(counters) = p["counters"].as_object() {
                    player.counters = counters.iter().filter_map(|(k, v)| Some((k.clone(), v.as_u64()? as u32))).collect();
                }
                d.next_player = d.next_player.max(player.player_id + 1);
                players.push(player);
            }
            d.next_user = d.next_user.max(user_id + 1);
            let mut account = Directory::account(user_id, players);
            account.dpoint = a["dpoint"].as_u64().map_or(START_BP, |n| n as u32);
            account.country = a["country"].as_str().unwrap_or_default().to_string();
            for p in &mut account.players {
                p.country.clone_from(&account.country);
            }
            d.accounts.push(Entry { key, login, account: Arc::new(account) });
        }
        d.saved = d.accounts.iter().map(|e| (e.account.user_id, Saved::of(e))).collect();
        d.invites = saved["invites"].as_array().into_iter().flatten().filter_map(|i| serde_json::from_value(i.clone()).ok()).collect();
        for c in saved["bp_codes"].as_array().into_iter().flatten() {
            let (Some(code), Some(bp)) = (c["code"].as_str(), c["bp"].as_u64()) else { continue };
            let used_by: Vec<u32> = c["used_by"].as_array().into_iter().flatten().filter_map(|u| u.as_u64()).map(|u| u as u32).collect();
            let times = c["claimed_at"].as_array();
            let claimed_at = (0..used_by.len()).map(|i| times.and_then(|t| t.get(i)).and_then(|t| t.as_str()).map(str::to_string)).collect();
            d.codes.push(BpCode { code: code.to_string(), bp: bp as u32, uses: c["uses"].as_u64().map(|n| n as u32), used_by, claimed_at });
        }
        Ok(d)
    }

    fn account(user_id: u32, players: Vec<Player>) -> Account {
        Account { user_id, name: format!("user{user_id}"), dpoint: START_BP, players, country: String::new() }
    }

    /// Tests only: the fixture account named by a made-up address (through a key made from it, so it survives a
    /// restart), made with the starting players (their names made unique) the first time.
    #[doc(hidden)]
    pub fn account_for(&mut self, addr: IpAddr) -> Account {
        let key = fixture_key(addr);
        if let Some(e) = self.accounts.iter().find(|e| e.key.as_deref() == Some(key.as_str())) {
            return Account::clone(&e.account);
        }
        let names: Vec<String> = self.player_names.iter().take(MAX_PLAYERS).cloned().collect();
        let names: Vec<String> = names.iter().map(|n| self.free_name(n)).collect();
        let account = self.create(addr, &names);
        self.save();
        account
    }

    fn create(&mut self, addr: IpAddr, names: &[String]) -> Account {
        let account = self.create_account(names);
        self.accounts.push(Entry { key: Some(fixture_key(addr)), login: None, account: Arc::new(account.clone()) });
        account
    }

    /// Makes an account found by key only, with players named `names` (at most four, made unique; the starting
    /// players when empty); its key.
    pub fn add_account(&mut self, names: &[String]) -> Result<String, String> {
        let names: Vec<String> = if names.is_empty() {
            self.player_names.iter().take(MAX_PLAYERS).map(|n| self.free_name(n)).collect()
        } else {
            names.to_vec()
        };
        if names.len() > MAX_PLAYERS || names.iter().any(|n| n.trim().is_empty() || n.chars().count() > MAX_NAME || n.chars().any(char::is_control)) {
            return Err(format!("an account takes one to {MAX_PLAYERS} players with names of 1 to {MAX_NAME} characters"));
        }
        if let Some(n) = names.iter().enumerate().find_map(|(i, n)| (self.taken(n) || names[..i].iter().any(|m| m.eq_ignore_ascii_case(n))).then_some(n)) {
            return Err(format!("the name {n:?} is taken"));
        }
        self.atomic(|d| {
            let account = d.create_account(&names);
            let key = new_key();
            d.accounts.push(Entry { key: Some(key.clone()), login: None, account: Arc::new(account) });
            Ok(key)
        })
    }

    /// Whether `key` is an account's.
    pub fn has_key(&self, key: &str) -> bool {
        !key.is_empty() && self.accounts.iter().any(|e| e.key.as_deref() == Some(key))
    }

    /// Protected transport login: no source-address association and no implicit account creation.
    pub fn authenticate(&self, key: &[u8]) -> Option<Account> {
        use subtle::ConstantTimeEq;
        if key.len() != KEY_LEN { return None }
        self.accounts.iter().find(|e| e.key.as_ref().is_some_and(|k| bool::from(k.as_bytes().ct_eq(key))))
            .map(|e| Account::clone(&e.account))
    }

    /// The key of the account at `who` (a User ID, or a player's name), made if the account has none.
    pub fn key_of(&mut self, who: &str) -> Result<String, String> {
        let at = self.find(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
        if let Some(key) = &self.accounts[at].key {
            return Ok(key.clone());
        }
        let key = new_key();
        self.atomic(|d| { d.accounts[at].key = Some(key.clone()); Ok(key) })
    }

    /// The account an admin argument names: a User ID, or a player's name (ignoring case).
    fn find(&self, who: &str) -> Option<usize> {
        self.login_at(who).or_else(|| self.accounts.iter().position(|e| e.account.players.iter().any(|p| p.name.eq_ignore_ascii_case(who))))
    }

    /// The player ids of the account at `who` (a User ID, or a player's name): who sees a BP change (`Hub::push_bp`).
    pub fn players_of(&self, who: &str) -> Vec<u32> {
        self.find(who).map(|at| self.accounts[at].account.players.iter().map(|p| p.player_id).collect()).unwrap_or_default()
    }

    /// The BP of the account holding `player_id`.
    pub fn player_bp(&self, player_id: u32) -> Option<u32> {
        self.accounts.iter().map(|e| &e.account).find(|a| a.players.iter().any(|p| p.player_id == player_id)).map(|a| a.dpoint)
    }

    /// The players (id and name) of the account at `who` (a User ID, or a player's name).
    pub fn named_players_of(&self, who: &str) -> Vec<(u32, String)> {
        self.find(who).map(|at| self.accounts[at].account.players.iter().map(|p| (p.player_id, p.name.clone())).collect()).unwrap_or_default()
    }

    /// The player ids of the account holding `player_id` (empty when there is none).
    pub fn account_players(&self, player_id: u32) -> Vec<u32> {
        self.accounts.iter().map(|e| &e.account).find(|a| a.players.iter().any(|p| p.player_id == player_id))
            .map(|a| a.players.iter().map(|p| p.player_id).collect()).unwrap_or_default()
    }

    /// The id of the player named `name` (ignoring case).
    pub fn player_id_named(&self, name: &str) -> Option<u32> {
        self.accounts.iter().flat_map(|e| &e.account.players).find(|p| p.name.eq_ignore_ascii_case(name)).map(|p| p.player_id)
    }

    /// The user id of the account at `who` (a User ID, or a player's name).
    pub fn user_of(&self, who: &str) -> Option<u32> {
        self.find(who).map(|i| self.accounts[i].account.user_id)
    }

    /// Gives account `user_id` a player named `name`.
    pub fn add_player_to(&mut self, user_id: u32, name: &str) -> Result<(), String> {
        let (mut next, _) = self.prepare_add_player(user_id, name)?;
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(())
    }

    pub(crate) fn prepare_add_player(&self, user_id: u32, name: &str) -> Result<(Self, u32), String> {
        let name = checked_name(name)?;
        if self.taken(&name) { return Err("That player name is already taken.".into()) }
        let id = self.next_player;
        let following = id.checked_add(1).ok_or("The player number limit has been reached.")?;
        let mut next = self.clone();
        let a = next.account_mut_where(|a| a.user_id == user_id).ok_or("No such account.")?;
        let slot = free_slot(&a.players).filter(|_| a.players.len() < MAX_PLAYERS).ok_or("This account already has four players.")?;
        let mut player = Player::new(id, &name);
        player.index = slot;
        player.country.clone_from(&a.country);
        a.players.push(player);
        next.next_player = following;
        Ok((next, id))
    }

    /// Sets the country of account `user_id` (see `set_country`).
    pub fn set_country_of(&mut self, user_id: u32, code: &str) -> Result<(), String> {
        let at = self.accounts.iter().position(|e| e.account.user_id == user_id);
        self.set_country_at(at, code)
    }

    fn create_account(&mut self, names: &[String]) -> Account {
        let user_id = self.next_user;
        self.next_user += 1;
        let mut players = Vec::new();
        for (slot, name) in names.iter().enumerate() {
            let mut p = Player::new(self.next_player, name);
            p.index = slot as u32;
            self.next_player += 1;
            players.push(p);
        }
        Directory::account(user_id, players)
    }

    fn taken(&self, name: &str) -> bool {
        self.accounts.iter().map(|e| &e.account).flat_map(|a| &a.players).any(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// `name`, or the first of name2, name3... nobody has.
    fn free_name(&self, name: &str) -> String {
        std::iter::once(name.to_string()).chain((2..).map(|n| format!("{name}{n}"))).find(|n| !self.taken(n)).unwrap_or_default()
    }

    /// Tests only: sets the country flag of the fixture account at `addr` (an ISO code of the client's `countrytbl.bin`, two letters in
    /// any case); an empty code clears it, so the flag follows the client's language again.
    #[doc(hidden)]
    pub fn set_country(&mut self, addr: IpAddr, code: &str) -> Result<(), String> {
        let code = code.trim().to_ascii_uppercase();
        if !code.is_empty() && !(code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())) {
            return Err(format!("{code:?} is not a two-letter country code"));
        }
        let key = fixture_key(addr);
        let at = self.accounts.iter().position(|e| e.key.as_deref() == Some(key.as_str()));
        if at.is_none() {
            return Err(format!("no account at {addr}"));
        }
        self.set_country_at(at, &code)
    }

    fn set_country_at(&mut self, at: Option<usize>, code: &str) -> Result<(), String> {
        let user = at.map(|i| self.accounts[i].account.user_id).ok_or("no such account")?;
        let mut next = self.prepare_country(user, code)?;
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(())
    }

    pub(crate) fn prepare_country(&self, user_id: u32, code: &str) -> Result<Self, String> {
        let code = code.trim().to_ascii_uppercase();
        if !code.is_empty() && !(code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())) {
            return Err("Use a two-letter country code, or automatic.".into())
        }
        let mut next = self.clone();
        let account = next.account_mut_where(|a| a.user_id == user_id).ok_or("No such account.")?;
        account.country = code;
        for p in &mut account.players { p.country.clone_from(&account.country); }
        Ok(next)
    }

    /// The account `user_id`, as kept now.
    pub fn account_of(&self, user_id: u32) -> Option<Account> {
        self.accounts.iter().map(|e| &*e.account).find(|a| a.user_id == user_id).cloned()
    }

    /// Makes a player named `name` in slot `index` (0-3, free) of account `user_id` (Select Player's 0x1F4A). Names
    /// are unique, ignoring case, printable and at most `MAX_NAME` characters; an account holds `MAX_PLAYERS`.
    pub fn create_player(&mut self, user_id: u32, index: u32, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control) || self.taken(name) {
            return Err(format!("the name {name:?} is empty, too long or taken"));
        }
        self.atomic(|d| {
            let id = d.next_player;
            let Some(a) = d.account_mut_where(|a| a.user_id == user_id) else { return Err(format!("no account {user_id}")) };
            if a.players.len() >= MAX_PLAYERS || index >= MAX_PLAYERS as u32 || a.players.iter().any(|p| p.index == index) {
                return Err(format!("slot {index} of account {user_id} is not free"));
            }
            let mut p = Player::new(id, name);
            p.index = index;
            p.country.clone_from(&a.country);
            a.players.push(p);
            d.next_player += 1;
            Ok(())
        })
    }

    /// Deletes a player of account `user_id` (Select Player's 0x1F4B), whose slot is then free. An account keeps at
    /// least one player.
    pub fn delete_player(&mut self, user_id: u32, player_id: u32) -> Result<(), String> {
        self.atomic(|d| {
            let Some(a) = d.account_mut_where(|a| a.user_id == user_id) else { return Err(format!("no account {user_id}")) };
            let Some(at) = a.players.iter().position(|p| p.player_id == player_id) else { return Err(format!("no player {player_id}")) };
            if a.players.len() == 1 {
                return Err("an account keeps at least one player".into());
            }
            a.players.remove(at);
            Ok(())
        })
    }

    /// Deletes account `user_id` (`delete-account`): its entry with its key, its User ID, password, recovery code and
    /// devices, and its players with their collections and decks. An invite it issued no longer names it. What it was
    /// known by (its User ID, or "#ID" for a key account) and its players.
    pub fn delete_account(&mut self, user_id: u32) -> Result<(String, Vec<(u32, String)>), String> {
        self.atomic(|d| {
            let at = d.accounts.iter().position(|e| e.account.user_id == user_id).ok_or_else(|| format!("no account {user_id}"))?;
            let entry = d.accounts.remove(at);
            let label = entry.login.as_ref().map_or_else(|| format!("#{user_id}"), |l| l.user.clone());
            if let Some(login) = &entry.login {
                for invite in d.invites.iter_mut().filter(|i| i.issued_by.eq_ignore_ascii_case(&login.user)) { invite.issued_by = "(a deleted account)".into() }
            }
            Ok((label, entry.account.players.iter().map(|p| (p.player_id, p.name.clone())).collect()))
        })
    }

    /// Renames a player (names are unique, ignoring case).
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        let id = self.player_id_named(old).ok_or_else(|| format!("no player named {old:?}"))?;
        let mut next = self.prepare_rename(id, new)?;
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(())
    }

    pub(crate) fn prepare_rename(&self, player_id: u32, new: &str) -> Result<Self, String> {
        let new = checked_name(new)?;
        if self.player_id_named(&new).is_some_and(|id| id != player_id) { return Err("That player name is already taken.".into()) }
        let mut next = self.clone();
        next.player_mut(player_id).ok_or("No such player.")?.name = new;
        Ok(next)
    }

    /// A player by id.
    pub fn player(&self, player_id: u32) -> Option<&Player> {
        self.accounts.iter().map(|e| &e.account).flat_map(|a| &a.players).find(|p| p.player_id == player_id)
    }

    /// Changes a player (their saved character settings) and saves the accounts: refused, and nothing changed, when
    /// there is no such player or the save fails.
    pub fn update_player(&mut self, player_id: u32, change: impl FnOnce(&mut Player)) -> Result<(), String> {
        self.atomic(|d| {
            let p = d.player_mut(player_id);
            change(p.ok_or_else(|| format!("no player {player_id}"))?);
            Ok(())
        })
    }

    /// Pays `bp` into the account of a player; its new BP, if the player has an account and the payment was saved.
    pub fn reward(&mut self, player_id: u32, bp: u32) -> Option<u32> {
        self.pay(player_id, bp).inspect_err(|why| tracing::warn!("duel reward of {bp} BP to player {player_id} not paid: {why}")).ok().flatten()
    }

    /// Pays `bp` into the account of a player: its new BP, None when no account has the player, or why it wasn't saved.
    pub fn pay(&mut self, player_id: u32, bp: u32) -> Result<Option<u32>, String> {
        if !self.accounts.iter().any(|e| e.account.players.iter().any(|p| p.player_id == player_id)) { return Ok(None) }
        self.atomic(|d| {
            let account = d.account_mut_where(|a| a.players.iter().any(|p| p.player_id == player_id)).ok_or("no such player")?;
            account.dpoint = account.dpoint.saturating_add(bp);
            Ok(Some(account.dpoint))
        })
    }

    /// Takes back in memory a payment that was saved inside a `store::together` that then didn't commit: the disk never
    /// had it. The next save writes the account as it is again.
    pub fn unpay(&mut self, player_id: u32, bp: u32) {
        if let Some(account) = self.account_mut_where(|a| a.players.iter().any(|p| p.player_id == player_id)) {
            account.dpoint = account.dpoint.saturating_sub(bp);
        }
    }

    /// Gives `bp` to the account at `who` (a User ID, or one of its players' names): who it went to, and the
    /// account's BP now.
    pub fn give_bp(&mut self, who: &str, bp: u32) -> Result<(String, u32), String> {
        if bp == 0 {
            return Err("give at least 1 BP".into());
        }
        let at = self.find(who).ok_or_else(|| format!("no account with the User ID or a player named {who:?}"))?;
        self.atomic(|d| {
            let account = Arc::make_mut(&mut d.accounts[at].account);
            account.dpoint = account.dpoint.saturating_add(bp);
            let names: Vec<_> = account.players.iter().map(|p| p.name.as_str()).collect();
            Ok((format!("user {} ({})", account.user_id, names.join(", ")), account.dpoint))
        })
    }

    /// A BP grant prepared on a copy. Nothing is written or published until its caller commits the whole operation.
    pub(crate) fn prepare_bp(&self, user_id: u32, bp: u32) -> Result<(Self, u32), String> {
        let mut next = self.clone();
        let account = next.accounts.iter_mut().find(|e| e.account.user_id == user_id).ok_or("the account is gone")?;
        let account = Arc::make_mut(&mut account.account);
        let total = account.dpoint.checked_add(bp).ok_or("BP balance limit exceeded")?;
        account.dpoint = total;
        Ok((next, total))
    }

    /// The host's BOOSTERPASS codes.
    pub fn codes(&self) -> &[BpCode] {
        &self.codes
    }

    /// Makes a BOOSTERPASS code worth `bp`, usable by `uses` accounts (any number when none), each once. It must fit the
    /// game's window ([`CODE_LEN`] letters and digits) and differ from the others, whatever the case.
    pub fn add_code(&mut self, code: &str, bp: u32, uses: Option<u32>) -> Result<(), String> {
        let mut next = self.prepare_add_code(code, bp, uses)?;
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(())
    }

    pub(crate) fn prepare_add_code(&self, code: &str, bp: u32, uses: Option<u32>) -> Result<Self, String> {
        if code.len() != CODE_LEN || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(format!("a code is {CODE_LEN} letters and digits (the game's window has {CODE_LEN} boxes): {code:?}"));
        }
        if bp == 0 || uses == Some(0) {
            return Err("a code gives at least 1 BP, to at least one account".into());
        }
        if self.code_at(code).is_some() {
            return Err(format!("the code {code} exists already"));
        }
        let code = BpCode { code: code.to_string(), bp, uses, used_by: vec![], claimed_at: vec![] };
        let mut next = self.clone();
        next.codes.push(code);
        Ok(next)
    }

    /// A new random BOOSTERPASS code that fits the game's window: capital letters and digits without look-alikes.
    pub fn new_code(&self) -> String {
        const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        loop {
            let mut bytes = [0u8; CODE_LEN];
            getrandom::fill(&mut bytes).expect("OS randomness unavailable");
            let code: String = bytes.iter().map(|b| CHARS[*b as usize % CHARS.len()] as char).collect();
            if self.code_at(&code).is_none() { return code }
        }
    }

    /// Removes a BOOSTERPASS code (any case).
    pub fn remove_code(&mut self, code: &str) -> Result<(), String> {
        let mut next = self.prepare_remove_code(code)?;
        next.persist().map_err(|e| format!("change was not saved: {e}"))?;
        *self = next;
        Ok(())
    }

    pub(crate) fn prepare_remove_code(&self, code: &str) -> Result<Self, String> {
        let at = self.code_at(code).ok_or_else(|| format!("no code {code}"))?;
        let mut next = self.clone();
        next.codes.remove(at);
        Ok(next)
    }

    /// The game's BOOSTERPASS window: account `user_id` enters `code` (any case). The BP got and the account's BP now;
    /// refused when no such code, the account used it already, or it is used up.
    pub fn redeem(&mut self, user_id: u32, code: &str) -> Result<(u32, u32), String> {
        let at = self.code_at(code).ok_or("no such code")?;
        let c = &self.codes[at];
        if c.used_by.contains(&user_id) {
            return Err("this account used the code already".into());
        }
        if c.uses.is_some_and(|n| c.used_by.len() as u32 >= n) {
            return Err("the code is used up".into());
        }
        let bp = c.bp;
        self.atomic(|d| {
            let account = d.account_mut_where(|a| a.user_id == user_id).ok_or("no such account")?;
            account.dpoint = account.dpoint.saturating_add(bp);
            let total = account.dpoint;
            d.codes[at].used_by.push(user_id);
            d.codes[at].claimed_at.push(Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()));
            Ok((bp, total))
        })
    }

    fn code_at(&self, code: &str) -> Option<usize> {
        self.codes.iter().position(|c| c.code.eq_ignore_ascii_case(code.trim()))
    }

    /// The player names of the account `user_id`, for showing who claimed a code.
    pub fn player_names(&self, user_id: u32) -> Vec<String> {
        self.accounts.iter().map(|e| &e.account).find(|a| a.user_id == user_id)
            .map(|a| a.players.iter().map(|p| p.name.clone()).collect()).unwrap_or_default()
    }

    /// An account's BP.
    pub fn dpoint(&self, user_id: u32) -> Option<u32> {
        self.accounts.iter().map(|e| &e.account).find(|a| a.user_id == user_id).map(|a| a.dpoint)
    }

    /// Takes `cost` BP from the account's stored BP (not a copy a session cached, which duel rewards may have outgrown)
    /// if it has them; its BP left.
    pub fn charge(&mut self, user_id: u32, cost: u32) -> Option<u32> {
        self.atomic(|d| {
            let account = d.account_mut_where(|a| a.user_id == user_id).ok_or("no such account")?;
            account.dpoint = account.dpoint.checked_sub(cost).ok_or("not enough BP")?;
            Ok(account.dpoint)
        }).ok()
    }

    /// A shop purchase: charges `cost` to the account holding `player_id` and applies `grant` to that player in one
    /// save, both or neither. Refused when the player is gone, already owns `item` (checked against the stored player,
    /// so two sessions of one account cannot buy it twice), the BP fall short, or the save fails. The BP left and the
    /// player as stored.
    pub fn purchase(&mut self, player_id: u32, cost: u32, item: Option<u16>, grant: impl FnOnce(&mut Player))
        -> Result<(u32, Player), String> {
        self.atomic(|d| {
            let account = d.account_mut_where(|a| a.players.iter().any(|p| p.player_id == player_id)).ok_or("no such player")?;
            account.dpoint = account.dpoint.checked_sub(cost).ok_or("not enough BP")?;
            let left = account.dpoint;
            let player = account.players.iter_mut().find(|p| p.player_id == player_id).ok_or("no such player")?;
            if item.is_some_and(|id| player.items.contains(&id)) {
                return Err("already owned".into());
            }
            grant(player);
            Ok((left, player.clone()))
        })
    }

    /// Changes several players in one step and one save (a trade hands cards both ways): all of them or, when one is
    /// not there or the save fails, none. Whether it was done.
    pub fn update_players(&mut self, changes: Vec<PlayerChange<'_>>) -> bool {
        if changes.iter().any(|(id, _)| self.player(*id).is_none()) {
            return false;
        }
        self.atomic(|d| {
            for (id, change) in changes {
                if let Some(p) = d.player_mut(id) {
                    change(p);
                }
            }
            Ok(())
        }).inspect_err(|why| tracing::warn!("players not changed: {why}")).is_ok()
    }

    /// Changes an account (its BP) and saves the accounts: refused, and nothing changed, when there is no such account
    /// or the save fails.
    pub fn update_account(&mut self, user_id: u32, change: impl FnOnce(&mut Account)) -> Result<(), String> {
        self.atomic(|d| {
            change(d.account_mut_where(|a| a.user_id == user_id).ok_or_else(|| format!("no account {user_id}"))?);
            Ok(())
        })
    }

    /// Tests only: every account with its user id, by user id.
    #[doc(hidden)]
    pub fn accounts(&self) -> impl Iterator<Item = (u32, &Account)> {
        self.entries().map(|(_, _, a)| (a.user_id, a))
    }

    /// Every account with its User ID (a password account) and key (a key account), by user id.
    pub fn entries(&self) -> impl Iterator<Item = (Option<&str>, Option<&str>, &Account)> {
        let mut all: Vec<_> = self.accounts.iter().map(|e| (e.login.as_ref().map(|l| l.user.as_str()), e.key.as_deref(), &*e.account)).collect();
        all.sort_by_key(|(_, _, a)| a.user_id);
        all.into_iter()
    }

    fn save(&mut self) {
        if let Err(e) = self.persist() { tracing::warn!("accounts not saved: {e}"); }
    }

    /// One account as saved: its players with their collections and decks, and how the account is found.
    fn account_json(e: &Entry) -> Json {
        let a = &e.account;
        let players: Vec<Json> = a.players.iter().map(|p| {
            let avatar: String = p.avatar.iter().map(|b| format!("{b:02x}")).collect();
            let cards: Vec<[u32; 2]> = p.cards.iter().map(|(&row, &n)| [row as u32, n]).collect();
            let given: Vec<[u32; 2]> = p.given.iter().map(|(&row, &n)| [row as u32, n]).collect();
            let uploaded: Vec<Json> = p.uploaded.iter().map(|d| json!({ "id": d.id, "name": d.name, "main": d.main, "extra": d.extra, "side": d.side })).collect();
            json!({ "player_id": p.player_id, "uploaded": uploaded, "name": p.name, "index": p.index, "level": p.level, "avatar": avatar,
                "speeches": p.speeches, "comments": p.comments, "title": p.title, "cards": cards, "decks": p.decks,
                "items": p.items, "given": given, "eval_num": p.eval_num, "eval_total": p.eval_total, "counters": p.counters })
        }).collect();
        let mut account = json!({ "user_id": a.user_id, "dpoint": a.dpoint, "players": players });
        if let Some(key) = &e.key { account["key"] = json!(key); }
        if let Some(login) = &e.login { account["login"] = serde_json::to_value(login).unwrap_or_default(); }
        if !a.country.is_empty() { account["country"] = json!(a.country); }
        account
    }

    /// Saves what changed since the last save. In server.db only the accounts whose `Arc` or fields changed are
    /// serialized and written (#22); outside it (tests, tools) the whole file.
    pub(crate) fn persist(&mut self) -> io::Result<()> {
        let Some(file) = self.file.clone() else { return Ok(()) };
        let codes: Vec<Json> = self.codes.iter().map(|c| json!({ "code": c.code, "bp": c.bp, "uses": c.uses, "used_by": c.used_by, "claimed_at": c.claimed_at })).collect();
        let mut whole = json!({ "next_user": self.next_user, "next_player": self.next_player, "bp_codes": codes, "invites": self.invites });
        if store::in_database(&file) {
            let changed: Vec<(String, Json)> = self.accounts.iter()
                .filter(|e| !self.saved.get(&e.account.user_id).is_some_and(|s| s.same(e)))
                .map(|e| (e.account.user_id.to_string(), Self::account_json(e))).collect();
            let present: std::collections::HashSet<u32> = self.accounts.iter().map(|e| e.account.user_id).collect();
            let gone: Vec<String> = self.saved.keys().filter(|id| !present.contains(id)).map(u32::to_string).collect();
            store::write_changes(&file, &whole, "accounts", &changed, &gone)?;
        } else {
            let mut all: Vec<&Entry> = self.accounts.iter().collect();
            all.sort_by_key(|e| e.account.user_id);
            whole["accounts"] = Json::Array(all.into_iter().map(Self::account_json).collect());
            store::write(&file, &whole)?;
        }
        self.saved = self.accounts.iter().map(|e| (e.account.user_id, Saved::of(e))).collect();
        Ok(())
    }
}

/// The first slot of the Select Player screen no player has.
fn checked_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control) {
        return Err(format!("Use 1 to {MAX_NAME} printable characters for a player name."))
    }
    Ok(name.into())
}

fn free_slot(players: &[Player]) -> Option<u32> {
    (0..MAX_PLAYERS as u32).find(|i| players.iter().all(|p| p.index != *i))
}

/// Bytes from their hex digits (the saved avatar).
fn from_hex(s: &str) -> Option<Vec<u8>> {
    (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok())).collect()
}
