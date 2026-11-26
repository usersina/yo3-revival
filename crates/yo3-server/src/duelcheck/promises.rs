//! What a card's effects promise and what their resolutions did: the per-card fuzz's checks (`tests/card_fuzz.rs`).
//!
//! A script declares what each effect does with `SetCategory` (CATEGORY_DESTROY, CATEGORY_SPECIAL_SUMMON...). The self-play
//! follows every chain link from its activation (MSG_CHAINING: the card and the effect's description) to its resolution
//! (between MSG_CHAIN_SOLVING and MSG_CHAIN_SOLVED) and notes what happened there. An effect that resolves several times
//! and never once does one of the things it declares (the anime Dark Element: a Special Summon that never came), or a
//! player's effect that never activates at all (the anime Emerald Tortoise's), is worth a scenario test: the fuzz lists
//! them, it doesn't judge them (an effect may fizzle for good reasons).
use std::collections::BTreeMap;

use crate::ocg::message::Event;

// EDOPro's CATEGORY_* (CardScripts' constant.lua): the ones a resolution's messages show.
pub const DESTROY: u64 = 0x1;
pub const RELEASE: u64 = 0x2;
pub const REMOVE: u64 = 0x4;
pub const TOHAND: u64 = 0x8;
pub const TODECK: u64 = 0x10;
pub const TOGRAVE: u64 = 0x20;
pub const DECKDES: u64 = 0x40;
pub const HANDES: u64 = 0x80;
pub const SPECIAL_SUMMON: u64 = 0x200;
pub const TOKEN: u64 = 0x400;
pub const POSITION: u64 = 0x1000;
pub const CONTROL: u64 = 0x2000;
pub const DRAW: u64 = 0x10000;
pub const SEARCH: u64 = 0x20000;
pub const EQUIP: u64 = 0x40000;
pub const DAMAGE: u64 = 0x80000;
pub const RECOVER: u64 = 0x100000;
pub const ATKCHANGE: u64 = 0x200000;
pub const DEFCHANGE: u64 = 0x400000;
pub const COUNTER: u64 = 0x800000;
pub const COIN: u64 = 0x1000000;
pub const DICE: u64 = 0x2000000;
pub const LVCHANGE: u64 = 0x8000000;

/// The categories a resolution's messages can show; the others (a negation, a declaration...) are left unjudged. ATK, DEF
/// and Level changes too: the session reports the monsters' stats once a step, after the chain link that changed them has
/// closed, so they can't be told apart from the link's.
pub const OBSERVABLE: u64 = DESTROY | RELEASE | REMOVE | TOHAND | TODECK | TOGRAVE | DECKDES | HANDES | SPECIAL_SUMMON | TOKEN
    | POSITION | CONTROL | DRAW | SEARCH | EQUIP | DAMAGE | RECOVER | COUNTER | COIN | DICE;

const NAMES: [(&str, u64); 23] = [
    ("DESTROY", DESTROY), ("RELEASE", RELEASE), ("REMOVE", REMOVE), ("TOHAND", TOHAND), ("TODECK", TODECK), ("TOGRAVE", TOGRAVE),
    ("DECKDES", DECKDES), ("HANDES", HANDES), ("SPECIAL_SUMMON", SPECIAL_SUMMON), ("TOKEN", TOKEN), ("POSITION", POSITION),
    ("CONTROL", CONTROL), ("DRAW", DRAW), ("SEARCH", SEARCH), ("EQUIP", EQUIP), ("DAMAGE", DAMAGE), ("RECOVER", RECOVER),
    ("ATKCHANGE", ATKCHANGE), ("DEFCHANGE", DEFCHANGE), ("COUNTER", COUNTER), ("COIN", COIN), ("DICE", DICE), ("LVCHANGE", LVCHANGE),
];

/// The names of the categories in `bits` (for a report).
pub fn names(bits: u64) -> Vec<&'static str> {
    NAMES.iter().filter(|(_, b)| bits & b != 0).map(|(n, _)| *n).collect()
}

/// One effect of a script: its description (`aux.Stringid(code, n)` is code << 20 | n, utility.lua; 0 without one), the
/// categories it
/// declares, and whether a player activates it (it starts a chain link: activation, ignition, trigger, quick). Match an
/// activation's description by [`effect_number`]: an anime script often names its effects by the printed card's code.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Declared {
    pub description: u64,
    pub categories: u64,
    pub activated: bool,
}

/// The effects `script` (the script of the card `code`) registers in plain `local eN=Effect.CreateEffect(...)` blocks: a
/// line scan of `SetDescription`, `SetCategory` and `SetType`, clones taking their source's. Effects a helper registers
/// (`aux.AddEquipProcedure`...) or that set their category later are not seen. Effects sharing a description are merged.
pub fn declared(script: &str, code: u64) -> Vec<Declared> {
    let mut effects: BTreeMap<String, Declared> = BTreeMap::new();
    let mut order: Vec<String> = vec![];
    for line in script.lines() {
        let line = line.split("--").next().unwrap_or("").trim();
        let line = line.strip_prefix("local ").unwrap_or(line);
        if let Some((var, rest)) = line.split_once('=') {
            let (var, rest) = (var.trim(), rest.trim());
            if !var.is_empty() && var.chars().all(|c| c.is_alphanumeric() || c == '_') {
                if rest.starts_with("Effect.CreateEffect(") {
                    effects.insert(var.to_string(), Declared::default());
                    order.push(var.to_string());
                    continue;
                }
                if let Some(source) = rest.strip_suffix(":Clone()") {
                    let copy = effects.get(source.trim()).cloned().unwrap_or_default();
                    effects.insert(var.to_string(), copy);
                    order.push(var.to_string());
                    continue;
                }
            }
        }
        let Some((var, call)) = line.split_once(':') else { continue };
        let Some(e) = effects.get_mut(var.trim()) else { continue };
        let arg = call.split_once('(').map(|(_, a)| a.trim_end_matches(')')).unwrap_or("");
        if call.starts_with("SetDescription(") {
            e.description = stringid(arg, code).unwrap_or(0);
        } else if call.starts_with("SetCategory(") {
            e.categories = arg.split(['+', '|']).filter_map(|t| {
                let t = t.trim().strip_prefix("CATEGORY_")?;
                NAMES.iter().find(|(n, _)| *n == t).map(|&(_, b)| b)
            }).fold(0, |a, b| a | b);
        } else if call.starts_with("SetType(") {
            e.activated = ["EFFECT_TYPE_ACTIVATE", "EFFECT_TYPE_IGNITION", "EFFECT_TYPE_TRIGGER", "EFFECT_TYPE_QUICK"].iter().any(|t| arg.contains(t));
        }
    }
    let mut merged: BTreeMap<u64, Declared> = BTreeMap::new();
    for var in order {
        let e = &effects[&var];
        if !e.activated {
            continue;
        }
        let m = merged.entry(e.description).or_insert(Declared { description: e.description, categories: 0, activated: true });
        m.categories |= e.categories;
    }
    merged.into_values().collect()
}

/// `aux.Stringid(id, n)` or `aux.Stringid(12345, n)` as its number, `code` standing for `id`.
fn stringid(arg: &str, code: u64) -> Option<u64> {
    let inner = arg.trim().strip_prefix("aux.Stringid(")?;
    let (of, n) = inner.split_once(',')?;
    let n: u64 = n.trim().trim_end_matches(')').trim().parse().ok()?;
    let of = of.trim();
    let base = if of == "id" || of == "s.id" { code } else { of.parse().ok()? };
    Some(base << 20 | n)
}

/// An effect description's number within its card (`aux.Stringid`'s `n`).
pub fn effect_number(description: u64) -> u64 {
    description & 0xFFFFF
}

// ocgcore's LOCATION_* and REASON_*.
const DECK: u64 = 0x1;
const HAND: u64 = 0x2;
const MZONE: u64 = 0x4;
const SZONE: u64 = 0x8;
const GRAVE: u64 = 0x10;
const REMOVED: u64 = 0x20;
const EXTRA: u64 = 0x40;
const REASON_DESTROY: u64 = 0x1;
const REASON_RELEASE: u64 = 0x2;

/// The categories an engine message shows happening.
pub fn shown(e: &Event) -> u64 {
    match e {
        Event::Move(mv) => {
            let (from, to) = (mv.from.location, mv.to.location);
            let mut bits = 0;
            if mv.reason & REASON_DESTROY != 0 { bits |= DESTROY }
            if mv.reason & REASON_RELEASE != 0 { bits |= RELEASE }
            match to {
                GRAVE => bits |= TOGRAVE | if from == DECK { DECKDES } else if from == HAND { HANDES } else { 0 },
                HAND => bits |= TOHAND | if from == DECK { SEARCH } else { 0 },
                REMOVED => bits |= REMOVE,
                DECK | EXTRA if from != 0 && from != DECK => bits |= TODECK,
                MZONE if from == 0 => bits |= SPECIAL_SUMMON | TOKEN,
                MZONE if from != MZONE => bits |= SPECIAL_SUMMON,
                _ => {}
            }
            // A card kept on the field by the other player (a monster taken, Bounce Spell's Spell).
            if from == to && from & (MZONE | SZONE) != 0 && mv.from.controller != mv.to.controller {
                bits |= CONTROL;
            }
            bits
        }
        Event::SpSummoning(_) => SPECIAL_SUMMON,
        Event::Draw(_) => DRAW,
        Event::Damage(_) => DAMAGE,
        Event::Recover(_) => RECOVER,
        Event::Equip(_) => EQUIP,
        Event::PosChange(_) => POSITION,
        Event::AddCounter(_) => COUNTER,
        Event::TossCoin(_) => COIN,
        Event::TossDice(_) => DICE,
        Event::Swap(_) => CONTROL,
        _ => 0,
    }
}

/// One effect's activations in the self-play, and what its resolutions showed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub activated: u32,
    pub resolved: u32,
    /// Every category a resolution of it showed.
    pub seen: u64,
}

/// The chain links of a duel, from activation to resolution: the tallies by (card's Konami id, effect description).
#[derive(Debug, Default)]
pub struct Links {
    stack: Vec<(u16, u64, u64)>,
    solving: Option<usize>,
}

impl Links {
    /// Follows one engine message.
    pub fn observe(&mut self, e: &Event, tallies: &mut BTreeMap<(u16, u64), Tally>) {
        match e {
            Event::Chaining(c) => {
                let key = (c.loc.cid.unwrap_or(0) as u16, c.description);
                tallies.entry(key).or_default().activated += 1;
                self.stack.push((key.0, key.1, 0));
            }
            Event::ChainSolving(c) => self.solving = (c.chain_size as usize).checked_sub(1),
            Event::ChainSolved(c) => {
                if let Some(link) = (c.chain_size as usize).checked_sub(1).and_then(|i| self.stack.get(i)) {
                    let t = tallies.entry((link.0, link.1)).or_default();
                    t.resolved += 1;
                    t.seen |= link.2;
                }
                self.solving = None;
            }
            Event::ChainEnd => {
                self.stack.clear();
                self.solving = None;
            }
            e => {
                if let Some(link) = self.solving.and_then(|i| self.stack.get_mut(i)) {
                    link.2 |= shown(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scripts_effects_with_their_descriptions_categories_and_clones() {
        let script = "local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetDescription(aux.Stringid(id,0))
	e1:SetCategory(CATEGORY_SPECIAL_SUMMON+CATEGORY_DAMAGE)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	c:RegisterEffect(e1)
	local e2=e1:Clone()
	e2:SetDescription(aux.Stringid(id,1))
	e2:SetCategory(CATEGORY_DESTROY|CATEGORY_TOGRAVE)
	c:RegisterEffect(e2)
	local e3=Effect.CreateEffect(c)
	e3:SetType(EFFECT_TYPE_FIELD)
	e3:SetCode(EFFECT_UPDATE_ATTACK)
	c:RegisterEffect(e3)
end";
        assert_eq!(declared(script, 100), vec![
            Declared { description: 100 << 20, categories: SPECIAL_SUMMON | DAMAGE, activated: true },
            Declared { description: 100 << 20 | 1, categories: DESTROY | TOGRAVE, activated: true },
        ]);
    }
}
