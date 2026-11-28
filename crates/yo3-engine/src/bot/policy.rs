//! The bot's rules of thumb: answers that need no look-ahead (positions, places, attacks, which cards to pay a cost
//! with), the answers it gives inside a look-ahead copy, and the placeholder CPU they fall back on (`simple`).
//! Every rule reads only what the bot's player may see ([`Board`] hides the opponent's hand and Set cards).
use serde_json::Value as Json;

use super::board::{Board, Monster};
use yo3_ocg::{Response, battle, hintmsg, idle, loc, msg, pos};

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
    matches!(hint, Some(hintmsg::RELEASE | hintmsg::DISCARD | hintmsg::DESTROY | hintmsg::REMOVE | hintmsg::TOGRAVE | hintmsg::RTOHAND
        | hintmsg::TODECK | hintmsg::XMATERIAL))
}

/// How much the bot wants card `c` (a selectable place) chosen, for selection hint `hint`, higher first.
fn pick_value(c: &Json, me: u8, board: &Board, hint: Option<u64>) -> i64 {
    let mine = u(&c["controller"]) as u8 == me;
    let worth = card_worth(c, board) as i64;
    match (hint_hurts(hint) || hint == Some(hintmsg::SET), mine) {
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
    if u(&c["location"]) == loc::MZONE {
        return board.sides[player].monster(u(&c["sequence"]) as u8).map_or(0, |m| m.worth());
    }
    if u(&c["location"]) == loc::SZONE {
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
        msg::SELECT_IDLECMD => {
            // Summon the strongest, else Set (a Flip monster first: its effect works only when Set and flipped, #80),
            // else battle (when anything can attack), else end.
            let flip = |c: &Json| board.is_flip(u(&c["code"]) as u32);
            let strongest = |k: &str, flips: bool| {
                list(m, k).iter().enumerate().filter(|(_, c)| flips || !flip(c))
                    .max_by_key(|(_, c)| (flip(c), board.base_worth(u(&c["code"]) as u32))).map(|(i, _)| i)
            };
            if let Some(i) = strongest("summons", false) {
                return Response::IdleCmd { action: idle::SUMMON, index: Some(i) }.json();
            }
            if let Some(i) = strongest("monster_sets", true) {
                return Response::IdleCmd { action: idle::MONSTER_SET, index: Some(i) }.json();
            }
            let action = if m["to_bp"] == true && !mine.monsters.is_empty() { idle::TO_BP } else { idle::TO_EP };
            Response::IdleCmd { action, index: None }.json()
        }
        msg::SELECT_BATTLECMD => battle(m, me, board),
        msg::SELECT_CHAIN => {
            Response::Chain((m["forced"] == true).then_some(0)).json()
        }
        msg::SELECT_CARD | msg::SELECT_TRIBUTE => {
            let selects = list(m, "selects");
            let (min, _) = min_max(m, selects.len());
            let picks = if hint == Some(hintmsg::ATTACKTARGET) {
                // The target an attacker of this ATK would choose (the attacker is the one just declared).
                let atk = board.attacker.unwrap_or(0);
                let targets: Vec<&Monster> = selects.iter().filter_map(|c| opponent.monster(u(&c["sequence"]) as u8)).collect();
                let best = attack_target(atk, &targets.iter().map(|m| (*m).clone()).collect::<Vec<_>>(), board.style.aggression).map(|t| t.seq);
                vec![selects.iter().position(|c| Some(u(&c["sequence"]) as u8) == best).unwrap_or(0)]
            } else {
                best_picks(selects, min, me, board, hint)
            };
            if kind == msg::SELECT_CARD { Response::Card(Some(picks)).json() } else { Response::Tribute(Some(picks)).json() }
        }
        msg::SELECT_UNSELECT_CARD => {
            // Finish once something is picked; until then pick (finishing with nothing picked cancels: an attack
            // declared again and again).
            if m["can_finish"] == true && !list(m, "unselect_cards").is_empty() {
                return Response::UnselectCard(None).json();
            }
            let selects = list(m, "select_cards");
            let pick = best_picks(selects, 1, me, board, hint).first().copied().unwrap_or(0);
            Response::UnselectCard(Some(pick)).json()
        }
        msg::SELECT_EFFECTYN => Response::EffectYn(true).json(),
        msg::SELECT_YESNO => Response::YesNo(true).json(),
        msg::SELECT_OPTION => Response::Option(0).json(),
        msg::SELECT_POSITION => {
            let allowed = u(&m["positions"]);
            let atk = board.base_atk(u(&m["code"]) as u32);
            let def = board.base_def(u(&m["code"]) as u32);
            let threat = opponent.monsters.iter().filter(|o| o.faceup && o.attack_pos).map(|o| o.atk).max().unwrap_or(0);
            let attack = atk >= def || atk > threat;
            let order: [u64; 4] = if attack {
                [pos::FACEUP_ATTACK, pos::FACEUP_DEFENSE, pos::FACEDOWN_DEFENSE, pos::FACEDOWN_ATTACK]
            } else {
                [pos::FACEUP_DEFENSE, pos::FACEDOWN_DEFENSE, pos::FACEUP_ATTACK, pos::FACEDOWN_ATTACK]
            };
            let position = order.into_iter().find(|p| allowed & p != 0).unwrap_or(1 << allowed.trailing_zeros().min(3));
            Response::Position(position).json()
        }
        _ => simple(m),
    }
}

/// The Battle Phase: attack where it pays (directly, or a monster an attacker destroys), else Main Phase 2 to Set
/// what was drawn, else end. Into Set Spells and Traps it doesn't know, the weakest attacker that can do something goes
/// first, to spring them on the monster it would miss least; otherwise the strongest first, each taking the best target
/// it beats (style `bait` false: always the strongest first).
fn battle(m: &Json, me: u8, board: &Board) -> Json {
    let opponent = &board.sides[1 - me as usize];
    let attacks = list(m, "attacks");
    let atk_of = |a: &Json| board.sides[me as usize].monster(u(&a["sequence"]) as u8).map_or(0, |m| m.atk);
    let pays = |a: &Json| {
        opponent.monsters.is_empty() && a["can_direct"] == true || attack_target(atk_of(a), &opponent.monsters, board.style.aggression).is_some()
    };
    let backrow = opponent.spells.iter().any(|s| !s.faceup && s.kind.is_none());
    let mut order: Vec<usize> = (0..attacks.len()).filter(|&i| pays(&attacks[i])).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(atk_of(&attacks[i])));
    // Lethal now: no bait, everything goes in.
    let direct: i32 = order.iter().filter(|&&i| attacks[i]["can_direct"] == true && opponent.monsters.is_empty()).map(|&i| atk_of(&attacks[i])).sum();
    if board.style.bait && backrow && direct < opponent.lp {
        order.reverse();
    }
    if let Some(&i) = order.first() {
        return Response::BattleCmd { action: battle::ATTACK, index: Some(i) }.json();
    }
    let action = if m["to_m2"] == true { battle::TO_M2 } else { battle::TO_EP };
    Response::BattleCmd { action, index: None }.json()
}

/// The ATK of the attacker the bot declares with `answer` (to choose its target next).
pub fn declared_attacker(m: &Json, answer: &Json, me: u8, board: &Board) -> Option<i32> {
    if u(&m["type"]) != msg::SELECT_BATTLECMD || u(&answer["action"]) != battle::ATTACK {
        return None;
    }
    let a = list(m, "attacks").get(u(&answer["index"]) as usize)?;
    board.sides[me as usize].monster(u(&a["sequence"]) as u8).map(|m| m.atk)
}

/// The bot's own answer inside a look-ahead copy: as [`answer`], except at an idle prompt, where it plays toward its
/// deck's bosses ([`Board::knowledge`]): it Special Summons a boss when it can, and after summoning activates what it
/// can, a piece that brings a boss out first, else the first card offered, so that a Spell held or drawn counts for
/// what it does in the copy. With `plain_copies` in its style, only the first card offered (the bot before #85's work).
pub fn in_copy(m: &Json, me: u8, board: &Board, hint: Option<u64>) -> Json {
    if u(&m["type"]) == msg::SELECT_IDLECMD {
        let k = &board.knowledge;
        let code = |c: &Json| u(&c["code"]) as u32;
        if !board.style.plain_copies
            && let Some(i) = list(m, "special_summons").iter().position(|c| k.is_boss(code(c)))
        {
            return Response::IdleCmd { action: idle::SPSUMMON, index: Some(i) }.json();
        }
        let activates = list(m, "activates");
        if list(m, "summons").is_empty() && !activates.is_empty() {
            let piece = activates.iter().position(|c| k.is_piece(code(c)) || k.is_boss(code(c)));
            let i = if board.style.plain_copies { 0 } else { piece.unwrap_or(0) };
            return Response::IdleCmd { action: idle::ACTIVATE, index: Some(i) }.json();
        }
    }
    answer(m, me, board, hint)
}

/// An answer that moves the duel on, for a prompt that came straight back after the bot's answer: leave the phase
/// (battle: Main Phase 2 or the End Phase; main: the Battle or End Phase), else the placeholder's first choices.
pub fn move_on(m: &Json) -> Json {
    match u(&m["type"]) {
        msg::SELECT_BATTLECMD => {
            Response::BattleCmd { action: if m["to_m2"] == true { battle::TO_M2 } else { battle::TO_EP }, index: None }.json()
        }
        msg::SELECT_IDLECMD => {
            Response::IdleCmd { action: if m["to_bp"] == true { idle::TO_BP } else { idle::TO_EP }, index: None }.json()
        }
        _ => simple(m),
    }
}

/// The opponent inside a look-ahead copy: it never responds (no chains, no optional effects), and answers what it
/// must with its first choices, so the bot can't learn its hidden cards from how it plays.
pub fn passive(m: &Json) -> Json {
    match u(&m["type"]) {
        msg::SELECT_CHAIN => Response::Chain((m["forced"] == true).then_some(0)).json(),
        msg::SELECT_EFFECTYN => Response::EffectYn(false).json(),
        msg::SELECT_YESNO => Response::YesNo(false).json(),
        _ => simple(m),
    }
}

/// The placeholder CPU (`yo3_ocg::placeholder`, the server's too): the first legal choices. The rules fall back on it
/// for the prompts they don't cover; alone it is the baseline the bot is measured against.
pub fn simple(m: &Json) -> Json {
    yo3_ocg::placeholder::answer(m).json()
}
