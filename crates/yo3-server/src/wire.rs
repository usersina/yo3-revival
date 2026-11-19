//! The client's duel vocabulary, each id defined once with its client evidence: engine commands (`cmd`, the ids
//! `CLIENT_COMMANDS` lists), duelRequestNames (`request`), its zone numbering, the command map's menu bits, other flags
//! and input modes, dialog kinds, message entries, card list kinds and marks, duel prompt texts and duel results. Names
//! follow Konami's own where they are known (docs/protocol.md, "Command names").

/// Engine commands (duelRequestName 0x11 {id, param1..param4}), by `CLIENT_COMMANDS`' names.
pub mod cmd {
    named! { u32;
        /// A field card's new face and position in place {place as moves give it} (CardFlipTurn, DuelCardManager
        /// 0x4FF000).
        FLIP_TURN = 0x1C,
        /// CardCheat {place word with face bits, Konami id, uid}: the card object (made when the uid has none) reloads
        /// its face from its table row and its stats from the place's record (DuelCardManager 0x4FF000; Konami's logs,
        /// 46 a duel).
        CARD_CHEAT = 0x1D,
        /// CardLockon {player, zone | index << 8, 2}: the card at that place pops up, face or not (0x4FF000 case 0x19).
        CARD_LOCKON = 0x19,
        /// DeckFlipTop {player, pile 0xD or 0xC}: the pile's top card (the table's index 0 of the Deck, the last of the
        /// Extra Deck) turns over, face up from its row, or back (0x4FF000 case 0x17; Konami's logs: 611 Deck, 48 Extra
        /// Deck).
        DECK_FLIP_TOP = 0x17,
        /// A card destroyed {player, zone | index << 8}, zone at most the hand (CardVanish): its label goes, the
        /// shatter (effect 0x33) plays at it and it is hidden (state 0x4FC520), before its move.
        CARD_VANISH = 0x1F,
        /// HandShow {player, index}: a hand card shows its face (DuelCardManager 0x4FF15C).
        HAND_SHOW = 0x13,
        /// HandShuffle {player}: the client rebuilds the player's hand from its card table, index by index (the card at
        /// (player, hand, i) by 0x523DF0, its object by uid 0x500470; DuelCardManager 0x4FF000 case 0x12). Every index
        /// below the hand's count needs an entry, or the client stops with an error state (0x1200).
        HAND_SHUFFLE = 0x12,
        /// DeckShuffle {player} (DuelScene 0x514290 case 0x15: the shuffle's effect at that player's Deck; Konami's logs, 943
        /// uses, all with param2-4 0): after a search or a card returned to the Deck.
        DECK_SHUFFLE = 0x15,
        /// Card relations, which the client draws as markers on the related slots while the pointer is on a face-up
        /// field card (0x50B700 → 0x50B1C0): 1017 {source place, target place, marker type 1-8, 1 add (0x5224E0) / 0
        /// remove the entry with all three (0x522530)} (pump 0x3F9), places as `player | zone << 8`; 0x26 redraws the
        /// hovered card's (DuelFieldManager 0x50A690). See "Card relations" in docs/protocol.md.
        RELATION = 0x3F9,
        RELATIONS_REDRAW = 0x26,
        /// The zones each side can use (pump 1006 → engine +0x124: own side low, bits 0-4 monster zones, 5-9
        /// Spell/Trap).
        ZONES_USABLE = 1006,
        /// CardSwap {_, place A, place B} as moves give them, both field places (zone < 11): the cards in the two slots
        /// trade places, each taking the word of the place it lands on as its place and position (DuelCardManager
        /// 0x4FF000 case 0x1B: the objects by 0x4F8240, 0x5000E0 with param_6 1, the slot array swapped). **Shape**:
        /// read in the code, not seen live.
        CARD_SWAP = 0x1B,
        /// A field card's counters {player | zone << 8, change, kind} (DuelCardManager 0x4FFD8E): the change is added
        /// to the card's count of that kind (0 erases it; a negative count is ignored), shown on its field label with
        /// the kind's icon (table 0x789020) and named in the card info as UI text 200 + kind (0x4551DB).
        COUNTER = 0x27,
        /// A battle (DuelBattle 0x4F8A90): start; the attack {attacker's player | zone << 8, target's player, target's
        /// zone or DIRECT_TARGET}; damage figures per player, then end. Native BattleRun also accepts destroy flags,
        /// but those hide the card: only a confirmed destruction move can do that, after any battle protection has
        /// applied.
        BATTLE_INIT = 0xA,
        BATTLE_ATTACK = 0xC,
        BATTLE_RUN = 0xD,
        BATTLE_END = 0xE,
        /// BattleSelect (no parameters, 0x4F8B95): DuelBattle's aiming state. While a field card whose menu has Confirm
        /// is hovered, it draws the aiming arc (effect 0xCB) to it from the attacker a BattleAttack with no target
        /// named (0x4F8CD9, DuelCardCommand +0x44 & 0x800): how the original asked for an attack target (#51).
        BATTLE_SELECT = 0xB,
        /// The summon effects at a card {player, uid} (DuelScene 0x514290): Normal (or the player's ace), Special.
        RUN_SUMMON = 0x38,
        /// Commands Konami's server sent that only show something (227 original duel logs), by their handlers:
        /// FieldChange 7 {Field Spell's Konami id, or 0} changes the duel field's background (DuelScene → `0x50CD60`);
        /// CardSet `0x1E` {player, zone, 1 for a monster} plays the Set effect (`0x22`, DuelScene); CardActivate `0x22`
        /// {place as moves give it, Konami id, uid} plays a card's activation effect by its type (DuelScene, waited
        /// for); the avatar speech cut-ins (DuelCutin `0x505A70`, each player's Avatar Settings lines; the client's
        /// duel option "speech" turns them off): `0x43` {player, 1, 0} at the turn's draw, `0x44` {player, 0x8000 |
        /// uid, kind} a summon (the "ace monster" line for the configured card, 8 a Synchro Summon's line), `0x47`
        /// {player, 0x8000 | uid, step} an activation (steps 1 or 2 note how the card came, 3 and up speak; the "trump
        /// card" line for the configured card), `0x48` {player, Konami id, face up, to a Spell/Trap zone} a card
        /// leaving the hand for the field (a Set speaks), `0x4C` {player} the turn's end, `0x4D` {player, LP change,
        /// kind} damage (by the LP left; not for a cost or gain).
        FIELD_CHANGE = 7,
        CARD_SET = 0x1E,
        CARD_ACTIVATE = 0x22,
        SPEAK_DRAW = 0x43,
        SPEAK_SUMMON = 0x44,
        SPEAK_ACTIVATE = 0x47,
        /// A card revealed to activate (a Set Spell/Trap, a hand card's effect): {player, 1, Konami id}, the speech
        /// slot 19 (DuelCutin 0x505A70); the next 0x47 stays silent.
        SPEAK_REVEAL = 0x46,
        SPEAK_PLAY = 0x48,
        SPEAK_TURN_END = 0x4C,
        SPEAK_DAMAGE = 0x4D,
        RUN_SP_SUMMON = 0x39,
        /// DuelSummon (0x5165A0): Set marks a card {player, zone (11 the hand), uid, variant} with its family's effect,
        /// Run plays the family's effect on it, Reset clears the marks. Tribute (effects 0x25 / 0x24), Material (0x27 /
        /// 0x26), Tuning (0x2B / 0x2A; Run's param4 is the level, with 1 in the next byte for the first Tuner, 2 for
        /// another).
        TRIBUTE_SET = 0x2A,
        TRIBUTE_RUN = 0x2C,
        MATERIAL_SET = 0x2D,
        MATERIAL_RUN = 0x2F,
        TUNING_SET = 0x30,
        TUNING_RUN = 0x32,
        /// RunFusion {player, the summoned monster's uid, kind << 16 | the number of materials} (DuelScene 0x514290 by
        /// the kind: 0 or 1 the Fusion animation, 2 the Synchro's).
        RUN_FUSION = 0x3A,
        /// DuelScene's RunCoin (0x51500E): DuelCoinToss (0x508050, duel_cointoss.swf) {player | coins << 6 | winning
        /// side << 14, 0, one bit a coin (1 heads)}. The SWF shows one to three coins, each "head" or "tail", "ok" on
        /// the winning side.
        RUN_COIN = 0x3C,
        /// DuelScene's RunDice (0x5150CD): DuelDiceRoll (0x507D10, duel_diceroll.swf) {player, 0, face}: one die.
        RUN_DICE = 0x3D,
        /// The summon's materials for the Fusion and Synchro animations (the pump 0x522A50 into the engine state's list
        /// at +0x1330, which DuelSynchro 0x507940 walks from the Tuner count on to show the others): 1015 {count, uid,
        /// uid, uid} starts it, 1016 {uid, uid, uid, uid} adds more (Konami's logs: right before every RunFusion, the
        /// Tuner first).
        MATERIAL_LIST = 1015,
        MATERIAL_LIST_MORE = 1016,
        /// The chain's links (DuelChain 0x501560): a new link {player, zone it shows over, link number}, link 1
        /// starting the list; a link resolving {_, _, link number}, which must come last link first (the list's end
        /// leaves).
        CHAIN_SET = 0x33,
        CHAIN_RUN = 0x34,
        /// The card info panel's level for a monster in the hand (0x453740, hand case): a map uid -> level at the
        /// engine state +0x90 +0xD8 (0x5227E0), which 1018 {uid, level} sets (0x5226B0) and {uid, 0xFFFF} clears
        /// (0x522750). Without an entry the panel gets -1 and draws no stars. Konami's logs: right before a monster's
        /// move into a hand, cleared right before it leaves.
        HAND_LEVEL = 1018,
        /// The card info panel's attribute, Type and level for a monster in a Graveyard (0x453740, case 0xE: 0x5241D0,
        /// 0x524170, 0x524230): a map Konami id -> {attribute, Type, level} at +0x90 +0xF8. 1019 {id, id, attribute,
        /// Type} stages it and 1020 {id, level} stores it (0x522850; with a 1019 of zeros it deletes, 0x522910).
        /// Without an entry all three are -1: no icons or stars, and Type -1 names the duel prompt text before the
        /// Types ("Send ... from your Deck."). Konami's logs: both right before a monster's table entry in a Graveyard.
        PILE_STATS = 1019,
        PILE_STATS_COMMIT = 1020,
        /// Data commands that build a field card's record (pop 0x522A50).
        RECORD_BEGIN = 0x3E9,
        RECORD_MORE = 0x3EA,
        RECORD_COMMIT = 0x3EB,
        /// The card list dialog: DuelScene command 0x37 {player, kind, message} (DuelCardListDialog 0x4C72D0), filled
        /// by data commands the pop keeps (0x522A50): 0x3EF {count, max picks}, 0x3F2 CARD_Prop rows and 0x3F3 marks,
        /// four at a time, and a message of 0x3EC {entries} and 0x3ED {entry, ...} (built by 0x519870).
        CARD_LIST = 0x37,
        LIST_BEGIN = 0x3EF,
        LIST_VALUES = 0x3F0,
        LIST_ROWS = 0x3F2,
        LIST_MARKS = 0x3F3,
        MESSAGE_LENGTH = 0x3EC,
        MESSAGE = 0x3ED,
        /// Dialog command {kind, message, _} (DuelScene 0x514C0C, factory 0x4C8CD0) and its Yes/No kind (buttons 0
        /// "No", 1 "Yes"); message 1 = from the message entries.
        DIALOG = 0x36,
        /// Duel end {result for this client: 1 won, 2 lost, 3 draw; reason} (DuelScene 0x514290).
        DUEL_END = 2,
        /// JankenScene commands (0x518400): begin, resolve, who chooses, chosen (its step leaves the scene by itself,
        /// 0x519230). 1505 there is the match's end {match result, ...} (0x5185A3), not part of the janken.
        JANKEN_BEGIN = 1500,
        JANKEN_RESOLVE = 1501,
        FIRST_BEGIN = 1502,
        FIRST_RESOLVE = 1503,
        /// Duel commands: start (DuelScene 0x514290), set LP {player, LP} (DuelCockpit 0x502EB0), phase {player, phase}
        /// (DuelScene, DuelCockpit).
        DUEL_START = 1,
        SET_LP = 0xF,
        /// Change a player's LP {player, signed change} (DuelCockpit 0x502EB0).
        CHANGE_LP = 0x10,
        PHASE = 5,
        /// Turn change {the player whose turn starts} (DuelScene; the pop counts the turn).
        TURN = 6,
        /// Wait for a player's input {mode, player}: DuelFieldManager highlights the player's cards in the command map,
        /// and modes 2 and 3 turn the phase panel on (0x504890).
        INPUT = 4,
        /// Card commands: set a card in the engine's card table {uid, row, 0, table place} (0x521DF0), and move a card
        /// on screen {uid | row << 16, from, to} (DuelCardManager 0x4FF6A2).
        CARD = 1000,
        MOVE = 0x1A,
        /// DeckReset {player, pile zone 0xC-0xF} (DuelCardManager 0x4FF3A5): the pile takes its count from the engine's
        /// per-zone count (0x5240F0, the setup's MainDeckNum and ExtraDeckNum at first) and shows it, with the count on
        /// its label. After that each pile counts itself (a card landing +1, 0x4FBF30; leaving -1, 0x5000E0), so
        /// without it the deck pile starts at 0 and never shows.
        DECK_RESET = 0x16,
        /// Card Swap between a match's duels (DuelManager 0x50FC00 takes it to 0x50FDF0), instead of the next janken's
        /// 1500.
        SWAP_BETWEEN_DUELS = 1504,
        /// A match's end for a spectator {match result: 1 won, 2 lost, 3 draw}: its DuelResult (0x512070) reads it at
        /// the head of its queue (MainGamen +0x820) where a player's reads the front's 0x2009.
        MATCH_END = 1505,
    }
}

/// duelRequestNames: the server's (pump cases) and the client's.
pub mod request {
    named! { u32;
        /// duelRequestName of an engine command {id, param1..param4} (pump case 0x11).
        ENGINE_COMMAND = 0x11,
        /// duelRequestName of the duel setup (pump case 0x15).
        DUEL_SETUP = 0x15,
        /// duelRequestName of the command map (pump case 0x12).
        COMMAND_DATA = 0x12,
        /// Client duelRequestName of a card command (0x520FD0) and of a phase button {phase} (0x521180).
        CARD_COMMAND = 2,
        PHASE_BUTTON = 3,
        /// Client duelRequestName (bare) of the field's right-click while the command map's other flag 6 is set
        /// (0x5212B0, from DuelFieldManager 0x50B700): taking back an attack whose target is picked on the field.
        RIGHT_CLICK = 5,
        /// Client duelRequestNames of a closing dialog: its result {DLGResult, debug} (0x521A70), and for the card list
        /// (0x4C74F0) first the first pick {SetIndex} (0x521BC0) and each pick {index, data: its order} (0x521CC0).
        DIALOG_RESULT = 4,
        PICK = 7,
        /// Client duelRequestNames: the janken hand {selectHand} (0x4F1330), first or second {choice} (0x4F1430).
        SELECT_HAND = 0xC,
        SELECT_FIRST = 0xD,
        /// Client duelRequestName (bare) of Card Swap's end, once the front took the swapped deck (DuelManager
        /// 0x50FDF0).
        SWAPPED = 0xE,
        /// Client duelRequestName of the duel options {player, autoSet} (0x515790): on entering the field and when the
        /// player changes them; `autoSet` is DUEL OPTIONS' auto card placing.
        DUEL_OPTIONS = 0xF,
        /// duelRequestName of a dialog text of the server's own {slot, text} (Revival, read by the connection DLL,
        /// `tools/mod/dialog-text.h`). The stock pump (0x5233A0) drops it: only cases 0x10-0x15 do anything (0x52341C).
        REVIVAL_TEXT = 0x7A12,
    }
}

/// The client's zones: field slots 0-4 monsters, 5-9 Spell/Trap, 10 the Field Spell, then these lists.
pub mod zone {
    named! { u32;
        /// The client's list zones besides hand and deck.
        EXTRA = 0xC,
        GRAVE = 0xE,
        REMOVED = 0xF,
        /// The Field Spell slot.
        FIELD_SPELL = 10,
        /// Zones (the client's numbering): 0-4 monsters, 5-9 Spell/Trap, then these.
        HAND = 0xB,
        DECK = 0xD,
    }
}

/// The card menu's entries (`duel_cardcmd.swf`, 0x4F9320): a command map's bits and a card command's
    /// CommandID.
pub mod menu {
    named! { u32;
        ATTACK = 0,
        DETAILS = 1,
        SPECIAL_SUMMON = 2,
        ACTIVATE = 3,
        SUMMON = 4,
        FLIP_SUMMON = 5,
        SET_MONSTER = 6,
        SET_SPELL = 7,
        TO_ATTACK = 8,
        TO_DEFENSE = 9,
        SURRENDER = 10,
        /// The card menu's Confirm (bit 11): on a pile, its view.
        CONFIRM = 11,
        DRAW = 12,
        SHUFFLE = 13,
    }
}

/// The command map's other flags {key, value} (DuelFieldManager reads them through 0x501C00).
pub mod flag {
    named! { u8;
        /// The command map's other flag that lets the field's right-click send duelRequestName 5 (0x50B700 → 0x501C00).
        RIGHT_CLICK = 6,
        /// Phase panel buttons (0 DP .. 5 EP) and the input mode that turns the panel on (command 4's param1).
        BUTTON_BP = 3,
        BUTTON_M2 = 4,
        BUTTON_EP = 5,
    }
}

/// WaitInput's input modes (command 4's param1).
pub mod mode {
    named! { u32;
        MAIN = 2,
        BATTLE = 3,
    }
}

/// Dialog kinds and their parameters (DuelScene 0x514C0C, factory 0x4C8CD0).
pub mod dialog {
    named! { u32;
        /// The OK dialog (factory 0x4C8CD0 kind 0).
        OK = 0,
        YES_NO = 2,
        FROM_ENTRIES = 1,
        /// The chain question {_, message, level} (DuelEffectDialog 0x4C5120): buttons as Yes/No's, with the list's
        /// rows (dlg_yesno_card.swf, at most five) when 0x3EF's param3 is 0xFF (0x4C51A0). Whether it shows depends on
        /// the level and the player's Activation Message option (invoke_conf, 0x4C5560): Manual shows every level, Semi
        /// Auto Cancel (the default) levels 2 and up, Auto Cancel none (unless a mouse button is held; level 4+ always
        /// shows). Unshown, it answers No. The levels sent: something happened, a phase or step, and no card fitting
        /// the timing.
        EFFECT = 3,
        WITH_CARDS = 0xFF,
        LEVEL_EVENT = 3,
        LEVEL_PHASE = 1,
        LEVEL_UNFIT = 0,
        /// The option list (DuelSelectDialog 0x4C5750): the 0x3F0 values up to 0x3EF's count, each a duel prompt text
        /// listed with bit 15, at most four (0x4C5880); DLGResult the value's index (0x4C5B20).
        SELECT = 4,
        LISTED = 0x8000,
        /// The declaration dialogs (factory 0x4C8CD0): Type {_, message, allowed types} (DuelSelectTypeDialog,
        /// dlg_seltype.swf: bit k for type k, one pick, DLGResult k), Attribute {_, message, allowed << 16 | picks}
        /// (DuelSelectAttrDialog, dlg_selattr.swf: DLGResult the picks as bits), Number {_, message, max << 24 | step
        /// << 20 | min << 16 | format} (DuelSelectNumberDialog 0x4C6430, dlg_selnum.swf: in thousands for format 302
        /// "%d000LP", hundreds for 303 "%d00LP", else ones; DLGResult the number in those units).
        TYPE = 5,
        ATTRIBUTE = 7,
        NUMBER = 11,
        /// The position dialog {_, message, uid | face-up defense << 16} (DuelSelectStandDialog 0x4C5E50,
        /// dlg_selstand.swf): button 0 attack, 1 defense, face-down unless the flag is set.
        POSITION = 8,
        FACE_UP_DEFENSE = 1 << 16,
    }
}

/// Message entry kinds (0x519870): kind << 28 | value.
pub mod entry {
    named! { u32;
        /// Message entries (0x519870): a text of the duel prompt table, and a card name by Konami id (0x5374B0) put
        /// into the text's %s.
        TEXT = 1 << 28,
        CARD_NAME = 5 << 28,
        /// Message entry kinds 2 (a line break) and 8 (a number put into the text's `%d`, `0x519D60`), and the duel
        /// prompt text "Select %d card(s)."
        LINE_BREAK = 2 << 28,
        NUMBER = 8 << 28,
    }
}

/// Card list kinds (DuelCardListDialog 0x4C72D0) and marks.
pub mod list {
    named! { u32;
        /// A list mark: the opponent's card (dlg_cardlist.swf). The low bits add icons, meaning unknown.
        OPPONENTS = 0x8000,
        /// Card list kinds by the picks OK needs (0x4C72D0): none, all `max`, one.
        PICK_ANY = 1,
        PICK_ALL = 6,
        PICK_ONE = 0x27,
        /// The kind that draws a card as a back where its flag is clear (getCardInfo 0x4C78A0, "0000" for
        /// data/mini/0000.png), by the card table's flag at the banished place of the same index on the side `(client
        /// != the list's player)`: so a list of the CPU's side (player 1) reads the opponent's banished entries. No
        /// picks needed, like kind 1 (the other kinds draw a card's image from its row's id, so a hidden card, row 0,
        /// drew nothing and couldn't be clicked).
        PICK_MIXED = 4,
        /// The card list's extended kind that lists the client's own card database with a name search
        /// (DuelCardListDialogEx 0x5131E0, dlg_cardlistex.swf; CardDuelFilter 0x4976A0 mode 0: any card); DLGResult the
        /// card's Konami id.
        SEARCH = 0x4C,
    }
}

/// Duel prompt texts (`DLG_Text_E.bin`), by id.
pub mod text {
    named! { u32;
        SELECT_COUNT = 592,
        /// "@3Tribute@0 the necessary @3monsters@0 to match the required number of Level." (the duel prompt table's
        /// 207).
        SUM = 207,
        /// Duel prompt texts for a zone an effect takes: 155 "Select Monster Card Zones to render unusable.", 324
        /// "Select a Spell & Trap Card Zone to make unusable.", 472 "Move this card to an adjacent unoccupied Monster
        /// Card Zone.".
        MONSTER_ZONE = 155,
        SPELL_ZONE = 324,
        ADJACENT_ZONE = 472,
        /// Duel prompt text 245: "Select the cards you wish to return to the Deck in order of your choice."
        SORT = 245,
        /// Duel prompt texts: "Select the @3monster(s)@0 as @3Tribute@0.", 'Activate the effect of "@3%s@0"?', 'Use the
        /// effect of "@3%s@0"?', "Use the effect now?".
        TRIBUTE = 18,
        ACTIVATE = 250,
        USE = 226,
        USE_NOW = 270,
        DIRECT = 225,
        /// "Select the @3monster(s)@0 to @3Special Summon@0 from your @2Extra Deck@0." and "Select the @3monster(s)@0
        /// to @3Special Summon@0.".
        SPSUMMON_EXTRA = 26,
        SPSUMMON = 25,
        /// "Do you @3surrender@0?" and "You may @3surrender@0 starting from the @25th turn@0." (DLG_Text 6 and 82).
        SURRENDER = 6,
        SURRENDER_EARLY = 82,
        /// 'Select position for "@3%s@0".'
        POSITION = 327,
        /// 'The effect of "@3%s@0" is @3activated@0. @3Chain@0 another card or effect?', '"@3%s@0" is @3activated@0.
        /// ...' and, outside a chain, "@3Activate@0 the effect of a @3card@0?"; then "Select effect to @3activate@0.".
        CHAIN_EFFECT = 7,
        CHAIN_CARD = 8,
        ACTIVATE_CARD = 269,
        SELECT_EFFECT = 276,
        /// "Select @3Type(s)@0.", "Select @3Attribute(s)@0.", "Declare @3monster@0 level."
        TYPE = 241,
        ATTRIBUTE = 242,
        LEVEL = 280,
    }
}

/// The duel-end reason "has surrendered" (text 1999 + 12, 0x508650): Konami's logs end every surrender with command 2
/// {result, 12}, after the surrendering player's LP set to 0 (command 0xF {player, 0}).
pub const SURRENDERED: u32 = 12;
/// The turn Surrender opens from (DLG_Text 82: "starting from the 5th turn"; a turn is either player's).
pub const SURRENDER_TURN: u32 = 5;

/// Duel results for a client (DuelEnd's param1).
pub mod result {
    named! { u32;
        WON = 1,
        LOST = 2,
        DRAW = 3,
    }
}

/// A place as the card table keys it (0x523DF0): player | zone << 8 | index << 16.
pub fn table_place(player: u32, zone: u32, index: usize) -> u32 {
    player | zone << 8 | (index as u32) << 16
}

/// A place as card moves give it (0x522A50 counts by bit 0 and bits 1-5). The index bits are a guess (Online 2's
/// packing).
pub fn move_place(player: u32, zone: u32, index: usize) -> u32 {
    player | zone << 1 | (index as u32) << 6
}

/// A place with its card's face and position, as card moves give it: [`move_place`], face up bit 14, defense bit 15
/// (0x5000E0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Place {
    pub player: u32,
    pub zone: u32,
    pub index: usize,
    pub face_up: bool,
    pub defense: bool,
}

impl Place {
    pub fn packed(&self) -> u32 {
        move_place(self.player, self.zone, self.index) | (self.face_up as u32) << 14 | (self.defense as u32) << 15
    }

    pub fn of_move(word: u32) -> Place {
        Place {
            player: word & 1,
            zone: word >> 1 & 0x1F,
            index: (word >> 6 & 0xFF) as usize,
            face_up: word >> 14 & 1 == 1,
            defense: word >> 15 & 1 == 1,
        }
    }

    /// The player as an index into per-player arrays.
    pub fn side(&self) -> usize {
        self.player as usize
    }
}

/// A card table entry's flag (command 1000's uid top byte, kept as `!= 0` by 0x521DF0): a pile whose count changes
/// draws its top card's face when its entry has it, else a back (0x4FE4D0 → 0x523DA0), and the pointer over the pile
/// previews that card (0x50B9C0). Set for the open piles: the Graveyard and the banished cards.
pub const OPEN: u32 = 1 << 24;
/// BattleAttack's target zone for "none yet" (Konami's 0xC {attacker, 0, -1} before every 0xB).
pub const NO_TARGET: u32 = u32::MAX;
/// A direct attack's target zone: any zone from 5 plays as one (the direct cut-in and strike, 0x4F8430 state 3);
/// 0xFFFFFFFF plays nothing. 16 is the field's centre, as ChainSet's.
pub const DIRECT_TARGET: u32 = 16;

/// What a BattleAttack aims at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A monster in that player's zone (0-4).
    Monster { player: u32, zone: u32 },
    /// That player, directly.
    Direct { player: u32 },
    /// Nothing yet: the attacker is marked while the player aims (with BattleSelect, #51).
    NotYet { player: u32 },
}

/// The commands both the server and the duel check's client model read and write, each packed and unpacked here only
/// (`Cmd::params`, `Cmd::decode`): the battle's, a card's move, a card table entry and a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// A battle's start, BattleSelect's aiming state and the battle's end (no parameters).
    BattleInit,
    BattleSelect,
    BattleEnd,
    /// The attack {attacker's player | zone << 8, target's player, target's zone (or [`DIRECT_TARGET`], [`NO_TARGET`])}.
    BattleAttack { player: u32, zone: u32, target: Target },
    /// Damage figures per player, and flags (bit 1 << (8 × player): that player's battler destroyed, which hides it).
    BattleRun { damage: [u32; 2], flags: u32 },
    /// A card's move on screen {uid | kind << 16, from, to} (DuelCardManager 0x4FF6A2): kind as `translate.rs`'s
    /// MOVE_* (the opening draw sends the card's row there).
    Move { uid: u32, kind: u32, from: Place, to: Place },
    /// A card table entry {uid, with [`OPEN`] for an open pile, row, 0, table place} (0x521DF0).
    Card { uid: u32, open: bool, row: u32, place: u32 },
    /// A dialog {kind, message, parameter} (DuelScene 0x514C0C; kinds in [`dialog`]).
    Dialog { kind: u32, message: u32, param: u32 },
}

impl Cmd {
    /// The command's id and parameters.
    pub fn params(&self) -> (u32, [u32; 4]) {
        match *self {
            Cmd::BattleInit => (cmd::BATTLE_INIT, [0; 4]),
            Cmd::BattleSelect => (cmd::BATTLE_SELECT, [0; 4]),
            Cmd::BattleEnd => (cmd::BATTLE_END, [0; 4]),
            Cmd::BattleAttack { player, zone, target } => {
                let (to, at) = match target {
                    Target::Monster { player, zone } => (player, zone),
                    Target::Direct { player } => (player, DIRECT_TARGET),
                    Target::NotYet { player } => (player, NO_TARGET),
                };
                (cmd::BATTLE_ATTACK, [player | zone << 8, to, at, 0])
            }
            Cmd::BattleRun { damage, flags } => (cmd::BATTLE_RUN, [damage[0], damage[1], flags, 0]),
            Cmd::Move { uid, kind, from, to } => (cmd::MOVE, [uid | kind << 16, from.packed(), to.packed(), 0]),
            Cmd::Card { uid, open, row, place } => (cmd::CARD, [uid | if open { OPEN } else { 0 }, row, 0, place]),
            Cmd::Dialog { kind, message, param } => (cmd::DIALOG, [kind, message, param, 0]),
        }
    }

    /// The command for an id and its parameters, if it is one of these.
    pub fn decode(id: u32, p: [u32; 4]) -> Option<Cmd> {
        Some(match id {
            cmd::BATTLE_INIT => Cmd::BattleInit,
            cmd::BATTLE_SELECT => Cmd::BattleSelect,
            cmd::BATTLE_END => Cmd::BattleEnd,
            cmd::BATTLE_ATTACK => {
                let to = p[1] & 1;
                let target = match p[2] {
                    NO_TARGET => Target::NotYet { player: to },
                    zone @ 0..5 => Target::Monster { player: to, zone },
                    _ => Target::Direct { player: to },
                };
                Cmd::BattleAttack { player: p[0] & 1, zone: p[0] >> 8 & 0xFF, target }
            }
            cmd::BATTLE_RUN => Cmd::BattleRun { damage: [p[0], p[1]], flags: p[2] },
            cmd::MOVE => Cmd::Move { uid: p[0] & 0xFFFF, kind: p[0] >> 16, from: Place::of_move(p[1]), to: Place::of_move(p[2]) },
            cmd::CARD => Cmd::Card { uid: p[0] & 0xFF_FFFF, open: p[0] & OPEN != 0, row: p[1], place: p[3] },
            cmd::DIALOG => Cmd::Dialog { kind: p[0], message: p[1], param: p[2] },
            _ => return None,
        })
    }

    /// The engine command message (duelRequestName 0x11).
    pub fn message(&self) -> yo3_proto::Message {
        let (id, params) = self.params();
        crate::duel::command(id, params)
    }
}
