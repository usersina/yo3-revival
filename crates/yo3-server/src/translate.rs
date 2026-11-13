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
const MSG_SHUFFLE_HAND: u64 = 33;
const MSG_EQUIP: u64 = 93;
const MSG_CARD_TARGET: u64 = 96;
const MSG_CANCEL_TARGET: u64 = 97;
const MSG_SWAP_GRAVE_DECK: u64 = 35;
const MSG_SHUFFLE_SET_CARD: u64 = 36;
const MSG_SWAP: u64 = 55;
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
/// HandShuffle {player}: the client rebuilds the player's hand from its card table, index by index (the card at
/// (player, hand, i) by 0x523DF0, its object by uid 0x500470; DuelCardManager 0x4FF000 case 0x12). Every index below
/// the hand's count needs an entry, or the client stops with an error state (0x1200).
const HAND_SHUFFLE: u32 = 0x12;
/// Card relations, which the client draws as markers on the related slots while the pointer is on a face-up field card
/// (0x50B700 → 0x50B1C0): 1017 {source place, target place, marker type 1-8, 1 add (0x5224E0) / 0 remove the entry with
/// all three (0x522530)} (pump 0x3F9), places as `player | zone << 8`; 0x26 redraws the hovered card's (DuelFieldManager
/// 0x50A690). See "Card relations" in docs/protocol.md.
const RELATION: u32 = 0x3F9;
const RELATIONS_REDRAW: u32 = 0x26;
/// The marker types used: an equip, the yellow ∞ (effect 0x35); a lasting target, the pink reticle (0x3B). A guess:
/// the client names none of its eight markers.
const MARK_EQUIP: u32 = 2;
const MARK_TARGET: u32 = 8;
/// CardSwap {_, place A, place B} as moves give them, both field places (zone < 11): the cards in the two slots trade
/// places, each taking the word of the place it lands on as its place and position (DuelCardManager 0x4FF000 case 0x1B:
/// the objects by 0x4F8240, 0x5000E0 with param_6 1, the slot array swapped). **Shape**: read in the code, not seen live.
const CARD_SWAP: u32 = 0x1B;
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
/// A battle (DuelBattle 0x4F8A90): start; the attack {attacker's player | zone << 8, target's player, target's zone or
/// DIRECT_TARGET}; damage figures per player, then end. Native BattleRun also accepts destroy flags, but those hide
/// the card: only a confirmed destruction move can do that, after any battle protection has applied.
const VIEW_BATTLE_INIT: u32 = 0xA;
const VIEW_BATTLE_ATTACK: u32 = 0xC;
const VIEW_BATTLE_RUN: u32 = 0xD;
const VIEW_BATTLE_END: u32 = 0xE;
/// A direct attack's target zone: any zone from 5 plays as one (the direct cut-in and strike, 0x4F8430 state 3);
/// 0xFFFFFFFF plays nothing. 16 is the field's centre, as ChainSet's.
const DIRECT_TARGET: u32 = 16;
/// The summon effects at a card {player, uid} (DuelScene 0x514290): Normal (or the player's ace), Special.
const RUN_SUMMON: u32 = 0x38;
/// Commands Konami's server sent that only show something (227 original duel logs), by their handlers:
/// FieldChange 7 {Field Spell's Konami id, or 0} changes the duel field's background (DuelScene → `0x50CD60`);
/// CardSet `0x1E` {player, zone, 1 for a monster} plays the Set effect (`0x22`, DuelScene); CardActivate `0x22`
/// {place as moves give it, Konami id, uid} plays a card's activation effect by its type (DuelScene, waited for);
/// the avatar speech cut-ins (DuelCutin `0x505A70`, each player's Avatar Settings lines; the client's duel option
/// "speech" turns them off): `0x43` {player, 1, 0} at the turn's draw, `0x44` {player, 0x8000 | uid, kind} a summon
/// (the "ace monster" line for the configured card, 8 a Synchro Summon's line), `0x47` {player, 0x8000 | uid, step}
/// an activation (steps 1 or 2 note how the card came, 3 and up speak; the "trump card" line for the configured card),
/// `0x48` {player, Konami id, face up, to a Spell/Trap zone} a card leaving the hand for the field (a Set speaks),
/// `0x4C` {player} the turn's end, `0x4D` {player, LP change, kind} damage (by the LP left; not for a cost or gain).
const FIELD_CHANGE: u32 = 7;
const CARD_SET: u32 = 0x1E;
const CARD_ACTIVATE: u32 = 0x22;
const SPEAK_DRAW: u32 = 0x43;
const SPEAK_SUMMON: u32 = 0x44;
const SPEAK_ACTIVATE: u32 = 0x47;
const SPEAK_PLAY: u32 = 0x48;
const SPEAK_TURN_END: u32 = 0x4C;
const SPEAK_DAMAGE: u32 = 0x4D;
/// LifeDamage `0x10`'s kinds (`param3 >> 8`), as in Konami's logs: 0 effect damage, 1 battle damage (after BattleRun),
/// 2 a cost, 4 a gain. DuelCockpit (`0x502EB0`) plays the recover sound for 4, the payment sound for 2, else damage's.
const DAMAGE_EFFECT: u32 = 0;
const DAMAGE_BATTLE: u32 = 1;
const DAMAGE_COST: u32 = 2;
const DAMAGE_GAIN: u32 = 4;
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
/// Message entry kinds 2 (a line break) and 8 (a number put into the text's `%d`, `0x519D60`), and the duel prompt
/// text "Select %d card(s)."
const LINE_BREAK: u32 = 2 << 28;
const NUMBER: u32 = 8 << 28;
const SELECT_COUNT_TEXT: u32 = 592;
/// "@3Tribute@0 the necessary @3monsters@0 to match the required number of Level." (the duel prompt table's 207).
const SUM_TEXT: u32 = 207;
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
    chain_list: bool,
    /// The chain's last link (MSG_CHAINING) until the chain ends, and each link's {player, zone} by its number - 1.
    chaining: Option<Json>,
    chain: Vec<(u32, u32)>,
    /// A battle shown on the client and not ended yet, and whether the duel is in a Battle Phase.
    in_battle: bool,
    battle_phase: bool,
    /// The last selection hint (HINTMSG_*) and the client's picks in the card list so far.
    hint: u64,
    picks: Vec<usize>,
    /// The duel's result for the client (1 won, 2 lost, 3 draw), once it ended.
    result: Option<u32>,
    /// By player, whether that side's hidden cards (deck, hand, extra deck, face-down) go to the client without their
    /// rows: the opponent's against a player or an NPC (`duel::npc_translator`), both for a spectator, none for a
    /// translator that shows everything (`new`: duel log replays, the duel check's view of what the client isn't told).
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
                let zone = self.sides[p].zone(EXTRA);
                zone.push(Card { uid, row });
                let shown = if self.hidden[p] { 0 } else { row as u32 };
                out.push(command(CARD, [uid, shown, 0, table_place(p as u32, EXTRA, zone.len() - 1)]));
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
                    // The turn's end speaks for the player whose turn it was.
                    if self.turns > 0 {
                        out.push(command(SPEAK_TURN_END, [self.turn_player, 0, 0, 0]));
                    }
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
                        // The turn player's draw line, as the Draw Phase starts.
                        if phase == 0 {
                            out.push(command(SPEAK_DRAW, [self.turn_player, 1, 0, 0]));
                        }
                    }
                }
                Some(MSG_MOVE) => out.extend(self.move_card(m)),
                Some(MSG_HINT) if m["hint_type"].as_u64() == Some(HINT_SELECTMSG) && self.player(&m["player"]) == CLIENT => {
                    self.hint = m["hint"].as_u64().unwrap_or(0);
                }
                // Also a Set Spell/Trap turning face-up as it is activated.
                Some(MSG_POS_CHANGE | MSG_FLIPSUMMONING) => out.extend(self.turn(m, m["position"].as_u64().unwrap_or(0))),
                Some(MSG_CHAINING) => {
                    out.extend(self.activation(m));
                    let link = self.chain_link(m);
                    let param = |k: &str| link.get(k).and_then(Value::as_u32).unwrap_or(0);
                    self.chain.push((param("param1"), param("param2")));
                    out.push(link);
                    self.chaining = Some(m.clone());
                }
                // The link's own {player, zone, link}, as its ChainSet (Konami's logs; the client reads only the link).
                Some(MSG_CHAIN_SOLVING) => {
                    let n = m["chain_size"].as_u64().unwrap_or(0) as u32;
                    let (player, zone) = self.chain.get((n as usize).wrapping_sub(1)).copied().unwrap_or((0, 0));
                    out.push(command(CHAIN_RUN, [player, zone, n, 0]));
                }
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
                    // The place as moves give it, with its face and position bits (Konami's logs: 0x4000 | place for a
                    // Normal Summon, 0x4000/0xC000/0x8000 for a Special Summon).
                    if let Some((place, uid)) = card {
                        // The summon line first: 0 a Normal Summon, 5 a Special Summon (Konami's kinds).
                        let kind = if kind == MSG_SUMMONING { 0 } else { 5 };
                        out.push(command(SPEAK_SUMMON, [place.player, 0x8000 | uid, kind, 0]));
                        out.push(command(id, [Place { index: 0, ..place }.packed(), uid, 0, 0]));
                    }
                }
                Some(MSG_CONFIRM_CARDS) => out.extend(self.confirm(m)),
                Some(MSG_SHUFFLE_HAND) => out.extend(self.shuffle_hand(m)),
                Some(MSG_SWAP) => out.extend(self.swap(m)),
                Some(MSG_SHUFFLE_SET_CARD) => out.extend(self.shuffle_set(m)),
                Some(MSG_SWAP_GRAVE_DECK) => out.extend(self.swap_grave_deck(m)),
                Some(MSG_CHAIN_END) => {
                    self.chaining = None;
                    self.chain.clear();
                }
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
                    let damage = match kind {
                        MSG_RECOVER => DAMAGE_GAIN,
                        MSG_PAY_LPCOST => DAMAGE_COST,
                        _ if self.in_battle => DAMAGE_BATTLE,
                        _ => DAMAGE_EFFECT,
                    };
                    // As Konami's server sent it: the change shown (param3's low byte 1: its number and sound), the
                    // damage line, then the change applied (low byte 0). One message with 0 changed the LP silently.
                    let p = self.player(&m["player"]);
                    out.push(command(CHANGE_LP, [p, change as u32, damage << 8 | 1, 0]));
                    out.push(command(SPEAK_DAMAGE, [p, change as u32, damage, 0]));
                    out.push(command(CHANGE_LP, [p, change as u32, damage << 8, 0]));
                }
                Some(kind @ (MSG_ADD_COUNTER | MSG_REMOVE_COUNTER)) => out.extend(self.counter(m, kind == MSG_ADD_COUNTER)),
                Some(MSG_LPUPDATE) => {
                    out.push(command(SET_LP, [self.player(&m["player"]), m["lp"].as_u64().unwrap_or(0) as u32, 0, 0]));
                }
                Some(kind @ (MSG_EQUIP | MSG_CARD_TARGET | MSG_CANCEL_TARGET)) => self.link(m, kind),
                _ => {}
            }
        }
        out.extend(self.sync_relations());
        with_card_ids(&self.cards, &mut out);
        out
    }

    /// MSG_EQUIP {card, target} (an equip's new monster: its earlier link goes), MSG_CARD_TARGET {card, target} (a
    /// lasting target) and MSG_CANCEL_TARGET (its end): the cards' links, by uid.
    fn link(&mut self, m: &Json, kind: u64) {
        let uid = |t: &Self, loc: &Json| {
            let place = t.engine_place(loc).filter(|p| p.zone <= FIELD_SPELL)?;
            t.sides[place.player as usize].zones.get(&place.zone)?.first().map(|c| c.uid)
        };
        let (Some(card), Some(target)) = (uid(self, &m["card"]), uid(self, &m["target"])) else { return };
        match kind {
            MSG_EQUIP => {
                self.links.retain(|&(c, _, mark)| !(c == card && mark == MARK_EQUIP));
                self.links.push((card, target, MARK_EQUIP));
            }
            MSG_CARD_TARGET if !self.links.contains(&(card, target, MARK_TARGET)) => self.links.push((card, target, MARK_TARGET)),
            MSG_CANCEL_TARGET => self.links.retain(|&l| l != (card, target, MARK_TARGET)),
            _ => {}
        }
    }

    /// The client's relations brought in line with the links, both ways round so either card shows it: a link whose
    /// card left the field goes, one whose cards moved is re-sent at their new places. Removals, additions, then a redraw.
    fn sync_relations(&mut self) -> Vec<Message> {
        let place_of = |t: &Self, uid: u32| {
            (0..2u32).find_map(|p| {
                (0..=FIELD_SPELL).find(|z| t.sides[p as usize].zones.get(z).and_then(|c| c.first()).is_some_and(|c| c.uid == uid)).map(|z| p | z << 8)
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
            .map(|&(a, b, mark)| command(RELATION, [a, b, mark, 0]))
            .collect();
        out.extend(wanted.iter().filter(|r| !self.relations.contains(r)).map(|&(a, b, mark)| command(RELATION, [a, b, mark, 1])));
        if !out.is_empty() {
            out.push(command(RELATIONS_REDRAW, [0; 4]));
        }
        self.relations = wanted;
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
        if is_material(&m["to"]) && !is_material(&m["from"]) {
            return self.attach(m, cid);
        }
        if is_material(&m["from"]) {
            return self.engine_place(&m["to"]).map(|to| self.detach(cid, to)).unwrap_or_default();
        }
        if m["from"]["location"].as_u64() == Some(0) {
            return self.engine_place(&m["to"]).map(|to| self.token(m, to)).unwrap_or_default();
        }
        if let Some(from) = self.engine_place(&m["from"]).filter(|f| f.zone < 5)
            && let Some(card) = self.sides[from.player as usize].zones.get(&from.zone).and_then(|z| z.first()).copied()
            && self.tokens.contains(&card.uid)
            && !self.engine_place(&m["to"]).is_some_and(|to| to.zone <= FIELD_SPELL)
        {
            // A token leaving the field is gone (EDOPro may send it to a pile first): it shatters off the field. One that
            // changes sides (a control change) stays and moves like any card.
            let mut out = self.drop_counters(&from);
            self.sides[from.player as usize].zone(from.zone).clear();
            self.tokens.retain(|&t| t != card.uid);
            out.push(command(CARD_VANISH, [from.player, from.zone, 0, 0]));
            return out;
        }
        match (self.engine_place(&m["from"]), self.engine_place(&m["to"])) {
            (Some(from), Some(to)) => {
                let reason = m["reason"].as_u64().unwrap_or(0);
                let dropped = if to.zone > FIELD_SPELL { self.drop_counters(&from) } else { vec![] };
                let kind = move_kind(reason, from, to);
                let mut moved = self.relocate_as(cid, from, to, kind);
                if !moved.is_empty() {
                    moved = self.presented(moved, cid, from, to, kind);
                }
                if to.zone == HAND && let Some(at) = self.revealing.iter().position(|&c| c == cid) {
                    self.revealing.remove(at);
                    let index = self.sides[to.player as usize].zone(HAND).len().saturating_sub(1).min(to.index);
                    moved.extend(self.show_hand(to.player, index));
                }
                // Only an actual destruction move confirms the card died. MSG_BATTLE precedes limited battle
                // protection (Fortress Warrior, Shield Wing), so its tentative destroy flag cannot hide the card.
                if moved.is_empty() || reason & REASON_DESTROY == 0 || from.zone > HAND {
                    return dropped.into_iter().chain(moved).collect();
                }
                let vanish = command(CARD_VANISH, [from.player, from.zone | (from.index as u32) << 8, 0, 0]);
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
        let hand = self.sides[p as usize].zone(HAND);
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
                let row = self.shown_row(&Place { player: p, zone: HAND, index: i, face_up: false, defense: false }, c.row);
                command(CARD, [c.uid, row, 0, table_place(p, HAND, i)])
            })
            .collect();
        out.push(command(HAND_SHUFFLE, [p, 0, 0, 0]));
        out
    }

    /// A field card becomes an Xyz material: the client has no place under a monster, so the card leaves the field with
    /// CardVanish (0x1F: marked off the field and hidden, 0x4FC520; the slot keeps the object until another card lands
    /// there) and the mirror keeps it among the materials. **Shape**: the shatter effect plays for it. From elsewhere (a
    /// hand or pile) it isn't followed (logged).
    fn attach(&mut self, m: &Json, cid: u16) -> Vec<Message> {
        let Some(from) = self.engine_place(&m["from"]) else { return vec![] };
        if from.zone > FIELD_SPELL {
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
        out.push(command(CARD_VANISH, [from.player, from.zone, 0, 0]));
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
        let empty = |s: &Self, p: u32| (0..=FIELD_SPELL).find(|&z| empty_at(s, p, z)).map(|z| (p, z));
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
        while let Some(card) = self.sides[p as usize].zones.get(&GRAVE).and_then(|z| z.last()).copied() {
            let cid = cid_of(self, &card);
            let index = self.sides[p as usize].zone(GRAVE).len() - 1;
            let in_extra = self.sides[p as usize].zones.get(&EXTRA).map_or(0, |z| z.iter().filter(|c| cid_of(self, c) == cid).count());
            let to = if extra.iter().filter(|&&c| c == cid).count() > in_extra {
                Place { face_up: false, ..at(EXTRA, self.sides[p as usize].zone(EXTRA).len()) }
            } else {
                Place { face_up: false, ..at(DECK, 0) }
            };
            out.extend(self.relocate(cid, at(GRAVE, index), to));
        }
        for (i, cid) in grave.into_iter().enumerate() {
            out.extend(self.relocate(cid, Place { face_up: false, ..at(DECK, 0) }, at(GRAVE, i)));
        }
        out
    }

    /// MSG_SWAP {card1, card2}: two field cards trade places (Creature Swap: two monsters change sides), each keeping its
    /// own position. Which mirror card is the engine's card1 is told by its id; then [`trade`](Self::trade).
    fn swap(&mut self, m: &Json) -> Vec<Message> {
        let (Some(p1), Some(p2)) = (self.engine_place(&m["card1"]), self.engine_place(&m["card2"])) else { return vec![] };
        if p1.zone > FIELD_SPELL || p2.zone > FIELD_SPELL {
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
            .filter(|(p, _)| p.zone <= FIELD_SPELL)
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
                side.zones.iter().find(|(z, cards)| **z <= FIELD_SPELL && cards.first().is_some_and(|c| c.uid == uid)).map(|(z, _)| (p as u32, *z))
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
            (Some(_), Some(_)) => vec![command(CARD_SWAP, [0, a.packed(), b.packed(), 0])],
            (Some(c), None) => vec![command(MOVE, [c.uid, Place { face_up: false, defense: false, ..a }.packed(), b.packed(), 0])],
            (None, Some(c)) => vec![command(MOVE, [c.uid, Place { face_up: false, defense: false, ..b }.packed(), a.packed(), 0])],
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
                    out.push(command(CARD, [c.uid | open(place.zone), row, 0, table_place(place.player, place.zone, 0)]));
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
        // A token with a Konami id (Doomsday Token, 3940) is the client's card of that id; one without, by its name.
        let cid = m["cid"].as_u64().filter(|&c| c != 0).map(|c| c as u16);
        let by_id: Option<Vec<u16>> = cid.map(|c| {
            let mut rows: Vec<u16> = self.cards.iter().filter(|(_, s)| s.cid == c).map(|(&r, _)| r).collect();
            rows.sort_unstable();
            rows
        });
        let by_name = || m["name"].as_str().and_then(|n| self.token_rows.get(n)).cloned();
        let Some(rows) = by_id.filter(|r| !r.is_empty()).or_else(by_name).filter(|r| !r.is_empty()) else {
            tracing::warn!("a token the client has no card for: {} {}", m["cid"], m["name"]);
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
        out.push(command(MOVE, [uid | MOVE_SPECIAL_SUMMON << 16, to.packed(), to.packed(), 0]));
        out
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
        if from.zone == HAND && to.zone <= FIELD_SPELL && matches!(kind, MOVE_NORMAL_SUMMON | MOVE_ACTIVATE | MOVE_SET) {
            // The speech never reads the id; a Set card the client may not see goes without it.
            let shown = if kind == MOVE_SET && self.hidden[to.player as usize] { 0 } else { cid as u32 };
            out.push(command(SPEAK_PLAY, [from.player, shown, (kind != MOVE_SET) as u32, (to.zone >= 5) as u32]));
        }
        out.extend(moved);
        if kind == MOVE_SET && to.zone <= FIELD_SPELL {
            out.push(command(CARD_SET, [to.player, to.zone, (to.zone < 5) as u32, 0]));
        }
        if to.zone == FIELD_SPELL && to.face_up {
            out.push(command(FIELD_CHANGE, [cid as u32, 0, 0, 0]));
        } else if from.zone == FIELD_SPELL && from.face_up && to.zone != FIELD_SPELL {
            out.push(command(FIELD_CHANGE, [0; 4]));
        }
        out
    }

    /// An activation's presentation before its ChainSet, as Konami's server sent it: the activation line (`0x47`; for a
    /// Spell/Trap step 2 then 3, as from the hand, since a Set card is face up by its MSG_CHAINING; Konami's sent 1 for
    /// a Set one, which the client only keeps as context) and the card's activation effect (`0x22`). Nothing for a card
    /// the client has no place for.
    fn activation(&self, m: &Json) -> Vec<Message> {
        let mut at = m.clone();
        at["location"] = json!(m["location"].as_u64().unwrap_or(0) & !LOC_OVERLAY);
        let Some(place) = self.engine_place(&at) else { return vec![] };
        let index = if place.zone <= FIELD_SPELL { 0 } else { place.index };
        let Some(card) = self.sides[place.player as usize].zones.get(&place.zone).and_then(|z| z.get(index)).copied() else { return vec![] };
        let cid = self.cards.get(&card.row).map_or(0, |c| c.cid as u32);
        let mut out = vec![];
        if (5..=FIELD_SPELL).contains(&place.zone) {
            out.push(command(SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 2, 0]));
            out.push(command(SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 3, 0]));
        } else {
            out.push(command(SPEAK_ACTIVATE, [place.player, 0x8000 | card.uid, 4, 0]));
        }
        let place = Place { index, face_up: true, ..place };
        out.push(command(CARD_ACTIVATE, [place.packed(), cid, card.uid, 0]));
        out
    }

    /// [`relocate`] with the move's kind (`move_kind`), which the client's move takes in `param1`'s high half.
    fn relocate_as(&mut self, cid: u16, from: Place, to: Place, kind: u32) -> Vec<Message> {
        let record = (from.zone <= FIELD_SPELL).then(|| self.records.get(&(from.player, from.zone)).copied()).flatten();
        let cards = &self.cards;
        let source = self.sides[from.player as usize].zone(from.zone);
        let at = if from.zone == DECK || from.zone == EXTRA {
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
        let from = if from.zone == DECK || from.zone == EXTRA { from } else { Place { index: at, ..from } };
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
            out.extend(match record {
                Some(values) => self.record(to.player, to.zone, card.row, values),
                None => self.monster_record(to.player, to.zone, card.row),
            });
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
        out.push(command(MOVE, [card.uid | kind << 16, from.packed(), to.packed(), 0]));
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
    /// would be destroyed. Limited protection is applied after this message, so destruction is shown only when
    /// MSG_MOVE confirms it. The client pairs the values with the cards by player (DuelBattle states 5 and 6). A direct
    /// attack's target is an empty place (location 0; ocgcore-wasm parses it like any card): no figures, since there
    /// is no target card to show them at (the LP change shows the damage).
    fn battle(&self, m: &Json) -> Vec<Message> {
        let (card, target) = (&m["card"], &m["target"]);
        if target["location"].as_u64().unwrap_or(0) == 0 {
            return vec![command(VIEW_BATTLE_RUN, [0; 4])];
        }
        let n = |c: &Json, k: &str| c[k].as_i64().unwrap_or(0);
        let (to_attacker, to_target) = if target["position"].as_u64().unwrap_or(0) & (POS_FACEUP_ATTACK | POS_FACEDOWN_ATTACK) != 0 {
            let diff = n(card, "attack") - n(target, "attack");
            (-diff.min(0), diff.max(0))
        } else {
            ((n(target, "defense") - n(card, "attack")).max(0), 0)
        };
        let mut damage = [0u32; 2];
        let attacker = self.player(&card["controller"]) as usize;
        damage[attacker] = to_attacker as u32;
        damage[1 - attacker] = to_target as u32;
        vec![command(VIEW_BATTLE_RUN, [damage[0], damage[1], 0, 0])]
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
                out.push(command(MOVE, [card.uid, down.packed(), place.packed(), 0]));
                return out;
            }
        }
        // A player's own monster turned in place stayed upright on that player's screen (live, the VM round: every
        // 0x1C after its summon), while a card arriving by a move is drawn in its position: it moves onto its own
        // slot in the new position first.
        if let (Some(before), Some(card)) = (before, card) {
            let (face_up, defense) = before.unwrap_or((!place.face_up, !place.defense));
            let from = Place { face_up, defense, ..place };
            out.push(command(MOVE, [card.uid, from.packed(), place.packed(), 0]));
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
        // An Xyz material: ocgcore-wasm gives its Xyz Monster's location with an `overlay_sequence` (a material attached
        // in the Extra Deck read as the Extra Deck, one detached as its monster's zone); the client has no place for it.
        if is_material(loc) {
            return None;
        }
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
            if kind == MSG_ANNOUNCE_CARD {
                return Prompt::Answer(self.cpu_card_name(m));
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
            // A sum (tributes or discards that make a total Level): the card list, as many picks as the bounds allow;
            // picks that don't make the sum bring the list back. Only when some picks can make it: a list nothing
            // satisfies would hold the player forever, so that one gets the automatic answer (and the core's
            // refusals end the duel, see `Repeats`).
            MSG_SELECT_SUM if m["selects"].as_array().is_some_and(|s| !s.is_empty()) && sum_fits(m, &sum_choice(m)) => {
                // Kind 1 (min 0): any number of picks, confirmed with a right-click (dlg_cardlist.swf has no OK
                // button). A list of kind 0x27 with a larger max closes only at max picks (live: Machina Fortress's
                // discards held the player at two picks of six).
                let n = m["selects"].as_array().map_or(0, Vec::len);
                let (_, max) = sum_bounds(m);
                let list = json!({ "selects": m["selects"], "min": 0, "max": max.min(n).max(1) });
                // The engine's hint (Machina Fortress: "Select the card(s) in your hand you want to discard."), kept for
                // the list shown again, else the prompt table's 207 "Tribute the necessary monsters to match the
                // required number of Level.".
                let text = hint_text(std::mem::take(&mut self.hint)).or(self.sum_text).unwrap_or(SUM_TEXT);
                self.sum_text = Some(text);
                let text = Some(text);
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
                // Options the client has no labels for: the card list, the option's card once per option in the
                // engine's order, rather than the first option for the player.
                None if m["options"].as_array().is_some_and(|o| o.len() > 1) => {
                    let card = |i: usize| m["option_cids"][i].as_u64().filter(|&c| c != 0)
                        .or_else(|| self.chaining.as_ref().and_then(|c| c["cid"].as_u64()))
                        .unwrap_or(0);
                    let selects: Vec<Json> = (0..m["options"].as_array().map_or(0, Vec::len))
                        .map(|i| json!({ "controller": m["player"], "cid": card(i) }))
                        .collect();
                    let out = self.card_list(&json!({ "selects": selects, "min": 1, "max": 1 }), Some(SELECT_EFFECT_TEXT));
                    self.option_cards = true;
                    self.picks.clear();
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
                None if kind == MSG_ANNOUNCE_CARD => Prompt::Answer(self.cpu_card_name(m)),
                None => Prompt::Answer(cpu_answer(kind, m)),
            },
            // No window for these yet.
            _ => Prompt::Answer(cpu_answer(kind, m)),
        }
    }

    /// The CPU's card name declaration: the cards in the duel by turns, a new one each time it is asked.
    fn cpu_card_name(&mut self, m: &Json) -> Json {
        // A card the effect's filter allows (the sidecar's `fits`, the duel's own cards first).
        if let Some(cid) = m["fits"].as_array().and_then(|f| f.first()).and_then(Json::as_u64) {
            return json!({ "type": RESP_CARD_NAME, "cid": cid });
        }
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
        // A choice the player may back out of (can_cancel, or a chain list that isn't forced) opens as kind 1: the list
        // (no OK button) closes on a right-click with the picks so far, none cancelling (dlg_cardlist.swf `onMouseUp`
        // with `_cancelable`, which 0x4C73A0 always sets).
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
        // A list that needs several picks says how many (min == max here): the hint alone left a player of Magical Hats
        // with one card marked and no hint that a second one decides.
        let several = m["min"].as_u64().unwrap_or(1) >= 2 && max >= 2 && kind != PICK_MIXED;
        let entries: Vec<u32> = match (text, several) {
            (Some(text), true) => vec![TEXT | text, LINE_BREAK, TEXT | SELECT_COUNT_TEXT, NUMBER | max],
            (None, true) => vec![TEXT | SELECT_COUNT_TEXT, NUMBER | max],
            (Some(text), false) => vec![TEXT | text],
            (None, false) => vec![],
        };
        if !entries.is_empty() {
            out.extend(message(&entries));
        }
        out.push(command(CARD_LIST, [player, kind, !entries.is_empty() as u32, 0]));
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
        let prompt = self.dialog_result_for(count);
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
        const DIALOGS: [u64; 13] = [
            MSG_SELECT_SUM,
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
            MSG_SELECT_OPTION if std::mem::take(&mut self.option_cards) => match std::mem::take(&mut self.picks)[..] {
                [index] if index < m["options"].as_array().map_or(0, Vec::len) => json!({ "type": RESP_OPTION, "index": index }),
                _ => cpu_answer(kind, &m),
            },
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
            MSG_SELECT_SUM => {
                let picks = std::mem::take(&mut self.picks);
                if !sum_fits(&m, &picks) {
                    return Some(self.prompt(&m));
                }
                self.sum_text = None;
                json!({ "type": RESP_SUM, "indicies": picks })
            }
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
        if cmd.command == CONFIRM
            && let Some(out) = self.pile_view(cmd.player, cmd.zone)
        {
            self.viewing = true;
            return Some(Prompt::Client(out));
        }
        if cmd.zone >= EXTRA && cmd.zone != DECK {
            return self.pile_command(&m, cmd);
        }
        // A card that offers this command more than once (two effects it can activate now): the card list asks which,
        // the card once per effect in the engine's order.
        if let Some((list, kind, action, indices)) = self.listed(&cmd).filter(|l| l.3.len() > 1) {
            let selects: Vec<Json> = indices.iter().map(|&i| m[list][i].clone()).collect();
            let out = self.card_list(&json!({ "selects": selects, "min": 0, "max": 1 }), Some(SELECT_EFFECT_TEXT));
            self.choosing = Some((kind, action, indices));
            self.picks.clear();
            return Some(Prompt::Client(out));
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

    /// The duel check's round trip for the pending idle or battle prompt: every answer the engine offers that the
    /// client can't give through its menus, described. The client's side is played on copies of the translator: each
    /// command of the command map as the client sends it (a field card from its own slot, `answer`), the card list a
    /// pile's command opens and each of its picks, the direct attack's Yes, and the phase buttons. Empty when every
    /// option is reachable (or nothing such is pending).
    pub fn unreachable_answers(&self) -> Vec<String> {
        let Some(m) = self.pending.as_ref() else { return vec![] };
        let (lists, kind, phases): (Lists, u64, &[(&str, u64)]) = match m["type"].as_u64() {
            Some(MSG_SELECT_IDLECMD) => (&IDLE_LISTS, RESP_IDLECMD, &[("to_bp", IDLE_TO_BP), ("to_ep", IDLE_TO_EP)]),
            Some(MSG_SELECT_BATTLECMD) => (&BATTLE_LISTS, RESP_BATTLECMD, &[("to_m2", BATTLE_TO_M2), ("to_ep", BATTLE_TO_EP)]),
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
                (5, 5) => (FIELD_SPELL, 0),
                (5, i @ 0..=4) => (5 + i, 0),
                other => other,
            };
            for bit in (0..32).filter(|b| bits >> b & 1 != 0) {
                let mut t = self.clone();
                let p = t.command(CardCommand { command: bit, player, zone, index });
                reach(p, &t);
            }
        }
        for button in [BUTTON_BP, BUTTON_M2, BUTTON_EP] {
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

    /// [`unreachable_answers`] for the other windows: each card of a card choice (one pick, when one may be picked),
    /// a chain chance's Yes for each card and its No, a question's Yes and No, and each option.
    fn unreachable_choices(&self, m: &Json) -> Vec<String> {
        let kind = m["type"].as_u64().unwrap_or(0);
        let len = |k: &str| m[k].as_array().map_or(0, Vec::len);
        // What, the clicks (each: the picks, then the dialog's result), the answer expected.
        type Case = (String, Vec<(Vec<u32>, u32)>, Json);
        let mut cases: Vec<Case> = vec![];
        match kind {
            MSG_SELECT_CARD | MSG_SELECT_TRIBUTE => {
                let response = if kind == MSG_SELECT_TRIBUTE { RESP_TRIBUTE } else { RESP_CARD };
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
            MSG_SELECT_UNSELECT_CARD => {
                for i in 0..len("select_cards") as u32 {
                    cases.push((format!("select_cards[{i}]"), vec![(vec![i], 1)], json!({ "type": RESP_UNSELECT_CARD, "index": i })));
                }
                if m["can_cancel"] == true || m["can_finish"] == true {
                    let what = if m["can_cancel"] == true { "cancelling" } else { "finishing" };
                    cases.push((what.into(), vec![(vec![], 0)], json!({ "type": RESP_UNSELECT_CARD, "index": null })));
                }
            }
            MSG_SELECT_CHAIN => {
                let n = len("selects") as u32;
                for i in 0..n {
                    // Yes, then the card list when there are several (or a forced chain's list right away).
                    let clicks = match (m["forced"] == true, n) {
                        (true, _) => vec![(vec![i], 1)],
                        (false, 1) => vec![(vec![], 1)],
                        (false, _) => vec![(vec![], 1), (vec![i], 1)],
                    };
                    cases.push((format!("selects[{i}] (chain)"), clicks, json!({ "type": RESP_CHAIN, "index": i })));
                }
                if m["forced"] != true {
                    cases.push(("passing the chain".into(), vec![(vec![], 0)], json!({ "type": RESP_CHAIN, "index": null })));
                }
            }
            MSG_SELECT_SUM => {
                let picks: Vec<u32> = sum_choice(m).into_iter().map(|i| i as u32).collect();
                let n = picks.len() as u32;
                cases.push((format!("the sum {picks:?}"), vec![(picks.clone(), n)], json!({ "type": RESP_SUM, "indicies": picks })));
            }
            MSG_SELECT_EFFECTYN | MSG_SELECT_YESNO => {
                let response = if kind == MSG_SELECT_EFFECTYN { RESP_EFFECTYN } else { RESP_YESNO };
                cases.push(("yes".into(), vec![(vec![], 1)], json!({ "type": response, "yes": true })));
                cases.push(("no".into(), vec![(vec![], 0)], json!({ "type": response, "yes": false })));
            }
            MSG_SELECT_OPTION => {
                for i in 0..len("options") as u32 {
                    let clicks = if self.option_cards { vec![(vec![i], 1)] } else { vec![(vec![], i)] };
                    cases.push((format!("options[{i}]"), clicks, json!({ "type": RESP_OPTION, "index": i })));
                }
            }
            _ => {}
        }
        cases
            .into_iter()
            .filter(|(_, clicks, expected)| {
                let mut t = self.clone();
                let mut last = None;
                for (picks, result) in clicks {
                    for &p in picks {
                        t.pick(p);
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
const REASON_RELEASE: u64 = 0x2;
const REASON_SUMMON: u64 = 0x10;
const REASON_BATTLE: u64 = 0x20;
const REASON_SPSUMMON: u64 = 0x800;

/// A move's kind (see [`MOVE_PLAIN`]) from MSG_MOVE's reason and the places.
fn move_kind(reason: u64, from: Place, to: Place) -> u32 {
    if reason & REASON_DESTROY != 0 {
        return if reason & REASON_BATTLE != 0 { MOVE_DESTROYED_BY_BATTLE } else { MOVE_DESTROYED };
    }
    if reason & REASON_RELEASE != 0 {
        return MOVE_TRIBUTED;
    }
    match (from.zone, to.zone) {
        (_, 0..5) if reason & REASON_SPSUMMON != 0 => MOVE_SPECIAL_SUMMON,
        (HAND, 0..5) if !to.face_up => MOVE_SET,
        (HAND, 0..5) if reason & REASON_SUMMON != 0 => MOVE_NORMAL_SUMMON,
        (HAND, 5..=FIELD_SPELL) if to.face_up => MOVE_ACTIVATE,
        (HAND, 5..=FIELD_SPELL) => MOVE_SET,
        _ => MOVE_PLAIN,
    }
}

/// Card table entries (1000) carry the card's Konami id in `param3`, as Konami's server sent them (all 53,586 entries of
/// 227 original duel logs); a hidden card (row 0) has none. The client keeps it in its card table (`0x521DF0`).
fn with_card_ids(cards: &HashMap<u16, CardStats>, out: &mut [Message]) {
    for m in out.iter_mut().filter(|m| m.get("id").and_then(Value::as_u32) == Some(CARD)) {
        let row = m.get("param2").and_then(Value::as_u32).unwrap_or(0) as u16;
        let Some(cid) = cards.get(&row).map(|c| c.cid).filter(|_| row != 0) else { continue };
        if let Some((_, v)) = m.fields.iter_mut().find(|(k, _)| k == "param3") {
            *v = Value::U32(cid as u32);
        }
    }
}

/// Whether an engine location is an Xyz material's (ocgcore-wasm adds `overlay_sequence`; a raw location has bit 0x80).
fn is_material(loc: &Json) -> bool {
    loc["overlay_sequence"].is_u64() || loc["location"].as_u64().unwrap_or(0) & LOC_OVERLAY != 0
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
            // Finishing with nothing picked cancels: Panther Warrior's attack (a Tribute to declare it) came back
            // again and again. Pick first.
            let finish = m["can_finish"] == true && m["unselect_cards"].as_array().is_some_and(|c| !c.is_empty());
            let index = if finish { Json::Null } else { json!(0) };
            json!({ "type": RESP_UNSELECT_CARD, "index": index })
        }
        MSG_SELECT_EFFECTYN => json!({ "type": RESP_EFFECTYN, "yes": true }),
        MSG_SELECT_YESNO => json!({ "type": RESP_YESNO, "yes": true }),
        MSG_SELECT_OPTION => json!({ "type": RESP_OPTION, "index": 0 }),
        MSG_SORT_CHAIN | MSG_SORT_CARD => json!({ "type": RESP_SORT, "order": null }),
        MSG_SELECT_COUNTER => {
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
            json!({ "type": RESP_COUNTER, "counters": counters })
        }
        // The first free own zone (field_mask bits set = unavailable; own monster zones bits 0-4, Spell/Trap 8-12).
        MSG_SELECT_PLACE | MSG_SELECT_DISFIELD => {
            // The first zone the mask leaves free: the player's monster zones (bits 0-4), Spell/Trap zones (8-12, the
            // Field zone 13), then the opponent's (16-20, 24-29): Give and Take and Black Garden place on the
            // opponent's field.
            let mask = m["field_mask"].as_u64().unwrap_or(0);
            let me = m["player"].as_u64().unwrap_or(0);
            let free = [(me, LOC_MZONE, 0..5, 0), (me, LOC_SZONE, 0..6, 8), (1 - me, LOC_MZONE, 0..5, 16), (1 - me, LOC_SZONE, 0..6, 24)]
                .into_iter()
                .flat_map(|(p, l, seqs, base)| seqs.map(move |s| (p, l, s, base + s)));
            // As many as asked (Ground Collapse and Phantom Dragon each disable two zones).
            let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
            let mut places: Vec<Json> = free
                .into_iter()
                .filter(|&(_, _, _, bit)| mask & 1 << bit == 0)
                .take(count)
                .map(|(p, l, s, _)| json!({ "player": p, "location": l, "sequence": s }))
                .collect();
            if places.is_empty() {
                places.push(json!({ "player": me, "location": LOC_MZONE, "sequence": 0 }));
            }
            let kind = if kind == MSG_SELECT_PLACE { RESP_PLACE } else { RESP_DISFIELD };
            json!({ "type": kind, "places": places })
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
        MSG_ANNOUNCE_CARD => json!({ "type": RESP_CARD_NAME, "cid": m["fits"][0].as_u64().unwrap_or(0) }),
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

    /// An idle prompt of the client's: the hand card at sequence 0 offers `effects` activations.
    fn idle_with_activations(effects: u64) -> Json {
        let activates: Vec<Json> = (0..effects)
            .map(|e| json!({ "controller": 0, "location": LOC_HAND, "sequence": 0, "cid": 4007, "description": e }))
            .collect();
        json!({ "type": MSG_SELECT_IDLECMD, "player": 0, "summons": [], "special_summons": [], "pos_changes": [],
                "monster_sets": [], "spell_sets": [], "activates": activates, "to_bp": false, "to_ep": true, "shuffle": false })
    }

    #[test]
    fn a_card_offering_two_effects_asks_which_and_both_are_reachable() {
        let (mut t, _) = Translator::new(CLIENT, [&[], &[]], HashMap::new());
        assert!(matches!(t.prompt(&idle_with_activations(2)), Prompt::Client(_)));
        // The round trip (the duel check's): before the card list, only the first effect was reachable.
        assert_eq!(t.unreachable_answers(), Vec::<String>::new());
        let activate = CardCommand { command: 3, player: CLIENT, zone: HAND, index: 0 };
        let Some(Prompt::Client(list)) = t.command(activate) else { panic!("a card list") };
        let begin = list.iter().find(|m| m.get("id").and_then(yo3_proto::Value::as_u32) == Some(LIST_BEGIN)).expect("a list");
        assert_eq!(begin.get("param1").and_then(yo3_proto::Value::as_u32), Some(2));
        t.pick(1);
        assert_eq!(t.dialog_result(1), Some(Prompt::Answer(json!({ "type": RESP_IDLECMD, "action": 5, "index": 1 }))));
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
