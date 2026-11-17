//! The engine's messages as the server reads them: one struct per message type, with the fields `yo3-engine` sends
//! (its `decode.rs`, field for field as ocgcore-wasm 0.1.6, plus what its session adds: a card's Konami id `cid` next
//! to every code, `description_cid`, `description_text`, `hint_text`, `option_cids`, `option_texts`, an announce's
//! `fits`). A field the server reads is a field here, so a misspelled one is a compile error, and a message that
//! doesn't fit its struct is an error the duel check reports ([`parse`]), not a silent null.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value as Json;

use crate::msg;

/// A card's place `{controller, location, sequence, position}` with its code and Konami id where the message has them.
/// An Xyz material's place is its monster's, with the material's index as `overlay_sequence`. A field the message
/// lacks is `None` (a banished card with no position reads face up, a card with no Konami id matches none).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Loc {
    pub controller: u64,
    pub location: u64,
    pub sequence: u64,
    #[serde(default)]
    pub position: Option<u64>,
    #[serde(default)]
    pub overlay_sequence: Option<u64>,
    #[serde(default)]
    pub code: Option<u64>,
    #[serde(default)]
    pub cid: Option<u64>,
}

impl Loc {
    /// The place in a message's field (a move's `from`, an attack's `target`), if it is one.
    pub fn of(v: &Json) -> Option<Loc> {
        Loc::deserialize(v).ok()
    }

    /// Whether it is an Xyz material's: its monster's location with an `overlay_sequence` (or the overlay bit).
    pub fn is_material(&self) -> bool {
        self.overlay_sequence.is_some() || self.location & crate::loc::OVERLAY != 0
    }
}

/// An effect a card offers (idle and battle commands' activations, a chain's links): its place and description.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Effect {
    #[serde(flatten)]
    pub loc: Loc,
    /// 0 (no description) where the message has none.
    #[serde(default)]
    pub description: u64,
    #[serde(default)]
    pub description_cid: Option<u64>,
    #[serde(default)]
    pub description_text: Option<String>,
    #[serde(default)]
    pub client_mode: u64,
}

/// A monster that may attack.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Attacker {
    #[serde(flatten)]
    pub loc: Loc,
    #[serde(default)]
    pub can_direct: bool,
}

/// A card that counts toward a sum: its place and amount (a level, or two it may count as, low and high 16 bits).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SumCard {
    #[serde(flatten)]
    pub loc: Loc,
    pub amount: u64,
}

/// A card holding counters, and how many.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CounterCard {
    #[serde(flatten)]
    pub loc: Loc,
    pub count: u64,
}

/// MSG_SELECT_BATTLECMD.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectBattleCmd {
    pub player: u64,
    pub chains: Vec<Effect>,
    pub attacks: Vec<Attacker>,
    pub to_m2: bool,
    pub to_ep: bool,
}

/// MSG_SELECT_IDLECMD.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectIdleCmd {
    pub player: u64,
    pub summons: Vec<Loc>,
    pub special_summons: Vec<Loc>,
    pub pos_changes: Vec<Loc>,
    pub monster_sets: Vec<Loc>,
    pub spell_sets: Vec<Loc>,
    pub activates: Vec<Effect>,
    pub to_bp: bool,
    pub to_ep: bool,
    #[serde(default)]
    pub shuffle: bool,
}

/// MSG_SELECT_EFFECTYN: a card's optional effect, yes or no.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectEffectYn {
    pub player: u64,
    #[serde(flatten)]
    pub effect: Effect,
}

/// MSG_SELECT_YESNO: a question by its description (a card's string, or a system string).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectYesNo {
    pub player: u64,
    pub description: u64,
    #[serde(default)]
    pub description_cid: Option<u64>,
    #[serde(default)]
    pub description_text: Option<String>,
}

/// MSG_SELECT_OPTION and MSG_ANNOUNCE_NUMBER: options (descriptions, or numbers), with each description's card and
/// text where the session found them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectOption {
    pub player: u64,
    pub options: Vec<u64>,
    #[serde(default)]
    pub option_cids: Vec<u64>,
    #[serde(default)]
    pub option_texts: Vec<Option<String>>,
}

/// MSG_SELECT_CARD and MSG_SELECT_TRIBUTE: `min` to `max` of `selects`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectCard {
    pub player: u64,
    pub can_cancel: bool,
    pub min: u64,
    pub max: u64,
    pub selects: Vec<Loc>,
}

/// MSG_SELECT_CHAIN: a chance to chain, `spe_count` of the links mattering (optional triggers among them).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectChain {
    pub player: u64,
    pub spe_count: u64,
    pub forced: bool,
    pub hint_timing: u64,
    pub hint_timing_other: u64,
    pub selects: Vec<Effect>,
}

/// MSG_SELECT_PLACE and MSG_SELECT_DISFIELD: `count` zones not in `field_mask`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectPlace {
    pub player: u64,
    pub count: u64,
    pub field_mask: u64,
}

/// MSG_SELECT_POSITION.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectPosition {
    pub player: u64,
    pub code: u64,
    #[serde(default)]
    pub cid: Option<u64>,
    /// A card's name where it has no Konami id (a token).
    #[serde(default)]
    pub name: Option<String>,
    pub positions: u64,
}

/// MSG_SORT_CARD and MSG_SORT_CHAIN.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SortCards {
    pub player: u64,
    pub cards: Vec<Loc>,
}

/// MSG_SELECT_COUNTER: `count` counters of `counter_type` from `cards`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectCounter {
    pub player: u64,
    pub counter_type: u64,
    pub count: u64,
    pub cards: Vec<CounterCard>,
}

/// MSG_SELECT_SUM: cards adding up to `amount` (exactly, or with `select_max` at least it with none to spare).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectSum {
    pub player: u64,
    pub select_max: u64,
    pub amount: u64,
    pub min: u64,
    pub max: u64,
    pub selects_must: Vec<SumCard>,
    pub selects: Vec<SumCard>,
}

/// MSG_SELECT_UNSELECT_CARD: one card at a time, until the engine lets the player finish.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SelectUnselectCard {
    pub player: u64,
    pub can_finish: bool,
    pub can_cancel: bool,
    pub min: u64,
    pub max: u64,
    pub select_cards: Vec<Loc>,
    pub unselect_cards: Vec<Loc>,
}

/// MSG_ANNOUNCE_RACE and MSG_ANNOUNCE_ATTRIB: `count` of the `available` bits.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Announce {
    pub player: u64,
    pub count: u64,
    pub available: u64,
}

/// MSG_ANNOUNCE_CARD: a card name fitting `opcodes`; the session lists the Konami ids that fit.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AnnounceCard {
    pub player: u64,
    pub opcodes: Vec<u64>,
    #[serde(default)]
    pub fits: Vec<u64>,
}

/// MSG_ROCK_PAPER_SCISSORS.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RockPaperScissors {
    pub player: u64,
}

/// A prompt: the engine waits for this answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    BattleCmd(SelectBattleCmd),
    IdleCmd(SelectIdleCmd),
    EffectYn(SelectEffectYn),
    YesNo(SelectYesNo),
    Option(SelectOption),
    Card(SelectCard),
    Tribute(SelectCard),
    Chain(SelectChain),
    Place(SelectPlace),
    Disfield(SelectPlace),
    Position(SelectPosition),
    SortChain(SortCards),
    SortCard(SortCards),
    Counter(SelectCounter),
    Sum(SelectSum),
    UnselectCard(SelectUnselectCard),
    Race(Announce),
    Attrib(Announce),
    CardName(AnnounceCard),
    Number(SelectOption),
    RockPaperScissors(RockPaperScissors),
}

/// A message as `T`, every field it has: the duel check's read, which fails on a field the engine didn't send.
pub fn strict<T: DeserializeOwned>(m: &Json) -> Result<T, String> {
    T::deserialize(m).map_err(|e| format!("{e} in {m}"))
}

/// A message as `T`, a top-level field it lacks taking its default (as the server always read a missing field): the
/// server's read. Fails only on a field of the wrong kind.
pub fn lenient<T: DeserializeOwned + Serialize + Default>(m: &Json) -> Result<T, String> {
    let mut base = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
    if let (Json::Object(base), Json::Object(fields)) = (&mut base, m) {
        for (k, v) in fields {
            base.insert(k.clone(), v.clone());
        }
    }
    T::deserialize(&base).map_err(|e| format!("{e} in {m}"))
}

impl Ask {
    /// The prompt in an engine message, every field present (see [`strict`]); `Ok(None)` for a message that is no
    /// prompt.
    pub fn parse(m: &Json) -> Result<Option<Ask>, String> {
        Ask::read(m, true)
    }

    /// The prompt in an engine message as the server reads it (see [`lenient`]).
    pub fn lenient(m: &Json) -> Result<Option<Ask>, String> {
        Ask::read(m, false)
    }

    fn read(m: &Json, strictly: bool) -> Result<Option<Ask>, String> {
        fn get<T: DeserializeOwned + Serialize + Default>(m: &Json, strictly: bool) -> Result<T, String> {
            if strictly { strict(m) } else { lenient(m) }
        }
        Ok(Some(match m["type"].as_u64().unwrap_or(u64::MAX) {
            msg::SELECT_BATTLECMD => Ask::BattleCmd(get(m, strictly)?),
            msg::SELECT_IDLECMD => Ask::IdleCmd(get(m, strictly)?),
            msg::SELECT_EFFECTYN => Ask::EffectYn(get(m, strictly)?),
            msg::SELECT_YESNO => Ask::YesNo(get(m, strictly)?),
            msg::SELECT_OPTION => Ask::Option(get(m, strictly)?),
            msg::SELECT_CARD => Ask::Card(get(m, strictly)?),
            msg::SELECT_TRIBUTE => Ask::Tribute(get(m, strictly)?),
            msg::SELECT_CHAIN => Ask::Chain(get(m, strictly)?),
            msg::SELECT_PLACE => Ask::Place(get(m, strictly)?),
            msg::SELECT_DISFIELD => Ask::Disfield(get(m, strictly)?),
            msg::SELECT_POSITION => Ask::Position(get(m, strictly)?),
            msg::SORT_CHAIN => Ask::SortChain(get(m, strictly)?),
            msg::SORT_CARD => Ask::SortCard(get(m, strictly)?),
            msg::SELECT_COUNTER => Ask::Counter(get(m, strictly)?),
            msg::SELECT_SUM => Ask::Sum(get(m, strictly)?),
            msg::SELECT_UNSELECT_CARD => Ask::UnselectCard(get(m, strictly)?),
            msg::ANNOUNCE_RACE => Ask::Race(get(m, strictly)?),
            msg::ANNOUNCE_ATTRIB => Ask::Attrib(get(m, strictly)?),
            msg::ANNOUNCE_CARD => Ask::CardName(get(m, strictly)?),
            msg::ANNOUNCE_NUMBER => Ask::Number(get(m, strictly)?),
            msg::ROCK_PAPER_SCISSORS => Ask::RockPaperScissors(get(m, strictly)?),
            _ => return Ok(None),
        }))
    }

    /// The engine player asked.
    pub fn player(&self) -> u64 {
        match self {
            Ask::BattleCmd(a) => a.player,
            Ask::IdleCmd(a) => a.player,
            Ask::EffectYn(a) => a.player,
            Ask::YesNo(a) => a.player,
            Ask::Option(a) | Ask::Number(a) => a.player,
            Ask::Card(a) | Ask::Tribute(a) => a.player,
            Ask::Chain(a) => a.player,
            Ask::Place(a) | Ask::Disfield(a) => a.player,
            Ask::Position(a) => a.player,
            Ask::SortChain(a) | Ask::SortCard(a) => a.player,
            Ask::Counter(a) => a.player,
            Ask::Sum(a) => a.player,
            Ask::UnselectCard(a) => a.player,
            Ask::Race(a) | Ask::Attrib(a) => a.player,
            Ask::CardName(a) => a.player,
            Ask::RockPaperScissors(a) => a.player,
        }
    }
}

/// Whether an engine message fits the struct the server reads it as: an error naming the field and the message if not.
pub fn parse(m: &Json) -> Result<(), String> {
    Ask::parse(m).map(|_| ())
}
