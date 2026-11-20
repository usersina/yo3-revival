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
//! - The summon commands (Set, Run, the material list 1015/1016, RunFusion 0x3A) follow the order rules of
//!   [`summon`], learnt from client crashes and checked against Konami's own duel logs; a Set or Run names a card at
//!   the place it gives (the hand 11, or a field zone).
//! - The position dialog 0x36 {kind 8, _, uid | flag << 16} (DuelSelectStandDialog 0x4C5E50) draws its two buttons from
//!   the uid's row in the card table: a uid the table has no row for leaves them blank, nothing to click, and the duel
//!   stops there (a token asked its position before it appeared, Gorz the Emissary of Darkness's).
//!
//! Where the model itself sees the client go wrong (a hand card taken out by an index that holds another card, a
//! shuffle with an empty index) it notes a problem at once; [`ClientModel::check`] then compares every hand, field
//! slot and pile with the engine's `state` (see crates/yo3-engine's session.rs). Cards whose row the client was never given
//! (an opponent's hidden cards) are compared by count and place only, unless [`ClientModel::know`] seeded their row.
use std::collections::{HashMap, HashSet, VecDeque};

use serde_json::Value as Json;
use yo3_proto::{Message, Value};

use crate::translate::CardStats;
use crate::ocg::message::Event;
use crate::wire::{Cmd, Place, Target, cmd, dialog, request, table_place, zone};
pub mod native;
pub mod coverage;
pub mod summon;
pub mod scenario;

const SERVER_DUEL: u32 = crate::duel::SERVER_DUEL;
/// The uids a mixed card list puts in the table for the time it is open (`translate.rs` STAGED_UID), and the first
/// token uid.
const STAGED: u32 = 0xF0_0000;
const FIRST_TOKEN: u32 = 0x4000;
/// How many applied commands a problem report shows.
const RECENT: usize = 12;

/// A pile's index (0 extra, 1 Graveyard, 2 banished, 3 deck... as `piles` keeps them) for a list zone.
fn pile(place: Place) -> Option<usize> {
    (zone::EXTRA..=zone::REMOVED).contains(&place.zone).then(|| (place.zone - zone::EXTRA) as usize)
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
    /// The player who surrendered (command 2 with reason 12): the client set their LP to 0, which the engine never
    /// knew, so the LP check skips them.
    surrendered: Option<usize>,
    record: [u32; 9],
    records: HashMap<(usize, u32), [u32; 9]>,
    /// What the selected card profile says of its cards (by CARD_Prop row), and the cards already judged against it.
    expect: HashMap<u16, crate::profile::Expect>,
    profile_seen: std::sync::Mutex<HashSet<u32>>,
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
    /// The summon commands' order (`summon.rs`).
    summon: summon::SummonOrder,
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
            surrendered: None,
            record: [0; 9],
            records: HashMap::new(),
            expect: HashMap::new(),
            profile_seen: Default::default(),
            counters: HashMap::new(),
            turn: (first as usize, 1), phase: None,
            engine_turn: (first as usize, 0), engine_phase: None,
            coverage_gaps: HashSet::new(),
            relations: vec![],
            counts: Default::default(),
            piles: Default::default(),
            problems: vec![],
            recent: VecDeque::new(),
            summon: summon::SummonOrder::default(),
        }
    }

    /// What the selected card profile says of its cards, by CARD_Prop row (`CardView::expectations`): the first time a
    /// face-up monster of one is seen on the field, its engine level, base ATK/DEF, race and attribute must be the
    /// profile's, so a profile that says what the engine does not play fails the check (issue #60).
    pub fn expect_profile(&mut self, expect: HashMap<u16, crate::profile::Expect>) {
        self.expect = expect;
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
            match Event::lenient(m) {
                Ok(Some(Event::NewTurn(t))) => {
                    let player = if t.player == 0 { self.first } else { 1 - self.first };
                    self.engine_turn = (player, self.engine_turn.1 + 1);
                }
                Ok(Some(Event::NewPhase(n))) => {
                    let phase = match n.phase {
                        1 => Some(0), 2 => Some(1), 4 => Some(2),
                        8 | 16 | 32 | 64 | 128 => Some(3), 256 => Some(4), 512 => Some(5), _ => None,
                    };
                    if let Some(phase) = phase { self.engine_phase = Some((self.engine_turn.0, phase)); }
                }
                _ => {}
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
            Some(request::DUEL_SETUP) => {
                // A new duel (a match's next one too): nothing of the last one stays.
                let rows = std::mem::take(&mut self.rows);
                let gaps = std::mem::take(&mut self.coverage_gaps);
                let oracle = (self.engine_turn, self.engine_phase);
                let expect = std::mem::take(&mut self.expect);
                *self = ClientModel::new(self.first as u32, std::mem::take(&mut self.cards));
                self.rows = rows;
                self.expect = expect;
                self.coverage_gaps = gaps;
                (self.engine_turn, self.engine_phase) = oracle;
                self.counts = [
                    [int("ExtraDeckNum001") as i64, int("MainDeckNum001") as i64, 0, 0],
                    [int("ExtraDeckNum002") as i64, int("MainDeckNum002") as i64, 0, 0],
                ];
                self.note("setup".into());
            }
            Some(request::ENGINE_COMMAND) => {
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

    /// The commands [`Cmd`] packs, as the client reads them.
    fn typed(&mut self, c: Cmd) {
        match c {
            Cmd::BattleInit | Cmd::BattleEnd => self.battle = [None; 2],
            Cmd::BattleSelect | Cmd::Dialog { .. } => {}
            Cmd::BattleAttack { player, zone, target } => {
                self.battle = [None; 2];
                if zone < 5 { self.battle[player as usize] = Some((player as usize, zone as usize)); }
                if let Target::Monster { player, zone } = target { self.battle[player as usize] = Some((player as usize, zone as usize)); }
            }
            Cmd::BattleRun { flags, .. } => {
                self.note(format!("BattleRun flags {flags:#x}"));
                for player in 0..2 {
                    if flags & (2 << (player * 8)) != 0
                        && let Some((side, zone)) = self.battle[player]
                        && let Some(uid) = self.slots[side][zone]
                    { self.off_field.insert(uid); }
                }
            }
            Cmd::Card { uid, row, place, .. } => {
                if row != 0 && uid < STAGED {
                    self.rows.insert(uid, row as u16);
                }
                self.table.insert(uid, place);
                self.note(format!("1000 {{uid {uid}, row {row}, place {place:#x}}}"));
            }
            Cmd::Move { uid, from, to, .. } => {
                self.note(format!("0x1A {{uid {uid}, {:#x} -> {:#x}}}", from.packed(), to.packed()));
                self.card_move(uid, from, to);
            }
        }
    }

    fn command(&mut self, id: u32, [p1, p2, p3, p4]: [u32; 4]) {
        if let Some(problem) = self.summon.command(id, [p1, p2, p3, p4]) {
            self.problem(problem);
        }
        // A summon's Set or Run names a card at its place: in the hand (11) or a field zone. DuelSummon finds the card
        // object by uid (0x500470) and plays the effect at the place.
        if matches!(id, cmd::TRIBUTE_SET | cmd::TRIBUTE_RUN | cmd::MATERIAL_SET | cmd::MATERIAL_RUN | cmd::TUNING_SET | cmd::TUNING_RUN) {
            let (player, zone, uid) = ((p1 & 1) as usize, p2, p3 & 0xFFFF);
            let there = match zone {
                zone::HAND => self.hands[player].contains(&uid),
                z if z <= zone::FIELD_SPELL => self.slots[player][z as usize] == Some(uid),
                _ => true,
            };
            self.note(format!("{id:#x} {{{p1}, {p2}, uid {uid}, {p4:#x}}}"));
            if !there {
                self.problem(format!("summon command {id:#x} names {} at player {player} zone {zone}, where the client doesn't hold it", self.describe(uid)));
            }
        }
        if let Some(Cmd::Dialog { kind: dialog::POSITION, param, .. }) = Cmd::decode(id, [p1, p2, p3, p4]) {
            let uid = param & 0xFFFF;
            self.note(format!("0x36 position {{uid {uid}}}"));
            if self.rows.get(&uid).is_none_or(|&row| row == 0) {
                self.problem(format!("the position dialog names {}, which has no card in the client's card table: its buttons show no card to click", self.describe(uid)));
            }
        }
        if let Some(typed) = Cmd::decode(id, [p1, p2, p3, p4]) {
            self.typed(typed);
            return;
        }
        match id {
            cmd::PHASE => self.phase = Some(((p1 & 1) as usize, p2)),
            cmd::TURN => self.turn = ((p1 & 1) as usize, self.turn.1 + 1),
            cmd::SET_LP => self.lp[(p1 & 1) as usize] = Some(p2 as i64),
            cmd::DUEL_END if p2 == crate::wire::SURRENDERED => self.surrendered = Some(if p1 == crate::wire::result::LOST { 0 } else { 1 }),
            // A change applies with param3's low byte 0; 1 only shows it (DuelCockpit 0x502EB0).
            cmd::CHANGE_LP if p3 & 0xFF == 0 => {
                if let Some(lp) = &mut self.lp[(p1 & 1) as usize] { *lp = (*lp + p2 as i32 as i64).clamp(0, 99999); }
            },
            cmd::CHANGE_LP => {},
            cmd::RECORD_BEGIN => { self.record[0] = p2; self.record[2] = p3; self.record[3] = p4; },
            cmd::RECORD_MORE => { self.record[4] = p2; self.record[5] = p3; self.record[6] = p4; },
            cmd::RECORD_COMMIT => {
                self.record[7] = p2; self.record[8] = p3; self.record[1] = p4;
                self.records.insert(((p1 & 1) as usize, p1 >> 8 & 0xFF), self.record);
            },
            cmd::COUNTER => {
                let (p, z) = ((p1 & 1) as usize, (p1 >> 8 & 0xFF) as usize);
                if let Some(uid) = self.slots[p].get(z).copied().flatten() {
                    let count = self.counters.entry((uid, p3)).or_default();
                    *count = (*count + p2 as i32 as i64).max(0);
                } else { self.problem(format!("counter change at empty player {p} zone {z}")); }
            },
            cmd::HAND_SHUFFLE => {
                self.note(format!("0x12 {{{p1}}}"));
                self.hand_shuffle((p1 & 1) as usize);
            }
            cmd::DECK_RESET => {
                let (player, zone) = ((p1 & 1) as usize, p2);
                if (zone::EXTRA..=zone::REMOVED).contains(&zone) {
                    let pile = (zone - zone::EXTRA) as usize;
                    self.piles[player][pile] = self.counts[player][pile];
                }
            }
            cmd::CARD_SWAP => {
                let (a, b) = (Place::of_move(p2), Place::of_move(p3));
                self.note(format!("0x1B {{{p2:#x}, {p3:#x}}}"));
                if a.zone > zone::FIELD_SPELL || b.zone > zone::FIELD_SPELL {
                    self.problem(format!("CardSwap of {p2:#x} and {p3:#x}: the client swaps only field places"));
                    return;
                }
                let (in_a, in_b) = (self.slots[a.side()][a.zone as usize], self.slots[b.side()][b.zone as usize]);
                self.slots[a.side()][a.zone as usize] = in_b;
                self.slots[b.side()][b.zone as usize] = in_a;
                for (uid, place) in [(in_b, a), (in_a, b)] {
                    if let Some(uid) = uid {
                        if self.places.get(&uid).is_some_and(|old| old.face_up != place.face_up) {
                            self.counters.retain(|(card, _), _| *card != uid);
                        }
                        self.places.insert(uid, place);
                    }
                }
            }
            cmd::FLIP_TURN => {
                let place = Place::of_move(p1);
                if place.zone <= zone::FIELD_SPELL
                    && let Some(uid) = self.slots[place.side()][place.zone as usize]
                {
                    if self.places.get(&uid).is_some_and(|old| old.face_up != place.face_up) {
                        self.counters.retain(|(card, _), _| *card != uid);
                    }
                    self.places.insert(uid, place);
                }
                self.note(format!("0x1C {{{p1:#x}}}"));
            }
            cmd::CARD_VANISH => {
                let (player, zone, index) = ((p1 & 1) as usize, p2 & 0xFF, (p2 >> 8 & 0xFF) as usize);
                let uid = match zone {
                    z if z <= zone::FIELD_SPELL => self.slots[player][z as usize],
                    zone::HAND => self.hands[player].get(index).copied(),
                    _ => None,
                };
                if let Some(uid) = uid {
                    self.off_field.insert(uid);
                }
                self.note(format!("0x1F {{{p1}, {p2:#x}}}"));
            }
            cmd::RELATION => {
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
        if from.zone <= zone::FIELD_SPELL && from.face_up != to.face_up {
            self.counters.retain(|(card, _), _| *card != uid);
        }
        if from.zone <= zone::FIELD_SPELL {
            self.slots[from.side()][from.zone as usize] = None;
        }
        if to.zone <= zone::FIELD_SPELL {
            self.slots[to.side()][to.zone as usize] = Some(uid);
        }
        if from.zone == zone::HAND {
            let hand = &mut self.hands[from.side()];
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
        if to.zone == zone::HAND {
            if self.hands.iter().any(|h| h.contains(&uid)) {
                self.problem(format!("uid {uid} reaches hand {} while a hand list still holds it", to.player));
            }
            self.hands[to.side()].push(uid);
        }
        if let Some(pile) = pile(from) {
            self.counts[from.side()][pile] -= 1;
            self.piles[from.side()][pile] -= 1;
        }
        if let Some(pile) = pile(to) {
            self.counts[to.side()][pile] += 1;
            self.piles[to.side()][pile] += 1;
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
            match self.at_table(table_place(player as u32, zone::HAND, i))[..] {
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
            zone <= zone::FIELD_SPELL as usize && self.slots[player][zone].is_some_and(|uid| !self.off_field.contains(&uid))
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
        if let Some(expected) = side["lp"].as_i64() && self.lp[p] != Some(expected) && self.surrendered != Some(p) {
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
            let entries = self.at_table(table_place(p as u32, zone::HAND, i));
            if entries != [uid] {
                out.push(format!("player {p}'s hand index {i}: the client holds {}, its table has {entries:?} there", self.describe(uid)));
            }
        }
        // The field: each slot as the engine has it, with its face (and a monster's position).
        let mzone = list("mzone");
        let szone = list("szone");
        let mut slots: Vec<(u32, Json)> = (0..5).map(|s| (s as u32, mzone.get(s).cloned().unwrap_or(Json::Null))).collect();
        slots.extend((0..5).map(|s| (5 + s as u32, szone.get(s).cloned().unwrap_or(Json::Null))));
        slots.push((zone::FIELD_SPELL, szone.get(5).cloned().unwrap_or(Json::Null)));
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
                    // A token: one with no Konami id, or the client's token card of the engine's id (Doomsday Token).
                    let token_row_cid = self.rows.get(&uid).and_then(|r| self.cards.get(r)).map(|c| c.cid);
                    let same = if uid >= FIRST_TOKEN { token(&engine) || token_row_cid == Some(cid_of(&engine)) } else { self.cid(uid).is_none_or(|c| c == cid_of(&engine)) };
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
                    if zone < 5 && face_up
                        && let Some(want) = self.rows.get(&uid).and_then(|row| self.expect.get(row))
                        && self.profile_seen.lock().unwrap_or_else(|e| e.into_inner()).insert(uid)
                    {
                        for problem in want.problems(&engine) {
                            out.push(format!("player {p}'s zone {zone}: {} differs from the card profile: {problem}", self.describe(uid)));
                        }
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
        for (key, zone) in [("extra", zone::EXTRA), ("grave", zone::GRAVE), ("removed", zone::REMOVED)] {
            let n = list(key).len() as i64;
            let shown = self.piles[p][(zone - zone::EXTRA) as usize];
            if n != shown {
                out.push(format!("player {p}'s {key} pile: the engine has {n} cards, the client counts {shown}"));
            }
        }
        let deck = side["deck"].as_i64().unwrap_or(0);
        if deck != self.piles[p][(zone::DECK - zone::EXTRA) as usize] {
            out.push(format!("player {p}'s deck: the engine has {deck} cards, the client counts {}", self.piles[p][(zone::DECK - zone::EXTRA) as usize]));
        }
        for (i, engine) in list("grave").iter().enumerate() {
            match self.at_table(table_place(p as u32, zone::GRAVE, i))[..] {
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
    use crate::duel::{CLIENT, CPU, command, next, piles, setup, shuffle};
    use crate::ocg::message::{Ask, Event};
    use crate::ocg::{Response, battle, hint, idle, loc, msg, pos};
    use crate::wire::{cmd, dialog, zone};
    use crate::engine::Engine;
    use crate::front::Deck;
    use crate::translate::{CardStats, ChainWindow, Prompt, Translator, bot_response, chain_window, settle_positions};

    const SHOWN_BY_DEFAULT: u32 = 2;
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
    /// A Fusion package for every seventh deck (`deck`): Polymerization, Flame Manipulator and Masaki the Legendary
    /// Swordsman for Flame Swordsman, and King of the Swamp in either's place, so that Fusion Summons from the hand
    /// (issue #40) and with a substitute are played.
    pub const FUSION: [u16; 7] = [4837, 4837, 4148, 4148, 4299, 4299, 6044];
    pub const FUSION_EXTRA: [u16; 2] = [4021, 4021];
    /// A token package for every seventh deck (`deck`): cards whose tokens may come in Attack or Defense Position, so
    /// the player is asked the position of a token that doesn't exist yet (Gorz the Emissary of Darkness's froze the
    /// duel: a dialog with no card to click). Fiend's Sanctuary and Physical Double (tokens without a Konami id, found
    /// by name), Phantom Skyblaster (Skyblaster Tokens, 3935) and Gorz (Emissary of Darkness Token, 3922).
    pub const TOKENS: [u16; 7] = [5632, 5632, 5632, 7384, 7384, 5621, 6558];

    /// Every card the duel check's decks draw from (Konami ids), for the effect coverage report.
    pub fn pool() -> Vec<u16> {
        let mut all: Vec<u16> = [&MONSTERS[..], &SPELLS, &TRAPS, &EXTRA, &FUSION, &FUSION_EXTRA, &TOKENS].concat();
        all.sort_unstable();
        all.dedup();
        all
    }
    /// The client's token cards for the token package: (row, Konami id, name).
    const TOKEN_CARDS: [(u16, u16, &str); 4] =
        [(60010, 3914, "Metal Fiend Token"), (60011, 3913, "Mirage Token"), (60012, 3935, "Skyblaster Token"), (60013, 3922, "Emissary of Darkness Token")];

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
        let mut extra: Vec<u16> = EXTRA.iter().flat_map(|&c| [c, c]).collect();
        // Deck seeds 5 apart from a multiple of 7: none of the seeds the duel check keeps for its known cases.
        if seed % 7 == 5 {
            main.truncate(40 - FUSION.len());
            main.extend(FUSION);
            extra.extend(FUSION_EXTRA);
        }
        // And multiples of 7, which none of them are either.
        if seed.is_multiple_of(7) {
            main.truncate(40 - TOKENS.len());
            main.extend(TOKENS);
        }
        (main, extra)
    }

    /// The harness's card data: every card is its own row (row = Konami id), the Sheep Tokens and the token package's.
    fn card_data(decks: &[(&[u16], &[u16]); 2]) -> HashMap<u16, CardStats> {
        let mut cards: HashMap<u16, CardStats> = decks
            .iter()
            .flat_map(|(m, e)| m.iter().chain(e.iter()))
            .map(|&cid| (cid, CardStats { cid, atk: 1000, def: 1000, level: 4, attribute: 1, race: 1 }))
            .collect();
        for row in SHEEP_ROWS {
            cards.insert(row, CardStats { cid: 0, atk: 0, def: 0, level: 1, attribute: 1, race: 1 });
        }
        for (row, cid, _) in TOKEN_CARDS {
            cards.insert(row, CardStats { cid, atk: 0, def: 0, level: 1, attribute: 1, race: 1 });
        }
        cards
    }

    /// The harness's token cards by name, as the server's (`token_rows`).
    fn token_rows() -> HashMap<String, Vec<u16>> {
        let mut rows = HashMap::from([("Sheep Token".to_string(), SHEEP_ROWS.to_vec())]);
        rows.extend(TOKEN_CARDS.iter().map(|&(row, _, name)| (name.to_string(), vec![row])));
        rows
    }

    /// What a self-played duel did: engine steps, turns, how each engine message type came up, how many moves were Xyz
    /// materials' (to or from a place under a monster), whether it ended, and how often each card (Konami id)
    /// activated an effect (MSG_CHAINING) or was summoned.
    #[derive(Debug, Default)]
    pub struct Played {
        pub steps: usize,
        pub turns: u32,
        pub ended: bool,
        pub kinds: BTreeMap<u64, u32>,
        pub materials: u32,
        pub activated: BTreeMap<u16, u32>,
        pub summoned: BTreeMap<u16, u32>,
        /// The client's prompts the server answered itself although the player had a choice (see [`choices`]), by
        /// engine message type: windows the client lacks (a zone, a sum, a sort...).
        pub decided: BTreeMap<u64, u32>,
        /// The client's summon animations (RunFusion 0x3A) by kind, and the Fusion materials played from the hand.
        pub animations: BTreeMap<u32, u32>,
        pub hand_materials: u32,
        /// The position dialogs the client was shown for a token about to appear.
        pub token_positions: u32,
        /// Chain chances the engine marked as mattering (spe_count above 0) that the default Activation Message answers
        /// No to by itself, by window (its HINT_EVENT value, 0 for none): the player never sees them unless they chose
        /// Manual. Only windows whose question holds no optional trigger may be here.
        pub hidden_by_default: BTreeMap<u64, u32>,
        /// A phase's own windows with something to activate that the player saw on the default option (#53).
        pub phase_windows: u32,
        /// Attack targets the client picked on the field (BattleSelect, #51).
        pub field_targets: u32,
        /// Effect coverage (#67), by the Konami id of the card whose chain link it was: links that resolved, those
        /// that resolved with a visible result (see [`Effects`]), and those negated or disabled.
        pub resolved: BTreeMap<u16, u32>,
        pub visible: BTreeMap<u16, u32>,
        pub negated: BTreeMap<u16, u32>,
    }

    /// Follows the chain to the card whose effect resolves, for the effect coverage. A result is visible when, while a
    /// link resolves, a card moves, is summoned, drawn, equipped, changes position or gets counters, LP change, or a
    /// monster's ATK, DEF or Level on the field changes (the engine's field compared at each step: a change in the step that
    /// solved a link counts for it, a rough attribution).
    #[derive(Default)]
    struct Effects {
        chain: Vec<u16>,
        solving: Option<u16>,
        visible: bool,
        solved: Vec<(u16, bool)>,
        stats: Vec<(i64, i64, i64)>,
    }

    impl Effects {
        fn message(&mut self, m: &Json, played: &mut Played) {
            let kind = m["type"].as_u64().unwrap_or(0);
            let link = || m["chain_size"].as_u64().unwrap_or(0) as usize;
            match kind {
                msg::CHAINING => {
                    self.chain.truncate(link().saturating_sub(1));
                    self.chain.push(m["cid"].as_u64().unwrap_or(0) as u16);
                }
                msg::CHAIN_SOLVING => {
                    self.solving = self.chain.get(link().wrapping_sub(1)).copied();
                    self.visible = false;
                }
                msg::CHAIN_SOLVED => {
                    if let Some(cid) = self.solving.take() {
                        self.solved.push((cid, self.visible));
                    }
                }
                msg::CHAIN_NEGATED | msg::CHAIN_DISABLED => {
                    if let Some(&cid) = self.chain.get(link().wrapping_sub(1)) {
                        *played.negated.entry(cid).or_default() += 1;
                    }
                }
                msg::CHAIN_END => {
                    self.chain.clear();
                    self.solving = None;
                }
                msg::MOVE | msg::DRAW | msg::DAMAGE | msg::RECOVER | msg::PAY_LPCOST | msg::POS_CHANGE | msg::SPSUMMONING | msg::EQUIP
                | msg::ADD_COUNTER | msg::REMOVE_COUNTER | msg::SET | msg::SWAP | msg::SWAP_GRAVE_DECK
                    if self.solving.is_some() =>
                {
                    self.visible = true
                }
                _ => {}
            }
        }

        /// The step's field: a stat change counts for the links solved in this step (or the one still resolving).
        fn field(&mut self, state: &Json, played: &mut Played) {
            let stats: Vec<(i64, i64, i64)> = (0..2)
                .flat_map(|t| state[t]["mzone"].as_array().cloned().unwrap_or_default())
                .map(|c| (c["attack"].as_i64().unwrap_or(-1), c["defense"].as_i64().unwrap_or(-1), c["level"].as_i64().unwrap_or(-1)))
                .collect();
            let changed = stats != self.stats;
            self.stats = stats;
            if changed && self.solving.is_some() {
                self.visible = true;
            }
            for (cid, visible) in self.solved.drain(..) {
                *played.resolved.entry(cid).or_default() += 1;
                if visible || changed {
                    *played.visible.entry(cid).or_default() += 1;
                }
            }
        }
    }

    /// How many answers an engine prompt has, roughly, for the prompts the translator may answer itself: options,
    /// positions, free zones, cards to pick or sort. None for the others.
    pub fn choices(m: &Json) -> Option<usize> {
        Some(match Ask::lenient(m).ok()?? {
            Ask::EffectYn(_) | Ask::YesNo(_) => 2,
            Ask::Option(o) => o.options.len(),
            Ask::Card(c) | Ask::Tribute(c) => c.selects.len(),
            Ask::Chain(c) => c.selects.len(),
            Ask::Sum(s) => s.selects.len(),
            Ask::UnselectCard(u) => u.select_cards.len(),
            Ask::Place(p) | Ask::Disfield(p) => (!p.field_mask as u32 & 0x1F1F_1F1F).count_ones() as usize,
            Ask::Position(p) => (p.positions as u32 & 0xF).count_ones() as usize,
            Ask::Counter(c) => c.cards.len(),
            Ask::SortChain(s) | Ask::SortCard(s) => s.cards.len(),
            _ => return None,
        })
    }

    /// The cards a self-played duel knows: each CARD_Prop row's id and stats, the token rows by name, and the English
    /// names of the Konami ids (for another print of a card, see `Engine::new_duel_named`).
    #[derive(Clone, Debug, Default)]
    pub struct CardData {
        pub cards: HashMap<u16, CardStats>,
        pub tokens: HashMap<String, Vec<u16>>,
        pub names: HashMap<u16, String>,
        /// The Tuners' Konami ids.
        pub tuners: std::collections::HashSet<u16>,
        /// The engine plays the anime card versions (the `anime-cards` setting, #60).
        pub anime: bool,
        /// What the card profile says of its cards, by row (the duel check judges the engine's cards against it).
        pub expect: HashMap<u16, crate::profile::Expect>,
    }

    /// The cards the reviewed anime map approves, by what a deck keeps them in (Konami ids), for self-play with the
    /// `anime-cards` setting on.
    #[derive(Clone, Debug, Default)]
    pub struct AnimePool {
        pub monsters: Vec<u16>,
        pub spells: Vec<u16>,
        pub traps: Vec<u16>,
        /// Fusion and Synchro Monsters.
        pub extra: Vec<u16>,
    }

    /// The approved cards of `engine/anime-cards.json`, sorted by Konami id into an [`AnimePool`] by the type the engine
    /// (with the anime versions on) gives each.
    pub fn anime_pool(dir: &Path) -> Result<AnimePool, String> {
        anime_pool_where(dir, |_| true)
    }

    /// The [`anime_pool`] of the cards the card profile shows differently from the game's (the released categories: the map's
    /// `profile` entries), each class topped up with the ordinary pool's cards to `at_least` so a deck can be made: duels
    /// where nearly every card is one whose client row the profile rewrites.
    pub fn anime_pool_of_profile_cards(dir: &Path, at_least: usize) -> Result<AnimePool, String> {
        let mut pool = anime_pool_where(dir, |c| c["profile"].is_array())?;
        let base = anime_pool(dir)?;
        for (mine, all) in [(&mut pool.monsters, &base.monsters), (&mut pool.spells, &base.spells), (&mut pool.traps, &base.traps), (&mut pool.extra, &base.extra)] {
            for card in all {
                if mine.len() >= at_least {
                    break;
                }
                if !mine.contains(card) {
                    mine.push(*card);
                }
            }
        }
        Ok(pool)
    }

    /// [`anime_pool`] of the mapped cards whose map entry `keep` accepts.
    fn anime_pool_where(dir: &Path, keep: impl Fn(&Json) -> bool) -> Result<AnimePool, String> {
        let text = std::fs::read(dir.join("anime-cards.json")).map_err(|e| format!("anime-cards.json: {e}"))?;
        let map: Json = serde_json::from_slice(&text).map_err(|e| format!("anime-cards.json: {e}"))?;
        let mapped: Vec<u16> = map["cards"].as_array().into_iter().flatten().filter(|c| c["status"] == "mapped" && keep(c)).filter_map(|c| c["cid"].as_u64()).map(|c| c as u16).collect();
        let mut engine = Engine::start_with(dir, true).map_err(|e| format!("engine: {e}"))?;
        let list: Vec<(u16, &str)> = mapped.iter().map(|&c| (c, "")).collect();
        let mut pool = AnimePool::default();
        for chunk in list.chunks(100) {
            for a in engine.cards(chunk).map_err(|e| format!("engine: {e}"))? {
                let (cid, kind) = (a["cid"].as_u64().unwrap_or(0) as u16, a["data"]["type"].as_u64().unwrap_or(0));
                match kind {
                    k if k & 0x4000 != 0 => {}
                    k if k & 0x2040 != 0 => pool.extra.push(cid),
                    k if k & 1 != 0 => pool.monsters.push(cid),
                    k if k & 2 != 0 => pool.spells.push(cid),
                    k if k & 4 != 0 => pool.traps.push(cid),
                    _ => {}
                }
            }
        }
        Ok(pool)
    }

    /// [`deck`] from an anime pool: 40 cards (about half monsters, at most three of a card) and up to six Extra Deck cards.
    pub fn anime_deck(pool: &AnimePool, seed: u64) -> (Vec<u16>, Vec<u16>) {
        let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut counts: HashMap<u16, u32> = HashMap::new();
        let mut main = vec![];
        let (monsters, spells) = (18 + next(&mut rng) as usize % 5, 10 + next(&mut rng) as usize % 5);
        for (cards, n) in [(&pool.monsters, monsters), (&pool.spells, spells), (&pool.traps, 40 - monsters - spells)] {
            let mut added = 0;
            while added < n {
                let card = cards[next(&mut rng) as usize % cards.len()];
                let c = counts.entry(card).or_default();
                if *c < 3 {
                    *c += 1;
                    main.push(card);
                    added += 1;
                }
            }
        }
        let extra = (0..6).map(|_| pool.extra[next(&mut rng) as usize % pool.extra.len()]).collect();
        (main, extra)
    }

    /// Plays a duel between two seeded random players (`decks` by engine team: main and extra deck, Konami ids), the
    /// client seated as team `client_team` and, with `hidden`, given the opponent's hidden cards without rows as against a
    /// player. After every engine step the client model is checked against the engine: the first difference fails the
    /// duel with the seed, the step and the engine messages that came before it.
    pub fn play(dir: &Path, seed: u64, decks: [(&[u16], &[u16]); 2], client_team: usize, hidden: bool) -> Result<Played, String> {
        let data = CardData {
            cards: card_data(&decks),
            tokens: token_rows(),
            names: HashMap::new(),
            tuners: tuners(dir, &decks, false)?,
            anime: false,
            expect: HashMap::new(),
        };
        play_with(dir, seed, decks, &data, [&[], &[]], client_team, hidden)
    }

    pub type TokenCards = (HashMap<u16, CardStats>, HashMap<String, Vec<u16>>);

    /// The client's token cards (`engine/anime/client-tokens.tsv`) as rows of their own (60100 and up), for the anime
    /// cards' tokens: the rows and the rows by name.
    pub fn client_tokens(dir: &Path) -> Result<TokenCards, String> {
        let text = std::fs::read_to_string(dir.join("anime/client-tokens.tsv")).map_err(|e| format!("client-tokens.tsv: {e}"))?;
        let (mut cards, mut names): (HashMap<u16, CardStats>, HashMap<String, Vec<u16>>) = Default::default();
        for (i, line) in text.lines().enumerate() {
            let Some((cid, name)) = line.split_once('\t') else { continue };
            let row = 60100 + i as u16;
            cards.insert(row, CardStats { cid: cid.parse().map_err(|_| format!("client-tokens.tsv: {line}"))?, atk: 0, def: 0, level: 1, attribute: 1, race: 1 });
            names.entry(name.to_string()).or_default().push(row);
        }
        Ok((cards, names))
    }

    /// [`play`] with the anime card versions on (the `anime-cards` setting): the decks come from [`anime_deck`], and the
    /// client has every token card it has.
    pub fn play_anime(dir: &Path, seed: u64, decks: [(&[u16], &[u16]); 2], client_team: usize, hidden: bool) -> Result<Played, String> {
        play_anime_with(dir, seed, decks, client_team, hidden, |_| {})
    }

    /// [`play_anime`] with `tweak` run over the profile's expectations first: a test makes the profile say what the engine
    /// does not play, which the duel check must report.
    pub fn play_anime_with(
        dir: &Path,
        seed: u64,
        decks: [(&[u16], &[u16]); 2],
        client_team: usize,
        hidden: bool,
        tweak: impl FnOnce(&mut HashMap<u16, crate::profile::Expect>),
    ) -> Result<Played, String> {
        let (token_cards, tokens) = client_tokens(dir)?;
        let mut cards = card_data(&decks);
        cards.extend(token_cards);
        let data = CardData {
            cards,
            tokens,
            names: HashMap::new(),
            tuners: tuners(dir, &decks, true)?,
            anime: true,
            expect: {
                let mut expect = anime_expectations(dir, &decks)?;
                anime_card_problems(dir, &decks, &expect)?;
                tweak(&mut expect);
                expect
            },
        };
        play_with(dir, seed, decks, &data, [&[], &[]], client_team, hidden)
    }

    /// The decks' cards as the rules engine plays them with the anime versions on: its card data (Level, race, attribute...),
    /// asked once per card for all duels.
    fn anime_card_data(dir: &Path, decks: &[(&[u16], &[u16]); 2]) -> Result<BTreeMap<u16, Json>, String> {
        static KNOWN: std::sync::Mutex<BTreeMap<u16, Json>> = std::sync::Mutex::new(BTreeMap::new());
        let mut known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
        let mut ask: Vec<u16> = decks.iter().flat_map(|(m, e)| m.iter().chain(e.iter())).copied().filter(|c| !known.contains_key(c)).collect();
        ask.sort_unstable();
        ask.dedup();
        if !ask.is_empty() {
            let mut engine = Engine::start_with(dir, true).map_err(|e| format!("engine: {e}"))?;
            let list: Vec<(u16, &str)> = ask.iter().map(|&c| (c, "")).collect();
            for (cid, a) in ask.iter().zip(engine.cards(&list).map_err(|e| format!("engine: {e}"))?) {
                known.insert(*cid, a);
            }
        }
        Ok(decks.iter().flat_map(|(m, e)| m.iter().chain(e.iter())).filter_map(|c| known.get(c).map(|a| (*c, a.clone()))).collect())
    }

    /// The decks' cards the profile changes the Level, race or attribute of, against the engine's card data: the engine plays what
    /// the profile shows. (On the field those are the effects' as much as the card's, so the duel check judges only ATK and DEF
    /// there.)
    fn anime_card_problems(dir: &Path, decks: &[(&[u16], &[u16]); 2], expect: &HashMap<u16, crate::profile::Expect>) -> Result<(), String> {
        let data = anime_card_data(dir, decks)?;
        let mut problems = vec![];
        for (cid, answer) in &data {
            if let Some(want) = expect.get(cid) {
                for problem in want.card_problems(&answer["data"]) {
                    problems.push(format!("card {cid} differs from the card profile: {problem}"));
                }
            }
        }
        if problems.is_empty() { Ok(()) } else { Err(problems.join("\n")) }
    }

    /// What the anime profile says of the decks' cards (the self-play's rows are the Konami ids): the engine database's
    /// level, ATK, DEF, race and attribute of the approved cards, which the duel check holds the engine's monsters to.
    pub fn anime_expectations(dir: &Path, decks: &[(&[u16], &[u16]); 2]) -> Result<HashMap<u16, crate::profile::Expect>, String> {
        let anime = crate::profile::read_anime(dir)?;
        Ok(decks.iter().flat_map(|(m, e)| m.iter().chain(e.iter())).filter_map(|c| anime.get(c).map(|d| (*c, crate::profile::Expect::of_anime(d)))).collect())
    }

    /// The Tuners among the decks' cards (Konami ids), as the rules engine's card data has them (TYPE_TUNER), asked once
    /// per card for all duels: a Synchro's tuning tells them from the others, as the server does from the client's data.
    /// With `anime`, the anime versions' types (Boost Warrior is a Tuner in the anime, not in the game): the engine that plays
    /// them is the one asked.
    fn tuners(dir: &Path, decks: &[(&[u16], &[u16]); 2], anime: bool) -> Result<std::collections::HashSet<u16>, String> {
        static GAME: std::sync::Mutex<BTreeMap<u16, bool>> = std::sync::Mutex::new(BTreeMap::new());
        static ANIME: std::sync::Mutex<BTreeMap<u16, bool>> = std::sync::Mutex::new(BTreeMap::new());
        const TYPE_TUNER: u64 = 0x1000;
        let mut known = if anime { &ANIME } else { &GAME }.lock().unwrap_or_else(|e| e.into_inner());
        let mut ask: Vec<u16> = decks.iter().flat_map(|(m, e)| m.iter().chain(e.iter())).copied().filter(|c| !known.contains_key(c)).collect();
        ask.sort_unstable();
        ask.dedup();
        if !ask.is_empty() {
            let mut engine = Engine::start_with(dir, anime).map_err(|e| format!("engine: {e}"))?;
            let list: Vec<(u16, &str)> = ask.iter().map(|&c| (c, "")).collect();
            let answers = engine.cards(&list).map_err(|e| format!("engine: {e}"))?;
            for (cid, a) in ask.iter().zip(&answers) {
                known.insert(*cid, a["data"]["type"].as_u64().is_some_and(|t| t & TYPE_TUNER != 0));
            }
        }
        Ok(decks.iter().flat_map(|(m, e)| m.iter().chain(e.iter())).copied().filter(|c| known.get(c) == Some(&true)).collect())
    }

    /// [`play`] with real card data: `decks` are CARD_Prop rows of `data`, and each team's `top` rows (cards of its main
    /// deck) go on top of its shuffled deck (the last one drawn first), into the opening hand. A script error the
    /// engine reports fails the duel too.
    pub fn play_with(
        dir: &Path,
        seed: u64,
        decks: [(&[u16], &[u16]); 2],
        data: &CardData,
        top: [&[u16]; 2],
        client_team: usize,
        hidden: bool,
    ) -> Result<Played, String> {
        let mut rng = seed | 1;
        let cards = data.cards.clone();
        let cid = |row: &u16| cards.get(row).map_or(0, |c| c.cid);
        // The client's engine player of team 0 (team 0 goes first), and the decks by client player.
        let first = if client_team == 0 { CLIENT } else { CPU };
        let (mine, theirs) = (decks[client_team], decks[1 - client_team]);
        let build = if hidden { Translator::against_player } else { Translator::new };
        let (mut t, mut table) = build(first, [mine.0, theirs.0], cards.clone());
        t.tokens(data.tokens.clone());
        t.tuners(data.tuners.clone());
        table.extend(t.extra_decks([mine.1, theirs.1]));
        let mut model = ClientModel::new(first, cards.clone());
        model.expect_profile(data.expect.clone());
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
        opening.extend([command(cmd::DUEL_START, [0; 4]), command(cmd::SET_LP, [CLIENT, lp, 0, 0]), command(cmd::SET_LP, [CPU, lp, 0, 0])]);
        opening.extend(piles());
        model.apply_all(&opening);

        let mut engine = Engine::start_with(dir, data.anime).map_err(|e| format!("engine: {e}"))?;
        let mut shuffled = [decks[0].0.to_vec(), decks[1].0.to_vec()];
        for (d, top) in shuffled.iter_mut().zip(top) {
            shuffle(&mut rng, d);
            for row in top {
                if let Some(at) = d.iter().position(|r| r == row) {
                    let card = d.remove(at);
                    d.push(card);
                }
            }
        }
        let ids = |rows: &[u16]| rows.iter().map(cid).collect::<Vec<u16>>();
        let (m0, e0, m1, e1) = (ids(&shuffled[0]), ids(decks[0].1), ids(&shuffled[1]), ids(decks[1].1));
        // `YO3_CHECK_SKILLS=1`: the side the client faces has the heroes' Heart of the Cards (crate::npc), which moves
        // cards within the Deck without a message: the client's view must still follow.
        let skills: Vec<(u32, String)> = if std::env::var_os("YO3_CHECK_SKILLS").is_some() {
            vec![(1 - client_team as u32, "heart_of_the_cards".into())]
        } else {
            vec![]
        };
        let mut answer = engine.new_duel_with(seed, lp, [(&m0, &e0), (&m1, &e1)], &data.names, &skills);
        let mut played = Played::default();
        let mut effects = Effects::default();
        let mut prompt: Option<Json> = None;
        let mut retries = 0;
        let mut actions = 0;
        while played.steps < MAX_STEPS {
            played.steps += 1;
            let a = answer.map_err(|e| format!("seed {seed}: engine: {e}"))?;
            if !a.errors.is_empty() {
                return Err(format!("seed {seed}, step {}: the engine reported {:?}", played.steps, a.errors));
            }
            model.observe_engine(&a.messages);
            if std::env::var_os("YO3_CHECK_TRACE").is_some() {
                for m in &a.messages {
                    eprintln!("step {}: {m}", played.steps);
                }
            }
            for m in &a.messages {
                effects.message(m, &mut played);
            }
            let events: Vec<Event> = a.messages.iter().filter_map(|m| Event::lenient(m).ok().flatten()).collect();
            for m in &a.messages {
                if let Some(kind) = m["type"].as_u64() {
                    *played.kinds.entry(kind).or_default() += 1;
                }
            }
            for e in &events {
                match e {
                    Event::Chaining(c) => *played.activated.entry(c.loc.cid.unwrap_or(0) as u16).or_default() += 1,
                    Event::Summoning(s) | Event::SpSummoning(s) | Event::FlipSummoning(s) => *played.summoned.entry(s.loc.cid.unwrap_or(0) as u16).or_default() += 1,
                    Event::Move(mv) if mv.to.overlay_sequence.is_some() || mv.from.overlay_sequence.is_some() => played.materials += 1,
                    Event::NewTurn(_) => {
                        played.turns += 1;
                        actions = 0;
                    }
                    _ => {}
                }
            }
            // Every window the core marks must be one the translator knows how to treat (CHAIN_WINDOWS).
            let windows = || events.iter().filter_map(|e| match e { Event::Hint(h) if h.hint_type == hint::EVENT => Some(h), _ => None });
            if let Some(h) = windows().find(|h| chain_window(h.hint).is_none()) {
                return Err(format!("seed {seed}, step {}: the core's window {} (MSG_HINT HINT_EVENT) isn't in CHAIN_WINDOWS", played.steps, h.hint));
            }
            // The client's window for the prompt this step ends with: the last one the core marked for its player.
            let window = windows().rev().find(|h| h.player == client_team as u64).map(|h| h.hint);
            let mut out = t.events(&a.messages);
            settle_positions(&mut out);
            model.apply_all(&out);
            for m in &out {
                let int = |k: &str| m.get(k).and_then(yo3_proto::Value::as_u32).unwrap_or(0);
                match int("id") {
                    0x3A => *played.animations.entry(int("param3") >> 16).or_default() += 1,
                    0x2F if int("param2") == zone::HAND => played.hand_materials += 1,
                    _ => {}
                }
            }
            let state = engine.state().map_err(|e| format!("seed {seed}: engine state: {e}"))?;
            effects.field(&state, &mut played);
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
            let mut problems = model.check(&state);
            // Every engine message fits the struct the server reads it as (yo3_ocg::message).
            problems.extend(a.messages.iter().filter_map(|m| crate::ocg::message::parse(m).err()));
            if !problems.is_empty() {
                return Err(fail(problems));
            }
            if a.status != "await" || played.turns > MAX_TURNS {
                played.ended = a.status == "end";
                return Ok(played);
            }
            let last = a.messages.last().cloned().unwrap_or(Json::Null);
            let response = if last["type"].as_u64() == Some(msg::RETRY) {
                // The core refused the last answer: the safe one this time, a few times at most.
                retries += 1;
                if retries > 4 {
                    return Err(fail(vec![format!("the core keeps refusing answers to {}", prompt.as_ref().map(short).unwrap_or_default())]));
                }
                bot_response(prompt.as_ref().unwrap_or(&last))
            } else {
                retries = 0;
                // The translator's windows for the client (its prompts), then the random player's answer for whoever is
                // asked (the translator's own answer for the CPU is the placeholder AI's).
                match t.prompt(&last) {
                    Prompt::Client(mut msgs) => {
                        let int = |m: &yo3_proto::Message, k: &str| m.get(k).and_then(yo3_proto::Value::as_u32).unwrap_or(0);
                        if msgs.iter().any(|m| int(m, "id") == 0x36 && int(m, "param1") == 8 && int(m, "param3") & 0xFFFF >= super::FIRST_TOKEN) {
                            played.token_positions += 1;
                        }
                        if msgs.iter().any(|m| int(m, "id") == cmd::BATTLE_SELECT) {
                            played.field_targets += 1;
                        }
                        settle_positions(&mut msgs);
                        model.apply_all(&msgs);
                        let mut problems = model.check(&state);
                        // A chance that matters, on the default Activation Message: shown, or answered No by the client
                        // itself, which only a window without optional triggers may be (#53: Stardust Dragon's return).
                        if let Ok(Some(Ask::Chain(c))) = Ask::lenient(&last)
                            && c.spe_count > 0
                        {
                            let level = msgs.iter().find(|m| int(m, "id") == cmd::DIALOG && int(m, "param1") == dialog::EFFECT).map(|m| int(m, "param3"));
                            let phase = window.and_then(chain_window) == Some(ChainWindow::Phase);
                            match level {
                                Some(level) if level < SHOWN_BY_DEFAULT && phase => problems.push(format!(
                                    "the default Activation Message answers No for the player in the {} window ({}), where optional triggers come: {}",
                                    window.map_or("?", |h| crate::translate::CHAIN_WINDOWS.iter().find(|w| w.0 == h).map_or("?", |w| w.1)),
                                    window.unwrap_or(0),
                                    short(&last)
                                )),
                                Some(level) if level < SHOWN_BY_DEFAULT => *played.hidden_by_default.entry(window.unwrap_or(0)).or_default() += 1,
                                _ if phase => played.phase_windows += 1,
                                _ => {}
                            }
                        }
                        // Every answer the engine offers must be one the client's windows can give.
                        problems.extend(t.unreachable_answers().into_iter().map(|what| format!("the client can't answer {what}")));
                        if !problems.is_empty() {
                            return Err(fail(problems));
                        }
                    }
                    Prompt::Answer(_) if Ask::lenient(&last).ok().flatten().is_some_and(|a| a.player() == client_team as u64) && choices(&last).is_some_and(|n| n > 1) => {
                        let kind = last["type"].as_u64().unwrap_or(0);
                        // The options of an effect always reach the player (a card list when the client has no labels).
                        if kind == msg::SELECT_OPTION || kind == msg::SELECT_SUM || kind == msg::SELECT_EFFECTYN {
                            return Err(fail(vec![format!("the server chose for the player: {}", short(&last))]));
                        }
                        *played.decided.entry(kind).or_default() += 1;
                    }
                    Prompt::Answer(_) => {}
                }
                prompt = Some(last.clone());
                actions += 1;
                random_answer(&last, &mut rng, actions > ACTIONS_PER_TURN)
            };
            t.answered(&last, &response);
            answer = engine.respond(response.json());
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
    pub fn random_answer(m: &Json, rng: &mut u64, tired: bool) -> Response {
        let mut pick = |n: usize| (next(rng) as usize) % n.max(1);
        let Ok(Some(ask)) = Ask::lenient(m) else { return bot_response(m) };
        match &ask {
            Ask::IdleCmd(p) => {
                let mut options: Vec<(u64, Option<usize>)> = vec![];
                if !tired {
                    let lists = [(p.summons.len(), idle::SUMMON), (p.special_summons.len(), idle::SPSUMMON), (p.pos_changes.len(), idle::REPOS),
                        (p.monster_sets.len(), idle::MONSTER_SET), (p.spell_sets.len(), idle::SPELL_SET), (p.activates.len(), idle::ACTIVATE)];
                    for (len, action) in lists {
                        options.extend((0..len).map(|i| (action, Some(i))));
                    }
                }
                let leave: Vec<(u64, Option<usize>)> =
                    [(p.to_bp, idle::TO_BP), (p.to_ep, idle::TO_EP)].into_iter().filter(|(ok, _)| *ok).map(|(_, a)| (a, None)).collect();
                // Leaving is one option among the others (always, once tired).
                if !leave.is_empty() && (options.is_empty() || pick(options.len() + 2) < 2) {
                    let (action, index) = leave[pick(leave.len())];
                    return Response::IdleCmd { action, index };
                }
                if options.is_empty() {
                    return bot_response(m);
                }
                let (action, index) = options[pick(options.len())];
                Response::IdleCmd { action, index }
            }
            Ask::BattleCmd(p) => {
                let mut options: Vec<(u64, Option<usize>)> = vec![];
                if !tired {
                    options.extend((0..p.attacks.len()).map(|i| (battle::ATTACK, Some(i))));
                    options.extend((0..p.chains.len()).map(|i| (battle::CHAIN, Some(i))));
                }
                if options.is_empty() || pick(options.len() + 1) == 0 {
                    let action = if p.to_m2 && pick(2) == 0 { battle::TO_M2 } else { battle::TO_EP };
                    return Response::BattleCmd { action, index: None };
                }
                let (action, index) = options[pick(options.len())];
                Response::BattleCmd { action, index }
            }
            Ask::Chain(c) => {
                let n = c.selects.len();
                Response::Chain((n > 0 && (c.forced || pick(2) == 0)).then(|| pick(n)))
            }
            Ask::EffectYn(_) => Response::EffectYn(pick(2) == 0),
            Ask::YesNo(_) => Response::YesNo(pick(2) == 0),
            Ask::Option(o) => Response::Option(pick(o.options.len())),
            Ask::Card(c) | Ask::Tribute(c) => {
                let n = c.selects.len();
                let min = (c.min as usize).min(n);
                let max = (c.max as usize).clamp(min, n.max(min));
                let count = min + pick(max - min + 1);
                let mut all: Vec<usize> = (0..n).collect();
                for i in (1..all.len()).rev() {
                    all.swap(i, pick(i + 1));
                }
                all.truncate(count);
                all.sort_unstable();
                if matches!(ask, Ask::Card(_)) { Response::Card(Some(all)) } else { Response::Tribute(Some(all)) }
            }
            Ask::UnselectCard(u) => {
                let n = u.select_cards.len();
                Response::UnselectCard((n != 0 && !(u.can_finish && pick(2) == 0)).then(|| pick(n)))
            }
            Ask::Place(p) | Ask::Disfield(p) => {
                // Random free zones, `count` of them (field_mask bits set = unavailable): the player's monster zones
                // 0-4, Spell/Trap 8-13, then the opponent's 16-20 and 24-29.
                let (mask, me) = (p.field_mask, p.player);
                let free: Vec<(u64, u64, u64)> = [(me, loc::MZONE, 0..5, 0), (me, loc::SZONE, 0..6, 8), (1 - me, loc::MZONE, 0..5, 16), (1 - me, loc::SZONE, 0..6, 24)]
                    .into_iter()
                    .flat_map(|(p, l, seqs, base)| seqs.map(move |s| (p, l, s, base + s)))
                    .filter(|&(_, _, _, b)| mask & 1 << b == 0)
                    .map(|(p, l, s, _)| (p, l, s))
                    .collect();
                let count = (p.count as usize).max(1);
                if free.len() < count {
                    return bot_response(m);
                }
                let mut free = free;
                let places: Vec<Json> = (0..count)
                    .map(|_| {
                        let (player, location, sequence) = free.remove(pick(free.len()));
                        json!({ "player": player, "location": location, "sequence": sequence })
                    })
                    .collect();
                if matches!(ask, Ask::Place(_)) { Response::Place(places) } else { Response::Disfield(places) }
            }
            Ask::Position(p) => {
                let allowed = if p.positions == 0 { pos::FACEUP_ATTACK } else { p.positions };
                let bits: Vec<u64> = [pos::FACEUP_ATTACK, pos::FACEDOWN_ATTACK, pos::FACEUP_DEFENSE, pos::FACEDOWN_DEFENSE].into_iter().filter(|b| allowed & b != 0).collect();
                Response::Position(if bits.is_empty() { pos::FACEUP_ATTACK } else { bits[pick(bits.len())] })
            }
            // An order (chain links, cards): half the time a random rank per card, else the engine's own.
            Ask::SortChain(sort) | Ask::SortCard(sort) => {
                let n = sort.cards.len();
                if n < 2 || pick(2) == 0 {
                    return bot_response(m);
                }
                let mut order: Vec<usize> = (0..n).collect();
                for i in (1..n).rev() {
                    order.swap(i, pick(i + 1));
                }
                Response::Sort(Some(json!(order)))
            }
            _ => bot_response(m),
        }
    }
}
