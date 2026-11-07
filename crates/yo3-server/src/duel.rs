//! Duel server: the connection DuelManager (0x50F690) opens at the address in the 0x22C4 push.
//!
//! Both directions are key/value messages carrying a duelRequestName; see "Duel connection" in
//! `docs/protocol.md`. The server drives the duel with engine commands, which the client's scenes
//! pop one at a time. So far it plays the janken (who goes first) as the CPU opponent and opens the
//! duel (setup, both decks, LP, the opening hands, the first phase); the duel itself needs a rules
//! engine (M5).
use std::sync::{Arc, Mutex};

use yo3_proto::{Message, Packet, Value};

use crate::front::{DuelEntry, Hub, World};

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
/// Client duelRequestNames: the janken hand {selectHand} (0x4F1330), first or second {choice}
/// (0x4F1430).
const SELECT_HAND: u32 = 0xC;
const SELECT_FIRST: u32 = 0xD;

/// JankenScene commands (0x518400): begin, resolve, who chooses, chosen, end.
const JANKEN_BEGIN: u32 = 1500;
const JANKEN_RESOLVE: u32 = 1501;
const FIRST_BEGIN: u32 = 1502;
const FIRST_RESOLVE: u32 = 1503;
const JANKEN_END: u32 = 1505;

/// Turn order as the janken commands give it: 1 first, 2 second.
const FIRST: u32 = 1;
const SECOND: u32 = 2;

/// Duel commands: start (DuelScene 0x514290), set LP {player, LP} (DuelCockpit 0x502EB0), phase
/// {player, phase} (DuelScene, DuelCockpit).
const DUEL_START: u32 = 1;
const SET_LP: u32 = 0xF;
const PHASE: u32 = 5;
const DRAW_PHASE: u32 = 0;
const START_LP: u32 = 8000;
/// Card commands: set a card in the engine's card table {uid, row, 0, table place} (0x521DF0), and
/// move a card on screen {uid | row << 16, from, to} (DuelCardManager 0x4FF6A2).
const CARD: u32 = 1000;
const MOVE: u32 = 0x1A;
/// Zones (the client's numbering): 0-4 monsters, 5-9 Spell/Trap, then these.
const HAND: u32 = 0xB;
const DECK: u32 = 0xD;
const OPENING_HAND: usize = 5;
/// Engine players: the client is player 0 (setup PlayerNum), the CPU player 1.
const CLIENT: u32 = 0;
const CPU: u32 = 1;

/// A place as the card table keys it (0x523DF0).
fn table_place(player: u32, zone: u32, index: usize) -> u32 {
    player | zone << 8 | (index as u32) << 16
}

/// A place as card moves give it (0x522A50 counts by bit 0 and bits 1-5). The index bits are a guess
/// (Online 2's packing).
fn move_place(player: u32, zone: u32, index: usize) -> u32 {
    player | zone << 1 | (index as u32) << 6
}

/// An engine command, as pump case 0x11 reads it.
pub fn command(id: u32, params: [u32; 4]) -> Message {
    let m = Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(ENGINE_COMMAND)).with("id", Value::U32(id));
    params.iter().zip(["param1", "param2", "param3", "param4"]).fold(m, |m, (v, k)| m.with(k, Value::U32(*v)))
}

/// One client's duel connection, against the server as the CPU.
pub struct DuelSession {
    world: Arc<World>,
    hub: Arc<Mutex<Hub>>,
    /// What the player brought (found by the player_id of 0x200A).
    entry: Option<DuelEntry>,
    rng: u64,
}

impl DuelSession {
    /// A session whose CPU choices follow `seed`.
    pub fn new(world: Arc<World>, hub: Arc<Mutex<Hub>>, seed: u64) -> DuelSession {
        DuelSession { world, hub, entry: None, rng: seed | 1 }
    }

    /// Handles one client packet and returns the messages to send, in order.
    pub fn handle(&mut self, packet: &Packet) -> Vec<Message> {
        let Packet::Message(msg) = packet else { return vec![] };
        let int = |k| msg.get(k).and_then(Value::as_u32);
        match (msg.request(), int("duelRequestName")) {
            (Some(ENTER), _) => {
                let entry = int("player_id").and_then(|id| self.hub.lock().expect("hub lock").duel_entry(id));
                self.entry = entry;
                vec![command(JANKEN_BEGIN, [0; 4])]
            }
            (Some(CLIENT_DUEL), Some(SELECT_HAND)) => self.janken(int("selectHand").unwrap_or(0) % 3),
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

    /// The player's turn order, the end of the janken, then the duel's opening.
    fn turn_order(&mut self, order: u32) -> Vec<Message> {
        let mut out = vec![command(FIRST_RESOLVE, [order, 0, 0, 0]), command(JANKEN_END, [order, 0, 0, 0])];
        out.extend(self.opening(if order == FIRST { CLIENT } else { CPU }));
        out
    }

    /// Setup (both decks' sizes), both shuffled decks in the card table, the duel start, both players'
    /// LP, five cards each (the first player first), then the first player's Draw Phase. Nothing
    /// without the player's deck and an NPC to play.
    fn opening(&mut self, first: u32) -> Vec<Message> {
        let Some(entry) = self.entry.clone() else { return vec![] };
        let Some(npc) = entry.npc_id.and_then(|id| self.world.npc_decks.get(&id)).cloned() else { return vec![] };
        let len = |cards: &Vec<u16>| cards.len() as u32;
        // PlayerType: 0 for the client, 1 for the CPU (meaning unknown, a guess).
        let setup = [
            ("FirstPlayer", first),
            ("PlayerNum", CLIENT),
            ("PlayerType001", 0),
            ("PlayerType002", 1),
            ("MainDeckNum001", len(&entry.deck.main)),
            ("MainDeckNum002", len(&npc.main)),
            ("ExtraDeckNum001", len(&entry.deck.extra)),
            ("ExtraDeckNum002", len(&npc.extra)),
        ]
        .into_iter()
        .fold(Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(DUEL_SETUP)), |m, (k, v)| m.with(k, Value::U32(v)));
        let mut out = vec![setup];
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
            command(SET_LP, [CLIENT, START_LP, 0, 0]),
            command(SET_LP, [CPU, START_LP, 0, 0]),
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
