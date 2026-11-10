//! The duel check: what the client holds after the duel commands the server sent it, compared with where the rules
//! engine has every card. The translator keeps its own mirror of the client; this is a second, independent model of
//! the client built only from the client's code, so a translator that drifts from the client (or from the engine) is
//! caught at the first step where they differ, before the client crashes on it (the null read at 0x516185: a hand card
//! that left by the wrong index stayed in the client's hand list).
//!
//! What the model follows, all in DuelCardManager (0x4FF000 dispatches the card commands):
//! - 1000 sets a card table entry {uid | flag << 24, row, 0, player | zone << 8 | index << 16} (0x521DF0); the client
//!   finds a card by place in that table (0x523DF0) and by uid among its card objects (0x500470).
//! - 0x1A CardMove {uid | row << 16, from, to} (case 0x1A): the card object by uid, made when missing (0x5003A0), then
//!   0x5000E0. A field zone (0-10) it leaves loses whatever card it held, the one it lands on holds it (the slot array
//!   at +0x4C); a hand it leaves loses the card at the move's index (0x5005D0, by index, not uid), a hand it lands in
//!   gets it at the end (0x52A400, a push_back), whatever index the move names; the hand is renumbered by position
//!   (0x500640). A pile (Extra Deck 0xC, Deck 0xD, Graveyard 0xE, banished 0xF) counts down when a card leaves it
//!   (0x4FE4D0 with the count less one) and up when one lands (0x4FBF30). The card takes the move's `to` as its place
//!   and position (+0x82: face up bit 14, defense bit 15).
//! - 0x16 DeckReset {player, pile} sets the pile's count from the client's per-zone count (0x5240F0), which starts at
//!   the setup's MainDeckNum and ExtraDeckNum (pump case 0x15) and follows the moves.
//! - 0x12 HandShuffle {player} rebuilds the hand from the table, index by index (0x523DF0(player, 0xB, i)); an index
//!   with no entry stops it with an error state (0x1200).
//! - 0x1B CardSwap {_, place A, place B}, both field places: the cards in the two slots trade places (case 0x1B: the
//!   objects by 0x4F8240, 0x5000E0 with param_6 1, the slots swapped), each taking the word of the place it lands on.
//! - 0x1C CardFlipTurn {place}: the field card there takes the place's face and position (+0x82).
//! - 0x1F CardVanish {player, zone | index << 8}, zone below 0xC: the card is marked off the field (+0xD7) until it
//!   lands again (0x4FBF30).
//! - BattleAttack 0xC remembers both cards' slots; BattleRun 0xD's destroy bits hide the corresponding cards
//!   (0x4F8920 -> 0x4FB220 mode 0), while leaving their slots occupied. BattleEnd does not restore visibility.
//! - 1017 (pump 0x3F9) {source place, target place, marker type, 1 add / 0 remove}, places `player | zone << 8`: the
//!   relation list (0x5224E0 appends; 0x522530 removes the entry matching all three). Every relation must join two
//!   field slots that hold a card, or the client draws a marker on an empty slot when the source is hovered.
//!
//! Where the model itself sees the client go wrong (a hand card taken out by an index that holds another card, a
//! shuffle with an empty index) it notes a problem at once; [`ClientModel::check`] then compares every hand, field
//! slot and pile with the engine's `state` (see `engine/duelcore.ts`). Cards whose row the client was never given
//! (an opponent's hidden cards) are compared by count and place only, unless [`ClientModel::know`] seeded their row.
use std::collections::{HashMap, HashSet, VecDeque};

use serde_json::Value as Json;
use yo3_proto::{Message, Value};

use crate::translate::CardStats;
pub mod native;
pub mod coverage;

const SERVER_DUEL: u32 = crate::duel::SERVER_DUEL;
/// Pump cases (duelRequestName): an engine command, the duel setup.
const ENGINE_COMMAND: u32 = 0x11;
const DUEL_SETUP: u32 = 0x15;
/// Card commands the model follows.
const CARD: u32 = 1000;
const MOVE: u32 = 0x1A;
const HAND_SHUFFLE: u32 = 0x12;
const DECK_RESET: u32 = 0x16;
const CARD_SWAP: u32 = 0x1B;
const FLIP_TURN: u32 = 0x1C;
const CARD_VANISH: u32 = 0x1F;
const BATTLE_INIT: u32 = 0xA;
const BATTLE_ATTACK: u32 = 0xC;
const BATTLE_RUN: u32 = 0xD;
const BATTLE_END: u32 = 0xE;
const RELATION: u32 = 0x3F9;
/// Zones: field slots 0-10 (monsters 0-4, Spell/Trap 5-9, Field Spell 10), then the hand and the piles.
const FIELD_SPELL: u32 = 10;
const HAND: u32 = 0xB;
const EXTRA: u32 = 0xC;
const DECK: u32 = 0xD;
const GRAVE: u32 = 0xE;
const REMOVED: u32 = 0xF;
/// The uids a mixed card list puts in the table for the time it is open (`translate.rs` STAGED_UID), and the first
/// token uid.
const STAGED: u32 = 0xF0_0000;
const FIRST_TOKEN: u32 = 0x4000;
/// How many applied commands a problem report shows.
const RECENT: usize = 12;

/// A move place: player | zone << 1 | index << 6 | face up << 14 | defense << 15.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    player: usize,
    zone: u32,
    index: usize,
    face_up: bool,
    defense: bool,
}

impl Place {
    fn of_move(word: u32) -> Place {
        Place {
            player: (word & 1) as usize,
            zone: word >> 1 & 0x1F,
            index: (word >> 6 & 0xFF) as usize,
            face_up: word >> 14 & 1 == 1,
            defense: word >> 15 & 1 == 1,
        }
    }

    fn pile(self) -> Option<usize> {
        (EXTRA..=REMOVED).contains(&self.zone).then(|| (self.zone - EXTRA) as usize)
    }
}

/// A card table place: player | zone << 8 | index << 16.
fn table_place(player: usize, zone: u32, index: usize) -> u32 {
    player as u32 | zone << 8 | (index as u32) << 16
}

/// What a client holds of the duel, as its card manager would after the same commands.
pub struct ClientModel {
    /// The client's player of engine team 0.
    first: usize,
    cards: HashMap<u16, CardStats>,
    /// Each card's row as far as known (any 1000 with a row, or [`know`](Self::know)), and its table place.
    rows: HashMap<u32, u16>,
    table: HashMap<u32, u32>,
    hands: [Vec<u32>; 2],
    /// Field slots by player and zone 0-10: the card object there (uid).
    slots: [[Option<u32>; 11]; 2],
    /// Each card's place word as last given (its face and position), and the cards marked off the field.
    places: HashMap<u32, Place>,
    off_field: HashSet<u32>,
    battle: [Option<(usize, usize)>; 2],
    lp: [Option<i64>; 2],
    record: [u32; 9],
    records: HashMap<(usize, u32), [u32; 9]>,
    counters: HashMap<(u32, u32), i64>,
    turn: (usize, u32),
    phase: Option<(usize, u32)>,
    engine_turn: (usize, u32),
    engine_phase: Option<(usize, u32)>,
    coverage_gaps: HashSet<u64>,
    /// The relations the client holds: (source place, target place, marker type).
    relations: Vec<(u32, u32, u32)>,
    /// Per player, the per-zone counts (0x5240F0) and the piles' shown counts, Extra Deck to banished.
    counts: [[i64; 4]; 2],
    piles: [[i64; 4]; 2],
    problems: Vec<String>,
    recent: VecDeque<String>,
}

impl ClientModel {
    /// A model for a client that is player `first` of engine team 0, with the client's card data (rows' Konami ids).
    pub fn new(first: u32, cards: HashMap<u16, CardStats>) -> ClientModel {
        ClientModel {
            first: first as usize,
            cards,
            rows: HashMap::new(),
            table: HashMap::new(),
            hands: Default::default(),
            slots: Default::default(),
            places: HashMap::new(),
            off_field: HashSet::new(),
            battle: [None; 2],
            lp: [None; 2],
            record: [0; 9],
            records: HashMap::new(),
            counters: HashMap::new(),
            turn: (first as usize, 1), phase: None,
            engine_turn: (first as usize, 0), engine_phase: None,
            coverage_gaps: HashSet::new(),
            relations: vec![],
            counts: Default::default(),
            piles: Default::default(),
            problems: vec![],
            recent: VecDeque::new(),
        }
    }

    /// Seeds a card's row that the client is not given (an opponent's hidden card), so its place can be checked.
    pub fn know(&mut self, uid: u32, row: u16) {
        if row != 0 {
            self.rows.insert(uid, row);
        }
    }

    /// Follows the commands sent to the client, in order.
    pub fn apply_all<'a>(&mut self, messages: impl IntoIterator<Item = &'a Message>) {
        for m in messages {
            self.apply(m);
        }
    }

    /// Engine events form the independent turn/phase oracle and audit the engine side of the translation boundary.
    pub fn observe_engine(&mut self, messages: &[Json]) {
        for m in messages {
            if m["type"] == "field" { continue; }
            let Some(kind) = m["type"].as_u64() else { self.problem(format!("unclassified engine event {}", m["type"])); continue };
            if coverage::engine(kind) == coverage::Coverage::Unknown {
                self.problem(format!("unclassified engine event {kind}"));
            } else if coverage::engine(kind) == coverage::Coverage::Unsupported && self.coverage_gaps.insert(kind) {
                tracing::warn!("[YO3 GAP] engine event {kind}: known unsupported presentation/interaction; state agreement does not validate it");
            }
            let player = if m["player"] == 0 { self.first } else { 1 - self.first };
            if kind == 40 { self.engine_turn = (player, self.engine_turn.1 + 1); }
            if kind == 41 {
                let phase = match m["phase"].as_u64() {
                    Some(1) => Some(0), Some(2) => Some(1), Some(4) => Some(2),
                    Some(8 | 16 | 32 | 64 | 128) => Some(3), Some(256) => Some(4), Some(512) => Some(5), _ => None,
                };
                if let Some(phase) = phase { self.engine_phase = Some((self.engine_turn.0, phase)); }
            }
        }
    }

    pub fn coverage_gaps(&self) -> &HashSet<u64> { &self.coverage_gaps }

    /// Follows one message sent to the client.
    pub fn apply(&mut self, m: &Message) {
        if m.request() != Some(SERVER_DUEL) {
            return;
        }
        let int = |k: &str| m.get(k).and_then(Value::as_u32).unwrap_or(0);
        match m.get("duelRequestName").and_then(Value::as_u32) {
            Some(DUEL_SETUP) => {
                // A new duel (a match's next one too): nothing of the last one stays.
                let rows = std::mem::take(&mut self.rows);
                let gaps = std::mem::take(&mut self.coverage_gaps);
                let oracle = (self.engine_turn, self.engine_phase);
                *self = ClientModel::new(self.first as u32, std::mem::take(&mut self.cards));
                self.rows = rows;
                self.coverage_gaps = gaps;
                (self.engine_turn, self.engine_phase) = oracle;
                self.counts = [
                    [int("ExtraDeckNum001") as i64, int("MainDeckNum001") as i64, 0, 0],
                    [int("ExtraDeckNum002") as i64, int("MainDeckNum002") as i64, 0, 0],
                ];
                self.note("setup".into());
            }
            Some(ENGINE_COMMAND) => {
                let p = [int("param1"), int("param2"), int("param3"), int("param4")];
                self.command(int("id"), p);
            }
            _ => {}
        }
    }

    fn note(&mut self, what: String) {
        if self.recent.len() == RECENT {
            self.recent.pop_front();
        }
        self.recent.push_back(what);
    }

    fn problem(&mut self, what: String) {
        let recent: Vec<_> = self.recent.iter().cloned().collect();
        self.problems.push(format!("{what} (after: {})", recent.join("; ")));
    }

    fn command(&mut self, id: u32, [p1, p2, p3, p4]: [u32; 4]) {
        match id {
            5 => self.phase = Some(((p1 & 1) as usize, p2)),
            6 => self.turn = ((p1 & 1) as usize, self.turn.1 + 1),
            0xF => self.lp[(p1 & 1) as usize] = Some(p2 as i64),
            0x10 => {
                if let Some(lp) = &mut self.lp[(p1 & 1) as usize] { *lp = (*lp + p2 as i32 as i64).clamp(0, 99999); }
            },
            0x3E9 => { self.record[0] = p2; self.record[2] = p3; self.record[3] = p4; },
            0x3EA => { self.record[4] = p2; self.record[5] = p3; self.record[6] = p4; },
            0x3EB => {
                self.record[7] = p2; self.record[8] = p3; self.record[1] = p4;
                self.records.insert(((p1 & 1) as usize, p1 >> 8 & 0xFF), self.record);
            },
            0x27 => {
                let (p, z) = ((p1 & 1) as usize, (p1 >> 8 & 0xFF) as usize);
                if let Some(uid) = self.slots[p].get(z).copied().flatten() {
                    let count = self.counters.entry((uid, p3)).or_default();
                    *count = (*count + p2 as i32 as i64).max(0);
                } else { self.problem(format!("counter change at empty player {p} zone {z}")); }
            },
            BATTLE_INIT | BATTLE_END => self.battle = [None; 2],
            BATTLE_ATTACK => {
                self.battle = [None; 2];
                let (attacker, zone, target) = ((p1 & 1) as usize, (p1 >> 8 & 0xFF) as usize, (p2 & 1) as usize);
                if zone < 5 { self.battle[attacker] = Some((attacker, zone)); }
                if p3 < 5 { self.battle[target] = Some((target, p3 as usize)); }
            },
            BATTLE_RUN => {
                self.note(format!("BattleRun flags {p3:#x}"));
                for player in 0..2 {
                    if p3 & (2 << (player * 8)) != 0
                        && let Some((side, zone)) = self.battle[player]
                        && let Some(uid) = self.slots[side][zone]
                    { self.off_field.insert(uid); }
                }
            },
            CARD => {
                let uid = p1 & 0xFF_FFFF;
                if p2 != 0 && uid < STAGED {
                    self.rows.insert(uid, p2 as u16);
                }
                self.table.insert(uid, p4);
                self.note(format!("1000 {{uid {uid}, row {p2}, place {p4:#x}}}"));
            }
            MOVE => {
                self.note(format!("0x1A {{uid {}, {:#x} -> {:#x}}}", p1 & 0xFFFF, p2, p3));
                self.card_move(p1 & 0xFFFF, Place::of_move(p2), Place::of_move(p3));
            }
            HAND_SHUFFLE => {
                self.note(format!("0x12 {{{p1}}}"));
                self.hand_shuffle((p1 & 1) as usize);
            }
            DECK_RESET => {
                let (player, zone) = ((p1 & 1) as usize, p2);
                if (EXTRA..=REMOVED).contains(&zone) {
                    let pile = (zone - EXTRA) as usize;
                    self.piles[player][pile] = self.counts[player][pile];
                }
            }
            CARD_SWAP => {
                let (a, b) = (Place::of_move(p2), Place::of_move(p3));
                self.note(format!("0x1B {{{p2:#x}, {p3:#x}}}"));
                if a.zone > FIELD_SPELL || b.zone > FIELD_SPELL {
                    self.problem(format!("CardSwap of {p2:#x} and {p3:#x}: the client swaps only field places"));
                    return;
                }
                let (in_a, in_b) = (self.slots[a.player][a.zone as usize], self.slots[b.player][b.zone as usize]);
                self.slots[a.player][a.zone as usize] = in_b;
                self.slots[b.player][b.zone as usize] = in_a;
                for (uid, place) in [(in_b, a), (in_a, b)] {
                    if let Some(uid) = uid {
                        if self.places.get(&uid).is_some_and(|old| old.face_up != place.face_up) {
                            self.counters.retain(|(card, _), _| *card != uid);
                        }
                        self.places.insert(uid, place);
                    }
                }
            }
            FLIP_TURN => {
                let place = Place::of_move(p1);
                if place.zone <= FIELD_SPELL
                    && let Some(uid) = self.slots[place.player][place.zone as usize]
                {
                    if self.places.get(&uid).is_some_and(|old| old.face_up != place.face_up) {
                        self.counters.retain(|(card, _), _| *card != uid);
                    }
                    self.places.insert(uid, place);
                }
                self.note(format!("0x1C {{{p1:#x}}}"));
            }
            CARD_VANISH => {
                let (player, zone, index) = ((p1 & 1) as usize, p2 & 0xFF, (p2 >> 8 & 0xFF) as usize);
                let uid = match zone {
                    z if z <= FIELD_SPELL => self.slots[player][z as usize],
                    HAND => self.hands[player].get(index).copied(),
                    _ => None,
                };
                if let Some(uid) = uid {
                    self.off_field.insert(uid);
                }
                self.note(format!("0x1F {{{p1}, {p2:#x}}}"));
            }
            RELATION => {
                self.note(format!("1017 {{{p1:#x}, {p2:#x}, {p3}, {p4}}}"));
                if p4 != 0 {
                    self.relations.push((p1, p2, p3));
                } else if let Some(at) = self.relations.iter().position(|&r| r == (p1, p2, p3)) {
                    self.relations.remove(at);
                } else {
                    self.problem(format!("1017 removes a relation the client doesn't hold: {p1:#x} -> {p2:#x}, type {p3}"));
                }
            }
            _ if matches!(coverage::native(id), coverage::Coverage::Interaction | coverage::Coverage::Presentation) => {},
            _ => self.problem(format!("unclassified native command {id:#x}")),
        }
    }

    /// 0x5000E0 for one card (param_6 0, as case 0x1A calls it).
    fn card_move(&mut self, uid: u32, from: Place, to: Place) {
        if from.zone <= FIELD_SPELL && from.face_up != to.face_up {
            self.counters.retain(|(card, _), _| *card != uid);
        }
        if from.zone <= FIELD_SPELL {
            self.slots[from.player][from.zone as usize] = None;
        }
        if to.zone <= FIELD_SPELL {
            self.slots[to.player][to.zone as usize] = Some(uid);
        }
        if from.zone == HAND {
            let hand = &mut self.hands[from.player];
            match hand.get(from.index).copied() {
                None => {
                    let n = hand.len();
                    self.problem(format!("uid {uid} leaves hand {} at index {} of a hand of {n}", from.player, from.index));
                }
                Some(there) => {
                    hand.remove(from.index);
                    if there != uid {
                        // The crash's cause: the card that left stays in the hand list, another one goes.
                        self.problem(format!(
                            "uid {uid} leaves hand {} by index {}, which holds uid {there}: the client drops uid {there} and keeps uid {uid}",
                            from.player, from.index
                        ));
                    }
                }
            }
        }
        if to.zone == HAND {
            if self.hands.iter().any(|h| h.contains(&uid)) {
                self.problem(format!("uid {uid} reaches hand {} while a hand list still holds it", to.player));
            }
            self.hands[to.player].push(uid);
        }
        if let Some(pile) = from.pile() {
            self.counts[from.player][pile] -= 1;
            self.piles[from.player][pile] -= 1;
        }
        if let Some(pile) = to.pile() {
            self.counts[to.player][pile] += 1;
            self.piles[to.player][pile] += 1;
        }
        self.places.insert(uid, to);
        self.off_field.remove(&uid);
    }

    /// The uids whose table entry is at a place.
    fn at_table(&self, place: u32) -> Vec<u32> {
        let mut uids: Vec<u32> = self.table.iter().filter(|(_, p)| **p == place).map(|(u, _)| *u).collect();
        uids.sort_unstable();
        uids
    }

    fn hand_shuffle(&mut self, player: usize) {
        let mut rebuilt = vec![];
        for i in 0..self.hands[player].len() {
            match self.at_table(table_place(player, HAND, i))[..] {
                [uid] => rebuilt.push(uid),
                [] => {
                    self.problem(format!("HandShuffle {player}: no table entry at hand index {i} (the client's error 0x1200)"));
                    return;
                }
                ref several => {
                    self.problem(format!("HandShuffle {player}: hand index {i} has several table entries {several:?}"));
                    return;
                }
            }
        }
        self.hands[player] = rebuilt;
    }

    /// The problems the model noted while following the commands (each once).
    pub fn take_problems(&mut self) -> Vec<String> {
        std::mem::take(&mut self.problems)
    }

    /// A card's Konami id as far as the client's data knows it (None for a token or an unknown row).
    fn cid(&self, uid: u32) -> Option<u16> {
        if uid >= FIRST_TOKEN {
            return None;
        }
        self.rows.get(&uid).and_then(|r| self.cards.get(r)).map(|c| c.cid)
    }

    fn describe(&self, uid: u32) -> String {
        match self.cid(uid) {
            Some(cid) => format!("uid {uid} (card {cid})"),
            None if uid >= FIRST_TOKEN => format!("token uid {uid}"),
            None => format!("uid {uid}"),
        }
    }

    /// Compares the client with the engine's `state` (by engine team). Returns every difference, after the problems
    /// noted while following the commands; empty when they agree.
    pub fn check(&mut self, state: &Json) -> Vec<String> {
        let mut out = self.take_problems();
        if self.engine_turn.1 > 0 && self.turn != self.engine_turn {
            out.push(format!("turn: engine {:?}, client {:?}", self.engine_turn, self.turn));
        }
        if self.engine_phase.is_some() && self.phase != self.engine_phase {
            out.push(format!("phase: engine {:?}, client {:?}", self.engine_phase, self.phase));
        }
        let Some(teams) = state.as_array() else {
            out.push(format!("no engine state: {state}"));
            return out;
        };
        let recent: Vec<_> = self.recent.iter().cloned().collect();
        let mut found = vec![];
        for (team, side) in teams.iter().enumerate().take(2) {
            let p = if team == 0 { self.first } else { 1 - self.first };
            found.extend(self.check_side(p, side));
        }
        let held = |place: u32| {
            let (player, zone) = ((place & 1) as usize, (place >> 8 & 0xFF) as usize);
            zone <= FIELD_SPELL as usize && self.slots[player][zone].is_some_and(|uid| !self.off_field.contains(&uid))
        };
        for &(a, b, mark) in &self.relations {
            if !held(a) || !held(b) {
                found.push(format!("a relation joins an empty slot: {a:#x} -> {b:#x}, type {mark}"));
            }
        }
        out.extend(found.into_iter().map(|f| format!("{f} (after: {})", recent.join("; "))));
        out
    }

    fn check_side(&self, p: usize, side: &Json) -> Vec<String> {
        let mut out = vec![];
        if let Some(expected) = side["lp"].as_i64() && self.lp[p] != Some(expected) {
            out.push(format!("player {p}'s LP: engine {expected}, client {:?}", self.lp[p]));
        }
        let list = |k: &str| side[k].as_array().cloned().unwrap_or_default();
        let cid_of = |c: &Json| c["cid"].as_u64().unwrap_or(0) as u16;
        let token = |c: &Json| c["cid"].as_u64().unwrap_or(0) == 0;
        // The hand: same cards in the same order, and the table's hand entries on them.
        let hand = list("hand");
        let mine = &self.hands[p];
        if hand.len() != mine.len() {
            out.push(format!("player {p}'s hand: the engine has {} cards, the client {}", hand.len(), mine.len()));
        }
        for (i, (engine, &uid)) in hand.iter().zip(mine).enumerate() {
            if let Some(cid) = self.cid(uid)
                && cid != cid_of(engine)
            {
                out.push(format!("player {p}'s hand index {i}: the engine has card {}, the client {}", cid_of(engine), self.describe(uid)));
            }
        }
        for (i, &uid) in mine.iter().enumerate() {
            let entries = self.at_table(table_place(p, HAND, i));
            if entries != [uid] {
                out.push(format!("player {p}'s hand index {i}: the client holds {}, its table has {entries:?} there", self.describe(uid)));
            }
        }
        // The field: each slot as the engine has it, with its face (and a monster's position).
        let mzone = list("mzone");
        let szone = list("szone");
        let mut slots: Vec<(u32, Json)> = (0..5).map(|s| (s as u32, mzone.get(s).cloned().unwrap_or(Json::Null))).collect();
        slots.extend((0..5).map(|s| (5 + s as u32, szone.get(s).cloned().unwrap_or(Json::Null))));
        slots.push((FIELD_SPELL, szone.get(5).cloned().unwrap_or(Json::Null)));
        for (seq, c) in mzone.iter().enumerate().skip(5).chain(szone.iter().enumerate().skip(6)) {
            if !c.is_null() {
                out.push(format!("player {p} has card {} at an engine place the client has no slot for (sequence {seq})", cid_of(c)));
            }
        }
        for (zone, engine) in slots {
            let held = self.slots[p][zone as usize].filter(|u| !self.off_field.contains(u));
            match (held, engine.is_null()) {
                (None, true) => {}
                (Some(uid), true) => out.push(format!("player {p}'s zone {zone}: empty in the engine, the client shows {}", self.describe(uid))),
                (None, false) => out.push(format!("player {p}'s zone {zone}: the engine has card {}, the client nothing", cid_of(&engine))),
                (Some(uid), false) => {
                    let same = if uid >= FIRST_TOKEN { token(&engine) } else { self.cid(uid).is_none_or(|c| c == cid_of(&engine)) };
                    if !same {
                        out.push(format!("player {p}'s zone {zone}: the engine has card {}, the client {}", cid_of(&engine), self.describe(uid)));
                        continue;
                    }
                    let position = engine["position"].as_u64().unwrap_or(0);
                    let (face_up, defense) = (position & 5 != 0, position & 12 != 0);
                    if let Some(place) = self.places.get(&uid)
                        && (place.face_up != face_up || (zone < 5 && place.defense != defense))
                    {
                        out.push(format!(
                            "player {p}'s zone {zone}: {} is {} {} in the engine, the client shows it {} {}",
                            self.describe(uid),
                            if face_up { "face-up" } else { "face-down" },
                            if defense { "defense" } else { "attack" },
                            if place.face_up { "face-up" } else { "face-down" },
                            if place.defense { "defense" } else { "attack" },
                        ));
                    }
                    if zone < 5 && face_up {
                        for (key, index) in [("level", 4), ("attack", 5), ("defense", 6), ("base_attack", 7), ("base_defense", 8)] {
                            if let Some(mut value) = engine[key].as_i64() {
                                if key == "level" && value == 0 {
                                    value = self.rows.get(&uid).and_then(|row| self.cards.get(row)).map_or(0, |c| c.level as i64);
                                }
                                let shown = self.records.get(&(p, zone)).map(|r| r[index]);
                                if shown != Some(value as u32) {
                                    out.push(format!("player {p}'s zone {zone} {key}: engine {value}, client {shown:?}"));
                                }
                            }
                        }
                    }
                    if let Some(counters) = engine["counters"].as_object() {
                        let mut expected = HashMap::<u32, i64>::new();
                        for (kind, count) in counters {
                            let native = crate::translate::COUNTER_KINDS.iter().find(|(id, _)| Some(*id) == kind.parse().ok()).map_or(38, |(_, k)| *k);
                            *expected.entry(native).or_default() += count.as_i64().unwrap_or(0);
                        }
                        let shown: HashMap<_, _> = self.counters.iter().filter(|((card, _), n)| *card == uid && **n != 0)
                            .map(|((_, kind), n)| (*kind, *n)).collect();
                        expected.retain(|_, n| *n != 0);
                        if shown != expected { out.push(format!("player {p}'s zone {zone} counters: engine {expected:?}, client {shown:?}")); }
                    }
                }
            }
        }
        // The piles: counts, and for the Graveyard (whose top card and views come from the table) each index's entry.
        for (key, zone) in [("extra", EXTRA), ("grave", GRAVE), ("removed", REMOVED)] {
            let n = list(key).len() as i64;
            let shown = self.piles[p][(zone - EXTRA) as usize];
            if n != shown {
                out.push(format!("player {p}'s {key} pile: the engine has {n} cards, the client counts {shown}"));
            }
        }
        let deck = side["deck"].as_i64().unwrap_or(0);
        if deck != self.piles[p][(DECK - EXTRA) as usize] {
            out.push(format!("player {p}'s deck: the engine has {deck} cards, the client counts {}", self.piles[p][(DECK - EXTRA) as usize]));
        }
        for (i, engine) in list("grave").iter().enumerate() {
            match self.at_table(table_place(p, GRAVE, i))[..] {
                [uid] if self.cid(uid).is_none_or(|c| c == cid_of(engine)) => {}
                ref entries => out.push(format!(
                    "player {p}'s Graveyard index {i}: the engine has card {}, the table has {:?}",
                    cid_of(engine),
                    entries.iter().map(|u| self.describe(*u)).collect::<Vec<_>>()
                )),
            }
        }
        out
    }
}

/// Self-play for the duel check: two seeded random players duel through the rules engine and a translator, and the
/// client model is compared with the engine after every engine step (see [`play`]). The tests run it over many seeds and
/// deck mixes; it needs the engine (`engine/fetch.sh`).
pub mod selfplay {
    use std::collections::{BTreeMap, HashMap};
    use std::path::Path;

    use serde_json::{Value as Json, json};

    use super::ClientModel;
    use crate::duel::{CLIENT, CPU, DUEL_START, SET_LP, command, next, piles, setup, shuffle};
    use crate::engine::Engine;
    use crate::front::Deck;
    use crate::translate::{CardStats, Prompt, Translator, bot_answer, settle_positions};

    /// ocgcore message types the harness looks at.
    const MSG_RETRY: u64 = 1;
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
    const MSG_SELECT_UNSELECT_CARD: u64 = 26;
    const MSG_NEW_TURN: u64 = 40;
    /// The Sheep Tokens' rows in the harness's card data (the client's token cards are 3900-3999; any rows do here).
    const SHEEP_ROWS: [u16; 4] = [60001, 60002, 60003, 60004];
    /// When a self-played duel stops: engine steps, turns.
    const MAX_STEPS: usize = 1500;
    const MAX_TURNS: u32 = 24;
    /// Main phase or battle actions a player takes in one turn before moving on.
    const ACTIONS_PER_TURN: u32 = 12;

    /// The card pool (Konami ids) by theme. Searches and discards churn the hand, control changes and swaps move cards
    /// between sides, flips and hats turn and shuffle Set cards, tokens appear and vanish.
    pub const MONSTERS: [u16; 31] = [
        4054, // Sangan
        4580, // Witch of the Black Forest
        6960, // Gene-Warped Warwolf
        5004, // Vorse Raider
        4930, // Mystic Tomato
        4915, // Giant Rat
        4597, // Morphing Jar
        4913, // Cyber Jar
        4608, // Penguin Soldier
        4507, // Man-Eater Bug
        4481, // Sinister Serpent
        5526, // Spirit Reaper
        5655, // Breaker the Magical Warrior
        5323, // Exiled Force
        5719, // D.D. Warrior Lady
        4758, // Jinzo
        4007, // Blue-Eyes White Dragon
        4028, // Summoned Skull
        5318, // Marauding Captain
        7716, // Krebons
        7687, // Junk Synchron
        4108, // Mask of Darkness
        4434, // Magician of Faith
        5371, // Fiber Jar
        4922, // Nimble Momonga
        5651, // Old Vindictive Magician
        5238, // Ancient Lamp
        9618, // Gagaga Magician
        9621, // Goblindbergh
        9563, // Zubaba Knight
        4969, // Morphing Jar #2
    ];
    pub const SPELLS: [u16; 32] = [
        4844, // Pot of Greed
        4812, // Graceful Charity
        4821, // Card Destruction
        5849, // Reload
        4842, // Monster Reborn
        4966, // Premature Burial
        4860, // Change of Heart
        4898, // Snatch Steal
        5162, // Creature Swap
        4818, // Scapegoat
        4342, // Dark Hole
        4343, // Raigeki
        4891, // Heavy Storm
        4909, // Mystical Space Typhoon
        4678, // Harpie's Feather Duster
        4354, // Swords of Revealing Light
        5432, // Book of Moon
        4895, // Upstart Goblin
        7389, // Hand Destruction
        5328, // Reinforcement of the Army
        5236, // Foolish Burial
        4910, // Giant Trunade
        4803, // Brain Control
        5505, // Enemy Controller
        5123, // Card of Safe Return
        4963, // Nobleman of Crossout
        5132, // Dimensionhole
        6511, // Pot of Avarice
        6161, // Gold Sarcophagus
        6541, // Magical Mallet
        5392, // Convulsion of Nature
        7256, // Swing of Memories
    ];
    pub const TRAPS: [u16; 16] = [
        4962, // Magical Hats
        4836, // Trap Hole
        5400, // Bottomless Trap Hole
        4887, // Mirror Force
        5114, // Torrential Tribute
        4989, // Call of the Haunted
        5005, // Ring of Destruction
        4988, // Dust Tornado
        5124, // Magic Cylinder
        5799, // Sakuretsu Armor
        5914, // Compulsory Evacuation Device
        7694, // Scrap-Iron Scarecrow
        4866, // Royal Decree
        4355, // Spellbinding Circle
        5360, // Exchange of the Spirit
        6078, // Return from the Different Dimension
    ];
    /// Synchro Monsters for the Tuners in the pool (Goyo Guardian, Stardust Dragon, Junk Warrior) and Xyz Monsters for its
    /// Level 4s (Gem-Knight Pearl, Number 39: Utopia).
    pub const EXTRA: [u16; 5] = [7736, 7734, 7696, 9601, 9575];

    /// A seeded deck: 40 cards drawn from the pool (about half monsters), at most three of a card, and the extra deck.
    pub fn deck(seed: u64) -> (Vec<u16>, Vec<u16>) {
        let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut counts: HashMap<u16, u32> = HashMap::new();
        let mut main = vec![];
        let (monsters, spells) = (18 + next(&mut rng) as usize % 5, 10 + next(&mut rng) as usize % 5);
        for (pool, n) in [(&MONSTERS[..], monsters), (&SPELLS[..], spells), (&TRAPS[..], 40 - monsters - spells)] {
            let mut added = 0;
            while added < n {
                let card = pool[next(&mut rng) as usize % pool.len()];
                let c = counts.entry(card).or_default();
                if *c < 3 {
                    *c += 1;
                    main.push(card);
                    added += 1;
                }
            }
        }
        (main, EXTRA.iter().flat_map(|&c| [c, c]).collect())
    }

    /// The harness's card data: every card is its own row (row = Konami id), and the Sheep Tokens.
    fn card_data(decks: &[(&[u16], &[u16]); 2]) -> HashMap<u16, CardStats> {
        let mut cards: HashMap<u16, CardStats> = decks
            .iter()
            .flat_map(|(m, e)| m.iter().chain(e.iter()))
            .map(|&cid| (cid, CardStats { cid, atk: 1000, def: 1000, level: 4, attribute: 1, race: 1 }))
            .collect();
        for row in SHEEP_ROWS {
            cards.insert(row, CardStats { cid: 0, atk: 0, def: 0, level: 1, attribute: 1, race: 1 });
        }
        cards
    }

    /// What a self-played duel did: engine steps, turns, how each engine message type came up, how many moves were Xyz
    /// materials' (to or from a place under a monster), and whether it ended.
    #[derive(Debug, Default)]
    pub struct Played {
        pub steps: usize,
        pub turns: u32,
        pub ended: bool,
        pub kinds: BTreeMap<u64, u32>,
        pub materials: u32,
    }

    /// Plays a duel between two seeded random players (`decks` by engine team: main and extra deck, Konami ids), the
    /// client seated as team `client_team` and, with `hidden`, given the opponent's hidden cards without rows as against a
    /// player. After every engine step the client model is checked against the engine: the first difference fails the
    /// duel with the seed, the step and the engine messages that came before it.
    pub fn play(dir: &Path, seed: u64, decks: [(&[u16], &[u16]); 2], client_team: usize, hidden: bool) -> Result<Played, String> {
        let mut rng = seed | 1;
        let cards = card_data(&decks);
        // The client's engine player of team 0 (team 0 goes first), and the decks by client player.
        let first = if client_team == 0 { CLIENT } else { CPU };
        let (mine, theirs) = (decks[client_team], decks[1 - client_team]);
        let build = if hidden { Translator::against_player } else { Translator::new };
        let (mut t, mut table) = build(first, [mine.0, theirs.0], cards.clone());
        t.tokens(HashMap::from([("Sheep Token".to_string(), SHEEP_ROWS.to_vec())]));
        table.extend(t.extra_decks([mine.1, theirs.1]));
        let mut model = ClientModel::new(first, cards.clone());
        // The rows the client isn't given: the same uids in the same order from a translator that hides nothing.
        let (mut full, mut known) = Translator::new(first, [mine.0, theirs.0], cards.clone());
        known.extend(full.extra_decks([mine.1, theirs.1]));
        let int = |m: &yo3_proto::Message, k: &str| m.get(k).and_then(yo3_proto::Value::as_u32).unwrap_or(0);
        for m in &known {
            model.know(int(m, "param1") & 0xFF_FFFF, int(m, "param2") as u16);
        }
        let deck = |(main, extra): (&[u16], &[u16])| Deck { id: 0, name: String::new(), main: main.to_vec(), extra: extra.to_vec(), side: vec![] };
        let mut opening = vec![setup(first, &deck(mine), &deck(theirs))];
        opening.extend(table);
        // Every fourth duel starts low, so that duels reach their end too.
        let lp = if seed.is_multiple_of(4) { 2000 } else { 8000 };
        opening.extend([command(DUEL_START, [0; 4]), command(SET_LP, [CLIENT, lp, 0, 0]), command(SET_LP, [CPU, lp, 0, 0])]);
        opening.extend(piles());
        model.apply_all(&opening);

        let mut engine = Engine::start(dir).map_err(|e| format!("engine: {e}"))?;
        let mut shuffled = [decks[0].0.to_vec(), decks[1].0.to_vec()];
        for d in shuffled.iter_mut() {
            shuffle(&mut rng, d);
        }
        let mut answer = engine.new_duel(seed, lp, [(&shuffled[0], decks[0].1), (&shuffled[1], decks[1].1)]);
        let mut played = Played::default();
        let mut prompt: Option<Json> = None;
        let mut retries = 0;
        let mut actions = 0;
        while played.steps < MAX_STEPS {
            played.steps += 1;
            let a = answer.map_err(|e| format!("seed {seed}: engine: {e}"))?;
            model.observe_engine(&a.messages);
            for m in &a.messages {
                if let Some(kind) = m["type"].as_u64() {
                    *played.kinds.entry(kind).or_default() += 1;
                    if kind == 50 && (m["to"]["overlay_sequence"].is_u64() || m["from"]["overlay_sequence"].is_u64()) {
                        played.materials += 1;
                    }
                    if kind == MSG_NEW_TURN {
                        played.turns += 1;
                        actions = 0;
                    }
                }
            }
            let mut out = t.events(&a.messages);
            settle_positions(&mut out);
            model.apply_all(&out);
            let state = engine.state().map_err(|e| format!("seed {seed}: engine state: {e}"))?;
            let fail = |problems: Vec<String>| {
                let recent: Vec<String> = a.messages.iter().map(short).collect();
                format!(
                    "seed {seed} (client team {client_team}{}), step {}, turn {}:\n  {}\n  engine messages of this step: {}",
                    if hidden { ", hidden" } else { "" },
                    played.steps,
                    played.turns,
                    problems.join("\n  "),
                    recent.join(" | ")
                )
            };
            let problems = model.check(&state);
            if !problems.is_empty() {
                return Err(fail(problems));
            }
            if a.status != "await" || played.turns > MAX_TURNS {
                played.ended = a.status == "end";
                return Ok(played);
            }
            let last = a.messages.last().cloned().unwrap_or(Json::Null);
            let response = if last["type"].as_u64() == Some(MSG_RETRY) {
                // The core refused the last answer: the safe one this time, a few times at most.
                retries += 1;
                if retries > 4 {
                    return Err(fail(vec![format!("the core keeps refusing answers to {}", prompt.as_ref().map(short).unwrap_or_default())]));
                }
                bot_answer(prompt.as_ref().unwrap_or(&last))
            } else {
                retries = 0;
                // The translator's windows for the client (its prompts), then the random player's answer for whoever is
                // asked (the translator's own answer for the CPU is the placeholder AI's).
                if let Prompt::Client(mut msgs) = t.prompt(&last) {
                    settle_positions(&mut msgs);
                    model.apply_all(&msgs);
                    let problems = model.check(&state);
                    if !problems.is_empty() {
                        return Err(fail(problems));
                    }
                }
                prompt = Some(last.clone());
                actions += 1;
                random_answer(&last, &mut rng, actions > ACTIONS_PER_TURN)
            };
            answer = engine.respond(response);
        }
        Ok(played)
    }

    /// One engine message in a few words, for failure reports.
    fn short(m: &Json) -> String {
        let mut s = m.to_string();
        if s.len() > 160 {
            s.truncate(160);
            s.push('…');
        }
        s
    }

    /// A random legal answer: any summon, Set, activation, attack or position change offered (moving on once `tired`),
    /// random picks within the bounds asked, random yes/no and options. Prompts it doesn't know get the placeholder AI's.
    pub fn random_answer(m: &Json, rng: &mut u64, tired: bool) -> Json {
        let len = |k: &str| m[k].as_array().map_or(0, Vec::len);
        let mut pick = |n: usize| (next(rng) as usize) % n.max(1);
        match m["type"].as_u64().unwrap_or(0) {
            MSG_SELECT_IDLECMD => {
                let mut options: Vec<(u64, Json)> = vec![];
                if !tired {
                    for (list, action) in [("summons", 0), ("special_summons", 1), ("pos_changes", 2), ("monster_sets", 3), ("spell_sets", 4), ("activates", 5)] {
                        options.extend((0..len(list)).map(|i| (action, json!(i))));
                    }
                }
                let leave: Vec<(u64, Json)> =
                    [(m["to_bp"] == true, 6), (m["to_ep"] == true, 7)].into_iter().filter(|(ok, _)| *ok).map(|(_, a)| (a, Json::Null)).collect();
                // Leaving is one option among the others (always, once tired).
                if !leave.is_empty() && (options.is_empty() || pick(options.len() + 2) < 2) {
                    let (action, index) = leave[pick(leave.len())].clone();
                    return json!({ "type": 1, "action": action, "index": index });
                }
                if options.is_empty() {
                    return bot_answer(m);
                }
                let (action, index) = options[pick(options.len())].clone();
                json!({ "type": 1, "action": action, "index": index })
            }
            MSG_SELECT_BATTLECMD => {
                let mut options: Vec<(u64, Json)> = vec![];
                if !tired {
                    options.extend((0..len("attacks")).map(|i| (1, json!(i))));
                    options.extend((0..len("chains")).map(|i| (0, json!(i))));
                }
                if options.is_empty() || pick(options.len() + 1) == 0 {
                    let action = if m["to_m2"] == true && pick(2) == 0 { 2 } else { 3 };
                    return json!({ "type": 0, "action": action, "index": null });
                }
                let (action, index) = options[pick(options.len())].clone();
                json!({ "type": 0, "action": action, "index": index })
            }
            MSG_SELECT_CHAIN => {
                let n = len("selects");
                let index = if n > 0 && (m["forced"] == true || pick(2) == 0) { json!(pick(n)) } else { Json::Null };
                json!({ "type": 8, "index": index })
            }
            MSG_SELECT_EFFECTYN => json!({ "type": 2, "yes": pick(2) == 0 }),
            MSG_SELECT_YESNO => json!({ "type": 3, "yes": pick(2) == 0 }),
            MSG_SELECT_OPTION => json!({ "type": 4, "index": pick(len("options")) }),
            kind @ (MSG_SELECT_CARD | MSG_SELECT_TRIBUTE) => {
                let n = len("selects");
                let min = (m["min"].as_u64().unwrap_or(1) as usize).min(n);
                let max = (m["max"].as_u64().unwrap_or(1) as usize).clamp(min, n.max(min));
                let count = min + pick(max - min + 1);
                let mut all: Vec<usize> = (0..n).collect();
                for i in (1..all.len()).rev() {
                    all.swap(i, pick(i + 1));
                }
                all.truncate(count);
                all.sort_unstable();
                json!({ "type": if kind == MSG_SELECT_CARD { 5 } else { 12 }, "indicies": all })
            }
            MSG_SELECT_UNSELECT_CARD => {
                let n = len("select_cards");
                let index = if n == 0 || (m["can_finish"] == true && pick(2) == 0) { Json::Null } else { json!(pick(n)) };
                json!({ "type": 7, "index": index })
            }
            kind @ (MSG_SELECT_PLACE | 24) => {
                // A random free own zone (field_mask bits set = unavailable: monster zones 0-4, Spell/Trap 8-12).
                let mask = m["field_mask"].as_u64().unwrap_or(0);
                let free: Vec<(u64, u64)> =
                    (0..5).map(|s| (4, s, s)).chain((0..5).map(|s| (8, s, s + 8))).filter(|(_, _, b)| mask & 1 << b == 0).map(|(l, s, _)| (l, s)).collect();
                if free.is_empty() {
                    return bot_answer(m);
                }
                let (location, sequence) = free[pick(free.len())];
                json!({ "type": if kind == MSG_SELECT_PLACE { 10 } else { 9 }, "places": [{ "player": m["player"], "location": location, "sequence": sequence }] })
            }
            MSG_SELECT_POSITION => {
                let allowed = m["positions"].as_u64().unwrap_or(1);
                let bits: Vec<u64> = [1, 2, 4, 8].into_iter().filter(|b| allowed & b != 0).collect();
                json!({ "type": 11, "position": if bits.is_empty() { 1 } else { bits[pick(bits.len())] } })
            }
            _ => bot_answer(m),
        }
    }
}
