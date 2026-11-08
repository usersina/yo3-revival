//! Accounts by key or by source address.
//!
//! The client never sends a user id or password: 0x1F47 is a bare request, and the
//! credentials typed at login stay on the client's connection object. What it does send, before every
//! connection to the gate, is a GET of `PatchUpdateUrl` + `yo3.idx` from its `yo3.ini`: an account's secret key
//! rides in that URL (`/yo3/<key>/yo3.idx`), the patch server notes which address presented it (`present_key`)
//! and the next login from that address gets the key's account (`login`), wherever the friend connects from. A
//! client with no (or an unknown) key is told apart by the address it connects from (one account per address).
//! Each account offers up to four players to pick from at Select Player. With a file, accounts
//! survive restarts and the host can name a friend's player ahead (`add_player`) or rename one.
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value as Json, json};

use crate::front::{Account, COMMENTS, Deck, Player, SPEECHES};
use crate::store;

/// The client shows four player slots.
pub const MAX_PLAYERS: usize = 4;
/// The longest player name the server takes, in characters (**guess**: the client's name field has its own limit).
pub const MAX_NAME: usize = 16;
/// The BOOSTERPASS Points (the shop's BP, the account's dpoint) a new account starts with: 50 packs (**guess**).
pub const START_BP: u32 = 5000;

/// How many accounts one address may make on the join page.
pub const JOIN_PER_ADDRESS: usize = 4;
/// How many accounts the join page makes per hour in all (**guess**: a stop to abuse, a group of friends is far below).
pub const JOIN_PER_HOUR: usize = 20;

/// What the join page shows an address: the accounts it has (key and player names) and whether an address-only
/// account could be given a key.
pub struct JoinState {
    pub accounts: Vec<(String, Vec<String>)>,
    pub keepable: bool,
    /// Whether the address may still make an account (`JOIN_PER_ADDRESS`).
    pub can_join: bool,
}

/// A change to one player (`Directory::update_players`).
pub type PlayerChange<'a> = (u32, Box<dyn FnOnce(&mut Player) + 'a>);

/// How long a key an address presented counts for its next login (a client fetches it just before connecting; a
/// reconnect through the same address refreshes it).
pub const KEY_WINDOW: Duration = Duration::from_secs(600);
/// The characters of a key (URL-safe).
const KEY_CHARS: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// The characters in a key.
pub const KEY_LEN: usize = 20;

/// An account with how a client finds it: by the address it connects from, by its key, or both.
struct Entry {
    address: Option<IpAddr>,
    key: Option<String>,
    /// The address that made it on the join page (for the page's per-address cap).
    joined_from: Option<IpAddr>,
    account: Account,
}

/// A random key: 20 characters of a 57-letter alphabet (about 117 bits), from the OS-seeded hasher so it needs no
/// extra crate and works on every host.
fn new_key() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    let mut bits = 0u128;
    for i in 0..2u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(nanos);
        h.write_u64(i);
        bits = bits << 64 | h.finish() as u128;
    }
    (0..KEY_LEN)
        .map(|_| {
            let c = KEY_CHARS[(bits % KEY_CHARS.len() as u128) as usize] as char;
            bits /= KEY_CHARS.len() as u128;
            c
        })
        .collect()
}

pub struct Directory {
    player_names: Vec<String>,
    accounts: Vec<Entry>,
    /// The key each address presented last, and when (not saved: a client presents it again at every start).
    presented: HashMap<IpAddr, (String, Instant)>,
    /// When the join page made its accounts, for the hourly cap (not saved).
    joins: Vec<Instant>,
    next_user: u32,
    next_player: u32,
    /// Where the accounts are kept (none: in memory only).
    file: Option<PathBuf>,
}

impl Directory {
    /// `player_names`: the players every new account starts with (at most four are used).
    pub fn new(player_names: Vec<String>) -> Directory {
        Directory { player_names, accounts: Vec::new(), presented: HashMap::new(), joins: Vec::new(), next_user: 1001, next_player: 1001, file: None }
    }

    /// The accounts kept in `file` (none yet if it doesn't exist); changes are written back to it.
    pub fn open(player_names: Vec<String>, file: &Path) -> io::Result<Directory> {
        let mut d = Directory { file: Some(file.to_path_buf()), ..Directory::new(player_names) };
        let Some(saved) = store::read(file)? else { return Ok(d) };
        // The counters are kept: ids are never handed out twice, even after the newest player was deleted (the
        // friends, mail and rentals of the old one would else belong to the new).
        d.next_player = d.next_player.max(saved["next_player"].as_u64().unwrap_or(0) as u32);
        d.next_user = d.next_user.max(saved["next_user"].as_u64().unwrap_or(0) as u32);
        for a in saved["accounts"].as_array().into_iter().flatten() {
            let bad = || io::Error::new(io::ErrorKind::InvalidData, format!("{}: bad account {a}", file.display()));
            // Old files have an address and no key; a key account may have no address.
            let address: Option<IpAddr> = a["address"].as_str().map(|s| s.parse().map_err(|_| bad())).transpose()?;
            let key = a["key"].as_str().filter(|k| !k.is_empty()).map(str::to_string);
            if address.is_none() && key.is_none() {
                return Err(bad());
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
            let joined_from = a["joined_from"].as_str().and_then(|s| s.parse().ok());
            d.accounts.push(Entry { address, key, joined_from, account });
        }
        Ok(d)
    }

    fn account(user_id: u32, players: Vec<Player>) -> Account {
        Account { user_id, name: format!("user{user_id}"), dpoint: START_BP, players, country: String::new() }
    }

    /// The account of an address, made with the starting players (their names made unique) on first contact.
    pub fn account_for(&mut self, addr: IpAddr) -> Account {
        if let Some(e) = self.accounts.iter().find(|e| e.address == Some(addr)) {
            return e.account.clone();
        }
        let names: Vec<String> = self.player_names.iter().take(MAX_PLAYERS).cloned().collect();
        let names: Vec<String> = names.iter().map(|n| self.free_name(n)).collect();
        let account = self.create(addr, &names);
        self.save();
        account
    }

    fn create(&mut self, addr: IpAddr, names: &[String]) -> Account {
        let account = self.create_account(names);
        self.accounts.push(Entry { address: Some(addr), key: None, joined_from: None, account: account.clone() });
        account
    }

    /// Notes that `addr` presented `key` (the patch server saw `/yo3/<key>/yo3.idx`) at `now`; whether the key is
    /// an account's. An unknown key counts as none: it forgets what the address presented before, so its login
    /// goes to its address account (the most recent fetch decides, and a client without a key is the most recent).
    pub fn present_key(&mut self, addr: IpAddr, key: &str, now: Instant) -> bool {
        let known = self.accounts.iter().any(|e| e.key.as_deref() == Some(key));
        if known {
            self.presented.insert(addr, (key.to_string(), now));
        } else {
            self.presented.remove(&addr);
        }
        known
    }

    /// A fetch of the patch index from `addr` with no key in the URL: see `present_key`.
    pub fn present_no_key(&mut self, addr: IpAddr) {
        self.presented.remove(&addr);
    }

    /// The account a client connecting from `addr` logs in to at `now`: that of the key the address presented most
    /// recently within `KEY_WINDOW` (and the window starts over, so a reconnect later in a long session finds it),
    /// else the address account.
    pub fn login(&mut self, addr: IpAddr, now: Instant) -> Account {
        if let Some((key, at)) = self.presented.get(&addr).cloned() {
            if now.saturating_duration_since(at) <= KEY_WINDOW
                && let Some(e) = self.accounts.iter().find(|e| e.key.as_deref() == Some(key.as_str()))
            {
                let account = e.account.clone();
                self.presented.insert(addr, (key, now));
                return account;
            }
            self.presented.remove(&addr);
        }
        self.account_for(addr)
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
        let account = self.create_account(&names);
        let key = new_key();
        self.accounts.push(Entry { address: None, key: Some(key.clone()), joined_from: None, account });
        self.save();
        Ok(key)
    }

    /// Whether `key` is an account's.
    pub fn has_key(&self, key: &str) -> bool {
        !key.is_empty() && self.accounts.iter().any(|e| e.key.as_deref() == Some(key))
    }

    /// The join page's view of `addr`: the accounts with a key it made or has, and whether an address-only account
    /// could be given a key.
    pub fn join_state(&self, addr: IpAddr) -> JoinState {
        let mine = |e: &&Entry| e.key.is_some() && (e.joined_from == Some(addr) || e.address == Some(addr));
        let accounts = self.accounts.iter().filter(mine).filter_map(|e| Some((e.key.clone()?, e.account.players.iter().map(|p| p.name.clone()).collect()))).collect();
        let keepable = self.accounts.iter().any(|e| e.address == Some(addr) && e.key.is_none());
        let can_join = self.accounts.iter().filter(|e| e.joined_from == Some(addr)).count() < JOIN_PER_ADDRESS;
        JoinState { accounts, keepable, can_join }
    }

    /// The join page makes an account with one player `name` for `addr` at `now`; its key. Refused for a bad or
    /// taken name (the rules of Create Player), more than `JOIN_PER_ADDRESS` accounts for the address or
    /// `JOIN_PER_HOUR` in the last hour.
    pub fn join(&mut self, addr: IpAddr, name: &str, now: Instant) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control) {
            return Err(format!("a name is 1 to {MAX_NAME} characters, with no control characters"));
        }
        if self.taken(name) {
            return Err(format!("the name {name:?} is taken, pick another"));
        }
        if self.accounts.iter().filter(|e| e.joined_from == Some(addr)).count() >= JOIN_PER_ADDRESS {
            return Err(format!("this address already made {JOIN_PER_ADDRESS} accounts"));
        }
        self.joins.retain(|t| now.saturating_duration_since(*t) < Duration::from_secs(3600));
        if self.joins.len() >= JOIN_PER_HOUR {
            return Err("too many accounts were made in the last hour, try again later".to_string());
        }
        let key = self.add_account(&[name.to_string()])?;
        if let Some(e) = self.accounts.last_mut() {
            e.joined_from = Some(addr);
        }
        self.joins.push(now);
        self.save();
        Ok(key)
    }

    /// The key of the address-only account at `addr` ("keep my account"), made if it has none.
    pub fn keep(&mut self, addr: IpAddr) -> Result<String, String> {
        self.key_of(&addr.to_string())
    }

    /// The key of the account at `who` (an address, or a player's name), made if the account has none.
    pub fn key_of(&mut self, who: &str) -> Result<String, String> {
        let at = self.find(who).ok_or_else(|| format!("no account at or with a player named {who:?}"))?;
        if let Some(key) = &self.accounts[at].key {
            return Ok(key.clone());
        }
        let key = new_key();
        self.accounts[at].key = Some(key.clone());
        self.save();
        Ok(key)
    }

    /// The account an admin argument names: an address, or a player's name.
    fn find(&self, who: &str) -> Option<usize> {
        match who.parse::<IpAddr>() {
            Ok(addr) => self.accounts.iter().position(|e| e.address == Some(addr)),
            Err(_) => self.accounts.iter().position(|e| e.account.players.iter().any(|p| p.name.eq_ignore_ascii_case(who))),
        }
    }

    /// The user id of the account at `who` (an address, or a player's name).
    pub fn user_of(&self, who: &str) -> Option<u32> {
        self.find(who).map(|i| self.accounts[i].account.user_id)
    }

    /// Gives account `user_id` a player named `name`.
    pub fn add_player_to(&mut self, user_id: u32, name: &str) -> Result<(), String> {
        if name.is_empty() || self.taken(name) {
            return Err(format!("the name {name:?} is taken or empty"));
        }
        let Some(a) = self.accounts.iter_mut().map(|e| &mut e.account).find(|a| a.user_id == user_id) else { return Err(format!("no account {user_id}")) };
        if a.players.len() >= MAX_PLAYERS {
            return Err(format!("account {user_id} already has {MAX_PLAYERS} players"));
        }
        let mut p = Player::new(self.next_player, name);
        p.index = free_slot(&a.players).unwrap_or_default();
        p.country.clone_from(&a.country);
        self.next_player += 1;
        a.players.push(p);
        self.save();
        Ok(())
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

    /// Gives the account at `addr` a player named `name` (making the account, with only that player, if new).
    pub fn add_player(&mut self, addr: IpAddr, name: &str) -> Result<(), String> {
        if name.is_empty() || self.taken(name) {
            return Err(format!("the name {name:?} is taken or empty"));
        }
        match self.accounts.iter().position(|e| e.address == Some(addr)) {
            None => {
                self.create(addr, &[name.to_string()]);
                self.save();
                Ok(())
            }
            Some(at) => self.add_player_to(self.accounts[at].account.user_id, name),
        }
    }

    /// Moves the account at `from` to `to`, for a friend whose address changed (everything else keys players and
    /// accounts by id). An account already at `to` (made when the friend first connected from there) is discarded
    /// only with `replace`.
    pub fn move_account(&mut self, from: IpAddr, to: IpAddr, replace: bool) -> Result<(), String> {
        if from == to {
            return Err(format!("{from} is already the account's address"));
        }
        let Some(from_at) = self.accounts.iter().position(|e| e.address == Some(from)) else {
            return Err(format!("no account at {from}"));
        };
        let there = self.accounts.iter().position(|e| e.address == Some(to));
        if let Some(there) = there
            && !replace
        {
            let names: Vec<&str> = self.accounts[there].account.players.iter().map(|p| p.name.as_str()).collect();
            return Err(format!("{to} already has an account ({}); pass --replace to discard it", names.join(", ")));
        }
        self.accounts[from_at].address = Some(to);
        if let Some(there) = there {
            // The discarded account goes whole, key included (a key it had stops working).
            self.accounts.remove(there);
        }
        self.save();
        Ok(())
    }

    /// Sets the country flag of the account at `addr` (an ISO code of the client's `countrytbl.bin`, two letters in
    /// any case); an empty code clears it, so the flag follows the client's language again.
    pub fn set_country(&mut self, addr: IpAddr, code: &str) -> Result<(), String> {
        let code = code.trim().to_ascii_uppercase();
        if !code.is_empty() && !(code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())) {
            return Err(format!("{code:?} is not a two-letter country code"));
        }
        let at = self.accounts.iter().position(|e| e.address == Some(addr));
        if at.is_none() {
            return Err(format!("no account at {addr}"));
        }
        self.set_country_at(at, &code)
    }

    fn set_country_at(&mut self, at: Option<usize>, code: &str) -> Result<(), String> {
        let code = code.trim().to_ascii_uppercase();
        if !code.is_empty() && !(code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())) {
            return Err(format!("{code:?} is not a two-letter country code"));
        }
        let Some(at) = at else { return Err("no such account".into()) };
        let account = &mut self.accounts[at].account;
        account.country = code;
        for p in &mut account.players {
            p.country.clone_from(&account.country);
        }
        self.save();
        Ok(())
    }

    /// The account `user_id`, as kept now.
    pub fn account_of(&self, user_id: u32) -> Option<Account> {
        self.accounts.iter().map(|e| &e.account).find(|a| a.user_id == user_id).cloned()
    }

    /// Makes a player named `name` in slot `index` (0-3, free) of account `user_id` (Select Player's 0x1F4A). Names
    /// are unique, ignoring case, printable and at most `MAX_NAME` characters; an account holds `MAX_PLAYERS`.
    pub fn create_player(&mut self, user_id: u32, index: u32, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control) || self.taken(name) {
            return Err(format!("the name {name:?} is empty, too long or taken"));
        }
        let id = self.next_player;
        let Some(a) = self.accounts.iter_mut().map(|e| &mut e.account).find(|a| a.user_id == user_id) else { return Err(format!("no account {user_id}")) };
        if a.players.len() >= MAX_PLAYERS || index >= MAX_PLAYERS as u32 || a.players.iter().any(|p| p.index == index) {
            return Err(format!("slot {index} of account {user_id} is not free"));
        }
        let mut p = Player::new(id, name);
        p.index = index;
        p.country.clone_from(&a.country);
        a.players.push(p);
        self.next_player += 1;
        self.save();
        Ok(())
    }

    /// Deletes a player of account `user_id` (Select Player's 0x1F4B), whose slot is then free. An account keeps at
    /// least one player.
    pub fn delete_player(&mut self, user_id: u32, player_id: u32) -> Result<(), String> {
        let Some(a) = self.accounts.iter_mut().map(|e| &mut e.account).find(|a| a.user_id == user_id) else { return Err(format!("no account {user_id}")) };
        let Some(at) = a.players.iter().position(|p| p.player_id == player_id) else { return Err(format!("no player {player_id}")) };
        if a.players.len() == 1 {
            return Err("an account keeps at least one player".into());
        }
        a.players.remove(at);
        self.save();
        Ok(())
    }

    /// Renames a player (names are unique, ignoring case).
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        if new.is_empty() || (self.taken(new) && !old.eq_ignore_ascii_case(new)) {
            return Err(format!("the name {new:?} is taken or empty"));
        }
        let player = self.accounts.iter_mut().map(|e| &mut e.account).flat_map(|a| &mut a.players).find(|p| p.name.eq_ignore_ascii_case(old));
        let Some(player) = player else { return Err(format!("no player named {old:?}")) };
        player.name = new.to_string();
        self.save();
        Ok(())
    }

    /// A player by id.
    pub fn player(&self, player_id: u32) -> Option<&Player> {
        self.accounts.iter().map(|e| &e.account).flat_map(|a| &a.players).find(|p| p.player_id == player_id)
    }

    /// Changes a player (their saved character settings) and saves the accounts.
    pub fn update_player(&mut self, player_id: u32, change: impl FnOnce(&mut Player)) {
        if let Some(p) = self.accounts.iter_mut().map(|e| &mut e.account).flat_map(|a| &mut a.players).find(|p| p.player_id == player_id) {
            change(p);
            self.save();
        }
    }

    /// Pays `bp` into the account of a player; its new BP, if the player has an account.
    pub fn reward(&mut self, player_id: u32, bp: u32) -> Option<u32> {
        let account = self.accounts.iter_mut().map(|e| &mut e.account).find(|a| a.players.iter().any(|p| p.player_id == player_id))?;
        account.dpoint = account.dpoint.saturating_add(bp);
        let total = account.dpoint;
        self.save();
        Some(total)
    }

    /// An account's BP.
    pub fn dpoint(&self, user_id: u32) -> Option<u32> {
        self.accounts.iter().map(|e| &e.account).find(|a| a.user_id == user_id).map(|a| a.dpoint)
    }

    /// Takes `cost` BP from the account's stored BP (not a copy a session cached, which duel rewards may have outgrown)
    /// if it has them; its BP left.
    pub fn charge(&mut self, user_id: u32, cost: u32) -> Option<u32> {
        let account = self.accounts.iter_mut().map(|e| &mut e.account).find(|a| a.user_id == user_id)?;
        account.dpoint = account.dpoint.checked_sub(cost)?;
        let left = account.dpoint;
        self.save();
        Some(left)
    }

    /// Changes several players in one step and one save (a trade hands cards both ways): all of them or, when one is
    /// not there, none. Whether it was done.
    pub fn update_players(&mut self, changes: Vec<PlayerChange<'_>>) -> bool {
        if changes.iter().any(|(id, _)| self.player(*id).is_none()) {
            return false;
        }
        for (id, change) in changes {
            if let Some(p) = self.accounts.iter_mut().map(|e| &mut e.account).flat_map(|a| &mut a.players).find(|p| p.player_id == id) {
                change(p);
            }
        }
        self.save();
        true
    }

    /// Changes an account (its BP) and saves the accounts.
    pub fn update_account(&mut self, user_id: u32, change: impl FnOnce(&mut Account)) {
        if let Some(a) = self.accounts.iter_mut().map(|e| &mut e.account).find(|a| a.user_id == user_id) {
            change(a);
            self.save();
        }
    }

    /// Every account found by an address, with it, by address.
    pub fn accounts(&self) -> impl Iterator<Item = (IpAddr, &Account)> {
        let mut all: Vec<_> = self.accounts.iter().filter_map(|e| Some((e.address?, &e.account))).collect();
        all.sort_by_key(|(addr, _)| *addr);
        all.into_iter()
    }

    /// Every account with its address and key, by user id.
    pub fn entries(&self) -> impl Iterator<Item = (Option<IpAddr>, Option<&str>, &Account)> {
        let mut all: Vec<_> = self.accounts.iter().map(|e| (e.address, e.key.as_deref(), &e.account)).collect();
        all.sort_by_key(|(_, _, a)| a.user_id);
        all.into_iter()
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let accounts: Vec<Json> = self
            .entries()
            .map(|(addr, key, a)| {
                let players: Vec<Json> = a
                    .players
                    .iter()
                    .map(|p| {
                        let avatar: String = p.avatar.iter().map(|b| format!("{b:02x}")).collect();
                        let cards: Vec<[u32; 2]> = p.cards.iter().map(|(&row, &n)| [row as u32, n]).collect();
                        let given: Vec<[u32; 2]> = p.given.iter().map(|(&row, &n)| [row as u32, n]).collect();
                        let uploaded: Vec<Json> = p
                            .uploaded
                            .iter()
                            .map(|d| json!({ "id": d.id, "name": d.name, "main": d.main, "extra": d.extra, "side": d.side }))
                            .collect();
                        json!({ "player_id": p.player_id, "uploaded": uploaded, "name": p.name, "index": p.index, "level": p.level, "avatar": avatar,
                            "speeches": p.speeches, "comments": p.comments, "title": p.title, "cards": cards, "decks": p.decks,
                            "items": p.items, "given": given, "eval_num": p.eval_num, "eval_total": p.eval_total })
                    })
                    .collect();
                let mut account = json!({ "user_id": a.user_id, "dpoint": a.dpoint, "players": players });
                if let Some(addr) = addr {
                    account["address"] = json!(addr.to_string());
                }
                if let Some(key) = key {
                    account["key"] = json!(key);
                }
                if let Some(from) = self.accounts.iter().find(|e| e.account.user_id == a.user_id).and_then(|e| e.joined_from) {
                    account["joined_from"] = json!(from.to_string());
                }
                if !a.country.is_empty() {
                    account["country"] = json!(a.country);
                }
                account
            })
            .collect();
        if let Err(e) = store::write(file, &json!({ "next_user": self.next_user, "next_player": self.next_player, "accounts": accounts })) {
            tracing::warn!("accounts not saved to {}: {e}", file.display());
        }
    }
}

/// The first slot of the Select Player screen no player has.
fn free_slot(players: &[Player]) -> Option<u32> {
    (0..MAX_PLAYERS as u32).find(|i| players.iter().all(|p| p.index != *i))
}

/// Bytes from their hex digits (the saved avatar).
fn from_hex(s: &str) -> Option<Vec<u8>> {
    (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok())).collect()
}
