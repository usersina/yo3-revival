//! What an NPC says during a duel, in the duel's chat window. The client's own speech for NPCs is fixed (every NPC
//! slot is `/preset 0`, docs/protocol.md), so the characters speak through chat instead: a `0x2455` push to the
//! player's front connection, which the duel screen's chat window shows (live: `chat_type` 0 and 1 show there,
//! 0 with the speaker's name in red, like the opponent's name plate; 2 and up don't). The lines are ours
//! (`npc_lines.txt`), chosen at moments read from the rules engine's messages: the duel's start and end, the NPC's
//! turns, its big monsters, its direct attacks, the damage it takes, a hero's draw, and Pegasus' Millennium Eye
//! (a card in the player's hand or one the player just Set; his duel bot sees them too, `crate::npc`).
//!
//! The engine runs a whole turn at once and the client then plays it out, so each line carries how far into those
//! events it belongs (`Said::after`, a rough pace per event), and the session sends it then.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde_json::Value as Json;
use yo3_proto::{Message, Value};

use crate::engine::Engine;
use crate::front::World;

const LINES: &str = include_str!("npc_lines.txt");

const MSG_WIN: u64 = 5;
const MSG_NEW_TURN: u64 = 40;
const MSG_NEW_PHASE: u64 = 41;
const MSG_MOVE: u64 = 50;
const MSG_POS_CHANGE: u64 = 53;
const MSG_SET: u64 = 54;
const MSG_SUMMONING: u64 = 60;
const MSG_SPSUMMONING: u64 = 62;
const MSG_FLIPSUMMONING: u64 = 64;
const MSG_CHAINING: u64 = 70;
const MSG_DRAW: u64 = 90;
const MSG_DAMAGE: u64 = 91;
const MSG_RECOVER: u64 = 92;
const MSG_PAY_LPCOST: u64 = 100;
const MSG_ATTACK: u64 = 110;
const MSG_BATTLE: u64 = 111;

/// The chat type of an NPC's line: shown in the duel's chat window with the speaker's name in red (ChatWindow
/// `0x435950`: colour 0xB for chat type 0, its tab 7).
pub const NPC_CHAT: u32 = 0;

/// How the client's pace compares with `pace()`'s estimate (a line's `after` is multiplied by it). Live (Xvfb,
/// 2026-11-13): Pegasus' direct attack showed 21.6 s into his turn, `pace()` put it at 14.2 s.
pub const PACE: f64 = 1.5;
/// Before the opening's first engine message the client sets the field up and fills its card table (live: his first
/// turn began about 13 s after the turn order was chosen), in `pace()` seconds.
const OPENING: f64 = 3.0;
/// The least time between two lines.
pub const GAP: std::time::Duration = std::time::Duration::from_millis(1500);

/// A monster at least this strong is an "ace" when the NPC summons it.
pub const ACE_ATK: u32 = 2400;
/// Damage at least this big in one go makes the NPC complain.
pub const HURT: u64 = 2000;
/// Lines an NPC may say in one turn (the duel's end aside).
pub const PER_TURN: u32 = 2;

/// One character's lines by kind ("greet", "ace", ...).
pub type Lines = HashMap<&'static str, Vec<&'static str>>;

/// Every character's lines, by the NPC's name.
pub fn table() -> &'static HashMap<&'static str, Lines> {
    static TABLE: OnceLock<HashMap<&'static str, Lines>> = OnceLock::new();
    TABLE.get_or_init(|| parse(LINES))
}

/// `[Name]` starts a character; `kind: text` adds a line; `#` lines are comments.
fn parse(text: &'static str) -> HashMap<&'static str, Lines> {
    let mut table: HashMap<&str, Lines> = HashMap::new();
    let mut current = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = Some(name);
            table.entry(name).or_default();
        } else if let (Some(name), Some((kind, said))) = (current, line.split_once(':')) {
            table.entry(name).or_default().entry(kind.trim()).or_default().push(said.trim());
        }
    }
    table
}

/// A line to say, `after` seconds into the events the client is about to play.
#[derive(Clone, Debug, PartialEq)]
pub struct Said {
    pub after: f64,
    pub text: String,
}

/// What the banter needs to know about the duel beyond the engine's messages.
pub trait Duel {
    fn card_name(&self, cid: u16) -> Option<&str>;
    fn card_atk(&self, cid: u16) -> u32;
    /// The cards (cids) in engine team `team`'s hand now.
    fn hand(&mut self, team: u64) -> Vec<u16>;
}

/// The duel as the rules engine and the client's card data know it.
pub struct Live<'a> {
    pub engine: &'a mut Engine,
    pub world: &'a World,
}

impl Duel for Live<'_> {
    fn card_name(&self, cid: u16) -> Option<&str> {
        self.world.card_names.get(&cid).map(String::as_str)
    }

    fn card_atk(&self, cid: u16) -> u32 {
        self.world.cards.values().find(|c| c.cid == cid).map_or(0, |c| c.atk)
    }

    fn hand(&mut self, team: u64) -> Vec<u16> {
        let state = self.engine.state().unwrap_or_default();
        state[team as usize]["hand"].as_array().into_iter().flatten().filter_map(|c| c["cid"].as_u64()).map(|c| c as u16).collect()
    }
}

/// An NPC's line as the chat window's push (`0x2455`, read like a player's: from_id, pname, chat_type, chat_str).
pub fn chat_line(npc_id: u32, npc_name: &str, text: &str) -> Message {
    let line = Message::record()
        .with("from_id", Value::U32(npc_id))
        .with("pname", Value::text(npc_name))
        .with("chat_type", Value::U32(NPC_CHAT))
        .with("chat_str", Value::text(text));
    Message::reply(0x2455).item(line)
}

/// One NPC's voice through one duel.
pub struct Banter {
    lines: &'static Lines,
    you: String,
    /// The NPC's engine team.
    npc: u64,
    rng: u64,
    turns: u32,
    turn_player: Option<u64>,
    said_this_turn: u32,
    /// Kinds said this turn: one line of a kind a turn.
    kinds_this_turn: HashSet<&'static str>,
    /// Each kind's last line, not said twice in a row.
    last: HashMap<&'static str, usize>,
    /// Kinds said at least once: the first time a moment comes, the NPC always speaks.
    said_once: HashSet<&'static str>,
    aces: HashSet<u16>,
    /// Seconds of events the client has yet to play, as the messages are read.
    clock: f64,
    over: bool,
}

impl Banter {
    /// The banter of the NPC named `npc_name` (none if it has no lines) for engine team `npc`, to the player `you`.
    pub fn new(npc_name: &str, npc: u64, you: &str, seed: u64) -> Option<Banter> {
        let lines = table().get(npc_name)?;
        Some(Banter {
            lines,
            you: you.to_string(),
            npc,
            rng: seed | 1,
            turns: 0,
            turn_player: None,
            said_this_turn: 0,
            kinds_this_turn: HashSet::new(),
            last: HashMap::new(),
            said_once: HashSet::new(),
            aces: HashSet::new(),
            clock: 0.0,
            over: false,
        })
    }

    /// The opening line.
    pub fn greet(&mut self) -> Vec<Said> {
        self.line("greet", None).map(|text| vec![Said { after: 1.0, text }]).unwrap_or_default()
    }

    /// The client is about to play a new batch of events: the times start again (after the opening's setup, the
    /// first time).
    pub fn begin(&mut self) {
        self.clock = if self.turns == 0 { OPENING } else { 0.0 };
    }

    /// The lines for the engine's `messages`.
    pub fn observe(&mut self, messages: &[Json], duel: &mut dyn Duel) -> Vec<Said> {
        let mut out = vec![];
        for m in messages {
            let Some(kind) = m["type"].as_u64() else { continue };
            self.clock += pace(kind, m);
            if self.over {
                continue;
            }
            let player = m["player"].as_u64();
            let controller = m["controller"].as_u64();
            let said = match kind {
                MSG_NEW_TURN => {
                    self.turns += 1;
                    self.turn_player = player;
                    self.said_this_turn = 0;
                    self.kinds_this_turn.clear();
                    if player != Some(self.npc) {
                        None
                    } else if self.has("eye_hand") && self.chance("eye_hand", 0.5) {
                        let hand = duel.hand(1 - self.npc);
                        let card = self.pick(&hand).and_then(|cid| duel.card_name(cid)).map(str::to_string);
                        card.and_then(|card| self.line("eye_hand", Some(&card)))
                    } else if self.turns > 2 && self.chance("turn", 0.25) {
                        self.line("turn", None)
                    } else {
                        None
                    }
                }
                // A draw in the NPC's own turn (not the opening hand): the hero's Heart of the Cards.
                MSG_DRAW if player == Some(self.npc) && self.turn_player == Some(self.npc) && self.has("heart") => {
                    self.chance("heart", 0.35).then(|| self.line("heart", None)).flatten()
                }
                MSG_SET if controller == Some(1 - self.npc) && self.has("eye_set") => {
                    let card = cid(m).and_then(|c| duel.card_name(c)).map(str::to_string);
                    card.filter(|_| self.chance("eye_set", 0.5)).and_then(|card| self.line("eye_set", Some(&card)))
                }
                MSG_SUMMONING | MSG_SPSUMMONING | MSG_FLIPSUMMONING if controller == Some(self.npc) => {
                    let ace = cid(m).filter(|&c| duel.card_atk(c) >= ACE_ATK && !self.aces.contains(&c));
                    let card = ace.and_then(|c| duel.card_name(c).map(|n| (c, n.to_string())));
                    card.and_then(|(c, card)| {
                        self.aces.insert(c);
                        self.line("ace", Some(&card))
                    })
                }
                MSG_ATTACK if m["card"]["controller"].as_u64() == Some(self.npc) && m["target"].is_null() => {
                    self.chance("direct", 0.3).then(|| self.line("direct", None)).flatten()
                }
                MSG_DAMAGE if player == Some(self.npc) && m["amount"].as_u64().unwrap_or(0) >= HURT => {
                    self.chance("hurt", 0.6).then(|| self.line("hurt", None)).flatten()
                }
                MSG_WIN => {
                    self.over = true;
                    match player {
                        Some(p) if p == self.npc => self.say("win", None),
                        Some(p) if p == 1 - self.npc => self.say("lose", None),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(text) = said {
                out.push(Said { after: self.clock, text });
            }
        }
        out
    }

    fn has(&self, kind: &str) -> bool {
        self.lines.get(kind).is_some_and(|l| !l.is_empty())
    }

    /// Whether to speak at this moment: always the first time, then by `p`.
    fn chance(&mut self, kind: &'static str, p: f64) -> bool {
        let roll = (self.next() % 1000) as f64 / 1000.0;
        !self.said_once.contains(kind) || roll < p
    }

    /// One of the `kind` lines, within the turn's allowance.
    fn line(&mut self, kind: &'static str, card: Option<&str>) -> Option<String> {
        if self.said_this_turn >= PER_TURN || self.kinds_this_turn.contains(kind) {
            return None;
        }
        let text = self.say(kind, card)?;
        self.kinds_this_turn.insert(kind);
        self.said_once.insert(kind);
        self.said_this_turn += 1;
        Some(text)
    }

    /// One of the `kind` lines with its names filled in.
    fn say(&mut self, kind: &'static str, card: Option<&str>) -> Option<String> {
        let lines = self.lines.get(kind).filter(|l| !l.is_empty())?;
        let mut i = (self.next() % lines.len() as u64) as usize;
        if lines.len() > 1 && self.last.get(kind) == Some(&i) {
            i = (i + 1) % lines.len();
        }
        self.last.insert(kind, i);
        let pick = lines[i];
        if pick.contains("{card}") && card.is_none() {
            return None;
        }
        Some(pick.replace("{card}", card.unwrap_or_default()).replace("{you}", &self.you))
    }

    fn pick(&mut self, cards: &[u16]) -> Option<u16> {
        (!cards.is_empty()).then(|| cards[(self.next() % cards.len() as u64) as usize])
    }

    fn next(&mut self) -> u64 {
        crate::duel::next(&mut self.rng)
    }
}

fn cid(m: &Json) -> Option<u16> {
    m["cid"].as_u64().filter(|&c| c != 0).map(|c| c as u16)
}

/// About how long the client takes to show an engine message, in seconds (its animations; a rough pace).
fn pace(kind: u64, m: &Json) -> f64 {
    match kind {
        MSG_NEW_TURN => 1.5,
        MSG_NEW_PHASE => 0.5,
        MSG_DRAW => 0.5 * m["drawn"].as_array().map_or(1, Vec::len) as f64,
        MSG_MOVE | MSG_POS_CHANGE => 0.5,
        MSG_SET => 0.8,
        MSG_SUMMONING | MSG_SPSUMMONING | MSG_FLIPSUMMONING => 1.5,
        MSG_CHAINING => 1.2,
        MSG_ATTACK => 1.0,
        MSG_BATTLE => 1.5,
        MSG_DAMAGE | MSG_RECOVER | MSG_PAY_LPCOST => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Fake {
        hand: Vec<u16>,
    }

    impl Duel for Fake {
        fn card_name(&self, cid: u16) -> Option<&str> {
            match cid {
                1 => Some("Mirror Force"),
                2 => Some("Dark Magician"),
                3 => Some("Blue-Eyes White Dragon"),
                4 => Some("Kuriboh"),
                _ => None,
            }
        }

        fn card_atk(&self, cid: u16) -> u32 {
            match cid {
                2 => 2500,
                3 => 3000,
                _ => 300,
            }
        }

        fn hand(&mut self, _team: u64) -> Vec<u16> {
            self.hand.clone()
        }
    }

    fn duel() -> Fake {
        Fake { hand: vec![2] }
    }

    #[test]
    fn every_character_greets_and_ends_and_the_special_ones_have_their_lines() {
        let table = table();
        for (name, ..) in crate::npc::CHARACTERS {
            let lines = table.get(name).unwrap_or_else(|| panic!("{name} has no lines"));
            for kind in ["greet", "turn", "ace", "direct", "hurt", "win", "lose"] {
                assert!(lines.get(kind).is_some_and(|l| !l.is_empty()), "{name} has no {kind} line");
            }
            for (kind, said) in lines.iter().flat_map(|(k, l)| l.iter().map(move |s| (*k, *s))) {
                let cards = matches!(kind, "ace" | "eye_hand" | "eye_set");
                assert_eq!(said.contains("{card}"), cards, "{name} {kind}: {said}");
                assert!(said.len() <= 90, "{name}: too long for the chat window: {said}");
            }
        }
        for hero in crate::npc::HEROES {
            assert!(table[hero].contains_key("heart"), "{hero} has no Heart of the Cards");
        }
        assert!(table["Maximillion Pegasus"].contains_key("eye_hand"));
        assert!(table["Maximillion Pegasus"].contains_key("eye_set"));
        assert_eq!(table.len(), crate::npc::CHARACTERS.len(), "a character without a style, or a misspelt name");
    }

    #[test]
    fn pegasus_reads_your_hand_and_sees_what_you_set() {
        let mut b = Banter::new("Maximillion Pegasus", 1, "Tester", 7).unwrap();
        let greet = b.greet();
        assert!(greet[0].text.contains("Tester") || !greet[0].text.contains("{you}"));
        let mut d = duel();
        let set = json!({ "type": MSG_SET, "cid": 1, "controller": 0, "location": 8, "sequence": 0, "position": 10 });
        let said = b.observe(&[json!({ "type": MSG_NEW_TURN, "player": 0 }), set], &mut d);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].text.contains("Mirror Force"), "{said:?}");
        assert!(said[0].after > 1.5, "said after the Set shows, not before: {said:?}");
        let said = b.observe(&[json!({ "type": MSG_NEW_TURN, "player": 1 })], &mut d);
        assert!(said[0].text.contains("Dark Magician"), "his first turn reads the hand: {said:?}");
    }

    #[test]
    fn only_pegasus_sees_and_only_the_heroes_call_on_their_cards() {
        let mut d = duel();
        let mut kaiba = Banter::new("Seto Kaiba", 1, "Tester", 7).unwrap();
        let set = json!({ "type": MSG_SET, "cid": 1, "controller": 0, "location": 8, "sequence": 0, "position": 10 });
        let turn = json!({ "type": MSG_NEW_TURN, "player": 1 });
        let draw = json!({ "type": MSG_DRAW, "player": 1, "drawn": [{ "cid": 4 }] });
        let said = kaiba.observe(&[set.clone(), turn.clone(), draw.clone()], &mut d);
        assert!(said.iter().all(|s| !s.text.contains("Mirror Force") && !s.text.contains("Dark Magician")), "{said:?}");
        assert!(said.is_empty(), "Kaiba has nothing to say about a draw: {said:?}");
        let mut yugi = Banter::new("Yami Yugi", 1, "Tester", 7).unwrap();
        let said = yugi.observe(&[turn, draw], &mut d);
        assert_eq!(said.len(), 1, "his first draw: {said:?}");
        assert!(table()["Yami Yugi"]["heart"].contains(&said[0].text.as_str()));
    }

    #[test]
    fn an_ace_is_named_once_and_the_end_speaks_for_the_winner() {
        let mut d = duel();
        let mut kaiba = Banter::new("Seto Kaiba", 0, "Tester", 3).unwrap();
        let summon = json!({ "type": MSG_SPSUMMONING, "cid": 3, "controller": 0, "location": 4, "sequence": 2, "position": 1 });
        let said = kaiba.observe(&[json!({ "type": MSG_NEW_TURN, "player": 0 }), summon.clone()], &mut d);
        assert!(said.iter().any(|s| s.text.contains("Blue-Eyes White Dragon")), "{said:?}");
        let said = kaiba.observe(&[json!({ "type": MSG_NEW_TURN, "player": 0 }), summon], &mut d);
        assert!(said.iter().all(|s| !s.text.contains("Blue-Eyes")), "the same ace again goes unsaid: {said:?}");
        let small = json!({ "type": MSG_SUMMONING, "cid": 4, "controller": 0, "location": 4, "sequence": 1, "position": 1 });
        assert!(kaiba.observe(&[small], &mut d).is_empty(), "Kuriboh is no ace");
        let said = kaiba.observe(&[json!({ "type": MSG_WIN, "player": 0, "reason": 1 })], &mut d);
        assert!(table()["Seto Kaiba"]["win"].contains(&said[0].text.as_str()), "{said:?}");
        assert!(kaiba.observe(&[json!({ "type": MSG_NEW_TURN, "player": 0 })], &mut d).is_empty(), "nothing after the end");
        let mut lost = Banter::new("Seto Kaiba", 0, "Tester", 3).unwrap();
        let said = lost.observe(&[json!({ "type": MSG_WIN, "player": 1, "reason": 1 })], &mut d);
        assert!(table()["Seto Kaiba"]["lose"].contains(&said[0].text.replace("Tester", "{you}").as_str()), "{said:?}");
    }

    #[test]
    fn a_kind_speaks_once_a_turn_and_never_the_same_line_twice_running() {
        let mut d = duel();
        let mut b = Banter::new("Maximillion Pegasus", 1, "Tester", 5).unwrap();
        let set = |c: u16| json!({ "type": MSG_SET, "cid": c, "controller": 0, "location": 8, "sequence": 0, "position": 10 });
        let said = b.observe(&[json!({ "type": MSG_NEW_TURN, "player": 0 }), set(1), set(4)], &mut d);
        assert_eq!(said.len(), 1, "one Set named a turn: {said:?}");
        let mut lines = vec![];
        for _ in 0..40 {
            let turn = json!({ "type": MSG_NEW_TURN, "player": 1 });
            lines.extend(b.observe(&[turn, json!({ "type": MSG_WIN, "player": 3 })], &mut d).into_iter().map(|s| s.text));
            b.over = false;
        }
        assert!(lines.windows(2).all(|w| w[0] != w[1]), "{lines:?}");
    }

    #[test]
    fn a_turn_says_at_most_two_lines() {
        let mut d = duel();
        let mut b = Banter::new("Maximillion Pegasus", 1, "Tester", 11).unwrap();
        let mut batch = vec![json!({ "type": MSG_NEW_TURN, "player": 1 })];
        for c in [2u16, 3] {
            batch.push(json!({ "type": MSG_SUMMONING, "cid": c, "controller": 1, "location": 4, "sequence": 0, "position": 1 }));
        }
        batch.push(json!({ "type": MSG_DAMAGE, "player": 1, "amount": 3000 }));
        let said = b.observe(&batch, &mut d);
        assert_eq!(said.len(), PER_TURN as usize, "{said:?}");
        assert!(said.windows(2).all(|w| w[0].after < w[1].after));
    }
}
