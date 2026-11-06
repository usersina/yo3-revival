//! Accounts by source address.
//!
//! The client never sends a user id or password: 0x1F47 is a bare request, and the
//! credentials typed at login stay on the client's connection object. So the server tells
//! accounts apart by the address they connect from (one per friend on a LAN or VPN), and
//! each account offers up to four players to pick from at Select Player.
use std::collections::HashMap;
use std::net::IpAddr;

use crate::front::{Account, Player};

/// The client shows four player slots.
pub const MAX_PLAYERS: usize = 4;

pub struct Directory {
    player_names: Vec<String>,
    by_address: HashMap<IpAddr, Account>,
    next_user: u32,
    next_player: u32,
}

impl Directory {
    /// `player_names`: the players every new account starts with (at most four are used).
    pub fn new(player_names: Vec<String>) -> Directory {
        Directory { player_names, by_address: HashMap::new(), next_user: 1001, next_player: 1001 }
    }

    pub fn account_for(&mut self, addr: IpAddr) -> Account {
        if let Some(a) = self.by_address.get(&addr) {
            return a.clone();
        }
        let user_id = self.next_user;
        self.next_user += 1;
        let mut players = Vec::new();
        for (slot, name) in self.player_names.iter().take(MAX_PLAYERS).enumerate() {
            let mut p = Player::new(self.next_player, name);
            p.index = slot as u32;
            self.next_player += 1;
            players.push(p);
        }
        let account = Account { user_id, name: format!("user{user_id}"), dpoint: 0, regulation_id: 1, players };
        self.by_address.insert(addr, account.clone());
        account
    }
}
