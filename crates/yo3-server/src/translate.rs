//! Translator between the rules engine (ocgcore messages, see `engine.rs`) and the client's duel stream
//! (engine commands, command maps; see "Duel connection" in `docs/protocol.md`).
//!
//! Players: the engine's team 0 goes first; the client's engine players are 0 (the client) and 1 (the CPU).
//! The server mirrors each side's deck and hand as client uids with their CARD_Prop rows. The client's deck
//! is a face-down bag, so a draw takes any deck card with the drawn card's id.
use std::collections::HashMap;

use serde_json::{Value as Json, json};
use yo3_proto::Message;

use crate::duel::{
    CARD, CLIENT, CPU, CardCommand, DECK, HAND, INPUT, MOVE, PHASE, TURN, command, command_data, move_place, table_place,
};

/// ocgcore message types (OcgMessageType).
const MSG_SELECT_BATTLECMD: u64 = 10;
const MSG_SELECT_IDLECMD: u64 = 11;
const MSG_SELECT_CHAIN: u64 = 16;
const MSG_NEW_TURN: u64 = 40;
const MSG_NEW_PHASE: u64 = 41;
const MSG_DRAW: u64 = 90;
/// ocgcore response types (OcgResponseType) and actions.
const RESP_BATTLECMD: u64 = 0;
const RESP_IDLECMD: u64 = 1;
const RESP_CHAIN: u64 = 8;
const IDLE_TO_BP: u64 = 6;
const IDLE_TO_EP: u64 = 7;
const BATTLE_TO_EP: u64 = 3;
/// ocgcore locations.
const LOC_HAND: u64 = 2;
const LOC_MZONE: u64 = 4;
const LOC_SZONE: u64 = 8;

/// Card menu bits (`CardCommand::command`) per idle-command list, with the list's action number. Set: bit 6 for
/// monsters and 7 for Spells/Traps is a guess (both read "Set").
const IDLE_LISTS: [(&str, u32, u64); 7] = [
    ("summons", 4, 0),
    ("special_summons", 2, 1),
    ("pos_changes", 8, 2),
    ("pos_changes", 9, 2),
    ("monster_sets", 6, 3),
    ("spell_sets", 7, 4),
    ("activates", 3, 5),
];

#[derive(Clone, Copy, Debug)]
struct Card {
    uid: u32,
    row: u16,
}

#[derive(Default)]
struct Side {
    deck: Vec<Card>,
    hand: Vec<Card>,
}

/// What to do with an engine prompt.
#[derive(Debug, PartialEq)]
pub enum Prompt {
    /// Answer the engine right away (the CPU's choice, or a prompt the client has no window for yet).
    Answer(Json),
    /// Send these to the client and wait for its command.
    Client(Vec<Message>),
}

pub struct Translator {
    /// The client's engine player of team 0.
    first: u32,
    sides: [Side; 2],
    cid_of_row: HashMap<u16, u16>,
    turn_player: u32,
    turns: u32,
    /// The client's pending idle-command prompt.
    idle: Option<Json>,
}

impl Translator {
    /// `decks[p]` is the main deck (CARD_Prop rows) of the client's engine player p; `first` goes first. Returns
    /// the translator and the card-table commands that put both decks in the client's card table (uids 1..).
    pub fn new(first: u32, decks: [&[u16]; 2], cid_of_row: HashMap<u16, u16>) -> (Translator, Vec<Message>) {
        let mut sides = [Side::default(), Side::default()];
        let mut out = vec![];
        let mut uid = 0;
        for (p, deck) in decks.iter().enumerate() {
            for (i, &row) in deck.iter().enumerate() {
                uid += 1;
                sides[p].deck.push(Card { uid, row });
                out.push(command(CARD, [uid, row as u32, 0, table_place(p as u32, DECK, i)]));
            }
        }
        (Translator { first, sides, cid_of_row, turn_player: first, turns: 0, idle: None }, out)
    }

    /// The client's engine player of an engine team.
    fn player(&self, team: &Json) -> u32 {
        if team.as_u64() == Some(0) { self.first } else { 1 - self.first }
    }

    /// The client commands for the engine's events (prompts excluded).
    pub fn events(&mut self, messages: &[Json]) -> Vec<Message> {
        let mut out = vec![];
        for m in messages {
            match m["type"].as_u64() {
                Some(MSG_DRAW) => {
                    let p = self.player(&m["player"]);
                    for drawn in m["drawn"].as_array().into_iter().flatten() {
                        out.extend(self.draw(p, drawn["cid"].as_u64().unwrap_or(0) as u16));
                    }
                }
                Some(MSG_NEW_TURN) => {
                    self.turn_player = self.player(&m["player"]);
                    self.turns += 1;
                    // The client starts on turn 1; 6 would count it again.
                    if self.turns > 1 {
                        out.push(command(TURN, [self.turn_player, 0, 0, 0]));
                    }
                }
                Some(MSG_NEW_PHASE) => {
                    if let Some(phase) = m["phase"].as_u64().and_then(client_phase) {
                        out.push(command(PHASE, [self.turn_player, phase, 0, 0]));
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// A deck card with that id moves to the hand: card table, then on screen.
    fn draw(&mut self, p: u32, cid: u16) -> Vec<Message> {
        let side = &mut self.sides[p as usize];
        let at = side.deck.iter().position(|c| self.cid_of_row.get(&c.row) == Some(&cid)).unwrap_or(0);
        if side.deck.is_empty() {
            return vec![];
        }
        let c = side.deck.remove(at);
        let i = side.hand.len();
        side.hand.push(c);
        let row = c.row as u32;
        vec![
            command(CARD, [c.uid, row, 0, table_place(p, HAND, i)]),
            command(MOVE, [c.uid | row << 16, move_place(p, DECK, 0), move_place(p, HAND, i), 0]),
        ]
    }

    /// What to do with the engine's prompt (the last message when it awaits).
    pub fn prompt(&mut self, m: &Json) -> Prompt {
        let kind = m["type"].as_u64().unwrap_or(0);
        if self.player(&m["player"]) == CPU {
            return Prompt::Answer(cpu_answer(kind, m));
        }
        match kind {
            MSG_SELECT_IDLECMD => {
                let map = self.command_map(m);
                self.idle = Some(m.clone());
                Prompt::Client(vec![command_data(&map), command(INPUT, [CLIENT, 0, 0, 0])])
            }
            // No window for these yet: never chain, end the battle (temporary).
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// The command map of an idle-command prompt: each listed card's place and menu bits.
    fn command_map(&self, m: &Json) -> Vec<(u32, u32)> {
        let mut bits: Vec<(u32, u32)> = vec![];
        for (list, bit, _) in IDLE_LISTS {
            for loc in m[list].as_array().into_iter().flatten() {
                let Some(place) = self.place(loc) else { continue };
                match bits.iter_mut().find(|(p, _)| *p == place) {
                    Some((_, b)) => *b |= 1 << bit,
                    None => bits.push((place, 1 << bit)),
                }
            }
        }
        bits
    }

    /// The card-table place of an engine card location: hand (0xB, index), monster zone (0, slot), Spell/Trap
    /// zone (5, slot).
    fn place(&self, loc: &Json) -> Option<u32> {
        let p = self.player(&loc["controller"]);
        let seq = loc["sequence"].as_u64()? as usize;
        let zone = match loc["location"].as_u64()? {
            LOC_HAND => HAND,
            LOC_MZONE => 0,
            LOC_SZONE => 5,
            _ => return None,
        };
        Some(table_place(p, zone, seq))
    }

    /// The engine response for the client's card command, if it names a listed card.
    pub fn answer(&mut self, cmd: CardCommand) -> Option<Json> {
        let idle = self.idle.as_ref()?;
        let place = table_place(cmd.player, cmd.zone, cmd.index as usize);
        for (list, bit, action) in IDLE_LISTS {
            if bit != cmd.command {
                continue;
            }
            let at = idle[list].as_array()?.iter().position(|loc| self.place(loc) == Some(place));
            if let Some(index) = at {
                self.idle = None;
                return Some(json!({ "type": RESP_IDLECMD, "action": action, "index": index }));
            }
        }
        None
    }
}

/// The panel's phase (command 5) for an ocgcore phase: 0 Draw, 1 Standby, 2 Main 1, 3 Battle (all its steps),
/// 4 Main 2, 5 End.
fn client_phase(phase: u64) -> Option<u32> {
    Some(match phase {
        1 => 0,
        2 => 1,
        4 => 2,
        0x08..=0x80 => 3,
        0x100 => 4,
        0x200 => 5,
        _ => return None,
    })
}

/// The CPU's answer: end its turn as soon as it may, never chain. (Placeholder until an AI.)
fn cpu_answer(kind: u64, m: &Json) -> Json {
    match kind {
        MSG_SELECT_IDLECMD => {
            let action = if m["to_ep"].as_bool() == Some(true) { IDLE_TO_EP } else { IDLE_TO_BP };
            json!({ "type": RESP_IDLECMD, "action": action, "index": null })
        }
        MSG_SELECT_BATTLECMD => json!({ "type": RESP_BATTLECMD, "action": BATTLE_TO_EP, "index": null }),
        MSG_SELECT_CHAIN => {
            let index = if m["forced"].as_bool() == Some(true) { json!(0) } else { Json::Null };
            json!({ "type": RESP_CHAIN, "index": index })
        }
        _ => json!({ "type": RESP_CHAIN, "index": null }),
    }
}
