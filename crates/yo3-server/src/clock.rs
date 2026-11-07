//! Duel clocks between players: a chess clock with a turn budget. Each player has a turn clock, full again at the
//! start of each of their turns, and a total clock for the duel; both run while the engine waits on that player, and
//! the first to empty loses (reason 11, "has ran out of time"). Clients see them through the timers message
//! (duelRequestName 0x14), which sets the engine's clock slots (0x520A00); DuelCockpit (0x502660) counts a running
//! pair down by itself and shows it.
use std::time::{Duration, Instant};

use yo3_proto::{Message, Value};

use crate::duel::SERVER_DUEL;

/// The duel connection's timers message (pump case 0x52396A).
const TIMERS: u32 = 0x14;
/// Create Duel's default limits (its menu's choices: 180 s a turn, 600 s a duel), also used for Auto Duel (**guess**).
pub const DEFAULT_TURN: u32 = 180;
pub const DEFAULT_TOTAL: u32 = 600;
/// How long past 0 the server waits before a time-out: a client starts its countdown only when it takes the timers
/// message off its queue, after the animations before it (live: its turn clock still read 3 at the time-out).
const GRACE: Duration = Duration::from_secs(5);

pub struct Clocks {
    /// Seconds a turn and a duel may take (0: that clock is off).
    turn_limit: u32,
    total_limit: u32,
    /// Time left on each seat's turn and total clocks, as of the last charge.
    turn: [Duration; 2],
    total: [Duration; 2],
    /// The seat the engine waits on, and since when.
    running: Option<(usize, Instant)>,
}

impl Clocks {
    /// Clocks with these limits in seconds; none when both are off.
    pub fn new(turn: u32, total: u32) -> Option<Clocks> {
        if turn == 0 && total == 0 {
            return None;
        }
        let (t, d) = (Duration::from_secs(turn as u64), Duration::from_secs(total as u64));
        Some(Clocks { turn_limit: turn, total_limit: total, turn: [t; 2], total: [d; 2], running: None })
    }

    /// A seat's time left on both clocks at `now`.
    fn left(&self, seat: usize, now: Instant) -> (Duration, Duration) {
        let spent = match self.running {
            Some((s, since)) if s == seat => now.saturating_duration_since(since),
            _ => Duration::ZERO,
        };
        (self.turn[seat].saturating_sub(spent), self.total[seat].saturating_sub(spent))
    }

    /// The engine now waits on `seat` (None: on nobody): the time of the seat waited on so far is charged.
    pub fn wait_on(&mut self, seat: Option<usize>, now: Instant) {
        if let Some((s, _)) = self.running {
            (self.turn[s], self.total[s]) = self.left(s, now);
        }
        self.running = seat.map(|s| (s, now));
    }

    /// The start of one of `seat`'s turns: its turn clock is full again.
    pub fn new_turn(&mut self, seat: usize, now: Instant) {
        if let Some((s, _)) = self.running
            && s == seat
        {
            self.running = Some((s, now));
            self.total[s] = self.left(s, now).1;
        }
        self.turn[seat] = Duration::from_secs(self.turn_limit as u64);
    }

    /// The seat waited on, once one of its clocks that is on has run out (and the grace after it).
    pub fn expired(&self, now: Instant) -> Option<usize> {
        let (seat, _) = self.running?;
        let (turn, total) = self.left(seat, now.checked_sub(GRACE)?);
        let out = (self.turn_limit > 0 && turn.is_zero()) || (self.total_limit > 0 && total.is_zero());
        out.then_some(seat)
    }

    /// The timers message for the client of seat `view`: its own clocks as count1 / total1 (PlayerNum 0's slots), the
    /// opponent's as count2 / total2, in seconds with their limits as maxima; timerKind 0 runs the client's pair, 1
    /// the opponent's (start 1), or both stop (timerKind 0, start 0). Clocks that are off show 0.
    pub fn message(&self, view: usize, now: Instant) -> Message {
        let secs = |d: Duration, limit: u32| if limit == 0 { 0 } else { d.as_secs() as u32 };
        let [(c1, t1), (c2, t2)] = [view, 1 - view].map(|s| {
            let (turn, total) = self.left(s, now);
            (secs(turn, self.turn_limit), secs(total, self.total_limit))
        });
        let (kind, start) = match self.running {
            Some((s, _)) => ((s != view) as u32, 1),
            None => (0, 0),
        };
        let fields = [
            ("timerKind", kind),
            ("start", start),
            ("count1", c1),
            ("total1", t1),
            ("count2", c2),
            ("total2", t2),
            ("other", 0),
            ("count1_max", self.turn_limit),
            ("total1_max", self.total_limit),
            ("count2_max", self.turn_limit),
            ("total2_max", self.total_limit),
            ("other_max", 0),
        ];
        let m = Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(TIMERS));
        fields.into_iter().fold(m, |m, (k, v)| m.with(k, Value::U32(v)))
    }
}
