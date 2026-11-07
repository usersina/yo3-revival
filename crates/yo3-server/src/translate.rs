//! Translator between the rules engine (ocgcore messages, see `engine.rs`) and the client's duel stream
//! (engine commands, command maps; see "Duel connection" in `docs/protocol.md`).
//!
//! Players: the engine's team 0 goes first; the client's engine players are 0 (the client) and 1 (the CPU).
//! The server mirrors every zone of each side as client uids with their CARD_Prop rows, in the client's zone
//! numbering: field slots 0-4 (monsters), 5-9 (Spells/Traps), 10 (Field Spell), each holding one card at index 0;
//! then the lists hand 0xB, extra 0xC, deck 0xD, graveyard 0xE, banished 0xF. The client's deck is a face-down
//! bag, so a card leaving it is any deck card with that card's id.
use std::collections::HashMap;

use serde_json::{Value as Json, json};
use yo3_proto::Message;

use crate::duel::{
    CARD, CHANGE_LP, CLIENT, CPU, CardCommand, DECK, HAND, INPUT, MOVE, PHASE, SET_LP, TURN, command, command_data,
    move_place, table_place,
};

/// ocgcore message types (OcgMessageType).
const MSG_WIN: u64 = 5;
const MSG_SELECT_BATTLECMD: u64 = 10;
const MSG_SELECT_IDLECMD: u64 = 11;
const MSG_SELECT_EFFECTYN: u64 = 12;
const MSG_SELECT_YESNO: u64 = 13;
const MSG_SELECT_OPTION: u64 = 14;
const MSG_SELECT_CARD: u64 = 15;
const MSG_SELECT_CHAIN: u64 = 16;
const MSG_SELECT_PLACE: u64 = 18;
const MSG_SELECT_POSITION: u64 = 19;
const MSG_SELECT_TRIBUTE: u64 = 20;
const MSG_SORT_CHAIN: u64 = 21;
const MSG_SELECT_COUNTER: u64 = 22;
const MSG_SELECT_DISFIELD: u64 = 24;
const MSG_SORT_CARD: u64 = 25;
const MSG_SELECT_UNSELECT_CARD: u64 = 26;
const MSG_ROCK_PAPER_SCISSORS: u64 = 132;
const MSG_ANNOUNCE_RACE: u64 = 140;
const MSG_ANNOUNCE_ATTRIB: u64 = 141;
const MSG_ANNOUNCE_NUMBER: u64 = 143;
const MSG_MOVE: u64 = 50;
const MSG_NEW_TURN: u64 = 40;
const MSG_NEW_PHASE: u64 = 41;
const MSG_DRAW: u64 = 90;
const MSG_DAMAGE: u64 = 91;
const MSG_RECOVER: u64 = 92;
const MSG_LPUPDATE: u64 = 94;
const MSG_PAY_LPCOST: u64 = 100;
/// ocgcore response types (OcgResponseType) and actions.
const RESP_BATTLECMD: u64 = 0;
const RESP_IDLECMD: u64 = 1;
const RESP_EFFECTYN: u64 = 2;
const RESP_YESNO: u64 = 3;
const RESP_OPTION: u64 = 4;
const RESP_CARD: u64 = 5;
const RESP_UNSELECT_CARD: u64 = 7;
const RESP_CHAIN: u64 = 8;
const RESP_DISFIELD: u64 = 9;
const RESP_PLACE: u64 = 10;
const RESP_POSITION: u64 = 11;
const RESP_TRIBUTE: u64 = 12;
const RESP_COUNTER: u64 = 13;
const RESP_SORT: u64 = 15;
const RESP_RACE: u64 = 16;
const RESP_ATTRIB: u64 = 17;
const RESP_NUMBER: u64 = 19;
const RESP_RPS: u64 = 20;
const IDLE_SUMMON: u64 = 0;
const IDLE_MONSTER_SET: u64 = 3;
const IDLE_TO_BP: u64 = 6;
const IDLE_TO_EP: u64 = 7;
const BATTLE_CHAIN: u64 = 0;
const BATTLE_ATTACK: u64 = 1;
const BATTLE_TO_M2: u64 = 2;
const BATTLE_TO_EP: u64 = 3;
/// ocgcore locations.
const LOC_DECK: u64 = 1;
const LOC_HAND: u64 = 2;
const LOC_MZONE: u64 = 4;
const LOC_SZONE: u64 = 8;
const LOC_GRAVE: u64 = 16;
const LOC_REMOVED: u64 = 32;
const LOC_EXTRA: u64 = 64;
/// The client's list zones besides hand and deck.
const EXTRA: u32 = 0xC;
const GRAVE: u32 = 0xE;
const REMOVED: u32 = 0xF;
/// Data commands that build a field card's record (pop 0x522A50).
const RECORD_BEGIN: u32 = 0x3E9;
const RECORD_MORE: u32 = 0x3EA;
const RECORD_COMMIT: u32 = 0x3EB;
/// The Field Spell slot.
const FIELD_SPELL: u32 = 10;
/// Duel end {result for this client: 1 won, 2 lost; reason} (DuelScene 0x514290).
const DUEL_END: u32 = 2;
const WON: u32 = 1;
const LOST: u32 = 2;
/// Phase panel buttons (0 DP .. 5 EP) and the input mode that turns the panel on (command 4's param1).
const BUTTON_BP: u8 = 3;
const BUTTON_M2: u8 = 4;
const BUTTON_EP: u8 = 5;
const MODE_MAIN: u32 = 2;
const MODE_BATTLE: u32 = 3;
/// A prompt's card lists: (list name, card menu bit, response action).
type Lists = &'static [(&'static str, u32, u64)];

/// Card menu bits of the battle prompt's lists, with their actions.
const BATTLE_LISTS: [(&str, u32, u64); 2] = [("attacks", 0, BATTLE_ATTACK), ("chains", 3, BATTLE_CHAIN)];

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

/// What the duel shows of a card, from its CARD_Prop row.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CardStats {
    pub cid: u16,
    pub atk: u32,
    pub def: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u32,
}

#[derive(Clone, Copy, Debug)]
struct Card {
    uid: u32,
    row: u16,
}

/// One side's zones: field slots (0..=10) hold at most one card, list zones (0xB..=0xF) any number.
#[derive(Default)]
struct Side {
    zones: HashMap<u32, Vec<Card>>,
}

impl Side {
    fn zone(&mut self, zone: u32) -> &mut Vec<Card> {
        self.zones.entry(zone).or_default()
    }
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
    cards: HashMap<u16, CardStats>,
    turn_player: u32,
    turns: u32,
    /// The client's pending prompt (idle or battle command).
    pending: Option<Json>,
}

impl Translator {
    /// `decks[p]` is the main deck (CARD_Prop rows) of the client's engine player p; `first` goes first. Returns
    /// the translator and the card-table commands that put both decks in the client's card table (uids 1..).
    pub fn new(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        let mut sides = [Side::default(), Side::default()];
        let mut out = vec![];
        let mut uid = 0;
        for (p, deck) in decks.iter().enumerate() {
            for (i, &row) in deck.iter().enumerate() {
                uid += 1;
                sides[p].zone(DECK).push(Card { uid, row });
                out.push(command(CARD, [uid, row as u32, 0, table_place(p as u32, DECK, i)]));
            }
        }
        (Translator { first, sides, cards, turn_player: first, turns: 0, pending: None }, out)
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
                Some(MSG_MOVE) => out.extend(self.move_card(m)),
                Some(MSG_WIN) => {
                    let result = if self.player(&m["player"]) == CLIENT { WON } else { LOST };
                    out.push(command(DUEL_END, [result, 0, 0, 0]));
                }
                Some(kind @ (MSG_DAMAGE | MSG_RECOVER | MSG_PAY_LPCOST)) => {
                    let amount = m["amount"].as_i64().unwrap_or(0) as i32;
                    let change = if kind == MSG_RECOVER { amount } else { -amount };
                    out.push(command(CHANGE_LP, [self.player(&m["player"]), change as u32, 0, 0]));
                }
                Some(MSG_LPUPDATE) => {
                    out.push(command(SET_LP, [self.player(&m["player"]), m["lp"].as_u64().unwrap_or(0) as u32, 0, 0]));
                }
                _ => {}
            }
        }
        out
    }

    /// A deck card with that id moves to the end of the hand.
    fn draw(&mut self, p: u32, cid: u16) -> Vec<Message> {
        let hand = self.sides[p as usize].zone(HAND).len();
        let from = Place { player: p, zone: DECK, index: 0, face_up: false, defense: false };
        let to = Place { player: p, zone: HAND, index: hand, face_up: false, defense: false };
        self.relocate(cid, from, to)
    }

    /// MSG_MOVE {card, cid, from, to}: the card leaves its place for the new one.
    fn move_card(&mut self, m: &Json) -> Vec<Message> {
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
        match (self.engine_place(&m["from"]), self.engine_place(&m["to"])) {
            (Some(from), Some(to)) => self.relocate(cid, from, to),
            // Tokens and Xyz materials have no client place yet.
            _ => vec![],
        }
    }

    /// Moves a card in the mirror and on the client: the card table entry (1000), then on screen (0x1A); the
    /// cards whose index shifted in a list zone get their table entry again.
    fn relocate(&mut self, cid: u16, from: Place, to: Place) -> Vec<Message> {
        let cards = &self.cards;
        let source = self.sides[from.player as usize].zone(from.zone);
        let at = if from.zone == DECK {
            source.iter().position(|c| cards.get(&c.row).map(|s| s.cid) == Some(cid)).or(if source.is_empty() { None } else { Some(0) })
        } else {
            (!source.is_empty()).then(|| from.index.min(source.len() - 1))
        };
        let Some(at) = at else { return vec![] };
        let card = source.remove(at);
        let dest = self.sides[to.player as usize].zone(to.zone);
        let index = if to.zone > FIELD_SPELL { to.index.min(dest.len()) } else { 0 };
        if to.zone <= FIELD_SPELL {
            dest.clear();
        }
        dest.insert(index, card);
        let row = card.row as u32;
        let mut out = vec![command(CARD, [card.uid, row, 0, table_place(to.player, to.zone, index)])];
        if to.zone < 5 {
            out.extend(self.monster_record(to.player, to.zone, card.row));
        }
        out.push(command(MOVE, [card.uid | row << 16, from.packed(), Place { index, ..to }.packed(), 0]));
        // List zones renumber: re-send the shifted cards (not the face-down deck).
        for (p, zone, start) in [(from.player, from.zone, at), (to.player, to.zone, index + 1)] {
            if zone <= FIELD_SPELL || zone == DECK {
                continue;
            }
            for (i, c) in self.sides[p as usize].zone(zone).iter().enumerate().skip(start) {
                out.push(command(CARD, [c.uid, c.row as u32, 0, table_place(p, zone, i)]));
            }
        }
        out
    }

    /// The record the card info and the field label show for a monster zone (0x523F00, keyed player | zone << 8):
    /// shown id, second id, race, attribute, level, ATK, DEF, base ATK, base DEF. Data commands 0x3E9 {_, values 0,
    /// 2, 3}, 0x3EA {_, values 4, 5, 6}, 0x3EB {key, values 7, 8, 1} (which commits it; the pop 0x522A50).
    /// Stats from CARD_Prop: effects that change them are not followed yet.
    fn monster_record(&self, player: u32, zone: u32, row: u16) -> Vec<Message> {
        let s = self.cards.get(&row).copied().unwrap_or_default();
        let cid = s.cid as u32;
        vec![
            command(RECORD_BEGIN, [0, cid, s.race, s.attribute]),
            command(RECORD_MORE, [0, s.level, s.atk, s.def]),
            command(RECORD_COMMIT, [player | zone << 8, s.atk, s.def, cid]),
        ]
    }

    /// The client place of an engine location {controller, location, sequence, position}.
    fn engine_place(&self, loc: &Json) -> Option<Place> {
        let player = self.player(&loc["controller"]);
        let seq = loc["sequence"].as_u64()? as usize;
        let (zone, index) = match loc["location"].as_u64()? {
            LOC_DECK => (DECK, 0),
            LOC_HAND => (HAND, seq),
            LOC_MZONE => (seq as u32, 0),
            LOC_SZONE if seq == 5 => (FIELD_SPELL, 0),
            LOC_SZONE => (5 + seq as u32, 0),
            LOC_GRAVE => (GRAVE, seq),
            LOC_REMOVED => (REMOVED, seq),
            LOC_EXTRA => (EXTRA, seq),
            _ => return None,
        };
        // ocgcore positions: 1 face-up attack, 2 face-down attack, 4 face-up defense, 8 face-down defense. Only
        // field cards carry them; the client shows each list zone its own way.
        let position = if zone <= FIELD_SPELL { loc["position"].as_u64().unwrap_or(0) } else { 0 };
        Some(Place { player, zone, index, face_up: position & 5 != 0, defense: position & 12 != 0 })
    }

    /// What to do with the engine's prompt (the last message when it awaits).
    pub fn prompt(&mut self, m: &Json) -> Prompt {
        let kind = m["type"].as_u64().unwrap_or(0);
        if self.player(&m["player"]) == CPU {
            return Prompt::Answer(cpu_answer(kind, m));
        }
        match kind {
            MSG_SELECT_IDLECMD | MSG_SELECT_BATTLECMD => {
                let (lists, buttons, mode): (Lists, [(&str, u8); 2], u32) = if kind == MSG_SELECT_IDLECMD {
                    (&IDLE_LISTS, [("to_bp", BUTTON_BP), ("to_ep", BUTTON_EP)], MODE_MAIN)
                } else {
                    (&BATTLE_LISTS, [("to_m2", BUTTON_M2), ("to_ep", BUTTON_EP)], MODE_BATTLE)
                };
                let map = self.command_map(m, lists);
                let flags: Vec<(u8, u8)> = buttons.into_iter().filter(|(k, _)| m[*k] == true).map(|(_, b)| (b, 1)).collect();
                self.pending = Some(m.clone());
                Prompt::Client(vec![command_data(&map, &flags), command(INPUT, [mode, CLIENT, 0, 0])])
            }
            // No window for these yet: never chain, end the battle (temporary).
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// The command map of a prompt: each listed card's place and menu bits.
    fn command_map(&self, m: &Json, lists: Lists) -> Vec<(u32, u32)> {
        let mut bits: Vec<(u32, u32)> = vec![];
        for &(list, bit, _) in lists {
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

    /// The engine response for a phase button (0 DP .. 5 EP), if the pending prompt allows that phase.
    pub fn phase(&mut self, button: u32) -> Option<Json> {
        let m = self.pending.as_ref()?;
        let battle = m["type"].as_u64() == Some(MSG_SELECT_BATTLECMD);
        let (kind, action) = match (battle, button as u8) {
            (false, BUTTON_BP) if m["to_bp"] == true => (RESP_IDLECMD, IDLE_TO_BP),
            (false, BUTTON_EP) if m["to_ep"] == true => (RESP_IDLECMD, IDLE_TO_EP),
            (true, BUTTON_M2) if m["to_m2"] == true => (RESP_BATTLECMD, BATTLE_TO_M2),
            (true, BUTTON_EP) if m["to_ep"] == true => (RESP_BATTLECMD, BATTLE_TO_EP),
            _ => return None,
        };
        self.pending = None;
        Some(json!({ "type": kind, "action": action, "index": null }))
    }

    /// The engine response for the client's card command, if it names a listed card.
    pub fn answer(&mut self, cmd: CardCommand) -> Option<Json> {
        let m = self.pending.as_ref()?;
        let (lists, kind): (Lists, u64) = match m["type"].as_u64() {
            Some(MSG_SELECT_BATTLECMD) => (&BATTLE_LISTS, RESP_BATTLECMD),
            _ => (&IDLE_LISTS, RESP_IDLECMD),
        };
        let place = table_place(cmd.player, cmd.zone, cmd.index as usize);
        for &(list, bit, action) in lists {
            if bit != cmd.command {
                continue;
            }
            let at = m[list].as_array()?.iter().position(|loc| self.place(loc) == Some(place));
            if let Some(index) = at {
                self.pending = None;
                return Some(json!({ "type": kind, "action": action, "index": index }));
            }
        }
        None
    }
}

/// A place on the client's board.
#[derive(Clone, Copy, Debug)]
struct Place {
    player: u32,
    zone: u32,
    index: usize,
    face_up: bool,
    defense: bool,
}

impl Place {
    /// As card moves give it: player | zone << 1 | index << 6, face up bit 14, defense bit 15 (0x5000E0).
    fn packed(&self) -> u32 {
        move_place(self.player, self.zone, self.index) | (self.face_up as u32) << 14 | (self.defense as u32) << 15
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

/// The placeholder AI's answer to any prompt (see [`cpu_answer`]).
pub fn bot_answer(m: &Json) -> Json {
    cpu_answer(m["type"].as_u64().unwrap_or(0), m)
}

/// The CPU's answer, and the default for the client's prompts that have no window yet. A placeholder AI: Normal
/// Summon (or Set) the first monster it may, attack with every attacker, never chain; card selections take the first
/// `min` cards, questions are answered yes, lists their first entry.
fn cpu_answer(kind: u64, m: &Json) -> Json {
    let list_len = |k: &str| m[k].as_array().map_or(0, Vec::len);
    let first = |min: &Json, max: &Json, len: usize| {
        let n = (min.as_u64().unwrap_or(1) as usize).max(1).min(max.as_u64().unwrap_or(1) as usize).min(len);
        (0..n).collect::<Vec<_>>()
    };
    match kind {
        MSG_SELECT_IDLECMD => {
            let (action, index) = if list_len("summons") > 0 {
                (IDLE_SUMMON, json!(0))
            } else if list_len("monster_sets") > 0 {
                (IDLE_MONSTER_SET, json!(0))
            } else if m["to_bp"] == true {
                (IDLE_TO_BP, Json::Null)
            } else {
                (IDLE_TO_EP, Json::Null)
            };
            json!({ "type": RESP_IDLECMD, "action": action, "index": index })
        }
        MSG_SELECT_BATTLECMD if list_len("attacks") > 0 => json!({ "type": RESP_BATTLECMD, "action": BATTLE_ATTACK, "index": 0 }),
        MSG_SELECT_BATTLECMD => json!({ "type": RESP_BATTLECMD, "action": BATTLE_TO_EP, "index": null }),
        MSG_SELECT_CHAIN => {
            let index = if m["forced"].as_bool() == Some(true) { json!(0) } else { Json::Null };
            json!({ "type": RESP_CHAIN, "index": index })
        }
        MSG_SELECT_CARD => json!({ "type": RESP_CARD, "indicies": first(&m["min"], &m["max"], list_len("selects")) }),
        MSG_SELECT_TRIBUTE => json!({ "type": RESP_TRIBUTE, "indicies": first(&m["min"], &m["max"], list_len("selects")) }),
        MSG_SELECT_UNSELECT_CARD => {
            let index = if m["can_finish"] == true { Json::Null } else { json!(0) };
            json!({ "type": RESP_UNSELECT_CARD, "index": index })
        }
        MSG_SELECT_EFFECTYN => json!({ "type": RESP_EFFECTYN, "yes": true }),
        MSG_SELECT_YESNO => json!({ "type": RESP_YESNO, "yes": true }),
        MSG_SELECT_OPTION => json!({ "type": RESP_OPTION, "index": 0 }),
        MSG_SORT_CHAIN | MSG_SORT_CARD => json!({ "type": RESP_SORT, "order": null }),
        MSG_SELECT_COUNTER => {
            // Every counter from the first card.
            let mut counters = vec![0; list_len("cards").max(1)];
            counters[0] = m["count"].as_u64().unwrap_or(0);
            json!({ "type": RESP_COUNTER, "counters": counters })
        }
        // The first free own zone (field_mask bits set = unavailable; own monster zones bits 0-4, Spell/Trap 8-12).
        MSG_SELECT_PLACE | MSG_SELECT_DISFIELD => {
            let mask = m["field_mask"].as_u64().unwrap_or(0);
            let free = (0..5).map(|s| (LOC_MZONE, s, s)).chain((0..5).map(|s| (LOC_SZONE, s, s + 8)));
            let (location, sequence) = free.into_iter().find(|(_, _, bit)| mask & 1 << bit == 0).map_or((LOC_MZONE, 0), |(l, s, _)| (l, s));
            let kind = if kind == MSG_SELECT_PLACE { RESP_PLACE } else { RESP_DISFIELD };
            json!({ "type": kind, "places": [{ "player": m["player"], "location": location, "sequence": sequence }] })
        }
        // Face-up attack when allowed, else the first allowed position.
        MSG_SELECT_POSITION => {
            let allowed = m["positions"].as_u64().unwrap_or(1);
            let position = if allowed & 1 != 0 { 1 } else { 1 << allowed.trailing_zeros() };
            json!({ "type": RESP_POSITION, "position": position })
        }
        // The lowest `count` of the available bits.
        MSG_ANNOUNCE_RACE | MSG_ANNOUNCE_ATTRIB => {
            let available = m["available"].as_u64().unwrap_or(1);
            let picked: Vec<u64> = (0..64).map(|b| 1u64 << b).filter(|b| available & b != 0).take(m["count"].as_u64().unwrap_or(1) as usize).collect();
            let (kind, key) = if kind == MSG_ANNOUNCE_RACE { (RESP_RACE, "races") } else { (RESP_ATTRIB, "attributes") };
            json!({ "type": kind, key: picked })
        }
        MSG_ANNOUNCE_NUMBER => json!({ "type": RESP_NUMBER, "value": 0 }),
        MSG_ROCK_PAPER_SCISSORS => json!({ "type": RESP_RPS, "value": 1 }),
        _ => {
            tracing::warn!("rules engine: no answer for prompt {kind}");
            json!({ "type": RESP_CHAIN, "index": null })
        }
    }
}
