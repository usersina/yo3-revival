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
use crate::duel::{CLIENT, CPU, CardCommand, TEXT_IDS, TEXT_SLOTS, command, command_data, text_message};
use crate::ocg::{battle, hint, hintmsg, idle, loc, msg, pos, reason, resp};
use crate::wire::{Cmd, OPEN, Place, Target, cmd, dialog, entry, flag, list, menu, mode, move_place, result, table_place, text, zone};


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
    /// A MSG_SELECT_DISFIELD's zones clicked so far, once its OK dialog closed (None while it is open).
    zone_picks: Option<Vec<Json>>,
    /// The client's attacker (player, zone) from its last attack command, until the battle or the next command
    /// prompt; with it, an attack target is picked on the field (`field_target`, see [`VIEW_BATTLE_SELECT`]).
    attacker: Option<(u32, u32)>,
    field_target: bool,
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
    /// A direct attack waiting for the client's Yes: its engine response.
    direct: Option<Json>,
    /// A pile's view is open: closing it sends the pending prompt again.
    viewing: bool,
    /// Which card of a pile a command is for, asked in the card list: the response's type and action, and the
    /// candidates' indices in the prompt's list.
    choosing: Option<(u64, u64, Vec<usize>)>,
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
            zone_picks: None,
            attacker: None,
            field_target: false,
            taken_back: false,
            notes: vec![],
            decks_reversed: false,
            positions: HashMap::new(),
            direct: None,
            viewing: false,
            choosing: None,
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

    /// The client commands for the engine's events (prompts excluded).
    pub fn events(&mut self, messages: &[Json]) -> Vec<Message> {
        let staged = self.staged.take();
        let mut out = self.unstage_n(staged);
        for (i, m) in messages.iter().enumerate() {
            if m["type"] == "field" {
                out.extend(self.field(m));
                continue;
            }
            match m["type"].as_u64() {
                Some(msg::DRAW) => {
                    let p = self.player(&m["player"]);
                    for drawn in m["drawn"].as_array().into_iter().flatten() {
                        out.extend(self.draw(p, drawn["cid"].as_u64().unwrap_or(0) as u16));
                    }
                }
                Some(msg::NEW_TURN) => {
                    // The turn's end speaks for the player whose turn it was.
                    if self.turns > 0 {
                        out.push(command(cmd::SPEAK_TURN_END, [self.turn_player, 0, 0, 0]));
                    }
                    self.turn_player = self.player(&m["player"]);
                    self.turns += 1;
                    // The client starts on turn 1; 6 would count it again.
                    if self.turns > 1 {
                        out.push(command(cmd::TURN, [self.turn_player, 0, 0, 0]));
                    }
                }
                Some(msg::NEW_PHASE) => {
                    self.summoning = None;
                    // Leaving a Battle Phase ends its battle mode even with no attack: the swords the client drew over
                    // the possible attackers (effect 0x3C) go only with BattleSelect, BattleAttack or BattleEnd.
                    let battle = (0x08..=0x80).contains(&m["phase"].as_u64().unwrap_or(0));
                    let ended = self.end_battle();
                    if ended.is_empty() && self.battle_phase && !battle {
                        out.push(Cmd::BattleEnd.message());
                    }
                    out.extend(ended);
                    self.battle_phase = battle;
                    if let Some(phase) = m["phase"].as_u64().and_then(client_phase) {
                        out.push(command(cmd::PHASE, [self.turn_player, phase, 0, 0]));
                        // The turn player's draw line, as the Draw Phase starts.
                        if phase == 0 {
                            out.push(command(cmd::SPEAK_DRAW, [self.turn_player, 1, 0, 0]));
                        }
                    }
                }
                Some(msg::MOVE) => {
                    out.extend(self.summon_before_move(&messages[i..]));
                    out.extend(self.move_card(m));
                }
                Some(msg::FIELD_DISABLED) => out.extend(self.disabled_zones(m)),
                Some(msg::HINT) if m["hint_type"].as_u64() == Some(hint::EVENT) && self.player(&m["player"]) == CLIENT => {
                    self.phase_window = m["hint"].as_u64().and_then(chain_window) == Some(ChainWindow::Phase);
                }
                Some(msg::HINT) if m["hint_type"].as_u64() == Some(hint::SELECTMSG) && self.player(&m["player"]) == CLIENT => {
                    self.hint = m["hint"].as_u64().unwrap_or(0);
                    self.hint_note = m["hint_text"].as_str().filter(|t| !t.is_empty()).map(|t| (self.hint, t.to_owned()));
                }
                // Also a Set Spell/Trap turning face-up as it is activated.
                Some(msg::POS_CHANGE) => out.extend(self.turn(m, m["position"].as_u64().unwrap_or(0))),
                // A Flip Summon as Konami's logs show it: the summon line with kind 4 before the flip, RunSummon with 1
                // after it (DuelScene reads only its place and uid: the summon effect at the card).
                Some(msg::FLIPSUMMONING) => {
                    let mut turned = self.turn(m, m["position"].as_u64().unwrap_or(0));
                    let card = self.engine_place(m).and_then(|p| Some((p, self.sides[p.player as usize].zones.get(&p.zone)?.first()?.uid)));
                    if let Some((place, uid)) = card {
                        let flip = turned.iter().position(|c| c.get("id").and_then(Value::as_u32) == Some(cmd::FLIP_TURN)).unwrap_or(turned.len());
                        turned.insert(flip, command(cmd::SPEAK_SUMMON, [place.player, 0x8000 | uid, SUMMON_FLIP, 0]));
                        let place = Place { index: 0, face_up: true, ..place };
                        turned.push(command(cmd::RUN_SUMMON, [place.packed(), uid, 1, 0]));
                    }
                    out.extend(turned);
                }
                Some(msg::CHAINING) => {
                    if self.player(&m["controller"]) == CLIENT {
                        match self.card_kinds.get(&(m["cid"].as_u64().unwrap_or(0) as u16)) {
                            Some(13) => self.stats.spells += 1,
                            Some(14) => self.stats.traps += 1,
                            _ => {}
                        }
                    }
                    out.extend(self.activation(m));
                    let link = self.chain_link(m);
                    let param = |k: &str| link.get(k).and_then(Value::as_u32).unwrap_or(0);
                    self.chain.push((param("param1"), param("param2")));
                    out.push(link);
                    // A hand card's effect: its avatar's reveal line after the ChainSet (Konami's logs).
                    if m["location"].as_u64() == Some(loc::HAND) {
                        out.push(command(cmd::SPEAK_REVEAL, [self.player(&m["controller"]), 1, m["cid"].as_u64().unwrap_or(0) as u32, 0]));
                    }
                    self.chaining = Some(m.clone());
                }
                // The link's own {player, zone, link}, as its ChainSet (Konami's logs; the client reads only the link).
                Some(msg::CHAIN_SOLVING) => {
                    let n = m["chain_size"].as_u64().unwrap_or(0) as u32;
                    let (player, zone) = self.chain.get((n as usize).wrapping_sub(1)).copied().unwrap_or((0, 0));
                    out.push(command(cmd::CHAIN_RUN, [player, zone, n, 0]));
                }
                Some(msg::ATTACK) => {
                    out.extend(self.end_battle());
                    out.extend(self.attack(m));
                }
                Some(msg::BATTLE) => out.extend(self.battle(m)),
                Some(msg::DAMAGE_STEP_END | msg::ATTACK_DISABLED) => out.extend(self.end_battle()),
                Some(kind @ (msg::SUMMONING | msg::SPSUMMONING)) => {
                    let id = if kind == msg::SUMMONING { cmd::RUN_SUMMON } else { cmd::RUN_SP_SUMMON };
                    // The effect plays at the place's zone (0x514E5A), so the place, not only the player.
                    let card = self.engine_place(m).and_then(|p| Some((p, self.sides[p.player as usize].zones.get(&p.zone)?.first()?.uid)));
                    // The place as moves give it, with its face and position bits (Konami's logs: 0x4000 | place for a
                    // Normal Summon, 0x4000/0xC000/0x8000 for a Special Summon).
                    // A Fusion or Synchro Summon said its line as it arrived; a Tribute Summon's line has its own kind.
                    let cid = m["cid"].as_u64().unwrap_or(0) as u16;
                    if self.player(&m["controller"]) == CLIENT {
                        self.stats.summoned.push(cid);
                        if self.cards.values().find(|c| c.cid == cid).is_some_and(|c| c.level >= crate::titles::HIGH) {
                            self.stats.high_level += 1;
                        }
                    }
                    let summoning = self.summoning.take();
                    let spoken = summoning.as_ref().is_some_and(|s| s.arrived && s.cid == cid);
                    let tribute = summoning.is_some_and(|s| s.family == Family::Tribute);
                    if let Some((place, uid)) = card {
                        // The summon line first: 0 a Normal Summon, 1 a Tribute Summon, 5 a Special Summon (Konami's).
                        let kind = match kind {
                            msg::SUMMONING if tribute => SUMMON_TRIBUTE,
                            msg::SUMMONING => SUMMON_NORMAL,
                            _ => SUMMON_SPECIAL,
                        };
                        if !spoken {
                            out.push(command(cmd::SPEAK_SUMMON, [place.player, 0x8000 | uid, kind, 0]));
                        }
                        out.push(command(id, [Place { index: 0, ..place }.packed(), uid, 0, 0]));
                    }
                }
                Some(msg::CONFIRM_CARDS) => out.extend(self.confirm(m)),
                Some(msg::CONFIRM_DECKTOP) => out.extend(self.reveal_deck_top(m)),
                Some(msg::REVERSE_DECK) => {
                    self.decks_reversed = !self.decks_reversed;
                    if !self.decks_reversed {
                        out.extend((0..2).flat_map(|p| self.face_down_deck_top(p)).collect::<Vec<_>>());
                    }
                }
                Some(msg::DECK_TOP) => out.extend(self.face_up_deck_top(m)),
                Some(msg::SHUFFLE_HAND) => out.extend(self.shuffle_hand(m)),
                Some(msg::SWAP) => out.extend(self.swap(m)),
                Some(msg::SHUFFLE_SET_CARD) => out.extend(self.shuffle_set(m)),
                Some(msg::SWAP_GRAVE_DECK) => out.extend(self.swap_grave_deck(m)),
                Some(msg::CHAIN_END) => {
                    self.chaining = None;
                    self.chain.clear();
                }
                Some(msg::WIN) => {
                    let result = match m["player"].as_u64() {
                        Some(TEAM_DRAW) => result::DRAW,
                        _ if self.player(&m["player"]) == CLIENT => result::WON,
                        _ => result::LOST,
                    };
                    if result == result::WON {
                        self.stats.win_reason = m["reason"].as_u64();
                    }
                    let reason = end_reason(m["reason"].as_u64().unwrap_or(1));
                    out.push(command(cmd::DUEL_END, [result, reason, 0, 0]));
                    self.result = Some(result);
                }
                Some(kind @ (msg::DAMAGE | msg::RECOVER | msg::PAY_LPCOST)) => {
                    let amount = m["amount"].as_i64().unwrap_or(0) as i32;
                    let change = if kind == msg::RECOVER { amount } else { -amount };
                    let damage = match kind {
                        msg::RECOVER => DAMAGE_GAIN,
                        msg::PAY_LPCOST => DAMAGE_COST,
                        _ if self.in_battle => DAMAGE_BATTLE,
                        _ => DAMAGE_EFFECT,
                    };
                    // As Konami's server sent it: the change shown (param3's low byte 1: its number and sound), the
                    // damage line, then the change applied (low byte 0). One message with 0 changed the LP silently.
                    let p = self.player(&m["player"]);
                    if p == CLIENT {
                        self.lp_change += change as i64;
                        self.stats.lowest_change = self.stats.lowest_change.min(self.lp_change);
                    }
                    out.push(command(cmd::CHANGE_LP, [p, change as u32, damage << 8 | 1, 0]));
                    out.push(command(cmd::SPEAK_DAMAGE, [p, change as u32, damage, 0]));
                    out.push(command(cmd::CHANGE_LP, [p, change as u32, damage << 8, 0]));
                }
                Some(kind @ (msg::ADD_COUNTER | msg::REMOVE_COUNTER)) => out.extend(self.counter(m, kind == msg::ADD_COUNTER)),
                Some(msg::LPUPDATE) => {
                    out.push(command(cmd::SET_LP, [self.player(&m["player"]), m["lp"].as_u64().unwrap_or(0) as u32, 0, 0]));
                }
                Some(kind @ (msg::EQUIP | msg::CARD_TARGET | msg::CANCEL_TARGET)) => self.link(m, kind),
                Some(msg::TOSS_COIN) => {
                    let p = self.player(&m["player"]);
                    let wins = self.coin_call.take().unwrap_or(true) as u32;
                    let heads: Vec<bool> = m["results"].as_array().into_iter().flatten().map(|r| r.as_bool().unwrap_or(false)).collect();
                    for coins in heads.chunks(COINS_A_TOSS) {
                        let faces = coins.iter().enumerate().fold(0, |bits, (i, &h)| bits | (h as u32) << i);
                        out.push(command(cmd::RUN_COIN, [p | (coins.len() as u32) << 6 | wins << 14, 0, faces, 0]));
                    }
                }
                Some(msg::TOSS_DICE) => {
                    let p = self.player(&m["player"]);
                    for face in m["results"].as_array().into_iter().flatten() {
                        out.push(command(cmd::RUN_DICE, [p, 0, face.as_u64().unwrap_or(1) as u32, 0]));
                    }
                }
                _ => self.unshown(m),
            }
        }
        out.extend(self.sync_relations());
        with_card_ids(&self.cards, &mut out);
        self.note_deck_tops(&out);
        out
    }

    /// An engine message `events` has no case for: fine when [`ENGINE_MESSAGES`] says so, else a warning, once a duel.
    fn unshown(&mut self, m: &Json) {
        let Some(kind) = m["type"].as_u64() else { return };
        let entry = ENGINE_MESSAGES.iter().find(|(k, _, _)| *k == kind);
        if matches!(entry, Some((_, _, Handling::Shown | Handling::Asked | Handling::Ignored(_)))) || !self.unshown.insert(kind) {
            return;
        }
        let name = entry.map_or("not in ENGINE_MESSAGES", |(_, name, _)| name);
        tracing::warn!("[duel] engine message {kind} ({name}) is not shown to the player");
    }

    /// The engine messages this duel dropped without a case for them (each warned once).
    pub fn unshown_messages(&self) -> Vec<u64> {
        let mut kinds: Vec<u64> = self.unshown.iter().copied().collect();
        kinds.sort();
        kinds
    }

    /// MSG_EQUIP {card, target} (an equip's new monster: its earlier link goes), MSG_CARD_TARGET {card, target} (a
    /// lasting target) and MSG_CANCEL_TARGET (its end): the cards' links, by uid.
    fn link(&mut self, m: &Json, kind: u64) {
        let uid = |t: &Self, loc: &Json| {
            let place = t.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL)?;
            t.sides[place.player as usize].zones.get(&place.zone)?.first().map(|c| c.uid)
        };
        let (Some(card), Some(target)) = (uid(self, &m["card"]), uid(self, &m["target"])) else { return };
        match kind {
            msg::EQUIP => {
                self.links.retain(|&(c, _, mark)| !(c == card && mark == MARK_EQUIP));
                self.links.push((card, target, MARK_EQUIP));
            }
            msg::CARD_TARGET if !self.links.contains(&(card, target, MARK_TARGET)) => self.links.push((card, target, MARK_TARGET)),
            msg::CANCEL_TARGET => self.links.retain(|&l| l != (card, target, MARK_TARGET)),
            _ => {}
        }
    }

    /// The client's relations brought in line with the links, both ways round so either card shows it: a link whose
    /// card left the field goes, one whose cards moved is re-sent at their new places. Removals, additions, then a redraw.
    fn sync_relations(&mut self) -> Vec<Message> {
        let place_of = |t: &Self, uid: u32| {
            (0..2u32).find_map(|p| {
                (0..=zone::FIELD_SPELL).find(|z| t.sides[p as usize].zones.get(z).and_then(|c| c.first()).is_some_and(|c| c.uid == uid)).map(|z| p | z << 8)
            })
        };
        let placed: Vec<_> = self.links.iter().map(|&(a, b, mark)| (place_of(self, a), place_of(self, b), mark)).collect();
        let mut wanted = vec![];
        let mut kept = vec![];
        for (&link, (a, b, mark)) in self.links.iter().zip(placed) {
            let (Some(a), Some(b)) = (a, b) else { continue };
            kept.push(link);
            for pair in [(a, b, mark), (b, a, mark)] {
                if !wanted.contains(&pair) {
                    wanted.push(pair);
                }
            }
        }
        self.links = kept;
        let mut out: Vec<Message> = self
            .relations
            .iter()
            .filter(|r| !wanted.contains(r))
            .map(|&(a, b, mark)| command(cmd::RELATION, [a, b, mark, 0]))
            .collect();
        out.extend(wanted.iter().filter(|r| !self.relations.contains(r)).map(|&(a, b, mark)| command(cmd::RELATION, [a, b, mark, 1])));
        if !out.is_empty() {
            out.push(command(cmd::RELATIONS_REDRAW, [0; 4]));
        }
        self.relations = wanted;
        out
    }

    /// A deck card with that id moves to the end of the hand.
    /// The zones a MSG_SELECT_DISFIELD may still take, as click targets: the command map keyed as a card on each zone
    /// would be (an empty zone's click sends the entry's command with the zone, 0x50B510 → 0x4F9510), then the input
    /// wait.
    fn zone_map(&self, m: &Json) -> Vec<Message> {
        let picked = self.zone_picks.clone().unwrap_or_default();
        let mut map = vec![];
        for (player, location, sequence) in free_zones(m) {
            if picked.contains(&json!({ "player": player, "location": location, "sequence": sequence })) {
                continue;
            }
            let side = self.player(&json!(player));
            // Keyed like a card on that zone (`place`): monster zones as zone 0, Spell/Trap zones as zone 5, with the
            // zone's index; the click comes back as the zone itself (position, index 0).
            let (zone, bit) = if location == loc::MZONE { (0, ZONE_PICK_MONSTER) } else { (5, ZONE_PICK_SPELL) };
            map.push((table_place(side, zone, sequence as usize), 1 << bit));
        }
        vec![command_data(&map, &[]), command(cmd::INPUT, [mode::MAIN, CLIENT, 0, 0])]
    }

    /// A zone clicked for MSG_SELECT_DISFIELD: a free one not picked yet counts; with `count` of them the answer, else
    /// the zones again.
    fn zone_click(&mut self, m: &Json, cmd: CardCommand) -> Prompt {
        let player = (0..2u64).find(|&p| self.player(&json!(p)) == cmd.player).unwrap_or(0);
        let (location, sequence) = match cmd.zone {
            z @ 0..=4 => (loc::MZONE, z as u64),
            z @ 5..=9 => (loc::SZONE, z as u64 - 5),
            _ => (0, 0),
        };
        let place = json!({ "player": player, "location": location, "sequence": sequence });
        let picks = self.zone_picks.get_or_insert_with(Vec::new);
        if cmd.index == 0 && free_zones(m).contains(&(player, location, sequence)) && !picks.contains(&place) {
            picks.push(place);
        }
        let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
        if picks.len() >= count {
            let places = std::mem::take(picks);
            self.zone_picks = None;
            self.pending = None;
            let kind = if m["type"] == msg::SELECT_PLACE { resp::PLACE } else { resp::DISFIELD };
            return Prompt::Answer(json!({ "type": kind, "places": places }));
        }
        Prompt::Client(self.zone_map(m))
    }

    /// MSG_FIELD_DISABLED {field_mask}: per player (× 16) monster zones bits 0-4, Spell/Trap zones 8-12, set when
    /// disabled. The client's 1006 {mask}: its own side low, the opponent's high, monster zones bits 0-4 and Spell/Trap
    /// zones 5-9, set when usable; DuelFieldManager applies it on the next 0x26 (0x50AA90), DuelInvalidZone drawing a
    /// zone that became unusable (effect 0x2D) or usable again (0x2E).
    fn disabled_zones(&mut self, m: &Json) -> Vec<Message> {
        let mask = m["field_mask"].as_u64().unwrap_or(0) as u32;
        let usable = |p: u32| {
            let dis = mask >> (16 * p);
            (0..5).filter(|z| dis & 1 << z == 0).fold(0, |a, z| a | 1 << z) | (0..5).filter(|z| dis & 1 << (8 + z) == 0).fold(0, |a, z| a | 1 << (5 + z))
        };
        let own = (0..2).find(|&p| self.player(&json!(p)) == CLIENT).unwrap_or(0);
        vec![command(cmd::ZONES_USABLE, [usable(own) | usable(1 - own) << 16, 0, 0, 0]), command(cmd::RELATIONS_REDRAW, [0; 4])]
    }

    fn draw(&mut self, p: u32, cid: u16) -> Vec<Message> {
        let hand = self.sides[p as usize].zone(zone::HAND).len();
        let from = Place { player: p, zone: zone::DECK, index: 0, face_up: false, defense: false };
        let to = Place { player: p, zone: zone::HAND, index: hand, face_up: false, defense: false };
        self.relocate(cid, from, to)
    }

    /// MSG_MOVE {card, cid, from, to}: the card leaves its place for the new one.
    /// Before a MSG_MOVE (`rest[0]`, the rest of the batch after it): a Tribute, Fusion or Synchro Summon's commands
    /// (see [`Summoning`]). The first material's move marks every material of the summon that moves in this batch; each
    /// plays its effect before its own move, but a Synchro's are all tuned at the first, the Tuners first (the client's
    /// first Tuner starts its level sums again, 0x5165A0; Konami's logs always tune the Tuner first, and the Synchro
    /// animation crashed on sums that left one out). An Extra Deck monster's arrival plays RunFusion and its summon
    /// line. The engine may report a summon's materials and its arrival in different batches (a Synchro Summon asks
    /// for its position in between).
    fn summon_before_move(&mut self, rest: &[Json]) -> Vec<Message> {
        let m = &rest[0];
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
        let reason = m["reason"].as_u64().unwrap_or(0);
        let mut out = vec![];
        if let Some(family) = Family::of(reason) {
            let mut s = match self.summoning.take() {
                Some(s) if s.family == family && !s.arrived => s,
                _ => Summoning { family, count: 0, tuned: 0, arrived: false, cid: 0, played: vec![], uids: vec![] },
            };
            if let Some(at) = s.played.iter().position(|(c, _)| *c == cid) {
                out.extend(s.played.remove(at).1);
                self.summoning = Some(s);
                return out;
            }
            // This move's material and the others of this batch: (Konami id, place, uid, Tuner). Each move's place is
            // the engine's as it happens, after the earlier ones: three materials from the hand all leave its index 0
            // (Ultimate Ancient Gear Golem's two Golems: both were taken for the first one's card), so each is found
            // among the cards the earlier ones left.
            let mut materials: Vec<(u16, Place, u32, bool)> = vec![];
            for x in rest {
                if x["type"].as_u64() != Some(msg::MOVE) || Family::of(x["reason"].as_u64().unwrap_or(0)) != Some(family) {
                    continue;
                }
                let cid = x["cid"].as_u64().unwrap_or(0) as u16;
                let Some(from) = self.engine_place(&x["from"]) else { continue };
                let taken: Vec<u32> = materials.iter().map(|&(_, _, uid, _)| uid).collect();
                if let Some(uid) = self.uid_left(&from, cid, &taken) {
                    materials.push((cid, from, uid, self.tuners.contains(&cid)));
                }
            }
            materials.sort_by_key(|&(.., tuner)| !(family == Family::Synchro && tuner));
            let marks = family.set_and_run();
            for &(_, from, uid, _) in materials.iter().filter(|_| marks.is_some()) {
                out.push(command(marks.map_or(0, |m| m.0), [from.player, from.zone, uid, 0]));
            }
            // Each material's effect (none for a Contact Fusion's), by its Konami id.
            let mut runs: Vec<(u16, Option<Message>)> = vec![];
            for &(material, from, uid, tuner) in &materials {
                let variant = match family {
                    Family::Synchro if tuner => {
                        s.tuned += 1;
                        self.level_of(material) | if s.tuned == 1 { 0x100 } else { 0x200 }
                    }
                    Family::Synchro => self.level_of(material),
                    _ => 0,
                };
                runs.push((material, marks.map(|m| command(m.1, [from.player, from.zone, uid, variant]))));
            }
            s.count += materials.len() as u32;
            s.uids.extend(materials.iter().map(|&(_, _, uid, _)| uid));
            // A Synchro tunes them all now, the Tuners first. A Fusion's or a Tribute's material plays its effect right
            // before its own move, as in Konami's logs: run ahead of the first move, a Fusion's materials from the hand
            // crashed the client (0x4FC4A9, live 2026-11-15, issue #40).
            if family == Family::Synchro {
                for (_, run) in &mut runs {
                    out.extend(run.take());
                }
            }
            if let Some(at) = runs.iter().position(|(c, _)| *c == cid) {
                out.extend(runs.remove(at).1);
            }
            // The others' moves come later in this batch, with their effects if not yet played.
            s.played = runs;
            self.summoning = Some(s);
            return out;
        }
        // A tribute for a Set: the monster goes to its zone face down and no summon follows.
        let set = m["to"]["location"].as_u64() == Some(loc::MZONE) && m["to"]["position"].as_u64().is_some_and(|p| p & 0xA != 0);
        if set && self.summoning.as_ref().is_some_and(|s| s.family == Family::Tribute) {
            self.summoning = None;
        }
        // The Extra Deck monster arrives: its animation and its line (Konami's logs: 8 for a Synchro, 6 for a Fusion).
        let arriving = reason & reason::SPSUMMON != 0 && m["from"]["location"].as_u64() == Some(loc::EXTRA);
        let place = self.engine_place(&m["from"]).and_then(|from| Some((from, self.uid_at(&from, cid)?)));
        if let Some(s) = self.summoning.as_mut().filter(|s| arriving && s.family != Family::Tribute && !s.arrived)
            && let Some((from, uid)) = place
        {
            let (kind, line) = match s.family {
                Family::Synchro => (2, SUMMON_SYNCHRO),
                Family::Contact => (1, SUMMON_EXTRA),
                _ => (0, SUMMON_EXTRA),
            };
            let at = |i: usize| s.uids.get(i).copied().unwrap_or(0);
            out.push(command(cmd::MATERIAL_LIST, [s.uids.len() as u32, at(0), at(1), at(2)]));
            for more in s.uids.get(3..).unwrap_or_default().chunks(4) {
                let at = |i: usize| more.get(i).copied().unwrap_or(0);
                out.push(command(cmd::MATERIAL_LIST_MORE, [at(0), at(1), at(2), at(3)]));
            }
            if from.player == CLIENT {
                match s.family {
                    Family::Synchro => self.stats.synchros += 1,
                    _ => self.stats.fusions += 1,
                }
            }
            out.push(command(cmd::RUN_FUSION, [from.player, uid, kind << 16 | s.count, 0]));
            out.push(command(cmd::SPEAK_SUMMON, [from.player, 0x8000 | uid, line, 0]));
            s.arrived = true;
            s.cid = cid;
        }
        out
    }

    /// The uid of the card with Konami id `cid` at `place` (in a list zone, at the place's index if it is there).
    fn uid_at(&self, place: &Place, cid: u16) -> Option<u32> {
        let zone = self.sides[place.player as usize].zones.get(&place.zone)?;
        let id = |c: &Card| self.cards.get(&c.row).map(|s| s.cid);
        zone.get(place.index).filter(|c| id(c) == Some(cid)).or_else(|| zone.iter().find(|c| id(c) == Some(cid))).map(|c| c.uid)
    }

    /// [`uid_at`](Self::uid_at) once the cards `taken` have left: a place the engine gives after earlier moves of the
    /// same batch, which the translator hasn't followed yet.
    fn uid_left(&self, place: &Place, cid: u16, taken: &[u32]) -> Option<u32> {
        let zone = self.sides[place.player as usize].zones.get(&place.zone)?;
        let id = |c: &Card| self.cards.get(&c.row).map(|s| s.cid);
        let left: Vec<&Card> = zone.iter().filter(|c| !taken.contains(&c.uid)).collect();
        left.get(place.index).filter(|c| id(c) == Some(cid)).or_else(|| left.iter().find(|c| id(c) == Some(cid))).map(|c| c.uid)
    }

    fn level_of(&self, cid: u16) -> u32 {
        self.cards.values().find(|c| c.cid == cid).map_or(0, |c| c.level)
    }

    fn move_card(&mut self, m: &Json) -> Vec<Message> {
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
        if is_material(&m["to"]) && !is_material(&m["from"]) {
            return self.attach(m, cid);
        }
        if is_material(&m["from"]) {
            return self.engine_place(&m["to"]).map(|to| self.detach(cid, to)).unwrap_or_default();
        }
        if m["from"]["location"].as_u64() == Some(0) {
            return self.engine_place(&m["to"]).map(|to| self.token(m, to)).unwrap_or_default();
        }
        if let Some(from) = self.engine_place(&m["from"]).filter(|f| f.zone <= zone::FIELD_SPELL)
            && let Some(card) = self.sides[from.player as usize].zones.get(&from.zone).and_then(|z| z.first()).copied()
            && self.tokens.contains(&card.uid)
            && !self.engine_place(&m["to"]).is_some_and(|to| to.zone <= zone::FIELD_SPELL)
        {
            // A token leaving the field is gone (EDOPro may send it to a pile first): it shatters off the field, a monster
            // zone or, equipped as a card (Allure Queen LV3), a Spell/Trap zone. One that changes sides (a control
            // change) or becomes an equip stays and moves like any card.
            let mut out = self.drop_counters(&from);
            self.sides[from.player as usize].zone(from.zone).clear();
            self.tokens.retain(|&t| t != card.uid);
            out.push(command(cmd::CARD_VANISH, [from.player, from.zone, 0, 0]));
            return out;
        }
        match (self.engine_place(&m["from"]), self.engine_place(&m["to"])) {
            (Some(from), Some(to)) => {
                let reason = m["reason"].as_u64().unwrap_or(0);
                let dropped = if to.zone > zone::FIELD_SPELL { self.drop_counters(&from) } else { vec![] };
                let kind = move_kind(reason, from, to);
                let mut moved = self.relocate_as(cid, from, to, kind);
                if !moved.is_empty() {
                    moved = self.presented(moved, cid, from, to, kind);
                }
                if to.zone == zone::HAND && let Some(at) = self.revealing.iter().position(|&c| c == cid) {
                    self.revealing.remove(at);
                    let index = self.sides[to.player as usize].zone(zone::HAND).len().saturating_sub(1).min(to.index);
                    moved.extend(self.show_hand(to.player, index));
                }
                // Only an actual destruction move confirms the card died. MSG_BATTLE precedes limited battle
                // protection (Fortress Warrior, Shield Wing), so its tentative destroy flag cannot hide the card.
                if moved.is_empty() || reason & reason::DESTROY == 0 || from.zone > zone::HAND {
                    return dropped.into_iter().chain(moved).collect();
                }
                let vanish = command(cmd::CARD_VANISH, [from.player, from.zone | (from.index as u32) << 8, 0, 0]);
                dropped.into_iter().chain(std::iter::once(vanish)).chain(moved).collect()
            }
            // Tokens and Xyz materials have no client place yet.
            _ => vec![],
        }
    }

    /// MSG_SHUFFLE_HAND {player, cards, cids}: the engine's hand in its new order (after a card in it was shown). The
    /// mirror takes that order and the client follows: each card's table entry at its new index, then HandShuffle. A
    /// list that doesn't match the mirror's hand (another count, an id it doesn't hold) is left alone: moves still find
    /// their card by id.
    fn shuffle_hand(&mut self, m: &Json) -> Vec<Message> {
        let p = self.player(&m["player"]);
        let cids: Vec<u16> = m["cids"].as_array().into_iter().flatten().map(|c| c.as_u64().unwrap_or(0) as u16).collect();
        let cards = &self.cards;
        let hand = self.sides[p as usize].zone(zone::HAND);
        if cids.len() != hand.len() {
            tracing::warn!("a hand shuffle of {} cards for a hand of {}", cids.len(), hand.len());
            return vec![];
        }
        let mut left = hand.clone();
        let mut order = Vec::with_capacity(left.len());
        for cid in &cids {
            let Some(at) = left.iter().position(|c| cards.get(&c.row).map(|s| s.cid) == Some(*cid)) else {
                tracing::warn!("a hand shuffle names card {cid}, which the hand doesn't hold");
                return vec![];
            };
            order.push(left.remove(at));
        }
        if order.iter().map(|c| c.uid).eq(hand.iter().map(|c| c.uid)) {
            return vec![];
        }
        *hand = order.clone();
        tracing::debug!("hand shuffle: player {p}, {} cards", order.len());
        let mut out: Vec<Message> = order
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let row = self.shown_row(&Place { player: p, zone: zone::HAND, index: i, face_up: false, defense: false }, c.row);
                Cmd::Card { uid: c.uid, open: false, row, place: table_place(p, zone::HAND, i) }.message()
            })
            .collect();
        out.push(command(cmd::HAND_SHUFFLE, [p, 0, 0, 0]));
        out
    }

    /// A field card becomes an Xyz material: the client has no place under a monster, so the card leaves the field with
    /// CardVanish (0x1F: marked off the field and hidden, 0x4FC520; the slot keeps the object until another card lands
    /// there) and the mirror keeps it among the materials. **Shape**: the shatter effect plays for it. From elsewhere (a
    /// hand or pile) it isn't followed (logged).
    fn attach(&mut self, m: &Json, cid: u16) -> Vec<Message> {
        let Some(from) = self.engine_place(&m["from"]) else { return vec![] };
        if from.zone > zone::FIELD_SPELL {
            tracing::warn!("an Xyz material from a place the client can't take it from: {m}");
            return vec![];
        }
        let Some(card) = self.sides[from.player as usize].zones.get(&from.zone).and_then(|z| z.first()).copied() else { return vec![] };
        if self.cards.get(&card.row).map(|s| s.cid) != Some(cid) && cid != 0 {
            tracing::warn!("mirror out of step: Xyz material {cid} not at {from:?}");
        }
        let mut out = self.drop_counters(&from);
        self.sides[from.player as usize].zone(from.zone).clear();
        self.positions.remove(&(from.player, from.zone));
        self.materials.push((card, from));
        out.push(command(cmd::CARD_VANISH, [from.player, from.zone, 0, 0]));
        out
    }

    /// An Xyz material leaves its monster for a place (a detach to the Graveyard, its monster leaving the field): the
    /// card goes there with a move from the field slot it left when that slot is still empty (the client clears the
    /// move's slot, 0x5000E0, and so lets go of the hidden object), else from any empty field slot (another card landed
    /// on its old one, which let go of it). **Shape**: the card flies from that slot. None empty: not followed (logged).
    fn detach(&mut self, cid: u16, to: Place) -> Vec<Message> {
        let Some(at) = self.materials.iter().position(|(c, _)| self.cards.get(&c.row).map(|s| s.cid) == Some(cid) || cid == 0) else { return vec![] };
        let empty_at = |s: &Self, p: u32, z: u32| s.sides[p as usize].zones.get(&z).is_none_or(Vec::is_empty);
        let left = self.materials[at].1;
        let empty = |s: &Self, p: u32| (0..=zone::FIELD_SPELL).find(|&z| empty_at(s, p, z)).map(|z| (p, z));
        let slot = if empty_at(self, left.player, left.zone) { Some((left.player, left.zone)) } else { empty(self, to.player).or_else(|| empty(self, 1 - to.player)) };
        let Some((player, zone)) = slot else {
            tracing::warn!("an Xyz material leaves with no empty field slot to move it from");
            return vec![];
        };
        let (card, _) = self.materials.remove(at);
        self.sides[player as usize].zone(zone).push(card);
        let via = Place { player, zone, index: 0, face_up: false, defense: false };
        self.relocate(cid, via, to)
    }

    /// MSG_SWAP_GRAVE_DECK {player, grave, extra}: a player's Graveyard and Deck trade their cards (Exchange of the Spirit),
    /// Extra Deck monsters going back to the Extra Deck. The sidecar gives the Graveyard and Extra Deck as they are after
    /// it. Each old Graveyard card moves out, from the top: to the Extra Deck while the engine's holds more of its id than
    /// the mirror's, else to the Deck; then each new Graveyard card moves in from the Deck, in the engine's order.
    fn swap_grave_deck(&mut self, m: &Json) -> Vec<Message> {
        let p = self.player(&m["player"]);
        let cids = |k: &str| -> Vec<u16> { m[k].as_array().into_iter().flatten().map(|c| c["cid"].as_u64().unwrap_or(0) as u16).collect() };
        let (grave, extra) = (cids("grave"), cids("extra"));
        let cid_of = |s: &Self, c: &Card| s.cards.get(&c.row).map_or(0, |s| s.cid);
        let at = |zone: u32, index: usize| Place { player: p, zone, index, face_up: true, defense: false };
        let mut out = vec![];
        while let Some(card) = self.sides[p as usize].zones.get(&zone::GRAVE).and_then(|z| z.last()).copied() {
            let cid = cid_of(self, &card);
            let index = self.sides[p as usize].zone(zone::GRAVE).len() - 1;
            let in_extra = self.sides[p as usize].zones.get(&zone::EXTRA).map_or(0, |z| z.iter().filter(|c| cid_of(self, c) == cid).count());
            let to = if extra.iter().filter(|&&c| c == cid).count() > in_extra {
                Place { face_up: false, ..at(zone::EXTRA, self.sides[p as usize].zone(zone::EXTRA).len()) }
            } else {
                Place { face_up: false, ..at(zone::DECK, 0) }
            };
            out.extend(self.relocate(cid, at(zone::GRAVE, index), to));
        }
        for (i, cid) in grave.into_iter().enumerate() {
            out.extend(self.relocate(cid, Place { face_up: false, ..at(zone::DECK, 0) }, at(zone::GRAVE, i)));
        }
        out
    }

    /// MSG_SWAP {card1, card2}: two field cards trade places (Creature Swap: two monsters change sides), each keeping its
    /// own position. Which mirror card is the engine's card1 is told by its id; then [`trade`](Self::trade).
    fn swap(&mut self, m: &Json) -> Vec<Message> {
        let (Some(p1), Some(p2)) = (self.engine_place(&m["card1"]), self.engine_place(&m["card2"])) else { return vec![] };
        if p1.zone > zone::FIELD_SPELL || p2.zone > zone::FIELD_SPELL {
            return vec![];
        }
        let cid1 = m["card1"]["cid"].as_u64().unwrap_or(0) as u16;
        let holds = |p: &Place| self.sides[p.player as usize].zones.get(&p.zone).and_then(|z| z.first()).and_then(|c| self.cards.get(&c.row)).map(|s| s.cid);
        // The message may name the places before or after the swap: card1 goes to whichever place doesn't hold it now.
        let (card1_at, card2_at) = if holds(&p2) == Some(cid1) && holds(&p1) != Some(cid1) { (p2, p1) } else { (p1, p2) };
        let (card1_to, card2_to) = (Place { face_up: p1.face_up, defense: p1.defense, ..card2_at }, Place { face_up: p2.face_up, defense: p2.defense, ..card1_at });
        self.trade(card1_to, card2_to)
    }

    /// MSG_SHUFFLE_SET_CARD {location, places: [{controller, location, sequence, position, cid}]}: Set cards shuffled
    /// among their zones (Magical Hats). The sidecar lists each shuffled zone and the card it holds after the shuffle (the
    /// core's own lists give only the old places, see `fix_shuffles` in crates/yo3-engine's session.rs); each of the mirror's cards in those
    /// zones is matched by id to its new zone (identical cards are interchangeable) and traded into place.
    fn shuffle_set(&mut self, m: &Json) -> Vec<Message> {
        let places: Vec<(Place, u16)> = m["places"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| Some((self.engine_place(p)?, p["cid"].as_u64().unwrap_or(0) as u16)))
            .filter(|(p, _)| p.zone <= zone::FIELD_SPELL)
            .collect();
        let mut left: Vec<(u32, u16)> = places
            .iter()
            .filter_map(|(p, _)| self.sides[p.player as usize].zones.get(&p.zone).and_then(|z| z.first()))
            .map(|c| (c.uid, self.cards.get(&c.row).map_or(0, |s| s.cid)))
            .collect();
        let mut moves = vec![];
        for (to, cid) in &places {
            if let Some(i) = left.iter().position(|(_, c)| c == cid) {
                moves.push((left.remove(i).0, *to));
            }
        }
        if !left.is_empty() {
            tracing::warn!("a Set card shuffle the mirror can't follow: {m}");
        }
        let mut out = vec![];
        for (uid, to) in moves {
            let at = self.sides.iter().enumerate().find_map(|(p, side)| {
                side.zones.iter().find(|(z, cards)| **z <= zone::FIELD_SPELL && cards.first().is_some_and(|c| c.uid == uid)).map(|(z, _)| (p as u32, *z))
            });
            let Some((player, zone)) = at.filter(|&(p, z)| (p, z) != (to.player, to.zone)) else { continue };
            // The card already in the new zone takes this card's old zone as it is shown now.
            let (face_up, defense) = self.positions.get(&(to.player, to.zone)).copied().unwrap_or((to.face_up, to.defense));
            out.extend(self.trade(to, Place { player, zone, index: 0, face_up, defense }));
        }
        out
    }

    /// The cards at field places `a` and `b` trade places: the one at `b` lands at `a` with `a`'s face and position, the
    /// one at `a` at `b` with `b`'s. CardSwap (0x1B) when both are there, a move (0x1A) when one place is empty; then each
    /// card's table entry and, for a monster zone, its record.
    fn trade(&mut self, a: Place, b: Place) -> Vec<Message> {
        let at = |s: &Self, p: &Place| s.sides[p.player as usize].zones.get(&p.zone).and_then(|z| z.first()).copied();
        let (card_a, card_b) = (at(self, &a), at(self, &b));
        let (record_a, record_b) = (self.records.get(&(a.player, a.zone)).copied(), self.records.get(&(b.player, b.zone)).copied());
        let mut out = match (card_a, card_b) {
            (Some(_), Some(_)) => vec![command(cmd::CARD_SWAP, [0, a.packed(), b.packed(), 0])],
            (Some(c), None) => vec![Cmd::Move { uid: c.uid, kind: 0, from: Place { face_up: false, defense: false, ..a }, to: b }.message()],
            (None, Some(c)) => vec![Cmd::Move { uid: c.uid, kind: 0, from: Place { face_up: false, defense: false, ..b }, to: a }.message()],
            (None, None) => return vec![],
        };
        for (place, card, record) in [(a, card_b, record_b), (b, card_a, record_a)] {
            let zone = self.sides[place.player as usize].zone(place.zone);
            zone.clear();
            zone.extend(card);
            match card {
                Some(c) => {
                    self.positions.insert((place.player, place.zone), (place.face_up, place.defense));
                    let row = self.shown_row(&place, c.row);
                    out.push(Cmd::Card { uid: c.uid, open: open(place.zone), row, place: table_place(place.player, place.zone, 0) }.message());
                    if place.zone < 5 && row != 0 && place.face_up {
                        out.extend(match record {
                            Some(values) => self.record(place.player, place.zone, c.row, values),
                            None => self.monster_record(place.player, place.zone, c.row),
                        });
                    }
                }
                None => {
                    self.positions.remove(&(place.player, place.zone));
                }
            }
        }
        out
    }

    /// MSG_CONFIRM_CARDS {player, cards}: the cards are shown to `player`. The client sees one it is shown to, and its
    /// own as the other side sees them; a hand card shows its face at once, a card from elsewhere (the Deck, a search)
    /// when it reaches the hand.
    fn confirm(&mut self, m: &Json) -> Vec<Message> {
        let viewer = self.player(&m["player"]);
        let mut out = vec![];
        // A Deck's top cards shown to the client (Ancient Telescope): turned over on the pile, as MSG_CONFIRM_DECKTOP.
        let cards: Vec<&Json> = m["cards"].as_array().into_iter().flatten().collect();
        let deck_cards: Vec<&Json> = cards.iter().copied().filter(|c| c["location"].as_u64() == Some(loc::DECK)).collect();
        if viewer == CLIENT && !deck_cards.is_empty() && deck_cards.len() == cards.len() {
            let owner = deck_cards[0]["controller"].clone();
            let size = self.sides[self.player(&owner) as usize].zones.get(&zone::DECK).map_or(0, Vec::len) as u64;
            let top = deck_cards.iter().all(|c| c["controller"] == owner && c["sequence"].as_u64().is_some_and(|s| s + deck_cards.len() as u64 >= size));
            if top {
                let shown = json!({ "player": owner, "cards": deck_cards });
                return self.reveal_deck_top(&shown);
            }
        }
        for card in m["cards"].as_array().into_iter().flatten() {
            let owner = self.player(&card["controller"]);
            if viewer != CLIENT && owner != CLIENT {
                continue;
            }
            let cid = card["cid"].as_u64().unwrap_or(0) as u16;
            match card["location"].as_u64() {
                Some(loc::HAND) => out.extend(self.show_hand(owner, card["sequence"].as_u64().unwrap_or(0) as usize)),
                Some(loc::MZONE | loc::SZONE) => {
                    if viewer == CLIENT && owner != CLIENT {
                        out.extend(self.reveal_field(card));
                    }
                }
                _ if !self.revealing.contains(&cid) => self.revealing.push(cid),
                _ => {}
            }
        }
        out
    }

    /// A face-down field card the client can't see, shown to it and hidden again, as Konami's server did (Stern Mystic in
    /// its logs): the table entry with the row, CardCheat (0x1D: the card object reloads its face from the row,
    /// 0x4FF000), the flip face up (0x1C), the card's popup (CardLockon 0x19 {player, zone, 2}), then the entry without a
    /// row, CardCheat and the flip back.
    fn reveal_field(&mut self, loc: &Json) -> Vec<Message> {
        let Some(place) = self.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL) else { return vec![] };
        let Some(&(face_up, defense)) = self.positions.get(&(place.player, place.zone)) else { return vec![] };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        if face_up || !self.hidden[place.player as usize] {
            return vec![];
        }
        let cid = self.cards.get(&card.row).map_or(0, |s| s.cid as u32);
        let down = Place { face_up: false, defense: place.zone < 5 && defense, ..place };
        let up = Place { face_up: true, ..down };
        let key = table_place(place.player, place.zone, 0);
        vec![
            Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: key }.message(),
            command(cmd::CARD_CHEAT, [up.packed(), cid, card.uid, 0]),
            command(cmd::FLIP_TURN, [up.packed(), 0, 0, 0]),
            command(cmd::CARD_LOCKON, [place.player, place.zone, POPUP, 0]),
            Cmd::Card { uid: card.uid, open: false, row: 0, place: key }.message(),
            command(cmd::CARD_CHEAT, [down.packed(), 0, card.uid, 0]),
            command(cmd::FLIP_TURN, [down.packed(), 0, 0, 0]),
        ]
    }

    /// Remembers the table entries sent for each Deck's index 0 (see `deck_tops`).
    fn note_deck_tops(&mut self, out: &[Message]) {
        for m in out {
            let int = |k: &str| m.get(k).and_then(Value::as_u32).unwrap_or(0);
            if int("id") != cmd::CARD {
                continue;
            }
            for p in 0..2 {
                if int("param4") == table_place(p, zone::DECK, 0) {
                    self.deck_tops[p as usize] = Some([int("param1"), int("param2"), int("param3"), int("param4")]);
                }
            }
        }
    }

    /// MSG_CONFIRM_DECKTOP {player, cards}: the Deck's top cards shown to both players, one at a time as Konami's server
    /// showed them: the card's table entry at the Deck's index 0, DeckReset (once), CardCheat, DeckFlipTop (the pile's
    /// top turns over from index 0's row) and the card's popup (CardLockon); then the top turns back and index 0 gets
    /// its last entry again, and DeckReset redraws the pile.
    fn reveal_deck_top(&mut self, m: &Json) -> Vec<Message> {
        let p = self.player(&m["player"]);
        let key = table_place(p, zone::DECK, 0);
        let deck = self.sides[p as usize].zones.get(&zone::DECK).cloned().unwrap_or_default();
        let mut used: Vec<u32> = vec![];
        let mut out = vec![];
        for c in m["cards"].as_array().into_iter().flatten() {
            let cid = c["cid"].as_u64().unwrap_or(0) as u16;
            let has_cid = |d: &&Card| !used.contains(&d.uid) && self.cards.get(&d.row).map(|s| s.cid) == Some(cid);
            let Some(card) = deck.iter().find(has_cid).copied() else { continue };
            used.push(card.uid);
            let hidden = self.shown_row(&Place { player: p, zone: zone::DECK, index: 0, face_up: false, defense: false }, card.row);
            // Index 0's last entry back while its card is still in the Deck; one that has left (drawn, sent) would pull
            // its table entry off its new place, so the revealed card stays there instead, hidden.
            let in_deck = |uid: u32| deck.iter().any(|d| d.uid == uid & !OPEN);
            let back = self.deck_tops[p as usize].filter(|e| in_deck(e[0]) && e[0] & OPEN == 0).unwrap_or([card.uid, hidden, 0, key]);
            out.push(Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: key }.message());
            if used.len() == 1 {
                out.push(command(cmd::DECK_RESET, [p, zone::DECK, 0, 0]));
            }
            out.extend([
                command(cmd::CARD_CHEAT, [0x4000 | move_place(p, zone::DECK, 0), cid as u32, card.uid, 0]),
                command(cmd::DECK_FLIP_TOP, [p, zone::DECK, 0, 0]),
                command(cmd::CARD_LOCKON, [p, zone::DECK, POPUP, 0]),
                command(cmd::DECK_FLIP_TOP, [p, zone::DECK, 0, 0]),
                command(cmd::CARD, back),
                command(cmd::DECK_RESET, [p, zone::DECK, 0, 0]),
            ]);
        }
        out
    }

    /// MSG_DECK_TOP {player, count, cid, position}: a face-up card on top of a Deck (Convulsion of Nature turned the
    /// Decks over): the table entry at the Deck's index 0 with its row and the public flag, then DeckReset, so the pile
    /// shows its face (0x4FE4D0), as a Graveyard shows its top card.
    fn face_up_deck_top(&mut self, m: &Json) -> Vec<Message> {
        let p = self.player(&m["player"]);
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
        // A reversed Deck's top is public though its cards keep their position (field.cpp reverse_deck only reorders);
        // otherwise only a card turned face up in the Deck (ReverseInDeck).
        let face_up = self.decks_reversed || m["position"].as_u64().unwrap_or(0) & (pos::FACEUP_ATTACK | pos::FACEUP_DEFENSE) != 0;
        if m["count"].as_u64().unwrap_or(0) != 0 || !face_up {
            return vec![];
        }
        let deck = self.sides[p as usize].zones.get(&zone::DECK);
        let Some(card) = deck.and_then(|d| d.iter().find(|c| self.cards.get(&c.row).map(|s| s.cid) == Some(cid))).copied() else {
            return vec![];
        };
        vec![Cmd::Card { uid: card.uid, open: true, row: card.row as u32, place: table_place(p, zone::DECK, 0) }.message(), command(cmd::DECK_RESET, [p, zone::DECK, 0, 0])]
    }

    /// The Decks turned back: the card last shown on top of player `p`'s Deck loses its face (and, hidden, its row).
    fn face_down_deck_top(&mut self, p: u32) -> Vec<Message> {
        let Some([uid, row, _, place]) = self.deck_tops[p as usize].filter(|e| e[0] & OPEN != 0) else { return vec![] };
        let uid = uid & !OPEN;
        // Only while that card is still in the Deck (a drawn one has its own entry by now).
        if !self.sides[p as usize].zones.get(&zone::DECK).is_some_and(|d| d.iter().any(|c| c.uid == uid)) {
            return vec![];
        }
        let shown = self.shown_row(&Place { player: p, zone: zone::DECK, index: 0, face_up: false, defense: false }, row as u16);
        vec![Cmd::Card { uid, open: false, row: shown, place }.message(), command(cmd::DECK_RESET, [p, zone::DECK, 0, 0])]
    }

    /// A hand card shows its face: its table entry with the row (the opponent's hand has none), then HandShow.
    fn show_hand(&self, player: u32, index: usize) -> Vec<Message> {
        let Some(card) = self.sides[player as usize].zones.get(&zone::HAND).and_then(|z| z.get(index)).copied() else { return vec![] };
        vec![
            Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: table_place(player, zone::HAND, index) }.message(),
            command(cmd::HAND_SHOW, [player, index as u32, 0, 0]),
        ]
    }

    /// A token appears on a monster zone: the client's token card of its name (in turn when it has several, as for
    /// the four Sheep Tokens), a card table entry, its record and a move onto its own slot, which makes the card
    /// object (0x1A finds none by uid, 0x5003A0). A token with no card of its name isn't shown.
    fn token(&mut self, m: &Json, to: Place) -> Vec<Message> {
        let reserved = self.reserved_token.take();
        let Some(rows) = self.token_card_rows(m) else {
            tracing::warn!("a token the client has no card for: {} {}", m["cid"], m["name"]);
            return vec![];
        };
        if to.zone >= 5 {
            return vec![];
        }
        // The uid and card its position dialog showed, so the dialog's card is the one that appears.
        let (uid, row) = match reserved {
            Some((uid, row)) if rows.contains(&row) => (uid, row),
            _ => self.next_token_card(&rows),
        };
        self.tokens.push(uid);
        let zone = self.sides[to.player as usize].zone(to.zone);
        zone.clear();
        zone.push(Card { uid, row });
        self.positions.insert((to.player, to.zone), (to.face_up, to.defense));
        let mut out = vec![Cmd::Card { uid, open: false, row: row as u32, place: table_place(to.player, to.zone, 0) }.message()];
        out.extend(self.monster_record(to.player, to.zone, row));
        out.push(Cmd::Move { uid, kind: MOVE_SPECIAL_SUMMON, from: to, to }.message());
        out
    }

    /// The client's token cards (CARD_Prop rows) for an engine token: a token with a Konami id (Doomsday Token, 3940)
    /// is the client's card of that id; one without, by its name. None for a card that isn't a token the client has.
    fn token_card_rows(&self, m: &Json) -> Option<Vec<u16>> {
        let cid = m["cid"].as_u64().filter(|&c| c != 0).map(|c| c as u16);
        let by_id: Option<Vec<u16>> = cid.map(|c| {
            let mut rows: Vec<u16> = self.cards.iter().filter(|(_, s)| s.cid == c).map(|(&r, _)| r).collect();
            rows.sort_unstable();
            rows
        });
        let by_name = || m["name"].as_str().and_then(|n| self.token_rows.get(n)).cloned();
        by_id.filter(|r| !r.is_empty()).or_else(by_name).filter(|r| !r.is_empty())
    }

    /// A new token's uid and card: the next of its cards in turn (the four Sheep Tokens' colours).
    fn next_token_card(&mut self, rows: &[u16]) -> (u32, u16) {
        let row = rows[(self.next_token - FIRST_TOKEN_UID) as usize % rows.len()];
        let uid = self.next_token;
        self.next_token += 1;
        (uid, row)
    }

    /// Moves a card in the mirror and on the client: the card table entry (1000), then on screen (0x1A); the
    /// cards whose index shifted in a list zone get their table entry again.
    fn relocate(&mut self, cid: u16, from: Place, to: Place) -> Vec<Message> {
        self.relocate_as(cid, from, to, MOVE_PLAIN)
    }

    /// A move's presentation around it, as Konami's server sent it: a card leaving the hand for the field is announced
    /// first (`0x48`: a Set speaks), a Set plays the Set effect after it (`0x1E`), and the Field zone's card changes the
    /// field's background (7: its Konami id face up, 0 when it leaves).
    fn presented(&self, moved: Vec<Message>, cid: u16, from: Place, to: Place, kind: u32) -> Vec<Message> {
        let mut out = vec![];
        if from.zone == zone::HAND && to.zone <= zone::FIELD_SPELL && matches!(kind, MOVE_NORMAL_SUMMON | MOVE_ACTIVATE | MOVE_SET) {
            // The speech never reads the id; a Set card the client may not see goes without it.
            let shown = if kind == MOVE_SET && self.hidden[to.player as usize] { 0 } else { cid as u32 };
            out.push(command(cmd::SPEAK_PLAY, [from.player, shown, (kind != MOVE_SET) as u32, (to.zone >= 5) as u32]));
        }
        out.extend(moved);
        if kind == MOVE_SET && to.zone <= zone::FIELD_SPELL {
            out.push(command(cmd::CARD_SET, [to.player, to.zone, (to.zone < 5) as u32, 0]));
        }
        if to.zone == zone::FIELD_SPELL && to.face_up {
            out.push(command(cmd::FIELD_CHANGE, [cid as u32, 0, 0, 0]));
        } else if from.zone == zone::FIELD_SPELL && from.face_up && to.zone != zone::FIELD_SPELL {
            out.push(command(cmd::FIELD_CHANGE, [0; 4]));
        }
        out
    }

    /// An activation's presentation before its ChainSet, as Konami's server sent it: the activation line (`0x47`; for a
    /// Spell/Trap step 2 then 3, as from the hand, since a Set card is face up by its MSG_CHAINING; Konami's sent 1 for
    /// a Set one, which the client only keeps as context) and the card's activation effect (`0x22`). Nothing for a card
    /// the client has no place for.
    fn activation(&self, m: &Json) -> Vec<Message> {
        let mut at = m.clone();
        at["location"] = json!(m["location"].as_u64().unwrap_or(0) & !loc::OVERLAY);
        let Some(place) = self.engine_place(&at) else { return vec![] };
        let index = if place.zone <= zone::FIELD_SPELL { 0 } else { place.index };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.get(index)).copied() else { return vec![] };
        let cid = self.cards.get(&card.row).map_or(0, |c| c.cid as u32);
        let mut out = vec![];
        if (5..=zone::FIELD_SPELL).contains(&place.zone) {
            out.push(command(cmd::SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 2, 0]));
            out.push(command(cmd::SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 3, 0]));
        } else {
            out.push(command(cmd::SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 4, 0]));
        }
        let place = Place { index, face_up: true, ..place };
        out.push(command(cmd::CARD_ACTIVATE, [place.packed(), cid, card.uid, 0]));
        out
    }

    /// What the card info panel reads for a monster moving into or out of the client's hand, or into a Graveyard, which
    /// field records don't cover ([`HAND_LEVEL`], [`PILE_STATS`]). The opponent's hand isn't given levels: its cards
    /// are hidden against a player.
    fn panel_stats(&self, card: Card, from: Place, to: Place, row: u32) -> Vec<Message> {
        let Some(s) = self.cards.get(&card.row).filter(|s| s.level > 0) else { return vec![] };
        let mut out = vec![];
        if from.zone == zone::HAND && to.zone != zone::HAND && from.player == CLIENT {
            out.push(command(cmd::HAND_LEVEL, [card.uid, NO_LEVEL, 0, 0]));
        }
        if to.zone == zone::HAND && from.zone != zone::HAND && to.player == CLIENT && row != 0 {
            out.push(command(cmd::HAND_LEVEL, [card.uid, s.level, 0, 0]));
        }
        if to.zone == zone::GRAVE && from.zone != zone::GRAVE {
            let id = s.cid as u32;
            out.extend([command(cmd::PILE_STATS, [id, id, s.attribute, s.race]), command(cmd::PILE_STATS_COMMIT, [id, s.level, 0, 0])]);
        }
        out
    }

    /// [`relocate`] with the move's kind (`move_kind`), which the client's move takes in `param1`'s high half.
    fn relocate_as(&mut self, cid: u16, from: Place, to: Place, kind: u32) -> Vec<Message> {
        let record = (from.zone <= zone::FIELD_SPELL).then(|| self.records.get(&(from.player, from.zone)).copied()).flatten();
        let cards = &self.cards;
        let source = self.sides[from.player as usize].zone(from.zone);
        let at = if from.zone == zone::DECK || from.zone == zone::EXTRA {
            source.iter().position(|c| cards.get(&c.row).map(|s| s.cid) == Some(cid)).or(if source.is_empty() { None } else { Some(0) })
        } else if source.is_empty() {
            None
        } else {
            // The engine's sequence, unless the mirror holds another card there: a list zone that fell out of step
            // with the engine would move (and show) the wrong card, so the card with the id is taken instead.
            let at = from.index.min(source.len() - 1);
            let id_at = |i: usize| cards.get(&source[i].row).map(|s| s.cid);
            let known = cid != 0 && !self.tokens.contains(&source[at].uid);
            if known && id_at(at) != Some(cid) && let Some(found) = (0..source.len()).find(|&i| id_at(i) == Some(cid)) {
                tracing::warn!("mirror out of step: card {cid} not at {from:?}, found at {found}");
                Some(found)
            } else {
                Some(at)
            }
        };
        let Some(at) = at else { return vec![] };
        // The client takes a hand card out by the move's index (0x5000E0 → 0x5005D0), so the move names the index the
        // mirror (the client's view) found it at, not the engine's: from another index the client drops the wrong card
        // and keeps this one in its hand list, and the next renumbering of that hand crashes on it (0x516185).
        let from = if from.zone == zone::DECK || from.zone == zone::EXTRA { from } else { Place { index: at, ..from } };
        let card = source.remove(at);
        let dest = self.sides[to.player as usize].zone(to.zone);
        let index = if to.zone > zone::FIELD_SPELL { to.index.min(dest.len()) } else { 0 };
        if to.zone <= zone::FIELD_SPELL {
            dest.clear();
        }
        dest.insert(index, card);
        let to = Place { index, ..to };
        // Monsters' face and position, and Spell/Traps' face (a revealed Set card, `reveal_field`).
        if to.zone <= zone::FIELD_SPELL {
            self.positions.insert((to.player, to.zone), (to.face_up, to.defense));
        }
        let row = self.shown_row(&to, card.row);
        let mut out = self.panel_stats(card, from, to, row);
        out.push(Cmd::Card { uid: card.uid, open: open(to.zone), row, place: table_place(to.player, to.zone, index) }.message());
        if to.zone < 5 && row != 0 {
            out.extend(match record {
                Some(values) => self.record(to.player, to.zone, card.row, values),
                None => self.monster_record(to.player, to.zone, card.row),
            });
        }
        // List zones renumber: re-send the shifted cards (not the face-down deck). A pile's go before the move, whose
        // count change redraws the pile from the table's top entry (0x4FE4D0); the hand's after.
        let mut hand = vec![];
        for (p, zone, start) in [(from.player, from.zone, at), (to.player, to.zone, index + 1)] {
            if zone <= zone::FIELD_SPELL || zone == zone::DECK {
                continue;
            }
            let shifted: Vec<(usize, Card)> = self.sides[p as usize].zone(zone).iter().copied().enumerate().skip(start).collect();
            for (i, c) in shifted {
                let row = self.shown_row(&Place { player: p, zone, index: i, face_up: true, defense: false }, c.row);
                let entry = Cmd::Card { uid: c.uid, open: open(zone), row, place: table_place(p, zone, i) }.message();
                if zone == zone::HAND { hand.push(entry) } else { out.push(entry) }
            }
        }
        out.push(Cmd::Move { uid: card.uid, kind, from, to }.message());
        out.extend(hand);
        out
    }

    /// MSG_ATTACK {card, target}: the battle's start and the attack, at the target or directly.
    fn attack(&mut self, m: &Json) -> Vec<Message> {
        let Some(attacker) = self.engine_place(&m["card"]) else { return vec![] };
        let target = self.engine_place(&m["target"]).map_or(Target::Direct { player: 1 - attacker.player }, |t| Target::Monster { player: t.player, zone: t.zone });
        self.in_battle = true;
        vec![Cmd::BattleInit.message(), Cmd::BattleAttack { player: attacker.player, zone: attacker.zone, target }.message()]
    }

    /// MSG_BATTLE {card, target: {attack, defense, position, destroyed}}: per player, the battle damage they take, by
    /// ATK against ATK, or DEF when the target defends (effects that change it aren't followed), and whether their card
    /// would be destroyed. Limited protection is applied after this message, so destruction is shown only when
    /// MSG_MOVE confirms it. The client pairs the values with the cards by player (DuelBattle states 5 and 6). A direct
    /// attack's target is an empty place (location 0; ocgcore-wasm parses it like any card): no figures, since there
    /// is no target card to show them at (the LP change shows the damage).
    fn battle(&self, m: &Json) -> Vec<Message> {
        let (card, target) = (&m["card"], &m["target"]);
        if target["location"].as_u64().unwrap_or(0) == 0 {
            return vec![Cmd::BattleRun { damage: [0; 2], flags: 0 }.message()];
        }
        let n = |c: &Json, k: &str| c[k].as_i64().unwrap_or(0);
        let (to_attacker, to_target) = if target["position"].as_u64().unwrap_or(0) & (pos::FACEUP_ATTACK | pos::FACEDOWN_ATTACK) != 0 {
            let diff = n(card, "attack") - n(target, "attack");
            (-diff.min(0), diff.max(0))
        } else {
            ((n(target, "defense") - n(card, "attack")).max(0), 0)
        };
        let mut damage = [0u32; 2];
        let attacker = self.player(&card["controller"]) as usize;
        damage[attacker] = to_attacker as u32;
        damage[1 - attacker] = to_target as u32;
        vec![Cmd::BattleRun { damage, flags: 0 }.message()]
    }

    /// The end of the battle shown on the client, if one is.
    fn end_battle(&mut self) -> Vec<Message> {
        self.attacker = None;
        if std::mem::take(&mut self.in_battle) { vec![Cmd::BattleEnd.message()] } else { vec![] }
    }

    /// A field card turns in place to `position`: 0x1C with its new face and position, after its table entry and record
    /// when it shows a row it didn't before (the opponent's face-down card turning face-up when hiding).
    fn turn(&mut self, loc: &Json, position: u64) -> Vec<Message> {
        let Some(place) = self.engine_place(loc) else { return vec![] };
        if place.zone > zone::FIELD_SPELL {
            return vec![];
        }
        // A Spell/Trap has no battle position.
        let place = Place { face_up: position & 5 != 0, defense: place.zone < 5 && position & 12 != 0, ..place };
        let before = (place.zone < 5).then(|| self.positions.insert((place.player, place.zone), (place.face_up, place.defense)));
        let mut out = vec![];
        let card = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied();
        if place.zone >= 5 {
            // A Set Spell/Trap turning face up is activated: its avatar's reveal line first (Konami's logs: 0x46 before the
            // flip; the activation line after it stays silent, DuelCutin +0x68).
            let was_down = self.positions.insert((place.player, place.zone), (place.face_up, false)).is_some_and(|(f, _)| !f);
            if let Some(card) = card
                && was_down
                && place.face_up
            {
                let cid = self.cards.get(&card.row).map_or(0, |c| c.cid as u32);
                out.push(command(cmd::SPEAK_REVEAL, [place.player, 1, cid, 0]));
            }
        }
        // Face down, the client empties the card's counters itself (0x4FBE50); so does EDOPro.
        if let Some(card) = card
            && !place.face_up
        {
            self.counters.retain(|(uid, _), _| *uid != card.uid);
        }
        if let Some(card) = card
            && self.hidden[place.player as usize]
            && place.face_up
        {
            out.push(Cmd::Card { uid: card.uid, open: false, row: card.row as u32, place: table_place(place.player, place.zone, 0) }.message());
            if place.zone < 5 {
                out.extend(self.monster_record(place.player, place.zone, card.row));
            } else {
                // The field card takes its face from a move (a flip only turns it over), and a Spell/Trap has no
                // record to carry its id: it moves onto its own slot with its row, face up.
                let down = Place { face_up: false, ..place };
                out.push(Cmd::Move { uid: card.uid, kind: 0, from: down, to: place }.message());
                return out;
            }
        }
        // A player's own monster turned in place stayed upright on that player's screen (live, the VM round: every
        // 0x1C after its summon), while a card arriving by a move is drawn in its position: it moves onto its own
        // slot in the new position first.
        if let (Some(before), Some(card)) = (before, card) {
            let (face_up, defense) = before.unwrap_or((!place.face_up, !place.defense));
            let from = Place { face_up, defense, ..place };
            out.push(Cmd::Move { uid: card.uid, kind: 0, from, to: place }.message());
        }
        out.push(command(cmd::FLIP_TURN, [place.packed(), 0, 0, 0]));
        out
    }

    /// MSG_ADD_COUNTER / MSG_REMOVE_COUNTER {counter_type, controller, location, sequence, count} on a field card: the
    /// change in the client's kind, a removal no more than the client holds.
    fn counter(&mut self, m: &Json, add: bool) -> Vec<Message> {
        let Some(place) = self.engine_place(m).filter(|p| p.zone <= zone::FIELD_SPELL) else { return vec![] };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        let id = m["counter_type"].as_u64().unwrap_or(0);
        let kind = COUNTER_KINDS.iter().find(|(e, _)| *e == id).map_or(ANY_COUNTER, |(_, k)| *k);
        let count = m["count"].as_u64().unwrap_or(0) as u32;
        let held = self.counters.entry((card.uid, kind)).or_default();
        let change = if add { count as i32 } else { -(count.min(*held) as i32) };
        *held = held.saturating_add_signed(change);
        if change == 0 {
            return vec![];
        }
        vec![command(cmd::COUNTER, [place.player | place.zone << 8, change as u32, kind, 0])]
    }

    /// A field card about to leave its place gives up its counters on the client, which would keep them (EDOPro drops
    /// them without a message).
    fn drop_counters(&mut self, place: &Place) -> Vec<Message> {
        if place.zone > zone::FIELD_SPELL {
            return vec![];
        }
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        let mut held: Vec<(u32, u32)> =
            self.counters.iter().filter(|((uid, _), n)| *uid == card.uid && **n > 0).map(|((_, kind), n)| (*kind, *n)).collect();
        held.sort_unstable();
        self.counters.retain(|(uid, _), _| *uid != card.uid);
        held.into_iter().map(|(kind, n)| command(cmd::COUNTER, [place.player | place.zone << 8, (n as i32).wrapping_neg() as u32, kind, 0])).collect()
    }

    /// The record the card info and the field label show for a monster zone (0x523F00, keyed player | zone << 8):
    /// shown id, second id, race, attribute, level, ATK, DEF, base ATK, base DEF. Data commands 0x3E9 {_, values 0,
    /// 2, 3}, 0x3EA {_, values 4, 5, 6}, 0x3EB {key, values 7, 8, 1} (which commits it; the pop 0x522A50).
    /// Stats from CARD_Prop as the monster enters; the sidecar's field report brings later changes (`field`).
    fn monster_record(&mut self, player: u32, zone: u32, row: u16) -> Vec<Message> {
        let s = self.cards.get(&row).copied().unwrap_or_default();
        self.record(player, zone, row, [s.level, s.atk, s.def, s.atk, s.def])
    }

    /// A monster zone's record with these values (level, ATK, DEF, base ATK, base DEF); race and attribute are
    /// CARD_Prop's.
    fn record(&mut self, player: u32, zone: u32, row: u16, values: [u32; 5]) -> Vec<Message> {
        self.kinds.remove(&(player, zone));
        self.record_with(player, zone, row, values, None)
    }

    /// [`record`] with a Type and Attribute an effect changed (the client's numbers), kept for the zone.
    fn record_with(&mut self, player: u32, zone: u32, row: u16, values: [u32; 5], kinds: Option<(u32, u32)>) -> Vec<Message> {
        self.records.insert((player, zone), values);
        let s = self.cards.get(&row).copied().unwrap_or_default();
        let cid = s.cid as u32;
        let (race, attribute) = kinds.unwrap_or((s.race, s.attribute));
        if let Some(k) = kinds {
            self.kinds.insert((player, zone), k);
        }
        let [level, atk, def, base_atk, base_def] = values;
        vec![
            command(cmd::RECORD_BEGIN, [0, cid, race, attribute]),
            command(cmd::RECORD_MORE, [0, level, atk, def]),
            command(cmd::RECORD_COMMIT, [player | zone << 8, base_atk, base_def, cid]),
        ]
    }

    /// The sidecar's report of the monsters' stats after an engine step ({"type": "field", monsters: [{controller,
    /// sequence, position, level, attack, defense, base_attack, base_defense, race, attribute}]}): a face-up monster
    /// whose values differ from those its record last went with gets the record again, which DuelCardManager picks up
    /// after the next command (0x4FFF14) and shows on its label. A level of 0 (an Xyz monster's) keeps CARD_Prop's.
    /// A Type or Attribute other than CARD_Prop's (DNA Surgery, Zombie World) goes in the record too: the info panel
    /// reads them from it (0x453740, 0x523F00).
    fn field(&mut self, m: &Json) -> Vec<Message> {
        let mut out = vec![];
        for s in m["monsters"].as_array().into_iter().flatten() {
            let int = |k: &str| s[k].as_u64().unwrap_or(0) as u32;
            if int("position") as u64 & (pos::FACEUP_ATTACK | pos::FACEUP_DEFENSE) == 0 || int("sequence") >= 5 {
                continue;
            }
            let (player, zone) = (self.player(&s["controller"]), int("sequence"));
            let Some(card) = self.sides[player as usize].zones.get(&zone).and_then(|z| z.first()).copied() else { continue };
            let level = Some(int("level")).filter(|&l| l > 0).unwrap_or(self.cards.get(&card.row).map_or(0, |c| c.level));
            let values = [level, int("attack"), int("defense"), int("base_attack"), int("base_defense")];
            let printed = self.cards.get(&card.row).map_or((0, 0), |c| (c.race, c.attribute));
            let race = client_number(s["race"].as_u64(), &TYPES).unwrap_or(printed.0);
            let attribute = client_number(s["attribute"].as_u64(), &ATTRIBUTES).unwrap_or(printed.1);
            let kinds = Some((race, attribute)).filter(|&k| k != printed);
            if self.records.get(&(player, zone)) != Some(&values) || self.kinds.get(&(player, zone)).copied() != kinds {
                out.extend(self.record_with(player, zone, card.row, values, kinds));
                if kinds.is_none() {
                    self.kinds.remove(&(player, zone));
                }
            }
        }
        out
    }

    /// The client place of an engine location {controller, location, sequence, position}.
    fn engine_place(&self, loc: &Json) -> Option<Place> {
        // An Xyz material: ocgcore-wasm gives its Xyz Monster's location with an `overlay_sequence` (a material attached
        // in the Extra Deck read as the Extra Deck, one detached as its monster's zone); the client has no place for it.
        if is_material(loc) {
            return None;
        }
        let player = self.player(&loc["controller"]);
        let seq = loc["sequence"].as_u64()? as usize;
        let (zone, index) = match loc["location"].as_u64()? {
            loc::DECK => (zone::DECK, 0),
            loc::HAND => (zone::HAND, seq),
            loc::MZONE => (seq as u32, 0),
            loc::SZONE if seq == 5 => (zone::FIELD_SPELL, 0),
            loc::SZONE => (5 + seq as u32, 0),
            loc::GRAVE => (zone::GRAVE, seq),
            loc::REMOVED => (zone::REMOVED, seq),
            loc::EXTRA => (zone::EXTRA, seq),
            _ => return None,
        };
        // ocgcore positions: 1 face-up attack, 2 face-down attack, 4 face-up defense, 8 face-down defense. Only
        // field cards carry them; the client shows each list zone its own way. Only monsters lie sideways: a set
        // Spell/Trap comes face-down defense, which drew it like a set monster (live).
        // The Graveyard is face up: face down, its top card showed only its back (live).
        let position = match zone {
            zone::GRAVE => pos::FACEUP_ATTACK,
            zone::REMOVED => loc["position"].as_u64().unwrap_or(pos::FACEUP_ATTACK),
            zone if zone <= zone::FIELD_SPELL => loc["position"].as_u64().unwrap_or(0),
            _ => 0,
        };
        Some(Place { player, zone, index, face_up: position & 5 != 0, defense: zone < 5 && position & 12 != 0 })
    }

    /// ChainSet for MSG_CHAINING {controller, location, sequence, chain_size}: the link over its card's zone (an Xyz
    /// material's over its monster).
    fn chain_link(&self, m: &Json) -> Message {
        let mut at = m.clone();
        at["location"] = json!(m["location"].as_u64().unwrap_or(0) & !loc::OVERLAY);
        let zone = self.engine_place(&at).map_or(FIELD_CENTRE, |p| p.zone);
        command(cmd::CHAIN_SET, [self.player(&m["controller"]), zone, m["chain_size"].as_u64().unwrap_or(0) as u32, 0])
    }

    /// What to do with the engine's prompt (the last message when it awaits).
    pub fn prompt(&mut self, m: &Json) -> Prompt {
        let staged = self.staged.take();
        self.list_order.clear();
        self.field_target = false;
        let prompt = self.prompt_for(m);
        // A window's hint is for the prompt right after it, whichever it was.
        self.phase_window = false;
        match prompt {
            Prompt::Client(out) => {
                let mut all = self.unstage_n(staged);
                if std::mem::take(&mut self.taken_back) {
                    all.insert(0, Cmd::BattleEnd.message());
                }
                all.extend(out);
                with_card_ids(&self.cards, &mut all);
                Prompt::Client(all)
            }
            answer => {
                self.staged = self.staged.or(staged);
                answer
            }
        }
    }

    fn prompt_for(&mut self, m: &Json) -> Prompt {
        let kind = m["type"].as_u64().unwrap_or(0);
        if self.player(&m["player"]) == CPU {
            if kind == msg::ANNOUNCE_CARD {
                return Prompt::Answer(self.cpu_card_name(m));
            }
            return Prompt::Answer(cpu_answer(kind, m));
        }
        match kind {
            msg::SELECT_IDLECMD | msg::SELECT_BATTLECMD => {
                let (lists, buttons, mode): (Lists, [(&str, u8); 2], u32) = if kind == msg::SELECT_IDLECMD {
                    (&IDLE_LISTS, [("to_bp", flag::BUTTON_BP), ("to_ep", flag::BUTTON_EP)], mode::MAIN)
                } else {
                    (&BATTLE_LISTS, [("to_m2", flag::BUTTON_M2), ("to_ep", flag::BUTTON_EP)], mode::BATTLE)
                };
                self.attacker = None;
                let mut map = self.command_map(m, lists);
                self.add_pile_views(&mut map);
                let flags: Vec<(u8, u8)> = buttons.into_iter().filter(|(k, _)| m[*k] == true).map(|(_, b)| (b, 1)).collect();
                self.pending = Some(m.clone());
                Prompt::Client(vec![command_data(&map, &flags), command(cmd::INPUT, [mode, CLIENT, 0, 0])])
            }
            msg::SELECT_EFFECTYN | msg::SELECT_YESNO => {
                let battle = BATTLE_QUESTIONS.iter().find(|(d, _)| m["description"].as_u64() == Some(*d)).map(|(_, text)| *text);
                let entries = match (kind, m["cid"].as_u64(), m["description_cid"].as_u64(), battle) {
                    (msg::SELECT_EFFECTYN, cid, _, _) => vec![entry::TEXT | text::ACTIVATE, entry::CARD_NAME | cid.unwrap_or(0) as u32],
                    (_, _, _, Some(text)) => vec![entry::TEXT | text],
                    (_, _, Some(cid), _) => vec![entry::TEXT | text::USE, entry::CARD_NAME | cid as u32],
                    _ => vec![entry::TEXT | text::USE_NOW],
                };
                // A card's own question (its string as the description) under the client's line, through the DLL.
                let mut out = vec![];
                let mut entries = entries;
                if let Some(question) = m["description_text"].as_str().filter(|t| !t.is_empty() && entries.len() <= 2) {
                    let id = self.text(&mut out, question);
                    entries.extend([entry::LINE_BREAK, entry::TEXT | id]);
                }
                out.extend(message(&entries));
                out.push(Cmd::Dialog { kind: dialog::YES_NO, message: dialog::FROM_ENTRIES, param: 0 }.message());
                self.pending = Some(m.clone());
                Prompt::Client(out)
            }
            msg::SELECT_CARD | msg::SELECT_TRIBUTE => {
                let hint = if kind == msg::SELECT_TRIBUTE { hintmsg::RELEASE } else { std::mem::take(&mut self.hint) };
                if hint == hintmsg::ATTACKTARGET
                    && let Some(out) = self.target_on_field(m)
                {
                    return Prompt::Client(out);
                }
                let mut out = vec![];
                let text = self.hint_message(&mut out, hint);
                out.extend(self.card_list(m, text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            // Materials, one at a time: the card list until the engine lets the player finish and nothing is left.
            msg::SELECT_UNSELECT_CARD if m["select_cards"].as_array().is_some_and(|s| !s.is_empty()) => {
                let open = m["can_finish"] == true || m["can_cancel"] == true;
                let list = json!({ "selects": m["select_cards"], "min": if open { 0 } else { 1 }, "max": 1 });
                let mut out = vec![];
                let hint = std::mem::take(&mut self.hint);
                let text = self.hint_message(&mut out, hint);
                out.extend(self.card_list(&list, text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            // A sum (tributes or discards that make a total Level): the card list, as many picks as the bounds allow;
            // picks that don't make the sum bring the list back. Only when some picks can make it: a list nothing
            // satisfies would hold the player forever, so that one gets the automatic answer (and the core's
            // refusals end the duel, see `Repeats`).
            msg::SELECT_SUM if m["selects"].as_array().is_some_and(|s| !s.is_empty()) && sum_fits(m, &sum_choice(m)) => {
                // Kind 1 (min 0): any number of picks, confirmed with a right-click (dlg_cardlist.swf has no OK
                // button). A list of kind 0x27 with a larger max closes only at max picks (live: Machina Fortress's
                // discards held the player at two picks of six).
                let n = m["selects"].as_array().map_or(0, Vec::len);
                let (_, max) = sum_bounds(m);
                let list = json!({ "selects": m["selects"], "min": 0, "max": max.min(n).max(1) });
                // The engine's hint (Machina Fortress: "Select the card(s) in your hand you want to discard."), kept for
                // the list shown again, else the prompt table's 207 "Tribute the necessary monsters to match the
                // required number of Level.".
                let mut out = vec![];
                let hint = std::mem::take(&mut self.hint);
                let text = self.hint_message(&mut out, hint).or(self.sum_text).unwrap_or(text::SUM);
                self.sum_text = Some(text);
                out.extend(self.card_list(&list, Some(text)));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            msg::SELECT_POSITION => match self.position_dialog(m) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            msg::SELECT_OPTION => match option_list(m).or_else(|| self.worded_options(m)) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                // More options than the Select dialog's four labels, or one with no words: the card list, the option's
                // card once per option in the engine's order, rather than the first option for the player.
                None if m["options"].as_array().is_some_and(|o| o.len() > 1) => {
                    let card = |i: usize| m["option_cids"][i].as_u64().filter(|&c| c != 0)
                        .or_else(|| self.chaining.as_ref().and_then(|c| c["cid"].as_u64()))
                        .unwrap_or(0);
                    let selects: Vec<Json> = (0..m["options"].as_array().map_or(0, Vec::len))
                        .map(|i| json!({ "controller": m["player"], "cid": card(i) }))
                        .collect();
                    let out = self.card_list(&json!({ "selects": selects, "min": 1, "max": 1 }), Some(text::SELECT_EFFECT));
                    // What each option does, from the engine's card strings: the chat window, numbered as listed.
                    for (i, text) in m["option_texts"].as_array().into_iter().flatten().enumerate() {
                        if let Some(text) = text.as_str().filter(|t| !t.is_empty()) {
                            self.notes.push(format!("{}: {text}", i + 1));
                        }
                    }
                    self.option_cards = true;
                    self.picks.clear();
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            msg::SELECT_CHAIN => self.chain_chance(m),
            // An order: chain links triggered together (21; the first pick becomes chain link 1) or cards put back on
            // the Deck (25, Big Eye). The card list orders the picks itself (dlg_cardlist.swf: a badge per click, a
            // second click takes it back; kind 6 needs all of them), and each card's rank answers the engine.
            msg::SORT_CHAIN | msg::SORT_CARD if m["cards"].as_array().is_some_and(|c| c.len() >= 2) => {
                let n = m["cards"].as_array().map_or(0, Vec::len);
                let text = if kind == msg::SORT_CARD { text::SORT } else { text::SELECT_EFFECT };
                let out = self.card_list(&json!({ "selects": m["cards"], "min": n, "max": n }), Some(text));
                self.pending = Some(m.clone());
                self.picks.clear();
                Prompt::Client(out)
            }
            msg::ANNOUNCE_RACE | msg::ANNOUNCE_ATTRIB | msg::ANNOUNCE_NUMBER | msg::ANNOUNCE_CARD => match declaration(m, std::mem::take(&mut self.hint) == hintmsg::LVRANK) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None if kind == msg::ANNOUNCE_CARD => Prompt::Answer(self.cpu_card_name(m)),
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            // A zone an effect takes (Ground Collapse's, Senet Switch's move): the client's own prompt in an OK dialog, then
            // the zones as click targets (`zone_map`).
            msg::SELECT_DISFIELD if !free_zones(m).is_empty() => self.zone_prompt(m),
            // Where a summoned, Set or activated card goes, with auto card placing off and a choice to make.
            msg::SELECT_PLACE if !self.auto_place && free_zones(m).len() > m["count"].as_u64().unwrap_or(1).max(1) as usize => {
                self.zone_prompt(m)
            }
            // Where a summoned, Set or activated card goes: chosen for the player, centre out, as Konami's server did.
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// A zone prompt (MSG_SELECT_DISFIELD, or MSG_SELECT_PLACE with auto card placing off): the client's own prompt in
    /// an OK dialog, then the zones as click targets (`zone_map`).
    fn zone_prompt(&mut self, m: &Json) -> Prompt {
        let hint = std::mem::take(&mut self.hint);
        let zones = free_zones(m);
        let spells = zones.iter().all(|&(_, location, _)| location == loc::SZONE);
        let mut out = vec![];
        // The client's prompts are the zone-disabling effects' ("Select Monster Card Zones to render unusable."); a
        // placement is worded by the server through the connection DLL, in the client's terms.
        let text = if m["type"] == msg::SELECT_PLACE {
            self.text(&mut out, if spells { PLACE_SPELL_WORDS } else { PLACE_MONSTER_WORDS })
        } else if hint == hintmsg::TOZONE {
            text::ADJACENT_ZONE
        } else if spells {
            text::SPELL_ZONE
        } else {
            text::MONSTER_ZONE
        };
        out.extend(message(&[entry::TEXT | text]));
        out.push(Cmd::Dialog { kind: dialog::OK, message: dialog::FROM_ENTRIES, param: 0 }.message());
        self.pending = Some(m.clone());
        self.zone_picks = None;
        Prompt::Client(out)
    }

    /// The Select dialog for options the client has no label for, worded by the server through the connection DLL
    /// (`text`): each option the client's own label where it has one, else the engine's text for it (the card's
    /// string). None for more than the dialog's four labels or an option with no words. The title is the hint's, else
    /// "Select effect to activate." (276).
    fn worded_options(&mut self, m: &Json) -> Option<Vec<Message>> {
        let options = m["options"].as_array().filter(|o| !o.is_empty() && o.len() <= SELECT_MAX)?;
        let texts: Vec<Option<&str>> = (0..options.len()).map(|i| m["option_texts"][i].as_str().filter(|t| !t.is_empty())).collect();
        let own = |d: u64| OPTION_LABELS.iter().find(|(o, _, _)| *o == d).map(|&(_, label, _)| label)
            .or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| label));
        let codes: Vec<u64> = options.iter().map(|o| o.as_u64()).collect::<Option<_>>()?;
        if codes.iter().zip(&texts).any(|(&d, t)| own(d).is_none() && t.is_none()) {
            return None;
        }
        let mut out = vec![];
        let mut labels = vec![];
        for (&d, text) in codes.iter().zip(&texts) {
            let label = match own(d) {
                Some(label) => label,
                None => self.text(&mut out, &shortened(text.unwrap_or_default(), LABEL_CHARS)),
            };
            labels.push(label | dialog::LISTED);
        }
        let hint = std::mem::take(&mut self.hint);
        let title = self.hint_message(&mut out, hint).unwrap_or(text::SELECT_EFFECT);
        out.push(command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0]));
        out.extend(fours(cmd::LIST_VALUES, &labels));
        out.extend(message(&[entry::TEXT | title]));
        out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
        Some(out)
    }

    /// The CPU's card name declaration: the cards in the duel by turns, a new one each time it is asked.
    fn cpu_card_name(&mut self, m: &Json) -> Json {
        // A card the effect's filter allows (the sidecar's `fits`, the duel's own cards first).
        if let Some(cid) = m["fits"].as_array().and_then(|f| f.first()).and_then(Json::as_u64) {
            return json!({ "type": resp::CARD_NAME, "cid": cid });
        }
        let mut cids: Vec<u16> =
            self.sides.iter().flat_map(|s| s.zones.values()).flatten().filter_map(|c| self.cards.get(&c.row)).map(|c| c.cid).collect();
        cids.sort_unstable();
        cids.dedup();
        let cid = cids.get(self.declared % cids.len().max(1)).copied().unwrap_or(0);
        self.declared += 1;
        json!({ "type": resp::CARD_NAME, "cid": cid })
    }

    /// The position dialog for MSG_SELECT_POSITION {cid, positions}, with the card's image (any uid with its id) and
    /// name. None without an attack position: the first button is always attack.
    ///
    /// A token is asked its position before it exists (MSG_SELECT_PLACE, MSG_SELECT_POSITION, then its MSG_MOVE from
    /// nowhere): the dialog finds no card by uid and shows nothing to click, so the duel stops there (Gorz the
    /// Emissary of Darkness's token, live). It gets the uid and token card it will appear as now, in the card table at
    /// a free monster zone of its player (`token` puts it at its own zone).
    fn position_dialog(&mut self, m: &Json) -> Option<Vec<Message>> {
        let allowed = m["positions"].as_u64().unwrap_or(0);
        if allowed & (pos::FACEUP_ATTACK | pos::FACEDOWN_ATTACK) == 0 {
            return None;
        }
        let cid = m["cid"].as_u64().unwrap_or(0) as u16;
        let mut out = vec![];
        // The message names only the card's id, so the dialog shows a copy of it: one the client can see. The
        // opponent's hidden copies (hand, deck, extra deck) have row 0 there and draw nothing (live: Doom Shaman
        // summoned from an NPC's Graveyard while another sat in its hand). When every copy is hidden, the one shown
        // gets its row first: it is about to be summoned from where it is.
        let mut copies: Vec<(u32, u16, Place)> = vec![];
        for (p, side) in self.sides.iter().enumerate() {
            for (&zone, cards) in &side.zones {
                for (index, c) in cards.iter().enumerate() {
                    if cid != 0 && self.cards.get(&c.row).map(|s| s.cid) == Some(cid) {
                        copies.push((c.uid, c.row, Place { player: p as u32, zone, index, face_up: true, defense: false }));
                    }
                }
            }
        }
        copies.sort_by_key(|(uid, ..)| *uid);
        let seen = copies.iter().find(|(_, row, place)| self.shown_row(place, *row) != 0);
        let held = match seen.or(copies.first()) {
            Some(&(uid, row, ref place)) => {
                if self.shown_row(place, row) == 0 {
                    out.push(Cmd::Card { uid, open: false, row: row as u32, place: table_place(place.player, place.zone, place.index) }.message());
                }
                Some(uid)
            }
            None => None,
        };
        let (uid, name) = match (held, self.token_card_rows(m)) {
            (Some(uid), _) => (uid, cid),
            (None, Some(rows)) => {
                // The same one when the dialog opens again (after a look at a pile).
                let (uid, row) = match self.reserved_token {
                    Some((uid, row)) if rows.contains(&row) => (uid, row),
                    _ => self.next_token_card(&rows),
                };
                self.reserved_token = Some((uid, row));
                let player = self.player(&m["player"]);
                let side = &self.sides[player as usize];
                let zone = (0..5).find(|z| side.zones.get(z).is_none_or(Vec::is_empty)).unwrap_or(0);
                out.push(Cmd::Card { uid, open: false, row: row as u32, place: table_place(player, zone, 0) }.message());
                // A token without a Konami id is named by its client card's.
                (uid, self.cards.get(&row).map_or(cid, |s| s.cid))
            }
            (None, None) => (0, cid),
        };
        let face_up = if allowed & pos::FACEUP_DEFENSE != 0 { dialog::FACE_UP_DEFENSE } else { 0 };
        out.extend(message(&[entry::TEXT | text::POSITION, entry::CARD_NAME | name as u32]));
        out.push(Cmd::Dialog { kind: dialog::POSITION, message: dialog::FROM_ENTRIES, param: uid | face_up }.message());
        Some(out)
    }

    /// MSG_SELECT_CHAIN {spe_count, forced, hint_timing, selects}: the question shows the cards, names the chain's last
    /// link, and says how much it matters, so the player's Activation Message option decides whether it shows. A forced
    /// chain of several goes straight to the card list, of one needs no choice.
    fn chain_chance(&mut self, m: &Json) -> Prompt {
        let selects = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let forced = m["forced"] == true;
        let phase_window = self.phase_window;
        if selects.is_empty() {
            return Prompt::Answer(json!({ "type": resp::CHAIN, "index": null }));
        }
        if forced && selects.len() == 1 {
            return Prompt::Answer(json!({ "type": resp::CHAIN, "index": 0 }));
        }
        self.pending = Some(m.clone());
        self.picks.clear();
        self.chain_list = forced;
        if forced {
            return Prompt::Client(self.card_list(m, Some(text::SELECT_EFFECT)));
        }
        let rows: Vec<u32> = selects.iter().map(|loc| self.row_of(loc) as u32).collect();
        let mut out = vec![command(cmd::LIST_BEGIN, [rows.len() as u32, 0, dialog::WITH_CARDS, 0])];
        out.extend(fours(cmd::LIST_ROWS, &rows));
        let entries = match &self.chaining {
            Some(link) => {
                let spell_trap = link["location"].as_u64() == Some(loc::SZONE);
                let text = if spell_trap { text::CHAIN_CARD } else { text::CHAIN_EFFECT };
                vec![entry::TEXT | text, entry::CARD_NAME | link["cid"].as_u64().unwrap_or(0) as u32]
            }
            None => vec![entry::TEXT | text::ACTIVATE_CARD],
        };
        out.extend(message(&entries));
        let timing = m["hint_timing"].as_u64().unwrap_or(0) | m["hint_timing_other"].as_u64().unwrap_or(0);
        let level = if m["spe_count"].as_u64().unwrap_or(0) == 0 {
            dialog::LEVEL_UNFIT
        } else if self.chaining.is_some() || timing & !QUIET_TIMINGS != 0 || phase_window {
            // A phase's own window counts its optional triggers in spe_count (Stardust Dragon's return in the End
            // Phase): at the phase level the default Activation Message answered No for the player, and the chance
            // was gone (#53). The core doesn't say which cards are triggers, so the whole window shows.
            dialog::LEVEL_EVENT
        } else {
            dialog::LEVEL_PHASE
        };
        out.push(Cmd::Dialog { kind: dialog::EFFECT, message: dialog::FROM_ENTRIES, param: level }.message());
        Prompt::Client(out)
    }

    /// The card list for a card choice: its rows and marks, the message, then the dialog. The kind gives the picks OK
    /// needs: none for min 0, one for min 1, else exactly min (the list's max is lowered to it). A list holding a
    /// hidden card (a Set card has no row for the client) opens as the card-back kind with a table entry per card
    /// (flag set for the cards shown), so face-up cards keep their faces and Set cards are backs; no pick is needed
    /// there, so the list closing with nothing picked asks again when the choice is forced.
    fn card_list(&mut self, m: &Json, text: Option<u32>) -> Vec<Message> {
        let engine_order = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let mut order: Vec<usize> = (0..engine_order.len()).collect();
        order.sort_by_key(|&i| self.field_order(&engine_order[i], i));
        let shown: Vec<Json> = order.iter().map(|&i| engine_order[i].clone()).collect();
        let selects = &shown[..];
        self.list_order = order;
        let (min, max) = (m["min"].as_u64().unwrap_or(1) as u32, m["max"].as_u64().unwrap_or(1) as u32);
        // A choice the player may back out of (can_cancel, or a chain list that isn't forced) opens as kind 1: the list
        // (no OK button) closes on a right-click with the picks so far, none cancelling (dlg_cardlist.swf `onMouseUp`
        // with `_cancelable`, which 0x4C73A0 always sets).
        let cancelable = m["can_cancel"] == true || (m["type"] == msg::SELECT_CHAIN && m["forced"] != true);
        let min = if cancelable { 0 } else { min };
        let (kind, max) = match min {
            0 => (list::PICK_ANY, max),
            1 => (list::PICK_ONE, max),
            _ => (list::PICK_ALL, min),
        };
        let rows: Vec<u32> = selects.iter().map(|loc| self.list_row(loc)).collect();
        let mixed = rows.contains(&0) && min <= 1;
        self.mixed = mixed;
        let (kind, player) = if mixed { (list::PICK_MIXED, CPU) } else { (kind, CLIENT) };
        let marks: Vec<u32> = selects.iter().map(|loc| if self.player(&loc["controller"]) == CPU { list::OPPONENTS } else { 0 }).collect();
        let mut out = vec![];
        if mixed {
            for (i, &row) in rows.iter().enumerate() {
                let flag = if row != 0 { OPEN } else { 0 };
                out.push(Cmd::Card { uid: (STAGED_UID + i as u32) | flag, open: false, row, place: table_place(CPU, zone::REMOVED, i) }.message());
            }
            self.staged = Some(rows.len());
        }
        out.push(command(cmd::LIST_BEGIN, [selects.len() as u32, max, 0, 0]));
        out.extend(fours(cmd::LIST_ROWS, &rows));
        out.extend(fours(cmd::LIST_MARKS, &marks));
        // A list that needs several picks says how many (min == max here): the hint alone left a player of Magical Hats
        // with one card marked and no hint that a second one decides.
        let several = m["min"].as_u64().unwrap_or(1) >= 2 && max >= 2 && kind != list::PICK_MIXED;
        let entries: Vec<u32> = match (text, several) {
            (Some(text), true) => vec![entry::TEXT | text, entry::LINE_BREAK, entry::TEXT | text::SELECT_COUNT, entry::NUMBER | max],
            (None, true) => vec![entry::TEXT | text::SELECT_COUNT, entry::NUMBER | max],
            (Some(text), false) => vec![entry::TEXT | text],
            (None, false) => vec![],
        };
        if !entries.is_empty() {
            out.extend(message(&entries));
        }
        out.push(command(cmd::CARD_LIST, [player, kind, !entries.is_empty() as u32, 0]));
        out
    }

    /// The opponent's banished entries a mixed list overwrote (`n` of them), as they are: sent before anything else
    /// once it closes.
    fn unstage_n(&self, n: Option<usize>) -> Vec<Message> {
        let Some(n) = n else { return vec![] };
        let pile = self.sides[CPU as usize].zones.get(&zone::REMOVED).map_or(&[][..], Vec::as_slice);
        pile.iter()
            .enumerate()
            .take(n)
            .map(|(i, c)| {
                let row = self.shown_row(&Place { player: CPU, zone: zone::REMOVED, index: i, face_up: true, defense: false }, c.row);
                Cmd::Card { uid: c.uid, open: true, row, place: table_place(CPU, zone::REMOVED, i) }.message()
            })
            .collect()
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
        // A token has no Konami id: the one at its place.
        let has_cid = |c: &&Card| self.cards.get(&c.row).map(|s| s.cid) == Some(cid) || (cid == 0 && self.tokens.contains(&c.uid));
        let at_place = self.engine_place(loc).and_then(|p| {
            let zone = self.sides[p.player as usize].zones.get(&p.zone)?;
            if p.zone == zone::DECK { zone.iter().find(has_cid) } else { zone.get(p.index).filter(has_cid) }.map(|c| c.row)
        });
        at_place.or_else(|| self.cards.iter().find(|(_, s)| s.cid == cid).map(|(&row, _)| row)).unwrap_or(0)
    }

    /// A pick in the card list (duelRequestName 7 {index: the list position}), kept as the engine's index.
    pub fn pick(&mut self, index: u32) {
        // A list holds a few dozen entries: more picks than that are a client making the server keep them.
        if self.picks.len() < MAX_PICKS {
            let index = index as usize;
            self.picks.push(self.list_order.get(index).copied().unwrap_or(index));
        }
    }

    /// Where a card list puts a card: as the field lies from the player's seat, top to bottom (the opponent's Spell &
    /// Trap row, their monsters, the player's monsters, the player's Spell & Trap row), each row left to right (the
    /// opponent's zones are mirrored: their zone 0 is on the player's right), the Field Spells at their rows' ends;
    /// then the cards off the field (hand, piles) in the engine's order. The engine's order (the player's cards first,
    /// each side by zone number) didn't match the field, so two Set cards in a list couldn't be told apart.
    fn field_order(&self, loc: &Json, engine: usize) -> (u32, usize) {
        let Some(place) = self.engine_place(loc).filter(|p| p.zone <= zone::FIELD_SPELL) else { return (4, engine) };
        let theirs = place.player == CPU;
        let (row, x) = match place.zone {
            zone::FIELD_SPELL => (if theirs { 0 } else { 3 }, 5),
            z if z >= 5 => (if theirs { 0 } else { 3 }, if theirs { 4 - (z - 5) } else { z - 5 }),
            z => (if theirs { 1 } else { 2 }, if theirs { 4 - z } else { z }),
        };
        (row, x as usize)
    }

    /// A dialog closed (duelRequestName 4 {DLGResult}): the engine response for the pending question (the button: 1
    /// yes, 0 attack / 1 defense), option (its index) or card choice (`count` picks), or for a chain chance's Yes with
    /// several cards, the card list. No pick cancels when the engine allows it; picks that don't fit the choice get
    /// the default.
    pub fn dialog_result(&mut self, count: u32) -> Option<Prompt> {
        let asked = self.pending.clone();
        let prompt = self.dialog_result_for(count);
        if let (Some(m), Some(Prompt::Answer(a))) = (&asked, &prompt)
            && m["type"] == msg::SELECT_OPTION
            && a["type"] == resp::OPTION
        {
            let called = a["index"].as_u64().and_then(|i| m["options"][i as usize].as_u64());
            self.coin_call = match called { Some(CALL_HEADS) => Some(true), Some(CALL_TAILS) => Some(false), _ => self.coin_call };
        }
        prompt.map(|p| self.with_ids(p))
    }

    fn dialog_result_for(&mut self, count: u32) -> Option<Prompt> {
        if let Some((kind, action, candidates)) = self.choosing.take() {
            // Which card of the pile: the pick answers the engine; none goes back to the prompt.
            let pick = (count == 1).then(|| self.picks.first().and_then(|&i| candidates.get(i).copied())).flatten();
            self.picks.clear();
            if let Some(index) = pick {
                self.pending = None;
                return Some(Prompt::Answer(json!({ "type": kind, "action": action, "index": index })));
            }
            let m = self.pending.clone()?;
            return Some(self.prompt(&m));
        }
        if std::mem::take(&mut self.viewing) {
            // A pile's view closed: the prompt it was opened from, again.
            self.picks.clear();
            let m = self.pending.clone()?;
            return Some(self.prompt(&m));
        }
        if let Some(response) = self.direct.take() {
            // Yes attacks; No goes back to the battle commands.
            let m = self.pending.take()?;
            return Some(if count == 1 { Prompt::Answer(response) } else { self.prompt(&m) });
        }
        let kind = self.pending.as_ref()?["type"].as_u64()?;
        if matches!(kind, msg::SELECT_DISFIELD | msg::SELECT_PLACE) && self.zone_picks.is_none() {
            self.zone_picks = Some(vec![]);
            let m = self.pending.clone()?;
            return Some(Prompt::Client(self.zone_map(&m)));
        }
        const DIALOGS: [u64; 15] = [
            msg::SORT_CHAIN,
            msg::SORT_CARD,
            msg::SELECT_SUM,
            msg::SELECT_CARD,
            msg::SELECT_UNSELECT_CARD,
            msg::SELECT_TRIBUTE,
            msg::SELECT_EFFECTYN,
            msg::SELECT_YESNO,
            msg::SELECT_POSITION,
            msg::SELECT_OPTION,
            msg::SELECT_CHAIN,
            msg::ANNOUNCE_RACE,
            msg::ANNOUNCE_ATTRIB,
            msg::ANNOUNCE_NUMBER,
            msg::ANNOUNCE_CARD,
        ];
        if !DIALOGS.contains(&kind) {
            return None;
        }
        let m = self.pending.take()?;
        let selects = m["selects"].as_array().or(m["select_cards"].as_array()).map_or(0, Vec::len);
        let answer = match kind {
            msg::SELECT_EFFECTYN => json!({ "type": resp::EFFECTYN, "yes": count == 1 }),
            msg::SELECT_YESNO => json!({ "type": resp::YESNO, "yes": count == 1 }),
            msg::SELECT_POSITION => {
                let allowed = m["positions"].as_u64().unwrap_or(0);
                let position = match count {
                    0 if allowed & pos::FACEUP_ATTACK != 0 => pos::FACEUP_ATTACK,
                    0 => pos::FACEDOWN_ATTACK,
                    _ if allowed & pos::FACEUP_DEFENSE != 0 => pos::FACEUP_DEFENSE,
                    _ => pos::FACEDOWN_DEFENSE,
                };
                match allowed & position {
                    0 => cpu_answer(kind, &m),
                    _ => json!({ "type": resp::POSITION, "position": position }),
                }
            }
            msg::SELECT_OPTION if std::mem::take(&mut self.option_cards) => match std::mem::take(&mut self.picks)[..] {
                [index] if index < m["options"].as_array().map_or(0, Vec::len) => json!({ "type": resp::OPTION, "index": index }),
                _ => cpu_answer(kind, &m),
            },
            msg::SELECT_OPTION if (count as usize) < m["options"].as_array().map_or(0, Vec::len) => {
                json!({ "type": resp::OPTION, "index": count })
            }
            msg::SELECT_CHAIN if !self.chain_list && count == 1 && selects > 1 => {
                self.chain_list = true;
                self.picks.clear();
                let out = self.card_list(&m, Some(text::SELECT_EFFECT));
                self.pending = Some(m);
                return Some(Prompt::Client(out));
            }
            msg::SELECT_CHAIN if !self.chain_list => {
                let index = if count == 1 { json!(0) } else { Json::Null };
                json!({ "type": resp::CHAIN, "index": index })
            }
            msg::SELECT_CHAIN => match std::mem::take(&mut self.picks)[..] {
                [index] if index < selects => json!({ "type": resp::CHAIN, "index": index }),
                _ if m["forced"] == true => cpu_answer(kind, &m),
                _ => json!({ "type": resp::CHAIN, "index": null }),
            },
            // A mixed list needs no picks to close: a forced choice with nothing picked is asked again.
            msg::SELECT_CARD | msg::SELECT_TRIBUTE if self.mixed && self.picks.is_empty() && m["can_cancel"] != true && m["min"].as_u64().unwrap_or(1) >= 1 => {
                return Some(self.prompt(&m));
            }
            msg::SELECT_CARD | msg::SELECT_TRIBUTE => self.card_choice(&m, count),
            msg::SELECT_SUM => {
                let picks = std::mem::take(&mut self.picks);
                if !sum_fits(&m, &picks) {
                    return Some(self.prompt(&m));
                }
                self.sum_text = None;
                json!({ "type": resp::SUM, "indicies": picks })
            }
            msg::SELECT_UNSELECT_CARD => match std::mem::take(&mut self.picks)[..] {
                [index] if index < selects => json!({ "type": resp::UNSELECT_CARD, "index": index }),
                _ if m["can_finish"] == true || m["can_cancel"] == true => json!({ "type": resp::UNSELECT_CARD, "index": null }),
                _ => cpu_answer(kind, &m),
            },
            msg::ANNOUNCE_RACE => match TYPES.get((count as usize).wrapping_sub(1)) {
                Some(&race) if m["available"].as_u64().unwrap_or(0) & race != 0 => json!({ "type": resp::RACE, "races": [race] }),
                _ => cpu_answer(kind, &m),
            },
            msg::ANNOUNCE_ATTRIB => {
                let picked: Vec<u64> = ATTRIBUTES.iter().enumerate().filter(|(i, _)| count >> (i + 1) & 1 != 0).map(|(_, &a)| a).collect();
                let allowed = m["available"].as_u64().unwrap_or(0);
                match picked.len() as u64 == m["count"].as_u64().unwrap_or(1) && picked.iter().all(|a| allowed & a != 0) {
                    true => json!({ "type": resp::ATTRIB, "attributes": picked }),
                    false => cpu_answer(kind, &m),
                }
            }
            msg::ANNOUNCE_NUMBER => {
                let options: Vec<u64> = m["options"].as_array().into_iter().flatten().filter_map(Json::as_u64).collect();
                let unit = number_unit(&options).map_or(1, |(unit, _)| unit);
                let value = count as u64 * unit;
                // The smallest option at least as large (the dialog's steps may pass between uneven options), else the
                // largest. By value: the engine may list them largest first (Card Trooper's 3, 2, 1).
                let index = options.iter().enumerate().filter(|(_, o)| **o >= value).min_by_key(|(_, o)| **o)
                    .or_else(|| options.iter().enumerate().max_by_key(|(_, o)| **o))
                    .map_or(0, |(i, _)| i);
                json!({ "type": resp::NUMBER, "value": index })
            }
            msg::ANNOUNCE_CARD => json!({ "type": resp::CARD_NAME, "cid": count }),
            // The clicks in order: card picks[k] gets rank k (the core reads byte i as card i's rank). Anything but each
            // card once keeps the engine's own order.
            msg::SORT_CHAIN | msg::SORT_CARD => {
                let picks = std::mem::take(&mut self.picks);
                let n = m["cards"].as_array().map_or(0, Vec::len);
                let mut order = vec![usize::MAX; n];
                let whole = picks.len() == n && picks.iter().enumerate().all(|(rank, &i)| {
                    i < n && order[i] == usize::MAX && { order[i] = rank; true }
                });
                if whole { json!({ "type": resp::SORT, "order": order }) } else { cpu_answer(kind, &m) }
            }
            _ => cpu_answer(kind, &m),
        };
        Some(Prompt::Answer(answer))
    }

    /// The response for a closed card list of MSG_SELECT_CARD or MSG_SELECT_TRIBUTE.
    fn card_choice(&mut self, m: &Json, count: u32) -> Json {
        let kind = m["type"].as_u64().unwrap_or(0);
        let response = if kind == msg::SELECT_TRIBUTE { resp::TRIBUTE } else { resp::CARD };
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
                if list == "pos_changes" && bit != self.position_change(place) {
                    continue;
                }
                // The client looks a pile's commands up at index 0 only (0x50B510, 0x5204E0).
                let place = if place >> 8 & 0xFF >= zone::EXTRA { place & 0xFFFF } else { place };
                match bits.iter_mut().find(|(p, _)| *p == place) {
                    Some((_, b)) => *b |= 1 << bit,
                    None => bits.push((place, 1 << bit)),
                }
            }
        }
        bits
    }

    /// The menu entry a monster's position change shows (`table_place`): Flip Summon face down, to Attack Position in
    /// defense, to Defence Position in attack (live: an attack position monster offered both positions).
    fn position_change(&self, place: u32) -> u32 {
        match self.positions.get(&(place & 1, place >> 16 & 0xFF)) {
            Some((false, _)) => 5,
            Some((true, true)) => 8,
            _ => 9,
        }
    }

    /// The card-table place of an engine card location: hand (0xB, index), monster zone (0, slot), Spell/Trap
    /// zone (5, slot), and the extra deck, Graveyard and banished cards (0xC, 0xE, 0xF) at the index of a card with
    /// the location's `cid` (the engine's extra deck order isn't ours).
    fn place(&self, loc: &Json) -> Option<u32> {
        let p = self.player(&loc["controller"]);
        let seq = loc["sequence"].as_u64()? as usize;
        let zone = match loc["location"].as_u64()? {
            loc::HAND => zone::HAND,
            loc::MZONE => 0,
            loc::SZONE => 5,
            loc::EXTRA => zone::EXTRA,
            loc::GRAVE => zone::GRAVE,
            loc::REMOVED => zone::REMOVED,
            _ => return None,
        };
        if zone >= zone::EXTRA {
            let cid = |c: &Card| self.cards.get(&c.row).map(|s| s.cid as u64);
            let cards = self.sides[p as usize].zones.get(&zone)?;
            let index = match cards.get(seq) {
                Some(c) if cid(c) == loc["cid"].as_u64() => seq,
                _ => cards.iter().position(|c| cid(c) == loc["cid"].as_u64())?,
            };
            return Some(table_place(p, zone, index));
        }
        Some(table_place(p, zone, seq))
    }

    /// The engine response for a phase button (0 DP .. 5 EP), if the pending prompt allows that phase.
    pub fn phase(&mut self, button: u32) -> Option<Json> {
        let m = self.pending.as_ref()?;
        let battle = m["type"].as_u64() == Some(msg::SELECT_BATTLECMD);
        let (kind, action) = match (battle, button as u8) {
            (false, flag::BUTTON_BP) if m["to_bp"] == true => (resp::IDLECMD, idle::TO_BP),
            (false, flag::BUTTON_EP) if m["to_ep"] == true => (resp::IDLECMD, idle::TO_EP),
            (true, flag::BUTTON_M2) if m["to_m2"] == true => (resp::BATTLECMD, battle::TO_M2),
            (true, flag::BUTTON_EP) if m["to_ep"] == true => (resp::BATTLECMD, battle::TO_EP),
            _ => return None,
        };
        self.pending = None;
        Some(json!({ "type": kind, "action": action, "index": null }))
    }

    /// What the client's card command leads to: an attack with no monster to target asks "Attack your opponent
    /// directly?" first (225, the original's dialog: EDOPro asks only when the attacker could also target a monster;
    /// live, an attack on an empty field just went through), anything else answers the engine.
    pub fn command(&mut self, cmd: CardCommand) -> Option<Prompt> {
        let prompt = self.command_for(cmd);
        prompt.map(|p| self.with_ids(p))
    }

    /// A client prompt's card table entries with their Konami ids (see [`with_card_ids`]).
    fn with_ids(&self, p: Prompt) -> Prompt {
        match p {
            Prompt::Client(mut out) => {
                with_card_ids(&self.cards, &mut out);
                Prompt::Client(out)
            }
            answer => answer,
        }
    }

    fn command_for(&mut self, cmd: CardCommand) -> Option<Prompt> {
        let m = self.pending.clone()?;
        if self.field_target {
            return self.target_click(&m, cmd);
        }
        if (m["type"] == msg::SELECT_DISFIELD || m["type"] == msg::SELECT_PLACE) && self.zone_picks.is_some() {
            return Some(self.zone_click(&m, cmd));
        }
        if cmd.command == menu::CONFIRM
            && let Some(out) = self.pile_view(cmd.player, cmd.zone)
        {
            self.viewing = true;
            return Some(Prompt::Client(out));
        }
        if cmd.zone >= zone::EXTRA && cmd.zone != zone::DECK {
            return self.pile_command(&m, cmd);
        }
        // A card that offers this command more than once (two effects it can activate now): the card list asks which,
        // the card once per effect in the engine's order.
        if let Some((list, kind, action, indices)) = self.listed(&cmd).filter(|l| l.3.len() > 1) {
            let selects: Vec<Json> = indices.iter().map(|&i| m[list][i].clone()).collect();
            let out = self.card_list(&json!({ "selects": selects, "min": 0, "max": 1 }), Some(text::SELECT_EFFECT));
            self.choosing = Some((kind, action, indices));
            self.picks.clear();
            return Some(Prompt::Client(out));
        }
        let response = self.answer(cmd)?;
        self.chose(&m, &response);
        let attack = &m["attacks"][response["index"].as_u64().unwrap_or(u64::MAX) as usize];
        let opponent = 1 - self.player(&attack["controller"]) as usize;
        let no_target = (0..5).all(|zone| self.sides[opponent].zones.get(&zone).is_none_or(Vec::is_empty));
        if response["type"] != resp::BATTLECMD || response["action"] != battle::ATTACK || attack["can_direct"] != true || !no_target {
            return Some(Prompt::Answer(response));
        }
        self.pending = Some(m);
        self.direct = Some(response);
        let mut out = message(&[entry::TEXT | text::DIRECT]);
        out.push(Cmd::Dialog { kind: dialog::YES_NO, message: dialog::FROM_ENTRIES, param: 0 }.message());
        Some(Prompt::Client(out))
    }

    /// The client's answer to a prompt, as the engine gets it: an attack names the attacker its target is aimed from.
    /// The duel check calls it with the self-played answers, which reach the engine without a card command.
    pub fn answered(&mut self, prompt: &Json, response: &Json) {
        if self.player(&prompt["player"]) == CLIENT {
            self.chose(prompt, response);
        }
    }

    fn chose(&mut self, m: &Json, response: &Json) {
        if m["type"] == msg::SELECT_BATTLECMD && response["type"] == resp::BATTLECMD && response["action"] == battle::ATTACK {
            let attack = &m["attacks"][response["index"].as_u64().unwrap_or(u64::MAX) as usize];
            self.attacker = self.engine_place(attack).filter(|p| p.zone < 5).map(|p| (p.player, p.zone));
        }
    }

    /// The target of the client's own attack, picked on the field as the original did (#51): the battle's start, the
    /// attack with no target yet, BattleSelect's aiming state, Confirm on each target and the battle input wait, with
    /// the field's right-click for taking the attack back. None (the card list instead) without an attacker or with a
    /// target that is not a monster on the field.
    fn target_on_field(&mut self, m: &Json) -> Option<Vec<Message>> {
        let (player, zone) = self.attacker?;
        let mut map = vec![];
        for card in m["selects"].as_array()? {
            let place = self.place(card).filter(|p| p >> 8 & 0xFF == 0)?;
            map.push((place, 1 << menu::CONFIRM));
        }
        self.field_target = true;
        self.pending = Some(m.clone());
        Some(vec![
            Cmd::BattleInit.message(),
            Cmd::BattleAttack { player, zone, target: Target::NotYet { player: 1 - player } }.message(),
            Cmd::BattleSelect.message(),
            command_data(&map, &[(flag::RIGHT_CLICK, 1)]),
            command(cmd::INPUT, [mode::BATTLE, CLIENT, 0, 0]),
        ])
    }

    /// A click while an attack target is picked on the field: Confirm on a target answers, anything else waits on.
    fn target_click(&mut self, m: &Json, cmd: CardCommand) -> Option<Prompt> {
        // A field card comes from its own slot, index 0 (see `listed`); targets are keyed at (zone 0, index slot).
        let slot = if cmd.index == 0 && cmd.zone < 5 { cmd.zone as usize } else { return None };
        let place = table_place(cmd.player, 0, slot);
        let index = m["selects"].as_array()?.iter().position(|c| self.place(c) == Some(place)).filter(|_| cmd.command == menu::CONFIRM)?;
        self.field_target = false;
        self.pending = None;
        Some(Prompt::Answer(json!({ "type": resp::CARD, "indicies": [index] })))
    }

    /// The field's right-click (duelRequestName 5): while an attack target is picked on the field, the attack taken
    /// back when the engine allows it.
    pub fn finish(&mut self) -> Option<Prompt> {
        let m = self.pending.as_ref().filter(|_| self.field_target)?;
        if m["can_cancel"] != true {
            return None;
        }
        self.field_target = false;
        self.taken_back = true;
        self.pending = None;
        Some(Prompt::Answer(json!({ "type": resp::CARD, "indicies": null })))
    }

    /// A command on a pile (sent at index 0, where the client found it): the one card of the pile it fits, or with
    /// several, the card list asks which (the original's "Select the monster(s) to Special Summon from your Extra
    /// Deck.", 26).
    fn pile_command(&mut self, m: &Json, cmd: CardCommand) -> Option<Prompt> {
        let (lists, kind): (Lists, u64) = match m["type"].as_u64() {
            Some(msg::SELECT_BATTLECMD) => (&BATTLE_LISTS, resp::BATTLECMD),
            _ => (&IDLE_LISTS, resp::IDLECMD),
        };
        let &(list, _, action) = lists.iter().find(|(_, bit, _)| *bit == cmd.command)?;
        let pile = table_place(cmd.player, cmd.zone, 0);
        let candidates: Vec<(usize, &Json)> = m[list]
            .as_array()?
            .iter()
            .enumerate()
            .filter(|(_, loc)| self.place(loc).is_some_and(|p| p & 0xFFFF == pile))
            .collect();
        match candidates[..] {
            [] => None,
            [(index, _)] => {
                self.pending = None;
                Some(Prompt::Answer(json!({ "type": kind, "action": action, "index": index })))
            }
            _ => {
                let text = match (cmd.command, cmd.zone) {
                    (2, zone::EXTRA) => text::SPSUMMON_EXTRA,
                    (2, _) => text::SPSUMMON,
                    _ => text::SELECT_EFFECT,
                };
                let selects: Vec<Json> = candidates.iter().map(|(_, loc)| (*loc).clone()).collect();
                let out = self.card_list(&json!({ "selects": selects, "min": 0, "max": 1 }), Some(text));
                self.choosing = Some((kind, action, candidates.iter().map(|(i, _)| *i).collect()));
                self.picks.clear();
                Some(Prompt::Client(out))
            }
        }
    }

    /// Confirm (bit 11) at index 0 of each pile that can be looked through: both players' Graveyards and banished
    /// piles and the player's own extra deck, when not empty. The field highlights a pile of either player whose
    /// bits there have Activate Effect or Confirm (0x808, 0x50A690).
    fn add_pile_views(&self, map: &mut Vec<(u32, u32)>) {
        for (player, zone) in [(CLIENT, zone::GRAVE), (CPU, zone::GRAVE), (CLIENT, zone::REMOVED), (CPU, zone::REMOVED), (CLIENT, zone::EXTRA)] {
            if self.sides[player as usize].zones.get(&zone).is_none_or(Vec::is_empty) {
                continue;
            }
            let place = table_place(player, zone, 0);
            match map.iter_mut().find(|(p, _)| *p == place) {
                Some((_, bits)) => *bits |= 1 << menu::CONFIRM,
                None => map.push((place, 1 << menu::CONFIRM)),
            }
        }
    }

    /// A pile's view: its cards from the bottom in a card list with no picks (max 0: dlg_cardlist.swf ignores card
    /// clicks, and a right-click closes it), under the pile's title.
    fn pile_view(&self, player: u32, zone: u32) -> Option<Vec<Message>> {
        let title = PILE_TITLES.iter().find(|(z, _)| *z == zone)?.1 + (player != CLIENT) as u32;
        if zone == zone::EXTRA && player != CLIENT {
            return None;
        }
        let cards = self.sides.get(player as usize)?.zones.get(&zone).filter(|c| !c.is_empty())?;
        let rows: Vec<u32> = cards
            .iter()
            .enumerate()
            .map(|(index, c)| self.shown_row(&Place { player, zone, index, face_up: true, defense: false }, c.row))
            .collect();
        let mark = if player == CLIENT { 0 } else { list::OPPONENTS };
        let mut out = vec![command(cmd::LIST_BEGIN, [rows.len() as u32, 0, 0, 0])];
        out.extend(fours(cmd::LIST_ROWS, &rows));
        out.extend(fours(cmd::LIST_MARKS, &vec![mark; rows.len()]));
        out.extend(message(&[entry::TEXT | title]));
        out.push(command(cmd::CARD_LIST, [CLIENT, list::PICK_ANY, 1, 0]));
        Some(out)
    }

    /// The duel check's round trip for the pending idle or battle prompt: every answer the engine offers that the
    /// client can't give through its menus, described. The client's side is played on copies of the translator: each
    /// command of the command map as the client sends it (a field card from its own slot, `answer`), the card list a
    /// pile's command opens and each of its picks, the direct attack's Yes, and the phase buttons. Empty when every
    /// option is reachable (or nothing such is pending).
    pub fn unreachable_answers(&self) -> Vec<String> {
        let Some(m) = self.pending.as_ref() else { return vec![] };
        let (lists, kind, phases): (Lists, u64, &[(&str, u64)]) = match m["type"].as_u64() {
            Some(msg::SELECT_IDLECMD) => (&IDLE_LISTS, resp::IDLECMD, &[("to_bp", idle::TO_BP), ("to_ep", idle::TO_EP)]),
            Some(msg::SELECT_BATTLECMD) => (&BATTLE_LISTS, resp::BATTLECMD, &[("to_m2", battle::TO_M2), ("to_ep", battle::TO_EP)]),
            Some(msg::SELECT_DISFIELD) if self.player(&m["player"]) == CLIENT => return self.unreachable_zones(m),
            Some(msg::SELECT_CARD) if self.field_target => return self.unreachable_targets(m),
            _ => return self.unreachable_choices(m),
        };
        let mut reachable: Vec<Json> = vec![];
        let mut reach = |p: Option<Prompt>, t: &Translator| match p {
            Some(Prompt::Answer(r)) => reachable.push(r),
            Some(Prompt::Client(_)) if t.choosing.is_some() => {
                for i in 0..t.choosing.as_ref().map_or(0, |c| c.2.len()) {
                    let mut pick = t.clone();
                    pick.pick(i as u32);
                    if let Some(Prompt::Answer(r)) = pick.dialog_result(1) {
                        reachable.push(r);
                    }
                }
            }
            Some(Prompt::Client(_)) if t.direct.is_some() => {
                if let Some(Prompt::Answer(r)) = t.clone().dialog_result(1) {
                    reachable.push(r);
                }
            }
            _ => {}
        };
        for (place, bits) in self.command_map(m, lists) {
            let (player, zone, index) = (place & 0xFF, place >> 8 & 0xFF, place >> 16 & 0xFF);
            // As the client sends it: a field card from its own slot, index 0 (see `answer`).
            let (zone, index) = match (zone, index) {
                (0, i @ 1..=4) => (i, 0),
                (5, 5) => (zone::FIELD_SPELL, 0),
                (5, i @ 0..=4) => (5 + i, 0),
                other => other,
            };
            for bit in (0..32).filter(|b| bits >> b & 1 != 0) {
                let mut t = self.clone();
                let p = t.command(CardCommand { command: bit, player, zone, index });
                reach(p, &t);
            }
        }
        for button in [flag::BUTTON_BP, flag::BUTTON_M2, flag::BUTTON_EP] {
            if let Some(r) = self.clone().phase(button as u32) {
                reachable.push(r);
            }
        }
        let mut offered: Vec<(Json, String)> = vec![];
        let mut seen_lists = vec![];
        for &(list, _, action) in lists {
            if seen_lists.contains(&list) {
                continue;
            }
            seen_lists.push(list);
            for (i, loc) in m[list].as_array().into_iter().flatten().enumerate() {
                let what = format!("{list}[{i}] (card {} at location {} sequence {}, effect {})", loc["cid"], loc["location"], loc["sequence"], loc["description"]);
                offered.push((json!({ "type": kind, "action": action, "index": i }), what));
            }
        }
        for &(flag, action) in phases {
            if m[flag] == true {
                offered.push((json!({ "type": kind, "action": action, "index": null }), flag.to_string()));
            }
        }
        offered.into_iter().filter(|(answer, _)| !reachable.contains(answer)).map(|(_, what)| what).collect()
    }

    /// [`unreachable_answers`] for an attack target picked on the field: each target's Confirm, and the right-click.
    fn unreachable_targets(&self, m: &Json) -> Vec<String> {
        let mut missing = vec![];
        for (i, card) in m["selects"].as_array().into_iter().flatten().enumerate() {
            let Some(place) = self.place(card) else { continue };
            let click = CardCommand { command: menu::CONFIRM, player: place & 0xFF, zone: place >> 16 & 0xFF, index: 0 };
            if self.clone().command(click) != Some(Prompt::Answer(json!({ "type": resp::CARD, "indicies": [i] }))) {
                missing.push(format!("selects[{i}] (attack target on the field)"));
            }
        }
        if m["can_cancel"] == true && self.clone().finish().is_none() {
            missing.push("taking the attack back (right-click)".into());
        }
        missing
    }

    /// [`unreachable_answers`] for a zone choice: each free zone through the OK dialog and its click (and, for several,
    /// the first ones in order).
    fn unreachable_zones(&self, m: &Json) -> Vec<String> {
        let zones = free_zones(m);
        let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
        let click = |t: &mut Translator, (player, location, sequence): (u64, u64, u64)| {
            let zone = if location == loc::MZONE { sequence as u32 } else { 5 + sequence as u32 };
            t.command(CardCommand { command: ZONE_PICK_MONSTER, player: t.player(&json!(player)), zone, index: 0 })
        };
        let place = |(p, l, s): (u64, u64, u64)| json!({ "player": p, "location": l, "sequence": s });
        let mut missing = vec![];
        let picks: Vec<Vec<(u64, u64, u64)>> = if count == 1 { zones.iter().map(|&z| vec![z]).collect() } else { vec![zones.iter().copied().take(count).collect()] };
        for pick in picks {
            let mut t = self.clone();
            t.dialog_result(0);
            let mut last = None;
            for &z in &pick {
                last = click(&mut t, z);
            }
            let expected = json!({ "type": resp::DISFIELD, "places": pick.iter().map(|&z| place(z)).collect::<Vec<_>>() });
            if last != Some(Prompt::Answer(expected)) {
                missing.push(format!("zones {pick:?}"));
            }
        }
        missing
    }

    /// [`unreachable_answers`] for the other windows: each card of a card choice (one pick, when one may be picked),
    /// a chain chance's Yes for each card and its No, a question's Yes and No, and each option.
    fn unreachable_choices(&self, m: &Json) -> Vec<String> {
        let kind = m["type"].as_u64().unwrap_or(0);
        let len = |k: &str| m[k].as_array().map_or(0, Vec::len);
        // What, the clicks (each: the picks, then the dialog's result), the answer expected.
        type Case = (String, Vec<(Vec<u32>, u32)>, Json);
        let mut cases: Vec<Case> = vec![];
        match kind {
            msg::SELECT_CARD | msg::SELECT_TRIBUTE => {
                let response = if kind == msg::SELECT_TRIBUTE { resp::TRIBUTE } else { resp::CARD };
                let (min, max) = (m["min"].as_u64().unwrap_or(1), m["max"].as_u64().unwrap_or(1));
                if min <= 1 && max >= 1 {
                    for i in 0..len("selects") as u32 {
                        cases.push((format!("selects[{i}]"), vec![(vec![i], 1)], json!({ "type": response, "indicies": [i] })));
                    }
                } else {
                    let picks: Vec<u32> = (0..min as u32).collect();
                    cases.push((format!("the first {min} of selects"), vec![(picks.clone(), min as u32)], json!({ "type": response, "indicies": picks })));
                }
                // Backing out (a right-click with nothing picked) when the engine allows it.
                if m["can_cancel"] == true {
                    cases.push(("cancelling".into(), vec![(vec![], 0)], json!({ "type": response, "indicies": null })));
                }
            }
            msg::SELECT_UNSELECT_CARD => {
                for i in 0..len("select_cards") as u32 {
                    cases.push((format!("select_cards[{i}]"), vec![(vec![i], 1)], json!({ "type": resp::UNSELECT_CARD, "index": i })));
                }
                if m["can_cancel"] == true || m["can_finish"] == true {
                    let what = if m["can_cancel"] == true { "cancelling" } else { "finishing" };
                    cases.push((what.into(), vec![(vec![], 0)], json!({ "type": resp::UNSELECT_CARD, "index": null })));
                }
            }
            msg::SELECT_CHAIN => {
                let n = len("selects") as u32;
                for i in 0..n {
                    // Yes, then the card list when there are several (or a forced chain's list right away).
                    let clicks = match (m["forced"] == true, n) {
                        (true, _) => vec![(vec![i], 1)],
                        (false, 1) => vec![(vec![], 1)],
                        (false, _) => vec![(vec![], 1), (vec![i], 1)],
                    };
                    cases.push((format!("selects[{i}] (chain)"), clicks, json!({ "type": resp::CHAIN, "index": i })));
                }
                if m["forced"] != true {
                    cases.push(("passing the chain".into(), vec![(vec![], 0)], json!({ "type": resp::CHAIN, "index": null })));
                }
            }
            msg::SELECT_SUM => {
                let picks: Vec<u32> = sum_choice(m).into_iter().map(|i| i as u32).collect();
                let n = picks.len() as u32;
                cases.push((format!("the sum {picks:?}"), vec![(picks.clone(), n)], json!({ "type": resp::SUM, "indicies": picks })));
            }
            msg::SELECT_EFFECTYN | msg::SELECT_YESNO => {
                let response = if kind == msg::SELECT_EFFECTYN { resp::EFFECTYN } else { resp::YESNO };
                cases.push(("yes".into(), vec![(vec![], 1)], json!({ "type": response, "yes": true })));
                cases.push(("no".into(), vec![(vec![], 0)], json!({ "type": response, "yes": false })));
            }
            msg::SELECT_OPTION => {
                for i in 0..len("options") as u32 {
                    let clicks = if self.option_cards { vec![(vec![i], 1)] } else { vec![(vec![], i)] };
                    cases.push((format!("options[{i}]"), clicks, json!({ "type": resp::OPTION, "index": i })));
                }
            }
            // Each allowed position: the dialog's button 0 (attack) or 1 (defense).
            msg::SELECT_POSITION => {
                let allowed = m["positions"].as_u64().unwrap_or(0);
                for (bit, button) in [(pos::FACEUP_ATTACK, 0), (pos::FACEDOWN_ATTACK, 0), (pos::FACEUP_DEFENSE, 1), (pos::FACEDOWN_DEFENSE, 1)] {
                    if allowed & bit != 0 {
                        cases.push((format!("position {bit}"), vec![(vec![], button)], json!({ "type": resp::POSITION, "position": bit })));
                    }
                }
            }
            // Each allowed Type and Attribute, one declared (the client's number, or its bit for an attribute).
            msg::ANNOUNCE_RACE if m["count"].as_u64() == Some(1) => {
                let allowed = m["available"].as_u64().unwrap_or(0);
                for (i, &race) in TYPES.iter().enumerate().filter(|(_, r)| allowed & **r != 0) {
                    cases.push((format!("type {race:#x}"), vec![(vec![], i as u32 + 1)], json!({ "type": resp::RACE, "races": [race] })));
                }
            }
            msg::ANNOUNCE_ATTRIB if m["count"].as_u64() == Some(1) => {
                let allowed = m["available"].as_u64().unwrap_or(0);
                for (i, &attribute) in ATTRIBUTES.iter().enumerate().filter(|(_, a)| allowed & **a != 0) {
                    cases.push((format!("attribute {attribute:#x}"), vec![(vec![], 1 << (i + 1))], json!({ "type": resp::ATTRIB, "attributes": [attribute] })));
                }
            }
            // Each number offered, as the dialog sends it (in its unit).
            msg::ANNOUNCE_NUMBER => {
                let options: Vec<u64> = m["options"].as_array().into_iter().flatten().filter_map(Json::as_u64).collect();
                let unit = number_unit(&options).map_or(1, |(unit, _)| unit);
                for (i, &o) in options.iter().enumerate() {
                    cases.push((format!("number {o}"), vec![(vec![], (o / unit) as u32)], json!({ "type": resp::NUMBER, "value": i })));
                }
            }
            // An order: the cards clicked last to first.
            msg::SORT_CHAIN | msg::SORT_CARD if len("cards") >= 2 => {
                let n = len("cards") as u32;
                let picks: Vec<u32> = (0..n).rev().collect();
                let order: Vec<u32> = (0..n).rev().collect();
                cases.push(("the reverse order".into(), vec![(picks, n)], json!({ "type": resp::SORT, "order": order })));
            }
            _ => {}
        }
        cases
            .into_iter()
            .filter(|(_, clicks, expected)| {
                let mut t = self.clone();
                let mut last = None;
                for (picks, result) in clicks {
                    // A case names the engine's indices; the client clicks where the list shows each (`field_order`).
                    for &p in picks {
                        let at = t.list_order.iter().position(|&e| e == p as usize).map_or(p, |at| at as u32);
                        t.pick(at);
                    }
                    last = t.dialog_result(*result);
                }
                last != Some(Prompt::Answer(expected.clone()))
            })
            .map(|(what, _, _)| format!("{what} of a {} prompt (card {})", kind, m["cid"]))
            .collect()
    }

    /// The engine response for the client's card command, if it names a listed card (its first entry: `command`
    /// asks which when there are several).
    pub fn answer(&mut self, cmd: CardCommand) -> Option<Json> {
        let (_, kind, action, indices) = self.listed(&cmd)?;
        self.pending = None;
        Some(json!({ "type": kind, "action": action, "index": indices[0] }))
    }

    /// The pending prompt's entries the client's card command names: the list, the response's type and action, and
    /// the entries' indices in the list (several when the card offers the command more than once).
    fn listed(&self, cmd: &CardCommand) -> Option<(&'static str, u64, u64, Vec<usize>)> {
        let m = self.pending.as_ref()?;
        let (lists, kind): (Lists, u64) = match m["type"].as_u64() {
            Some(msg::SELECT_BATTLECMD) => (&BATTLE_LISTS, resp::BATTLECMD),
            _ => (&IDLE_LISTS, resp::IDLECMD),
        };
        // The client looks a card's commands up at (zone 0, index slot) for monsters and (zone 5, index slot) for
        // Spells/Traps (0x50A690), but sends the one picked with the card's own place, (zone slot, index 0) (live: an
        // attack from the second monster slot came as zone 1, index 0).
        let (zone, index) = match (cmd.zone, cmd.index) {
            (z @ 1..=4, 0) => (0, z as usize),
            (z @ 6..=9, 0) => (5, z as usize - 5),
            (zone::FIELD_SPELL, 0) => (5, 5),
            (z, i) => (z, i as usize),
        };
        let place = table_place(cmd.player, zone, index);
        for &(list, bit, action) in lists {
            if bit != cmd.command {
                continue;
            }
            let at: Vec<usize> = m[list].as_array()?.iter().enumerate().filter(|(_, loc)| self.place(loc) == Some(place)).map(|(i, _)| i).collect();
            if !at.is_empty() {
                return Some((list, kind, action, at));
            }
        }
        None
    }
}

/// The kinds of a card move (`0x1A`'s `param1` high half, the third argument of the client's move `0x5000E0`), as
/// Konami's server sent them (227 original duel logs): 0 a plain move (draws, discards, cards sent or banished), 2 a
/// Normal Summon, 3 a Special Summon, 4 a Spell/Trap activated from the hand, 5 a Set, 6 destroyed by an effect, 8 by
/// battle, 9 tributed (1 and 7 are rare; meaning open). The client plays no move sound for 6, 8 and 9 (the destruction
/// has its own) nor for 3 from anywhere but the hand.
const MOVE_PLAIN: u32 = 0;
const MOVE_NORMAL_SUMMON: u32 = 2;
const MOVE_SPECIAL_SUMMON: u32 = 3;
const MOVE_ACTIVATE: u32 = 4;
const MOVE_SET: u32 = 5;
const MOVE_DESTROYED: u32 = 6;
const MOVE_DESTROYED_BY_BATTLE: u32 = 8;
const MOVE_TRIBUTED: u32 = 9;

/// The summon line's kinds (SpeakSummon 0x44 param3) in Konami's logs: 0 a Normal Summon, 1 a Tribute Summon, 5 a
/// Special Summon, 6 an Extra Deck monster's arrival (after RunFusion), 8 a Synchro Summon (DuelCutin: its own slot).
const SUMMON_NORMAL: u32 = 0;
const SUMMON_TRIBUTE: u32 = 1;
const SUMMON_FLIP: u32 = 4;
const SUMMON_SPECIAL: u32 = 5;
const SUMMON_EXTRA: u32 = 6;
const SUMMON_SYNCHRO: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Family {
    Tribute,
    Fusion,
    Synchro,
    /// A Contact Fusion's materials (Gladiator Beasts, Elemental HEROes' Neos fusions): ProjectIgnis' contact
    /// procedures send them as a cost, with no Fusion reason. Konami's logs show no Material commands for them.
    Contact,
}

impl Family {
    /// The family of a material's move by its reason: a tribute for a summon, a Fusion or Synchro material.
    fn of(reason: u64) -> Option<Family> {
        if reason & reason::RELEASE != 0 && reason & reason::SUMMON != 0 {
            Some(Family::Tribute)
        } else if reason & reason::MATERIAL != 0 && reason & reason::SYNCHRO != 0 {
            Some(Family::Synchro)
        } else if reason & reason::MATERIAL != 0 && reason & reason::FUSION != 0 {
            Some(Family::Fusion)
        } else if reason & reason::MATERIAL != 0 && reason & (reason::RELEASE | reason::RITUAL | reason::XYZ) == 0 {
            Some(Family::Contact)
        } else {
            None
        }
    }

    /// The DuelSummon commands that mark a material and play its effect (none for a Contact Fusion's).
    fn set_and_run(self) -> Option<(u32, u32)> {
        match self {
            Family::Tribute => Some((cmd::TRIBUTE_SET, cmd::TRIBUTE_RUN)),
            Family::Fusion => Some((cmd::MATERIAL_SET, cmd::MATERIAL_RUN)),
            Family::Synchro => Some((cmd::TUNING_SET, cmd::TUNING_RUN)),
            Family::Contact => None,
        }
    }
}

/// A Tribute, Fusion or Synchro Summon as Konami's server showed it: each material marked and playing its effect
/// before its move, then (from the Extra Deck) RunFusion and the summon line before the monster arrives, or (a Tribute
/// Summon) the line with its own kind; the summoned monster's line is then not said again.
#[derive(Clone, Debug)]
struct Summoning {
    family: Family,
    /// The materials moved, and the Tuners among them.
    count: u32,
    tuned: u32,
    /// The Extra Deck monster arrived (its Konami id): its line was said.
    arrived: bool,
    cid: u16,
    /// Materials marked ahead of their moves (later in the same batch), with the effect each plays as it goes (none
    /// when it was played already: a Synchro's).
    played: Vec<(u16, Option<Message>)>,
    /// The materials' uids, the Tuners first (the animations' material list).
    uids: Vec<u32>,
}

/// A move's kind (see [`MOVE_PLAIN`]) from MSG_MOVE's reason and the places.
fn move_kind(reason: u64, from: Place, to: Place) -> u32 {
    if reason & reason::DESTROY != 0 {
        return if reason & reason::BATTLE != 0 { MOVE_DESTROYED_BY_BATTLE } else { MOVE_DESTROYED };
    }
    if reason & reason::RELEASE != 0 {
        return MOVE_TRIBUTED;
    }
    match (from.zone, to.zone) {
        (_, 0..5) if reason & reason::SPSUMMON != 0 => MOVE_SPECIAL_SUMMON,
        (zone::HAND, 0..5) if !to.face_up => MOVE_SET,
        (zone::HAND, 0..5) if reason & reason::SUMMON != 0 => MOVE_NORMAL_SUMMON,
        (zone::HAND, 5..=zone::FIELD_SPELL) if to.face_up => MOVE_ACTIVATE,
        (zone::HAND, 5..=zone::FIELD_SPELL) => MOVE_SET,
        _ => MOVE_PLAIN,
    }
}

/// Card table entries (1000) carry the card's Konami id in `param3`, as Konami's server sent them (all 53,586 entries of
/// 227 original duel logs); a hidden card (row 0) has none. The client keeps it in its card table (`0x521DF0`).
fn with_card_ids(cards: &HashMap<u16, CardStats>, out: &mut [Message]) {
    for m in out.iter_mut().filter(|m| m.get("id").and_then(Value::as_u32) == Some(cmd::CARD)) {
        let row = m.get("param2").and_then(Value::as_u32).unwrap_or(0) as u16;
        let Some(cid) = cards.get(&row).map(|c| c.cid).filter(|_| row != 0) else { continue };
        if let Some((_, v)) = m.fields.iter_mut().find(|(k, _)| k == "param3") {
            *v = Value::U32(cid as u32);
        }
    }
}

/// Whether an engine location is an Xyz material's (ocgcore-wasm adds `overlay_sequence`; a raw location has bit 0x80).
fn is_material(loc: &Json) -> bool {
    loc["overlay_sequence"].is_u64() || loc["location"].as_u64().unwrap_or(0) & loc::OVERLAY != 0
}

/// Tokens' uids, after any deck's (a move carries the uid in 16 bits).
const FIRST_TOKEN_UID: u32 = 0x4000;

/// The table entry flag of a card in this zone: open in the Graveyard and among the banished cards.
fn open(zone: u32) -> bool {
    zone == zone::GRAVE || zone == zone::REMOVED
}

/// A monster turned right after its summon lands in its new position: within one batch to the client, a summon move
/// onto a monster zone followed by its summon effect (RunSummon / RunSpSummon) and then a turn (CardFlipTurn) of that
/// zone gets the turn's position on the move itself. CardFlipTurn waits only for the card manager, so it could run
/// while the summoner's longer summon effect still played and the monster stayed upright there (live: Stumbling, on
/// the Windows VM test; the other client, with a shorter effect, showed it right).
pub fn settle_positions(out: &mut [Message]) {
    let param = |m: &Message, k: &str| m.get(k).and_then(Value::as_u32).unwrap_or(0);
    // By slot (player | zone << 1): the summon move's index, and whether its summon effect has come since.
    let mut moved: HashMap<u32, (usize, bool)> = HashMap::new();
    for i in 0..out.len() {
        let id = param(&out[i], "id");
        let slot = |p: u32| ((p >> 1) & 0x1F < 5).then_some(p & 0x3F);
        match id {
            cmd::MOVE => {
                if let Some(from) = slot(param(&out[i], "param2")) {
                    moved.remove(&from);
                }
                if let Some(to) = slot(param(&out[i], "param3")) {
                    moved.insert(to, (i, false));
                }
            }
            cmd::RUN_SUMMON | cmd::RUN_SP_SUMMON => moved.values_mut().for_each(|(_, effect)| *effect = true),
            cmd::FLIP_TURN => {
                let place = param(&out[i], "param1");
                if let Some((at, true)) = slot(place).and_then(|s| moved.remove(&s)) {
                    let to = param(&out[at], "param3") & !0xC000 | place & 0xC000;
                    if let Some((_, v)) = out[at].fields.iter_mut().find(|(k, _)| k == "param3") {
                        *v = Value::U32(to);
                    }
                }
            }
            _ => {}
        }
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
/// A card's own strings (code << 20 | n) have labels only where CARD_OPTION_LABELS pairs them.
fn option_list(m: &Json) -> Option<Vec<Message>> {
    let options = m["options"].as_array()?;
    let label = |d: u64| {
        let system = OPTION_LABELS.iter().find(|(o, _, _)| *o == d).map(|&(_, label, title)| (label, title));
        system.or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| (label, text::SELECT_EFFECT)))
    };
    let found: Option<Vec<(u32, u32)>> = options.iter().map(|o| label(o.as_u64()?)).collect();
    let found = found.filter(|f| !f.is_empty() && f.len() <= SELECT_MAX)?;
    let labels: Vec<u32> = found.iter().map(|(label, _)| label | dialog::LISTED).collect();
    let mut out = vec![command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0])];
    out.extend(fours(cmd::LIST_VALUES, &labels));
    out.extend(message(&[entry::TEXT | found[0].1]));
    out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
    Some(out)
}

/// The message entries (at most four) a dialog shows: 0x3EC {count}, 0x3ED {entries}.
fn message(entries: &[u32]) -> Vec<Message> {
    let mut params = [0; 4];
    params[..entries.len()].copy_from_slice(entries);
    vec![command(cmd::MESSAGE_LENGTH, [entries.len() as u32, 0, 0, 0]), command(cmd::MESSAGE, params)]
}

/// The client's dialog for a declaration (MSG_ANNOUNCE_RACE / ATTRIB {count, available}, NUMBER {options}, CARD):
/// None when it can't ask it (several types at once, numbers outside the number dialog's range). `level`: the number is
/// a Level (hint 567 before it), which the dialog says; Duel.AnnounceLevel is only an alias of AnnounceNumberRange, and
/// the client's other cards declare counts and die rolls with it.
fn declaration(m: &Json, level: bool) -> Option<Vec<Message>> {
    let kind = m["type"].as_u64()?;
    let allowed = m["available"].as_u64().unwrap_or(0);
    let bits = |table: &[u64]| table.iter().enumerate().filter(|(_, b)| allowed & **b != 0).fold(0u32, |f, (i, _)| f | 1 << (i + 1));
    let (dialog, text, param3) = match kind {
        msg::ANNOUNCE_RACE if m["count"].as_u64() == Some(1) => (dialog::TYPE, text::TYPE, bits(&TYPES)),
        msg::ANNOUNCE_ATTRIB => (dialog::ATTRIBUTE, text::ATTRIBUTE, bits(&ATTRIBUTES) << 16 | m["count"].as_u64().unwrap_or(1) as u32),
        msg::ANNOUNCE_NUMBER => {
            let options: Vec<u64> = m["options"].as_array()?.iter().filter_map(Json::as_u64).collect();
            let (unit, format) = number_unit(&options)?;
            let (min, max) = (options.iter().min()? / unit, options.iter().max()? / unit);
            let step = options.windows(2).map(|w| w[1].abs_diff(w[0]) / unit).filter(|&d| d > 0).min().unwrap_or(1);
            if min > 15 || step > 15 || max > 255 {
                return None;
            }
            let param3 = (max as u32) << 24 | (step as u32) << 20 | (min as u32) << 16 | format;
            // A Level gets "Declare monster level."; other numbers no message.
            if level {
                let mut out = message(&[entry::TEXT | text::LEVEL]);
                out.push(Cmd::Dialog { kind: dialog::NUMBER, message: dialog::FROM_ENTRIES, param: param3 }.message());
                return Some(out);
            }
            return Some(vec![Cmd::Dialog { kind: dialog::NUMBER, message: 0, param: param3 }.message()]);
        }
        msg::ANNOUNCE_CARD => return Some(vec![command(cmd::LIST_BEGIN, [0, 1, 0, 0]), command(cmd::CARD_LIST, [CLIENT, list::SEARCH, 0, 0])]),
        _ => return None,
    };
    let mut out = message(&[entry::TEXT | text]);
    out.push(Cmd::Dialog { kind: dialog, message: dialog::FROM_ENTRIES, param: param3 }.message());
    Some(out)
}

/// The client's number for EDOPro's Type or Attribute bits: its place in the client's list, plus one (CARD_Prop and the
/// declaration dialogs number them so). None without bits or for one the client lacks.
fn client_number(bits: Option<u64>, table: &[u64]) -> Option<u32> {
    let bits = bits.filter(|&b| b != 0)?;
    table.iter().position(|&t| bits & t != 0).map(|i| i as u32 + 1)
}

/// The number dialog's unit and format for these options: the largest unit all of them are multiples of.
fn number_unit(options: &[u64]) -> Option<(u64, u32)> {
    if options.is_empty() {
        return None;
    }
    NUMBER_FORMATS.iter().copied().find(|(unit, _)| options.iter().all(|o| o % unit == 0))
}

/// The duel prompt table's text for an EDOPro selection hint (HINTMSG_*), where one says the same thing.
fn hint_text(hint: u64) -> Option<u32> {
    Some(match hint {
        hintmsg::RELEASE => text::TRIBUTE,
        501 => 347, // HINTMSG_DISCARD: "Select the @2card(s)@0 in your hand you want to discard."
        502 => 281, // HINTMSG_DESTROY: "Select @3card@0 to @3destroy@0."
        503 => 70,  // HINTMSG_REMOVE: "Select @3card(s)@0 to @3remove from play@0."
        504 => 58,  // HINTMSG_TOGRAVE: "Select the @3card@0 to send to the @2Graveyard@0."
        505 => 348, // HINTMSG_RTOHAND: "Select the @3card(s)@0 to return to the @2hand@0."
        506 => 88,  // HINTMSG_ATOHAND: "Select the @3card(s)@0 to add to your @2hand@0."
        507 => 78,  // HINTMSG_TODECK: "Select a @3card@0 to be returned the owner's @2Deck@0."
        508 => 39,  // HINTMSG_SUMMON: "Select a @3monster@0 to @3Normal Summon or Set@0."
        509 => 25,  // HINTMSG_SPSUMMON: "Select the @3monster(s)@0 to @3Special Summon@0."
        510 => 46,  // HINTMSG_SET: "Select a @3Spell or Trap Card@0 to @3Set@0 on the field."
        511 => 92,  // HINTMSG_FMATERIAL: "Select @3monsters@0 to use as @3Fusion Material@0."
        // HINTMSG_FACEUP: the client has no "face-up card" prompt; EDOPro asks it for targets (Spellbinding Circle).
        514 => 94,
        515 => 273, // HINTMSG_FACEDOWN: "Select the @3monster(s)@0 in @3face-down@0 Position."
        518 => 101, // HINTMSG_EQUIP: "Select a @3monster@0 to @3equip@0."
        520 => 119, // HINTMSG_CONTROL: "Select the @3monster(s)@0 your opponent controls to take control."
        526 => 341, // HINTMSG_CONFIRM: "Select the @3card(s)@0 to show your opponent."
        528 => 357, // HINTMSG_POSCHANGE: "Select the @3monster@0 to change its battle position."
        549 => 222, // HINTMSG_ATTACKTARGET: "Select an attack target @3monster@0."
        550 => text::SELECT_EFFECT, // HINTMSG_EFFECT: "Select effect to @3activate@0."
        551 => 94,  // HINTMSG_TARGET: "Select the @3card(s)@0 to target."
        _ => return None,
    })
}

/// The server's words for EDOPro's selection hints (HINTMSG_*) that the client's prompt table has no text for, in
/// the table's style (`@3` highlights, `@2` places). Shown through the connection DLL (`Translator::text`).
const HINT_WORDS: [(u64, &str); 38] = [
    (512, "Select the @3monster(s)@0 to use as @3Synchro Material@0."),
    (513, "Select the @3monsters@0 to use as @3Xyz Material@0."),
    (516, "Select a @3monster@0 in @3Attack Position@0."),
    (517, "Select a @3monster@0 in @3Defense Position@0."),
    (519, "Select the @3Xyz Material@0 to detach."),
    (521, "Select the @3card@0 to destroy instead."),
    (522, "Select a @3monster@0 in face-up @3Attack Position@0."),
    (523, "Select a @3monster@0 in face-up @3Defense Position@0."),
    (524, "Select a @3monster@0 in face-down @3Attack Position@0."),
    (525, "Select a @3monster@0 in face-down @3Defense Position@0."),
    (527, "Select the @3card@0 to place on the field."),
    (529, "Select a @3card@0 you control."),
    (530, "Select a @3card@0 your opponent controls."),
    (531, "Select the @3monster(s)@0 to @3Tribute@0."),
    (532, "Select the @3Xyz Monster@0 to detach material from."),
    (533, "Select the @3monsters@0 to use as @3Link Material@0."),
    (552, "Call heads or tails."),
    (553, "Select a die result."),
    (554, "Declare a @3card type@0."),
    (555, "Select an option."),
    (556, "Select the effect to apply."),
    (560, "Select a @3card@0."),
    (561, "Select a battle position."),
    (562, "Declare an @3Attribute@0."),
    (563, "Declare a @3Type@0."),
    (564, "Declare a @3card name@0."),
    (565, "Declare a number."),
    (566, "Select the @3effect@0 to activate."),
    (567, "Declare a @3Level@0."),
    (568, "Select the @3card@0 whose effect to apply."),
    (569, "Select a @3zone@0."),
    (570, "Select a @3zone@0 to make unusable."),
    (572, "Select the @3card@0 to place a @3counter@0 on."),
    (575, "Select the @3card@0 to negate."),
    (576, "Select the @3monster@0 whose @3ATK/DEF@0 changes."),
    (577, "Select the @3card@0 to apply the effect to."),
    (578, "Select the @3card(s)@0 to attach as @3Xyz Material@0."),
    (579, "Select the @3banished card(s)@0 to return to the @2Graveyard@0."),
];

/// How many characters fit on one of the Select dialog's labels (one line; live, 57 of Pyramid Energy's 61 showed).
const LABEL_CHARS: usize = 56;

/// `text` cut to `max` characters at a word, with "…" to say so.
fn shortened(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max - 1).collect();
    let cut = cut.rfind(' ').filter(|&at| at > max / 2).map_or(cut.as_str(), |at| &cut[..at]);
    format!("{}…", cut.trim_end_matches([' ', ',', '.', ';']))
}

/// The client's duel-end reason (text 1999 + reason, 0x508650) for an EDOPro win reason.
fn end_reason(reason: u64) -> u32 {
    match reason {
        0 => 12,    // "has surrendered" (12, not 5: 106 of Konami's 227 logs end on 12, none on 5)
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

/// MSG_SELECT_SUM {amount: the target, selects_must: cards that count without being picked, select_max: 0 exactly the
/// amount, 1 at least it with no card to spare (Ritual Summons), min..max: how many cards are picked}: its bounds
/// on how many cards are picked, where 0 sets none (Machina Fortress: discard any number of monsters with Levels of 8
/// or more).
fn sum_bounds(m: &Json) -> (usize, usize) {
    (m["min"].as_u64().unwrap_or(0) as usize, m["max"].as_u64().filter(|&m| m != 0).unwrap_or(u32::MAX as u64) as usize)
}

/// Whether picking these of MSG_SELECT_SUM's `selects` (indices; the `selects_must` count too) makes its sum.
fn sum_fits(m: &Json, picked: &[usize]) -> bool {
    let level = |c: &Json| c["amount"].as_u64().unwrap_or(0);
    let selects: Vec<u64> = m["selects"].as_array().into_iter().flatten().map(level).collect();
    let mut seen = vec![];
    let distinct = picked.iter().all(|&i| i < selects.len() && !seen.contains(&i) && { seen.push(i); true });
    let (min, max) = sum_bounds(m);
    distinct && (min.min(selects.len())..=max).contains(&picked.len()) && sum_reached(m, &picked.iter().map(|&i| selects[i]).collect::<Vec<_>>())
}

/// Whether cards of these amounts (with the `selects_must`) count to MSG_SELECT_SUM's target: exactly, or with
/// `select_max` at least it with no card to spare. A card's `amount` is its level, or two levels (low and high 16
/// bits) it may count as.
fn sum_reached(m: &Json, chosen: &[u64]) -> bool {
    let level = |c: &Json| c["amount"].as_u64().unwrap_or(0);
    let must: Vec<u64> = m["selects_must"].as_array().into_iter().flatten().map(level).collect();
    let target = m["amount"].as_u64().unwrap_or(0);
    let greater = m["select_max"].as_u64().unwrap_or(0) != 0;
    let values = |a: u64| -> Vec<u64> { [a & 0xFFFF, a >> 16].into_iter().filter(|v| *v > 0).collect() };
    {
        let all: Vec<u64> = must.iter().chain(chosen).copied().collect();
        let mut sums = vec![0u64];
        for a in &all {
            let vs = values(*a);
            sums = sums.iter().flat_map(|s| if vs.is_empty() { vec![*s] } else { vs.iter().map(|v| s + v).collect() }).collect();
            sums.sort_unstable();
            sums.dedup();
        }
        if !greater {
            return sums.contains(&target);
        }
        // At least the target, and dropping the smallest card would leave less than it.
        let smallest = all.iter().map(|a| values(*a).into_iter().min().unwrap_or(0)).min().unwrap_or(0);
        sums.iter().any(|s| *s >= target && (all.is_empty() || s - smallest.min(*s) < target))
    }
}

/// Indices into MSG_SELECT_SUM's `selects` that make its sum (see [`sum_reached`]), the fewest cards first; none
/// that fits: every card, which the core refuses and the caller's repeated-prompt check then ends.
fn sum_choice(m: &Json) -> Vec<usize> {
    let selects: Vec<u64> = m["selects"].as_array().into_iter().flatten().map(|c| c["amount"].as_u64().unwrap_or(0)).collect();
    let (min, max) = sum_bounds(m);
    let fits = |chosen: &[u64]| sum_reached(m, chosen);
    let n = selects.len();
    if n <= 16 {
        let mut masks: Vec<u32> = (0..1u32 << n).collect();
        masks.sort_by_key(|k| k.count_ones());
        for mask in masks {
            let count = mask.count_ones() as usize;
            if count < min.min(n) || count > max {
                continue;
            }
            let chosen: Vec<u64> = (0..n).filter(|i| mask >> i & 1 == 1).map(|i| selects[i]).collect();
            if fits(&chosen) {
                return (0..n).filter(|i| mask >> i & 1 == 1).collect();
            }
        }
    }
    (0..n.min(max)).collect()
}

/// Spots an engine that keeps asking the same prompt after it was answered (the core refuses an answer it can't use and
/// asks again; an answer the server can't give would then loop under the table's lock).
#[derive(Default)]
pub struct Repeats {
    last: Option<Json>,
    times: u32,
}

/// How many times the same prompt may come back in a row before the duel is given up.
const MAX_REPEATS: u32 = 8;
/// The most picks of a card list kept (a list holds a few dozen entries).
const MAX_PICKS: usize = 256;

impl Repeats {
    /// Whether `prompt` was just asked too many times in a row.
    pub fn stuck(&mut self, prompt: &Json) -> bool {
        if self.last.as_ref() == Some(prompt) {
            self.times += 1;
        } else {
            self.last = Some(prompt.clone());
            self.times = 0;
        }
        self.times >= MAX_REPEATS
    }
}

/// The CPU's answer, and the default for the client's prompts that have no window yet. A placeholder AI: Normal
/// Summon (or Set) the first monster it may, attack with every attacker, never chain; card selections take the first
/// `min` cards, questions are answered yes, lists their first entry.
/// The order Konami's server filled zones in, centre out (all 129 placements of its logs).
const ZONE_ORDER: [u64; 5] = [2, 3, 1, 4, 0];

/// The zones a place prompt leaves free (field_mask bits set = unavailable, relative to the choosing player: monster
/// zones 0-4, Spell/Trap 8-12, the opponent's 16-20 and 24-28), as (engine player, location, sequence), centre out.
fn free_zones(m: &Json) -> Vec<(u64, u64, u64)> {
    let mask = m["field_mask"].as_u64().unwrap_or(0);
    let me = m["player"].as_u64().unwrap_or(0);
    [(me, loc::MZONE, 0), (me, loc::SZONE, 8), (1 - me, loc::MZONE, 16), (1 - me, loc::SZONE, 24)]
        .into_iter()
        .flat_map(|(p, l, base)| ZONE_ORDER.map(|s| (p, l, s, base + s)))
        .filter(|&(_, _, _, bit)| mask & 1 << bit == 0)
        .map(|(p, l, s, _)| (p, l, s))
        .collect()
}

fn cpu_answer(kind: u64, m: &Json) -> Json {
    let list_len = |k: &str| m[k].as_array().map_or(0, Vec::len);
    let first = |min: &Json, max: &Json, len: usize| {
        let n = (min.as_u64().unwrap_or(1) as usize).max(1).min(max.as_u64().unwrap_or(1) as usize).min(len);
        (0..n).collect::<Vec<_>>()
    };
    match kind {
        msg::SELECT_IDLECMD => {
            let (action, index) = if list_len("summons") > 0 {
                (idle::SUMMON, json!(0))
            } else if list_len("monster_sets") > 0 {
                (idle::MONSTER_SET, json!(0))
            } else if m["to_bp"] == true {
                (idle::TO_BP, Json::Null)
            } else {
                (idle::TO_EP, Json::Null)
            };
            json!({ "type": resp::IDLECMD, "action": action, "index": index })
        }
        msg::SELECT_BATTLECMD if list_len("attacks") > 0 => json!({ "type": resp::BATTLECMD, "action": battle::ATTACK, "index": 0 }),
        msg::SELECT_BATTLECMD => json!({ "type": resp::BATTLECMD, "action": battle::TO_EP, "index": null }),
        msg::SELECT_CHAIN => {
            let index = if m["forced"].as_bool() == Some(true) { json!(0) } else { Json::Null };
            json!({ "type": resp::CHAIN, "index": index })
        }
        msg::SELECT_CARD => json!({ "type": resp::CARD, "indicies": first(&m["min"], &m["max"], list_len("selects")) }),
        msg::SELECT_TRIBUTE => json!({ "type": resp::TRIBUTE, "indicies": first(&m["min"], &m["max"], list_len("selects")) }),
        msg::SELECT_UNSELECT_CARD => {
            // Finishing with nothing picked cancels: Panther Warrior's attack (a Tribute to declare it) came back
            // again and again. Pick first.
            let finish = m["can_finish"] == true && m["unselect_cards"].as_array().is_some_and(|c| !c.is_empty());
            let index = if finish { Json::Null } else { json!(0) };
            json!({ "type": resp::UNSELECT_CARD, "index": index })
        }
        msg::SELECT_EFFECTYN => json!({ "type": resp::EFFECTYN, "yes": true }),
        msg::SELECT_YESNO => json!({ "type": resp::YESNO, "yes": true }),
        msg::SELECT_OPTION => json!({ "type": resp::OPTION, "index": 0 }),
        msg::SORT_CHAIN | msg::SORT_CARD => json!({ "type": resp::SORT, "order": null }),
        msg::SELECT_COUNTER => {
            // The counters from the cards in order, each giving what it holds until the count is made.
            let mut left = m["count"].as_u64().unwrap_or(0);
            let mut counters: Vec<u64> = m["cards"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| {
                    let take = left.min(c["count"].as_u64().unwrap_or(0));
                    left -= take;
                    take
                })
                .collect();
            if counters.is_empty() {
                counters.push(left);
            }
            json!({ "type": resp::COUNTER, "counters": counters })
        }
        // The free zones centre out, as Konami's server filled them: the player's monster zones, Spell/Trap zones, then
        // the opponent's (Give and Take and Black Garden place on the opponent's field), and the Field zone (13).
        msg::SELECT_PLACE | msg::SELECT_DISFIELD => {
            let mask = m["field_mask"].as_u64().unwrap_or(0);
            let me = m["player"].as_u64().unwrap_or(0);
            let field = (mask & 1 << 13 == 0).then_some((me, loc::SZONE, 5));
            // As many as asked (Ground Collapse and Phantom Dragon each disable two zones).
            let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
            let mut places: Vec<Json> = free_zones(m)
                .into_iter()
                .chain(field)
                .take(count)
                .map(|(p, l, s)| json!({ "player": p, "location": l, "sequence": s }))
                .collect();
            if places.is_empty() {
                places.push(json!({ "player": me, "location": loc::MZONE, "sequence": 0 }));
            }
            let kind = if kind == msg::SELECT_PLACE { resp::PLACE } else { resp::DISFIELD };
            json!({ "type": kind, "places": places })
        }
        // Face-up attack when allowed, else the first allowed position.
        msg::SELECT_POSITION => {
            let allowed = m["positions"].as_u64().unwrap_or(1);
            let position = if allowed & 1 != 0 { 1 } else { 1 << allowed.trailing_zeros() };
            json!({ "type": resp::POSITION, "position": position })
        }
        // The lowest `count` of the available bits.
        msg::ANNOUNCE_RACE | msg::ANNOUNCE_ATTRIB => {
            let available = m["available"].as_u64().unwrap_or(1);
            let picked: Vec<u64> = (0..64).map(|b| 1u64 << b).filter(|b| available & b != 0).take(m["count"].as_u64().unwrap_or(1) as usize).collect();
            let (kind, key) = if kind == msg::ANNOUNCE_RACE { (resp::RACE, "races") } else { (resp::ATTRIB, "attributes") };
            json!({ "type": kind, key: picked })
        }
        msg::ANNOUNCE_NUMBER => json!({ "type": resp::NUMBER, "value": 0 }),
        msg::ANNOUNCE_CARD => json!({ "type": resp::CARD_NAME, "cid": m["fits"][0].as_u64().unwrap_or(0) }),
        msg::ROCK_PAPER_SCISSORS => json!({ "type": resp::RPS, "value": 1 }),
        msg::SELECT_SUM => json!({ "type": resp::SUM, "indicies": sum_choice(m) }),
        _ => {
            tracing::warn!("rules engine: no answer for prompt {kind}");
            json!({ "type": resp::CHAIN, "index": null })
        }
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
