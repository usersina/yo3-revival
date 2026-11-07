//! Duel server: the connection DuelManager (0x50F690) opens at the address in the 0x22C4 push.
//!
//! Both directions are key/value messages carrying a duelRequestName; see "Duel connection" in
//! `docs/protocol.md`. The server drives the duel with engine commands, which the client's scenes
//! pop one at a time. So far it plays the janken (who goes first) as the CPU opponent and opens the
//! duel (setup, both decks, LP, the opening hands, the first phase); the duel itself needs a rules
//! engine (M5).
use std::sync::{Arc, Mutex};

use yo3_proto::{Message, Packet, Value};

use crate::engine::{Answer, Engine};
use crate::front::{Deck, DuelEntry, Hub, World};
use crate::translate::{Prompt, Translator};

/// Request id of the server's duel messages. A guess: the pump (0x5233A0) reads only
/// duelRequestName, and treats a negative id as a lost connection.
pub const SERVER_DUEL: u32 = 0x200D;
/// The client's first message on the duel connection, {player_id, autoSet} (0x50FAD1).
const ENTER: u32 = 0x200A;
/// Every later client message (0x49F990), told apart by duelRequestName.
const CLIENT_DUEL: u32 = 0x200C;
/// duelRequestName of an engine command {id, param1..param4} (pump case 0x11).
const ENGINE_COMMAND: u32 = 0x11;
/// duelRequestName of the duel setup (pump case 0x15).
const DUEL_SETUP: u32 = 0x15;
/// duelRequestName of the command map (pump case 0x12).
const COMMAND_DATA: u32 = 0x12;
/// Client duelRequestName of a card command (0x520FD0) and of a phase button {phase} (0x521180).
const CARD_COMMAND: u32 = 2;
const PHASE_BUTTON: u32 = 3;
/// Client duelRequestNames of a closing dialog: its result {DLGResult, debug} (0x521A70), and for the card list
/// (0x4C74F0) first the first pick {SetIndex} (0x521BC0) and each pick {index, data: its order} (0x521CC0).
const DIALOG_RESULT: u32 = 4;
const PICK: u32 = 7;
/// Client duelRequestNames: the janken hand {selectHand} (0x4F1330), first or second {choice}
/// (0x4F1430).
const SELECT_HAND: u32 = 0xC;
const SELECT_FIRST: u32 = 0xD;

/// JankenScene commands (0x518400): begin, resolve, who chooses, chosen (its step leaves the scene by itself,
/// 0x519230). 1505 there is the match's end {match result, ...} (0x5185A3), not part of the janken.
const JANKEN_BEGIN: u32 = 1500;
const JANKEN_RESOLVE: u32 = 1501;
const FIRST_BEGIN: u32 = 1502;
const FIRST_RESOLVE: u32 = 1503;

/// Turn order as the janken commands give it: 1 first, 2 second.
const FIRST: u32 = 1;
const SECOND: u32 = 2;

/// Duel commands: start (DuelScene 0x514290), set LP {player, LP} (DuelCockpit 0x502EB0), phase
/// {player, phase} (DuelScene, DuelCockpit).
const DUEL_START: u32 = 1;
pub(crate) const SET_LP: u32 = 0xF;
/// Change a player's LP {player, signed change} (DuelCockpit 0x502EB0).
pub(crate) const CHANGE_LP: u32 = 0x10;
pub(crate) const PHASE: u32 = 5;
/// Turn change {the player whose turn starts} (DuelScene; the pop counts the turn).
pub(crate) const TURN: u32 = 6;
/// Wait for a player's input {mode, player}: DuelFieldManager highlights the player's cards in the command map,
/// and modes 2 and 3 turn the phase panel on (0x504890).
pub(crate) const INPUT: u32 = 4;
const DRAW_PHASE: u32 = 0;
/// Card commands: set a card in the engine's card table {uid, row, 0, table place} (0x521DF0), and
/// move a card on screen {uid | row << 16, from, to} (DuelCardManager 0x4FF6A2).
pub(crate) const CARD: u32 = 1000;
pub(crate) const MOVE: u32 = 0x1A;
/// Zones (the client's numbering): 0-4 monsters, 5-9 Spell/Trap, then these.
pub(crate) const HAND: u32 = 0xB;
pub(crate) const DECK: u32 = 0xD;
const OPENING_HAND: usize = 5;
/// Engine players: the client is player 0 (setup PlayerNum), the CPU player 1.
pub(crate) const CLIENT: u32 = 0;
pub(crate) const CPU: u32 = 1;

/// A place as the card table keys it (0x523DF0).
pub(crate) fn table_place(player: u32, zone: u32, index: usize) -> u32 {
    player | zone << 8 | (index as u32) << 16
}

/// A place as card moves give it (0x522A50 counts by bit 0 and bits 1-5). The index bits are a guess
/// (Online 2's packing).
pub(crate) fn move_place(player: u32, zone: u32, index: usize) -> u32 {
    player | zone << 1 | (index as u32) << 6
}

/// An engine command, as pump case 0x11 reads it.
pub fn command(id: u32, params: [u32; 4]) -> Message {
    let m = Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(ENGINE_COMMAND)).with("id", Value::U32(id));
    params.iter().zip(["param1", "param2", "param3", "param4"]).fold(m, |m, (v, k)| m.with(k, Value::U32(*v)))
}

/// The commands each card offers, as {place, command bits} (pump case 0x12), then the other flags as {key,
/// value} bytes (the phase panel reads key = button 0 DP .. 5 EP, value != 0 = enabled; 0x501C00). Places are
/// `player | zone << 8 | index << 16`; bit n is the menu entry n (see [`CardCommand`]).
pub fn command_data(cards: &[(u32, u32)], flags: &[(u8, u8)]) -> Message {
    let mut data: Vec<u8> = cards.iter().flat_map(|(place, bits)| place.to_be_bytes().into_iter().chain(bits.to_be_bytes())).collect();
    data.extend(flags.iter().flat_map(|(k, v)| [*k, *v]));
    Message::reply(SERVER_DUEL)
        .with("duelRequestName", Value::U32(COMMAND_DATA))
        .with("comIDsize", Value::U32(cards.len() as u32))
        .with("helpIDsize", Value::U32(0))
        .with("otherFlagSize", Value::U32(flags.len() as u32))
        .with("COMIDDATA", Value::Bin(data))
}

/// A command the player chose from a card's menu (client duelRequestName 2). `command` is the bit
/// number: 0 Attack, 1 See Details, 2 Special Summon, 3 Activate Effect, 4 Summon, 5 Flip Summon,
/// 6 and 7 Set, 8 to Attack Position, 9 to Defence Position, 10 Surrender, 11 Confirm, 12 Draw,
/// 13 Shuffle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardCommand {
    pub command: u32,
    pub player: u32,
    pub zone: u32,
    pub index: u32,
}

impl CardCommand {
    pub fn from_message(m: &Message) -> Option<CardCommand> {
        let int = |k| m.get(k).and_then(Value::as_u32);
        if m.request() != Some(CLIENT_DUEL) || int("duelRequestName") != Some(CARD_COMMAND) {
            return None;
        }
        Some(CardCommand { command: int("CommandID")?, player: int("player")?, zone: int("position")?, index: int("index")? })
    }
}

/// One client's duel connection, against the server as the CPU.
pub struct DuelSession {
    world: Arc<World>,
    hub: Arc<Mutex<Hub>>,
    /// What the player brought (found by the player_id of 0x200A).
    entry: Option<DuelEntry>,
    /// The player, from 0x200A.
    player_id: Option<u32>,
    /// The rules engine running this duel, once it started.
    engine: Option<(Engine, Translator)>,
    /// The result went to the player's front connection.
    reported: bool,
    rng: u64,
}

impl DuelSession {
    /// A session whose CPU choices follow `seed`.
    pub fn new(world: Arc<World>, hub: Arc<Mutex<Hub>>, seed: u64) -> DuelSession {
        DuelSession { world, hub, entry: None, player_id: None, engine: None, reported: false, rng: seed | 1 }
    }

    /// Handles one client packet and returns the messages to send, in order.
    pub fn handle(&mut self, packet: &Packet) -> Vec<Message> {
        let Packet::Message(msg) = packet else { return vec![] };
        let int = |k| msg.get(k).and_then(Value::as_u32);
        match (msg.request(), int("duelRequestName")) {
            (Some(ENTER), _) => {
                self.player_id = int("player_id");
                let mut entry = self.player_id.and_then(|id| self.hub.lock().expect("hub lock").duel_entry(id));
                if let (Some(entry), Some(rows)) = (entry.as_mut(), &self.world.test_deck) {
                    entry.deck.main = rows.iter().copied().cycle().take(40).collect();
                }
                self.entry = entry;
                vec![command(JANKEN_BEGIN, [0; 4])]
            }
            (Some(CLIENT_DUEL), Some(SELECT_HAND)) => self.janken(int("selectHand").unwrap_or(0) % 3),
            (Some(CLIENT_DUEL), Some(CARD_COMMAND)) => {
                let answer = CardCommand::from_message(msg)
                    .and_then(|cmd| self.engine.as_mut().and_then(|(_, t)| t.answer(cmd)))
                    .and_then(|r| self.engine.as_mut().map(|(e, _)| e.respond(r)));
                match answer {
                    Some(answer) => self.advance(answer),
                    None => vec![],
                }
            }
            (Some(CLIENT_DUEL), Some(PHASE_BUTTON)) => {
                let answer = int("phase")
                    .and_then(|phase| self.engine.as_mut().and_then(|(_, t)| t.phase(phase)))
                    .and_then(|r| self.engine.as_mut().map(|(e, _)| e.respond(r)));
                match answer {
                    Some(answer) => self.advance(answer),
                    None => vec![],
                }
            }
            (Some(CLIENT_DUEL), Some(PICK)) => {
                if let (Some((_, t)), Some(index)) = (self.engine.as_mut(), int("index")) {
                    t.pick(index);
                }
                vec![]
            }
            (Some(CLIENT_DUEL), Some(DIALOG_RESULT)) => {
                let answer = int("DLGResult")
                    .and_then(|n| self.engine.as_mut().and_then(|(_, t)| t.dialog_result(n)))
                    .and_then(|r| self.engine.as_mut().map(|(e, _)| e.respond(r)));
                match answer {
                    Some(answer) => self.advance(answer),
                    None => vec![],
                }
            }
            (Some(CLIENT_DUEL), Some(SELECT_FIRST)) => {
                let order = if int("choice") == Some(SECOND) { SECOND } else { FIRST };
                self.turn_order(order)
            }
            _ => vec![],
        }
    }

    /// The CPU's hand against the player's (0 rock, 1 scissors, 2 paper), and what follows: nothing
    /// on a draw (the scene asks for another hand by itself, 0x518ED0), the player's choice of turn
    /// order on a win, the CPU going first on a loss.
    fn janken(&mut self, mine: u32) -> Vec<Message> {
        let theirs = (self.next() % 3) as u32;
        // Each hand beats the next one: rock > scissors > paper > rock.
        let result = if theirs == mine { 0 } else if theirs == (mine + 1) % 3 { 1 } else { 2 };
        let mut out = vec![command(JANKEN_RESOLVE, [result, theirs, mine, 0])];
        match result {
            0 => {}
            1 => out.push(command(FIRST_BEGIN, [1, 0, 0, 0])),
            _ => {
                out.push(command(FIRST_BEGIN, [0; 4]));
                out.extend(self.turn_order(SECOND));
            }
        }
        out
    }

    /// The player's turn order, then the duel's opening.
    fn turn_order(&mut self, order: u32) -> Vec<Message> {
        let mut out = vec![command(FIRST_RESOLVE, [order, 0, 0, 0])];
        let first = if order == FIRST { CLIENT } else { CPU };
        let opening = match self.world.engine_dir.clone() {
            Some(dir) => self.engine_opening(first, &dir),
            None => None,
        };
        out.extend(opening.unwrap_or_else(|| self.opening(first)));
        out
    }

    /// The setup message: who goes first, and both decks' sizes.
    fn setup(first: u32, mine: &Deck, theirs: &Deck) -> Message {
        let len = |cards: &Vec<u16>| cards.len() as u32;
        // PlayerType: 0 for the client, 1 for the CPU (meaning unknown, a guess).
        [
            ("FirstPlayer", first),
            ("PlayerNum", CLIENT),
            ("PlayerType001", 0),
            ("PlayerType002", 1),
            ("MainDeckNum001", len(&mine.main)),
            ("MainDeckNum002", len(&theirs.main)),
            ("ExtraDeckNum001", len(&mine.extra)),
            ("ExtraDeckNum002", len(&theirs.extra)),
        ]
        .into_iter()
        .fold(Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(DUEL_SETUP)), |m, (k, v)| m.with(k, Value::U32(v)))
    }

    /// The opening with the rules engine: setup, both decks in the card table, start and LP, then whatever the
    /// engine does up to the client's first choice (its draws, turns and phases). None if the engine won't start.
    fn engine_opening(&mut self, first: u32, dir: &std::path::Path) -> Option<Vec<Message>> {
        let entry = self.entry.clone()?;
        let npc = entry.npc_id.and_then(|id| self.world.npc_decks.get(&id)).cloned()?;
        let decks = [&entry.deck, &npc];
        let cids = |rows: &[u16]| rows.iter().filter_map(|r| self.world.cards.get(r).map(|c| c.cid)).collect::<Vec<u16>>();
        // Engine team 0 goes first.
        let teams = if first == CLIENT { [decks[0], decks[1]] } else { [decks[1], decks[0]] };
        let (mut m0, e0, mut m1, e1) = (cids(&teams[0].main), cids(&teams[0].extra), cids(&teams[1].main), cids(&teams[1].extra));
        // The core deals in the order the cards were added (each new card goes on top; live, every duel opened
        // with the same hand), so the decks are shuffled here.
        self.shuffle(&mut m0);
        self.shuffle(&mut m1);
        let mut engine = Engine::start(dir).map_err(|e| tracing::warn!("rules engine not started: {e}")).ok()?;
        let seed = self.next();
        let lp = self.world.start_lp;
        let answer = engine.new_duel(seed, lp, [(&m0, &e0), (&m1, &e1)]);
        let (translator, table) = Translator::new(first, [&entry.deck.main, &npc.main], self.world.cards.clone());
        self.engine = Some((engine, translator));
        let mut out = vec![Self::setup(first, &entry.deck, &npc)];
        out.extend(table);
        out.extend([
            command(DUEL_START, [0; 4]),
            command(SET_LP, [CLIENT, lp, 0, 0]),
            command(SET_LP, [CPU, lp, 0, 0]),
        ]);
        out.extend(self.advance(answer));
        Some(out)
    }

    /// Runs the engine from `answer`: its events become client commands, the CPU's prompts (and those the client
    /// has no window for yet) are answered, until the client must choose or the duel ends.
    fn advance(&mut self, mut answer: std::io::Result<Answer>) -> Vec<Message> {
        let Some((engine, translator)) = self.engine.as_mut() else { return vec![] };
        let mut out = vec![];
        // Each round answers one prompt; a duel needs far fewer between two of the client's choices.
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
            out.extend(translator.events(&a.messages));
            let last = a.messages.last();
            let (true, Some(last)) = (a.status == "await", last) else { break };
            match translator.prompt(last) {
                Prompt::Answer(response) => answer = engine.respond(response),
                Prompt::Client(messages) => {
                    out.extend(messages);
                    break;
                }
            }
        }
        if let (Some(result), Some(player_id), false) = (translator.result(), self.player_id, self.reported) {
            self.reported = true;
            let opponent = self.entry.as_ref().and_then(|e| e.npc_id).unwrap_or(0);
            let mut hub = self.hub.lock().expect("hub lock");
            let record = hub.record_result(player_id, opponent, result);
            hub.push(player_id, &crate::front::duel_over(player_id, result, record));
        }
        out
    }

    /// Setup (both decks' sizes), both shuffled decks in the card table, the duel start, both players'
    /// LP, five cards each (the first player first), then the first player's Draw Phase. Nothing
    /// without the player's deck and an NPC to play.
    fn opening(&mut self, first: u32) -> Vec<Message> {
        let Some(entry) = self.entry.clone() else { return vec![] };
        let Some(npc) = entry.npc_id.and_then(|id| self.world.npc_decks.get(&id)).cloned() else { return vec![] };
        let mut out = vec![Self::setup(first, &entry.deck, &npc)];
        // uids 1.. in table order: the client's deck, then the CPU's.
        let mut decks = [entry.deck.main.clone(), npc.main.clone()];
        let mut uids: [Vec<u32>; 2] = [vec![], vec![]];
        for (player, deck) in decks.iter_mut().enumerate() {
            self.shuffle(deck);
            for (i, &row) in deck.iter().enumerate() {
                let uid = uids[0].len() as u32 + uids[1].len() as u32 + 1;
                uids[player].push(uid);
                out.push(command(CARD, [uid, row as u32, 0, table_place(player as u32, DECK, i)]));
            }
        }
        out.extend([
            command(DUEL_START, [0; 4]),
            command(SET_LP, [CLIENT, self.world.start_lp, 0, 0]),
            command(SET_LP, [CPU, self.world.start_lp, 0, 0]),
        ]);
        for player in [first, 1 - first] {
            let p = player as usize;
            for i in 0..OPENING_HAND.min(decks[p].len()) {
                let (uid, row) = (uids[p][i], decks[p][i] as u32);
                out.push(command(CARD, [uid, row, 0, table_place(player, HAND, i)]));
                out.push(command(MOVE, [uid | row << 16, move_place(player, DECK, 0), move_place(player, HAND, i), 0]));
            }
        }
        out.push(command(PHASE, [first, DRAW_PHASE, 0, 0]));
        out
    }

    /// Fisher-Yates.
    fn shuffle(&mut self, cards: &mut [u16]) {
        for i in (1..cards.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            cards.swap(i, j);
        }
    }

    /// xorshift64*
    fn next(&mut self) -> u64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}
