//! The placeholder CPU: the first legal choices, defined once for both programs. The server answers with it for the
//! duel check's players and for the client's prompts that have no window yet; the engine's duel bot falls back on it
//! for prompts its rules don't cover, and plays with it alone as the baseline the bot is measured against
//! (`{"op":"think","brain":"placeholder"}`). Normal Summon (or Set) the first monster it may, attack with the first
//! attacker, never chain; card selections take the first `min` cards (Tributes: the cards that count most), questions
//! are answered yes, lists their first entry, zones centre out.
use serde_json::{Value as Json, json};

use crate::message::{Ask, SelectCard, SelectPlace, SelectSum};
use crate::{Response, battle, idle, loc, pos};

/// The placeholder's answer to prompt `m` (an engine message); a prompt it can't read is answered "no chain", which
/// the core refuses where that doesn't fit (its caller's repeated-prompt check ends such a duel).
pub fn answer(m: &Json) -> Response {
    match Ask::lenient(m) {
        Ok(Some(ask)) => respond(&ask, m),
        _ => Response::Chain(None),
    }
}

/// [`answer`] for a prompt already read.
pub fn respond(ask: &Ask, m: &Json) -> Response {
    let first = |min: u64, max: u64, len: usize| (0..(min.max(1) as usize).min(max as usize).min(len)).collect::<Vec<_>>();
    match ask {
        Ask::IdleCmd(p) => {
            let (action, index) = if !p.summons.is_empty() {
                (idle::SUMMON, Some(0))
            } else if !p.monster_sets.is_empty() {
                (idle::MONSTER_SET, Some(0))
            } else if p.to_bp {
                (idle::TO_BP, None)
            } else {
                (idle::TO_EP, None)
            };
            Response::IdleCmd { action, index }
        }
        Ask::BattleCmd(p) if !p.attacks.is_empty() => Response::BattleCmd { action: battle::ATTACK, index: Some(0) },
        Ask::BattleCmd(_) => Response::BattleCmd { action: battle::TO_EP, index: None },
        Ask::Chain(p) => Response::Chain(p.forced.then_some(0)),
        Ask::Card(p) => Response::Card(Some(first(p.min, p.max, p.selects.len()))),
        Ask::Tribute(p) => {
            // The cards that count most first, as many as make the Tribute (a double Tribute counts for two).
            let params = release_params(m);
            let mut order: Vec<usize> = (0..p.selects.len()).collect();
            order.sort_by_key(|&i| std::cmp::Reverse(params.get(i).copied().unwrap_or(1)));
            order.truncate((tribute_view(m, p).min.max(1) as usize).min(p.selects.len()));
            Response::Tribute(Some(order))
        }
        Ask::UnselectCard(p) => {
            // Finishing with nothing picked cancels: Panther Warrior's attack (a Tribute to declare it) came back
            // again and again. Pick first.
            Response::UnselectCard((!(p.can_finish && !p.unselect_cards.is_empty())).then_some(0))
        }
        Ask::EffectYn(_) => Response::EffectYn(true),
        Ask::YesNo(_) => Response::YesNo(true),
        Ask::Option(_) => Response::Option(0),
        Ask::SortChain(_) | Ask::SortCard(_) => Response::Sort(None),
        Ask::Counter(p) => {
            // The counters from the cards in order, each giving what it holds until the count is made.
            let mut left = p.count;
            let mut counters: Vec<u64> = p.cards.iter().map(|c| {
                let take = left.min(c.count);
                left -= take;
                take
            }).collect();
            if counters.is_empty() {
                counters.push(left);
            }
            Response::Counter(json!(counters))
        }
        // The free zones centre out, as Konami's server filled them: the player's monster zones, Spell/Trap zones, then
        // the opponent's (Give and Take and Black Garden place on the opponent's field), and the Field zone (13).
        Ask::Place(p) | Ask::Disfield(p) => {
            let field = (p.field_mask & 1 << 13 == 0).then_some((p.player, loc::SZONE, 5));
            // As many as asked (Ground Collapse and Phantom Dragon each disable two zones).
            let mut places: Vec<Json> = free_zones(p)
                .into_iter()
                .chain(field)
                .take((p.count as usize).max(1))
                .map(|(p, l, s)| json!({ "player": p, "location": l, "sequence": s }))
                .collect();
            if places.is_empty() {
                places.push(json!({ "player": p.player, "location": loc::MZONE, "sequence": 0 }));
            }
            if matches!(ask, Ask::Place(_)) { Response::Place(places) } else { Response::Disfield(places) }
        }
        // Face-up attack when allowed, else the first allowed position.
        Ask::Position(p) => {
            let allowed = if p.positions == 0 { pos::FACEUP_ATTACK } else { p.positions };
            Response::Position(if allowed & pos::FACEUP_ATTACK != 0 { pos::FACEUP_ATTACK } else { 1 << allowed.trailing_zeros() })
        }
        // The lowest `count` of the available bits.
        Ask::Race(p) | Ask::Attrib(p) => {
            let available = p.available.max(1);
            let picked: Vec<u64> = (0..64).map(|b| 1u64 << b).filter(|b| available & b != 0).take(p.count.max(1) as usize).collect();
            if matches!(ask, Ask::Race(_)) { Response::Race(picked) } else { Response::Attrib(picked) }
        }
        Ask::Number(_) => Response::Number(0),
        Ask::CardName(p) => Response::CardName(p.fits.first().copied().unwrap_or(0)),
        Ask::RockPaperScissors(_) => Response::Rps(1),
        Ask::Sum(p) => Response::Sum(sum_choice(p)),
    }
}

/// The order Konami's server filled zones in, centre out (all 129 placements of its logs).
pub const ZONE_ORDER: [u64; 5] = [2, 3, 1, 4, 0];

/// The zones a place prompt leaves free (field_mask bits set = unavailable, relative to the choosing player: monster
/// zones 0-4, Spell/Trap 8-12, the opponent's 16-20 and 24-28), as (engine player, location, sequence), centre out.
pub fn free_zones(p: &SelectPlace) -> Vec<(u64, u64, u64)> {
    let (mask, me) = (p.field_mask, p.player);
    [(me, loc::MZONE, 0), (me, loc::SZONE, 8), (1 - me, loc::MZONE, 16), (1 - me, loc::SZONE, 24)]
        .into_iter()
        .flat_map(|(p, l, base)| ZONE_ORDER.map(|s| (p, l, s, base + s)))
        .filter(|&(_, _, _, bit)| mask & 1 << bit == 0)
        .map(|(p, l, s, _)| (p, l, s))
        .collect()
}

/// What each card of a MSG_SELECT_TRIBUTE counts for (`release_param`, 1 unless a card counts as two Tributes: Double
/// Coston, Kaiba's Blue-Eyes deck's Lord of D. peers).
pub fn release_params(m: &Json) -> Vec<u64> {
    m["selects"].as_array().map_or(vec![], |cards| cards.iter().map(|c| c["release_param"].as_u64().unwrap_or(1).max(1)).collect())
}

/// A Tribute prompt in cards: the engine's `min` and `max` count Tributes, and a card may count as two, so the player
/// needs only as many cards as the biggest Tributes make `min` (one Double Coston for a Tribute of 2). The client's
/// list takes picks by card; whether the picks make enough is [`tribute_enough`].
pub fn tribute_view(m: &Json, c: &SelectCard) -> SelectCard {
    let mut params = release_params(m);
    params.sort_unstable_by(|a, b| b.cmp(a));
    let (mut sum, mut cards) = (0, 0);
    for p in &params {
        if sum >= c.min {
            break;
        }
        sum += p;
        cards += 1;
    }
    SelectCard { min: cards, max: c.max.min(c.selects.len() as u64), ..c.clone() }
}

/// Whether the picks (card indices) make a Tribute the engine takes: their `release_param`s add up to `min..=max`.
pub fn tribute_enough(m: &Json, c: &SelectCard, picks: &[usize]) -> bool {
    let params = release_params(m);
    let sum: u64 = picks.iter().map(|&i| params.get(i).copied().unwrap_or(1)).sum();
    (c.min..=c.max).contains(&sum)
}

/// MSG_SELECT_SUM {amount: the target, selects_must: cards that count without being picked, select_max: 0 exactly the
/// amount, 1 at least it with no card to spare (Ritual Summons), min..max: how many cards are picked}: its bounds
/// on how many cards are picked, where 0 sets none (Machina Fortress: discard any number of monsters with Levels of 8
/// or more).
pub fn sum_bounds(s: &SelectSum) -> (usize, usize) {
    (s.min as usize, if s.max != 0 { s.max as usize } else { u32::MAX as usize })
}

/// Whether picking these of MSG_SELECT_SUM's `selects` (indices; the `selects_must` count too) makes its sum.
pub fn sum_fits(s: &SelectSum, picked: &[usize]) -> bool {
    let selects: Vec<u64> = s.selects.iter().map(|c| c.amount).collect();
    let mut seen = vec![];
    let distinct = picked.iter().all(|&i| i < selects.len() && !seen.contains(&i) && { seen.push(i); true });
    let (min, max) = sum_bounds(s);
    distinct && (min.min(selects.len())..=max).contains(&picked.len()) && sum_reached(s, &picked.iter().map(|&i| selects[i]).collect::<Vec<_>>())
}

/// Whether cards of these amounts (with the `selects_must`) count to MSG_SELECT_SUM's target: exactly, or with
/// `select_max` at least it with no card to spare. A card's `amount` is its level, or two levels (low and high 16
/// bits) it may count as.
pub fn sum_reached(s: &SelectSum, chosen: &[u64]) -> bool {
    let must: Vec<u64> = s.selects_must.iter().map(|c| c.amount).collect();
    let target = s.amount;
    let greater = s.select_max != 0;
    let values = |a: u64| -> Vec<u64> { [a & 0xFFFF, a >> 16].into_iter().filter(|v| *v > 0).collect() };
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

/// Indices into MSG_SELECT_SUM's `selects` that make its sum (see [`sum_reached`]), the fewest cards first; none
/// that fits: every card, which the core refuses and the caller's repeated-prompt check then ends.
pub fn sum_choice(s: &SelectSum) -> Vec<usize> {
    let selects: Vec<u64> = s.selects.iter().map(|c| c.amount).collect();
    let (min, max) = sum_bounds(s);
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
            if sum_reached(s, &chosen) {
                return (0..n).filter(|i| mask >> i & 1 == 1).collect();
            }
        }
    }
    (0..n.min(max)).collect()
}
