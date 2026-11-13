//! The duel bot: how an NPC plays. At a decision that matters (what to summon, activate or Set, whether to chain,
//! which card to pick, yes or no) it tries each of its options in a copy of the duel and keeps the option that leaves
//! it best placed ([`Board::value`]); everything else follows rules of thumb (`policy`).
//!
//! A copy plays the option out, the rest of the bot's turn by its rules, then the opponent's next turn, and is judged
//! when the bot's next turn begins ([`Reach::Turn`]). At an idle prompt the two most promising actions are also tried
//! with each follow-up action (a Tuner, then the Synchro Summon it allows). [`Reach::Action`] is the first version,
//! judging right after the action, kept to measure the bot against.
//!
//! The core can't copy a duel, so a copy is a new duel with the same seed and decks, given every answer since the
//! start ([`Setup`], the history): it plays out exactly the same. A few copies are kept in step with the real duel
//! ([`Pool`], rebuilt in the background while the duel goes on); the others are rebuilt when needed. Each option is
//! tried on a thread of its own; the choice doesn't depend on timing, so a recorded duel replays the bot exactly.
//!
//! The bot plays fair, unless its [`Style`] says it sees (Pegasus and his Millennium Eye). In a copy its opponent plays
//! only with its public cards: no chains, no optional effects, no cards from the hand, only its face-up monsters
//! attacking; the bot values a position only by what it may see ([`board`]); and it decides its own attacks, where a
//! copy would show a face-down monster's DEF, by rules alone.
use std::sync::{Arc, Mutex};

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
/// The most promising actions of an idle prompt that are also tried with each follow-up action, and how many
/// follow-ups each.
const COMBO_FIRSTS: usize = 2;
const COMBO_FOLLOWUPS: usize = 8;
/// A copy plays at most this many answers past the option before it is judged.
const MAX_STEPS: usize = 600;
/// Copies kept in step with the real duel: one for each thread, so a decision's first options start without a
/// rebuild (late in a duel with a full field, replaying an answer costs the core up to 80 ms: each idle prompt checks
/// every summon the Extra Deck allows).
fn pool_size() -> usize {
    threads()
}

/// The threads trying options at once: the machine's, at most 8.
fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8)
}

/// How long the bot may weigh one decision before it settles for the options it has tried. Each try needs a copy of
/// the duel, and once the pool's are used a copy is rebuilt by replaying every answer so far: late in a long duel a
/// decision took up to 2.7 s (tests/npc_bot.rs, seed 9001). The option of doing nothing more is always tried first,
/// each thread tries at least one option, and the two-step lines wait for time left.
pub const BUDGET: std::time::Duration = std::time::Duration::from_millis(700);
/// A try still playing this long past the deadline is judged where it stands.
const GRACE: std::time::Duration = std::time::Duration::from_millis(300);

/// How far a copy plays before it is judged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Reach {
    /// Until the option's action is done (the next idle or battle prompt, phase or turn).
    Action,
    /// Through the opponent's next turn, to the start of the bot's own.
    #[default]
    Turn,
}

/// How an NPC plays: its character.
#[derive(Clone, Copy, Debug, Default)]
pub struct Style {
    /// -1 (holds back) to 1 (attacks whenever it can, values the opponent's LP more).
    pub aggression: f64,
    /// 0 to 1: weighs what the opponent could do to it more, and needs more to spend a card.
    pub caution: f64,
    /// Sees the opponent's hand and Set cards, and expects the opponent to use them (Pegasus' Millennium Eye).
    pub sees: bool,
    pub reach: Reach,
}

impl Style {
    /// From the think request's `style` ({"aggression", "caution", "sees", "reach": "action"|"turn"}).
    pub fn from_json(v: &Json) -> Style {
        Style {
            aggression: v["aggression"].as_f64().unwrap_or(0.0).clamp(-1.0, 1.0),
            caution: v["caution"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0),
            sees: v["sees"] == true,
            reach: if v["reach"] == "action" { Reach::Action } else { Reach::Turn },
        }
    }
}

/// How a duel began: what `new` gave the core, in order.
#[derive(Clone, Default)]
pub struct Setup {
    pub seed: [u64; 4],
    pub lp: u32,
    /// (team, location, passcode), in the order the cards were made.
    pub cards: Vec<(u8, u32, u32)>,
    /// The skills given, as (team, Lua call), loaded after the cards (see [`skill_call`]).
    pub skills: Vec<(u8, String)>,
}

/// The skills' Lua (`YO3Skill`), loaded into a duel that has any.
pub const SKILLS: &str = include_str!("../skills.lua");

/// The Lua call that gives `team` the skill named `name` (as the server names it), or None for a skill this engine
/// doesn't know.
pub fn skill_call(name: &str, team: u8) -> Option<String> {
    let function = match name {
        "heart_of_the_cards" => "HeartOfTheCards",
        _ => return None,
    };
    Some(format!("YO3Skill.{function}({team})"))
}

/// Loads `setup`'s skills into `duel` (before it starts).
pub fn give_skills(duel: &mut Duel, setup: &Setup) {
    if setup.skills.is_empty() {
        return;
    }
    duel.load_script("yo3_skills.lua", SKILLS.as_bytes());
    for (i, (_, call)) in setup.skills.iter().enumerate() {
        duel.load_script(&format!("yo3_skill_{i}.lua"), call.as_bytes());
    }
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

/// A copy of a duel, which may move to another thread: its host is [`Quiet`] (card data behind an `Arc`, no
/// logging), and the core keeps no state outside its duel object.
pub struct Copy(Duel);

// SAFETY: a Copy's duel is only ever made by `build` with a Quiet host (Send), and is used by one thread at a time.
unsafe impl Send for Copy {}

/// Copies kept in step with the real duel: `at` answers into its history, for duel `generation`.
#[derive(Default)]
pub struct Pool {
    state: Mutex<PoolState>,
}

#[derive(Default)]
struct PoolState {
    generation: u64,
    ready: Vec<(Copy, usize)>,
    building: usize,
}

impl Pool {
    /// A new duel: the copies of the old one are dropped.
    pub fn reset(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.generation += 1;
        s.ready.clear();
    }

    fn take(&self) -> Option<(Copy, usize)> {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).ready.pop()
    }

    /// Starts building copies in the background until [`pool_size`] are ready or on their way, from the duel as it is
    /// now (`history`).
    pub fn refill(self: &Arc<Self>, library: &Arc<Library>, setup: &Arc<Setup>, history: &[Vec<u8>]) {
        let (missing, generation) = {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let missing = pool_size().saturating_sub(s.ready.len() + s.building);
            s.building += missing;
            (missing, s.generation)
        };
        for _ in 0..missing {
            let (pool, library, setup, history) = (self.clone(), library.clone(), setup.clone(), history.to_vec());
            std::thread::spawn(move || {
                let copy = build(&library, &setup, &history);
                let mut s = pool.state.lock().unwrap_or_else(|e| e.into_inner());
                s.building -= 1;
                if let Some(copy) = copy.filter(|_| s.generation == generation) {
                    s.ready.push((copy, history.len()));
                }
            });
        }
    }
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

/// A new copy of the duel `setup` began, after the answers `history`: None if the core refuses it.
fn build(library: &Arc<Library>, setup: &Setup, history: &[Vec<u8>]) -> Option<Copy> {
    let mut duel = Duel::new(setup.seed, MODE_MR2, setup.lp, 5, Box::new(Quiet(library.clone())))?;
    for name in ["constant.lua", "utility.lua"] {
        let text = library.script(name).unwrap_or_default();
        duel.load_script(name, &text);
    }
    for &(team, location, code) in &setup.cards {
        duel.new_card(team, 0, code, team, location, 0, POSITION_FACEDOWN_DEFENSE);
    }
    give_skills(&mut duel, setup);
    duel.start();
    settle(&mut duel, false);
    for answer in history {
        duel.respond(answer);
        settle(&mut duel, false);
    }
    Some(Copy(duel))
}

/// The options worth trying at prompt `m`, in the order the bot prefers them when they come out even; None when
/// rules decide. For an idle prompt the last option is doing nothing more (battle, or end the turn).
fn options(m: &Json, board: &Board, hint: Option<u64>) -> Option<Vec<Json>> {
    let len = |k: &str| m[k].as_array().map_or(0, Vec::len);
    let kind = m["type"].as_u64().unwrap_or(0);
    let sees = board.style.sees;
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
        // Attacks are tried only by a bot that sees: a copy would show a face-down monster's DEF.
        MSG_SELECT_BATTLECMD if sees => {
            let end = if m["to_m2"] == true { BATTLE_TO_M2 } else { BATTLE_TO_EP };
            (0..len("attacks")).map(|i| json!({ "type": RESP_BATTLECMD, "action": BATTLE_ATTACK, "index": i }))
                .chain([json!({ "type": RESP_BATTLECMD, "action": end, "index": null })])
                .collect()
        }
        MSG_SELECT_CHAIN if len("selects") > 0 && m["forced"] != true => {
            std::iter::once(json!({ "type": RESP_CHAIN, "index": null }))
                .chain((0..len("selects")).map(|i| json!({ "type": RESP_CHAIN, "index": i })))
                .collect()
        }
        MSG_SELECT_EFFECTYN => vec![json!({ "type": RESP_EFFECTYN, "yes": true }), json!({ "type": RESP_EFFECTYN, "yes": false })],
        MSG_SELECT_YESNO => vec![json!({ "type": RESP_YESNO, "yes": true }), json!({ "type": RESP_YESNO, "yes": false })],
        MSG_SELECT_OPTION if len("options") > 1 => (0..len("options")).map(|i| json!({ "type": RESP_OPTION, "index": i })).collect(),
        // One card, unless it is an attack target and the bot doesn't see: of many (the Heart of the Cards' Deck), the
        // rules' favourites; the rules' favourite first, so it wins a tie.
        MSG_SELECT_CARD if m["max"] == 1 && m["min"] == 1 && len("selects") >= 2 && (sees || hint != Some(HINTMSG_ATTACKTARGET)) => {
            policy::ranked(m, board.me, board, hint).into_iter().map(|i| json!({ "type": RESP_CARD, "indicies": [i] })).collect()
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

/// The decision at hand and what is needed to copy the duel.
pub struct Thinker<'a> {
    pub library: &'a Arc<Library>,
    pub setup: &'a Setup,
    /// Every answer given so far, as the core read it.
    pub history: &'a [Vec<u8>],
    pub pool: &'a Pool,
    pub style: Style,
    /// When to stop trying more options ([`BUDGET`] from the start of the decision).
    pub deadline: std::time::Instant,
}

impl Thinker<'_> {
    /// The duel as it is now: a pooled copy brought up to date, else one rebuilt from the start.
    fn copy(&self) -> Option<Duel> {
        if let Some((Copy(mut duel), at)) = self.pool.take().filter(|(_, at)| *at <= self.history.len()) {
            for answer in &self.history[at..] {
                duel.respond(answer);
                settle(&mut duel, false);
            }
            return Some(duel);
        }
        build(self.library, self.setup, self.history).map(|Copy(d)| d)
    }

    /// What `me` sees of `duel` with this thinker's style.
    fn observe(&self, duel: &mut Duel, me: u8, hidden: &Hidden) -> Board {
        Board::observe(duel, me, self.library, hidden, self.style)
    }

    /// Answers prompt `p` in a copy: the bot by its rules, the opponent with its public cards only.
    fn copy_answer(&self, duel: &mut Duel, p: &Json, me: u8, hidden: &Hidden, hint: Option<u64>) -> Json {
        if p["player"] == me {
            let board = self.observe(duel, me, hidden);
            return policy::in_copy(p, me, &board, hint);
        }
        let them = 1 - me;
        let kind = p["type"].as_u64().unwrap_or(0);
        match kind {
            // Its public cards only: battle with the monsters it shows, nothing from the hand.
            MSG_SELECT_IDLECMD => {
                let action = if p["to_bp"] == true { IDLE_TO_BP } else { IDLE_TO_EP };
                json!({ "type": RESP_IDLECMD, "action": action, "index": null })
            }
            MSG_SELECT_BATTLECMD => {
                let mine_hidden: Hidden = Board::hidden_monsters(duel, them);
                let board = Board::observe(duel, them, self.library, &mine_hidden, Style::default());
                let public = json!({ "type": MSG_SELECT_BATTLECMD, "to_m2": p["to_m2"], "to_ep": p["to_ep"],
                    "attacks": p["attacks"].as_array().map(|a| a.iter().filter(|a| board.sides[them as usize].monster(a["sequence"].as_u64().unwrap_or(9) as u8).is_some_and(|m| m.faceup)).cloned().collect::<Vec<_>>()) });
                let mut answer = policy::answer(&public, them, &board, None);
                // Back to the real list's index.
                if answer["action"] == BATTLE_ATTACK
                    && let Some(chosen) = public["attacks"].get(answer["index"].as_u64().unwrap_or(0) as usize)
                {
                    answer["index"] = json!(p["attacks"].as_array().and_then(|a| a.iter().position(|x| x == chosen)).unwrap_or(0));
                }
                answer
            }
            _ => policy::passive(p),
        }
    }

    /// How the position looks to `me` after the answers `line` in a copy: the first answers the decision at hand,
    /// each further one the bot's next idle prompt. The copy plays on until [`Style::reach`] says to judge it.
    fn try_line(&self, line: &[Json], me: u8, hidden: &Hidden) -> f64 {
        let Some(mut duel) = self.copy() else { return f64::NEG_INFINITY };
        duel.respond(&crate::encode::response(&line[0]));
        let mut next = 1;
        let mut hint = None;
        let cutoff = self.deadline + GRACE;
        for _ in 0..MAX_STEPS {
            if std::time::Instant::now() > cutoff {
                break;
            }
            let (status, messages) = settle(&mut duel, true);
            for m in &messages {
                if m["type"] == MSG_HINT && m["hint_type"] == HINT_SELECTMSG && m["player"] == me {
                    hint = m["hint"].as_u64();
                }
            }
            let kind_of = |m: &Json| m["type"].as_u64().unwrap_or(0);
            let ended = messages.iter().any(|m| match self.style.reach {
                Reach::Action => [MSG_WIN, MSG_NEW_TURN, MSG_NEW_PHASE].contains(&kind_of(m)),
                Reach::Turn => kind_of(m) == MSG_WIN || kind_of(m) == MSG_NEW_TURN && m["player"] == me,
            });
            let Some(prompt) = messages.last().filter(|_| status == Status::Awaiting && !ended) else { break };
            let kind = kind_of(prompt);
            if kind == MSG_ANNOUNCE_CARD
                || self.style.reach == Reach::Action && [MSG_SELECT_IDLECMD, MSG_SELECT_BATTLECMD].contains(&kind)
            {
                break;
            }
            let answer = if kind == MSG_SELECT_IDLECMD && prompt["player"] == me && next < line.len() {
                next += 1;
                line[next - 1].clone()
            } else {
                self.copy_answer(&mut duel, prompt, me, hidden, hint)
            };
            duel.respond(&crate::encode::response(&answer));
        }
        self.observe(&mut duel, me, hidden).value()
    }

    /// The bot's next idle prompt after answering `first` in a copy (the bot's rules answering what comes between),
    /// with the board then: None when the turn moves on first.
    fn next_idle(&self, first: &Json, me: u8, hidden: &Hidden) -> Option<(Json, Board)> {
        let mut duel = self.copy()?;
        duel.respond(&crate::encode::response(first));
        let mut hint = None;
        for _ in 0..MAX_STEPS {
            let (status, messages) = settle(&mut duel, true);
            for m in &messages {
                if m["type"] == MSG_HINT && m["hint_type"] == HINT_SELECTMSG && m["player"] == me {
                    hint = m["hint"].as_u64();
                }
            }
            if messages.iter().any(|m| [MSG_WIN, MSG_NEW_TURN, MSG_NEW_PHASE].contains(&m["type"].as_u64().unwrap_or(0))) {
                return None;
            }
            let prompt = messages.last().filter(|_| status == Status::Awaiting)?;
            if prompt["player"] == me && prompt["type"] == MSG_SELECT_IDLECMD {
                let board = self.observe(&mut duel, me, hidden);
                return Some((prompt.clone(), board));
            }
            if [MSG_SELECT_BATTLECMD, MSG_ANNOUNCE_CARD].contains(&prompt["type"].as_u64().unwrap_or(0)) {
                return None;
            }
            let answer = self.copy_answer(&mut duel, prompt, me, hidden, hint);
            duel.respond(&crate::encode::response(&answer));
        }
        None
    }

    /// The bot's answer to prompt `m` in duel `now` (the real one), with the last selection hint it was given and the
    /// ATK of the attacker it just declared. Also the ATK of the attacker this answer declares, if it attacks.
    pub fn think(&self, now: &mut Duel, m: &Json, hint: Option<u64>, attacker: Option<i32>) -> (Json, Option<i32>) {
        let me = m["player"].as_u64().unwrap_or(0) as u8 & 1;
        let hidden = if self.style.sees { Hidden::new() } else { Board::hidden_monsters(now, me) };
        let mut board = self.observe(now, me, &hidden);
        board.attacker = attacker;
        let answer = match options(m, &board, hint) {
            None => policy::answer(m, me, &board, hint),
            Some(options) => self.best(&options, m, me, &hidden, &board),
        };
        let declared = policy::declared_attacker(m, &answer, me, &board);
        (answer, declared)
    }

    /// The lines' values, each tried in a copy, on threads of their own.
    fn values(&self, lines: &[Vec<Json>], me: u8, hidden: &Hidden) -> Vec<f64> {
        let threads = threads().min(lines.len().max(1));
        let mut values = vec![f64::NEG_INFINITY; lines.len()];
        std::thread::scope(|scope| {
            let chunks: Vec<_> = (0..threads)
                .map(|t| scope.spawn(move || {
                    // Past the deadline a thread stops after its first line; the lines left are not tried.
                    (t..lines.len()).step_by(threads).enumerate()
                        .take_while(|(k, _)| *k == 0 || std::time::Instant::now() < self.deadline)
                        .map(|(_, i)| (i, self.try_line(&lines[i], me, hidden))).collect::<Vec<_>>()
                }))
                .collect();
            for chunk in chunks {
                for (i, v) in chunk.join().unwrap_or_default() {
                    values[i] = v;
                }
            }
        });
        values
    }

    /// The best of `options`. At an idle prompt the last option is doing nothing more, which an action must beat by
    /// [`MARGIN`] (more for a cautious style); with [`Reach::Turn`] the most promising actions are also tried with
    /// each follow-up action, and count as their best line.
    fn best(&self, options: &[Json], m: &Json, me: u8, hidden: &Hidden, board: &Board) -> Json {
        let started = std::time::Instant::now();
        let idle = m["type"] == MSG_SELECT_IDLECMD;
        let deep = idle && self.style.reach == Reach::Turn;
        let tried = if idle && !deep { &options[..options.len() - 1] } else { options };
        let mut lines: Vec<Vec<Json>> = tried.iter().map(|o| vec![o.clone()]).collect();
        // Doing nothing more first: it is what every action must beat, so it is always tried.
        if deep {
            lines.rotate_right(1);
        }
        let mut values = self.values(&lines, me, hidden);
        if deep {
            values.rotate_left(1);
            if std::time::Instant::now() < self.deadline {
                self.combos(&tried[..tried.len() - 1], &mut values, me, hidden);
            }
        }
        trace(m, board, tried, &values, started.elapsed());
        let (mut best, mut best_value) = (None, f64::NEG_INFINITY);
        for (i, v) in values.iter().enumerate() {
            if *v > best_value {
                (best, best_value) = (Some(i), *v);
            }
        }
        if idle {
            let stay = if deep { values[values.len() - 1] } else { board.value() };
            let margin = MARGIN * (1.0 + 4.0 * self.style.caution);
            return match best {
                Some(i) if i < options.len() - 1 && best_value > stay + margin => options[i].clone(),
                _ => options[options.len() - 1].clone(),
            };
        }
        best.map_or_else(|| options[0].clone(), |i| options[i].clone())
    }

    /// Two-step lines: the [`COMBO_FIRSTS`] best `actions` (by `values`) each tried with every follow-up action at the
    /// bot's next idle prompt; an action's value becomes its best line's when that is better.
    fn combos(&self, actions: &[Json], values: &mut [f64], me: u8, hidden: &Hidden) {
        let mut order: Vec<usize> = (0..actions.len()).collect();
        order.sort_by(|a, b| values[*b].total_cmp(&values[*a]));
        let firsts: Vec<usize> = order.into_iter().take(COMBO_FIRSTS).collect();
        let nexts: Vec<Option<(Json, Board)>> = std::thread::scope(|scope| {
            let handles: Vec<_> = firsts.iter().map(|&i| scope.spawn(move || self.next_idle(&actions[i], me, hidden))).collect();
            handles.into_iter().map(|h| h.join().unwrap_or(None)).collect()
        });
        let mut lines = vec![];
        for (&i, next) in firsts.iter().zip(&nexts) {
            let Some((prompt, board)) = next else { continue };
            let Some(follow) = options(prompt, board, None) else { continue };
            // The follow-up actions, not doing nothing (that line is the action's own value).
            for f in follow.iter().take(follow.len() - 1).take(COMBO_FOLLOWUPS) {
                lines.push((i, vec![actions[i].clone(), f.clone()]));
            }
        }
        let found = self.values(&lines.iter().map(|(_, l)| l.clone()).collect::<Vec<_>>(), me, hidden);
        for ((i, _), v) in lines.iter().zip(found) {
            values[*i] = values[*i].max(v);
        }
    }
}

/// With `YO3_BOT_TRACE=FILE`, each look-ahead decision goes to FILE: the prompt, the position's value, and each
/// option with what it came to (for tuning the bot; off by default).
fn trace(m: &Json, board: &Board, options: &[Json], values: &[f64], took: std::time::Duration) {
    let Some(path) = std::env::var_os("YO3_BOT_TRACE") else { return };
    use std::io::Write;
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) else { return };
    let me = &board.sides[board.me as usize];
    let them = &board.sides[1 - board.me as usize];
    let _ = writeln!(file, "prompt {} now {:.0} (lp {}/{}, hand {}/{}, monsters {}/{}) in {} ms", m, board.value(), me.lp, them.lp, me.hand,
        them.hand, me.monsters.len(), them.monsters.len(), took.as_millis());
    for (o, v) in options.iter().zip(values) {
        let _ = writeln!(file, "  {v:>10.0}  {o}");
    }
}
