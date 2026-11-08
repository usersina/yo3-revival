//! User events (UserEventScene 0x4D9BB0, then the lobby, TournamentScene in user mode): a host's tournament that
//! players join, which the server then pairs round by round.
use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

/// An event's status (0x4D6970): open for entries, running, finished.
pub const OPEN: u32 = 12;
pub const RUNNING: u32 = 13;
pub const FINISHED: u32 = 14;

#[derive(Clone, Debug)]
pub struct Event {
    pub id: u32,
    pub host: (u32, String),
    /// The Create form's fields (0x3390: event_name, comment, entry_max, prize_type, level_min, level_max, eval_level,
    /// player_limit, password1, lang, match_type, duel_surrender, duel_watch, duel_time_turn, duel_time_total,
    /// duel_lp_start, regulation_id, join_max), as sent.
    pub fields: Vec<(String, Value)>,
    pub status: u32,
    /// The players in it, in the order they entered (the host first), and the ones still in the running.
    pub entrants: Vec<(u32, String)>,
    pub alive: Vec<u32>,
    /// The duels of the round under way: (duel_id, its two players).
    pub round: Vec<(u32, [u32; 2])>,
    pub round_number: u32,
    /// The bracket's seats, fixed when the event starts (the entrants in order; a player who leaves keeps theirs).
    pub slots: Vec<(u32, String)>,
    /// The rounds each player has gone through so far (a bye counts), which the bracket shows as wins.
    pub wins: Vec<(u32, u32)>,
    /// The loser of the final.
    pub second: Option<u32>,
    /// A duel of the round has begun (a player sat at its table): the lobby says "Duel in Progress".
    pub dueling: bool,
}

/// The lobby's own status (`0x32FB`'s `status`, TournamentScene `+0x2F8`; texts at `0x4CE670`): 0 and 1 the event
/// coming up, 6 "Prepare to Duel", 7 "Duel in Progress", 8 "Event Completed". Above 1 the host's Start and Cancel go.
pub const LOBBY_OPEN: u32 = 0;
pub const LOBBY_PREPARE: u32 = 6;
pub const LOBBY_DUELING: u32 = 7;
pub const LOBBY_COMPLETED: u32 = 8;

/// The bracket is a tree of 8 seats (`0x4CEBE0`): 14 nodes, the seats then the winners of the first round (8-11) and
/// of the second (12-13), and a block's champion is the one who won three rounds. The map (`0x3309`) has a flag per
/// block: 0x40 of the first level, 8 of the second (index 0x40), one of the last (0x48).
pub const MAP_BLOCKS: usize = 0x49;
const SECOND_LEVEL: u32 = 0x40;
const LAST_LEVEL: u32 = 0x48;

impl Event {
    pub fn new(id: u32, host: (u32, String), fields: Vec<(String, Value)>) -> Event {
        Event { id, host, fields, status: OPEN, entrants: vec![], alive: vec![], round: vec![], round_number: 0, slots: vec![], wins: vec![], second: None, dueling: false }
    }

    /// The lobby's status.
    pub fn lobby_status(&self) -> u32 {
        match self.status {
            OPEN => LOBBY_OPEN,
            RUNNING if self.dueling => LOBBY_DUELING,
            RUNNING => LOBBY_PREPARE,
            _ => LOBBY_COMPLETED,
        }
    }

    /// Everyone who is in the event's lobby: its entrants and the seats it started with.
    pub fn players(&self) -> std::collections::BTreeSet<u32> {
        self.slots.iter().chain(&self.entrants).map(|(p, _)| *p).collect()
    }

    pub fn winner(&self) -> Option<u32> {
        self.alive.first().copied().filter(|_| self.status == FINISHED)
    }

    /// A player's name, from the seats or the entrants.
    pub fn name(&self, player_id: u32) -> String {
        self.slots.iter().chain(&self.entrants).find(|(p, _)| *p == player_id).map(|(_, n)| n.clone()).unwrap_or_default()
    }

    /// Who sits in the bracket: the entrants while the event is open, then the seats fixed at its start.
    pub fn seats(&self) -> &[(u32, String)] {
        if self.status == OPEN { &self.entrants } else { &self.slots }
    }

    pub fn won(&self, player_id: u32) -> u32 {
        self.wins.iter().find(|(p, _)| *p == player_id).map_or(0, |(_, w)| *w)
    }

    /// A player has gone through a round.
    pub fn advance(&mut self, player_id: u32) {
        match self.wins.iter_mut().find(|(p, _)| *p == player_id) {
            Some((_, w)) => *w += 1,
            None => self.wins.push((player_id, 1)),
        }
    }

    /// How many rounds the knockout takes.
    pub fn total_rounds(&self) -> u32 {
        (self.seats().len().max(1) as f64).log2().ceil() as u32
    }

    /// The round as the lobby names it: `0x4CE670` shows the text `0x3BCD + n` ("1st Round" ... "6th Round", 7 Quarter-Final,
    /// 8 Semi-Finals, 9 Final) and the banners `0x32D3` the matching "Starting ..." and "... has now finished" (`0x4CC5B0`).
    pub fn round_label(&self) -> u32 {
        match self.total_rounds().saturating_sub(self.round_number) {
            _ if self.round_number == 0 => 0,
            0 => 9,
            1 if self.total_rounds() > 1 => 8,
            2 if self.total_rounds() > 2 => 7,
            _ => self.round_number.min(6),
        }
    }

    /// The block (0x32CB `block_index`) a seat belongs to on a level.
    fn block_of(seat: usize, level: u32) -> u32 {
        match level {
            0 => seat as u32 / 8,
            1 => SECOND_LEVEL + seat as u32 / 64,
            _ => LAST_LEVEL,
        }
    }

    /// The blocks the bracket has: a first-level block per 8 seats, and the levels above while more than one block is
    /// left below.
    fn block_count(&self) -> Vec<u32> {
        let seats = self.seats().len() as u32;
        let first = seats.div_ceil(8);
        let mut blocks: Vec<u32> = (0..first).collect();
        if first > 1 {
            let second = first.div_ceil(8);
            blocks.extend((0..second).map(|b| SECOND_LEVEL + b));
            if second > 1 {
                blocks.push(LAST_LEVEL);
            }
        }
        blocks
    }

    /// `0x3309`'s `map_data`: a non-zero byte per block the bracket has.
    pub fn map_data(&self) -> Vec<u8> {
        let mut map = vec![0; MAP_BLOCKS];
        for b in self.block_count() {
            map[b as usize] = 1;
        }
        map
    }

    /// The blocks the round under way plays in (`0x3305`'s `phase_block1-3`, lit on the map; -1 for none).
    pub fn phase_blocks(&self) -> [u32; 3] {
        let level = self.round_number.saturating_sub(1) / 3;
        let mut blocks: Vec<u32> = vec![];
        for (_, players) in &self.round {
            for p in players {
                if let Some(seat) = self.seats().iter().position(|(s, _)| s == p) {
                    let b = Event::block_of(seat, level);
                    if !blocks.contains(&b) {
                        blocks.push(b);
                    }
                }
            }
        }
        let mut out = [u32::MAX; 3];
        for (o, b) in out.iter_mut().zip(blocks) {
            *o = b;
        }
        out
    }

    /// One block of the bracket for `0x32FF` (read by `0x4CEBE0` by `block_number`): `start_level` and, per seat 1-8,
    /// `player_id{n}` (0 for empty) and `win_count{n}` (rounds won in this block: 1 puts the player on nodes 8-11,
    /// 2 on 12-13). A block of a higher level holds the champions (3 wins) of the blocks below it.
    pub fn block(&self, index: u32) -> Option<Message> {
        if !self.block_count().contains(&index) {
            return None;
        }
        let (level, number) = match index {
            LAST_LEVEL => (2, 0),
            i if i >= SECOND_LEVEL => (1, i - SECOND_LEVEL),
            i => (0, i),
        };
        let seats = self.seats();
        let mut block = Message::record().with("block_number", Value::U32(index)).with("start_level", Value::U32(level));
        for n in 0..8u32 {
            // The seat's player, and how far they have gone since this block began.
            let player = if level == 0 {
                seats.get((number * 8 + n) as usize).map(|(p, _)| *p)
            } else {
                // The champion of the lower block this seat stands for.
                let below = number * 8 + n;
                let width = 8usize.pow(level);
                let from = below as usize * width;
                seats.iter().skip(from).take(width).map(|(p, _)| *p).find(|p| self.won(*p) >= 3 * level)
            };
            let wins = player.map_or(0, |p| self.won(p).saturating_sub(3 * level).min(3));
            block = block.with(&format!("player_id{}", n + 1), Value::U32(player.unwrap_or(0))).with(&format!("win_count{}", n + 1), Value::U32(wins));
        }
        Some(block)
    }

    /// The event as the data directory keeps it.
    pub fn to_json(&self) -> Json {
        let fields: Vec<Json> = self
            .fields
            .iter()
            .filter_map(|(k, v)| match v {
                Value::Text(_) | Value::TextAlt(_) => Some(json!([k, "t", v.as_str().unwrap_or_default()])),
                v => v.as_u32().map(|n| json!([k, "n", n])),
            })
            .collect();
        let pairs = |v: &[(u32, String)]| v.iter().map(|(p, n)| json!([p, n])).collect::<Vec<_>>();
        json!({ "id": self.id, "host": [self.host.0, self.host.1], "fields": fields, "status": self.status,
            "entrants": pairs(&self.entrants), "alive": self.alive, "round_number": self.round_number,
            "slots": pairs(&self.slots), "wins": self.wins.iter().map(|(p, w)| json!([p, w])).collect::<Vec<_>>(), "second": self.second })
    }

    pub fn from_json(j: &Json) -> Option<Event> {
        let pair = |v: &Json| Some((v[0].as_u64()? as u32, v[1].as_str()?.to_string()));
        let fields = j["fields"]
            .as_array()?
            .iter()
            .filter_map(|f| {
                let k = f[0].as_str()?.to_string();
                Some((k, if f[1] == "t" { Value::text(f[2].as_str()?) } else { Value::U32(f[2].as_u64()? as u32) }))
            })
            .collect();
        let mut event = Event::new(j["id"].as_u64()? as u32, pair(&j["host"])?, fields);
        event.status = j["status"].as_u64()? as u32;
        event.entrants = j["entrants"].as_array()?.iter().filter_map(pair).collect();
        event.alive = j["alive"].as_array()?.iter().filter_map(|p| Some(p.as_u64()? as u32)).collect();
        event.round_number = j["round_number"].as_u64().unwrap_or(0) as u32;
        event.slots = j["slots"].as_array()?.iter().filter_map(pair).collect();
        event.wins = j["wins"].as_array()?.iter().filter_map(|w| Some((w[0].as_u64()? as u32, w[1].as_u64()? as u32))).collect();
        event.second = j["second"].as_u64().map(|p| p as u32);
        Some(event)
    }

    pub fn field(&self, k: &str) -> Option<&Value> {
        self.fields.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }

    pub fn int(&self, k: &str) -> u32 {
        self.field(k).and_then(Value::as_u32).unwrap_or(0)
    }

    pub fn text(&self, k: &str) -> String {
        self.field(k).and_then(Value::as_str).unwrap_or_default()
    }

    /// The most players it takes (entry_max; at least two).
    pub fn capacity(&self) -> usize {
        (self.int("entry_max") as usize).max(2)
    }

    /// A row of the event list (0x177B, read by 0x4D66A0) with the detail view's fields (0x4D75F0).
    pub fn row(&self) -> Message {
        let ints = ["prize_type", "match_type", "regulation_id", "level_min", "level_max", "eval_level", "duel_surrender"];
        let ints = ints.into_iter().chain(["duel_watch", "duel_time_turn", "duel_time_total", "duel_lp_start", "player_limit"]);
        let row = Message::record()
            .with("event_id", Value::U32(self.id))
            .with("event_name", Value::text(&self.text("event_name")))
            .with("comment", Value::text(&self.text("comment")))
            .with("status", Value::U32(self.status))
            // Participation Limit: 1 OFF (live), above it friends or team only, which shows the event as restricted
            // (crossed out, no Participate) for everyone (**guess**: the original checked the viewer).
            .with("join_flag", Value::U32((self.int("player_limit") > 1) as u32))
            .with("password1", Value::text(&self.text("password1")))
            .with("entry_max", Value::U32(self.capacity() as u32))
            .with("entry_num", Value::U32(self.entrants.len() as u32))
            .with("lang", Value::U32(self.int("lang")))
            .with("player_id", Value::U32(self.host.0))
            .with("pname", Value::text(&self.host.1));
        ints.fold(row, |m, k| m.with(k, Value::U32(self.int(k))))
    }
}
