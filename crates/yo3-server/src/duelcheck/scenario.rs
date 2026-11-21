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

/// A prompt without its long card lists, for an error.
fn short(p: &Json) -> String {
    let mut text = p.to_string();
    text.truncate(300);
    text
}

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
    /// Chain this card's effect only in answer to an attack declaration (Zero Gardna, Mirror Force): a chance of
    /// the declaring step, not one at a summon or a phase.
    ChainOnAttack(u16),
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
    /// The engine message types of the last steps (newest last), for an error to say what happened before it.
    trail: Vec<String>,
    /// An attack was declared and its battle has not been calculated yet (the chances to chain to the attack).
    attacking: bool,
    /// Cards the planned side picks first when an effect asks it to select cards (a target, a discard): (controller, Konami id).
    prefer: Vec<(u64, u16)>,
    /// The position the planned side's position choices take when it is allowed (`pos::FACEUP_DEFENSE`...); None: the CPU's (Attack).
    position: Option<u64>,
    /// The number the planned side declares when asked, through the client's windows (`declare_number`).
    number: Option<u64>,
    /// The client's dialogs shown for the planned side's numbers, one per step.
    pub dialogs: usize,
    /// Every prompt answered so far as (team, engine message type), oldest first.
    pub prompts: Vec<(usize, u64)>,
    /// Every difference between a seat's client model and the engine, with the seat and the step.
    pub problems: Vec<String>,
}

/// The most engine steps a scenario takes, and the own turns an action may wait to become possible.
const MAX_STEPS: usize = 800;
const MAX_WAIT_TURNS: u32 = 8;

impl Scenario {
    /// Starts the duel: `decks` by engine team, (main deck, extra deck) in Konami ids, team 0 planned.
    pub fn start(dir: &Path, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2]) -> Result<Scenario, String> {
        Scenario::start_with(dir, false, seed, lp, decks, &[])
    }

    /// `start` with hidden skills given to a team (the engine's `skills`: (team, name), as an NPC's or Destiny Draw, #71).
    pub fn start_skills(dir: &Path, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2], skills: &[(u32, String)]) -> Result<Scenario, String> {
        Scenario::start_with(dir, false, seed, lp, decks, skills)
    }

    /// `start` with the anime card versions on (the `anime-cards` setting): the reviewed cards play by their anime effects.
    pub fn start_anime(dir: &Path, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2]) -> Result<Scenario, String> {
        Scenario::start_with(dir, true, seed, lp, decks, &[])
    }

    fn start_with(dir: &Path, anime: bool, seed: u64, lp: u32, decks: [(&[u16], &[u16]); 2], skills: &[(u32, String)]) -> Result<Scenario, String> {
        let mut engine = Engine::start_with(dir, anime).map_err(|e| format!("engine: {e}"))?;
        if anime && !engine.anime_on().map_err(|e| format!("engine: {e}"))? {
            return Err("the anime card versions are not available: engine/fetch.sh and engine/anime-cards.json".into());
        }
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
        let mut cards: HashMap<u16, CardStats> = all.iter().map(|&c| (c, CardStats { cid: c, ..CardStats::default() })).collect();
        // The anime cards' tokens are the client's token cards, found by name.
        let tokens = if anime {
            let (rows, names) = super::selfplay::client_tokens(dir)?;
            cards.extend(rows);
            names
        } else {
            HashMap::new()
        };
        let views = (0..2)
            .map(|seat| {
                let (mine, theirs) = (decks[seat], decks[1 - seat]);
                let (mut translator, mut table) = Translator::against_player(seat as u32, [mine.0, theirs.0], cards.clone());
                translator.tokens(tokens.clone());
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
        let answer = engine.new_duel_with(seed, lp, decks, &HashMap::new(), skills).map_err(|e| format!("engine: {e}"))?;
        let mut s = Scenario { engine, views, answer, state: Json::Null, steps: 0, trail: vec![], attacking: false, prefer: vec![], position: None, number: None, dialogs: 0, prompts: vec![], problems: vec![] };
        s.observe()?;
        Ok(s)
    }

    /// Feeds the last answer to both seats and compares them with the engine.
    fn observe(&mut self) -> Result<(), String> {
        if !self.answer.errors.is_empty() {
            return Err(format!("step {}: the engine reported {:?}", self.steps, self.answer.errors));
        }
        for m in &self.answer.messages {
            match m["type"].as_u64() {
                Some(msg::ATTACK) => self.attacking = true,
                Some(msg::BATTLE) | Some(msg::ATTACK_DISABLED) | Some(msg::DAMAGE_STEP_END) | Some(msg::NEW_PHASE) => self.attacking = false,
                _ => {}
            }
        }
        let kinds: Vec<String> = self.answer.messages.iter().map(|m| m["type"].to_string()).collect();
        self.trail.push(format!("{}: {}", self.steps, kinds.join(" ")));
        if self.trail.len() > 12 {
            self.trail.remove(0);
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
        let asked = self.prompt();
        self.prompts.push((asked["player"].as_u64().unwrap_or(0) as usize & 1, asked["type"].as_u64().unwrap_or(0)));
        if self.steps > MAX_STEPS {
            return Err(format!("no end after {MAX_STEPS} steps"));
        }
        self.answer = self.engine.respond(response).map_err(|e| format!("engine: {e}"))?;
        self.observe()
    }

    /// A line for an error: the cards in each team's hand and the engine message types of the last steps.
    pub fn context(&self) -> String {
        let cids = |t: usize, place: &str| -> Vec<u64> {
            self.state[t][place].as_array().map_or(vec![], |h| h.iter().filter_map(|c| c["cid"].as_u64()).collect())
        };
        let teams: Vec<String> = (0..2)
            .map(|t| {
                let [h, m, st, g, r] = ["hand", "mzone", "szone", "grave", "removed"].map(|p| cids(t, p));
                format!("team {t}: LP {}, hand {h:?}, monsters {m:?}, spells {st:?}, grave {g:?}, banished {r:?}", self.lp(t))
            })
            .collect();
        format!("{}\nmessage types of the last steps: [{}]", teams.join("\n"), self.trail.join(" | "))
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

    /// The hover markers seat `seat`'s client holds: (source place, target place, marker type), places as the
    /// client's `player | zone << 8` (its own side 0).
    pub fn markers(&self, seat: usize) -> &[(u32, u32, u32)] {
        self.views[seat].1.relations()
    }

    /// Card choices (targets, discards, Tributes) take these cards first, (controller, Konami id), when they are on offer:
    /// "Brionac returns the opponent's monster, not itself". Either team's choices; the default answer fills the rest.
    pub fn prefer(&mut self, picks: &[(usize, u16)]) {
        self.prefer = picks.iter().map(|&(team, cid)| (team as u64, cid)).collect();
    }

    /// Position choices (a Special Summon that lets its player pick) take this position when it is allowed, for every
    /// player; `None` goes back to the CPU's choice (face-up Attack).
    pub fn prefer_position(&mut self, position: Option<u64>) {
        self.position = position;
    }

    /// Number declarations (Wall of Revealing Light's LP, Star Blast's) are answered the way a player does: through the
    /// translator's windows for that seat, their Select dialogs narrowed step by step to this number (the label whose
    /// range holds it) or the number dialog set to it, the client's model fed every window. `None`: the CPU's answer.
    pub fn declare_number(&mut self, number: Option<u64>) {
        self.number = number;
    }

    /// The pending prompt through the translator's windows for `seat`, as the client gets them: the messages (the client's
    /// model fed them and checked), or None when the translator answers by itself.
    pub fn windows(&mut self, seat: usize) -> Option<Vec<yo3_proto::Message>> {
        let p = self.prompt().clone();
        let (translator, model) = &mut self.views[seat];
        let crate::translate::Prompt::Client(out) = translator.prompt(&p) else { return None };
        model.apply_all(&out);
        for problem in model.check(&self.state) {
            self.problems.push(format!("seat {seat}, step {}: {problem}", self.steps));
        }
        Some(out)
    }

    /// Answers the pending prompt the way the client does: `picks` in the card list (indexes), then the dialog's `result`
    /// (a Yes/No's 1 or 0, a list's pick count); the translator's answer goes to the engine.
    pub fn answer_in_windows(&mut self, seat: usize, picks: &[u32], result: u32) -> Result<(), String> {
        let translator = &mut self.views[seat].0;
        for &i in picks {
            translator.pick(i);
        }
        match translator.dialog_result(result) {
            Some(crate::translate::Prompt::Answer(a)) => self.respond(a),
            _ => Err("the window's result was not an answer".into()),
        }
    }

    /// The answer to the number question now pending, played through the translator's windows.
    fn number_through_windows(&mut self, target: u64) -> Result<Json, String> {
        let p = self.prompt().clone();
        let seat = p["player"].as_u64().unwrap_or(0) as usize & 1;
        let texts_of = |out: &[yo3_proto::Message]| -> HashMap<u32, String> {
            out.iter()
                .filter(|m| m.get("duelRequestName") == Some(&yo3_proto::Value::U32(0x7A12)))
                .filter_map(|m| Some((0x3F00 + m.get("slot")?.as_u32()?, m.get("text")?.as_str()?)))
                .collect()
        };
        let (translator, model) = &mut self.views[seat];
        let mut shown = translator.prompt(&p);
        for _ in 0..16 {
            let crate::translate::Prompt::Client(out) = shown else {
                let crate::translate::Prompt::Answer(a) = shown else { unreachable!() };
                return Ok(a);
            };
            self.dialogs += 1;
            model.apply_all(&out);
            for problem in model.check(&self.state) {
                self.problems.push(format!("seat {seat}, step {}: {problem}", self.steps));
            }
            let int = |m: &yo3_proto::Message, k: &str| m.get(k).and_then(yo3_proto::Value::as_u32).unwrap_or(0);
            let texts = texts_of(&out);
            let dialog = out.iter().rev().find(|m| int(m, "id") == cmd::DIALOG).ok_or("no dialog in the window")?;
            let result = if int(dialog, "param1") == crate::wire::dialog::NUMBER {
                // The number dialog: max << 24 | step << 20 | min << 16 | format, the number in its unit.
                let format = int(dialog, "param3") & 0xFFFF;
                let unit = match format { 302 => 1000, 303 => 100, _ => 1 };
                (target / unit) as u32
            } else {
                let ids: Vec<u32> = out.iter().filter(|m| int(m, "id") == cmd::LIST_VALUES).flat_map(|m| (1..=4).map(move |i| m.get(&format!("param{i}")).and_then(yo3_proto::Value::as_u32).unwrap_or(0))).filter(|&v| v != 0).map(|v| v & 0x7FFF).collect();
                let holds = |label: &str| {
                    let (lo, hi) = label.split_once('-').unwrap_or((label, label));
                    matches!((lo.parse::<u64>(), hi.parse::<u64>()), (Ok(lo), Ok(hi)) if (lo..=hi).contains(&target))
                };
                ids.iter().position(|id| texts.get(id).is_some_and(|t| holds(t))).ok_or_else(|| format!("{target} is in none of the labels {texts:?}"))? as u32
            };
            shown = translator.dialog_result(result).ok_or("the window's result was not taken")?;
        }
        Err(format!("{target} took more than 16 windows"))
    }

    /// The preferred cards among a card choice's options: as many as it needs, None when it offers too few of them.
    fn preferred(&self, p: &Json) -> Option<Json> {
        use crate::ocg::message::Ask;
        let Ok(Some(Ask::Card(c) | Ask::Tribute(c))) = Ask::lenient(p) else { return None };
        let tribute = p["type"] == msg::SELECT_TRIBUTE;
        let want = c.min.max(1) as usize;
        let mut picks: Vec<usize> = vec![];
        for &(team, cid) in &self.prefer {
            for (i, s) in c.selects.iter().enumerate() {
                if s.controller == team && s.cid == Some(cid as u64) && !picks.contains(&i) && picks.len() < want {
                    picks.push(i);
                }
            }
        }
        (picks.len() == want).then(|| Response::cards(if tribute { crate::ocg::resp::TRIBUTE } else { crate::ocg::resp::CARD }, Some(picks)).json())
    }

    /// The default answer: the opponent and the planned side between actions pass (end the turn from the Main Phase,
    /// leave the Battle Phase), every other prompt gets the CPU's answer.
    pub fn pass(&mut self) -> Result<(), String> {
        let p = self.prompt();
        let response = if p["type"] == msg::SELECT_IDLECMD {
            json!({"type": 1, "action": idle::TO_EP, "index": null})
        } else if p["type"] == msg::SELECT_BATTLECMD {
            json!({"type": 1, "action": battle::TO_EP, "index": null})
        } else if let (Some(wanted), Ok(Some(crate::ocg::message::Ask::Position(q)))) = (self.position, crate::ocg::message::Ask::lenient(p))
            && q.positions & wanted != 0
        {
            Response::Position(wanted).json()
        } else if let (Some(number), Ok(Some(crate::ocg::message::Ask::Number(_)))) = (self.number, crate::ocg::message::Ask::lenient(p)) {
            self.number_through_windows(number)?
        } else {
            self.preferred(p).unwrap_or_else(|| bot_answer(p))
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
                            return Err(format!("team {team}: {action:?} never became possible: {}\n{}", short(&p), self.context()));
                        }
                        self.pass()?;
                    }
                    (None, None) => self.pass()?,
                }
            } else if kind == msg::SELECT_BATTLECMD {
                match action {
                    Some(Action::Attack(c)) => {
                        let i = p["attacks"].as_array().and_then(|v| v.iter().position(|a| a["cid"] == c));
                        let Some(i) = i else { return Err(format!("team {team}: {c} can't attack: {}\n{}", short(&p), self.context())) };
                        next[team] += 1;
                        self.respond(json!({"type": 1, "action": battle::ATTACK, "index": i}))?;
                    }
                    _ => self.respond(json!({"type": 1, "action": battle::TO_M2, "index": null}))?,
                }
            } else if let (true, Some(Action::ChainOnAttack(c))) = (kind == msg::SELECT_CHAIN, action)
                && self.attacking
                && let Some(i) = found("selects", c)
            {
                next[team] += 1;
                self.respond(Response::Chain(Some(i)).json())?;
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
