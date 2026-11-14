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
    /// The seat the engine waits on, and since when its time counts (after the animations' allowance).
    running: Option<(usize, Instant)>,
    /// Whether the clients have been told the waited-on clocks run (false while an allowance holds them).
    announced: bool,
    /// The settle marker the waited-on client was sent with its batch (`settle_marker`), and until when its answer
    /// counts; the next marker's number.
    settle: Option<(u32, Instant)>,
    next_settle: u32,
}

/// Engine command {seq} at the end of a batch to the player the engine waits on (Revival, issue #11): the connection
/// DLL takes it off the queue when the commands before it have played (the scene pops a command only then, 0x522A50),
/// waits until cards, battles and chains have settled (`tools/mod/duel-observer.h`), and answers duelRequestName
/// `SETTLED {seq}`. The stock dispatcher (0x514290) has no case for it.
pub const SETTLE: u32 = 0x5F1;
/// Client duelRequestName of that answer (`tools/mod/duel-observer.h`); the stock client sends 1-15.
pub const SETTLED: u32 = 0x7A13;

/// The marker for the batch of the client waited on (see [`SETTLE`]).
pub fn settle_marker(seq: u32) -> Message {
    crate::duel::command(SETTLE, [seq, 0, 0, 0])
}

/// The commands that ask the player (the input wait 4, the dialogs 0x36 and 0x37 and the data they read, 0x3EC-0x3F3,
/// and a mixed list's staged card table entries, 1000): a batch ends with them, after its animations.
const PROMPT_COMMANDS: [u32; 11] = [4, 0x36, 0x37, 0x3EC, 0x3ED, 0x3EE, 0x3EF, 0x3F0, 0x3F1, 0x3F2, 0x3F3];

/// Puts the settle marker `seq` into a batch after its animations and before the prompt that ends it (and the messages
/// the pump handles at once: the command map, the server's texts), so the client answers once it has played the
/// animations, not after the player's choice (a dialog or input wait holds the queue).
pub fn mark_settle(part: &mut Vec<Message>, seq: u32) {
    let is_command = |m: &Message| m.get("duelRequestName").and_then(Value::as_u32) == Some(17);
    let id = |m: &Message| m.get("id").and_then(Value::as_u32).unwrap_or(0);
    let at = part.iter().rposition(|m| is_command(m) && !PROMPT_COMMANDS.contains(&id(m)) && id(m) != 1000).map_or(0, |i| i + 1);
    part.insert(at, settle_marker(seq));
}

/// How long the client takes to play these commands' animations before its player can answer: a duration per animated
/// duel command (**guess**, from watching the client: a move 0.4 s, a phase banner 1.2 s, a turn's banner 1.5 s, a
/// summon's effect 1 s, an activation 0.8 s, a chain link set or resolved 0.5 s each, a shatter 0.6 s, an LP change
/// 0.7 s, a battle 1.5 s, a reveal's popup 1 s), at most `MAX_ALLOWANCE`.
pub fn allowance(commands: &[Message]) -> Duration {
    let ms: u64 = commands
        .iter()
        .filter(|m| m.get("duelRequestName").and_then(Value::as_u32) == Some(17))
        .map(|m| match m.get("id").and_then(Value::as_u32).unwrap_or(0) {
            0x1A | 0x1C | 0x17 => 400,
            5 => 1200,
            6 => 1500,
            0x38 | 0x39 | 0x19 => 1000,
            0x22 => 800,
            0x33 | 0x34 => 500,
            0x1F => 600,
            0x10 => 700,
            0xD => 1500,
            _ => 0,
        })
        .sum();
    Duration::from_millis(ms).min(MAX_ALLOWANCE)
}

/// The longest a batch's animations hold the clocks.
const MAX_ALLOWANCE: Duration = Duration::from_secs(15);
/// How long after its batch a client's settle answer still counts (issue #11): live, the duel's opening took 16 s to
/// play on the software-rendered test client, against an allowance of 8.7 s.
const ANSWER_WINDOW: Duration = Duration::from_secs(30);

impl Clocks {
    /// Clocks with these limits in seconds; none when both are off.
    pub fn new(turn: u32, total: u32) -> Option<Clocks> {
        if turn == 0 && total == 0 {
            return None;
        }
        let (t, d) = (Duration::from_secs(turn as u64), Duration::from_secs(total as u64));
        Some(Clocks {
            turn_limit: turn,
            total_limit: total,
            turn: [t; 2],
            total: [d; 2],
            running: None,
            announced: true,
            settle: None,
            next_settle: 1,
        })
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
        self.wait_after(seat, now, Duration::ZERO);
    }

    /// [`wait_on`], the new seat's time counting only after `allowance` (the animations it was just sent, see
    /// [`allowance`]); the clocks show stopped until then.
    /// The allowance is the fallback: the client says itself when it has played the batch ([`settled`]).
    pub fn wait_after(&mut self, seat: Option<usize>, now: Instant, allowance: Duration) {
        if let Some((s, _)) = self.running {
            (self.turn[s], self.total[s]) = self.left(s, now);
        }
        self.running = seat.map(|s| (s, now + allowance));
        self.announced = allowance.is_zero();
        self.settle = seat.map(|_| (self.next_settle, now + ANSWER_WINDOW));
        self.next_settle = self.next_settle.wrapping_add(1).max(1);
    }

    /// The seat waited on and the number of the settle marker to end its batch with (see [`SETTLE`]).
    pub fn awaiting(&self) -> Option<(usize, u32)> {
        Some((self.running?.0, self.settle?.0))
    }

    /// `seat`'s client has played the batch marked `seq` (its `SETTLED`): its clocks count from `now`, earlier or later
    /// than the allowance said, but never later than `ANSWER_WINDOW` after the batch (a silent or late client keeps the
    /// allowance), and once. True when the clients are to be told the clocks run from here.
    pub fn settled(&mut self, seat: usize, seq: u32, now: Instant) -> bool {
        let fits = |(s, deadline): (u32, Instant)| s == seq && now <= deadline;
        if self.running.map(|(s, _)| s) != Some(seat) || !self.settle.is_some_and(fits) {
            return false;
        }
        self.settle = None;
        self.running = Some((seat, now));
        self.announced = true;
        true
    }

    /// True once, when the allowance holding the waited-on clocks has passed: the clients are to be told they run.
    pub fn starting(&mut self, now: Instant) -> bool {
        if self.announced || !self.running.is_some_and(|(_, since)| since <= now) {
            return false;
        }
        self.announced = true;
        true
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
            // Held by an allowance: shown stopped.
            Some((s, since)) => ((s != view) as u32, (since <= now) as u32),
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
