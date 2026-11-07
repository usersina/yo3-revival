//! Accounts by source address.
//!
//! The client never sends a user id or password: 0x1F47 is a bare request, and the
//! credentials typed at login stay on the client's connection object. So the server tells
//! accounts apart by the address they connect from (one per friend on a LAN or VPN), and
//! each account offers up to four players to pick from at Select Player. With a file, accounts
//! survive restarts and the host can name a friend's player ahead (`add_player`) or rename one.
use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};

use crate::front::{Account, COMMENTS, Player, SPEECHES};
use crate::store;

/// The client shows four player slots.
pub const MAX_PLAYERS: usize = 4;
/// The BOOSTERPASS Points (the shop's BP, the account's dpoint) a new account starts with: 50 packs (**guess**).
pub const START_BP: u32 = 5000;

pub struct Directory {
    player_names: Vec<String>,
    by_address: HashMap<IpAddr, Account>,
    next_user: u32,
    next_player: u32,
    /// Where the accounts are kept (none: in memory only).
    file: Option<PathBuf>,
}

impl Directory {
    /// `player_names`: the players every new account starts with (at most four are used).
    pub fn new(player_names: Vec<String>) -> Directory {
        Directory { player_names, by_address: HashMap::new(), next_user: 1001, next_player: 1001, file: None }
    }

    /// The accounts kept in `file` (none yet if it doesn't exist); changes are written back to it.
    pub fn open(player_names: Vec<String>, file: &Path) -> io::Result<Directory> {
        let mut d = Directory { file: Some(file.to_path_buf()), ..Directory::new(player_names) };
        let Some(saved) = store::read(file)? else { return Ok(d) };
        for a in saved["accounts"].as_array().into_iter().flatten() {
            let bad = || io::Error::new(io::ErrorKind::InvalidData, format!("{}: bad account {a}", file.display()));
            let address: IpAddr = a["address"].as_str().and_then(|s| s.parse().ok()).ok_or_else(bad)?;
            let user_id = a["user_id"].as_u64().ok_or_else(bad)? as u32;
            let mut players = vec![];
            for (slot, p) in a["players"].as_array().into_iter().flatten().take(MAX_PLAYERS).enumerate() {
                let mut player = Player::new(p["player_id"].as_u64().ok_or_else(bad)? as u32, p["name"].as_str().ok_or_else(bad)?);
                player.index = slot as u32;
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
                d.next_player = d.next_player.max(player.player_id + 1);
                players.push(player);
            }
            d.next_user = d.next_user.max(user_id + 1);
            let mut account = Directory::account(user_id, players);
            account.dpoint = a["dpoint"].as_u64().map_or(START_BP, |n| n as u32);
            d.by_address.insert(address, account);
        }
        Ok(d)
    }

    fn account(user_id: u32, players: Vec<Player>) -> Account {
        Account { user_id, name: format!("user{user_id}"), dpoint: START_BP, players }
    }

    /// The account of an address, made with the starting players (their names made unique) on first contact.
    pub fn account_for(&mut self, addr: IpAddr) -> Account {
        if let Some(a) = self.by_address.get(&addr) {
            return a.clone();
        }
        let names: Vec<String> = self.player_names.iter().take(MAX_PLAYERS).cloned().collect();
        let names: Vec<String> = names.iter().map(|n| self.free_name(n)).collect();
        let account = self.create(addr, &names);
        self.save();
        account
    }

    fn create(&mut self, addr: IpAddr, names: &[String]) -> Account {
        let user_id = self.next_user;
        self.next_user += 1;
        let mut players = Vec::new();
        for (slot, name) in names.iter().enumerate() {
            let mut p = Player::new(self.next_player, name);
            p.index = slot as u32;
            self.next_player += 1;
            players.push(p);
        }
        let account = Directory::account(user_id, players);
        self.by_address.insert(addr, account.clone());
        account
    }

    fn taken(&self, name: &str) -> bool {
        self.by_address.values().flat_map(|a| &a.players).any(|p| p.name.eq_ignore_ascii_case(name))
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
        match self.by_address.get_mut(&addr) {
            None => {
                self.create(addr, &[name.to_string()]);
            }
            Some(a) if a.players.len() >= MAX_PLAYERS => return Err(format!("{addr} already has {MAX_PLAYERS} players")),
            Some(a) => {
                let mut p = Player::new(self.next_player, name);
                p.index = a.players.len() as u32;
                self.next_player += 1;
                a.players.push(p);
            }
        }
        self.save();
        Ok(())
    }

    /// Renames a player (names are unique, ignoring case).
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        if new.is_empty() || (self.taken(new) && !old.eq_ignore_ascii_case(new)) {
            return Err(format!("the name {new:?} is taken or empty"));
        }
        let player = self.by_address.values_mut().flat_map(|a| &mut a.players).find(|p| p.name.eq_ignore_ascii_case(old));
        let Some(player) = player else { return Err(format!("no player named {old:?}")) };
        player.name = new.to_string();
        self.save();
        Ok(())
    }

    /// Changes a player (their saved character settings) and saves the accounts.
    pub fn update_player(&mut self, player_id: u32, change: impl FnOnce(&mut Player)) {
        if let Some(p) = self.by_address.values_mut().flat_map(|a| &mut a.players).find(|p| p.player_id == player_id) {
            change(p);
            self.save();
        }
    }

    /// Pays `bp` into the account of a player; its new BP, if the player has an account.
    pub fn reward(&mut self, player_id: u32, bp: u32) -> Option<u32> {
        let account = self.by_address.values_mut().find(|a| a.players.iter().any(|p| p.player_id == player_id))?;
        account.dpoint = account.dpoint.saturating_add(bp);
        let total = account.dpoint;
        self.save();
        Some(total)
    }

    /// An account's BP.
    pub fn dpoint(&self, user_id: u32) -> Option<u32> {
        self.by_address.values().find(|a| a.user_id == user_id).map(|a| a.dpoint)
    }

    /// Changes an account (its BP) and saves the accounts.
    pub fn update_account(&mut self, user_id: u32, change: impl FnOnce(&mut Account)) {
        if let Some(a) = self.by_address.values_mut().find(|a| a.user_id == user_id) {
            change(a);
            self.save();
        }
    }

    /// Every account with its address, by address.
    pub fn accounts(&self) -> impl Iterator<Item = (IpAddr, &Account)> {
        let mut all: Vec<_> = self.by_address.iter().map(|(addr, a)| (*addr, a)).collect();
        all.sort_by_key(|(addr, _)| *addr);
        all.into_iter()
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let accounts: Vec<Json> = self
            .accounts()
            .map(|(addr, a)| {
                let players: Vec<Json> = a
                    .players
                    .iter()
                    .map(|p| {
                        let avatar: String = p.avatar.iter().map(|b| format!("{b:02x}")).collect();
                        let cards: Vec<[u32; 2]> = p.cards.iter().map(|(&row, &n)| [row as u32, n]).collect();
                        let given: Vec<[u32; 2]> = p.given.iter().map(|(&row, &n)| [row as u32, n]).collect();
                        json!({ "player_id": p.player_id, "name": p.name, "level": p.level, "avatar": avatar,
                            "speeches": p.speeches, "comments": p.comments, "title": p.title, "cards": cards, "decks": p.decks,
                            "items": p.items, "given": given })
                    })
                    .collect();
                json!({ "address": addr.to_string(), "user_id": a.user_id, "dpoint": a.dpoint, "players": players })
            })
            .collect();
        if let Err(e) = store::write(file, &json!({ "accounts": accounts })) {
            tracing::warn!("accounts not saved to {}: {e}", file.display());
        }
    }
}

/// Bytes from their hex digits (the saved avatar).
fn from_hex(s: &str) -> Option<Vec<u8>> {
    (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok())).collect()
}
