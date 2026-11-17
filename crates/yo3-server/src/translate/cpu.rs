//! The placeholder AI's answers, the sums it picks, free zones and the repeat guard.
use super::*;

/// The placeholder AI's answer to any prompt (see [`cpu_answer`]).
pub fn bot_answer(m: &Json) -> Json {
    cpu_answer(m["type"].as_u64().unwrap_or(0), m)
}

/// MSG_SELECT_SUM {amount: the target, selects_must: cards that count without being picked, select_max: 0 exactly the
/// amount, 1 at least it with no card to spare (Ritual Summons), min..max: how many cards are picked}: its bounds
/// on how many cards are picked, where 0 sets none (Machina Fortress: discard any number of monsters with Levels of 8
/// or more).
pub(super) fn sum_bounds(m: &Json) -> (usize, usize) {
    (m["min"].as_u64().unwrap_or(0) as usize, m["max"].as_u64().filter(|&m| m != 0).unwrap_or(u32::MAX as u64) as usize)
}

/// Whether picking these of MSG_SELECT_SUM's `selects` (indices; the `selects_must` count too) makes its sum.
pub(super) fn sum_fits(m: &Json, picked: &[usize]) -> bool {
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
pub(super) fn sum_reached(m: &Json, chosen: &[u64]) -> bool {
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
pub(super) fn sum_choice(m: &Json) -> Vec<usize> {
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
    pub(super) last: Option<Json>,
    pub(super) times: u32,
}

/// How many times the same prompt may come back in a row before the duel is given up.
pub(super) const MAX_REPEATS: u32 = 8;
/// The most picks of a card list kept (a list holds a few dozen entries).
pub(super) const MAX_PICKS: usize = 256;

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
/// The order Konami's server filled zones in, centre out (all 129 placements of its logs).
pub(super) const ZONE_ORDER: [u64; 5] = [2, 3, 1, 4, 0];

/// The zones a place prompt leaves free (field_mask bits set = unavailable, relative to the choosing player: monster
/// zones 0-4, Spell/Trap 8-12, the opponent's 16-20 and 24-28), as (engine player, location, sequence), centre out.
pub(super) fn free_zones(m: &Json) -> Vec<(u64, u64, u64)> {
    let mask = m["field_mask"].as_u64().unwrap_or(0);
    let me = m["player"].as_u64().unwrap_or(0);
    [(me, loc::MZONE, 0), (me, loc::SZONE, 8), (1 - me, loc::MZONE, 16), (1 - me, loc::SZONE, 24)]
        .into_iter()
        .flat_map(|(p, l, base)| ZONE_ORDER.map(|s| (p, l, s, base + s)))
        .filter(|&(_, _, _, bit)| mask & 1 << bit == 0)
        .map(|(p, l, s, _)| (p, l, s))
        .collect()
}

pub(super) fn cpu_answer(kind: u64, m: &Json) -> Json {
    let list_len = |k: &str| m[k].as_array().map_or(0, Vec::len);
    let first = |min: &Json, max: &Json, len: usize| {
        let n = (min.as_u64().unwrap_or(1) as usize).max(1).min(max.as_u64().unwrap_or(1) as usize).min(len);
        (0..n).collect::<Vec<_>>()
    };
    match kind {
        msg::SELECT_IDLECMD => {
            let (action, index) = if list_len("summons") > 0 {
                (idle::SUMMON, json!(0))
            } else if list_len("monster_sets") > 0 {
                (idle::MONSTER_SET, json!(0))
            } else if m["to_bp"] == true {
                (idle::TO_BP, Json::Null)
            } else {
                (idle::TO_EP, Json::Null)
            };
            json!({ "type": resp::IDLECMD, "action": action, "index": index })
        }
        msg::SELECT_BATTLECMD if list_len("attacks") > 0 => json!({ "type": resp::BATTLECMD, "action": battle::ATTACK, "index": 0 }),
        msg::SELECT_BATTLECMD => json!({ "type": resp::BATTLECMD, "action": battle::TO_EP, "index": null }),
        msg::SELECT_CHAIN => {
            let index = if m["forced"].as_bool() == Some(true) { json!(0) } else { Json::Null };
            json!({ "type": resp::CHAIN, "index": index })
        }
        msg::SELECT_CARD => json!({ "type": resp::CARD, "indicies": first(&m["min"], &m["max"], list_len("selects")) }),
        msg::SELECT_TRIBUTE => json!({ "type": resp::TRIBUTE, "indicies": first(&m["min"], &m["max"], list_len("selects")) }),
        msg::SELECT_UNSELECT_CARD => {
            // Finishing with nothing picked cancels: Panther Warrior's attack (a Tribute to declare it) came back
            // again and again. Pick first.
            let finish = m["can_finish"] == true && m["unselect_cards"].as_array().is_some_and(|c| !c.is_empty());
            let index = if finish { Json::Null } else { json!(0) };
            json!({ "type": resp::UNSELECT_CARD, "index": index })
        }
        msg::SELECT_EFFECTYN => json!({ "type": resp::EFFECTYN, "yes": true }),
        msg::SELECT_YESNO => json!({ "type": resp::YESNO, "yes": true }),
        msg::SELECT_OPTION => json!({ "type": resp::OPTION, "index": 0 }),
        msg::SORT_CHAIN | msg::SORT_CARD => json!({ "type": resp::SORT, "order": null }),
        msg::SELECT_COUNTER => {
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
            json!({ "type": resp::COUNTER, "counters": counters })
        }
        // The free zones centre out, as Konami's server filled them: the player's monster zones, Spell/Trap zones, then
        // the opponent's (Give and Take and Black Garden place on the opponent's field), and the Field zone (13).
        msg::SELECT_PLACE | msg::SELECT_DISFIELD => {
            let mask = m["field_mask"].as_u64().unwrap_or(0);
            let me = m["player"].as_u64().unwrap_or(0);
            let field = (mask & 1 << 13 == 0).then_some((me, loc::SZONE, 5));
            // As many as asked (Ground Collapse and Phantom Dragon each disable two zones).
            let count = (m["count"].as_u64().unwrap_or(1) as usize).max(1);
            let mut places: Vec<Json> = free_zones(m)
                .into_iter()
                .chain(field)
                .take(count)
                .map(|(p, l, s)| json!({ "player": p, "location": l, "sequence": s }))
                .collect();
            if places.is_empty() {
                places.push(json!({ "player": me, "location": loc::MZONE, "sequence": 0 }));
            }
            let kind = if kind == msg::SELECT_PLACE { resp::PLACE } else { resp::DISFIELD };
            json!({ "type": kind, "places": places })
        }
        // Face-up attack when allowed, else the first allowed position.
        msg::SELECT_POSITION => {
            let allowed = m["positions"].as_u64().unwrap_or(1);
            let position = if allowed & 1 != 0 { 1 } else { 1 << allowed.trailing_zeros() };
            json!({ "type": resp::POSITION, "position": position })
        }
        // The lowest `count` of the available bits.
        msg::ANNOUNCE_RACE | msg::ANNOUNCE_ATTRIB => {
            let available = m["available"].as_u64().unwrap_or(1);
            let picked: Vec<u64> = (0..64).map(|b| 1u64 << b).filter(|b| available & b != 0).take(m["count"].as_u64().unwrap_or(1) as usize).collect();
            let (kind, key) = if kind == msg::ANNOUNCE_RACE { (resp::RACE, "races") } else { (resp::ATTRIB, "attributes") };
            json!({ "type": kind, key: picked })
        }
        msg::ANNOUNCE_NUMBER => json!({ "type": resp::NUMBER, "value": 0 }),
        msg::ANNOUNCE_CARD => json!({ "type": resp::CARD_NAME, "cid": m["fits"][0].as_u64().unwrap_or(0) }),
        msg::ROCK_PAPER_SCISSORS => json!({ "type": resp::RPS, "value": 1 }),
        msg::SELECT_SUM => json!({ "type": resp::SUM, "indicies": sum_choice(m) }),
        _ => {
            tracing::warn!("rules engine: no answer for prompt {kind}");
            json!({ "type": resp::CHAIN, "index": null })
        }
    }
}
