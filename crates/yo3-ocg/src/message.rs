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

/// A drawn card (MSG_DRAW's `drawn`), or one card of a list given by code.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Coded {
    #[serde(default)]
    pub code: u64,
    #[serde(default)]
    pub cid: Option<u64>,
    #[serde(default)]
    pub position: Option<u64>,
}

/// MSG_DRAW.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Draw {
    pub player: u64,
    pub drawn: Vec<Coded>,
}

/// A message about one player: MSG_NEW_TURN, MSG_SHUFFLE_DECK, MSG_ROCK_PAPER_SCISSORS.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct OfPlayer {
    pub player: u64,
}

/// MSG_NEW_PHASE.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct NewPhase {
    pub phase: u64,
}

/// MSG_MOVE: card `card` from one place to another, with the session's `reason` (read back from the card's new place)
/// and the card's Konami id (or, without one, its name).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Move {
    pub card: u64,
    #[serde(default)]
    pub cid: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    pub from: Loc,
    pub to: Loc,
    #[serde(default)]
    pub reason: u64,
}

/// MSG_FIELD_DISABLED.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FieldDisabled {
    pub field_mask: u64,
}

/// MSG_HINT: a hint of `hint_type` for `player`, with the card string's text where it names one.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Hint {
    pub hint_type: u64,
    pub player: u64,
    pub hint: u64,
    #[serde(default)]
    pub hint_text: Option<String>,
}

/// MSG_POS_CHANGE: a card's position, before and after.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PosChange {
    #[serde(flatten)]
    pub loc: Loc,
    pub prev_position: u64,
}

/// A card at its place: MSG_SUMMONING, MSG_SPSUMMONING, MSG_FLIPSUMMONING, MSG_SET (code, place, Konami id).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AtPlace {
    #[serde(flatten)]
    pub loc: Loc,
}

/// MSG_CHAINING: a chain link's card at its place, its description and the chain's size.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Chaining {
    #[serde(flatten)]
    pub loc: Loc,
    #[serde(default)]
    pub description: u64,
    pub chain_size: u64,
}

/// A chain message with only its size: MSG_CHAIN_SOLVING and the others of its family.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ChainSize {
    pub chain_size: u64,
}

/// MSG_ATTACK: the attacker and its target (none for a direct attack).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Attack {
    pub card: Loc,
    #[serde(default)]
    pub target: Option<Loc>,
}

/// One side of MSG_BATTLE: the card at its place and its figures.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Battler {
    #[serde(flatten)]
    pub loc: Loc,
    pub attack: u64,
    pub defense: u64,
    pub destroyed: bool,
}

/// MSG_BATTLE.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Battle {
    pub card: Battler,
    pub target: Battler,
}

/// Cards a player is shown: MSG_CONFIRM_CARDS, MSG_CONFIRM_DECKTOP, MSG_CONFIRM_EXTRATOP.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Confirm {
    pub player: u64,
    pub cards: Vec<Loc>,
}

/// MSG_BECOME_TARGET: the cards an effect targets as it is activated.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Targets {
    pub cards: Vec<Loc>,
}

/// MSG_DECK_TOP: the deck's top card turned (`count` from the top).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DeckTop {
    pub player: u64,
    pub count: u64,
    pub code: u64,
    #[serde(default)]
    pub cid: Option<u64>,
    pub position: u64,
}

/// MSG_SHUFFLE_HAND and MSG_SHUFFLE_EXTRA: the cards by code, and their Konami ids.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Shuffle {
    pub player: u64,
    pub cards: Vec<u64>,
    #[serde(default)]
    pub cids: Vec<u64>,
}

/// MSG_SWAP: two cards trade places.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Swap {
    pub card1: Loc,
    pub card2: Loc,
}

/// One pair of MSG_SHUFFLE_SET_CARD's `cards`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FromTo {
    pub from: Loc,
    pub to: Loc,
}

/// MSG_SHUFFLE_SET_CARD, with the session's `places`: each shuffled zone and the card in it now.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ShuffleSet {
    pub location: u64,
    pub cards: Vec<FromTo>,
    #[serde(default)]
    pub places: Vec<Loc>,
}

/// MSG_SWAP_GRAVE_DECK, with the session's `grave` and `extra` after the swap, in sequence order.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SwapGraveDeck {
    pub player: u64,
    #[serde(default)]
    pub grave: Vec<Coded>,
    #[serde(default)]
    pub extra: Vec<Coded>,
}

/// MSG_WIN: the winning team (2 a draw) and the reason.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Win {
    pub player: u64,
    pub reason: u64,
}

/// MSG_DAMAGE, MSG_RECOVER and MSG_PAY_LPCOST.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LpChange {
    pub player: u64,
    pub amount: u64,
}

/// MSG_LPUPDATE.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LpUpdate {
    pub player: u64,
    pub lp: u64,
}

/// MSG_ADD_COUNTER and MSG_REMOVE_COUNTER: `count` counters of `counter_type` on the card at a place.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Counter {
    pub counter_type: u64,
    pub controller: u64,
    pub location: u64,
    pub sequence: u64,
    pub count: u64,
}

/// MSG_EQUIP, MSG_CARD_TARGET and MSG_CANCEL_TARGET: a card and the card it is linked to.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Link {
    pub card: Loc,
    pub target: Loc,
}

/// MSG_TOSS_COIN: heads (true) or tails per coin.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TossCoin {
    pub player: u64,
    pub results: Vec<bool>,
}

/// MSG_TOSS_DICE: each die's face.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TossDice {
    pub player: u64,
    pub results: Vec<u64>,
}

/// A monster's stats in the session's field report (`{type: "field", monsters}`): what effects changed without a
/// message of their own.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Monster {
    pub controller: u64,
    pub sequence: u64,
    #[serde(default)]
    pub position: Option<u64>,
    #[serde(default)]
    pub level: Option<u64>,
    #[serde(default)]
    pub attack: Option<i64>,
    #[serde(default)]
    pub defense: Option<i64>,
    #[serde(default)]
    pub base_attack: Option<i64>,
    #[serde(default)]
    pub base_defense: Option<i64>,
    #[serde(default)]
    pub attribute: Option<u64>,
    #[serde(default)]
    pub race: Option<u64>,
}

/// The session's field report.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Field {
    pub monsters: Vec<Monster>,
}

/// An event: an engine message the server shows (or keeps track of), not a prompt.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Draw(Draw),
    NewTurn(OfPlayer),
    NewPhase(NewPhase),
    Move(Move),
    FieldDisabled(FieldDisabled),
    Hint(Hint),
    PosChange(PosChange),
    Summoning(AtPlace),
    SpSummoning(AtPlace),
    FlipSummoning(AtPlace),
    Set(AtPlace),
    Chaining(Chaining),
    ChainSolving(ChainSize),
    ChainEnd,
    Attack(Attack),
    Battle(Battle),
    AttackDisabled,
    DamageStepEnd,
    ConfirmCards(Confirm),
    ConfirmDecktop(Confirm),
    ReverseDeck,
    DeckTop(DeckTop),
    ShuffleHand(Shuffle),
    ShuffleDeck(OfPlayer),
    BecomeTarget(Targets),
    ChainSolved(ChainSize),
    Swap(Swap),
    ShuffleSetCard(ShuffleSet),
    SwapGraveDeck(SwapGraveDeck),
    Win(Win),
    Damage(LpChange),
    Recover(LpChange),
    PayLpCost(LpChange),
    AddCounter(Counter),
    RemoveCounter(Counter),
    LpUpdate(LpUpdate),
    Equip(Link),
    CardTarget(Link),
    CancelTarget(Link),
    TossCoin(TossCoin),
    TossDice(TossDice),
    /// The session's field report.
    Field(Field),
}

impl Event {
    /// The event in an engine message, every field present (see [`strict`]); `Ok(None)` for a prompt or a type the
    /// server doesn't read.
    pub fn parse(m: &Json) -> Result<Option<Event>, String> {
        Event::read(m, true)
    }

    /// The event in an engine message as the server reads it (see [`lenient`]).
    pub fn lenient(m: &Json) -> Result<Option<Event>, String> {
        Event::read(m, false)
    }

    fn read(m: &Json, strictly: bool) -> Result<Option<Event>, String> {
        fn get<T: DeserializeOwned + Serialize + Default>(m: &Json, strictly: bool) -> Result<T, String> {
            if strictly { strict(m) } else { lenient(m) }
        }
        if m["type"] == "field" {
            return Ok(Some(Event::Field(get(m, strictly)?)));
        }
        Ok(Some(match m["type"].as_u64().unwrap_or(u64::MAX) {
            msg::DRAW => Event::Draw(get(m, strictly)?),
            msg::NEW_TURN => Event::NewTurn(get(m, strictly)?),
            msg::NEW_PHASE => Event::NewPhase(get(m, strictly)?),
            msg::MOVE => Event::Move(get(m, strictly)?),
            msg::FIELD_DISABLED => Event::FieldDisabled(get(m, strictly)?),
            msg::HINT => Event::Hint(get(m, strictly)?),
            msg::POS_CHANGE => Event::PosChange(get(m, strictly)?),
            msg::SUMMONING => Event::Summoning(get(m, strictly)?),
            msg::SPSUMMONING => Event::SpSummoning(get(m, strictly)?),
            msg::FLIPSUMMONING => Event::FlipSummoning(get(m, strictly)?),
            msg::SET => Event::Set(get(m, strictly)?),
            msg::CHAINING => Event::Chaining(get(m, strictly)?),
            msg::CHAIN_SOLVING => Event::ChainSolving(get(m, strictly)?),
            msg::CHAIN_END => Event::ChainEnd,
            msg::ATTACK => Event::Attack(get(m, strictly)?),
            msg::BATTLE => Event::Battle(get(m, strictly)?),
            msg::ATTACK_DISABLED => Event::AttackDisabled,
            msg::DAMAGE_STEP_END => Event::DamageStepEnd,
            msg::CONFIRM_CARDS => Event::ConfirmCards(get(m, strictly)?),
            msg::CONFIRM_DECKTOP => Event::ConfirmDecktop(get(m, strictly)?),
            msg::REVERSE_DECK => Event::ReverseDeck,
            msg::DECK_TOP => Event::DeckTop(get(m, strictly)?),
            msg::SHUFFLE_HAND => Event::ShuffleHand(get(m, strictly)?),
            msg::SHUFFLE_DECK => Event::ShuffleDeck(get(m, strictly)?),
            msg::BECOME_TARGET => Event::BecomeTarget(get(m, strictly)?),
            msg::CHAIN_SOLVED => Event::ChainSolved(get(m, strictly)?),
            msg::SWAP => Event::Swap(get(m, strictly)?),
            msg::SHUFFLE_SET_CARD => Event::ShuffleSetCard(get(m, strictly)?),
            msg::SWAP_GRAVE_DECK => Event::SwapGraveDeck(get(m, strictly)?),
            msg::WIN => Event::Win(get(m, strictly)?),
            msg::DAMAGE => Event::Damage(get(m, strictly)?),
            msg::RECOVER => Event::Recover(get(m, strictly)?),
            msg::PAY_LPCOST => Event::PayLpCost(get(m, strictly)?),
            msg::ADD_COUNTER => Event::AddCounter(get(m, strictly)?),
            msg::REMOVE_COUNTER => Event::RemoveCounter(get(m, strictly)?),
            msg::LPUPDATE => Event::LpUpdate(get(m, strictly)?),
            msg::EQUIP => Event::Equip(get(m, strictly)?),
            msg::CARD_TARGET => Event::CardTarget(get(m, strictly)?),
            msg::CANCEL_TARGET => Event::CancelTarget(get(m, strictly)?),
            msg::TOSS_COIN => Event::TossCoin(get(m, strictly)?),
            msg::TOSS_DICE => Event::TossDice(get(m, strictly)?),
            _ => return Ok(None),
        }))
    }
}

/// Whether an engine message fits the struct the server reads it as: an error naming the field and the message if not.
pub fn parse(m: &Json) -> Result<(), String> {
    Ask::parse(m)?;
    Event::parse(m).map(|_| ())
}
