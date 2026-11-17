//! Card moves' kinds, summon families, card table ids and settled positions.
use super::*;

/// The kinds of a card move (`0x1A`'s `param1` high half, the third argument of the client's move `0x5000E0`), as
/// Konami's server sent them (227 original duel logs): 0 a plain move (draws, discards, cards sent or banished), 2 a
/// Normal Summon, 3 a Special Summon, 4 a Spell/Trap activated from the hand, 5 a Set, 6 destroyed by an effect, 8 by
/// battle, 9 tributed (1 and 7 are rare; meaning open). The client plays no move sound for 6, 8 and 9 (the destruction
/// has its own) nor for 3 from anywhere but the hand.
pub(super) const MOVE_PLAIN: u32 = 0;
pub(super) const MOVE_NORMAL_SUMMON: u32 = 2;
pub(super) const MOVE_SPECIAL_SUMMON: u32 = 3;
pub(super) const MOVE_ACTIVATE: u32 = 4;
pub(super) const MOVE_SET: u32 = 5;
pub(super) const MOVE_DESTROYED: u32 = 6;
pub(super) const MOVE_DESTROYED_BY_BATTLE: u32 = 8;
pub(super) const MOVE_TRIBUTED: u32 = 9;

/// The summon line's kinds (SpeakSummon 0x44 param3) in Konami's logs: 0 a Normal Summon, 1 a Tribute Summon, 5 a
/// Special Summon, 6 an Extra Deck monster's arrival (after RunFusion), 8 a Synchro Summon (DuelCutin: its own slot).
pub(super) const SUMMON_NORMAL: u32 = 0;
pub(super) const SUMMON_TRIBUTE: u32 = 1;
pub(super) const SUMMON_FLIP: u32 = 4;
pub(super) const SUMMON_SPECIAL: u32 = 5;
pub(super) const SUMMON_EXTRA: u32 = 6;
pub(super) const SUMMON_SYNCHRO: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Family {
    Tribute,
    Fusion,
    Synchro,
    /// A Contact Fusion's materials (Gladiator Beasts, Elemental HEROes' Neos fusions): ProjectIgnis' contact
    /// procedures send them as a cost, with no Fusion reason. Konami's logs show no Material commands for them.
    Contact,
}

impl Family {
    /// The family of a material's move by its reason: a tribute for a summon, a Fusion or Synchro material.
    pub(super) fn of(reason: u64) -> Option<Family> {
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
    pub(super) fn set_and_run(self) -> Option<(u32, u32)> {
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
pub(super) struct Summoning {
    pub(super) family: Family,
    /// The materials moved, and the Tuners among them.
    pub(super) count: u32,
    pub(super) tuned: u32,
    /// The Extra Deck monster arrived (its Konami id): its line was said.
    pub(super) arrived: bool,
    pub(super) cid: u16,
    /// Materials marked ahead of their moves (later in the same batch), with the effect each plays as it goes (none
    /// when it was played already: a Synchro's).
    pub(super) played: Vec<(u16, Option<Message>)>,
    /// The materials' uids, the Tuners first (the animations' material list).
    pub(super) uids: Vec<u32>,
}

/// A move's kind (see [`MOVE_PLAIN`]) from MSG_MOVE's reason and the places.
pub(super) fn move_kind(reason: u64, from: Place, to: Place) -> u32 {
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
pub(super) fn with_card_ids(cards: &HashMap<u16, CardStats>, out: &mut [Message]) {
    for m in out.iter_mut().filter(|m| m.get("id").and_then(Value::as_u32) == Some(cmd::CARD)) {
        let row = m.get("param2").and_then(Value::as_u32).unwrap_or(0) as u16;
        let Some(cid) = cards.get(&row).map(|c| c.cid).filter(|_| row != 0) else { continue };
        if let Some((_, v)) = m.fields.iter_mut().find(|(k, _)| k == "param3") {
            *v = Value::U32(cid as u32);
        }
    }
}

/// Whether an engine location is an Xyz material's (ocgcore-wasm adds `overlay_sequence`; a raw location has bit 0x80).
pub(super) fn is_material(loc: &Json) -> bool {
    loc["overlay_sequence"].is_u64() || loc["location"].as_u64().unwrap_or(0) & loc::OVERLAY != 0
}

/// Tokens' uids, after any deck's (a move carries the uid in 16 bits).
pub(super) const FIRST_TOKEN_UID: u32 = 0x4000;

/// The table entry flag of a card in this zone: open in the Graveyard and among the banished cards.
pub(super) fn open(zone: u32) -> bool {
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
