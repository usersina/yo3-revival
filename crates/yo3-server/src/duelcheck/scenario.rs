//! Scenario tests' harness: a duel through the real rules engine where the test plays by plan ("summon these, then
//! Synchro Summon that"; the opponent passes or has a plan of its own), with both seats' client models fed the
//! commands the translator sends and compared with the engine after every step, as the duel check does (#67).
//!
//! Decks are Konami ids and are drawn from their end: the last five cards of a main deck are the opening hand, the
//! card before them the first draw. Cards' stats come from the engine (the client model follows its updates).
//!
//! ```ignore
//! let Some(dir) = scenario::engine_dir() else { return };
//! let mut s = Scenario::start(&dir, 7, 8000, [(&deck, &[JUNK_WARRIOR]), (&[WARWOLF; 40], &[])]).unwrap();
//! s.run(&[Summon(HEDGEHOG), Summon(JUNK_SYNCHRON), SpSummon(JUNK_WARRIOR)]).unwrap();
//! assert_eq!(s.attack_of(0, JUNK_WARRIOR), Some(2300));
//! assert!(s.problems.is_empty(), "{:#?}", s.problems);
//! ```
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};

use super::ClientModel;
use crate::duel::{command, piles, setup};
use crate::engine::{Answer, Engine};
use crate::front::Deck;
use crate::ocg::{Response, battle, idle, msg};
use crate::translate::{CardStats, Translator, bot_answer};
use crate::wire::cmd;

/// One step of the planned side's play, by Konami id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Normal Summon.
    Summon(u16),
    /// Set a monster.
    SetMonster(u16),
    /// Special Summon (Synchro, Fusion, Xyz, or a card's own procedure).
    SpSummon(u16),
    /// Set a Spell or Trap.
    SetSpell(u16),
    /// Activate an effect or a card (a Set card, one in the hand, a monster's ignition effect).
    Activate(u16),
    /// Attack with this monster (entering the Battle Phase first); the target is the engine's default answer.
    Attack(u16),
    /// Chain this card's effect at the first chance its team gets to (Mirror Force at an attack).
    Chain(u16),
    /// End the turn: the plan goes on in the next own turn.
    EndTurn,
}

/// The engine's folder: `YO3_ENGINE_DIR`, else the repository's `engine/`; None (the test skips) before engine/fetch.sh.
pub fn engine_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("YO3_ENGINE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine"));
    if !dir.join("vendor/cardinfo.json").is_file() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    Some(dir)
}

/// A duel between the planned side (engine team 0, which goes first) and a passive opponent.
pub struct Scenario {
    engine: Engine,
    views: Vec<(Translator, ClientModel)>,
    answer: Answer,
    state: Json,
    steps: usize,
    /// Every difference between a seat's client model and the engine, with the seat and the step.
    pub problems: Vec<String>,
}

/// The most engine steps a scenario takes, and the own turns an action may wait to become possible.
const MAX_STEPS: usize = 800;
const MAX_WAIT_TURNS: u32 = 8;

impl Scenario {
    /// Starts the duel: `decks` by engine team, (main deck, extra deck) in Konami ids, team 0 planned.
    pub fn start(dir: &Path, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2]) -> Result<Scenario, String> {
        let mut engine = Engine::start(dir).map_err(|e| format!("engine: {e}"))?;
        let all: Vec<u16> = {
            let mut v: Vec<u16> = decks.iter().flat_map(|(m, e)| m.iter().chain(e.iter())).copied().collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let list: Vec<(u16, &str)> = all.iter().map(|&c| (c, "")).collect();
        let data = engine.cards(&list).map_err(|e| format!("engine: {e}"))?;
        const TYPE_TUNER: u64 = 0x1000;
        let tuners: HashSet<u16> =
            all.iter().zip(&data).filter(|(_, d)| d["data"]["type"].as_u64().is_some_and(|t| t & TYPE_TUNER != 0)).map(|(c, _)| *c).collect();
        let cards: HashMap<u16, CardStats> = all.iter().map(|&c| (c, CardStats { cid: c, ..CardStats::default() })).collect();
        let views = (0..2)
            .map(|seat| {
                let (mine, theirs) = (decks[seat], decks[1 - seat]);
                let (mut translator, mut table) = Translator::against_player(seat as u32, [mine.0, theirs.0], cards.clone());
                translator.tuners(tuners.clone());
                table.extend(translator.extra_decks([mine.1, theirs.1]));
                let mut model = ClientModel::new(seat as u32, cards.clone());
                let deck = |(main, extra): (&[u16], &[u16])| Deck { id: 0, name: String::new(), main: main.to_vec(), extra: extra.to_vec(), side: vec![] };
                let mut opening = vec![setup(seat as u32, &deck(mine), &deck(theirs))];
                opening.extend(table);
                opening.extend([command(cmd::DUEL_START, [0; 4]), command(cmd::SET_LP, [0, lp, 0, 0]), command(cmd::SET_LP, [1, lp, 0, 0])]);
                opening.extend(piles());
                model.apply_all(&opening);
                (translator, model)
            })
            .collect();
        let answer = engine.new_duel(seed, lp, decks).map_err(|e| format!("engine: {e}"))?;
        let mut s = Scenario { engine, views, answer, state: Json::Null, steps: 0, problems: vec![] };
        s.observe()?;
        Ok(s)
    }

    /// Feeds the last answer to both seats and compares them with the engine.
    fn observe(&mut self) -> Result<(), String> {
        if !self.answer.errors.is_empty() {
            return Err(format!("step {}: the engine reported {:?}", self.steps, self.answer.errors));
        }
        self.state = self.engine.state().map_err(|e| format!("engine state: {e}"))?;
        for (seat, (translator, model)) in self.views.iter_mut().enumerate() {
            model.observe_engine(&self.answer.messages);
            model.apply_all(&translator.events(&self.answer.messages));
            for problem in model.check(&self.state) {
                self.problems.push(format!("seat {seat}, step {}: {problem}", self.steps));
            }
        }
        Ok(())
    }

    /// Answers the current prompt and observes the result.
    pub fn respond(&mut self, response: Json) -> Result<(), String> {
        if self.answer.status != "await" {
            return Err(format!("the duel is over ({}) at step {}", self.answer.status, self.steps));
        }
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(format!("no end after {MAX_STEPS} steps"));
        }
        self.answer = self.engine.respond(response).map_err(|e| format!("engine: {e}"))?;
        self.observe()
    }

    /// The engine's last message: the prompt waiting for an answer.
    pub fn prompt(&self) -> &Json {
        self.answer.messages.last().unwrap_or(&Json::Null)
    }

    /// The engine's messages of the last step.
    pub fn messages(&self) -> &[Json] {
        &self.answer.messages
    }

    /// The engine's field (`state` op): per team, `mzone`, `szone`, `hand`, `grave`... with each card's stats.
    pub fn state(&self) -> &Json {
        &self.state
    }

    /// The first monster with this Konami id in a team's Monster Zones.
    pub fn monster(&self, team: usize, cid: u16) -> Option<&Json> {
        self.state[team]["mzone"].as_array()?.iter().find(|c| c["cid"] == cid)
    }

    /// That monster's current ATK.
    pub fn attack_of(&self, team: usize, cid: u16) -> Option<i64> {
        self.monster(team, cid)?["attack"].as_i64()
    }

    /// How many cards with this Konami id a team has in a place (`grave`, `hand`, `szone`, `removed`...).
    pub fn count(&self, team: usize, place: &str, cid: u16) -> usize {
        self.state[team][place].as_array().map_or(0, |cards| cards.iter().filter(|c| c["cid"] == cid).count())
    }

    /// A team's Life Points.
    pub fn lp(&self, team: usize) -> i64 {
        self.state[team]["lp"].as_i64().unwrap_or(-1)
    }

    /// A monster's position on the field (`pos`: 1 face-up Attack, 4 face-up Defense, 8 face-down Defense).
    pub fn position_of(&self, team: usize, cid: u16) -> Option<u64> {
        self.monster(team, cid)?["position"].as_u64()
    }

    /// A monster's current Level.
    pub fn level_of(&self, team: usize, cid: u16) -> Option<u64> {
        self.monster(team, cid)?["level"].as_u64()
    }

    /// The default answer: the opponent and the planned side between actions pass (end the turn from the Main Phase,
    /// leave the Battle Phase), every other prompt gets the CPU's answer.
    pub fn pass(&mut self) -> Result<(), String> {
        let p = self.prompt();
        let response = if p["type"] == msg::SELECT_IDLECMD {
            json!({"type": 1, "action": idle::TO_EP, "index": null})
        } else if p["type"] == msg::SELECT_BATTLECMD {
            json!({"type": 1, "action": battle::TO_EP, "index": null})
        } else {
            bot_answer(p)
        };
        self.respond(response)
    }

    /// Plays the plan in order on the planned side's idle prompts: an action not possible yet ends the turn and waits
    /// for the next own turn (a Normal Summon a turn), at most a few turns. Returns at the first settled idle prompt
    /// after the last action, when its effects have resolved.
    pub fn run(&mut self, plan: &[Action]) -> Result<(), String> {
        self.run_both([plan, &[]])
    }

    /// [`run`](Self::run) with a plan for each team (team 1 goes second): each team's actions happen on its own
    /// prompts, in order. Returns at the first idle prompt after both plans are done.
    pub fn run_both(&mut self, plans: [&[Action]; 2]) -> Result<(), String> {
        let mut next = [0usize; 2];
        let mut waited = [0u32; 2];
        loop {
            let p = self.prompt().clone();
            let team = p["player"].as_u64().unwrap_or(0) as usize & 1;
            let kind = p["type"].as_u64().unwrap_or(0);
            let done = next[0] == plans[0].len() && next[1] == plans[1].len();
            if done && (kind == msg::SELECT_IDLECMD || kind == msg::SELECT_BATTLECMD) {
                return Ok(());
            }
            let action = plans[team].get(next[team]).copied();
            let found = |list: &str, cid: u16| p[list].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid));
            if kind == msg::SELECT_IDLECMD {
                let choice = match action {
                    Some(Action::Summon(c)) => found("summons", c).map(|i| (idle::SUMMON, i)),
                    Some(Action::SetMonster(c)) => found("monster_sets", c).map(|i| (idle::MONSTER_SET, i)),
                    Some(Action::SpSummon(c)) => found("special_summons", c).map(|i| (idle::SPSUMMON, i)),
                    Some(Action::SetSpell(c)) => found("spell_sets", c).map(|i| (idle::SPELL_SET, i)),
                    Some(Action::Activate(c)) => found("activates", c).map(|i| (idle::ACTIVATE, i)),
                    Some(Action::Attack(_)) if p["to_bp"] == true => {
                        // Into the Battle Phase; the attack is chosen at its prompt below.
                        self.respond(json!({"type": 1, "action": idle::TO_BP, "index": null}))?;
                        continue;
                    }
                    _ => None,
                };
                match (choice, action) {
                    (Some((act, index)), _) => {
                        next[team] += 1;
                        waited[team] = 0;
                        self.respond(json!({"type": 1, "action": act, "index": index}))?;
                    }
                    (None, Some(Action::EndTurn)) => {
                        next[team] += 1;
                        self.pass()?;
                    }
                    (None, Some(action)) => {
                        waited[team] += 1;
                        if waited[team] > MAX_WAIT_TURNS {
                            return Err(format!("team {team}: {action:?} never became possible: {p}"));
                        }
                        self.pass()?;
                    }
                    (None, None) => self.pass()?,
                }
            } else if kind == msg::SELECT_BATTLECMD {
                match action {
                    Some(Action::Attack(c)) => {
                        let i = p["attacks"].as_array().and_then(|v| v.iter().position(|a| a["cid"] == c));
                        let Some(i) = i else { return Err(format!("team {team}: {c} can't attack: {p}")) };
                        next[team] += 1;
                        self.respond(json!({"type": 1, "action": battle::ATTACK, "index": i}))?;
                    }
                    _ => self.respond(json!({"type": 1, "action": battle::TO_M2, "index": null}))?,
                }
            } else if let (true, Some(Action::Chain(c))) = (kind == msg::SELECT_CHAIN, action)
                && let Some(i) = found("selects", c)
            {
                next[team] += 1;
                self.respond(Response::Chain(Some(i)).json())?;
            } else {
                self.pass()?;
            }
        }
    }
}
