//! The rules engine's vocabulary (ocgcore and EDOPro's card scripts), each id defined once and named as the engine
//! names it: message types, response types and actions, hints, positions, locations and reasons. `translate.rs` turns
//! these into the client's ([`crate::wire`]); the duel check reads them too.

/// ocgcore message types (OcgMessageType), without their `MSG_` prefix.
pub mod msg {
    named! { u64;
        HINT = 2,
        WIN = 5,
        SELECT_BATTLECMD = 10,
        SELECT_IDLECMD = 11,
        SELECT_EFFECTYN = 12,
        SELECT_YESNO = 13,
        SELECT_OPTION = 14,
        SELECT_CARD = 15,
        SELECT_CHAIN = 16,
        SELECT_PLACE = 18,
        SELECT_POSITION = 19,
        SELECT_TRIBUTE = 20,
        SORT_CHAIN = 21,
        SELECT_COUNTER = 22,
        SELECT_SUM = 23,
        SELECT_DISFIELD = 24,
        FIELD_DISABLED = 56,
        SORT_CARD = 25,
        SELECT_UNSELECT_CARD = 26,
        TOSS_COIN = 130,
        TOSS_DICE = 131,
        ROCK_PAPER_SCISSORS = 132,
        ANNOUNCE_RACE = 140,
        ANNOUNCE_ATTRIB = 141,
        ANNOUNCE_CARD = 142,
        ANNOUNCE_NUMBER = 143,
        MOVE = 50,
        /// The engine's MSG_NEW_TURN {player}.
        NEW_TURN = 40,
        NEW_PHASE = 41,
        DRAW = 90,
        DAMAGE = 91,
        RECOVER = 92,
        LPUPDATE = 94,
        CONFIRM_DECKTOP = 30,
        REVERSE_DECK = 37,
        DECK_TOP = 38,
        CONFIRM_CARDS = 31,
        SHUFFLE_HAND = 33,
        EQUIP = 93,
        CARD_TARGET = 96,
        CANCEL_TARGET = 97,
        SWAP_GRAVE_DECK = 35,
        SHUFFLE_SET_CARD = 36,
        SWAP = 55,
        POS_CHANGE = 53,
        SUMMONING = 60,
        SPSUMMONING = 62,
        FLIPSUMMONING = 64,
        ATTACK = 110,
        BATTLE = 111,
        ATTACK_DISABLED = 112,
        DAMAGE_STEP_END = 114,
        CHAINING = 70,
        CHAIN_SOLVING = 72,
        CHAIN_END = 74,
        PAY_LPCOST = 100,
        ADD_COUNTER = 101,
        REMOVE_COUNTER = 102,
        /// ocgcore message types the harness looks at.
        RETRY = 1,
    }
}

/// ocgcore response types (OcgResponseType).
pub mod resp {
    named! { u64;
        BATTLECMD = 0,
        IDLECMD = 1,
        EFFECTYN = 2,
        YESNO = 3,
        OPTION = 4,
        CARD = 5,
        UNSELECT_CARD = 7,
        CHAIN = 8,
        DISFIELD = 9,
        PLACE = 10,
        POSITION = 11,
        TRIBUTE = 12,
        COUNTER = 13,
        SUM = 14,
        SORT = 15,
        RACE = 16,
        ATTRIB = 17,
        CARD_NAME = 18,
        NUMBER = 19,
        RPS = 20,
    }
}

/// MSG_SELECT_IDLECMD's response actions.
pub mod idle {
    named! { u64;
        SUMMON = 0,
        MONSTER_SET = 3,
        TO_BP = 6,
        TO_EP = 7,
    }
}

/// MSG_SELECT_BATTLECMD's response actions.
pub mod battle {
    named! { u64;
        CHAIN = 0,
        ATTACK = 1,
        TO_M2 = 2,
        TO_EP = 3,
    }
}

/// MSG_HINT's hint types.
pub mod hint {
    named! { u64;
        /// MSG_HINT's hint_type for "what the next selection is for" (HINTMSG_* in the card scripts' constant.lua).
        SELECTMSG = 3,
        /// MSG_HINT's hint_type for the window the next chance comes in (its value: [`CHAIN_WINDOWS`]).
        EVENT = 1,
    }
}

/// HINT_SELECTMSG values (HINTMSG_* in the card scripts' constant.lua): what the next selection is for.
pub mod hintmsg {
    named! { u64;
        RELEASE = 500,
        ATTACKTARGET = 549,
        /// HINTMSG_TOZONE: Card.SelectAdjacent's hint (utility.lua) before its zone choice.
        TOZONE = 571,
        /// HINTMSG_LVRANK: the hint before a Level is declared (Reasoning).
        LVRANK = 567,
    }
}

/// ocgcore positions.
pub mod pos {
    named! { u64;
        FACEUP_ATTACK = 1,
        FACEDOWN_ATTACK = 2,
        FACEUP_DEFENSE = 4,
        FACEDOWN_DEFENSE = 8,
    }
}

/// ocgcore locations.
pub mod loc {
    named! { u64;
        DECK = 1,
        HAND = 2,
        MZONE = 4,
        SZONE = 8,
        GRAVE = 16,
        REMOVED = 32,
        EXTRA = 64,
        /// An Xyz material: or'ed with its monster's location, whose sequence it takes.
        OVERLAY = 0x80,
    }
}

/// ocgcore reasons (MSG_MOVE's `reason`).
pub mod reason {
    named! { u64;
        DESTROY = 0x1,
        RELEASE = 0x2,
        SUMMON = 0x10,
        BATTLE = 0x20,
        SPSUMMON = 0x800,
        MATERIAL = 0x8,
        FUSION = 0x40000,
        SYNCHRO = 0x80000,
        RITUAL = 0x100000,
        XYZ = 0x200000,
    }
}
