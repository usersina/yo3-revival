//! An animation shows a card the client knows (#79): every summon, Fusion or Synchro animation, a summon's material
//! effects, an activation and the avatar's summon and activation lines name their card by uid, and the client draws
//! that uid's card from its card table (1000 {uid, row}). A card it has with row 0 (an opponent's hand, Extra Deck or
//! Set card, as against a player) has no picture: Jaden's Fusion Summons showed only the light.
//!
//! Konami's server always told the client the card first (a 1000 with its row just before RunFusion for an opponent's
//! Fusion or Synchro): in its 228 duel logs, none of about 28,000 such commands named a card with row 0
//! (`tests/summon_order.rs` replays them through this rule).
use std::collections::HashMap;

use crate::wire::{Place, cmd, zone};

/// The uid whose card a command draws, if it names one.
pub fn names(id: u32, [_, p2, p3, _]: [u32; 4]) -> Option<u32> {
    match id {
        // {place, cid, uid}: the activation's card and a card turned up for a moment.
        cmd::CARD_ACTIVATE | cmd::CARD_CHEAT => Some(p3 & 0xFFFF),
        // {player, zone, uid, variant}: a summon's material marks and effects.
        cmd::TRIBUTE_SET | cmd::TRIBUTE_RUN | cmd::MATERIAL_SET | cmd::MATERIAL_RUN | cmd::TUNING_SET | cmd::TUNING_RUN => Some(p3 & 0xFFFF),
        // {place or player, uid}: the summon effects and the Fusion or Synchro animation.
        cmd::RUN_SUMMON | cmd::RUN_SP_SUMMON | cmd::RUN_FUSION => Some(p2 & 0xFFFF),
        // {player, 0x8000 | uid, kind}: the avatar's line names the card (without the flag it names none).
        cmd::SPEAK_SUMMON | cmd::SPEAK_ACTIVATE if p2 & 0x8000 != 0 => Some(p2 & 0x7FFF),
        _ => None,
    }
}

/// What the client's card table shows of each uid, as the commands sent so far gave it.
#[derive(Clone, Debug, Default)]
pub struct Shown {
    rows: HashMap<u32, u32>,
    /// Each card's place as its last move gave it: a monster Special Summoned face down stays hidden from the opponent
    /// (Konami's server told its card even then, which a hover would show).
    places: HashMap<u32, Place>,
    /// The material list since the last animation (DuelFusion makes a card image of each).
    materials: Vec<u32>,
}

impl Shown {
    /// Follows one command sent to the client: the problem, if it names a card the client can't draw.
    pub fn command(&mut self, id: u32, p: [u32; 4]) -> Option<String> {
        match id {
            cmd::CARD => {
                self.rows.insert(p[0] & 0xFF_FFFF, p[1]);
                return None;
            }
            cmd::MOVE => {
                self.places.insert(p[0] & 0xFFFF, Place::of_move(p[2]));
            }
            cmd::MATERIAL_LIST => self.materials = p[1..].iter().take(p[0] as usize).copied().collect(),
            cmd::MATERIAL_LIST_MORE => self.materials.extend(p.iter().filter(|&&u| u != 0)),
            _ => {}
        }
        let mut problem = None;
        // Hidden on purpose: a face-down monster's summon and line, and CardCheat turning a card back down (no card id).
        let face_down = |uid: u32| self.places.get(&uid).is_some_and(|at| at.zone <= zone::FIELD_SPELL && !at.face_up);
        if let Some(uid) = names(id, p)
            && self.rows.get(&uid).is_some_and(|&row| row == 0)
            && !face_down(uid)
            && !(id == cmd::CARD_CHEAT && p[1] == 0)
        {
            problem = Some(format!("command {id:#x} shows uid {uid}, which the client's card table holds without its card (row 0): it draws no picture"));
        }
        if id == cmd::RUN_FUSION {
            let hidden: Vec<u32> = self.materials.iter().copied().filter(|u| self.rows.get(u).is_some_and(|&row| row == 0)).collect();
            if !hidden.is_empty() && problem.is_none() {
                problem = Some(format!("RunFusion's material list holds uid(s) {hidden:?} without their card (row 0)"));
            }
            self.materials.clear();
        }
        problem
    }
}
