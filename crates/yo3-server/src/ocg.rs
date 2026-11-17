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
        /// A card is Set (banter's "I set a card"; the client is shown the move).
        SET = 54,
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

/// An answer to the engine's prompt, built here only, as ocgcore-wasm 0.1.6's JSON takes it (`yo3-engine`'s
/// `encode.rs`): `{type, ...}` with the response type's own fields. Indices are positions in the prompt's lists;
/// `None` is the JSON null that passes, cancels or leaves.
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    /// MSG_SELECT_BATTLECMD: an action ([`battle`]), with its list index (none for a phase change).
    BattleCmd { action: u64, index: Option<usize> },
    /// MSG_SELECT_IDLECMD: an action ([`idle`]), with its list index (none for a phase change).
    IdleCmd { action: u64, index: Option<usize> },
    EffectYn(bool),
    YesNo(bool),
    /// MSG_SELECT_OPTION: the option's index.
    Option(usize),
    /// MSG_SELECT_CARD: the picks, or none to cancel.
    Card(Option<Vec<usize>>),
    /// MSG_SELECT_TRIBUTE: the picks, or none to cancel.
    Tribute(Option<Vec<usize>>),
    /// MSG_SELECT_UNSELECT_CARD: a card, or none to finish or cancel.
    UnselectCard(Option<usize>),
    /// MSG_SELECT_CHAIN: a chain link to add, or none to pass.
    Chain(Option<usize>),
    /// MSG_SELECT_DISFIELD and MSG_SELECT_PLACE: zones {player, location, sequence}.
    Disfield(Vec<serde_json::Value>),
    Place(Vec<serde_json::Value>),
    /// MSG_SELECT_POSITION: the position ([`pos`]).
    Position(u64),
    /// MSG_SELECT_COUNTER: counters to remove per card.
    Counter(serde_json::Value),
    /// MSG_SELECT_SUM: the picks.
    Sum(Vec<usize>),
    /// MSG_SORT_CARD / MSG_SORT_CHAIN: an order, or none for the engine's.
    Sort(Option<serde_json::Value>),
    /// MSG_ANNOUNCE_RACE: Types (RACE_* bits).
    Race(Vec<u64>),
    /// MSG_ANNOUNCE_ATTRIB: Attributes (ATTRIBUTE_* bits).
    Attrib(Vec<u64>),
    /// MSG_ANNOUNCE_CARD: a card's code.
    CardName(u64),
    /// MSG_ANNOUNCE_NUMBER: the option's index.
    Number(u64),
    /// MSG_ROCK_PAPER_SCISSORS: 1 rock, 2 paper, 3 scissors.
    Rps(u64),
}

impl Response {
    /// A battle or idle command by the response type (for code that serves both lists).
    pub fn command(kind: u64, action: u64, index: Option<usize>) -> Response {
        if kind == resp::BATTLECMD { Response::BattleCmd { action, index } } else { Response::IdleCmd { action, index } }
    }

    /// A card or tribute choice by the response type.
    pub fn cards(kind: u64, picks: Option<Vec<usize>>) -> Response {
        if kind == resp::TRIBUTE { Response::Tribute(picks) } else { Response::Card(picks) }
    }

    /// A yes or no by the response type (MSG_SELECT_EFFECTYN or MSG_SELECT_YESNO).
    pub fn yes(kind: u64, yes: bool) -> Response {
        if kind == resp::EFFECTYN { Response::EffectYn(yes) } else { Response::YesNo(yes) }
    }

    /// The JSON the engine takes.
    pub fn json(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Response::BattleCmd { action, index } => json!({ "type": resp::BATTLECMD, "action": action, "index": index }),
            Response::IdleCmd { action, index } => json!({ "type": resp::IDLECMD, "action": action, "index": index }),
            Response::EffectYn(yes) => json!({ "type": resp::EFFECTYN, "yes": yes }),
            Response::YesNo(yes) => json!({ "type": resp::YESNO, "yes": yes }),
            Response::Option(index) => json!({ "type": resp::OPTION, "index": index }),
            Response::Card(picks) => json!({ "type": resp::CARD, "indicies": picks }),
            Response::Tribute(picks) => json!({ "type": resp::TRIBUTE, "indicies": picks }),
            Response::UnselectCard(index) => json!({ "type": resp::UNSELECT_CARD, "index": index }),
            Response::Chain(index) => json!({ "type": resp::CHAIN, "index": index }),
            Response::Disfield(places) => json!({ "type": resp::DISFIELD, "places": places }),
            Response::Place(places) => json!({ "type": resp::PLACE, "places": places }),
            Response::Position(position) => json!({ "type": resp::POSITION, "position": position }),
            Response::Counter(counters) => json!({ "type": resp::COUNTER, "counters": counters }),
            Response::Sum(picks) => json!({ "type": resp::SUM, "indicies": picks }),
            Response::Sort(order) => json!({ "type": resp::SORT, "order": order }),
            Response::Race(races) => json!({ "type": resp::RACE, "races": races }),
            Response::Attrib(attributes) => json!({ "type": resp::ATTRIB, "attributes": attributes }),
            Response::CardName(cid) => json!({ "type": resp::CARD_NAME, "cid": cid }),
            Response::Number(value) => json!({ "type": resp::NUMBER, "value": value }),
            Response::Rps(value) => json!({ "type": resp::RPS, "value": value }),
        }
    }
}

impl From<Response> for serde_json::Value {
    fn from(r: Response) -> Self {
        r.json()
    }
}
