//! A duel between two players: both clients' duel connections share one table, which plays the janken between their
//! hands and runs one rules engine for both. Each client sees itself as player 0 through its own translator; an engine
//! prompt goes to the client it asks, and each client's commands reach the other through that connection's outbox.
use crate::lock::Locked;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::Value as Json;
use yo3_proto::Message;

use crate::duel::{
    CLIENT, CPU, CardCommand, DUEL_START, FIRST, FIRST_BEGIN, FIRST_RESOLVE, JANKEN_BEGIN, JANKEN_RESOLVE, SECOND, SET_LP,
    command, piles, setup, shuffle,
};
use crate::clock::{Clocks, DEFAULT_TOTAL, DEFAULT_TURN};
use crate::engine::{Answer, Engine};
use crate::front::{DuelEntry, Ended, Hub, MATCH_MODE, Outbox, SINGLE_MODE, World};
use crate::translate::{DRAW, DUEL_END, LOST, Prompt, Repeats, Translator, WON};

/// The usual starting LP (the server's own unless --duel-lp changes it).
const DEFAULT_LP: u32 = 8000;
/// The duel-end reasons "has ran out of time" and "has disconnected" (text 1999 + 11, + 14).
const TIME_OUT: u32 = 11;
const DISCONNECTED: u32 = 14;
/// The most prompts the server answers by itself between two of a client's choices (a duel needs far fewer).
const MAX_ANSWERS: u32 = 10_000;
/// The engine's MSG_NEW_TURN {player}.
const MSG_NEW_TURN: u64 = 40;
/// Card Swap between a match's duels (DuelManager 0x50FC00 takes it to 0x50FDF0), instead of the next janken's 1500.
const CARD_SWAP: u32 = 1504;
/// A match's end for a spectator {match result: 1 won, 2 lost, 3 draw}: its DuelResult (0x512070) reads it at the
/// head of its queue (MainGamen +0x820) where a player's reads the front's 0x2009.
pub(crate) const MATCH_END: u32 = 1505;

struct Seat {
    player_id: u32,
    entry: DuelEntry,
    outbox: Outbox,
}

/// A client watching the duel from seat 0's side, with its own translator once the duel opened.
struct Spectator {
    outbox: Outbox,
    translator: Option<Translator>,
}

/// What each seat is sent, by seat.
pub type Out = [Vec<Message>; 2];

pub struct Table {
    world: Arc<World>,
    hub: Arc<Mutex<Hub>>,
    duel_id: u32,
    /// In the order the players connected.
    seats: Vec<Seat>,
    hands: [Option<u32>; 2],
    /// The janken's winner, who picks the turn order.
    picker: Option<usize>,
    engine: Option<Engine>,
    /// By seat, once the duel opened.
    translators: Vec<Translator>,
    /// The duel check (`--duel-check`): each seat's view of the duel, compared with the engine after each step.
    checks: Vec<crate::duelcheck::ClientModel>,
    /// Duels won so far in the match, by seat, and how many it takes.
    wins: [u32; 2],
    best_of: u32,
    /// The seat of the engine's first player, and this duel's clocks (none: no time limits).
    first: usize,
    clocks: Option<Clocks>,
    /// Spectators, and what they need to catch up: the duel's starting LP and every engine message since it opened.
    spectators: Vec<Spectator>,
    lp: u32,
    history: Vec<Json>,
    /// Seat 0's decisive janken {result, opponent's hand, own hand}, which a spectator's pre-duel screen plays first.
    janken: [u32; 3],
    /// Seats in Card Swap between a match's duels, until they are done (0xE).
    swapping: [bool; 2],
    /// Duels of the match that ended so far (drawn ones too).
    played: u32,
    /// The turn order was taken for the duel to come (once per janken).
    ordered: bool,
    over: bool,
    rng: u64,
    /// When the table was made (its first player connected), for [`NO_SHOW`].
    opened: Instant,
}

/// How long a match's first player waits for the other's duel connection before the match is cancelled (the other
/// client closed or crashed after Select Deck). Select Deck's own timer is 30 s and a cold Wine client takes about 20 s
/// to open the duel, so this leaves room for a slow one.
pub const NO_SHOW: std::time::Duration = std::time::Duration::from_secs(90);

impl Table {
    pub fn new(world: Arc<World>, hub: Arc<Mutex<Hub>>, duel_id: u32, seed: u64) -> Table {
        Table {
            world,
            hub,
            duel_id,
            seats: vec![],
            hands: [None; 2],
            picker: None,
            engine: None,
            translators: vec![],
            checks: vec![],
            wins: [0; 2],
            best_of: 1,
            first: 0,
            clocks: None,
            spectators: vec![],
            lp: DEFAULT_LP,
            history: vec![],
            janken: [0; 3],
            swapping: [false; 2],
            played: 0,
            ordered: false,
            over: false,
            rng: seed | 1,
            opened: Instant::now(),
        }
    }

    /// Seats a player (None when the table is full); once both sit, the janken begins.
    pub fn sit(&mut self, player_id: u32, entry: DuelEntry, outbox: Outbox) -> Option<(usize, Out)> {
        if self.seats.len() == 2 {
            return None;
        }
        self.best_of = self.best_of.max(entry.best_of);
        self.seats.push(Seat { player_id, entry, outbox });
        let seat = self.seats.len() - 1;
        let mut out = Out::default();
        if self.seats.len() == 2 {
            out = [vec![command(JANKEN_BEGIN, [0; 4])], vec![command(JANKEN_BEGIN, [0; 4])]];
        }
        Some((seat, out))
    }

    /// A janken hand (0 rock, 1 scissors, 2 paper). With both in, each client gets {its result: 0 draw (it asks for
    /// another hand by itself), 1 won, 2 lost; the opponent's hand; its own}, and the winner the turn-order choice.
    pub fn hand(&mut self, seat: usize, hand: u32) -> Out {
        let mut out = Out::default();
        if self.picker.is_some() || self.seats.len() < 2 {
            return out;
        }
        self.hands[seat] = Some(hand);
        let [Some(h0), Some(h1)] = self.hands else { return out };
        self.hands = [None; 2];
        for (s, (mine, theirs)) in [(h0, h1), (h1, h0)].into_iter().enumerate() {
            // Each hand beats the next one: rock > scissors > paper > rock.
            let result = if theirs == mine { 0 } else if theirs == (mine + 1) % 3 { 1 } else { 2 };
            out[s].push(command(JANKEN_RESOLVE, [result, theirs, mine, 0]));
            if s == 0 && result != 0 {
                self.janken = [result, theirs, mine];
            }
            match result {
                0 => {}
                1 => {
                    self.picker = Some(s);
                    out[s].push(command(FIRST_BEGIN, [1, 0, 0, 0]));
                }
                _ => out[s].push(command(FIRST_BEGIN, [0; 4])),
            }
        }
        out
    }

    /// The janken winner's choice (1 first, 2 second): the order from each side, then the duel's opening.
    pub fn choose(&mut self, seat: usize, choice: u32) -> Out {
        let mut out = Out::default();
        if self.picker != Some(seat) || self.ordered || self.over {
            return out;
        }
        self.ordered = true;
        let first = if choice == SECOND { 1 - seat } else { seat };
        for (s, part) in out.iter_mut().enumerate() {
            part.push(command(FIRST_RESOLVE, [if s == first { FIRST } else { SECOND }, 0, 0, 0]));
        }
        let opening = self.open(first);
        for (part, more) in out.iter_mut().zip(opening) {
            part.extend(more);
        }
        out
    }

    /// The engine's duel, team 0 the first player: each client's setup, card table, start and LP, then the engine up
    /// to its first prompt. Nothing without the engine.
    fn open(&mut self, first: usize) -> Out {
        let Some(dir) = self.world.engine_dir.clone() else { return Out::default() };
        let cids = |rows: &[u16]| rows.iter().filter_map(|r| self.world.cards.get(r).map(|c| c.cid)).collect::<Vec<u16>>();
        let (lead, other) = (&self.seats[first].entry.deck, &self.seats[1 - first].entry.deck);
        let (mut m0, e0, mut m1, e1) = (cids(&lead.main), cids(&lead.extra), cids(&other.main), cids(&other.extra));
        shuffle(&mut self.rng, &mut m0);
        shuffle(&mut self.rng, &mut m1);
        let mut engine = match Engine::start(&dir) {
            Ok(engine) => engine,
            Err(e) => {
                tracing::warn!("duel {}: rules engine not started: {e}", self.duel_id);
                return self.abandon();
            }
        };
        if self.world.duel_check { engine.record(); }
        // A hosted duel's starting LP (Create Duel's lp_start), unless the server's was set to test (--duel-lp).
        let hosted = self.seats.iter().find_map(|s| s.entry.lp);
        let lp = hosted.filter(|_| self.world.start_lp == DEFAULT_LP).unwrap_or(self.world.start_lp);
        let answer = engine.new_duel_named(crate::duel::next(&mut self.rng), lp, [(&m0, &e0), (&m1, &e1)], &self.world.card_names);
        self.engine = Some(engine);
        // A hosted duel's time_turn and time_total, else Create Duel's defaults, unless the server's were set to test.
        let hosted = self.seats.iter().find_map(|s| s.entry.times);
        let (turn, total) = self.world.test_times.or(hosted).unwrap_or((DEFAULT_TURN, DEFAULT_TOTAL));
        self.first = first;
        self.clocks = Clocks::new(turn, total);
        self.lp = lp;
        self.history.clear();
        let mut out = Out::default();
        self.checks.clear();
        for (s, part) in out.iter_mut().enumerate() {
            let (mine, theirs) = (&self.seats[s].entry.deck, &self.seats[1 - s].entry.deck);
            let order = if s == first { CLIENT } else { CPU };
            if self.world.duel_check {
                // The opponent's hidden cards' rows, from a translator that hides nothing (the same uids in the same order).
                let mut model = crate::duelcheck::ClientModel::new(order, self.world.cards.clone());
                let (mut full, mut known) = Translator::new(order, [&mine.main, &theirs.main], self.world.cards.clone());
                known.extend(full.extra_decks([&mine.extra, &theirs.extra]));
                for m in &known {
                    let int = |k: &str| m.get(k).and_then(yo3_proto::Value::as_u32).unwrap_or(0);
                    model.know(int("param1") & 0xFF_FFFF, int("param2") as u16);
                }
                self.checks.push(model);
            }
            let (mut translator, mut table) = Translator::against_player(order, [&mine.main, &theirs.main], self.world.cards.clone());
            translator.tokens(self.world.token_rows.clone());
            table.extend(translator.extra_decks([&mine.extra, &theirs.extra]));
            self.translators.push(translator);
            part.push(setup(order, mine, theirs));
            part.extend(table);
            part.extend([command(DUEL_START, [0; 4]), command(SET_LP, [CLIENT, lp, 0, 0]), command(SET_LP, [CPU, lp, 0, 0])]);
            part.extend(piles());
        }
        for i in 0..self.spectators.len() {
            let (translator, opening) = self.spectator_opening();
            self.spectators[i].translator = Some(translator);
            for m in opening {
                let _ = self.spectators[i].outbox.send(m);
            }
        }
        let more = self.advance(answer);
        for (part, more) in out.iter_mut().zip(more) {
            part.extend(more);
        }
        out
    }

    /// The players at the table, by seat, once both sat.
    pub fn players(&self) -> Option<[u32; 2]> {
        match &self.seats[..] {
            [a, b] => Some([a.player_id, b.player_id]),
            _ => None,
        }
    }

    /// Seat 0's opening for a spectator, with both sides' hidden cards hidden: its janken and turn order (which take the
    /// pre-duel screen to the field; 1502 {0}: no choice to make), then the setup, card table, duel start and LP.
    fn spectator_opening(&self) -> (Translator, Vec<Message>) {
        let (mine, theirs) = (&self.seats[0].entry.deck, &self.seats[1].entry.deck);
        let order = if self.first == 0 { CLIENT } else { CPU };
        let (mut translator, mut table) = Translator::spectating(order, [&mine.main, &theirs.main], self.world.cards.clone());
        translator.tokens(self.world.token_rows.clone());
        table.extend(translator.extra_decks([&mine.extra, &theirs.extra]));
        let [result, their_hand, my_hand] = self.janken;
        let mut out = vec![
            command(JANKEN_BEGIN, [0; 4]),
            command(JANKEN_RESOLVE, [result, their_hand, my_hand, 0]),
            command(FIRST_BEGIN, [0; 4]),
            command(FIRST_RESOLVE, [if self.first == 0 { FIRST } else { SECOND }, 0, 0, 0]),
            setup(order, mine, theirs),
        ];
        out.extend(table);
        out.extend([command(DUEL_START, [0; 4]), command(SET_LP, [CLIENT, self.lp, 0, 0]), command(SET_LP, [CPU, self.lp, 0, 0])]);
        out.extend(piles());
        (translator, out)
    }

    /// The last duel as a replay (`duellog::Replay`): what a spectator of the player who went first would have seen,
    /// from the card table to the duel's end (the janken left out), with that player first.
    fn replay(&self, hub: &Hub) -> Option<crate::duellog::Replay> {
        self.engine.as_ref()?;
        let [lead, other] = [self.first, 1 - self.first].map(|s| &self.seats[s]);
        let janken = crate::duel::lead_janken(self.janken, self.first == 0);
        let commands = crate::duel::replay_commands(&self.world, [&lead.entry.deck, &other.entry.deck], janken, self.lp, &self.history);
        let sides = [(lead, 0), (other, 1)].map(|(seat, kind)| hub.side(seat.player_id, &seat.entry.deck, kind));
        Some(crate::duellog::Replay { sides, commands })
    }

    /// A spectator joins (0x1838 on the duel connection): it watches from seat 0's side, and if the duel is on it gets
    /// seat 0's opening and the duel so far, replayed through its own translator, and the clocks. Returns those.
    pub fn watch(&mut self, outbox: Outbox) -> Vec<Message> {
        let mut spectator = Spectator { outbox, translator: None };
        let mut out = vec![];
        if self.engine.is_some() && self.seats.len() == 2 {
            let (mut translator, opening) = self.spectator_opening();
            out = opening;
            out.extend(translator.events(&self.history));
            if let Some(c) = &self.clocks {
                out.push(c.message(0, Instant::now()));
            }
            spectator.translator = Some(translator);
        }
        self.spectators.push(spectator);
        out
    }

    /// Sends each spectator its messages; one whose connection is gone leaves.
    fn tell_spectators(&mut self, parts: Vec<Vec<Message>>) {
        let mut gone = vec![];
        for (i, part) in parts.into_iter().enumerate() {
            if part.into_iter().any(|m| self.spectators[i].outbox.send(m).is_err()) {
                gone.push(i);
            }
        }
        for i in gone.into_iter().rev() {
            self.spectators.remove(i);
        }
    }

    /// Runs the engine from `answer`: its events go to both clients, each prompt to the client it asks (or is answered
    /// when that client has no window for it), until a client must choose or the duel ends. The clocks follow: a new
    /// turn fills its player's turn clock, and the client waited on has its clocks run (0x14 to both).
    fn advance(&mut self, mut answer: std::io::Result<Answer>) -> Out {
        let mut out = Out::default();
        let now = Instant::now();
        let mut waiting = None;
        let mut failed = false;
        let (mut repeats, mut answered) = (Repeats::default(), 0);
        let mut watched: Vec<Vec<Message>> = vec![vec![]; self.spectators.len()];
        let Table { engine: Some(engine), translators, checks, clocks, first, spectators, history, .. } = self else { return out };
        for _ in 0..10_000 {
            let a = match answer {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!("rules engine: {e}");
                    failed = true;
                    break;
                }
            };
            for e in &a.errors {
                tracing::warn!("rules engine: {e}");
            }
            for model in checks.iter_mut() { model.observe_engine(&a.messages); }
            for (part, t) in out.iter_mut().zip(translators.iter_mut()) {
                part.extend(t.events(&a.messages));
            }
            for (part, s) in watched.iter_mut().zip(spectators.iter_mut()) {
                if let Some(t) = s.translator.as_mut() {
                    part.extend(t.events(&a.messages));
                }
            }
            history.extend(a.messages.iter().cloned());
            if let Some(c) = clocks.as_mut() {
                for m in a.messages.iter().filter(|m| m["type"].as_u64() == Some(MSG_NEW_TURN)) {
                    let seat = if m["player"].as_u64() == Some(0) { *first } else { 1 - *first };
                    c.new_turn(seat, now);
                }
            }
            let last = a.messages.last();
            let (true, Some(last)) = (a.status == "await", last) else { break };
            let Some(s) = translators.iter().position(|t| t.prompted(last)) else { break };
            match translators[s].prompt(last) {
                Prompt::Answer(response) => {
                    answered += 1;
                    if repeats.stuck(last) || answered >= MAX_ANSWERS {
                        tracing::warn!("rules engine: the same prompt again and again ({last}); giving the duel up");
                        failed = true;
                        break;
                    }
                    answer = engine.respond(response)
                }
                Prompt::Client(messages) => {
                    out[s].extend(messages);
                    waiting = Some(s);
                    break;
                }
            }
        }
        for part in out.iter_mut().chain(watched.iter_mut()) {
            crate::translate::settle_positions(part);
        }
        if failed {
            // The engine died or hung: nobody could be asked anything again, so the duel ends here as a draw.
            self.tell_spectators(watched);
            for (part, more) in out.iter_mut().zip(self.abandon()) {
                part.extend(more);
            }
            return out;
        }
        let results: Vec<Option<u32>> = self.translators.iter().map(Translator::result).collect();
        let ended = results.iter().all(Option::is_some);
        if ended {
            self.clocks = None;
        }
        if let Some(c) = self.clocks.as_mut() {
            // The waited-on player's time counts once the client has played what it was just sent.
            let held = waiting.and_then(|s| out.get(s)).map_or(std::time::Duration::ZERO, |part| crate::clock::allowance(part));
            c.wait_after(waiting, now, held);
            for (view, part) in out.iter_mut().enumerate() {
                part.push(c.message(view, now));
            }
            for part in watched.iter_mut() {
                part.push(c.message(0, now));
            }
        }
        self.notes();
        // Spectators get the duel's end before the match's (duel_over).
        self.tell_spectators(watched);
        if let [Some(r0), Some(r1)] = results[..] {
            for (part, more) in out.iter_mut().zip(self.duel_over([r0, r1])) {
                part.extend(more);
            }
        }
        out
    }

    /// The engine failed (it died, hung or could not start): the duel and the match end as a draw for both seats (a
    /// no-result end the client shows), counted once, and the engine is dropped. Returns each seat's end command; the
    /// spectators are told here.
    fn abandon(&mut self) -> Out {
        self.engine = None;
        self.clocks = None;
        let end = || command(DUEL_END, [DRAW, 0, 0, 0]);
        self.tell_spectators(vec![vec![end()]; self.spectators.len()]);
        self.report([DRAW; 2]);
        [vec![end()], vec![end()]]
    }

    /// The table broke (a panic): its duel ends as a draw on both seats, as after an engine failure.
    pub fn abort(&mut self) {
        let out = self.abandon();
        for (seat, part) in self.seats.iter().zip(out) {
            for m in part {
                let _ = seat.outbox.send(m);
            }
        }
    }

    /// The server's clock check: once the player waited on is out of time, the duel ends with them losing by time out
    /// (command 2 {result, 11}), and the match goes on or ends as after any duel.
    pub fn tick(&mut self, now: Instant) {
        // The other player never connected: the one waiting is sent back, and the match ends (an event's round goes
        // on with the one who came).
        if !self.over && self.seats.len() == 1 && now.saturating_duration_since(self.opened) >= NO_SHOW {
            let waiting = self.seats[0].player_id;
            tracing::info!("duel {}: the other player never connected; cancelled", self.duel_id);
            self.over = true;
            let mut hub = self.hub.locked();
            hub.cancel_for(waiting);
            hub.end_match(self.duel_id, Some(waiting));
            return;
        }
        // The animations' allowance has passed: the clients' clocks run from here.
        if !self.over
            && let Some(c) = self.clocks.as_mut()
            && c.starting(now)
        {
            for (view, seat) in self.seats.iter().enumerate() {
                let _ = seat.outbox.send(c.message(view, now));
            }
            let watched = vec![vec![c.message(0, now)]; self.spectators.len()];
            self.tell_spectators(watched);
        }
        let Some(seat) = self.clocks.as_ref().and_then(|c| c.expired(now)) else { return };
        self.clocks = None;
        if self.over || self.seats.len() < 2 {
            return;
        }
        let mut results = [WON; 2];
        results[seat] = LOST;
        let watched = vec![vec![command(DUEL_END, [results[0], TIME_OUT, 0, 0])]; self.spectators.len()];
        self.tell_spectators(watched);
        let mut out: Out = results.map(|r| vec![command(DUEL_END, [r, TIME_OUT, 0, 0])]);
        for (part, more) in out.iter_mut().zip(self.duel_over(results)) {
            part.extend(more);
        }
        self.check_out(&mut out);
        for (s, part) in out.into_iter().enumerate() {
            for m in part {
                let _ = self.seats[s].outbox.send(m);
            }
        }
    }

    /// A duel's end: the match's end once someone has won the majority of `best_of` (the result screens), else the
    /// next duel's janken (1500; DuelResult 0x512070 sees it at the head of the queue and the match goes on), or Card
    /// Swap (1504, which DuelResult lets through the same way) for a player with a side deck. A drawn duel counts for
    /// neither.
    fn duel_over(&mut self, results: [u32; 2]) -> Out {
        self.played += 1;
        for (wins, result) in self.wins.iter_mut().zip(results) {
            *wins += (result == WON) as u32;
        }
        let needed = self.best_of / 2 + 1;
        if self.best_of <= 1 || self.wins.iter().any(|&w| w >= needed) {
            let match_results = if self.best_of <= 1 {
                results
            } else if self.wins[0] > self.wins[1] {
                [WON, LOST]
            } else {
                [LOST, WON]
            };
            self.report(match_results);
            return Out::default();
        }
        self.translators.clear();
        // Retain engine/model until deliver checks the final moves and LP, including between match duels.
        self.hands = [None; 2];
        self.picker = None;
        self.ordered = false;
        let mut out = Out::default();
        for (s, part) in out.iter_mut().enumerate() {
            self.swapping[s] = !self.seats[s].entry.deck.side.is_empty();
            part.push(command(if self.swapping[s] { CARD_SWAP } else { JANKEN_BEGIN }, [0; 4]));
        }
        out
    }

    /// A seat's Card Swap is done (0xE, after the front kept the swapped deck as its match entry, 0x1F4F update_type
    /// 2): the next duel uses that deck when it holds the same cards (a swap only moves cards between the side deck and
    /// the others), and the seat's janken begins (1500).
    pub fn swapped(&mut self, seat: usize) -> Out {
        let mut out = Out::default();
        if !std::mem::take(&mut self.swapping[seat]) || self.over {
            return out;
        }
        let entry = self.hub.locked().duel_entry(self.seats[seat].player_id);
        let sorted = |d: &crate::front::Deck| {
            let mut cards: Vec<u16> = d.cards().collect();
            cards.sort_unstable();
            cards
        };
        match entry.filter(|e| e.duel_id == Some(self.duel_id)) {
            Some(e) if sorted(&e.deck) == sorted(&self.seats[seat].entry.deck) => self.seats[seat].entry.deck = e.deck,
            _ => tracing::warn!("duel {}: seat {seat}'s swapped deck isn't its deck; kept the one it had", self.duel_id),
        }
        out[seat].push(command(JANKEN_BEGIN, [0; 4]));
        out
    }

    /// The match's end on both front connections (the result screen), counted against each other, and seat 0's
    /// result to the spectators (after the duel's end they were sent).
    fn report(&mut self, results: [u32; 2]) {
        if self.over || self.seats.len() < 2 {
            return;
        }
        self.over = true;
        let mut hub = self.hub.locked();
        let replay = self.replay(&hub);
        for (s, result) in results.into_iter().enumerate() {
            let (me, them) = (self.seats[s].player_id, self.seats[1 - s].player_id);
            let match_type = if self.best_of > 1 { MATCH_MODE } else { SINGLE_MODE };
            let ended = Ended { player_id: me, opponent: them, npc: false, duel_id: self.duel_id, match_type, result, replay: replay.clone() };
            hub.finish_duel(&ended);
        }
        let winner = results.iter().position(|&r| r == WON).map(|s| self.seats[s].player_id);
        hub.end_match(self.duel_id, winner);
        drop(hub);
        // Delivery checks the final state before releasing the engine process.
        self.tell_spectators(vec![vec![command(MATCH_END, [results[0], 0, 0, 0])]; self.spectators.len()]);
    }

    /// A client's response, then the engine onwards.
    fn respond(&mut self, response: Option<Json>) -> Out {
        let (Some(response), Some(engine)) = (response, self.engine.as_mut()) else { return Out::default() };
        let answer = engine.respond(response);
        self.advance(answer)
    }

    pub fn card_command(&mut self, seat: usize, cmd: CardCommand) -> Out {
        let prompt = self.translators.get_mut(seat).and_then(|t| t.command(cmd));
        self.prompted(seat, prompt)
    }

    pub fn phase(&mut self, seat: usize, button: u32) -> Out {
        let response = self.translators.get_mut(seat).and_then(|t| t.phase(button));
        self.respond(response)
    }

    pub fn pick(&mut self, seat: usize, index: u32) {
        if let Some(t) = self.translators.get_mut(seat) {
            t.pick(index);
        }
    }

    pub fn dialog_result(&mut self, seat: usize, n: u32) -> Out {
        let prompt = self.translators.get_mut(seat).and_then(|t| t.dialog_result(n));
        self.prompted(seat, prompt)
    }

    /// A translator's answer to a client: the engine onwards, or more for that client.
    fn prompted(&mut self, seat: usize, prompt: Option<Prompt>) -> Out {
        match prompt {
            Some(Prompt::Answer(response)) => self.respond(Some(response)),
            Some(Prompt::Client(messages)) => {
                self.notes();
                let mut out = Out::default();
                out[seat] = messages;
                out
            }
            None => Out::default(),
        }
    }

    /// What the duel windows can't say (an option list's texts), to each player's chat window now.
    fn notes(&mut self) {
        for (s, translator) in self.translators.iter_mut().enumerate() {
            let notes = translator.take_notes();
            let Some(seat) = self.seats.get(s).filter(|_| !notes.is_empty()) else { continue };
            if let Some((_, outbox)) = self.hub.locked().front_of(seat.player_id) {
                for note in notes {
                    let _ = outbox.send(crate::banter::note_line(&note));
                }
            }
        }
    }

    /// A client left. With both seats taken and the match not over, the other one is told at once (it wins by
    /// disconnection) wherever the match stood: in the janken, the turn order choice, Card Swap, a duel. Only a duel that
    /// began is counted; before that the other client gets its result screen with the records unchanged.
    pub fn leave(&mut self, seat: usize) {
        if !self.over && self.seats.len() == 2 {
            let other = 1 - seat;
            let _ = self.seats[other].outbox.send(command(DUEL_END, [WON, DISCONNECTED, 0, 0]));
            let seat0 = if seat == 0 { LOST } else { WON };
            self.tell_spectators(vec![vec![command(DUEL_END, [seat0, DISCONNECTED, 0, 0])]; self.spectators.len()]);
            if self.engine.is_some() || self.played > 0 {
                let mut results = [WON; 2];
                results[seat] = LOST;
                self.report(results);
            } else {
                // Both seats taken and no duel yet: the other client is on the janken or turn order screen.
                self.hub.locked().left_at_janken(self.seats[other].player_id, self.seats[seat].player_id);
            }
        }
        if !self.over {
            self.over = true;
            self.engine = None;
            self.hub.locked().end_match(self.duel_id, None);
        }
    }

    /// Sends the other seat its part and returns this seat's. With the duel check on, each seat's view follows its part
    /// and is compared with the engine: an ERROR names where they differ.
    pub fn deliver(&mut self, seat: usize, mut out: Out) -> Vec<Message> {
        self.check_out(&mut out);
        let [mut mine, mut theirs] = out;
        if seat == 1 {
            std::mem::swap(&mut mine, &mut theirs);
        }
        if let Some(other) = self.seats.get(1 - seat) {
            for m in theirs {
                let _ = other.outbox.send(m);
            }
        }
        mine
    }

    fn check_out(&mut self, out: &mut Out) {
        if let (Some(engine), [c0, c1]) = (self.engine.as_mut(), &mut self.checks[..]) {
            c0.apply_all(&out[0]);
            c1.apply_all(&out[1]);
            for (model, part) in [&mut *c0, &mut *c1].into_iter().zip(out.iter_mut()) {
                model.checkpoint_into(part);
            }
            let mut failed = false;
            match engine.state() {
                Ok(state) => {
                    for (s, c) in [c0, c1].into_iter().enumerate() {
                        let problems = c.check(&state);
                        if !problems.is_empty() {
                            tracing::error!("duel check (duel {}, seat {s}): {}", self.duel_id, problems.join(" | "));
                            failed = true;
                        }
                    }
                }
                Err(e) => tracing::error!("duel check: no engine state: {e}"),
            }
            // Once per duel: every later step would repeat the same difference.
            if failed {
                self.checks.clear();
            }
        }
        if self.over || !self.ordered {
            self.engine = None;
            self.checks.clear();
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn a_match_keeps_its_final_model_until_the_last_batch_is_delivered() {
        let world = Arc::new(World::new(vec![]));
        let mut table = Table::new(world, Arc::new(Mutex::new(Hub::default())), 1, 1);
        table.best_of = 3;
        for player_id in [1, 2] {
            let (outbox, _) = tokio::sync::mpsc::unbounded_channel();
            table.seats.push(Seat { player_id, entry: DuelEntry::default(), outbox });
        }
        table.checks = (0..2).map(|_| crate::duelcheck::ClientModel::new(0, Default::default())).collect();
        table.duel_over([WON, LOST]);
        assert_eq!(table.checks.len(), 2, "the final LP/moves still need checking before teardown");
    }
}
