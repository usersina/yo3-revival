//! User events (UserEventScene 0x4D9BB0, then the lobby, TournamentScene in user mode): a host's tournament that
//! players join, which the server then pairs round by round.
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
}

impl Event {
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
