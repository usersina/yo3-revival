//! The duel bot: how an NPC plays. At a decision that matters (what to summon, activate or Set, whether to chain,
//! which card to pick, yes or no) it tries each of its options in a copy of the duel, plays the action out, and keeps
//! the option that leaves it best placed ([`Board::value`]); everything else follows rules of thumb (`policy`).
//!
//! The core can't copy a duel, so a copy is a new duel with the same seed and decks, given every answer since the
//! start ([`Setup`], the history): it plays out exactly the same. Each option gets its own copy, on a thread of its
//! own; the choice doesn't depend on timing, so a recorded duel replays the bot's answers exactly.
//!
//! The bot plays fair. In a copy its opponent never responds (no chains, no optional effects), so it can't learn the
//! opponent's hand from how the copy plays out; it values the position only by what it may see ([`board`]); and it
//! decides its attacks, where the copy would show a face-down monster's DEF, by rules alone.
use std::sync::Arc;

use serde_json::{Value as Json, json};

use crate::cards::Library;
use crate::core::{Duel, Host, Status};
use crate::decode;

pub mod board;
pub mod policy;

use board::{Board, Hidden};
use policy::*;

const MODE_MR2: u64 = 0xD0600;
const POSITION_FACEDOWN_DEFENSE: u32 = 0x8;
const MSG_HINT: u64 = 2;
const MSG_WIN: u64 = 5;
const MSG_NEW_TURN: u64 = 40;
const MSG_NEW_PHASE: u64 = 41;
const HINT_SELECTMSG: u64 = 3;

/// An action only beats doing nothing by at least this much (in LP), so the bot doesn't spend cards for nothing.
const MARGIN: f64 = 30.0;
/// At most this many options are tried at one decision.
const MAX_OPTIONS: usize = 16;
/// A copy plays at most this many answers past the option before it is judged.
const MAX_STEPS: usize = 300;

/// How a duel began: what `new` gave the core, in order.
#[derive(Clone, Default)]
pub struct Setup {
    pub seed: [u64; 4],
    pub lp: u32,
    /// (team, location, passcode), in the order the cards were made.
    pub cards: Vec<(u8, u32, u32)>,
}

/// A copy's host: card data and scripts, no log.
struct Quiet(Arc<Library>);

impl Host for Quiet {
    fn card(&mut self, code: u32) -> Option<crate::core::Card> {
        self.0.card(code)
    }
    fn script(&mut self, name: &str) -> Option<Arc<Vec<u8>>> {
        self.0.script(name)
    }
    fn log(&mut self, _: i32, _: &str) {}
}

/// The decision at hand and what is needed to copy the duel.
pub struct Thinker<'a> {
    pub library: &'a Arc<Library>,
    pub setup: &'a Setup,
    /// Every answer given so far, as the core read it.
    pub history: &'a [Vec<u8>],
}

/// Runs a duel until it awaits an answer or ends; its messages (decoded when `decode`).
fn settle(duel: &mut Duel, decode: bool) -> (Status, Vec<Json>) {
    let mut out = vec![];
    loop {
        let status = duel.process();
        let bytes = duel.messages();
        if decode {
            out.extend(decode::messages(&bytes).unwrap_or_default());
        }
        if status != Status::Continue {
            return (status, out);
        }
    }
}

/// The options worth trying at prompt `m`, in the order the bot prefers them when they come out even; None when
/// rules decide. For an idle prompt the last option is doing nothing more (battle, or end the turn), judged as the
/// position is now.
fn options(m: &Json, board: &Board, hint: Option<u64>) -> Option<Vec<Json>> {
    let len = |k: &str| m[k].as_array().map_or(0, Vec::len);
    let kind = m["type"].as_u64().unwrap_or(0);
    let mut out = match kind {
        MSG_SELECT_IDLECMD => {
            let mut out = vec![];
            for (key, action) in [("summons", IDLE_SUMMON), ("special_summons", IDLE_SPSUMMON), ("activates", IDLE_ACTIVATE),
                ("monster_sets", IDLE_MONSTER_SET), ("spell_sets", IDLE_SPELL_SET), ("pos_changes", IDLE_REPOS)]
            {
                out.extend((0..len(key)).map(|i| json!({ "type": RESP_IDLECMD, "action": action, "index": i })));
            }
            out
        }
        MSG_SELECT_CHAIN if len("selects") > 0 && m["forced"] != true => {
            std::iter::once(json!({ "type": RESP_CHAIN, "index": null }))
                .chain((0..len("selects")).map(|i| json!({ "type": RESP_CHAIN, "index": i })))
                .collect()
        }
        MSG_SELECT_EFFECTYN => vec![json!({ "type": RESP_EFFECTYN, "yes": true }), json!({ "type": RESP_EFFECTYN, "yes": false })],
        MSG_SELECT_YESNO => vec![json!({ "type": RESP_YESNO, "yes": true }), json!({ "type": RESP_YESNO, "yes": false })],
        MSG_SELECT_OPTION if len("options") > 1 => (0..len("options")).map(|i| json!({ "type": RESP_OPTION, "index": i })).collect(),
        // One card out of a few, unless it is an attack target (the copy would show a face-down monster's DEF).
        MSG_SELECT_CARD if m["max"] == 1 && m["min"] == 1 && (2..=MAX_OPTIONS).contains(&len("selects")) && hint != Some(HINTMSG_ATTACKTARGET) => {
            // The rules' favourite first, so it wins a tie.
            let favourite = policy::answer(m, board.me, board, hint)["indicies"][0].as_u64().unwrap_or(0) as usize;
            std::iter::once(favourite).chain((0..len("selects")).filter(|i| *i != favourite))
                .map(|i| json!({ "type": RESP_CARD, "indicies": [i] })).collect()
        }
        _ => return None,
    };
    out.truncate(MAX_OPTIONS);
    if kind == MSG_SELECT_IDLECMD {
        if out.is_empty() {
            return None;
        }
        out.push(policy::answer(&json!({ "type": MSG_SELECT_IDLECMD, "to_bp": m["to_bp"] }), board.me, board, hint));
    }
    (out.len() > 1).then_some(out)
}

impl Thinker<'_> {
    /// The duel as it is now, rebuilt from its start: None if the core refuses it.
    fn copy(&self) -> Option<Duel> {
        let s = self.setup;
        let mut duel = Duel::new(s.seed, MODE_MR2, s.lp, 5, Box::new(Quiet(self.library.clone())))?;
        for name in ["constant.lua", "utility.lua"] {
            let text = self.library.script(name).unwrap_or_default();
            duel.load_script(name, &text);
        }
        for &(team, location, code) in &s.cards {
            duel.new_card(team, 0, code, team, location, 0, POSITION_FACEDOWN_DEFENSE);
        }
        duel.start();
        settle(&mut duel, false);
        for answer in self.history {
            duel.respond(answer);
            settle(&mut duel, false);
        }
        Some(duel)
    }

    /// How the position looks to `me` after answering with `option` in a copy and playing the action out: until an
    /// idle or battle prompt (anyone's), a new phase or turn, or the end of the duel. The bot answers its own prompts
    /// by its rules, the opponent never responds.
    fn try_option(&self, option: &Json, me: u8, hidden: &Hidden) -> f64 {
        let Some(mut duel) = self.copy() else { return f64::NEG_INFINITY };
        duel.respond(&crate::encode::response(option));
        let mut hint = None;
        for _ in 0..MAX_STEPS {
            let (status, messages) = settle(&mut duel, true);
            for m in &messages {
                if m["type"] == MSG_HINT && m["hint_type"] == HINT_SELECTMSG && m["player"] == me {
                    hint = m["hint"].as_u64();
                }
            }
            let ended = messages.iter().any(|m| [MSG_WIN, MSG_NEW_TURN, MSG_NEW_PHASE].contains(&m["type"].as_u64().unwrap_or(0)));
            let Some(prompt) = messages.last().filter(|_| status == Status::Awaiting && !ended) else { break };
            let kind = prompt["type"].as_u64().unwrap_or(0);
            if [MSG_SELECT_IDLECMD, MSG_SELECT_BATTLECMD, MSG_ANNOUNCE_CARD].contains(&kind) {
                break;
            }
            let answer = if prompt["player"] == me {
                let board = Board::observe(&mut duel, me, self.library, hidden);
                policy::answer(prompt, me, &board, hint)
            } else {
                policy::passive(prompt)
            };
            duel.respond(&crate::encode::response(&answer));
        }
        Board::observe(&mut duel, me, self.library, hidden).value()
    }

    /// The bot's answer to prompt `m` in duel `now` (the real one), with the last selection hint it was given and the
    /// ATK of the attacker it just declared. Also the ATK of the attacker this answer declares, if it attacks.
    pub fn think(&self, now: &mut Duel, m: &Json, hint: Option<u64>, attacker: Option<i32>) -> (Json, Option<i32>) {
        let me = m["player"].as_u64().unwrap_or(0) as u8 & 1;
        let hidden = Board::hidden_monsters(now, me);
        let mut board = Board::observe(now, me, self.library, &hidden);
        board.attacker = attacker;
        let answer = match options(m, &board, hint) {
            None => policy::answer(m, me, &board, hint),
            Some(options) => self.best(&options, m, me, &hidden, &board),
        };
        let declared = policy::declared_attacker(m, &answer, me, &board);
        (answer, declared)
    }

    /// The best of `options`: each tried in a copy, on threads of their own. For an idle prompt the last option
    /// (doing nothing more) is judged as the position is now, and an action must beat it by [`MARGIN`].
    fn best(&self, options: &[Json], m: &Json, me: u8, hidden: &Hidden, board: &Board) -> Json {
        let idle = m["type"] == MSG_SELECT_IDLECMD;
        let tried = if idle { &options[..options.len() - 1] } else { options };
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8).min(tried.len());
        let mut values = vec![f64::NEG_INFINITY; tried.len()];
        std::thread::scope(|scope| {
            let chunks: Vec<_> = (0..threads)
                .map(|t| {
                    scope.spawn(move || {
                        (t..tried.len()).step_by(threads).map(|i| (i, self.try_option(&tried[i], me, hidden))).collect::<Vec<_>>()
                    })
                })
                .collect();
            for chunk in chunks {
                for (i, v) in chunk.join().unwrap_or_default() {
                    values[i] = v;
                }
            }
        });
        trace(m, board, tried, &values);
        let (mut best, mut best_value) = (None, f64::NEG_INFINITY);
        for (i, v) in values.iter().enumerate() {
            if *v > best_value {
                (best, best_value) = (Some(i), *v);
            }
        }
        if idle {
            let stay = board.value();
            return match best {
                Some(i) if best_value > stay + MARGIN => tried[i].clone(),
                _ => options[options.len() - 1].clone(),
            };
        }
        best.map_or_else(|| options[0].clone(), |i| options[i].clone())
    }
}

/// With `YO3_BOT_TRACE=FILE`, each look-ahead decision goes to FILE: the prompt, the position's value, and each
/// option with what it came to (for tuning the bot; off by default).
fn trace(m: &Json, board: &Board, options: &[Json], values: &[f64]) {
    let Some(path) = std::env::var_os("YO3_BOT_TRACE") else { return };
    use std::io::Write;
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) else { return };
    let me = &board.sides[board.me as usize];
    let them = &board.sides[1 - board.me as usize];
    let _ = writeln!(file, "prompt {} now {:.0} (lp {}/{}, hand {}/{}, monsters {}/{})", m, board.value(), me.lp, them.lp, me.hand, them.hand,
        me.monsters.len(), them.monsters.len());
    for (o, v) in options.iter().zip(values) {
        let _ = writeln!(file, "  {v:>10.0}  {o}");
    }
}
