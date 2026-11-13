//! The bot's rules of thumb: answers that need no look-ahead (positions, places, attacks, which cards to pay a cost
//! with), the answers it gives inside a look-ahead copy, and the old placeholder CPU (`simple`), kept as a baseline.
//! Every rule reads only what the bot's player may see ([`Board`] hides the opponent's hand and Set cards).
use serde_json::{Value as Json, json};

use super::board::{Board, Monster};

// Prompts (OcgMessageType) and answers (OcgResponseType).
pub const MSG_SELECT_BATTLECMD: u64 = 10;
pub const MSG_SELECT_IDLECMD: u64 = 11;
pub const MSG_SELECT_EFFECTYN: u64 = 12;
pub const MSG_SELECT_YESNO: u64 = 13;
pub const MSG_SELECT_OPTION: u64 = 14;
pub const MSG_SELECT_CARD: u64 = 15;
pub const MSG_SELECT_CHAIN: u64 = 16;
pub const MSG_SELECT_PLACE: u64 = 18;
pub const MSG_SELECT_POSITION: u64 = 19;
pub const MSG_SELECT_TRIBUTE: u64 = 20;
pub const MSG_SORT_CHAIN: u64 = 21;
pub const MSG_SELECT_COUNTER: u64 = 22;
pub const MSG_SELECT_SUM: u64 = 23;
pub const MSG_SELECT_DISFIELD: u64 = 24;
pub const MSG_SORT_CARD: u64 = 25;
pub const MSG_SELECT_UNSELECT_CARD: u64 = 26;
pub const MSG_ROCK_PAPER_SCISSORS: u64 = 132;
pub const MSG_ANNOUNCE_RACE: u64 = 140;
pub const MSG_ANNOUNCE_ATTRIB: u64 = 141;
pub const MSG_ANNOUNCE_CARD: u64 = 142;
pub const MSG_ANNOUNCE_NUMBER: u64 = 143;

pub const RESP_BATTLECMD: u64 = 0;
pub const RESP_IDLECMD: u64 = 1;
pub const RESP_EFFECTYN: u64 = 2;
pub const RESP_YESNO: u64 = 3;
pub const RESP_OPTION: u64 = 4;
pub const RESP_CARD: u64 = 5;
pub const RESP_UNSELECT_CARD: u64 = 7;
pub const RESP_CHAIN: u64 = 8;
pub const RESP_DISFIELD: u64 = 9;
pub const RESP_PLACE: u64 = 10;
pub const RESP_POSITION: u64 = 11;
pub const RESP_TRIBUTE: u64 = 12;
pub const RESP_COUNTER: u64 = 13;
pub const RESP_SUM: u64 = 14;
pub const RESP_SORT: u64 = 15;
pub const RESP_RACE: u64 = 16;
pub const RESP_ATTRIB: u64 = 17;
pub const RESP_CARD_NAME: u64 = 18;
pub const RESP_NUMBER: u64 = 19;
pub const RESP_RPS: u64 = 20;

pub const IDLE_SUMMON: u64 = 0;
pub const IDLE_SPSUMMON: u64 = 1;
pub const IDLE_REPOS: u64 = 2;
pub const IDLE_MONSTER_SET: u64 = 3;
pub const IDLE_SPELL_SET: u64 = 4;
pub const IDLE_ACTIVATE: u64 = 5;
pub const IDLE_TO_BP: u64 = 6;
pub const IDLE_TO_EP: u64 = 7;
pub const BATTLE_ATTACK: u64 = 1;
pub const BATTLE_TO_M2: u64 = 2;
pub const BATTLE_TO_EP: u64 = 3;

const LOC_MZONE: u64 = 0x04;
const LOC_SZONE: u64 = 0x08;

pub const POS_FACEUP_ATTACK: u64 = 0x1;
pub const POS_FACEDOWN_ATTACK: u64 = 0x2;
pub const POS_FACEUP_DEFENSE: u64 = 0x4;
pub const POS_FACEDOWN_DEFENSE: u64 = 0x8;

// The selection hints that precede a card choice (HINT_SELECTMSG, EDOPro's HINTMSG_*): what will happen to the cards.
const HINTMSG_RELEASE: u64 = 500;
const HINTMSG_DISCARD: u64 = 501;
const HINTMSG_DESTROY: u64 = 502;
const HINTMSG_REMOVE: u64 = 503;
const HINTMSG_TOGRAVE: u64 = 504;
const HINTMSG_RTOHAND: u64 = 505;
const HINTMSG_TODECK: u64 = 507;
const HINTMSG_SET: u64 = 510;
const HINTMSG_XMATERIAL: u64 = 532;
pub const HINTMSG_ATTACKTARGET: u64 = 549;

/// What a monster you face can do in a battle: its ATK in Attack Position, its DEF in Defense Position, and a guess
/// for a face-down one (the bot can't see it).
pub const FACEDOWN_GUESS: i32 = 1500;

/// A face-down monster is attacked by a monster at least this strong.
const FACEDOWN_ATTACKER: i32 = 1900;

fn list<'a>(m: &'a Json, k: &str) -> &'a [Json] {
    m[k].as_array().map_or(&[][..], Vec::as_slice)
}

fn u(v: &Json) -> u64 {
    v.as_u64().unwrap_or(0)
}

/// Whether a selection hint means the chosen cards lose out (destroyed, released, sent away...): then the bot picks
/// the opponent's best cards, or its own least useful ones.
fn hint_hurts(hint: Option<u64>) -> bool {
    matches!(hint, Some(HINTMSG_RELEASE | HINTMSG_DISCARD | HINTMSG_DESTROY | HINTMSG_REMOVE | HINTMSG_TOGRAVE | HINTMSG_RTOHAND
        | HINTMSG_TODECK | HINTMSG_XMATERIAL))
}

/// How much the bot wants card `c` (a selectable place) chosen, for selection hint `hint`, higher first.
fn pick_value(c: &Json, me: u8, board: &Board, hint: Option<u64>) -> i64 {
    let mine = u(&c["controller"]) as u8 == me;
    let worth = card_worth(c, board) as i64;
    match (hint_hurts(hint) || hint == Some(HINTMSG_SET), mine) {
        // Losing a card: the opponent's best, else our least useful.
        (true, false) => 100_000 + worth,
        (true, true) => -worth,
        // Gaining or using a card (to the hand, summoned, equipped, targeted for good): our best, else theirs.
        (false, true) => 100_000 + worth,
        (false, false) => worth,
    }
}

/// A card's worth as the bot sees it: a monster on the field by its stats, other places by the card's data.
fn card_worth(c: &Json, board: &Board) -> i32 {
    let player = u(&c["controller"]) as usize & 1;
    if u(&c["location"]) == LOC_MZONE {
        return board.sides[player].monster(u(&c["sequence"]) as u8).map_or(0, |m| m.worth());
    }
    if u(&c["location"]) == LOC_SZONE {
        return 800;
    }
    board.base_worth(u(&c["code"]) as u32)
}

/// `n` picks among `selects` for hint `hint`: the cards the bot most wants chosen.
fn best_picks(selects: &[Json], n: usize, me: u8, board: &Board, hint: Option<u64>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..selects.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(pick_value(&selects[i], me, board, hint)));
    order.truncate(n);
    order
}

/// MSG_SELECT_CARD's `selects` in the order the rules want them, for hint `hint`.
pub fn ranked(m: &Json, me: u8, board: &Board, hint: Option<u64>) -> Vec<usize> {
    let selects = list(m, "selects");
    best_picks(selects, selects.len(), me, board, hint)
}

fn min_max(m: &Json, len: usize) -> (usize, usize) {
    let min = (u(&m["min"]) as usize).max(1).min(len);
    let max = (u(&m["max"]) as usize).max(min).min(len);
    (min, max)
}

/// The monster at the opponent's side the bot would attack with `atk`: the most valuable one it destroys, never a
/// face-down one with a small attacker. An aggressive style (`aggression` above 0) attacks face-down monsters with
/// smaller attackers, and from 0.5 also trades its monster for one of the same ATK.
pub fn attack_target(atk: i32, opponent: &[Monster], aggression: f64) -> Option<&Monster> {
    let facedown = FACEDOWN_ATTACKER - (400.0 * aggression.max(0.0)) as i32;
    let trade = aggression >= 0.5;
    opponent
        .iter()
        .filter(|t| if !t.known { atk >= facedown } else if t.attack_pos { atk > t.atk || trade && atk == t.atk } else { atk > t.def })
        .max_by_key(|t| t.worth())
}

/// The bot's answer to a prompt without looking ahead. `me` is the bot's engine player, `board` what it sees,
/// `hint` the last selection hint it was given.
pub fn answer(m: &Json, me: u8, board: &Board, hint: Option<u64>) -> Json {
    let kind = u(&m["type"]);
    let opponent = &board.sides[1 - me as usize];
    let mine = &board.sides[me as usize];
    match kind {
        MSG_SELECT_IDLECMD => {
            // Summon the strongest, else Set, else battle (when anything can attack), else end.
            let strongest = |k: &str| {
                list(m, k).iter().enumerate().max_by_key(|(_, c)| board.base_worth(u(&c["code"]) as u32)).map(|(i, _)| i)
            };
            if let Some(i) = strongest("summons") {
                return json!({ "type": RESP_IDLECMD, "action": IDLE_SUMMON, "index": i });
            }
            if let Some(i) = strongest("monster_sets") {
                return json!({ "type": RESP_IDLECMD, "action": IDLE_MONSTER_SET, "index": i });
            }
            let action = if m["to_bp"] == true && !mine.monsters.is_empty() { IDLE_TO_BP } else { IDLE_TO_EP };
            json!({ "type": RESP_IDLECMD, "action": action, "index": null })
        }
        MSG_SELECT_BATTLECMD => battle(m, me, board),
        MSG_SELECT_CHAIN => {
            let index = if m["forced"] == true { json!(0) } else { Json::Null };
            json!({ "type": RESP_CHAIN, "index": index })
        }
        MSG_SELECT_CARD | MSG_SELECT_TRIBUTE => {
            let selects = list(m, "selects");
            let (min, _) = min_max(m, selects.len());
            let picks = if hint == Some(HINTMSG_ATTACKTARGET) {
                // The target an attacker of this ATK would choose (the attacker is the one just declared).
                let atk = board.attacker.unwrap_or(0);
                let targets: Vec<&Monster> = selects.iter().filter_map(|c| opponent.monster(u(&c["sequence"]) as u8)).collect();
                let best = attack_target(atk, &targets.iter().map(|m| (*m).clone()).collect::<Vec<_>>(), board.style.aggression).map(|t| t.seq);
                vec![selects.iter().position(|c| Some(u(&c["sequence"]) as u8) == best).unwrap_or(0)]
            } else {
                best_picks(selects, min, me, board, hint)
            };
            let kind = if kind == MSG_SELECT_CARD { RESP_CARD } else { RESP_TRIBUTE };
            json!({ "type": kind, "indicies": picks })
        }
        MSG_SELECT_UNSELECT_CARD => {
            // Finish once something is picked; until then pick (finishing with nothing picked cancels: an attack
            // declared again and again).
            if m["can_finish"] == true && !list(m, "unselect_cards").is_empty() {
                return json!({ "type": RESP_UNSELECT_CARD, "index": null });
            }
            let selects = list(m, "select_cards");
            let pick = best_picks(selects, 1, me, board, hint).first().copied().unwrap_or(0);
            json!({ "type": RESP_UNSELECT_CARD, "index": pick })
        }
        MSG_SELECT_EFFECTYN => json!({ "type": RESP_EFFECTYN, "yes": true }),
        MSG_SELECT_YESNO => json!({ "type": RESP_YESNO, "yes": true }),
        MSG_SELECT_OPTION => json!({ "type": RESP_OPTION, "index": 0 }),
        MSG_SELECT_POSITION => {
            let allowed = u(&m["positions"]);
            let atk = board.base_atk(u(&m["code"]) as u32);
            let def = board.base_def(u(&m["code"]) as u32);
            let threat = opponent.monsters.iter().filter(|o| o.faceup && o.attack_pos).map(|o| o.atk).max().unwrap_or(0);
            let attack = atk >= def || atk > threat;
            let order: [u64; 4] = if attack {
                [POS_FACEUP_ATTACK, POS_FACEUP_DEFENSE, POS_FACEDOWN_DEFENSE, POS_FACEDOWN_ATTACK]
            } else {
                [POS_FACEUP_DEFENSE, POS_FACEDOWN_DEFENSE, POS_FACEUP_ATTACK, POS_FACEDOWN_ATTACK]
            };
            let position = order.into_iter().find(|p| allowed & p != 0).unwrap_or(1 << allowed.trailing_zeros().min(3));
            json!({ "type": RESP_POSITION, "position": position })
        }
        _ => simple(m),
    }
}

/// The Battle Phase: attack where it pays (directly, or a monster an attacker destroys), else Main Phase 2 to Set
/// what was drawn, else end.
fn battle(m: &Json, me: u8, board: &Board) -> Json {
    let opponent = &board.sides[1 - me as usize];
    let attacks = list(m, "attacks");
    let atk_of = |a: &Json| board.sides[me as usize].monster(u(&a["sequence"]) as u8).map_or(0, |m| m.atk);
    // Strongest attackers first: each takes the best target it beats.
    let mut order: Vec<usize> = (0..attacks.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(atk_of(&attacks[i])));
    for i in order {
        let a = &attacks[i];
        let atk = atk_of(a);
        if opponent.monsters.is_empty() && a["can_direct"] == true || attack_target(atk, &opponent.monsters, board.style.aggression).is_some() {
            return json!({ "type": RESP_BATTLECMD, "action": BATTLE_ATTACK, "index": i });
        }
    }
    let action = if m["to_m2"] == true { BATTLE_TO_M2 } else { BATTLE_TO_EP };
    json!({ "type": RESP_BATTLECMD, "action": action, "index": null })
}

/// The ATK of the attacker the bot declares with `answer` (to choose its target next).
pub fn declared_attacker(m: &Json, answer: &Json, me: u8, board: &Board) -> Option<i32> {
    if u(&m["type"]) != MSG_SELECT_BATTLECMD || u(&answer["action"]) != BATTLE_ATTACK {
        return None;
    }
    let a = list(m, "attacks").get(u(&answer["index"]) as usize)?;
    board.sides[me as usize].monster(u(&a["sequence"]) as u8).map(|m| m.atk)
}

/// The bot's own answer inside a look-ahead copy: as [`answer`], except that at an idle prompt, after summoning, it
/// activates what it can (the first card offered), so that a Spell held or drawn counts for what it does in the copy.
pub fn in_copy(m: &Json, me: u8, board: &Board, hint: Option<u64>) -> Json {
    if u(&m["type"]) == MSG_SELECT_IDLECMD && list(m, "summons").is_empty() && !list(m, "activates").is_empty() {
        return json!({ "type": RESP_IDLECMD, "action": IDLE_ACTIVATE, "index": 0 });
    }
    answer(m, me, board, hint)
}

/// An answer that moves the duel on, for a prompt that came straight back after the bot's answer: leave the phase
/// (battle: Main Phase 2 or the End Phase; main: the Battle or End Phase), else the placeholder's first choices.
pub fn move_on(m: &Json) -> Json {
    match u(&m["type"]) {
        MSG_SELECT_BATTLECMD => {
            json!({ "type": RESP_BATTLECMD, "action": if m["to_m2"] == true { BATTLE_TO_M2 } else { BATTLE_TO_EP }, "index": null })
        }
        MSG_SELECT_IDLECMD => {
            json!({ "type": RESP_IDLECMD, "action": if m["to_bp"] == true { IDLE_TO_BP } else { IDLE_TO_EP }, "index": null })
        }
        _ => simple(m),
    }
}

/// The opponent inside a look-ahead copy: it never responds (no chains, no optional effects), and answers what it
/// must with its first choices, so the bot can't learn its hidden cards from how it plays.
pub fn passive(m: &Json) -> Json {
    match u(&m["type"]) {
        MSG_SELECT_CHAIN => json!({ "type": RESP_CHAIN, "index": if m["forced"] == true { json!(0) } else { Json::Null } }),
        MSG_SELECT_EFFECTYN => json!({ "type": RESP_EFFECTYN, "yes": false }),
        MSG_SELECT_YESNO => json!({ "type": RESP_YESNO, "yes": false }),
        _ => simple(m),
    }
}

/// The old placeholder CPU: the first legal choices (summon the first monster, attack with the first attacker, never
/// chain). Kept as the baseline the bot is measured against.
pub fn simple(m: &Json) -> Json {
    let list_len = |k: &str| list(m, k).len();
    let first = |len: usize| -> Vec<usize> { (0..min_max(m, len).0).collect() };
    match u(&m["type"]) {
        MSG_SELECT_IDLECMD => {
            let (action, index) = if list_len("summons") > 0 {
                (IDLE_SUMMON, json!(0))
            } else if list_len("monster_sets") > 0 {
                (IDLE_MONSTER_SET, json!(0))
            } else if m["to_bp"] == true {
                (IDLE_TO_BP, Json::Null)
            } else {
                (IDLE_TO_EP, Json::Null)
            };
            json!({ "type": RESP_IDLECMD, "action": action, "index": index })
        }
        MSG_SELECT_BATTLECMD if list_len("attacks") > 0 => json!({ "type": RESP_BATTLECMD, "action": BATTLE_ATTACK, "index": 0 }),
        MSG_SELECT_BATTLECMD => json!({ "type": RESP_BATTLECMD, "action": BATTLE_TO_EP, "index": null }),
        MSG_SELECT_CHAIN => json!({ "type": RESP_CHAIN, "index": if m["forced"] == true { json!(0) } else { Json::Null } }),
        MSG_SELECT_CARD => json!({ "type": RESP_CARD, "indicies": first(list_len("selects")) }),
        MSG_SELECT_TRIBUTE => json!({ "type": RESP_TRIBUTE, "indicies": first(list_len("selects")) }),
        // Finishing with nothing picked cancels (an attack that needs a cost, declared again and again): pick first.
        MSG_SELECT_UNSELECT_CARD => {
            let finish = m["can_finish"] == true && !list(m, "unselect_cards").is_empty();
            json!({ "type": RESP_UNSELECT_CARD, "index": if finish { Json::Null } else { json!(0) } })
        }
        MSG_SELECT_EFFECTYN => json!({ "type": RESP_EFFECTYN, "yes": true }),
        MSG_SELECT_YESNO => json!({ "type": RESP_YESNO, "yes": true }),
        MSG_SELECT_OPTION => json!({ "type": RESP_OPTION, "index": 0 }),
        MSG_SORT_CHAIN | MSG_SORT_CARD => json!({ "type": RESP_SORT, "order": null }),
        MSG_SELECT_COUNTER => {
            let mut left = u(&m["count"]);
            let mut counters: Vec<u64> = list(m, "cards").iter().map(|c| {
                let take = left.min(u(&c["count"]));
                left -= take;
                take
            }).collect();
            if counters.is_empty() {
                counters.push(left);
            }
            json!({ "type": RESP_COUNTER, "counters": counters })
        }
        kind @ (MSG_SELECT_PLACE | MSG_SELECT_DISFIELD) => {
            // The first zone the mask leaves free: the player's monster zones (bits 0-4), Spell/Trap zones (8-12, the
            // Field zone 13), then the opponent's (16-20, 24-29).
            let mask = u(&m["field_mask"]);
            let me = u(&m["player"]);
            let free = [(me, LOC_MZONE, 0..5u64, 0u64), (me, LOC_SZONE, 0..6, 8), (1 - me, LOC_MZONE, 0..5, 16), (1 - me, LOC_SZONE, 0..6, 24)]
                .into_iter()
                .flat_map(|(p, l, seqs, base)| seqs.map(move |s| (p, l, s, base + s)));
            let count = (u(&m["count"]) as usize).max(1);
            let mut places: Vec<Json> = free.filter(|&(_, _, _, bit)| mask & 1 << bit == 0).take(count)
                .map(|(p, l, s, _)| json!({ "player": p, "location": l, "sequence": s })).collect();
            if places.is_empty() {
                places.push(json!({ "player": me, "location": LOC_MZONE, "sequence": 0 }));
            }
            json!({ "type": if kind == MSG_SELECT_PLACE { RESP_PLACE } else { RESP_DISFIELD }, "places": places })
        }
        MSG_SELECT_POSITION => {
            let allowed = u(&m["positions"]).max(1);
            json!({ "type": RESP_POSITION, "position": if allowed & 1 != 0 { 1 } else { 1 << allowed.trailing_zeros() } })
        }
        kind @ (MSG_ANNOUNCE_RACE | MSG_ANNOUNCE_ATTRIB) => {
            let available = u(&m["available"]).max(1);
            let picked: Vec<u64> = (0..64).map(|b| 1u64 << b).filter(|b| available & b != 0).take(u(&m["count"]).max(1) as usize).collect();
            let (kind, key) = if kind == MSG_ANNOUNCE_RACE { (RESP_RACE, "races") } else { (RESP_ATTRIB, "attributes") };
            json!({ "type": kind, key: picked })
        }
        MSG_ANNOUNCE_NUMBER => json!({ "type": RESP_NUMBER, "value": 0 }),
        MSG_ANNOUNCE_CARD => json!({ "type": RESP_CARD_NAME, "cid": m["fits"][0].as_u64().unwrap_or(0) }),
        MSG_ROCK_PAPER_SCISSORS => json!({ "type": RESP_RPS, "value": 1 }),
        MSG_SELECT_SUM => json!({ "type": RESP_SUM, "indicies": sum_choice(m) }),
        _ => json!({ "type": RESP_CHAIN, "index": null }),
    }
}

/// MSG_SELECT_SUM: the fewest of `selects` that make the sum with the `selects_must` (see the server's translate.rs,
/// which offers the same choice to the player).
pub fn sum_choice(m: &Json) -> Vec<usize> {
    let amount = |c: &Json| u(&c["amount"]);
    let selects: Vec<u64> = list(m, "selects").iter().map(amount).collect();
    let must: Vec<u64> = list(m, "selects_must").iter().map(amount).collect();
    let target = u(&m["amount"]);
    let greater = u(&m["select_max"]) != 0;
    let min = u(&m["min"]) as usize;
    let max = m["max"].as_u64().filter(|&x| x != 0).unwrap_or(u32::MAX as u64) as usize;
    let values = |a: u64| -> Vec<u64> { [a & 0xFFFF, a >> 16].into_iter().filter(|v| *v > 0).collect() };
    let reached = |chosen: &[u64]| {
        let mut sums = vec![0u64];
        for a in must.iter().chain(chosen) {
            let vs = values(*a);
            sums = sums.iter().flat_map(|s| if vs.is_empty() { vec![*s] } else { vs.iter().map(|v| s + v).collect() }).collect();
            sums.sort_unstable();
            sums.dedup();
        }
        if !greater {
            return sums.contains(&target);
        }
        let smallest = chosen.iter().map(|a| a & 0xFFFF).min().unwrap_or(0);
        sums.iter().any(|s| *s >= target && s.saturating_sub(smallest) < target)
    };
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
            if reached(&chosen) {
                return (0..n).filter(|i| mask >> i & 1 == 1).collect();
            }
        }
    }
    (0..n.min(max)).collect()
}
