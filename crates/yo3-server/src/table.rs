//! A duel between two players: both clients' duel connections share one table, which plays the janken between their
//! hands and runs one rules engine for both. Each client sees itself as player 0 through its own translator; an engine
//! prompt goes to the client it asks, and each client's commands reach the other through that connection's outbox.
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
use crate::front::{DuelEntry, Hub, Outbox, World, duel_over};
use crate::translate::{DUEL_END, LOST, Prompt, Translator, WON};

/// The usual starting LP (the server's own unless --duel-lp changes it).
const DEFAULT_LP: u32 = 8000;
/// The duel-end reasons "has ran out of time" and "has disconnected" (text 1999 + 11, + 14).
const TIME_OUT: u32 = 11;
const DISCONNECTED: u32 = 14;
/// The engine's MSG_NEW_TURN {player}.
const MSG_NEW_TURN: u64 = 40;
/// Card Swap between a match's duels (DuelManager 0x50FC00 takes it to 0x50FDF0), instead of the next janken's 1500.
const CARD_SWAP: u32 = 1504;
/// A match's end for a spectator {match result: 1 won, 2 lost, 3 draw}: its DuelResult (0x512070) reads it at the
/// head of its queue (MainGamen +0x820) where a player's reads the front's 0x2009.
const MATCH_END: u32 = 1505;

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
    over: bool,
    rng: u64,
}

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
            wins: [0; 2],
            best_of: 1,
            first: 0,
            clocks: None,
            spectators: vec![],
            lp: DEFAULT_LP,
            history: vec![],
            janken: [0; 3],
            swapping: [false; 2],
            over: false,
            rng: seed | 1,
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
        if self.picker != Some(seat) || self.engine.is_some() {
            return out;
        }
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
                tracing::warn!("rules engine not started: {e}");
                return Out::default();
            }
        };
        // A hosted duel's starting LP (Create Duel's lp_start), unless the server's was set to test (--duel-lp).
        let hosted = self.seats.iter().find_map(|s| s.entry.lp);
        let lp = hosted.filter(|_| self.world.start_lp == DEFAULT_LP).unwrap_or(self.world.start_lp);
        let answer = engine.new_duel(crate::duel::next(&mut self.rng), lp, [(&m0, &e0), (&m1, &e1)]);
        self.engine = Some(engine);
        // A hosted duel's time_turn and time_total, else Create Duel's defaults.
        let (turn, total) = self.seats.iter().find_map(|s| s.entry.times).unwrap_or((DEFAULT_TURN, DEFAULT_TOTAL));
        self.first = first;
        self.clocks = Clocks::new(turn, total);
        self.lp = lp;
        self.history.clear();
        let mut out = Out::default();
        for (s, part) in out.iter_mut().enumerate() {
            let (mine, theirs) = (&self.seats[s].entry.deck, &self.seats[1 - s].entry.deck);
            let order = if s == first { CLIENT } else { CPU };
            let (mut translator, mut table) = Translator::against_player(order, [&mine.main, &theirs.main], self.world.cards.clone());
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
        let mut watched: Vec<Vec<Message>> = vec![vec![]; self.spectators.len()];
        let Table { engine: Some(engine), translators, clocks, first, spectators, history, .. } = self else { return out };
        for _ in 0..10_000 {
            let a = match answer {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!("rules engine: {e}");
                    break;
                }
            };
            for e in &a.errors {
                tracing::warn!("rules engine: {e}");
            }
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
                Prompt::Answer(response) => answer = engine.respond(response),
                Prompt::Client(messages) => {
                    out[s].extend(messages);
                    waiting = Some(s);
                    break;
                }
            }
        }
        let results: Vec<Option<u32>> = self.translators.iter().map(Translator::result).collect();
        let ended = results.iter().all(Option::is_some);
        if ended {
            self.clocks = None;
        }
        if let Some(c) = self.clocks.as_mut() {
            c.wait_on(waiting, now);
            for (view, part) in out.iter_mut().enumerate() {
                part.push(c.message(view, now));
            }
            for part in watched.iter_mut() {
                part.push(c.message(0, now));
            }
        }
        // Spectators get the duel's end before the match's (duel_over).
        self.tell_spectators(watched);
        if let [Some(r0), Some(r1)] = results[..] {
            for (part, more) in out.iter_mut().zip(self.duel_over([r0, r1])) {
                part.extend(more);
            }
        }
        out
    }

    /// The server's clock check: once the player waited on is out of time, the duel ends with them losing by time out
    /// (command 2 {result, 11}), and the match goes on or ends as after any duel.
    pub fn tick(&mut self, now: Instant) {
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
        self.engine = None;
        self.translators.clear();
        self.hands = [None; 2];
        self.picker = None;
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
        let entry = self.hub.lock().expect("hub lock").duel_entry(self.seats[seat].player_id);
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
        let mut hub = self.hub.lock().expect("hub lock");
        for (s, result) in results.into_iter().enumerate() {
            let (me, them) = (self.seats[s].player_id, self.seats[1 - s].player_id);
            let record = hub.record_result(me, them, result);
            hub.push(me, &duel_over(me, result, record));
        }
        let winner = results.iter().position(|&r| r == WON).map(|s| self.seats[s].player_id);
        hub.end_match(self.duel_id, winner);
        drop(hub);
        self.tell_spectators(vec![vec![command(MATCH_END, [results[0], 0, 0, 0])]; self.spectators.len()]);
    }

    /// A client's response, then the engine onwards.
    fn respond(&mut self, response: Option<Json>) -> Out {
        let (Some(response), Some(engine)) = (response, self.engine.as_mut()) else { return Out::default() };
        let answer = engine.respond(response);
        self.advance(answer)
    }

    pub fn card_command(&mut self, seat: usize, cmd: CardCommand) -> Out {
        let response = self.translators.get_mut(seat).and_then(|t| t.answer(cmd));
        self.respond(response)
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
        match self.translators.get_mut(seat).and_then(|t| t.dialog_result(n)) {
            Some(Prompt::Answer(response)) => self.respond(Some(response)),
            Some(Prompt::Client(messages)) => {
                let mut out = Out::default();
                out[seat] = messages;
                out
            }
            None => Out::default(),
        }
    }

    /// A client left: during the duel, or between a match's duels, the other wins by disconnection.
    pub fn leave(&mut self, seat: usize) {
        let started = self.engine.is_some() || self.wins.iter().any(|&w| w > 0);
        if started && !self.over && self.seats.len() == 2 {
            let other = 1 - seat;
            let _ = self.seats[other].outbox.send(command(DUEL_END, [WON, DISCONNECTED, 0, 0]));
            let seat0 = if seat == 0 { LOST } else { WON };
            self.tell_spectators(vec![vec![command(DUEL_END, [seat0, DISCONNECTED, 0, 0])]; self.spectators.len()]);
            let mut results = [WON; 2];
            results[seat] = LOST;
            self.report(results);
        }
        if !self.over {
            self.over = true;
            self.hub.lock().expect("hub lock").end_match(self.duel_id, None);
        }
    }

    /// Sends the other seat its part and returns this seat's.
    pub fn deliver(&self, seat: usize, out: Out) -> Vec<Message> {
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
}
