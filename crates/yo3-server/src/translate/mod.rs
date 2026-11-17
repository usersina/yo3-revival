//! Translator between the rules engine (ocgcore messages, see `engine.rs`) and the client's duel stream
//! (engine commands, command maps; see "Duel connection" in `docs/protocol.md`).
//!
//! Players: the engine's team 0 goes first; the client's engine players are 0 (the client) and 1 (the CPU).
//! The server mirrors every zone of each side as client uids with their CARD_Prop rows, in the client's zone
//! numbering: field slots 0-4 (monsters), 5-9 (Spells/Traps), 10 (Field Spell), each holding one card at index 0;
//! then the lists hand 0xB, extra 0xC, deck 0xD, graveyard 0xE, banished 0xF. The client's deck is a face-down
//! bag, so a card leaving it is any deck card with that card's id.
use std::collections::{HashMap, HashSet};

use serde_json::{Value as Json, json};
use yo3_proto::{Message, Value};

use crate::card_options::CARD_OPTION_LABELS;
use crate::duel::{CLIENT, CPU, CardCommand, ClientRequest, TEXT_IDS, TEXT_SLOTS, command, command_data, text_message};
use crate::ocg::message::{AnnounceCard, Ask, Loc, SelectCard, SelectChain, SelectOption, SelectPlace, SelectPosition, SelectSum};
use crate::ocg::{Response, battle, hint, hintmsg, idle, loc, msg, pos, reason, resp};
use crate::wire::{Cmd, OPEN, Place, Target, cmd, dialog, entry, flag, list, menu, mode, move_place, result, table_place, text, zone};

mod answers;
mod cpu;
mod events;
mod moves;
mod prompts;
mod reach;
mod texts;

use cpu::*;
use moves::*;
use texts::*;
pub use cpu::{Repeats, bot_answer};
pub use moves::settle_positions;

/// The place in an engine message's field, as the place helpers read it; an unreadable one is no place (a default
/// location 0, which no helper maps). Callers whose message is typed pass its `Loc` instead.
fn loc_of(v: &Json) -> Loc {
    Loc::of(v).unwrap_or_default()
}

/// An engine message as the struct the server reads it as (`yo3_ocg::message`). The duel check fails a duel whose
/// messages don't fit; here one that doesn't is warned about and read as empty.
fn typed<T: serde::de::DeserializeOwned + serde::Serialize + Default>(m: &Json) -> T {
    crate::ocg::message::lenient(m).unwrap_or_else(|e| {
        tracing::warn!("engine message doesn't fit its struct: {e} in {m}");
        T::default()
    })
}

/// A command prompt's menu lists and its response type: the battle's, else the idle one's.
fn command_lists(ask: &Ask) -> (Lists, u64) {
    match ask {
        Ask::BattleCmd(_) => (&BATTLE_LISTS, resp::BATTLECMD),
        _ => (&IDLE_LISTS, resp::IDLECMD),
    }
}

/// The cards of a command prompt's list, by the name `IDLE_LISTS` and `BATTLE_LISTS` give it, in the engine's order,
/// each with its effect's description (0 for a summon, Set or attack).
fn command_list<'a>(ask: &'a Ask, list: &str) -> Vec<(&'a Loc, u64)> {
    let plain = |cards: &'a [Loc]| cards.iter().map(|l| (l, 0)).collect();
    let effects = |cards: &'a [crate::ocg::message::Effect]| cards.iter().map(|e| (&e.loc, e.description)).collect();
    match (ask, list) {
        (Ask::IdleCmd(p), "summons") => plain(&p.summons),
        (Ask::IdleCmd(p), "special_summons") => plain(&p.special_summons),
        (Ask::IdleCmd(p), "pos_changes") => plain(&p.pos_changes),
        (Ask::IdleCmd(p), "monster_sets") => plain(&p.monster_sets),
        (Ask::IdleCmd(p), "spell_sets") => plain(&p.spell_sets),
        (Ask::IdleCmd(p), "activates") => effects(&p.activates),
        (Ask::BattleCmd(p), "attacks") => p.attacks.iter().map(|a| (&a.loc, 0)).collect(),
        (Ask::BattleCmd(p), "chains") => effects(&p.chains),
        _ => vec![],
    }
}

/// A card list's cards, in the engine's order, and how many picks it takes: `min` to `max`, none when the player may
/// back out of it.
pub(super) struct CardList {
    pub(super) selects: Vec<Loc>,
    pub(super) min: u64,
    pub(super) max: u64,
    pub(super) cancelable: bool,
}

impl CardList {
    /// MSG_SELECT_CHAIN's cards, one pick; one the player may back out of unless the chain is forced.
    fn chain(m: &Json) -> CardList {
        let c: SelectChain = typed(m);
        CardList { selects: c.selects.into_iter().map(|e| e.loc).collect(), min: 1, max: 1, cancelable: !c.forced }
    }
}


/// What a window's chain question may hold, which decides whether the default Activation Message may answer it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainWindow {
    /// A phase's own window (the core's PhaseEvent): its optional triggers ("During the End Phase: you can...") come in
    /// the chain question with any other card that could be activated, counted in spe_count. It must show (#53).
    Phase,
    /// A step's window (the core's PointEvent or QuickEffect): its optional triggers are asked one by one before it
    /// (MSG_SELECT_EFFECTYN, which always shows), so the chain question holds cards the player may activate any time.
    Step,
}

/// Every window the core marks with MSG_HINT HINT_EVENT (processor.cpp), by its value. A value the core sends that isn't
/// here fails `the_cores_chain_windows_are_all_classified` and the duel check, until it is classified.
pub const CHAIN_WINDOWS: [(u64, &str, ChainWindow); 15] = [
    (20, "Draw Phase", ChainWindow::Phase),
    (21, "Standby Phase", ChainWindow::Phase),
    (25, "Battle Phase's end", ChainWindow::Phase),
    (26, "End Phase", ChainWindow::Phase),
    (28, "Battle Phase's start", ChainWindow::Phase),
    (23, "Main Phase's end", ChainWindow::Step),
    (24, "Battle Step", ChainWindow::Step),
    (27, "turn's start", ChainWindow::Step),
    (29, "battle step's end", ChainWindow::Step),
    (40, "Damage Step's start", ChainWindow::Step),
    (41, "before damage calculation", ChainWindow::Step),
    (42, "damage calculation", ChainWindow::Step),
    (43, "after damage calculation", ChainWindow::Step),
    (44, "Damage Step's end", ChainWindow::Step),
    (45, "damage calculation, again", ChainWindow::Step),
];

/// The kind of window a HINT_EVENT value marks; none for a value [`CHAIN_WINDOWS`] doesn't know.
pub fn chain_window(hint: u64) -> Option<ChainWindow> {
    CHAIN_WINDOWS.iter().find(|(h, _, _)| *h == hint).map(|&(_, _, w)| w)
}
const POPUP: u32 = 2;
const ALL_ZONES_USABLE: u32 = 0x3FF_03FF;
/// The marker types used: an equip, the yellow ∞ (effect 0x35); a lasting target, the pink reticle (0x3B). A guess:
/// the client names none of its eight markers.
const MARK_EQUIP: u32 = 2;
const MARK_TARGET: u32 = 8;
/// EDOPro's counter ids (strings.conf, less COUNTER_NEED_ENABLE) by the client's counter kinds of the same name:
/// Spell, Wedge, Bushido, Psychic, Shine, Crystal, Morph, Venom, Genex, A-, Worm, Black Feather, Hyper-Venom,
/// Spellstone ("Magical Stone", **guess**), Flower, Fog, Clock, D, Junk, Gate, Plant, Guard, Dragonic, Ocean,
/// Chronicle, Magic Reflector's, Maiden, Speed, Earthbound Immortal, Protection, Des ("Death", **guess**). The others
/// show as kind 38, "Counter".
pub(crate) const COUNTER_KINDS: [(u64, u32); 32] = [
    (0x1, 0),
    (0x1002, 42),
    (0x3, 6),
    (0x4, 15),
    (0x5, 8),
    (0x6, 4),
    (0x8, 11),
    (0x1009, 39),
    (0xa, 18),
    (0x100e, 37),
    (0xf, 10),
    (0x10, 21),
    (0x11, 3),
    (0x16, 24),
    (0x18, 12),
    (0x1019, 40),
    (0x1b, 2),
    (0x1c, 7),
    (0x1d, 17),
    (0x1e, 9),
    (0x20, 13),
    (0x1021, 43),
    (0x22, 19),
    (0x23, 20),
    (0x25, 5),
    (0x102a, 36),
    (0x90, 35),
    (0x1090, 35),
    (0x91, 14),
    (0x94, 16),
    (0x1096, 41),
    (0x1097, 22),
];
const ANY_COUNTER: u32 = 38;
/// LifeDamage `0x10`'s kinds (`param3 >> 8`), as in Konami's logs: 0 effect damage, 1 battle damage (after BattleRun),
/// 2 a cost, 4 a gain. DuelCockpit (`0x502EB0`) plays the recover sound for 4, the payment sound for 2, else damage's.
const DAMAGE_EFFECT: u32 = 0;
const DAMAGE_BATTLE: u32 = 1;
const DAMAGE_COST: u32 = 2;
const DAMAGE_GAIN: u32 = 4;
const COINS_A_TOSS: usize = 3;
/// EDOPro's SELECT_HEADS and SELECT_TAILS strings: a call's options (Time Wizard).
const CALL_HEADS: u64 = 60;
const CALL_TAILS: u64 = 61;
/// ChainSet's zone for a card with no place on the client's field: the centre (place table 0x81AE38).
const FIELD_CENTRE: u32 = 16;
const NO_LEVEL: u32 = 0xFFFF;
/// The uids of the entries a mixed list puts in the table (no card has one).
const STAGED_UID: u32 = 0xF0_0000;
/// A placement's prompt with auto card placing off (the client has none of its own).
const PLACE_MONSTER_WORDS: &str = "Select a Monster Card Zone to place the card in.";
const PLACE_SPELL_WORDS: &str = "Select a Spell & Trap Card Zone to place the card in.";
/// The command a zone's click sends (any bit works; the menu a right-click opens would name it): Summon for a monster
/// zone, Set for a Spell/Trap zone.
const ZONE_PICK_MONSTER: u32 = 4;
const ZONE_PICK_SPELL: u32 = 6;
/// The battle questions EDOPro's core asks by system string (processor.cpp: 31 attack directly instead of a monster,
/// 30 a replay), as the client's dialog texts 225 "@3Attack@0 your opponent directly?" and 223 "Replay happens.
/// Continue to @3attack@0?".
const BATTLE_QUESTIONS: [(u64, u32); 2] = [(31, 225), (30, 223)];
/// The pile views' titles (the duel prompts): 582/583 "Your/Opponent's Extra Deck", 586/587 "Your/Opponent's
/// Graveyard", 588/589 "Your/Opponent's Removed Cards", by zone, the opponent's one more.
const PILE_TITLES: [(u32, u32); 3] = [(zone::EXTRA, 582), (zone::GRAVE, 586), (zone::REMOVED, 588)];
/// Option labels of the duel prompt table for EDOPro's system strings, with the list's title: SELECT_HEADS/TAILS (60,
/// 61) "Heads", "Tails" under "Coin Toss Selection:"; DECLTYPE_MONSTER/SPELL/TRAP (70-72) "Monster Cards", "Spell
/// Cards", "Trap Cards" under "Select @3card@0 type.".
const OPTION_LABELS: [(u64, u32, u32); 5] =
    [(60, 400, 56), (61, 401, 56), (70, 440, 260), (71, 441, 260), (72, 442, 260)];
/// EDOPro's timings (TIMING_* in constant.lua) that only mark a phase or step: draw, standby and end phases, the
/// main phase's end, the battle's start, steps and end, a chain's end, a draw.
const QUIET_TIMINGS: u64 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x8000 | 0x10000 | 0x100_0000 | 0x400_0000;
const SELECT_MAX: usize = 4;
const NUMBER_FORMATS: [(u64, u32); 3] = [(1000, 302), (100, 303), (1, 283)];
/// The client's types (UI texts 1031-1052) and attributes (1061-1067) in its order, as EDOPro's RACE_* and
/// ATTRIBUTE_* bits.
const TYPES: [u64; 22] = [
    0x2000, 0x10, 0x8, 0x80, 0x40000, 0x100, 0x20, 0x20000, 0x10000, 0x800, 0x4000, 0x8000, 0x400, 0x40, 0x1, 0x200, 0x4,
    0x2, 0x1000, 0x80000, 0x100000, 0x200000,
];
const ATTRIBUTES: [u64; 7] = [0x10, 0x20, 0x2, 0x4, 0x1, 0x8, 0x40];
/// WIN's player for a draw.
const TEAM_DRAW: u64 = 2;
/// A prompt's card lists: (list name, card menu bit, response action).
type Lists = &'static [(&'static str, u32, u64)];

/// Card menu bits of the battle prompt's lists, with their actions.
const BATTLE_LISTS: [(&str, u32, u64); 2] = [("attacks", 0, battle::ATTACK), ("chains", 3, battle::CHAIN)];

/// Card menu bits (`CardCommand::command`) per idle-command list, with the list's action number. Set: bit 6 for
/// monsters and 7 for Spells/Traps (`duel_cardcmd.swf` draws item 6 as a card set sideways, 7 as one set upright).
const IDLE_LISTS: [(&str, u32, u64); 8] = [
    ("summons", 4, 0),
    ("special_summons", 2, 1),
    ("pos_changes", 5, 2),
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
#[derive(Clone, Default)]
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

/// What a duel does with each message the engine sends (every type `crates/yo3-engine/src/decode.rs` decodes; the
/// coverage test fails on a type missing here). Coin tosses and dice rolls once fell through `events` silently: a type
/// still `Undecided` warns in the server log the first time a duel drops it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handling {
    /// `events` turns it into duel commands (or keeps it for what follows).
    Shown,
    /// A prompt: `prompt` asks the client or answers it.
    Asked,
    /// Nothing to show, and why.
    Ignored(&'static str),
    /// Not yet decided whether the client should show something (each tracked in issue #52).
    Undecided,
}

/// Every engine message type, by EDOPro's name (ocgapi_constants.h), and what a duel does with it.
pub const ENGINE_MESSAGES: &[(u64, &str, Handling)] = &{
    use Handling::*;
    const SHOWN_AT_ING: Handling = Ignored("the summon was shown at its -ING message (60, 62, 64)");
    const NOT_HERE: Handling = Ignored("tag duels, puzzles and EDOPro's own client only: never sent in these duels");
    [
        (1, "MSG_RETRY", Ignored("the engine refused an answer: the session answers again, the client sees nothing")),
        (2, "MSG_HINT", Shown),
        (5, "MSG_WIN", Shown),
        (10, "MSG_SELECT_BATTLECMD", Asked),
        (11, "MSG_SELECT_IDLECMD", Asked),
        (12, "MSG_SELECT_EFFECTYN", Asked),
        (13, "MSG_SELECT_YESNO", Asked),
        (14, "MSG_SELECT_OPTION", Asked),
        (15, "MSG_SELECT_CARD", Asked),
        (16, "MSG_SELECT_CHAIN", Asked),
        (18, "MSG_SELECT_PLACE", Asked),
        (19, "MSG_SELECT_POSITION", Asked),
        (20, "MSG_SELECT_TRIBUTE", Asked),
        (21, "MSG_SORT_CHAIN", Asked),
        (22, "MSG_SELECT_COUNTER", Asked),
        (23, "MSG_SELECT_SUM", Asked),
        (24, "MSG_SELECT_DISFIELD", Asked),
        (25, "MSG_SORT_CARD", Asked),
        (26, "MSG_SELECT_UNSELECT_CARD", Asked),
        (30, "MSG_CONFIRM_DECKTOP", Shown),
        (31, "MSG_CONFIRM_CARDS", Shown),
        (32, "MSG_SHUFFLE_DECK", Undecided),
        (33, "MSG_SHUFFLE_HAND", Shown),
        (34, "MSG_REFRESH_DECK", Undecided),
        (35, "MSG_SWAP_GRAVE_DECK", Shown),
        (36, "MSG_SHUFFLE_SET_CARD", Shown),
        (37, "MSG_REVERSE_DECK", Shown),
        (38, "MSG_DECK_TOP", Shown),
        (39, "MSG_SHUFFLE_EXTRA", Undecided),
        (40, "MSG_NEW_TURN", Shown),
        (41, "MSG_NEW_PHASE", Shown),
        (42, "MSG_CONFIRM_EXTRATOP", Undecided),
        (50, "MSG_MOVE", Shown),
        (53, "MSG_POS_CHANGE", Shown),
        (54, "MSG_SET", Undecided),
        (55, "MSG_SWAP", Shown),
        (56, "MSG_FIELD_DISABLED", Shown),
        (60, "MSG_SUMMONING", Shown),
        (61, "MSG_SUMMONED", SHOWN_AT_ING),
        (62, "MSG_SPSUMMONING", Shown),
        (63, "MSG_SPSUMMONED", SHOWN_AT_ING),
        (64, "MSG_FLIPSUMMONING", Shown),
        (65, "MSG_FLIPSUMMONED", SHOWN_AT_ING),
        (70, "MSG_CHAINING", Shown),
        (71, "MSG_CHAINED", Ignored("the activation was shown at MSG_CHAINING")),
        (72, "MSG_CHAIN_SOLVING", Shown),
        (73, "MSG_CHAIN_SOLVED", Ignored("each link's end: MSG_CHAIN_END closes the chain on the client")),
        (74, "MSG_CHAIN_END", Shown),
        (75, "MSG_CHAIN_NEGATED", Undecided),
        (76, "MSG_CHAIN_DISABLED", Undecided),
        (80, "MSG_CARD_SELECTED", Undecided),
        (81, "MSG_RANDOM_SELECTED", Undecided),
        (83, "MSG_BECOME_TARGET", Undecided),
        (90, "MSG_DRAW", Shown),
        (91, "MSG_DAMAGE", Shown),
        (92, "MSG_RECOVER", Shown),
        (93, "MSG_EQUIP", Shown),
        (94, "MSG_LPUPDATE", Shown),
        (96, "MSG_CARD_TARGET", Shown),
        (97, "MSG_CANCEL_TARGET", Shown),
        (100, "MSG_PAY_LPCOST", Shown),
        (101, "MSG_ADD_COUNTER", Shown),
        (102, "MSG_REMOVE_COUNTER", Shown),
        (110, "MSG_ATTACK", Shown),
        (111, "MSG_BATTLE", Shown),
        (112, "MSG_ATTACK_DISABLED", Shown),
        (113, "MSG_DAMAGE_STEP_START", Undecided),
        (114, "MSG_DAMAGE_STEP_END", Shown),
        (120, "MSG_MISSED_EFFECT", Undecided),
        (121, "MSG_BE_CHAIN_TARGET", Undecided),
        (122, "MSG_CREATE_RELATION", Undecided),
        (123, "MSG_RELEASE_RELATION", Undecided),
        (130, "MSG_TOSS_COIN", Shown),
        (131, "MSG_TOSS_DICE", Shown),
        (132, "MSG_ROCK_PAPER_SCISSORS", Asked),
        (133, "MSG_HAND_RES", Undecided),
        (140, "MSG_ANNOUNCE_RACE", Asked),
        (141, "MSG_ANNOUNCE_ATTRIB", Asked),
        (142, "MSG_ANNOUNCE_CARD", Asked),
        (143, "MSG_ANNOUNCE_NUMBER", Asked),
        (160, "MSG_CARD_HINT", Undecided),
        (161, "MSG_TAG_SWAP", NOT_HERE),
        (162, "MSG_RELOAD_FIELD", NOT_HERE),
        (163, "MSG_AI_NAME", NOT_HERE),
        (164, "MSG_SHOW_HINT", Undecided),
        (165, "MSG_PLAYER_HINT", Undecided),
        (170, "MSG_MATCH_KILL", NOT_HERE),
        (180, "MSG_CUSTOM_MSG", NOT_HERE),
        (190, "MSG_REMOVE_CARDS", NOT_HERE),
    ]
};

/// What the client is answering besides the engine's prompt itself (`Translator::pending`): one thing at a time, so a
/// state can't outlive the window it belongs to.
#[derive(Clone, Debug, Default, PartialEq)]
enum Asking {
    /// The prompt's own window (a command map, a dialog, a card list), or nothing.
    #[default]
    Prompt,
    /// Which card of a pile, or which of a card's effects, a command is for, in the card list: the response's type and
    /// action, and the candidates' indices in the prompt's list.
    Choice { kind: u64, action: u64, candidates: Vec<usize> },
    /// A direct attack waiting for the client's Yes: its engine response.
    DirectAttack(Json),
    /// A pile's view: closing it sends the prompt again.
    PileView,
    /// A zone choice's clicks once its OK dialog closed: the zones picked so far.
    Zones(Vec<Json>),
    /// An attack's target, picked on the field (#51, see `target_on_field`).
    FieldTarget,
}

#[derive(Clone)]
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
    /// The side the client called for the coin toss that follows (heads true); none: heads wins.
    coin_call: Option<bool>,
    /// Engine messages this duel dropped that the table doesn't say are fine to drop: each warns once.
    unshown: HashSet<u64>,
    chain_list: bool,
    /// The next chain chance is a phase's own window (HINT_EVENT): it may hold an optional trigger.
    phase_window: bool,
    /// The chain's last link (MSG_CHAINING) until the chain ends, and each link's {player, zone} by its number - 1.
    chaining: Option<Json>,
    chain: Vec<(u32, u32)>,
    /// A battle shown on the client and not ended yet, and whether the duel is in a Battle Phase.
    in_battle: bool,
    battle_phase: bool,
    /// The last selection hint (HINTMSG_*), what it says when it is a card's own string, and the client's picks in the
    /// card list so far.
    hint: u64,
    hint_note: Option<(u64, String)>,
    picks: Vec<usize>,
    /// The engine's index of each card list position: the list shows the field's cards as they lie on it.
    list_order: Vec<usize>,
    /// The duel's result for the client (1 won, 2 lost, 3 draw), once it ended.
    result: Option<u32>,
    /// By player, whether that side's hidden cards (deck, hand, extra deck, face-down) go to the client without their
    /// rows: the opponent's against a player or an NPC (`duel::npc_translator`), both for a spectator, none for a
    /// translator that shows everything (`new`: duel log replays, the duel check's view of what the client isn't told).
    hidden: [bool; 2],
    /// The values each monster zone's record last went with (level, ATK, DEF, base ATK, base DEF), by player and zone.
    records: HashMap<(u32, u32), [u32; 5]>,
    /// A monster zone's Type and Attribute (the client's numbers) where an effect changed them from CARD_Prop's.
    kinds: HashMap<(u32, u32), (u32, u32)>,
    /// The last table entry sent for each player's Deck index 0, which the deck pile shows (0x4FE4D0): a revealed top
    /// card takes that place for a moment and the entry goes back.
    deck_tops: [Option<[u32; 4]>; 2],
    /// The client's attacker (player, zone) from its last attack command, until the battle or the next command
    /// prompt; with it, an attack target is picked on the field (`Asking::FieldTarget`, see `wire::cmd::BATTLE_SELECT`).
    attacker: Option<(u32, u32)>,
    /// What the client is answering besides the prompt's own window.
    asking: Asking,
    /// An attack taken back on the field: the client still marks its attacker (DuelBattle's state 3, effect 0x39)
    /// until a BattleEnd, which the next prompt starts with (the battle commands then draw the swords again).
    taken_back: bool,
    /// Lines for the player's chat window, for what a duel window can't say (`take_notes`).
    notes: Vec<String>,
    /// The next slot of the server's own dialog texts (`text`).
    next_text: u32,
    /// Both Decks turned over (Convulsion of Nature): each pile shows its top card's face.
    decks_reversed: bool,
    /// Each monster zone's position as last shown (face up, defense), by player and zone.
    positions: HashMap<(u32, u32), (bool, bool)>,
    /// The counts the client holds on each card, by uid and counter kind.
    counters: HashMap<(u32, u32), u32>,
    /// The client's token cards (CARD_Prop rows) by name, the tokens on the field (uids), and the next token's uid.
    token_rows: HashMap<String, Vec<u16>>,
    tokens: Vec<u32>,
    next_token: u32,
    /// The uid and row a token's position dialog gave the token about to appear (see `position_dialog`).
    reserved_token: Option<(u32, u16)>,
    /// The Tuners' Konami ids (a Synchro material's TuningRun says which levels are the Tuners').
    tuners: HashSet<u16>,
    /// A Tribute, Fusion or Synchro Summon whose materials are moving (see [`Summoning`]).
    summoning: Option<Summoning>,
    /// The player's DUEL OPTIONS auto card placing (`Translator::auto_place`): off, the player picks the zones.
    auto_place: bool,
    /// What the client's player did in the duel, for titles (`Translator::stats`), and each card's kind to tell Spells
    /// from Traps (`Translator::card_kinds`).
    stats: crate::titles::DuelStats,
    card_kinds: HashMap<u16, u32>,
    lp_change: i64,
    /// How many card names the CPU has declared: each try names the next card of the duel's (the core asks again
    /// when a name doesn't fit).
    declared: usize,
    /// Xyz materials: cards that left the field to sit under an Xyz Monster, which the client has no place for, with the
    /// field place each left (whose slot still holds its hidden object until another card lands there).
    materials: Vec<(Card, Place)>,
    /// Cards an effect revealed (MSG_CONFIRM_CARDS) that wait to reach a hand, by Konami id.
    revealing: Vec<u16>,
    /// The card list open is a mixed one (see `card_list`), and how many table entries it staged.
    mixed: bool,
    /// The card list open stands for SELECT_OPTION's options (the client has no labels for them).
    option_cards: bool,
    /// The text of the sum's card list, for showing it again after picks that don't make the sum.
    sum_text: Option<u32>,
    staged: Option<usize>,
    /// The engine's links between cards (uids): (card, linked card, marker type), from MSG_EQUIP and MSG_CARD_TARGET.
    links: Vec<(u32, u32, u32)>,
    /// The relations the client holds: (source place, target place, marker type), places as `player | zone << 8`.
    relations: Vec<(u32, u32, u32)>,
}

impl Translator {
    /// `decks[p]` is the main deck (CARD_Prop rows) of the client's engine player p; `first` goes first. Returns
    /// the translator and the card-table commands that put both decks in the client's card table (uids 1..).
    pub fn new(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        Translator::build(first, decks, cards, [false, false])
    }

    /// The same for a duel against another player, whose hidden cards the client gets with row 0 (**guess**: Online
    /// 2 used placeholders).
    pub fn against_player(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        Translator::build(first, decks, cards, [false, true])
    }

    /// Adds each side's extra deck (CARD_Prop rows) to the card table at the extra deck zone, after the main decks:
    /// the card table commands. Its cards are found by id when they leave (the engine keeps its own order).
    pub fn extra_decks(&mut self, extras: [&[u16]; 2]) -> Vec<Message> {
        let mut uid = self.sides.iter().flat_map(|s| s.zones.values()).flatten().map(|c| c.uid).max().unwrap_or(0);
        let mut out = vec![];
        for (p, extra) in extras.iter().enumerate() {
            for &row in extra.iter() {
                uid += 1;
                let zone = self.sides[p].zone(zone::EXTRA);
                zone.push(Card { uid, row });
                let shown = if self.hidden[p] { 0 } else { row as u32 };
                out.push(Cmd::Card { uid, open: false, row: shown, place: table_place(p as u32, zone::EXTRA, zone.len() - 1) }.message());
            }
        }
        with_card_ids(&self.cards, &mut out);
        out
    }

    /// The client's token cards (CARD_Prop rows, ids 3900-3999) by name: EDOPro's tokens have no Konami id, so the
    /// sidecar names them.
    pub fn tokens(&mut self, rows: HashMap<String, Vec<u16>>) {
        self.token_rows = rows;
    }

    /// The player's auto card placing (DUEL OPTIONS, `duelOption.auto_place`, sent as `autoSet` on entering the duel
    /// and when it changes): on (the default) the server places a summoned, Set or activated card centre out, as
    /// Konami's did; off, MSG_SELECT_PLACE is the player's, with the zone prompt of MSG_SELECT_DISFIELD.
    pub fn auto_place(&mut self, on: bool) {
        self.auto_place = on;
    }

    /// Each Konami id's card kind (CARD_Prop: 13 a Spell, 14 a Trap): a duel's stats count the client's activations.
    pub fn card_kinds(&mut self, kinds: HashMap<u16, u32>) {
        self.card_kinds = kinds;
    }

    /// What the client's player did in the duel so far: their summons, Spells and Traps, how low their LP went, and
    /// the win's reason (`crate::titles`).
    pub fn stats(&self) -> &crate::titles::DuelStats {
        &self.stats
    }

    /// The Tuners among the cards, by Konami id: a Synchro Summon's tuning tells the Tuners' levels from the others'.
    pub fn tuners(&mut self, cids: HashSet<u16>) {
        self.tuners = cids;
    }

    /// The same for a spectator watching from player 0's side, who sees neither player's hidden cards.
    pub fn spectating(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        Translator::build(first, decks, cards, [true, true])
    }

    fn build(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>, hidden: [bool; 2]) -> (Translator, Vec<Message>) {
        let mut sides = [Side::default(), Side::default()];
        // Every zone usable, as Konami's openings say (1006 0x3FF03FF): the client's starting mask isn't known, and a
        // later disabled zone is drawn against it.
        let mut out = vec![command(cmd::ZONES_USABLE, [ALL_ZONES_USABLE, 0, 0, 0])];
        let mut uid = 0;
        for (p, deck) in decks.iter().enumerate() {
            for (i, &row) in deck.iter().enumerate() {
                uid += 1;
                sides[p].zone(zone::DECK).push(Card { uid, row });
                let shown = if hidden[p] { 0 } else { row as u32 };
                out.push(Cmd::Card { uid, open: false, row: shown, place: table_place(p as u32, zone::DECK, i) }.message());
            }
        }
        let t = Translator {
            first,
            sides,
            cards,
            turn_player: first,
            turns: 0,
            pending: None,
            coin_call: None,
            unshown: HashSet::new(),
            chain_list: false,
            phase_window: false,
            chaining: None,
            in_battle: false,
            battle_phase: false,
            hint: 0,
            hint_note: None,
            next_text: 0,
            picks: vec![],
            list_order: vec![],
            result: None,
            hidden,
            records: HashMap::new(),
            kinds: HashMap::new(),
            deck_tops: [None; 2],
            attacker: None,
            asking: Asking::Prompt,
            taken_back: false,
            notes: vec![],
            decks_reversed: false,
            positions: HashMap::new(),
            token_rows: HashMap::new(),
            tokens: vec![],
            next_token: FIRST_TOKEN_UID,
            reserved_token: None,
            tuners: HashSet::new(),
            summoning: None,
            auto_place: true,
            stats: Default::default(),
            card_kinds: HashMap::new(),
            lp_change: 0,
            counters: HashMap::new(),
            declared: 0,
            revealing: vec![],
            materials: vec![],
            mixed: false,
            option_cards: false,
            sum_text: None,
            chain: vec![],
            staged: None,
            links: vec![],
            relations: vec![],
        };
        with_card_ids(&t.cards, &mut out);
        let mut t = t;
        t.note_deck_tops(&out);
        (t, out)
    }

    /// The lines for the player's chat window since the last call (an option list's texts): what a duel window can't
    /// say, as the client takes only text ids there.
    /// A text of the server's own for a dialog's message or label: the message carrying it (`duel::text_message`) goes
    /// into `out`, ahead of the dialog, and the duel prompt id that names it is returned. The connection DLL keeps the
    /// texts by slot and answers the client's lookups of those ids (`tools/mod/dialog-text.h`); slots are used in turn,
    /// so a text stays until 256 more have been sent.
    fn text(&mut self, out: &mut Vec<Message>, text: &str) -> u32 {
        let slot = self.next_text % TEXT_SLOTS;
        self.next_text = self.next_text.wrapping_add(1);
        out.push(text_message(slot, text));
        TEXT_IDS + slot
    }

    /// A card choice's message after the selection hint `hint`: the card's own string when the hint is one (code << 20
    /// | n), the client's text where one says the same, else the server's own words for EDOPro's hint (`HINT_WORDS`).
    fn hint_message(&mut self, out: &mut Vec<Message>, hint: u64) -> Option<u32> {
        if let Some((_, note)) = self.hint_note.take().filter(|(h, _)| *h == hint && hint != 0) {
            return Some(self.text(out, &note));
        }
        if let Some(id) = hint_text(hint) {
            return Some(id);
        }
        let words = HINT_WORDS.iter().find(|(h, _)| *h == hint).map(|(_, w)| *w)?;
        Some(self.text(out, words))
    }

    pub fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notes)
    }

    /// The duel's result for the client (1 won, 2 lost, 3 draw), once the engine ended the duel.
    pub fn result(&self) -> Option<u32> {
        self.result
    }

    /// The client's engine player of an engine team.
    fn player(&self, team: &Json) -> u32 {
        if team.as_u64() == Some(0) { self.first } else { 1 - self.first }
    }

    /// The client's player of an engine team.
    fn side(&self, team: u64) -> u32 {
        if team == 0 { self.first } else { 1 - self.first }
    }

    /// Whether an engine prompt asks this client (not the other side).
    pub fn prompted(&self, m: &Json) -> bool {
        self.player(&m["player"]) == CLIENT
    }

    /// The row the client gets for a card at a place: none for the opponent's hidden cards when hiding.
    fn shown_row(&self, place: &Place, row: u16) -> u32 {
        let hidden = match place.zone {
            zone::HAND | zone::DECK | zone::EXTRA => true,
            zone => zone <= zone::FIELD_SPELL && !place.face_up,
        };
        if self.hidden[place.player as usize] && hidden { 0 } else { row as u32 }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picks_of_a_card_list_are_capped() {
        let (mut t, _) = Translator::new(0, [&[], &[]], HashMap::new());
        for i in 0..1_000_000 {
            t.pick(i);
        }
        assert_eq!(t.picks.len(), MAX_PICKS);
    }

    /// An idle prompt of the client's: the hand card at sequence 0 offers `effects` activations.
    fn idle_with_activations(effects: u64) -> Json {
        let activates: Vec<Json> = (0..effects)
            .map(|e| json!({ "controller": 0, "location": loc::HAND, "sequence": 0, "cid": 4007, "description": e }))
            .collect();
        json!({ "type": msg::SELECT_IDLECMD, "player": 0, "summons": [], "special_summons": [], "pos_changes": [],
                "monster_sets": [], "spell_sets": [], "activates": activates, "to_bp": false, "to_ep": true, "shuffle": false })
    }

    #[test]
    fn a_card_offering_two_effects_asks_which_and_both_are_reachable() {
        let (mut t, _) = Translator::new(CLIENT, [&[], &[]], HashMap::new());
        assert!(matches!(t.prompt(&idle_with_activations(2)), Prompt::Client(_)));
        // The round trip (the duel check's): before the card list, only the first effect was reachable.
        assert_eq!(t.unreachable_answers(), Vec::<String>::new());
        let activate = CardCommand { command: 3, player: CLIENT, zone: zone::HAND, index: 0 };
        let Some(Prompt::Client(list)) = t.command(activate) else { panic!("a card list") };
        let begin = list.iter().find(|m| m.get("id").and_then(yo3_proto::Value::as_u32) == Some(cmd::LIST_BEGIN)).expect("a list");
        assert_eq!(begin.get("param1").and_then(yo3_proto::Value::as_u32), Some(2));
        t.pick(1);
        assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": resp::IDLECMD, "action": 5, "index": 1 }))));
    }

    #[test]
    fn the_round_trip_names_an_option_no_menu_reaches() {
        let (mut t, _) = Translator::new(CLIENT, [&[], &[]], HashMap::new());
        let mut idle = idle_with_activations(1);
        // A card in the Deck: the client has no menu there.
        idle["activates"][0]["location"] = json!(1);
        assert!(matches!(t.prompt(&idle), Prompt::Client(_)));
        let missing = t.unreachable_answers();
        assert!(missing.len() == 1 && missing[0].starts_with("activates[0]"), "{missing:?}");
    }
}
