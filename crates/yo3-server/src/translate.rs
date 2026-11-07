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
const MSG_HINT: u64 = 2;
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
const MSG_CHAINING: u64 = 70;
const MSG_CHAIN_END: u64 = 74;
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
/// MSG_HINT's hint_type for "what the next selection is for" (HINTMSG_* in the card scripts' constant.lua).
const HINT_SELECTMSG: u64 = 3;
const HINTMSG_RELEASE: u64 = 500;
/// ocgcore positions.
const POS_FACEUP_ATTACK: u64 = 1;
const POS_FACEDOWN_ATTACK: u64 = 2;
const POS_FACEUP_DEFENSE: u64 = 4;
const POS_FACEDOWN_DEFENSE: u64 = 8;
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
/// The card list dialog: DuelScene command 0x37 {player, kind, message} (DuelCardListDialog 0x4C72D0), filled by
/// data commands the pop keeps (0x522A50): 0x3EF {count, max picks}, 0x3F2 CARD_Prop rows and 0x3F3 marks, four at a
/// time, and a message of 0x3EC {entries} and 0x3ED {entry, ...} (built by 0x519870).
const CARD_LIST: u32 = 0x37;
const LIST_BEGIN: u32 = 0x3EF;
const LIST_VALUES: u32 = 0x3F0;
const LIST_ROWS: u32 = 0x3F2;
const LIST_MARKS: u32 = 0x3F3;
const MESSAGE_LENGTH: u32 = 0x3EC;
const MESSAGE: u32 = 0x3ED;
/// A list mark: the opponent's card (dlg_cardlist.swf). The low bits add icons, meaning unknown.
const OPPONENTS: u32 = 0x8000;
/// Card list kinds by the picks OK needs (0x4C72D0): none, all `max`, one.
const PICK_ANY: u32 = 1;
const PICK_ALL: u32 = 6;
const PICK_ONE: u32 = 0x27;
/// Message entries (0x519870): a text of the duel prompt table, and a card name by Konami id (0x5374B0) put into
/// the text's %s.
const TEXT: u32 = 1 << 28;
const CARD_NAME: u32 = 5 << 28;
/// Duel prompt texts: "Select the @3monster(s)@0 as @3Tribute@0.", 'Activate the effect of "@3%s@0"?', 'Use the
/// effect of "@3%s@0"?', "Use the effect now?".
const TRIBUTE_TEXT: u32 = 18;
const ACTIVATE_TEXT: u32 = 250;
const USE_TEXT: u32 = 226;
const USE_NOW_TEXT: u32 = 270;
/// 'Select position for "@3%s@0".'
const POSITION_TEXT: u32 = 327;
/// 'The effect of "@3%s@0" is @3activated@0. @3Chain@0 another card or effect?', '"@3%s@0" is @3activated@0. ...' and,
/// outside a chain, "@3Activate@0 the effect of a @3card@0?"; then "Select effect to @3activate@0.".
const CHAIN_EFFECT_TEXT: u32 = 7;
const CHAIN_CARD_TEXT: u32 = 8;
const ACTIVATE_CARD_TEXT: u32 = 269;
const SELECT_EFFECT_TEXT: u32 = 276;
/// Option labels of the duel prompt table for EDOPro's system strings, with the list's title: SELECT_HEADS/TAILS (60,
/// 61) "Heads", "Tails" under "Coin Toss Selection:"; DECLTYPE_MONSTER/SPELL/TRAP (70-72) "Monster Cards", "Spell
/// Cards", "Trap Cards" under "Select @3card@0 type.".
const OPTION_LABELS: [(u64, u32, u32); 5] =
    [(60, 400, 56), (61, 401, 56), (70, 440, 260), (71, 441, 260), (72, 442, 260)];
/// Dialog command {kind, message, _} (DuelScene 0x514C0C, factory 0x4C8CD0) and its Yes/No kind (buttons 0 "No", 1
/// "Yes"); message 1 = from the message entries.
const DIALOG: u32 = 0x36;
const YES_NO: u32 = 2;
const FROM_ENTRIES: u32 = 1;
/// The chain question {_, message, level} (DuelEffectDialog 0x4C5120): buttons as Yes/No's, with the list's rows
/// (dlg_yesno_card.swf, at most five) when 0x3EF's param3 is 0xFF (0x4C51A0). Whether it shows depends on the level
/// and the player's Activation Message option (invoke_conf, 0x4C5560): Manual shows every level, Semi Auto Cancel (the
/// default) levels 2 and up, Auto Cancel none (unless a mouse button is held; level 4+ always shows). Unshown, it
/// answers No. The levels sent: something happened, a phase or step, and no card fitting the timing.
const EFFECT: u32 = 3;
const WITH_CARDS: u32 = 0xFF;
const LEVEL_EVENT: u32 = 3;
const LEVEL_PHASE: u32 = 1;
const LEVEL_UNFIT: u32 = 0;
/// EDOPro's timings (TIMING_* in constant.lua) that only mark a phase or step: draw, standby and end phases, the
/// main phase's end, the battle's start, steps and end, a chain's end, a draw.
const QUIET_TIMINGS: u64 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x8000 | 0x10000 | 0x100_0000 | 0x400_0000;
/// The option list (DuelSelectDialog 0x4C5750): the 0x3F0 values up to 0x3EF's count, each a duel prompt text listed
/// with bit 15, at most four (0x4C5880); DLGResult the value's index (0x4C5B20).
const SELECT: u32 = 4;
const LISTED: u32 = 0x8000;
const SELECT_MAX: usize = 4;
/// The position dialog {_, message, uid | face-up defense << 16} (DuelSelectStandDialog 0x4C5E50, dlg_selstand.swf):
/// button 0 attack, 1 defense, face-down unless the flag is set.
const POSITION: u32 = 8;
const FACE_UP_DEFENSE: u32 = 1 << 16;
/// Duel end {result for this client: 1 won, 2 lost, 3 draw; reason} (DuelScene 0x514290).
pub(crate) const DUEL_END: u32 = 2;
pub(crate) const WON: u32 = 1;
pub(crate) const LOST: u32 = 2;
const DRAW: u32 = 3;
/// WIN's player for a draw.
const TEAM_DRAW: u64 = 2;
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
    /// The client's pending prompt (idle or battle command, card choice, question), and whether its chain chance shows
    /// the card list (after Yes).
    pending: Option<Json>,
    chain_list: bool,
    /// The chain's last link (MSG_CHAINING) until the chain ends.
    chaining: Option<Json>,
    /// The last selection hint (HINTMSG_*) and the client's picks in the card list so far.
    hint: u64,
    picks: Vec<usize>,
    /// The duel's result for the client (1 won, 2 lost, 3 draw), once it ended.
    result: Option<u32>,
    /// Whether the opponent's hidden cards (deck, hand, extra deck, face-down) go to the client without their rows.
    hide: bool,
}

impl Translator {
    /// `decks[p]` is the main deck (CARD_Prop rows) of the client's engine player p; `first` goes first. Returns
    /// the translator and the card-table commands that put both decks in the client's card table (uids 1..).
    pub fn new(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        Translator::build(first, decks, cards, false)
    }

    /// The same for a duel against another player, whose hidden cards the client gets with row 0 (**guess**: Online
    /// 2 used placeholders).
    pub fn against_player(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        Translator::build(first, decks, cards, true)
    }

    fn build(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>, hide: bool) -> (Translator, Vec<Message>) {
        let mut sides = [Side::default(), Side::default()];
        let mut out = vec![];
        let mut uid = 0;
        for (p, deck) in decks.iter().enumerate() {
            for (i, &row) in deck.iter().enumerate() {
                uid += 1;
                sides[p].zone(DECK).push(Card { uid, row });
                let shown = if hide && p as u32 == CPU { 0 } else { row as u32 };
                out.push(command(CARD, [uid, shown, 0, table_place(p as u32, DECK, i)]));
            }
        }
        let t = Translator {
            first,
            sides,
            cards,
            turn_player: first,
            turns: 0,
            pending: None,
            chain_list: false,
            chaining: None,
            hint: 0,
            picks: vec![],
            result: None,
            hide,
        };
        (t, out)
    }

    /// The duel's result for the client (1 won, 2 lost, 3 draw), once the engine ended the duel.
    pub fn result(&self) -> Option<u32> {
        self.result
    }

    /// The client's engine player of an engine team.
    fn player(&self, team: &Json) -> u32 {
        if team.as_u64() == Some(0) { self.first } else { 1 - self.first }
    }

    /// Whether an engine prompt asks this client (not the other side).
    pub fn prompted(&self, m: &Json) -> bool {
        self.player(&m["player"]) == CLIENT
    }

    /// The row the client gets for a card at a place: none for the opponent's hidden cards when hiding.
    fn shown_row(&self, place: &Place, row: u16) -> u32 {
        let hidden = match place.zone {
            HAND | DECK | EXTRA => true,
            zone => zone <= FIELD_SPELL && !place.face_up,
        };
        if self.hide && place.player == CPU && hidden { 0 } else { row as u32 }
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
                Some(MSG_HINT) if m["hint_type"].as_u64() == Some(HINT_SELECTMSG) && self.player(&m["player"]) == CLIENT => {
                    self.hint = m["hint"].as_u64().unwrap_or(0);
                }
                Some(MSG_CHAINING) => self.chaining = Some(m.clone()),
                Some(MSG_CHAIN_END) => self.chaining = None,
                Some(MSG_WIN) => {
                    let result = match m["player"].as_u64() {
                        Some(TEAM_DRAW) => DRAW,
                        _ if self.player(&m["player"]) == CLIENT => WON,
                        _ => LOST,
                    };
                    let reason = end_reason(m["reason"].as_u64().unwrap_or(1));
                    out.push(command(DUEL_END, [result, reason, 0, 0]));
                    self.result = Some(result);
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
        let to = Place { index, ..to };
        let row = self.shown_row(&to, card.row);
        let mut out = vec![command(CARD, [card.uid, row, 0, table_place(to.player, to.zone, index)])];
        if to.zone < 5 && row != 0 {
            out.extend(self.monster_record(to.player, to.zone, card.row));
        }
        out.push(command(MOVE, [card.uid | row << 16, from.packed(), to.packed(), 0]));
        // List zones renumber: re-send the shifted cards (not the face-down deck).
        for (p, zone, start) in [(from.player, from.zone, at), (to.player, to.zone, index + 1)] {
            if zone <= FIELD_SPELL || zone == DECK {
                continue;
            }
            let shifted: Vec<(usize, Card)> = self.sides[p as usize].zone(zone).iter().copied().enumerate().skip(start).collect();
            for (i, c) in shifted {
                let row = self.shown_row(&Place { player: p, zone, index: i, face_up: true, defense: false }, c.row);
                out.push(command(CARD, [c.uid, row, 0, table_place(p, zone, i)]));
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
            MSG_SELECT_EFFECTYN | MSG_SELECT_YESNO => {
                let entries = match (kind, m["cid"].as_u64(), m["description_cid"].as_u64()) {
                    (MSG_SELECT_EFFECTYN, cid, _) => vec![TEXT | ACTIVATE_TEXT, CARD_NAME | cid.unwrap_or(0) as u32],
                    (_, _, Some(cid)) => vec![TEXT | USE_TEXT, CARD_NAME | cid as u32],
                    _ => vec![TEXT | USE_NOW_TEXT],
                };
                let mut out = message(&entries);
                out.push(command(DIALOG, [YES_NO, FROM_ENTRIES, 0, 0]));
                self.pending = Some(m.clone());
                Prompt::Client(out)
            }
            MSG_SELECT_CARD | MSG_SELECT_TRIBUTE => {
                let hint = if kind == MSG_SELECT_TRIBUTE { HINTMSG_RELEASE } else { std::mem::take(&mut self.hint) };
                let out = self.card_list(m, hint_text(hint));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            MSG_SELECT_POSITION => match self.position_dialog(m) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            MSG_SELECT_OPTION => match option_list(m) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            MSG_SELECT_CHAIN => self.chain_chance(m),
            // No window for these yet.
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// The position dialog for MSG_SELECT_POSITION {cid, positions}, with the card's image (any uid with its id) and
    /// name. None without an attack position: the first button is always attack.
    fn position_dialog(&self, m: &Json) -> Option<Vec<Message>> {
        let allowed = m["positions"].as_u64().unwrap_or(0);
        if allowed & (POS_FACEUP_ATTACK | POS_FACEDOWN_ATTACK) == 0 {
            return None;
        }
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
        let has_cid = |c: &&Card| self.cards.get(&c.row).map(|s| s.cid) == Some(cid);
        let uid = self.sides.iter().flat_map(|s| s.zones.values().flatten()).find(has_cid);
        let face_up = if allowed & POS_FACEUP_DEFENSE != 0 { FACE_UP_DEFENSE } else { 0 };
        let mut out = message(&[TEXT | POSITION_TEXT, CARD_NAME | cid as u32]);
        out.push(command(DIALOG, [POSITION, FROM_ENTRIES, uid.map_or(0, |c| c.uid) | face_up, 0]));
        Some(out)
    }

    /// MSG_SELECT_CHAIN {spe_count, forced, hint_timing, selects}: the question shows the cards, names the chain's last
    /// link, and says how much it matters, so the player's Activation Message option decides whether it shows. A forced
    /// chain of several goes straight to the card list, of one needs no choice.
    fn chain_chance(&mut self, m: &Json) -> Prompt {
        let selects = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let forced = m["forced"] == true;
        if selects.is_empty() {
            return Prompt::Answer(json!({ "type": RESP_CHAIN, "index": null }));
        }
        if forced && selects.len() == 1 {
            return Prompt::Answer(json!({ "type": RESP_CHAIN, "index": 0 }));
        }
        self.pending = Some(m.clone());
        self.picks.clear();
        self.chain_list = forced;
        if forced {
            return Prompt::Client(self.card_list(m, Some(SELECT_EFFECT_TEXT)));
        }
        let rows: Vec<u32> = selects.iter().map(|loc| self.row_of(loc) as u32).collect();
        let mut out = vec![command(LIST_BEGIN, [rows.len() as u32, 0, WITH_CARDS, 0])];
        out.extend(fours(LIST_ROWS, &rows));
        let entries = match &self.chaining {
            Some(link) => {
                let spell_trap = link["location"].as_u64() == Some(LOC_SZONE);
                let text = if spell_trap { CHAIN_CARD_TEXT } else { CHAIN_EFFECT_TEXT };
                vec![TEXT | text, CARD_NAME | link["cid"].as_u64().unwrap_or(0) as u32]
            }
            None => vec![TEXT | ACTIVATE_CARD_TEXT],
        };
        out.extend(message(&entries));
        let timing = m["hint_timing"].as_u64().unwrap_or(0) | m["hint_timing_other"].as_u64().unwrap_or(0);
        let level = if m["spe_count"].as_u64().unwrap_or(0) == 0 {
            LEVEL_UNFIT
        } else if self.chaining.is_some() || timing & !QUIET_TIMINGS != 0 {
            LEVEL_EVENT
        } else {
            LEVEL_PHASE
        };
        out.push(command(DIALOG, [EFFECT, FROM_ENTRIES, level, 0]));
        Prompt::Client(out)
    }

    /// The card list for a card choice: its rows and marks, the message, then the dialog. The kind gives the picks OK
    /// needs: none for min 0, one for min 1, else exactly min (the list's max is lowered to it).
    fn card_list(&self, m: &Json, text: Option<u32>) -> Vec<Message> {
        let selects = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let (min, max) = (m["min"].as_u64().unwrap_or(1) as u32, m["max"].as_u64().unwrap_or(1) as u32);
        let (kind, max) = match min {
            0 => (PICK_ANY, max),
            1 => (PICK_ONE, max),
            _ => (PICK_ALL, min),
        };
        let rows: Vec<u32> = selects.iter().map(|loc| self.list_row(loc)).collect();
        let marks: Vec<u32> = selects.iter().map(|loc| if self.player(&loc["controller"]) == CPU { OPPONENTS } else { 0 }).collect();
        let mut out = vec![command(LIST_BEGIN, [selects.len() as u32, max, 0, 0])];
        out.extend(fours(LIST_ROWS, &rows));
        out.extend(fours(LIST_MARKS, &marks));
        if let Some(text) = text {
            out.extend(message(&[TEXT | text]));
        }
        out.push(command(CARD_LIST, [CLIENT, kind, text.is_some() as u32, 0]));
        out
    }

    /// A card's row as a list shows it: none for the opponent's hidden cards when hiding.
    fn list_row(&self, loc: &Json) -> u32 {
        let row = self.row_of(loc);
        self.engine_place(loc).map_or(row as u32, |place| self.shown_row(&place, row))
    }

    /// A card's CARD_Prop row: from the mirror at its place (any deck card with its id for the deck), else the
    /// first row with its id.
    fn row_of(&self, loc: &Json) -> u16 {
        let cid = loc["cid"].as_u64().unwrap_or(0) as u16;
        let has_cid = |c: &&Card| self.cards.get(&c.row).map(|s| s.cid) == Some(cid);
        let at_place = self.engine_place(loc).and_then(|p| {
            let zone = self.sides[p.player as usize].zones.get(&p.zone)?;
            if p.zone == DECK { zone.iter().find(has_cid) } else { zone.get(p.index).filter(has_cid) }.map(|c| c.row)
        });
        at_place.or_else(|| self.cards.iter().find(|(_, s)| s.cid == cid).map(|(&row, _)| row)).unwrap_or(0)
    }

    /// A pick in the card list (duelRequestName 7 {index: the list position}).
    pub fn pick(&mut self, index: u32) {
        self.picks.push(index as usize);
    }

    /// A dialog closed (duelRequestName 4 {DLGResult}): the engine response for the pending question (the button: 1
    /// yes, 0 attack / 1 defense), option (its index) or card choice (`count` picks), or for a chain chance's Yes with
    /// several cards, the card list. No pick cancels when the engine allows it; picks that don't fit the choice get
    /// the default.
    pub fn dialog_result(&mut self, count: u32) -> Option<Prompt> {
        let kind = self.pending.as_ref()?["type"].as_u64()?;
        const DIALOGS: [u64; 7] = [
            MSG_SELECT_CARD,
            MSG_SELECT_TRIBUTE,
            MSG_SELECT_EFFECTYN,
            MSG_SELECT_YESNO,
            MSG_SELECT_POSITION,
            MSG_SELECT_OPTION,
            MSG_SELECT_CHAIN,
        ];
        if !DIALOGS.contains(&kind) {
            return None;
        }
        let m = self.pending.take()?;
        let selects = m["selects"].as_array().map_or(0, Vec::len);
        let answer = match kind {
            MSG_SELECT_EFFECTYN => json!({ "type": RESP_EFFECTYN, "yes": count == 1 }),
            MSG_SELECT_YESNO => json!({ "type": RESP_YESNO, "yes": count == 1 }),
            MSG_SELECT_POSITION => {
                let allowed = m["positions"].as_u64().unwrap_or(0);
                let position = match count {
                    0 if allowed & POS_FACEUP_ATTACK != 0 => POS_FACEUP_ATTACK,
                    0 => POS_FACEDOWN_ATTACK,
                    _ if allowed & POS_FACEUP_DEFENSE != 0 => POS_FACEUP_DEFENSE,
                    _ => POS_FACEDOWN_DEFENSE,
                };
                match allowed & position {
                    0 => cpu_answer(kind, &m),
                    _ => json!({ "type": RESP_POSITION, "position": position }),
                }
            }
            MSG_SELECT_OPTION if (count as usize) < m["options"].as_array().map_or(0, Vec::len) => {
                json!({ "type": RESP_OPTION, "index": count })
            }
            MSG_SELECT_CHAIN if !self.chain_list && count == 1 && selects > 1 => {
                self.chain_list = true;
                self.picks.clear();
                let out = self.card_list(&m, Some(SELECT_EFFECT_TEXT));
                self.pending = Some(m);
                return Some(Prompt::Client(out));
            }
            MSG_SELECT_CHAIN if !self.chain_list => {
                let index = if count == 1 { json!(0) } else { Json::Null };
                json!({ "type": RESP_CHAIN, "index": index })
            }
            MSG_SELECT_CHAIN => match std::mem::take(&mut self.picks)[..] {
                [index] if index < selects => json!({ "type": RESP_CHAIN, "index": index }),
                _ if m["forced"] == true => cpu_answer(kind, &m),
                _ => json!({ "type": RESP_CHAIN, "index": null }),
            },
            MSG_SELECT_CARD | MSG_SELECT_TRIBUTE => self.card_choice(&m, count),
            _ => cpu_answer(kind, &m),
        };
        Some(Prompt::Answer(answer))
    }

    /// The response for a closed card list of MSG_SELECT_CARD or MSG_SELECT_TRIBUTE.
    fn card_choice(&mut self, m: &Json, count: u32) -> Json {
        let kind = m["type"].as_u64().unwrap_or(0);
        let response = if kind == MSG_SELECT_TRIBUTE { RESP_TRIBUTE } else { RESP_CARD };
        let picks = std::mem::take(&mut self.picks);
        let len = m["selects"].as_array().map_or(0, Vec::len);
        let (min, max) = (m["min"].as_u64().unwrap_or(1) as usize, m["max"].as_u64().unwrap_or(1) as usize);
        if count == 0 && picks.is_empty() && m["can_cancel"] == true {
            return json!({ "type": response, "indicies": null });
        }
        let mut seen = vec![];
        let fits = picks.iter().all(|&i| i < len && !seen.contains(&i) && { seen.push(i); true });
        if fits && (min..=max).contains(&picks.len()) {
            return json!({ "type": response, "indicies": picks });
        }
        tracing::warn!("card list: picks {picks:?} don't fit min {min} max {max} of {len}");
        cpu_answer(kind, m)
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

/// A data command per four values, which the pop appends in order (0x522A50).
fn fours(id: u32, values: &[u32]) -> Vec<Message> {
    let command_of = |four: &[u32]| {
        let mut params = [0; 4];
        params[..four.len()].copy_from_slice(four);
        command(id, params)
    };
    values.chunks(4).map(command_of).collect()
}

/// The option list for MSG_SELECT_OPTION {options}, when the client has a label for every option (at most four).
/// A card's own strings (code << 4 | n) have none.
fn option_list(m: &Json) -> Option<Vec<Message>> {
    let options = m["options"].as_array()?;
    let found: Option<Vec<(u32, u32)>> = options
        .iter()
        .map(|o| OPTION_LABELS.iter().find(|(d, _, _)| o.as_u64() == Some(*d)).map(|&(_, label, title)| (label, title)))
        .collect();
    let found = found.filter(|f| !f.is_empty() && f.len() <= SELECT_MAX)?;
    let labels: Vec<u32> = found.iter().map(|(label, _)| label | LISTED).collect();
    let mut out = vec![command(LIST_BEGIN, [labels.len() as u32, 0, 0, 0])];
    out.extend(fours(LIST_VALUES, &labels));
    out.extend(message(&[TEXT | found[0].1]));
    out.push(command(DIALOG, [SELECT, FROM_ENTRIES, 0, 0]));
    Some(out)
}

/// The message entries (at most four) a dialog shows: 0x3EC {count}, 0x3ED {entries}.
fn message(entries: &[u32]) -> Vec<Message> {
    let mut params = [0; 4];
    params[..entries.len()].copy_from_slice(entries);
    vec![command(MESSAGE_LENGTH, [entries.len() as u32, 0, 0, 0]), command(MESSAGE, params)]
}

/// The duel prompt table's text for an EDOPro selection hint (HINTMSG_*), where one says the same thing.
fn hint_text(hint: u64) -> Option<u32> {
    Some(match hint {
        HINTMSG_RELEASE => TRIBUTE_TEXT,
        503 => 70,  // HINTMSG_REMOVE: "Select @3card(s)@0 to @3remove from play@0."
        504 => 58,  // HINTMSG_TOGRAVE: "Select the @3card@0 to send to the @2Graveyard@0."
        506 => 88,  // HINTMSG_ATOHAND: "Select the @3card(s)@0 to add to your @2hand@0."
        509 => 25,  // HINTMSG_SPSUMMON: "Select the @3monster(s)@0 to @3Special Summon@0."
        549 => 222, // HINTMSG_ATTACKTARGET: "Select an attack target @3monster@0."
        551 => 94,  // HINTMSG_TARGET: "Select the @3card(s)@0 to target."
        _ => return None,
    })
}

/// The client's duel-end reason (text 1999 + reason, 0x508650) for an EDOPro win reason.
fn end_reason(reason: u64) -> u32 {
    match reason {
        0 => 5,     // "has surrendered"
        1 => 1,     // "Life Points have been reduced to 0"
        2 => 2,     // "has run out of cards in the Deck"
        3 => 11,    // "has ran out of time"
        4 => 14,    // "has disconnected"
        0x10 => 3,  // Exodia the Forbidden One
        0x11 => 7,  // Final Countdown
        0x12 => 9,  // Vennominaga the Deity of Poisonous Snakes
        0x14 => 10, // Exodius the Ultimate Forbidden Lord
        0x15 => 4,  // Destiny Board
        0x16 => 6,  // Last Turn
        _ => 8,     // "has won with" a card
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
