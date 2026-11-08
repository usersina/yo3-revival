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
use yo3_proto::{Message, Value};

use crate::card_options::CARD_OPTION_LABELS;
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
const MSG_SELECT_SUM: u64 = 23;
const MSG_SELECT_DISFIELD: u64 = 24;
const MSG_SORT_CARD: u64 = 25;
const MSG_SELECT_UNSELECT_CARD: u64 = 26;
const MSG_ROCK_PAPER_SCISSORS: u64 = 132;
const MSG_ANNOUNCE_RACE: u64 = 140;
const MSG_ANNOUNCE_ATTRIB: u64 = 141;
const MSG_ANNOUNCE_CARD: u64 = 142;
const MSG_ANNOUNCE_NUMBER: u64 = 143;
const MSG_MOVE: u64 = 50;
const MSG_NEW_TURN: u64 = 40;
const MSG_NEW_PHASE: u64 = 41;
const MSG_DRAW: u64 = 90;
const MSG_DAMAGE: u64 = 91;
const MSG_RECOVER: u64 = 92;
const MSG_LPUPDATE: u64 = 94;
const MSG_CONFIRM_CARDS: u64 = 31;
const MSG_POS_CHANGE: u64 = 53;
const MSG_SUMMONING: u64 = 60;
const MSG_SPSUMMONING: u64 = 62;
const MSG_FLIPSUMMONING: u64 = 64;
const MSG_ATTACK: u64 = 110;
const MSG_BATTLE: u64 = 111;
const MSG_ATTACK_DISABLED: u64 = 112;
const MSG_DAMAGE_STEP_END: u64 = 114;
const MSG_CHAINING: u64 = 70;
const MSG_CHAIN_SOLVING: u64 = 72;
const MSG_CHAIN_END: u64 = 74;
const MSG_PAY_LPCOST: u64 = 100;
const MSG_ADD_COUNTER: u64 = 101;
const MSG_REMOVE_COUNTER: u64 = 102;
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
const RESP_SUM: u64 = 14;
const RESP_SORT: u64 = 15;
const RESP_RACE: u64 = 16;
const RESP_ATTRIB: u64 = 17;
const RESP_CARD_NAME: u64 = 18;
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
/// ocgcore reasons (MSG_MOVE's `reason`, which the sidecar reads back).
const REASON_DESTROY: u64 = 0x1;
const REASON_BATTLE: u64 = 0x20;
/// An Xyz material: or'ed with its monster's location, whose sequence it takes.
const LOC_OVERLAY: u64 = 0x80;
/// The client's list zones besides hand and deck.
const EXTRA: u32 = 0xC;
const GRAVE: u32 = 0xE;
const REMOVED: u32 = 0xF;
/// A field card's new face and position in place {place as moves give it} (CardFlipTurn, DuelCardManager 0x4FF000).
/// A card destroyed {player, zone | index << 8}, zone at most the hand (CardVanish): its label goes, the shatter
/// (effect 0x33) plays at it and it is hidden (state 0x4FC520), before its move.
const FLIP_TURN: u32 = 0x1C;
const CARD_VANISH: u32 = 0x1F;
/// HandShow {player, index}: a hand card shows its face (DuelCardManager 0x4FF15C).
const HAND_SHOW: u32 = 0x13;
/// A card table entry's flag (command 1000's uid top byte, kept as `!= 0` by 0x521DF0): a pile whose count changes
/// draws its top card's face when its entry has it, else a back (0x4FE4D0 → 0x523DA0), and the pointer over the pile
/// previews that card (0x50B9C0). Set for the open piles: the Graveyard and the banished cards.
const OPEN: u32 = 1 << 24;
/// A field card's counters {player | zone << 8, change, kind} (DuelCardManager 0x4FFD8E): the change is added to the
/// card's count of that kind (0 erases it; a negative count is ignored), shown on its field label with the kind's
/// icon (table 0x789020) and named in the card info as UI text 200 + kind (0x4551DB).
const COUNTER: u32 = 0x27;
/// EDOPro's counter ids (strings.conf, less COUNTER_NEED_ENABLE) by the client's counter kinds of the same name:
/// Spell, Wedge, Bushido, Psychic, Shine, Crystal, Morph, Venom, Genex, A-, Worm, Black Feather, Hyper-Venom,
/// Spellstone ("Magical Stone", **guess**), Flower, Fog, Clock, D, Junk, Gate, Plant, Guard, Dragonic, Ocean,
/// Chronicle, Magic Reflector's, Maiden, Speed, Earthbound Immortal, Protection, Des ("Death", **guess**). The others
/// show as kind 38, "Counter".
const COUNTER_KINDS: [(u64, u32); 32] = [
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
/// A battle (DuelBattle 0x4F8A90): start; the attack {attacker's player | zone << 8, target's player, target's zone or
/// DIRECT_TARGET}; the battle's outcome per player {damage figure on the client's card, on the opponent's, flags: DESTROYED
/// for the client's card, << 8 for the opponent's} (the effect at a card ends hiding it when DESTROYED); end.
const VIEW_BATTLE_INIT: u32 = 0xA;
const VIEW_BATTLE_ATTACK: u32 = 0xC;
const VIEW_BATTLE_RUN: u32 = 0xD;
const VIEW_BATTLE_END: u32 = 0xE;
/// A direct attack's target zone: any zone from 5 plays as one (the direct cut-in and strike, 0x4F8430 state 3);
/// 0xFFFFFFFF plays nothing. 16 is the field's centre, as ChainSet's.
const DIRECT_TARGET: u32 = 16;
const DESTROYED: u32 = 2;
/// The summon effects at a card {player, uid} (DuelScene 0x514290): Normal (or the player's ace), Special.
const RUN_SUMMON: u32 = 0x38;
const RUN_SP_SUMMON: u32 = 0x39;
/// The chain's links (DuelChain 0x501560): a new link {player, zone it shows over, link number}, link 1 starting the
/// list; a link resolving {_, _, link number}, which must come last link first (the list's end leaves).
const CHAIN_SET: u32 = 0x33;
const CHAIN_RUN: u32 = 0x34;
/// ChainSet's zone for a card with no place on the client's field: the centre (place table 0x81AE38).
const FIELD_CENTRE: u32 = 16;
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
/// The kind that draws a card as a back where its flag is clear (getCardInfo 0x4C78A0, "0000" for data/mini/0000.png),
/// by the card table's flag at the banished place of the same index on the side `(client != the list's player)`: so
/// a list of the CPU's side (player 1) reads the opponent's banished entries. No picks needed, like kind 1 (the
/// other kinds draw a card's image from its row's id, so a hidden card, row 0, drew nothing and couldn't be clicked).
const PICK_MIXED: u32 = 4;
/// The uids of the entries a mixed list puts in the table (no card has one).
const STAGED_UID: u32 = 0xF0_0000;
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
/// The battle questions EDOPro's core asks by system string (processor.cpp: 31 attack directly instead of a monster,
/// 30 a replay), as the client's dialog texts 225 "@3Attack@0 your opponent directly?" and 223 "Replay happens.
/// Continue to @3attack@0?".
const BATTLE_QUESTIONS: [(u64, u32); 2] = [(31, 225), (30, 223)];
const DIRECT_TEXT: u32 = 225;
/// "Select the @3monster(s)@0 to @3Special Summon@0 from your @2Extra Deck@0." and "Select the @3monster(s)@0 to
/// @3Special Summon@0.".
const SPSUMMON_EXTRA_TEXT: u32 = 26;
const SPSUMMON_TEXT: u32 = 25;
/// The pile views' titles (the duel prompts): 582/583 "Your/Opponent's Extra Deck", 586/587 "Your/Opponent's
/// Graveyard", 588/589 "Your/Opponent's Removed Cards", by zone, the opponent's one more.
const PILE_TITLES: [(u32, u32); 3] = [(EXTRA, 582), (GRAVE, 586), (REMOVED, 588)];
/// The card menu's Confirm (bit 11): on a pile, its view.
const CONFIRM: u32 = 11;
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
/// The declaration dialogs (factory 0x4C8CD0): Type {_, message, allowed types} (DuelSelectTypeDialog, dlg_seltype.swf:
/// bit k for type k, one pick, DLGResult k), Attribute {_, message, allowed << 16 | picks} (DuelSelectAttrDialog,
/// dlg_selattr.swf: DLGResult the picks as bits), Number {_, message, max << 24 | step << 20 | min << 16 | format}
/// (DuelSelectNumberDialog 0x4C6430, dlg_selnum.swf: in thousands for format 302 "%d000LP", hundreds for 303
/// "%d00LP", else ones; DLGResult the number in those units).
const TYPE_DIALOG: u32 = 5;
const ATTRIBUTE_DIALOG: u32 = 7;
const NUMBER_DIALOG: u32 = 11;
const NUMBER_FORMATS: [(u64, u32); 3] = [(1000, 302), (100, 303), (1, 283)];
/// "Select @3Type(s)@0.", "Select @3Attribute(s)@0.", "Declare @3monster@0 level."
const TYPE_TEXT: u32 = 241;
const ATTRIBUTE_TEXT: u32 = 242;
const LEVEL_TEXT: u32 = 280;
/// The client's types (UI texts 1031-1052) and attributes (1061-1067) in its order, as EDOPro's RACE_* and
/// ATTRIBUTE_* bits.
const TYPES: [u64; 22] = [
    0x2000, 0x10, 0x8, 0x80, 0x40000, 0x100, 0x20, 0x20000, 0x10000, 0x800, 0x4000, 0x8000, 0x400, 0x40, 0x1, 0x200, 0x4,
    0x2, 0x1000, 0x80000, 0x100000, 0x200000,
];
const ATTRIBUTES: [u64; 7] = [0x10, 0x20, 0x2, 0x4, 0x1, 0x8, 0x40];
/// The card list's extended kind that lists the client's own card database with a name search (DuelCardListDialogEx
/// 0x5131E0, dlg_cardlistex.swf; CardDuelFilter 0x4976A0 mode 0: any card); DLGResult the card's Konami id.
const CARD_SEARCH: u32 = 0x4C;
/// The position dialog {_, message, uid | face-up defense << 16} (DuelSelectStandDialog 0x4C5E50, dlg_selstand.swf):
/// button 0 attack, 1 defense, face-down unless the flag is set.
const POSITION: u32 = 8;
const FACE_UP_DEFENSE: u32 = 1 << 16;
/// Duel end {result for this client: 1 won, 2 lost, 3 draw; reason} (DuelScene 0x514290).
pub(crate) const DUEL_END: u32 = 2;
pub(crate) const WON: u32 = 1;
pub(crate) const LOST: u32 = 2;
pub(crate) const DRAW: u32 = 3;
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
    /// A battle shown on the client and not ended yet, and whether the duel is in a Battle Phase.
    in_battle: bool,
    battle_phase: bool,
    /// The last selection hint (HINTMSG_*) and the client's picks in the card list so far.
    hint: u64,
    picks: Vec<usize>,
    /// The duel's result for the client (1 won, 2 lost, 3 draw), once it ended.
    result: Option<u32>,
    /// By player, whether that side's hidden cards (deck, hand, extra deck, face-down) go to the client without their
    /// rows: none against the CPU, the opponent's against a player, both for a spectator.
    hidden: [bool; 2],
    /// The values each monster zone's record last went with (level, ATK, DEF, base ATK, base DEF), by player and zone.
    records: HashMap<(u32, u32), [u32; 5]>,
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
    /// How many card names the CPU has declared: each try names the next card of the duel's (the core asks again
    /// when a name doesn't fit).
    declared: usize,
    /// Cards an effect revealed (MSG_CONFIRM_CARDS) that wait to reach a hand, by Konami id.
    revealing: Vec<u16>,
    /// The card list open is a mixed one (see `card_list`), and how many table entries it staged.
    mixed: bool,
    staged: Option<usize>,
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
                let zone = self.sides[p].zone(EXTRA);
                zone.push(Card { uid, row });
                let shown = if self.hidden[p] { 0 } else { row as u32 };
                out.push(command(CARD, [uid, shown, 0, table_place(p as u32, EXTRA, zone.len() - 1)]));
            }
        }
        out
    }

    /// The client's token cards (CARD_Prop rows, ids 3900-3999) by name: EDOPro's tokens have no Konami id, so the
    /// sidecar names them.
    pub fn tokens(&mut self, rows: HashMap<String, Vec<u16>>) {
        self.token_rows = rows;
    }

    /// The same for a spectator watching from player 0's side, who sees neither player's hidden cards.
    pub fn spectating(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>) -> (Translator, Vec<Message>) {
        Translator::build(first, decks, cards, [true, true])
    }

    fn build(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, CardStats>, hidden: [bool; 2]) -> (Translator, Vec<Message>) {
        let mut sides = [Side::default(), Side::default()];
        let mut out = vec![];
        let mut uid = 0;
        for (p, deck) in decks.iter().enumerate() {
            for (i, &row) in deck.iter().enumerate() {
                uid += 1;
                sides[p].zone(DECK).push(Card { uid, row });
                let shown = if hidden[p] { 0 } else { row as u32 };
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
            in_battle: false,
            battle_phase: false,
            hint: 0,
            picks: vec![],
            result: None,
            hidden,
            records: HashMap::new(),
            positions: HashMap::new(),
            direct: None,
            viewing: false,
            choosing: None,
            token_rows: HashMap::new(),
            tokens: vec![],
            next_token: FIRST_TOKEN_UID,
            counters: HashMap::new(),
            declared: 0,
            revealing: vec![],
            mixed: false,
            staged: None,
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
        if self.hidden[place.player as usize] && hidden { 0 } else { row as u32 }
    }

    /// The client commands for the engine's events (prompts excluded).
    pub fn events(&mut self, messages: &[Json]) -> Vec<Message> {
        let staged = self.staged.take();
        let mut out = self.unstage_n(staged);
        for m in messages {
            if m["type"] == "field" {
                out.extend(self.field(m));
                continue;
            }
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
                    // Leaving a Battle Phase ends its battle mode even with no attack: the swords the client drew over
                    // the possible attackers (effect 0x3C) go only with BattleSelect, BattleAttack or BattleEnd.
                    let battle = (0x08..=0x80).contains(&m["phase"].as_u64().unwrap_or(0));
                    let ended = self.end_battle();
                    if ended.is_empty() && self.battle_phase && !battle {
                        out.push(command(VIEW_BATTLE_END, [0; 4]));
                    }
                    out.extend(ended);
                    self.battle_phase = battle;
                    if let Some(phase) = m["phase"].as_u64().and_then(client_phase) {
                        out.push(command(PHASE, [self.turn_player, phase, 0, 0]));
                    }
                }
                Some(MSG_MOVE) => out.extend(self.move_card(m)),
                Some(MSG_HINT) if m["hint_type"].as_u64() == Some(HINT_SELECTMSG) && self.player(&m["player"]) == CLIENT => {
                    self.hint = m["hint"].as_u64().unwrap_or(0);
                }
                // Also a Set Spell/Trap turning face-up as it is activated.
                Some(MSG_POS_CHANGE | MSG_FLIPSUMMONING) => out.extend(self.turn(m, m["position"].as_u64().unwrap_or(0))),
                Some(MSG_CHAINING) => {
                    out.push(self.chain_link(m));
                    self.chaining = Some(m.clone());
                }
                Some(MSG_CHAIN_SOLVING) => out.push(command(CHAIN_RUN, [0, 0, m["chain_size"].as_u64().unwrap_or(0) as u32, 0])),
                Some(MSG_ATTACK) => {
                    out.extend(self.end_battle());
                    out.extend(self.attack(m));
                }
                Some(MSG_BATTLE) => out.extend(self.battle(m)),
                Some(MSG_DAMAGE_STEP_END | MSG_ATTACK_DISABLED) => out.extend(self.end_battle()),
                Some(kind @ (MSG_SUMMONING | MSG_SPSUMMONING)) => {
                    let id = if kind == MSG_SUMMONING { RUN_SUMMON } else { RUN_SP_SUMMON };
                    // The effect plays at the place's zone (0x514E5A), so the place, not only the player.
                    let card = self.engine_place(m).and_then(|p| Some((p, self.sides[p.player as usize].zones.get(&p.zone)?.first()?.uid)));
                    if let Some((place, uid)) = card {
                        out.push(command(id, [place.player | place.zone << 1, uid, 0, 0]));
                    }
                }
                Some(MSG_CONFIRM_CARDS) => out.extend(self.confirm(m)),
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
                Some(kind @ (MSG_ADD_COUNTER | MSG_REMOVE_COUNTER)) => out.extend(self.counter(m, kind == MSG_ADD_COUNTER)),
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
        if m["from"]["location"].as_u64() == Some(0) {
            return self.engine_place(&m["to"]).map(|to| self.token(m, to)).unwrap_or_default();
        }
        if let Some(from) = self.engine_place(&m["from"]).filter(|f| f.zone < 5)
            && let Some(card) = self.sides[from.player as usize].zones.get(&from.zone).and_then(|z| z.first()).copied()
            && self.tokens.contains(&card.uid)
        {
            // A token leaving the field is gone (EDOPro may send it to a pile first): it shatters off the field.
            let mut out = self.drop_counters(&from);
            self.sides[from.player as usize].zone(from.zone).clear();
            self.tokens.retain(|&t| t != card.uid);
            out.push(command(CARD_VANISH, [from.player, from.zone, 0, 0]));
            return out;
        }
        match (self.engine_place(&m["from"]), self.engine_place(&m["to"])) {
            (Some(from), Some(to)) => {
                let reason = m["reason"].as_u64().unwrap_or(0);
                let dropped = self.drop_counters(&from);
                let mut moved = self.relocate(cid, from, to);
                if to.zone == HAND && let Some(at) = self.revealing.iter().position(|&c| c == cid) {
                    self.revealing.remove(at);
                    let index = self.sides[to.player as usize].zone(HAND).len().saturating_sub(1).min(to.index);
                    moved.extend(self.show_hand(to.player, index));
                }
                // Destroyed other than by battle (a battle shows it with BattleRun): the shatter first.
                if moved.is_empty() || reason & REASON_DESTROY == 0 || reason & REASON_BATTLE != 0 || from.zone > HAND {
                    return dropped.into_iter().chain(moved).collect();
                }
                let vanish = command(CARD_VANISH, [from.player, from.zone | (from.index as u32) << 8, 0, 0]);
                dropped.into_iter().chain(std::iter::once(vanish)).chain(moved).collect()
            }
            // Tokens and Xyz materials have no client place yet.
            _ => vec![],
        }
    }

    /// MSG_CONFIRM_CARDS {player, cards}: the cards are shown to `player`. The client sees one it is shown to, and its
    /// own as the other side sees them; a hand card shows its face at once, a card from elsewhere (the Deck, a search)
    /// when it reaches the hand.
    fn confirm(&mut self, m: &Json) -> Vec<Message> {
        let viewer = self.player(&m["player"]);
        let mut out = vec![];
        for card in m["cards"].as_array().into_iter().flatten() {
            let owner = self.player(&card["controller"]);
            if viewer != CLIENT && owner != CLIENT {
                continue;
            }
            let cid = card["cid"].as_u64().unwrap_or(0) as u16;
            match card["location"].as_u64() {
                Some(LOC_HAND) => out.extend(self.show_hand(owner, card["sequence"].as_u64().unwrap_or(0) as usize)),
                _ if !self.revealing.contains(&cid) => self.revealing.push(cid),
                _ => {}
            }
        }
        out
    }

    /// A hand card shows its face: its table entry with the row (the opponent's hand has none), then HandShow.
    fn show_hand(&self, player: u32, index: usize) -> Vec<Message> {
        let Some(card) = self.sides[player as usize].zones.get(&HAND).and_then(|z| z.get(index)).copied() else { return vec![] };
        vec![
            command(CARD, [card.uid, card.row as u32, 0, table_place(player, HAND, index)]),
            command(HAND_SHOW, [player, index as u32, 0, 0]),
        ]
    }

    /// A token appears on a monster zone: the client's token card of its name (in turn when it has several, as for
    /// the four Sheep Tokens), a card table entry, its record and a move onto its own slot, which makes the card
    /// object (0x1A finds none by uid, 0x5003A0). A token with no card of its name isn't shown.
    fn token(&mut self, m: &Json, to: Place) -> Vec<Message> {
        let Some(rows) = m["name"].as_str().and_then(|n| self.token_rows.get(n)).filter(|r| !r.is_empty()) else {
            tracing::warn!("a token the client has no card for: {}", m["name"]);
            return vec![];
        };
        if to.zone >= 5 {
            return vec![];
        }
        let row = rows[(self.next_token - FIRST_TOKEN_UID) as usize % rows.len()];
        let uid = self.next_token;
        self.next_token += 1;
        self.tokens.push(uid);
        let zone = self.sides[to.player as usize].zone(to.zone);
        zone.clear();
        zone.push(Card { uid, row });
        self.positions.insert((to.player, to.zone), (to.face_up, to.defense));
        let mut out = vec![command(CARD, [uid, row as u32, 0, table_place(to.player, to.zone, 0)])];
        out.extend(self.monster_record(to.player, to.zone, row));
        out.push(command(MOVE, [uid | (row as u32) << 16, to.packed(), to.packed(), 0]));
        out
    }

    /// Moves a card in the mirror and on the client: the card table entry (1000), then on screen (0x1A); the
    /// cards whose index shifted in a list zone get their table entry again.
    fn relocate(&mut self, cid: u16, from: Place, to: Place) -> Vec<Message> {
        let cards = &self.cards;
        let source = self.sides[from.player as usize].zone(from.zone);
        let at = if from.zone == DECK || from.zone == EXTRA {
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
        if to.zone < 5 {
            self.positions.insert((to.player, to.zone), (to.face_up, to.defense));
        }
        let row = self.shown_row(&to, card.row);
        let mut out = vec![command(CARD, [card.uid | open(to.zone), row, 0, table_place(to.player, to.zone, index)])];
        if to.zone < 5 && row != 0 {
            out.extend(self.monster_record(to.player, to.zone, card.row));
        }
        // List zones renumber: re-send the shifted cards (not the face-down deck). A pile's go before the move, whose
        // count change redraws the pile from the table's top entry (0x4FE4D0); the hand's after.
        let mut hand = vec![];
        for (p, zone, start) in [(from.player, from.zone, at), (to.player, to.zone, index + 1)] {
            if zone <= FIELD_SPELL || zone == DECK {
                continue;
            }
            let shifted: Vec<(usize, Card)> = self.sides[p as usize].zone(zone).iter().copied().enumerate().skip(start).collect();
            for (i, c) in shifted {
                let row = self.shown_row(&Place { player: p, zone, index: i, face_up: true, defense: false }, c.row);
                let entry = command(CARD, [c.uid | open(zone), row, 0, table_place(p, zone, i)]);
                if zone == HAND { hand.push(entry) } else { out.push(entry) }
            }
        }
        out.push(command(MOVE, [card.uid | row << 16, from.packed(), to.packed(), 0]));
        out.extend(hand);
        out
    }

    /// MSG_ATTACK {card, target}: the battle's start and the attack, at the target or directly.
    fn attack(&mut self, m: &Json) -> Vec<Message> {
        let Some(attacker) = self.engine_place(&m["card"]) else { return vec![] };
        let (player, zone) = self.engine_place(&m["target"]).map_or((1 - attacker.player, DIRECT_TARGET), |t| (t.player, t.zone));
        self.in_battle = true;
        vec![command(VIEW_BATTLE_INIT, [0; 4]), command(VIEW_BATTLE_ATTACK, [attacker.player | attacker.zone << 8, player, zone, 0])]
    }

    /// MSG_BATTLE {card, target: {attack, defense, position, destroyed}}: per player, the battle damage they take, by
    /// ATK against ATK, or DEF when the target defends (effects that change it aren't followed), and whether their card
    /// is destroyed. The client pairs the values with the cards by player (DuelBattle states 5 and 6). A direct
    /// attack's target is an empty place (location 0; ocgcore-wasm parses it like any card): no figures, since there
    /// is no target card to show them at (the LP change shows the damage).
    fn battle(&self, m: &Json) -> Vec<Message> {
        let (card, target) = (&m["card"], &m["target"]);
        if target["location"].as_u64().unwrap_or(0) == 0 {
            return vec![command(VIEW_BATTLE_RUN, [0; 4])];
        }
        let n = |c: &Json, k: &str| c[k].as_i64().unwrap_or(0);
        let destroyed = |c: &Json| if c["destroyed"] == true { DESTROYED } else { 0 };
        let (to_attacker, to_target) = if target["position"].as_u64().unwrap_or(0) & (POS_FACEUP_ATTACK | POS_FACEDOWN_ATTACK) != 0 {
            let diff = n(card, "attack") - n(target, "attack");
            (-diff.min(0), diff.max(0))
        } else {
            ((n(target, "defense") - n(card, "attack")).max(0), 0)
        };
        let (mut damage, mut flags) = ([0u32; 2], [0u32; 2]);
        let attacker = self.player(&card["controller"]) as usize;
        damage[attacker] = to_attacker as u32;
        damage[1 - attacker] = to_target as u32;
        flags[attacker] = destroyed(card);
        flags[1 - attacker] = destroyed(target);
        vec![command(VIEW_BATTLE_RUN, [damage[0], damage[1], flags[0] | flags[1] << 8, 0])]
    }

    /// The end of the battle shown on the client, if one is.
    fn end_battle(&mut self) -> Vec<Message> {
        if std::mem::take(&mut self.in_battle) { vec![command(VIEW_BATTLE_END, [0; 4])] } else { vec![] }
    }

    /// A field card turns in place to `position`: 0x1C with its new face and position, after its table entry and record
    /// when it shows a row it didn't before (the opponent's face-down card turning face-up when hiding).
    fn turn(&mut self, loc: &Json, position: u64) -> Vec<Message> {
        let Some(place) = self.engine_place(loc) else { return vec![] };
        if place.zone > FIELD_SPELL {
            return vec![];
        }
        // A Spell/Trap has no battle position.
        let place = Place { face_up: position & 5 != 0, defense: place.zone < 5 && position & 12 != 0, ..place };
        let before = (place.zone < 5).then(|| self.positions.insert((place.player, place.zone), (place.face_up, place.defense)));
        let mut out = vec![];
        let card = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied();
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
            out.push(command(CARD, [card.uid, card.row as u32, 0, table_place(place.player, place.zone, 0)]));
            if place.zone < 5 {
                out.extend(self.monster_record(place.player, place.zone, card.row));
            } else {
                // The field card takes its face from a move (a flip only turns it over), and a Spell/Trap has no
                // record to carry its id: it moves onto its own slot with its row, face up.
                let down = Place { face_up: false, ..place };
                out.push(command(MOVE, [card.uid | (card.row as u32) << 16, down.packed(), place.packed(), 0]));
                return out;
            }
        }
        // A player's own monster turned in place stayed upright on that player's screen (live, the VM round: every
        // 0x1C after its summon), while a card arriving by a move is drawn in its position: it moves onto its own
        // slot in the new position first.
        if let (Some(before), Some(card)) = (before, card) {
            let (face_up, defense) = before.unwrap_or((!place.face_up, !place.defense));
            let from = Place { face_up, defense, ..place };
            let row = self.shown_row(&place, card.row);
            out.push(command(MOVE, [card.uid | row << 16, from.packed(), place.packed(), 0]));
        }
        out.push(command(FLIP_TURN, [place.packed(), 0, 0, 0]));
        out
    }

    /// MSG_ADD_COUNTER / MSG_REMOVE_COUNTER {counter_type, controller, location, sequence, count} on a field card: the
    /// change in the client's kind, a removal no more than the client holds.
    fn counter(&mut self, m: &Json, add: bool) -> Vec<Message> {
        let Some(place) = self.engine_place(m).filter(|p| p.zone <= FIELD_SPELL) else { return vec![] };
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
        vec![command(COUNTER, [place.player | place.zone << 8, change as u32, kind, 0])]
    }

    /// A field card about to leave its place gives up its counters on the client, which would keep them (EDOPro drops
    /// them without a message).
    fn drop_counters(&mut self, place: &Place) -> Vec<Message> {
        if place.zone > FIELD_SPELL {
            return vec![];
        }
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.first()).copied() else { return vec![] };
        let mut held: Vec<(u32, u32)> =
            self.counters.iter().filter(|((uid, _), n)| *uid == card.uid && **n > 0).map(|((_, kind), n)| (*kind, *n)).collect();
        held.sort_unstable();
        self.counters.retain(|(uid, _), _| *uid != card.uid);
        held.into_iter().map(|(kind, n)| command(COUNTER, [place.player | place.zone << 8, (n as i32).wrapping_neg() as u32, kind, 0])).collect()
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
        self.records.insert((player, zone), values);
        let s = self.cards.get(&row).copied().unwrap_or_default();
        let cid = s.cid as u32;
        let [level, atk, def, base_atk, base_def] = values;
        vec![
            command(RECORD_BEGIN, [0, cid, s.race, s.attribute]),
            command(RECORD_MORE, [0, level, atk, def]),
            command(RECORD_COMMIT, [player | zone << 8, base_atk, base_def, cid]),
        ]
    }

    /// The sidecar's report of the monsters' stats after an engine step ({"type": "field", monsters: [{controller,
    /// sequence, position, level, attack, defense, base_attack, base_defense}]}): a face-up monster whose values
    /// differ from those its record last went with gets the record again, which DuelCardManager picks up after the
    /// next command (0x4FFF14) and shows on its label. A level of 0 (an Xyz monster's) keeps CARD_Prop's.
    fn field(&mut self, m: &Json) -> Vec<Message> {
        let mut out = vec![];
        for s in m["monsters"].as_array().into_iter().flatten() {
            let int = |k: &str| s[k].as_u64().unwrap_or(0) as u32;
            if int("position") as u64 & (POS_FACEUP_ATTACK | POS_FACEUP_DEFENSE) == 0 || int("sequence") >= 5 {
                continue;
            }
            let (player, zone) = (self.player(&s["controller"]), int("sequence"));
            let Some(card) = self.sides[player as usize].zones.get(&zone).and_then(|z| z.first()).copied() else { continue };
            let level = Some(int("level")).filter(|&l| l > 0).unwrap_or(self.cards.get(&card.row).map_or(0, |c| c.level));
            let values = [level, int("attack"), int("defense"), int("base_attack"), int("base_defense")];
            if self.records.get(&(player, zone)) != Some(&values) {
                out.extend(self.record(player, zone, card.row, values));
            }
        }
        out
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
        // field cards carry them; the client shows each list zone its own way. Only monsters lie sideways: a set
        // Spell/Trap comes face-down defense, which drew it like a set monster (live).
        // The Graveyard is face up: face down, its top card showed only its back (live).
        let position = match zone {
            GRAVE => POS_FACEUP_ATTACK,
            REMOVED => loc["position"].as_u64().unwrap_or(POS_FACEUP_ATTACK),
            zone if zone <= FIELD_SPELL => loc["position"].as_u64().unwrap_or(0),
            _ => 0,
        };
        Some(Place { player, zone, index, face_up: position & 5 != 0, defense: zone < 5 && position & 12 != 0 })
    }

    /// ChainSet for MSG_CHAINING {controller, location, sequence, chain_size}: the link over its card's zone (an Xyz
    /// material's over its monster).
    fn chain_link(&self, m: &Json) -> Message {
        let mut at = m.clone();
        at["location"] = json!(m["location"].as_u64().unwrap_or(0) & !LOC_OVERLAY);
        let zone = self.engine_place(&at).map_or(FIELD_CENTRE, |p| p.zone);
        command(CHAIN_SET, [self.player(&m["controller"]), zone, m["chain_size"].as_u64().unwrap_or(0) as u32, 0])
    }

    /// What to do with the engine's prompt (the last message when it awaits).
    pub fn prompt(&mut self, m: &Json) -> Prompt {
        let staged = self.staged.take();
        let prompt = self.prompt_for(m);
        match prompt {
            Prompt::Client(out) => {
                let mut all = self.unstage_n(staged);
                all.extend(out);
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
            if kind == MSG_ANNOUNCE_CARD {
                return Prompt::Answer(self.cpu_card_name());
            }
            return Prompt::Answer(cpu_answer(kind, m));
        }
        match kind {
            MSG_SELECT_IDLECMD | MSG_SELECT_BATTLECMD => {
                let (lists, buttons, mode): (Lists, [(&str, u8); 2], u32) = if kind == MSG_SELECT_IDLECMD {
                    (&IDLE_LISTS, [("to_bp", BUTTON_BP), ("to_ep", BUTTON_EP)], MODE_MAIN)
                } else {
                    (&BATTLE_LISTS, [("to_m2", BUTTON_M2), ("to_ep", BUTTON_EP)], MODE_BATTLE)
                };
                let mut map = self.command_map(m, lists);
                self.add_pile_views(&mut map);
                let flags: Vec<(u8, u8)> = buttons.into_iter().filter(|(k, _)| m[*k] == true).map(|(_, b)| (b, 1)).collect();
                self.pending = Some(m.clone());
                Prompt::Client(vec![command_data(&map, &flags), command(INPUT, [mode, CLIENT, 0, 0])])
            }
            MSG_SELECT_EFFECTYN | MSG_SELECT_YESNO => {
                let battle = BATTLE_QUESTIONS.iter().find(|(d, _)| m["description"].as_u64() == Some(*d)).map(|(_, text)| *text);
                let entries = match (kind, m["cid"].as_u64(), m["description_cid"].as_u64(), battle) {
                    (MSG_SELECT_EFFECTYN, cid, _, _) => vec![TEXT | ACTIVATE_TEXT, CARD_NAME | cid.unwrap_or(0) as u32],
                    (_, _, _, Some(text)) => vec![TEXT | text],
                    (_, _, Some(cid), _) => vec![TEXT | USE_TEXT, CARD_NAME | cid as u32],
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
            // Materials, one at a time: the card list until the engine lets the player finish and nothing is left.
            MSG_SELECT_UNSELECT_CARD if m["select_cards"].as_array().is_some_and(|s| !s.is_empty()) => {
                let open = m["can_finish"] == true || m["can_cancel"] == true;
                let list = json!({ "selects": m["select_cards"], "min": if open { 0 } else { 1 }, "max": 1 });
                let text = hint_text(std::mem::take(&mut self.hint));
                let out = self.card_list(&list, text);
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
            MSG_ANNOUNCE_RACE | MSG_ANNOUNCE_ATTRIB | MSG_ANNOUNCE_NUMBER | MSG_ANNOUNCE_CARD => match declaration(m) {
                Some(out) => {
                    self.pending = Some(m.clone());
                    Prompt::Client(out)
                }
                None if kind == MSG_ANNOUNCE_CARD => Prompt::Answer(self.cpu_card_name()),
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            // No window for these yet.
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// The CPU's card name declaration: the cards in the duel by turns, a new one each time it is asked.
    fn cpu_card_name(&mut self) -> Json {
        let mut cids: Vec<u16> =
            self.sides.iter().flat_map(|s| s.zones.values()).flatten().filter_map(|c| self.cards.get(&c.row)).map(|c| c.cid).collect();
        cids.sort_unstable();
        cids.dedup();
        let cid = cids.get(self.declared % cids.len().max(1)).copied().unwrap_or(0);
        self.declared += 1;
        json!({ "type": RESP_CARD_NAME, "cid": cid })
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
    /// needs: none for min 0, one for min 1, else exactly min (the list's max is lowered to it). A list holding a
    /// hidden card (a Set card has no row for the client) opens as the card-back kind with a table entry per card
    /// (flag set for the cards shown), so face-up cards keep their faces and Set cards are backs; no pick is needed
    /// there, so the list closing with nothing picked asks again when the choice is forced.
    fn card_list(&mut self, m: &Json, text: Option<u32>) -> Vec<Message> {
        let selects = m["selects"].as_array().map_or(&[][..], Vec::as_slice);
        let (min, max) = (m["min"].as_u64().unwrap_or(1) as u32, m["max"].as_u64().unwrap_or(1) as u32);
        // A choice the player may back out of (can_cancel, or a chain list that isn't forced) opens with OK, where no
        // pick cancels: the list's right-click cancel reaches no code in this client (0x4C20C0 calls an empty hook).
        let cancelable = m["can_cancel"] == true || (m["type"] == MSG_SELECT_CHAIN && m["forced"] != true);
        let min = if cancelable { 0 } else { min };
        let (kind, max) = match min {
            0 => (PICK_ANY, max),
            1 => (PICK_ONE, max),
            _ => (PICK_ALL, min),
        };
        let rows: Vec<u32> = selects.iter().map(|loc| self.list_row(loc)).collect();
        let mixed = rows.contains(&0) && min <= 1;
        self.mixed = mixed;
        let (kind, player) = if mixed { (PICK_MIXED, CPU) } else { (kind, CLIENT) };
        let marks: Vec<u32> = selects.iter().map(|loc| if self.player(&loc["controller"]) == CPU { OPPONENTS } else { 0 }).collect();
        let mut out = vec![];
        if mixed {
            for (i, &row) in rows.iter().enumerate() {
                let flag = if row != 0 { OPEN } else { 0 };
                out.push(command(CARD, [(STAGED_UID + i as u32) | flag, row, 0, table_place(CPU, REMOVED, i)]));
            }
            self.staged = Some(rows.len());
        }
        out.push(command(LIST_BEGIN, [selects.len() as u32, max, 0, 0]));
        out.extend(fours(LIST_ROWS, &rows));
        out.extend(fours(LIST_MARKS, &marks));
        if let Some(text) = text {
            out.extend(message(&[TEXT | text]));
        }
        out.push(command(CARD_LIST, [player, kind, text.is_some() as u32, 0]));
        out
    }

    /// The opponent's banished entries a mixed list overwrote (`n` of them), as they are: sent before anything else
    /// once it closes.
    fn unstage_n(&self, n: Option<usize>) -> Vec<Message> {
        let Some(n) = n else { return vec![] };
        let pile = self.sides[CPU as usize].zones.get(&REMOVED).map_or(&[][..], Vec::as_slice);
        pile.iter()
            .enumerate()
            .take(n)
            .map(|(i, c)| {
                let row = self.shown_row(&Place { player: CPU, zone: REMOVED, index: i, face_up: true, defense: false }, c.row);
                command(CARD, [c.uid | OPEN, row, 0, table_place(CPU, REMOVED, i)])
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
            if p.zone == DECK { zone.iter().find(has_cid) } else { zone.get(p.index).filter(has_cid) }.map(|c| c.row)
        });
        at_place.or_else(|| self.cards.iter().find(|(_, s)| s.cid == cid).map(|(&row, _)| row)).unwrap_or(0)
    }

    /// A pick in the card list (duelRequestName 7 {index: the list position}).
    pub fn pick(&mut self, index: u32) {
        // A list holds a few dozen entries: more picks than that are a client making the server keep them.
        if self.picks.len() < MAX_PICKS {
            self.picks.push(index as usize);
        }
    }

    /// A dialog closed (duelRequestName 4 {DLGResult}): the engine response for the pending question (the button: 1
    /// yes, 0 attack / 1 defense), option (its index) or card choice (`count` picks), or for a chain chance's Yes with
    /// several cards, the card list. No pick cancels when the engine allows it; picks that don't fit the choice get
    /// the default.
    pub fn dialog_result(&mut self, count: u32) -> Option<Prompt> {
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
        const DIALOGS: [u64; 12] = [
            MSG_SELECT_CARD,
            MSG_SELECT_UNSELECT_CARD,
            MSG_SELECT_TRIBUTE,
            MSG_SELECT_EFFECTYN,
            MSG_SELECT_YESNO,
            MSG_SELECT_POSITION,
            MSG_SELECT_OPTION,
            MSG_SELECT_CHAIN,
            MSG_ANNOUNCE_RACE,
            MSG_ANNOUNCE_ATTRIB,
            MSG_ANNOUNCE_NUMBER,
            MSG_ANNOUNCE_CARD,
        ];
        if !DIALOGS.contains(&kind) {
            return None;
        }
        let m = self.pending.take()?;
        let selects = m["selects"].as_array().or(m["select_cards"].as_array()).map_or(0, Vec::len);
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
            // A mixed list needs no picks to close: a forced choice with nothing picked is asked again.
            MSG_SELECT_CARD | MSG_SELECT_TRIBUTE if self.mixed && self.picks.is_empty() && m["can_cancel"] != true && m["min"].as_u64().unwrap_or(1) >= 1 => {
                return Some(self.prompt(&m));
            }
            MSG_SELECT_CARD | MSG_SELECT_TRIBUTE => self.card_choice(&m, count),
            MSG_SELECT_UNSELECT_CARD => match std::mem::take(&mut self.picks)[..] {
                [index] if index < selects => json!({ "type": RESP_UNSELECT_CARD, "index": index }),
                _ if m["can_finish"] == true || m["can_cancel"] == true => json!({ "type": RESP_UNSELECT_CARD, "index": null }),
                _ => cpu_answer(kind, &m),
            },
            MSG_ANNOUNCE_RACE => match TYPES.get((count as usize).wrapping_sub(1)) {
                Some(&race) if m["available"].as_u64().unwrap_or(0) & race != 0 => json!({ "type": RESP_RACE, "races": [race] }),
                _ => cpu_answer(kind, &m),
            },
            MSG_ANNOUNCE_ATTRIB => {
                let picked: Vec<u64> = ATTRIBUTES.iter().enumerate().filter(|(i, _)| count >> (i + 1) & 1 != 0).map(|(_, &a)| a).collect();
                let allowed = m["available"].as_u64().unwrap_or(0);
                match picked.len() as u64 == m["count"].as_u64().unwrap_or(1) && picked.iter().all(|a| allowed & a != 0) {
                    true => json!({ "type": RESP_ATTRIB, "attributes": picked }),
                    false => cpu_answer(kind, &m),
                }
            }
            MSG_ANNOUNCE_NUMBER => {
                let options: Vec<u64> = m["options"].as_array().into_iter().flatten().filter_map(Json::as_u64).collect();
                let unit = number_unit(&options).map_or(1, |(unit, _)| unit);
                let value = count as u64 * unit;
                // The first option at least as large (the dialog's steps may pass between uneven options).
                let index = options.iter().position(|&o| o >= value).unwrap_or(options.len().saturating_sub(1));
                json!({ "type": RESP_NUMBER, "value": index })
            }
            MSG_ANNOUNCE_CARD => json!({ "type": RESP_CARD_NAME, "cid": count }),
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
                if list == "pos_changes" && bit != self.position_change(place) {
                    continue;
                }
                // The client looks a pile's commands up at index 0 only (0x50B510, 0x5204E0).
                let place = if place >> 8 & 0xFF >= EXTRA { place & 0xFFFF } else { place };
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
            LOC_HAND => HAND,
            LOC_MZONE => 0,
            LOC_SZONE => 5,
            LOC_EXTRA => EXTRA,
            LOC_GRAVE => GRAVE,
            LOC_REMOVED => REMOVED,
            _ => return None,
        };
        if zone >= EXTRA {
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

    /// What the client's card command leads to: an attack with no monster to target asks "Attack your opponent
    /// directly?" first (225, the original's dialog: EDOPro asks only when the attacker could also target a monster;
    /// live, an attack on an empty field just went through), anything else answers the engine.
    pub fn command(&mut self, cmd: CardCommand) -> Option<Prompt> {
        let m = self.pending.clone()?;
        if cmd.command == CONFIRM
            && let Some(out) = self.pile_view(cmd.player, cmd.zone)
        {
            self.viewing = true;
            return Some(Prompt::Client(out));
        }
        if cmd.zone >= EXTRA && cmd.zone != DECK {
            return self.pile_command(&m, cmd);
        }
        let response = self.answer(cmd)?;
        let attack = &m["attacks"][response["index"].as_u64().unwrap_or(u64::MAX) as usize];
        let opponent = 1 - self.player(&attack["controller"]) as usize;
        let no_target = (0..5).all(|zone| self.sides[opponent].zones.get(&zone).is_none_or(Vec::is_empty));
        if response["type"] != RESP_BATTLECMD || response["action"] != BATTLE_ATTACK || attack["can_direct"] != true || !no_target {
            return Some(Prompt::Answer(response));
        }
        self.pending = Some(m);
        self.direct = Some(response);
        let mut out = message(&[TEXT | DIRECT_TEXT]);
        out.push(command(DIALOG, [YES_NO, FROM_ENTRIES, 0, 0]));
        Some(Prompt::Client(out))
    }

    /// A command on a pile (sent at index 0, where the client found it): the one card of the pile it fits, or with
    /// several, the card list asks which (the original's "Select the monster(s) to Special Summon from your Extra
    /// Deck.", 26).
    fn pile_command(&mut self, m: &Json, cmd: CardCommand) -> Option<Prompt> {
        let (lists, kind): (Lists, u64) = match m["type"].as_u64() {
            Some(MSG_SELECT_BATTLECMD) => (&BATTLE_LISTS, RESP_BATTLECMD),
            _ => (&IDLE_LISTS, RESP_IDLECMD),
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
                    (2, EXTRA) => SPSUMMON_EXTRA_TEXT,
                    (2, _) => SPSUMMON_TEXT,
                    _ => SELECT_EFFECT_TEXT,
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
        for (player, zone) in [(CLIENT, GRAVE), (CPU, GRAVE), (CLIENT, REMOVED), (CPU, REMOVED), (CLIENT, EXTRA)] {
            if self.sides[player as usize].zones.get(&zone).is_none_or(Vec::is_empty) {
                continue;
            }
            let place = table_place(player, zone, 0);
            match map.iter_mut().find(|(p, _)| *p == place) {
                Some((_, bits)) => *bits |= 1 << CONFIRM,
                None => map.push((place, 1 << CONFIRM)),
            }
        }
    }

    /// A pile's view: its cards from the bottom in a card list with no picks (max 0: dlg_cardlist.swf ignores card
    /// clicks, and a right-click closes it), under the pile's title.
    fn pile_view(&self, player: u32, zone: u32) -> Option<Vec<Message>> {
        let title = PILE_TITLES.iter().find(|(z, _)| *z == zone)?.1 + (player != CLIENT) as u32;
        if zone == EXTRA && player != CLIENT {
            return None;
        }
        let cards = self.sides.get(player as usize)?.zones.get(&zone).filter(|c| !c.is_empty())?;
        let rows: Vec<u32> = cards
            .iter()
            .enumerate()
            .map(|(index, c)| self.shown_row(&Place { player, zone, index, face_up: true, defense: false }, c.row))
            .collect();
        let mark = if player == CLIENT { 0 } else { OPPONENTS };
        let mut out = vec![command(LIST_BEGIN, [rows.len() as u32, 0, 0, 0])];
        out.extend(fours(LIST_ROWS, &rows));
        out.extend(fours(LIST_MARKS, &vec![mark; rows.len()]));
        out.extend(message(&[TEXT | title]));
        out.push(command(CARD_LIST, [CLIENT, PICK_ANY, 1, 0]));
        Some(out)
    }

    /// The engine response for the client's card command, if it names a listed card.
    pub fn answer(&mut self, cmd: CardCommand) -> Option<Json> {
        let m = self.pending.as_ref()?;
        let (lists, kind): (Lists, u64) = match m["type"].as_u64() {
            Some(MSG_SELECT_BATTLECMD) => (&BATTLE_LISTS, RESP_BATTLECMD),
            _ => (&IDLE_LISTS, RESP_IDLECMD),
        };
        // The client looks a card's commands up at (zone 0, index slot) for monsters and (zone 5, index slot) for
        // Spells/Traps (0x50A690), but sends the one picked with the card's own place, (zone slot, index 0) (live: an
        // attack from the second monster slot came as zone 1, index 0).
        let (zone, index) = match (cmd.zone, cmd.index) {
            (z @ 1..=4, 0) => (0, z as usize),
            (z @ 6..=9, 0) => (5, z as usize - 5),
            (FIELD_SPELL, 0) => (5, 5),
            (z, i) => (z, i as usize),
        };
        let place = table_place(cmd.player, zone, index);
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

/// Tokens' uids, after any deck's (a move carries the uid in 16 bits).
const FIRST_TOKEN_UID: u32 = 0x4000;

/// The table entry flag of a card in this zone: open in the Graveyard and among the banished cards.
fn open(zone: u32) -> u32 {
    if zone == GRAVE || zone == REMOVED { OPEN } else { 0 }
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
            MOVE => {
                if let Some(from) = slot(param(&out[i], "param2")) {
                    moved.remove(&from);
                }
                if let Some(to) = slot(param(&out[i], "param3")) {
                    moved.insert(to, (i, false));
                }
            }
            RUN_SUMMON | RUN_SP_SUMMON => moved.values_mut().for_each(|(_, effect)| *effect = true),
            FLIP_TURN => {
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
        system.or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| (label, SELECT_EFFECT_TEXT)))
    };
    let found: Option<Vec<(u32, u32)>> = options.iter().map(|o| label(o.as_u64()?)).collect();
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

/// The client's dialog for a declaration (MSG_ANNOUNCE_RACE / ATTRIB {count, available}, NUMBER {options}, CARD):
/// None when it can't ask it (several types at once, numbers outside the number dialog's range).
fn declaration(m: &Json) -> Option<Vec<Message>> {
    let kind = m["type"].as_u64()?;
    let allowed = m["available"].as_u64().unwrap_or(0);
    let bits = |table: &[u64]| table.iter().enumerate().filter(|(_, b)| allowed & **b != 0).fold(0u32, |f, (i, _)| f | 1 << (i + 1));
    let (dialog, text, param3) = match kind {
        MSG_ANNOUNCE_RACE if m["count"].as_u64() == Some(1) => (TYPE_DIALOG, TYPE_TEXT, bits(&TYPES)),
        MSG_ANNOUNCE_ATTRIB => (ATTRIBUTE_DIALOG, ATTRIBUTE_TEXT, bits(&ATTRIBUTES) << 16 | m["count"].as_u64().unwrap_or(1) as u32),
        MSG_ANNOUNCE_NUMBER => {
            let options: Vec<u64> = m["options"].as_array()?.iter().filter_map(Json::as_u64).collect();
            let (unit, format) = number_unit(&options)?;
            let (min, max) = (options.iter().min()? / unit, options.iter().max()? / unit);
            let step = options.windows(2).map(|w| w[1].abs_diff(w[0]) / unit).filter(|&d| d > 0).min().unwrap_or(1);
            if min > 15 || step > 15 || max > 255 {
                return None;
            }
            let param3 = (max as u32) << 24 | (step as u32) << 20 | (min as u32) << 16 | format;
            // Levels get "Declare monster level."; other numbers no message.
            if options.iter().all(|o| (1..=12).contains(o)) {
                let mut out = message(&[TEXT | LEVEL_TEXT]);
                out.push(command(DIALOG, [NUMBER_DIALOG, FROM_ENTRIES, param3, 0]));
                return Some(out);
            }
            return Some(vec![command(DIALOG, [NUMBER_DIALOG, 0, param3, 0])]);
        }
        MSG_ANNOUNCE_CARD => return Some(vec![command(LIST_BEGIN, [0, 1, 0, 0]), command(CARD_LIST, [CLIENT, CARD_SEARCH, 0, 0])]),
        _ => return None,
    };
    let mut out = message(&[TEXT | text]);
    out.push(command(DIALOG, [dialog, FROM_ENTRIES, param3, 0]));
    Some(out)
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
        HINTMSG_RELEASE => TRIBUTE_TEXT,
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
        518 => 101, // HINTMSG_EQUIP: "Select a @3monster@0 to @3equip@0."
        520 => 119, // HINTMSG_CONTROL: "Select the @3monster(s)@0 your opponent controls to take control."
        526 => 341, // HINTMSG_CONFIRM: "Select the @3card(s)@0 to show your opponent."
        528 => 357, // HINTMSG_POSCHANGE: "Select the @3monster@0 to change its battle position."
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

/// Indices into MSG_SELECT_SUM's `selects` that make its sum: `amount` is the target, `selects_must` cards count
/// without being picked, `select_max` 0 asks for exactly the amount and 1 for at least it with no card to spare
/// (Ritual Summons), `min`..`max` bound how many cards are picked. A card's `amount` is its level, or two levels
/// (low and high 16 bits) it may count as. The smallest set that fits is picked; none that fits: every card, which
/// the core refuses and the caller's repeated-prompt check then ends.
fn sum_choice(m: &Json) -> Vec<usize> {
    let level = |c: &Json| c["amount"].as_u64().unwrap_or(0);
    let must: Vec<u64> = m["selects_must"].as_array().into_iter().flatten().map(level).collect();
    let selects: Vec<u64> = m["selects"].as_array().into_iter().flatten().map(level).collect();
    let target = m["amount"].as_u64().unwrap_or(0);
    let greater = m["select_max"].as_u64().unwrap_or(0) != 0;
    let (min, max) = (m["min"].as_u64().unwrap_or(0) as usize, m["max"].as_u64().unwrap_or(u32::MAX as u64) as usize);
    let values = |a: u64| -> Vec<u64> { [a & 0xFFFF, a >> 16].into_iter().filter(|v| *v > 0).collect() };
    // Whether these cards (the must ones included) can count to the target.
    let fits = |chosen: &[u64]| -> bool {
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
    };
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
        MSG_SELECT_SUM => json!({ "type": RESP_SUM, "indicies": sum_choice(m) }),
        _ => {
            tracing::warn!("rules engine: no answer for prompt {kind}");
            json!({ "type": RESP_CHAIN, "index": null })
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
}
