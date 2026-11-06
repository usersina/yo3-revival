//! Front server: login -> player select -> world entry -> main-screen polls.
//!
//! The request/reply catalog, with the client evidence and status of every pair, is
//! `docs/protocol.md`. Keep it, this file and `tests/front_contract.rs` in step.
use std::sync::Arc;

use yo3_proto::{Message, Packet, Value};

pub const VERSION_VALID: u32 = 0x219F;
pub const HEARTBEAT: u32 = 0x2724;

/// Server clock in Japan time: the client treats server timestamps as JST (UTC+9) and
/// converts them to the player's local time.
fn jst_now() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(9 * 3600).expect("valid offset"))
}

/// Main-screen polls whose replies carry nothing the client reads.
const BARE_REPLIES: [(u32, u32); 4] = [(0x271A, 0x271B), (0x2710, 0x2711), (0x2AFB, 0x2B13), (0x2AFD, 0x2B17)];

#[derive(Clone, Debug)]
pub struct Player {
    pub player_id: u32,
    pub name: String,
    pub index: u32,
    pub level: u32,
    /// 17 BE u16 appearance slots (meaning of each slot not mapped yet).
    pub avatar: Vec<u8>,
    /// Per-player flag blob; 4096 bits like the 4096 entries of data/extra/yo3pbininfo.bin.
    pub pbin: Vec<u8>,
}

impl Player {
    pub fn new(player_id: u32, name: &str) -> Player {
        Player { player_id, name: name.into(), index: 0, level: 1, avatar: vec![0; 34], pbin: vec![0; 512] }
    }

    /// The record read by the player list (0x46C1E4) and the PlayerData loader (0x51C260).
    pub fn record(&self) -> Message {
        let mut m = Message::record()
            .with("pname", Value::text(&self.name))
            .with("pindex", Value::U32(self.index))
            .with("player_id", Value::U32(self.player_id))
            .with("dlv", Value::U32(self.level))
            .with("title", Value::U32(0))
            .with("n_win", Value::U32(0))
            .with("n_lose", Value::U32(0))
            .with("n_draw", Value::U32(0))
            .with("team_level", Value::U32(0))
            .with("team_name", Value::text(""))
            .with("team_authority", Value::U32(0))
            .with("avatar1", Value::Bin(self.avatar.clone()))
            .with("pbin", Value::Bin(self.pbin.clone()));
        for i in 0..23 {
            m = m.with(&format!("speech{i:02}"), Value::text(""));
        }
        for i in 0..2 {
            m = m.with(&format!("comment{i:02}"), Value::text(""));
        }
        m
    }
}

#[derive(Clone, Debug)]
pub struct Account {
    pub user_id: u32,
    pub name: String,
    pub dpoint: u32,
    pub regulation_id: u32,
    pub players: Vec<Player>,
}

impl Account {
    pub fn test() -> Account {
        Account { user_id: 1001, name: "test".into(), dpoint: 0, regulation_id: 1, players: vec![Player::new(1001, "test")] }
    }
}

/// Server-wide state shared by all sessions.
pub struct World {
    pub regulation_ids: Vec<u16>,
    pub news: Vec<(String, String)>,
}

impl World {
    pub fn new(regulation_ids: Vec<u16>) -> World {
        World {
            regulation_ids,
            news: vec![(
                "Welcome back to Yu-Gi-Oh! ONLINE".into(),
                "This is a private revival server. Have fun duelling with your friends!".into(),
            )],
        }
    }
}

pub struct FrontSession {
    account: Account,
    world: Arc<World>,
    opened: bool,
    selected: Option<usize>,
}

impl FrontSession {
    pub fn new(account: Account, world: Arc<World>) -> FrontSession {
        FrontSession { account, world, opened: false, selected: None }
    }

    /// Handles one client packet and returns the replies to send, in order.
    pub fn handle(&mut self, packet: &Packet) -> Vec<Message> {
        if !self.opened {
            self.opened = true;
            return vec![Message::reply(VERSION_VALID)];
        }
        let Packet::Message(msg) = packet else { return vec![] };
        let Some(id) = msg.request() else { return vec![] };
        if let Some(&(_, reply)) = BARE_REPLIES.iter().find(|(req, _)| *req == id) {
            return vec![Message::reply(reply)];
        }
        let a = &self.account;
        let reply = match id {
            0x1F47 => Message::reply(0x219A)
                .with("regulation_id", Value::U32(a.regulation_id))
                .with("user_id", Value::U32(a.user_id))
                .with("uname", Value::text(&a.name))
                .with("dpoint", Value::U32(a.dpoint)),
            0x1F48 => Message::reply(0x2201).with("regulation_id", Value::U32(a.regulation_id)).item(
                Message::record()
                    .with("user_id", Value::U32(a.user_id))
                    .with("uname", Value::text(&a.name))
                    .with("dpoint", Value::U32(a.dpoint)),
            ),
            0x1F49 => a.players.iter().fold(Message::reply(0x2260), |m, p| m.item(p.record())),
            0x1F4C => {
                self.selected = self.find_player(msg);
                if self.selected.is_none() {
                    return vec![];
                }
                Message::reply(0x2266)
            }
            0x1FA4 => {
                if self.find_player(msg).or(self.selected).is_none() {
                    return vec![];
                }
                // Copied wholesale into the trunk dataset; which fields later scenes read is unknown.
                Message::reply(9000).item(Message::record())
            }
            0x2072 => Message::reply(0x2398).with("regulations", Value::u16_be_array(&self.world.regulation_ids)),
            0x27D8 => Message::reply(0x27D9).item(Message::record()),
            0x2CEE => Message::reply(0x2CF3).item(self.player().record()),
            0x2583 => Message::reply(0x2588).with("count", Value::U32(0)),
            0x1F41 => {
                let now = jst_now().format("%Y-%m-%d %H:%M:%S").to_string();
                let entries = self.world.news.iter().map(|(title, body)| {
                    Message::record()
                        .with("starttime", Value::text(&now))
                        .with("title", Value::text(title))
                        .with("body", Value::text(body))
                });
                let lang = entries.fold(Message::record().with("lang", Value::text("1")), Message::item);
                Message::reply(0x21FC).item(lang)
            }
            0x2CF0 => Message::reply(0x2CF7),
            _ => return vec![],
        };
        vec![reply]
    }

    fn find_player(&self, msg: &Message) -> Option<usize> {
        let id = msg.get("player_id").and_then(Value::as_u32)?;
        self.account.players.iter().position(|p| p.player_id == id)
    }

    fn player(&self) -> &Player {
        &self.account.players[self.selected.unwrap_or(0)]
    }
}
