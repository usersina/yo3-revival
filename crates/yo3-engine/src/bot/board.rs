//! What the bot's player can see of a duel, and how good a position looks to it. The core knows every card; the
//! board hides from the bot what a player couldn't see: the opponent's hand (only its size), the opponent's Set
//! cards, and the opponent's monsters that were face-down when the bot started thinking, even if a look-ahead copy
//! has turned them face-up since (it valued them by a guess before, so it values them by the same guess after).
use std::collections::HashSet;
use std::sync::Arc;

use crate::cards::Library;
use crate::core::Duel;
use crate::decode::{self, Reader};

use super::policy::FACEDOWN_GUESS;

const LOCATION_DECK: u32 = 0x01;
const LOCATION_HAND: u32 = 0x02;
const LOCATION_MZONE: u32 = 0x04;
const LOCATION_SZONE: u32 = 0x08;
const LOCATION_GRAVE: u32 = 0x10;
const LOCATION_REMOVED: u32 = 0x20;
const LOCATION_EXTRA: u32 = 0x40;
const QUERY_TYPE: u32 = 0x8;
const POS_FACEUP: u32 = 0x5;
const POS_ATTACK: u32 = 0x3;
const TYPE_MONSTER: i64 = 0x1;
const TYPE_TRAP: i64 = 0x4;
const TYPE_QUICKPLAY: i64 = 0x10000;

#[derive(Clone, Debug)]
pub struct Monster {
    pub seq: u8,
    pub faceup: bool,
    pub attack_pos: bool,
    pub atk: i32,
    pub def: i32,
    /// Whether the bot knows what it is (its own, or face-up when it started thinking).
    pub known: bool,
}

impl Monster {
    /// Its strength on the field: ATK in Attack Position, DEF in Defense Position (or half its ATK, which it gets back
    /// by changing position), a guess when unknown.
    pub fn worth(&self) -> i32 {
        if !self.known {
            FACEDOWN_GUESS
        } else if self.attack_pos {
            self.atk.max(0)
        } else {
            self.def.max(self.atk / 2).max(0)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Spell {
    pub faceup: bool,
    /// Its card type, when the bot knows it.
    pub kind: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct Side {
    pub lp: i32,
    pub hand: u32,
    pub deck: u32,
    pub grave: u32,
    pub removed: u32,
    pub extra: u32,
    pub monsters: Vec<Monster>,
    pub spells: Vec<Spell>,
}

impl Side {
    pub fn monster(&self, seq: u8) -> Option<&Monster> {
        self.monsters.iter().find(|m| m.seq == seq)
    }

    /// Cards in hand and on the field: what a player has to play with.
    fn cards(&self) -> u32 {
        self.hand + self.monsters.len() as u32 + self.spells.len() as u32
    }
}

/// The duel as player `me` sees it.
#[derive(Clone)]
pub struct Board {
    pub me: u8,
    pub sides: [Side; 2],
    /// The ATK of the monster the bot just declared an attack with (its target choice comes next).
    pub attacker: Option<i32>,
    library: Arc<Library>,
}

/// The opponent's monster zones the bot couldn't see into when it started thinking.
pub type Hidden = HashSet<u8>;

impl Board {
    /// Player `me`'s view of `duel`; `hidden` are the opponent's monster zones it is not allowed to know.
    pub fn observe(duel: &mut Duel, me: u8, library: &Arc<Library>, hidden: &Hidden) -> Board {
        let field = duel.query_field();
        let players = decode::field(&mut Reader::new(&field)).ok().and_then(|mut f| f.remove("players")).unwrap_or_default();
        let mut sides: [Side; 2] = Default::default();
        for t in 0..2u8 {
            let side = &mut sides[t as usize];
            side.lp = players[t as usize]["lp"].as_u64().unwrap_or(0) as u32 as i32;
            side.hand = duel.count(t, LOCATION_HAND);
            side.deck = duel.count(t, LOCATION_DECK);
            side.grave = duel.count(t, LOCATION_GRAVE);
            side.removed = duel.count(t, LOCATION_REMOVED);
            side.extra = duel.count(t, LOCATION_EXTRA);
            let flags = decode::QUERY_CODE | decode::QUERY_POSITION | decode::QUERY_ATTACK | decode::QUERY_DEFENSE;
            let mzone = decode::query_location(&duel.query_location(flags, t, LOCATION_MZONE)).unwrap_or_default();
            for (seq, card) in mzone.into_iter().enumerate() {
                let Some(card) = card else { continue };
                let position = card.get(decode::QUERY_POSITION).unwrap_or(0);
                let faceup = position & POS_FACEUP != 0;
                let known = t == me || faceup && !hidden.contains(&(seq as u8));
                side.monsters.push(Monster {
                    seq: seq as u8,
                    faceup,
                    attack_pos: position & POS_ATTACK != 0,
                    atk: if known { card.get(decode::QUERY_ATTACK).unwrap_or(0) as i32 } else { 0 },
                    def: if known { card.get(decode::QUERY_DEFENSE).unwrap_or(0) as i32 } else { 0 },
                    known,
                });
            }
            let szone = decode::query_location(&duel.query_location(decode::QUERY_CODE | decode::QUERY_POSITION | QUERY_TYPE, t, LOCATION_SZONE))
                .unwrap_or_default();
            for card in szone.into_iter().flatten() {
                let faceup = card.get(decode::QUERY_POSITION).unwrap_or(0) & POS_FACEUP != 0;
                let kind = (t == me || faceup).then(|| card.get(QUERY_TYPE).unwrap_or(0) as i64);
                side.spells.push(Spell { faceup, kind });
            }
        }
        Board { me, sides, attacker: None, library: library.clone() }
    }

    /// The opponent's monster zones holding a card the bot can't see now (for the look-ahead's `hidden`).
    pub fn hidden_monsters(duel: &mut Duel, me: u8) -> Hidden {
        let flags = decode::QUERY_POSITION;
        let mzone = decode::query_location(&duel.query_location(flags, 1 - me, LOCATION_MZONE)).unwrap_or_default();
        mzone.into_iter().enumerate()
            .filter(|(_, c)| c.as_ref().is_some_and(|c| c.get(decode::QUERY_POSITION).unwrap_or(0) & POS_FACEUP == 0))
            .map(|(seq, _)| seq as u8)
            .collect()
    }

    pub fn base_atk(&self, code: u32) -> i32 {
        self.library.row(code).map_or(0, |r| r[2] as i32)
    }

    pub fn base_def(&self, code: u32) -> i32 {
        self.library.row(code).map_or(0, |r| r[3] as i32)
    }

    /// A card's worth from its data: a monster by its better stat, any other card a fixed amount.
    pub fn base_worth(&self, code: u32) -> i32 {
        match self.library.row(code) {
            Some(r) if r[0] & TYPE_MONSTER != 0 => (r[2].max(r[3]) as i32).max(0),
            Some(_) => 800,
            None => 0,
        }
    }

    /// How good the position is for the bot, in Life Points: the LP difference, about 700 a card in hand or on the
    /// field, its monsters' strength, what the opponent's next battle would cost it ([`exposure`]), Traps ready to
    /// spring, an empty Deck. A won duel is worth everything.
    pub fn value(&self) -> f64 {
        let me = &self.sides[self.me as usize];
        let them = &self.sides[1 - self.me as usize];
        if me.lp <= 0 {
            return -1e9;
        }
        if them.lp <= 0 {
            return 1e9;
        }
        let mut v = (me.lp - them.lp) as f64;
        v += 700.0 * (me.cards() as f64 - them.cards() as f64);
        let strength = |s: &Side| s.monsters.iter().map(|m| m.worth() as f64).sum::<f64>();
        v += 0.6 * (strength(me) - strength(them));
        v -= 0.6 * exposure(me, them);
        for s in me.spells.iter().filter(|s| !s.faceup) {
            match s.kind {
                Some(k) if k & (TYPE_TRAP | TYPE_QUICKPLAY) != 0 => v += 60.0,
                Some(_) => v -= 100.0,
                None => {}
            }
        }
        if me.deck == 0 {
            v -= 3000.0;
        }
        v
    }
}

/// What `them`'s next Battle Phase would cost `me` with the monsters on the field now, in LP: each of their known
/// attackers, strongest first, destroys the most valuable of `me`'s monsters it beats (losing that card and, from
/// Attack Position, the damage), or attacks directly once `me` has no monsters left. Only monsters the bot knows
/// attack: a face-down one might not be able to.
pub fn exposure(me: &Side, them: &Side) -> f64 {
    let mut attackers: Vec<i32> = them.monsters.iter().filter(|m| m.known && m.attack_pos).map(|m| m.atk).collect();
    attackers.sort_unstable_by(|a, b| b.cmp(a));
    let mut mine: Vec<&Monster> = me.monsters.iter().collect();
    let mut cost = 0.0;
    for atk in attackers {
        if mine.is_empty() {
            cost += atk as f64;
            continue;
        }
        let beaten = mine.iter().enumerate()
            .filter(|(_, m)| if m.attack_pos { atk > m.atk } else { atk > m.def })
            .max_by_key(|(_, m)| m.worth())
            .map(|(i, _)| i);
        if let Some(i) = beaten {
            let m = mine.remove(i);
            cost += 700.0 + 0.6 * m.worth() as f64 + if m.attack_pos { (atk - m.atk) as f64 } else { 0.0 };
        }
    }
    cost
}
