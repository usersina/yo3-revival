//! What the bot knows about its own deck: its **bosses** (the monsters it plays toward) and the **pieces** that bring
//! them out (materials, the Ritual Spell, the Field Spell they need, the cards that search them). A position is worth
//! more with a boss on the field and pieces in hand, on the field or in the Graveyard, so that a step toward a boss
//! (Chaos Distill before Helios, a Fusion's materials kept back) no longer looks like a card spent for nothing.
//!
//! Derived from the cards' data and scripts, as a player reads their own cards: a card's script names other cards
//! (`s.listed_names`, the codes a Fusion or Ritual procedure takes, `CARD_*` constants) and archetypes (`SET_*`);
//! two cards of the deck are related when either names the other or an archetype the other belongs to. A character's
//! game plan (`plan` in its style: bosses and pieces by name) adds what scripts don't say (an Earthbound Immortal's
//! Field Spell). The bot knows only its own deck: the opponent's is unknown to it.
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use crate::cards::Library;

const TYPE_MONSTER: i64 = 0x1;
const TYPE_RITUAL: i64 = 0x80;
const TYPE_FUSION: i64 = 0x40;
const TYPE_SYNCHRO: i64 = 0x2000;
const TYPE_SPSUMMON: i64 = 0x2000000;
const LOCATION_DECK: u32 = 0x01;
const LOCATION_EXTRA: u32 = 0x40;

/// A main deck monster this strong is a boss.
const BOSS_ATK: i64 = 2400;

/// The bot's deck as it knows it.
#[derive(Clone, Debug, Default)]
pub struct Knowledge {
    /// The monsters it plays toward.
    pub bosses: HashSet<u32>,
    /// The cards that bring a boss out (a boss may be one too: Dark Magician for Dark Paladin).
    pub pieces: HashSet<u32>,
}

/// A deck's game plan from a character's style: `{"bosses": [names], "pieces": [names]}`, resolved to codes.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub bosses: Vec<u32>,
    pub pieces: Vec<u32>,
}

impl Knowledge {
    /// No knowledge: the opponent's deck, or a bot that plays without it.
    pub fn none() -> Arc<Knowledge> {
        static NONE: LazyLock<Arc<Knowledge>> = LazyLock::new(|| Arc::new(Knowledge::default()));
        NONE.clone()
    }

    /// What player `team` knows of its deck: the cards it began with (`cards`: (team, location, code), as a duel's
    /// setup lists them), read from `library`, with `plan`'s bosses and pieces added.
    pub fn of_deck(library: &Library, cards: &[(u8, u32, u32)], team: u8, plan: &Plan) -> Knowledge {
        let own: Vec<(u32, u32)> = cards.iter().filter(|(t, ..)| *t == team).map(|&(_, l, c)| (l, c)).collect();
        let codes: HashSet<u32> = own.iter().map(|&(_, c)| c).collect();
        let mut bosses: HashSet<u32> = HashSet::new();
        for &(location, code) in &own {
            let Some(row) = library.row(code) else { continue };
            let (kind, atk) = (row[0], row[2]);
            if kind & TYPE_MONSTER == 0 {
                continue;
            }
            let boss = if location == LOCATION_EXTRA {
                kind & (TYPE_FUSION | TYPE_SYNCHRO) != 0
            } else {
                location == LOCATION_DECK && (atk >= BOSS_ATK || kind & (TYPE_RITUAL | TYPE_SPSUMMON) != 0)
            };
            if boss {
                bosses.insert(code);
            }
        }
        bosses.extend(plan.bosses.iter().copied());
        // Who names whom: each card's named cards and archetypes.
        let named: HashMap<u32, (HashSet<u32>, HashSet<u64>)> = codes.iter().map(|&c| (c, names(library, c))).collect();
        let setcodes = |c: u32| library.row(c).map_or(0, |r| r[7] as u64);
        let related = |a: u32, b: u32| {
            let (codes_a, sets_a) = &named[&a];
            let (codes_b, sets_b) = &named[&b];
            codes_a.contains(&b) || codes_b.contains(&a) || in_sets(setcodes(b), sets_a) || in_sets(setcodes(a), sets_b)
        };
        let mut pieces: HashSet<u32> = codes.iter().copied()
            .filter(|&c| bosses.iter().any(|&b| b != c && named.contains_key(&b) && related(c, b)))
            .collect();
        pieces.extend(plan.pieces.iter().copied());
        Knowledge { bosses, pieces }
    }

    pub fn is_boss(&self, code: u32) -> bool {
        self.bosses.contains(&code)
    }

    pub fn is_piece(&self, code: u32) -> bool {
        self.pieces.contains(&code)
    }
}

/// Whether a card with setcode column `setcode` belongs to any of `sets` (as the core's IsSetCard: the archetype's low
/// 12 bits, and every bit of a sub-archetype).
fn in_sets(setcode: u64, sets: &HashSet<u64>) -> bool {
    sets.iter().any(|&set| {
        let mut s = setcode;
        while s > 0 {
            let part = s & 0xffff;
            if part & 0xfff == set & 0xfff && part & set == set {
                return true;
            }
            s >>= 16;
        }
        false
    })
}

/// The cards and archetypes card `code`'s script names (its own code left out).
fn names(library: &Library, code: u32) -> (HashSet<u32>, HashSet<u64>) {
    let script = library.script(&format!("c{code}.lua")).map(|s| String::from_utf8_lossy(&s).into_owned()).unwrap_or_default();
    let consts = constants(library);
    let mut cards = HashSet::new();
    let mut sets = HashSet::new();
    for word in script.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        if word.is_empty() {
            continue;
        }
        if word.as_bytes()[0].is_ascii_digit() {
            if (5..=9).contains(&word.len()) && let Ok(n) = word.parse::<u32>() && n != code && library.row(n).is_some() {
                cards.insert(n);
            }
        } else if word.starts_with("CARD_") {
            cards.extend(consts.get(word).map(|&n| n as u32));
        } else if word.starts_with("SET_") {
            sets.extend(consts.get(word).copied());
        }
    }
    (cards, sets)
}

/// The `CARD_*` and `SET_*` constants of the card scripts' Lua (card_counter_constants.lua,
/// archetype_setcode_constants.lua, constant.lua), read once.
fn constants(library: &Library) -> &'static HashMap<String, u64> {
    static ALL: std::sync::OnceLock<HashMap<String, u64>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut out = HashMap::new();
        for file in ["constant.lua", "card_counter_constants.lua", "archetype_setcode_constants.lua"] {
            let text = library.script(file).map(|s| String::from_utf8_lossy(&s).into_owned()).unwrap_or_default();
            for line in text.lines() {
                let Some((name, value)) = line.split_once('=') else { continue };
                let name = name.trim();
                if !(name.starts_with("CARD_") || name.starts_with("SET_")) || name.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                    continue;
                }
                let value = value.split("--").next().unwrap_or_default().trim();
                let parsed = match value.strip_prefix("0x") {
                    Some(hex) => u64::from_str_radix(hex, 16).ok(),
                    None => value.parse::<u64>().ok(),
                };
                if let Some(v) = parsed {
                    out.insert(name.to_string(), v);
                }
            }
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fusion_knows_its_materials_and_a_plan_adds_what_scripts_dont_say() {
        let vendor = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine/vendor");
        if !vendor.join("BabelCDB/cards.cdb").is_file() {
            return; // engine/fetch.sh not run
        }
        let cards = crate::cards::Cards::open(&vendor).expect("the card data");
        const DARK_PALADIN: u32 = 98502113; // Fusion.AddProcMix(c,true,true,CARD_DARK_MAGICIAN,CARD_BUSTER_BLADER)
        const DARK_MAGICIAN: u32 = 46986414;
        const BUSTER_BLADER: u32 = 78193831;
        const POLYMERIZATION: u32 = 24094653;
        const KURIBOH: u32 = 40640057;
        let deck = [(0, LOCATION_EXTRA, DARK_PALADIN), (0, LOCATION_DECK, DARK_MAGICIAN), (0, LOCATION_DECK, BUSTER_BLADER),
            (0, LOCATION_DECK, POLYMERIZATION), (0, LOCATION_DECK, KURIBOH), (1, LOCATION_DECK, DARK_MAGICIAN)];
        let k = Knowledge::of_deck(&cards.library, &deck, 0, &Plan::default());
        assert!(k.is_boss(DARK_PALADIN) && k.is_boss(DARK_MAGICIAN), "{k:?}");
        assert!(k.is_piece(DARK_MAGICIAN) && k.is_piece(BUSTER_BLADER), "Dark Paladin's materials: {k:?}");
        assert!(!k.is_piece(KURIBOH) && !k.is_piece(POLYMERIZATION), "{k:?}");
        let k = Knowledge::of_deck(&cards.library, &deck, 0, &Plan { bosses: vec![], pieces: vec![POLYMERIZATION] });
        assert!(k.is_piece(POLYMERIZATION));
        let theirs = Knowledge::of_deck(&cards.library, &deck, 1, &Plan::default());
        assert!(!theirs.is_boss(DARK_PALADIN), "only its own deck");
    }

    #[test]
    fn a_card_belongs_to_an_archetype_and_its_sub_archetypes_only_with_every_bit() {
        let sets: HashSet<u64> = [0x2].into();
        assert!(in_sets(0x2, &sets));
        assert!(in_sets(0x1002, &sets), "R-Genex is a Genex");
        assert!(!in_sets(0x3, &sets));
        let sub: HashSet<u64> = [0x1002].into();
        assert!(!in_sets(0x2, &sub), "a Genex isn't an R-Genex");
        assert!(in_sets(0x5_1002, &sub), "the second of two archetypes");
    }
}
