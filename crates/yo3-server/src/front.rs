//! Front server: login -> player select -> world entry -> main-screen polls -> chat.
//!
//! The request/reply catalog, with the client evidence and status of every pair, is
//! `docs/protocol.md`. Keep it, this file and the contract tests in step.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yo3_proto::{Message, Packet, Value};

/// Where a session's outgoing messages go: its connection's writer task.
pub type Outbox = tokio::sync::mpsc::UnboundedSender<Message>;

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

/// A chat room, as listed in 0x2458 (fields read by 0x434D20).
#[derive(Clone, Debug)]
pub struct Room {
    /// Must be at least 1: the 0x245A handler ignores lower ids.
    pub id: u32,
    pub name: String,
    pub capacity: u32,
    /// Meaning of region/status/type not mapped yet; 0 works.
    pub region: u32,
    pub status: u32,
    pub kind: u32,
}

impl Room {
    fn new(id: u32, name: &str) -> Room {
        Room { id, name: name.into(), capacity: 50, region: 0, status: 0, kind: 0 }
    }
}

/// Server-wide state shared by all sessions.
pub struct World {
    pub regulation_ids: Vec<u16>,
    pub news: Vec<(String, String)>,
    pub rooms: Vec<Room>,
}

impl World {
    pub fn new(regulation_ids: Vec<u16>) -> World {
        World {
            regulation_ids,
            news: vec![(
                "Welcome back to Yu-Gi-Oh! ONLINE".into(),
                "This is a private revival server. Have fun duelling with your friends!".into(),
            )],
            rooms: vec![
                Room::new(1, "Lobby"),
                Room::new(2, "Free Talk"),
                Room::new(3, "Trading"),
                Room::new(4, "Duel Recruiting"),
            ],
        }
    }
}

/// Who is online, in which chat room, and how to reach them.
#[derive(Default)]
pub struct Hub {
    next_id: u64,
    online: HashMap<u64, Online>,
}

struct Online {
    player_id: u32,
    name: String,
    level: u32,
    room: Option<u32>,
    outbox: Outbox,
}

impl Hub {
    pub fn online(&self) -> usize {
        self.online.len()
    }

    fn join(&mut self, player: &Player, outbox: Outbox) -> u64 {
        self.next_id += 1;
        let online = Online { player_id: player.player_id, name: player.name.clone(), level: player.level, room: None, outbox };
        self.online.insert(self.next_id, online);
        self.next_id
    }

    fn leave(&mut self, id: u64) {
        self.online.remove(&id);
    }

    fn set_room(&mut self, id: u64, room: Option<u32>) {
        if let Some(o) = self.online.get_mut(&id) {
            o.room = room;
        }
    }

    fn room_of(&self, id: u64) -> Option<u32> {
        self.online.get(&id).and_then(|o| o.room)
    }

    fn occupancy(&self, room: u32) -> u32 {
        self.online.values().filter(|o| o.room == Some(room)).count() as u32
    }

    /// Sends to everyone in `room` except `except`.
    fn send_room(&self, room: u32, msg: &Message, except: u64) {
        for (_, o) in self.online.iter().filter(|(id, o)| **id != except && o.room == Some(room)) {
            let _ = o.outbox.send(msg.clone());
        }
    }

    /// Sends to one player by id or name; false if they are not online.
    fn send_player(&self, to_id: Option<u32>, to_name: Option<&str>, msg: &Message) -> bool {
        let target = self.online.values().find(|o| Some(o.player_id) == to_id || Some(o.name.as_str()) == to_name);
        target.is_some_and(|o| o.outbox.send(msg.clone()).is_ok())
    }
}

pub struct FrontSession {
    account: Account,
    world: Arc<World>,
    hub: Arc<Mutex<Hub>>,
    outbox: Outbox,
    opened: bool,
    selected: Option<usize>,
    /// Hub registration once a player is selected.
    online: Option<u64>,
}

impl Drop for FrontSession {
    fn drop(&mut self) {
        if let Some(id) = self.online
            && let Ok(mut hub) = self.hub.lock()
        {
            hub.leave(id);
        }
    }
}

impl FrontSession {
    pub fn new(account: Account, world: Arc<World>, hub: Arc<Mutex<Hub>>, outbox: Outbox) -> FrontSession {
        FrontSession { account, world, hub, outbox, opened: false, selected: None, online: None }
    }

    fn hub(&self) -> std::sync::MutexGuard<'_, Hub> {
        self.hub.lock().expect("hub lock")
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
                let mut hub = self.hub.lock().expect("hub lock");
                if let Some(old) = self.online {
                    hub.leave(old);
                }
                let id = hub.join(self.player(), self.outbox.clone());
                drop(hub);
                self.online = Some(id);
                Message::reply(0x2266)
            }
            0x1FA4 => {
                if self.find_player(msg).or(self.selected).is_none() {
                    return vec![];
                }
                // Copied wholesale into the trunk dataset; which fields later scenes read is unknown.
                let trunk = Message::reply(9000).item(Message::record());
                // The client never asks for its friend and black lists; the global dispatcher stores
                // 0x251C in dataset slot 6 and 0x252A in slot 7, and listing any player reads both
                // without a NULL check (0x45E6D0, 0x45E810). Items would be {friend_id, friend_name,
                // login_flag} and {black_id, black_name, login_flag} (0x475A20 modes 2 and 3).
                // Likewise the rental records 0x2D6C (slot 8), which opening the shop reads (0x48F080).
                return vec![trunk, Message::reply(0x251C), Message::reply(0x252A), Message::reply(0x2D6C)];
            }
            // Shop (handler 0x48AA60): its text, then a chain where each reply asks for the next:
            // items, exchange items, protectors, event items, perks, rental terms, avatar items (the
            // last clears isLoading). The stock comes with the economy (M4).
            0x1B66 => Message::reply(0x1C20).with("text", Value::text("Welcome! The shelves are empty for now.")),
            0x1B67 => Message::reply(0x1C22),
            0x1B6F => Message::reply(0x1C26),
            0x1B6B => Message::reply(0x1C32),
            0x1B6D => Message::reply(0x1C2E),
            // perk_num is compared with 500 to show _root.expack_del; meaning unknown.
            0x1B72 => Message::reply(0x1C36).with("perk_num", Value::U32(0)),
            0x2C40 => ["rental_cardnum", "rental_point", "rental_term"]
                .into_iter()
                .fold(Message::reply(0x2D75), |m, k| m.with(k, Value::U32(0))),
            0x1B69 => Message::reply(0x1C2A),
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
            0x2CF0 => self.player_search(msg),
            // Next present: 0x2777 would carry one; 0x2778 = none, which closes PresentWindow
            // (0x4783C9) and lets the main screen unlock its facilities and menu (0x473B87).
            0x2775 => Message::reply(0x2778),
            0x2135 => self.room_list(),
            0x2136 => self.enter_room(msg),
            0x2137 => {
                if let Some(id) = self.online {
                    self.hub().set_room(id, None);
                }
                Message::reply(0x245C)
            }
            0x2134 => return self.chat(msg),
            _ => return vec![],
        };
        vec![reply]
    }

    /// 0x2135 -> 0x2458: top-level lang, one item per room (ChatWindow 0x435950, items via 0x434D20).
    fn room_list(&self) -> Message {
        let hub = self.hub();
        self.world.rooms.iter().fold(Message::reply(0x2458).with("lang", Value::U32(1)), |m, r| {
            m.item(
                Message::record()
                    .with("id", Value::U32(r.id))
                    .with("name", Value::text(&r.name))
                    .with("capacity", Value::U32(r.capacity))
                    .with("num", Value::U32(hub.occupancy(r.id)))
                    .with("region", Value::U32(r.region))
                    .with("status", Value::U32(r.status))
                    .with("type", Value::U32(r.kind)),
            )
        })
    }

    /// 0x2136 {room_id} -> 0x245A {room_id}, or 0x245B {error_code} if unknown or full.
    fn enter_room(&self, msg: &Message) -> Message {
        let wanted = msg.get("room_id").and_then(Value::as_u32);
        let room = self.world.rooms.iter().find(|r| Some(r.id) == wanted);
        let (Some(room), Some(id)) = (room, self.online) else {
            return Message::reply(0x245B).with("error_code", Value::U32(1));
        };
        let mut hub = self.hub();
        if hub.room_of(id) != Some(room.id) && hub.occupancy(room.id) >= room.capacity {
            return Message::reply(0x245B).with("error_code", Value::U32(2));
        }
        hub.set_room(id, Some(room.id));
        Message::reply(0x245A).with("room_id", Value::U32(room.id))
    }

    /// 0x2CF0 -> 0x2CF7: the online players matching the search, one page of them. InfoWindow
    /// 0x458410 reads result_number (the total; it shows pages of 100) and each item through
    /// 0x475A20 in player mode. It skips its own player_id, so the searcher is listed too.
    /// search_flag bits come from the builder 0x45F390; 0x4B6AF0 looks one player up by id.
    fn player_search(&self, msg: &Message) -> Message {
        const BY_NAME: u32 = 0x2;
        const BY_LEVEL: u32 = 0x10;
        const BY_ID: u32 = 0x40000;
        let uint = |k| msg.get(k).and_then(Value::as_u32);
        let flags = uint("search_flag").unwrap_or(0);
        // Guess: case-insensitive part of the name. Other criteria (lang, country, eval_level,
        // the preset tabs) are not stored yet, so they match everyone.
        let name = msg.get("pname").and_then(Value::as_str).unwrap_or_default().to_lowercase();
        let levels = uint("min_level").unwrap_or(0)..=uint("max_level").unwrap_or(u32::MAX);
        let hub = self.hub();
        let mut found: Vec<&Online> = hub
            .online
            .values()
            .filter(|o| flags & BY_NAME == 0 || o.name.to_lowercase().contains(&name))
            .filter(|o| flags & BY_LEVEL == 0 || levels.contains(&o.level))
            .filter(|o| flags & BY_ID == 0 || Some(o.player_id) == uint("player_id"))
            .collect();
        found.sort_by_key(|o| o.player_id);
        // Guess: a page starts at base_point and holds at most front_num players.
        let skip = uint("base_point").unwrap_or(0) as usize;
        let page = found.iter().skip(skip).take(uint("front_num").unwrap_or(100) as usize);
        page.fold(Message::reply(0x2CF7).with("result_number", Value::U32(found.len() as u32)), |m, o| {
            m.item(
                Message::record()
                    .with("player_id", Value::U32(o.player_id))
                    .with("pname", Value::text(&o.name))
                    .with("dlv", Value::U32(o.level)),
            )
        })
    }

    /// 0x2134 -> 0x2456 to the sender, and the line as a 0x2455 push to the room (or the whisper
    /// target). The client sends its fields in one item (tag 10): chat_str, chat_type, and to_id
    /// (room chat: chat_type 1, to_id = the room) or to_name (whisper). The client shows nothing
    /// on 0x2456, so the sender gets the 0x2455 echo too. 0x2455 is read by ChatWindow 0x435950.
    fn chat(&self, msg: &Message) -> Vec<Message> {
        const ROOM_CHAT: u32 = 1;
        let Some(id) = self.online else { return vec![Message::reply(0x2457)] };
        let body = msg.items.first().unwrap_or(msg);
        let chat_type = body.get("chat_type").and_then(Value::as_u32).unwrap_or(0);
        let player = self.player();
        let line = Message::record()
            .with("from_id", Value::U32(player.player_id))
            .with("pname", Value::text(&player.name))
            .with("chat_type", Value::U32(chat_type))
            .with("chat_str", Value::text(&body.get("chat_str").and_then(Value::as_str).unwrap_or_default()));
        let push = Message::reply(0x2455).item(line);
        let to_id = body.get("to_id").and_then(Value::as_u32);
        let to_name = body.get("to_name").and_then(Value::as_str);
        let hub = self.hub();
        let delivered = match (chat_type, to_id, to_name) {
            (_, _, Some(name)) => hub.send_player(None, Some(&name), &push),
            (ROOM_CHAT, Some(room), None) => {
                let here = hub.room_of(id) == Some(room);
                if here {
                    hub.send_room(room, &push, id);
                }
                here
            }
            (_, Some(player_id), None) => hub.send_player(Some(player_id), None, &push),
            _ => false,
        };
        if !delivered {
            return vec![Message::reply(0x2457)];
        }
        vec![Message::reply(0x2456), push]
    }

    fn find_player(&self, msg: &Message) -> Option<usize> {
        let id = msg.get("player_id").and_then(Value::as_u32)?;
        self.account.players.iter().position(|p| p.player_id == id)
    }

    fn player(&self) -> &Player {
        &self.account.players[self.selected.unwrap_or(0)]
    }
}
