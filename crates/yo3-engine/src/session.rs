//! One duel per process, spoken to in JSON lines: the protocol `engine/duelcore.ts` (the Node sidecar) set, answer for
//! answer. See main.rs for the requests.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use serde_json::{Map, Value as Json, json};

use crate::bot::{Pool, Setup, Style, Thinker};
use crate::cards::{Cards, Declarable, Library, Prints, is_card_script};
use crate::core::{Card, Duel, Host, Status};
use crate::decode::{self, Queried, Reader};

const MODE_MR2: u64 = 0xD0600;
const LOCATION_DECK: u32 = 0x01;
const LOCATION_HAND: u32 = 0x02;
const LOCATION_MZONE: u32 = 0x04;
const LOCATION_SZONE: u32 = 0x08;
const LOCATION_GRAVE: u32 = 0x10;
const LOCATION_REMOVED: u32 = 0x20;
const LOCATION_EXTRA: u32 = 0x40;
const POSITION_FACEDOWN_DEFENSE: u32 = 0x8;

const MSG_HINT: u64 = 2;
const HINT_SELECTMSG: u64 = 3;
const MSG_WIN: u64 = 5;
const MSG_SWAP_GRAVE_DECK: u64 = 35;
const MSG_SHUFFLE_SET_CARD: u64 = 36;
const MSG_MOVE: u64 = 50;
const MSG_ANNOUNCE_CARD: u64 = 142;

const MONSTER_STATS: u32 = decode::QUERY_POSITION | decode::QUERY_LEVEL | decode::QUERY_ATTACK | decode::QUERY_DEFENSE
    | decode::QUERY_BASE_ATTACK | decode::QUERY_BASE_DEFENSE | decode::QUERY_ATTRIBUTE | decode::QUERY_RACE;

/// What the core reports while a duel runs, besides its messages.
#[derive(Default)]
struct Log {
    /// `"<type>: <text>"`, as the sidecar listed the core's log.
    lines: Vec<String>,
    /// Card scripts (`c<passcode>.lua`) asked for and not found, when counting them (the card check).
    missing: Option<Vec<String>>,
}

struct DuelHost {
    library: Arc<Library>,
    log: Rc<RefCell<Log>>,
}

impl Host for DuelHost {
    fn card(&mut self, code: u32) -> Option<Card> {
        self.library.card(code)
    }
    fn script(&mut self, name: &str) -> Option<Arc<Vec<u8>>> {
        let text = self.library.script(name);
        if text.is_none() && is_card_script(name) && let Some(missing) = &mut self.log.borrow_mut().missing {
            missing.push(name.to_string());
        }
        text
    }
    fn log(&mut self, kind: i32, text: &str) {
        self.log.borrow_mut().lines.push(format!("{kind}: {text}"));
    }
}

pub struct Session {
    cards: Rc<RefCell<Cards>>,
    log: Rc<RefCell<Log>>,
    duel: Option<Duel>,
    prints: Prints,
    last_field: Option<Json>,
    /// How the duel began and every answer since, as the core read them: what the bot rebuilds copies from.
    setup: Arc<Setup>,
    history: Vec<Vec<u8>>,
    /// Copies of the duel kept in step for the bot.
    pool: Arc<Pool>,
    /// The prompt the duel awaits an answer to (as decoded, before Konami ids are added).
    pending: Option<Json>,
    /// Each player's last selection hint (HINT_SELECTMSG), and the ATK of the attacker the bot last declared.
    hints: [Option<u64>; 2],
    attacker: Option<i32>,
    /// The bot's last prompt and answer: the same prompt straight back means the answer went nowhere.
    thought: Option<(Json, Json)>,
}

type Answer = Result<Json, String>;

impl Session {
    pub fn new(cards: Cards) -> Session {
        Session {
            cards: Rc::new(RefCell::new(cards)),
            log: Default::default(),
            duel: None,
            prints: Prints::default(),
            last_field: None,
            setup: Arc::default(),
            history: vec![],
            pool: Arc::default(),
            pending: None,
            hints: [None; 2],
            attacker: None,
            thought: None,
        }
    }

    fn make_duel(&self, seed: [u64; 4], lp: u32, draw: u32, log: Rc<RefCell<Log>>) -> Result<Duel, String> {
        let host = DuelHost { library: self.cards.borrow().library.clone(), log };
        let mut duel = Duel::new(seed, MODE_MR2, lp, draw, Box::new(host)).ok_or("Error: the core refused the duel")?;
        for name in ["constant.lua", "utility.lua"] {
            let text = self.cards.borrow().script(name).unwrap_or_default();
            duel.load_script(name, &text);
        }
        Ok(duel)
    }

    /// One request line, answered with one line.
    pub fn handle(&mut self, line: &str) -> String {
        self.log.borrow_mut().lines.clear();
        let answer = serde_json::from_str::<Json>(line).map_err(|e| format!("SyntaxError: {e}")).and_then(|req| self.request(&req));
        let errors = std::mem::take(&mut self.log.borrow_mut().lines);
        match answer {
            Ok(Json::Object(mut m)) => {
                // identity and cards answer bare, the others list the core's errors.
                if m.remove("_bare").is_none() {
                    m.insert("errors".into(), json!(errors));
                }
                Json::Object(m).to_string()
            }
            Ok(other) => other.to_string(),
            Err(e) => json!({ "messages": [], "status": "error", "error": e, "errors": errors }).to_string(),
        }
    }

    fn request(&mut self, req: &Json) -> Answer {
        match req["op"].as_str() {
            Some("identity") => Ok(with_bare(crate::identity(&self.cards.borrow().vendor))),
            Some("new") => {
                let answer = self.new_duel(req)?;
                Ok(self.with_cids_answer(answer))
            }
            Some("respond") => {
                let r = &req["response"];
                let cards = self.cards.borrow();
                let response = if r["type"] == 18 && r.get("cid").is_some() {
                    json!({ "type": 18, "card": self.prints.declared(&cards, r["cid"].as_u64().unwrap_or(0) as u32) })
                } else {
                    r.clone()
                };
                drop(cards);
                let duel = self.duel.as_mut().ok_or("TypeError: no duel")?;
                let bytes = crate::encode::response(&response);
                duel.respond(&bytes);
                self.history.push(bytes);
                let answer = self.run()?;
                Ok(self.with_cids_answer(answer))
            }
            Some("cards") => Ok(with_bare(json!({ "status": "cards", "cards": self.check_cards(req)? }))),
            Some("state") => self.state(),
            Some("think") => self.think(req),
            other => Err(format!("Error: unknown op {}", other.map_or_else(|| req["op"].to_string(), String::from))),
        }
    }

    fn with_cids_answer(&self, (messages, status): (Vec<Json>, &str)) -> Json {
        let cards = self.cards.borrow();
        let messages: Vec<Json> = messages.into_iter().map(|m| with_cids(m, &cards, &self.prints)).collect();
        json!({ "messages": messages, "status": status })
    }

    fn new_duel(&mut self, req: &Json) -> Result<(Vec<Json>, &'static str), String> {
        let seed = req["seed"].as_u64().or_else(|| req["seed"].as_f64().map(|f| f as u64)).unwrap_or(0);
        let lp = req["lp"].as_u64().unwrap_or(8000) as u32;
        // A new duel replaces the old one.
        self.duel = None;
        let seeds = [seed | 1, seed ^ 0x9e3779b97f4a7c15, 2, 3];
        let duel = self.make_duel(seeds, lp, 5, self.log.clone())?;
        self.duel = Some(duel);
        self.last_field = None;
        self.prints = Prints::default();
        let mut setup = Setup { seed: seeds, lp, cards: vec![], skills: vec![] };
        self.history.clear();
        self.pool.reset();
        self.hints = [None; 2];
        self.attacker = None;
        let names = &req["names"];
        for (t, deck) in req["decks"].as_array().into_iter().flatten().enumerate() {
            for (key, location) in [("main", LOCATION_DECK), ("extra", LOCATION_EXTRA)] {
                for cid in deck[key].as_array().into_iter().flatten() {
                    let cid = cid.as_u64().unwrap_or(0) as u32;
                    let code = self.prints.resolve(&self.cards.borrow(), cid, names[cid.to_string()].as_str());
                    match code {
                        Some(code) if code != 0 => {
                            let duel = self.duel.as_mut().unwrap();
                            duel.new_card(t as u8, 0, code, t as u8, location, 0, POSITION_FACEDOWN_DEFENSE);
                            setup.cards.push((t as u8, location, code));
                        }
                        _ => self.log.borrow_mut().lines.push(format!("no passcode for cid {cid}")),
                    }
                }
            }
        }
        // Skills: {"player": team, "skill": name} (bot::skill_call).
        for s in req["skills"].as_array().into_iter().flatten() {
            let team = s["player"].as_u64().unwrap_or(0) as u8 & 1;
            match crate::bot::skill_call(s["skill"].as_str().unwrap_or_default(), team) {
                Some(call) => setup.skills.push((team, call)),
                None => self.log.borrow_mut().lines.push(format!("no skill {}", s["skill"])),
            }
        }
        crate::bot::give_skills(self.duel.as_mut().unwrap(), &setup);
        self.duel.as_mut().unwrap().start();
        self.setup = Arc::new(setup);
        self.run()
    }

    fn query(&mut self, flags: u32, controller: u8, location: u32, sequence: u32) -> Result<Option<Queried>, String> {
        let bytes = self.duel.as_mut().unwrap().query(flags, controller, location, sequence, 0);
        decode::query(&mut Reader::new(&bytes)).map_err(|e| e.to_string())
    }

    fn query_location(&mut self, flags: u32, controller: u8, location: u32) -> Result<Vec<Option<Queried>>, String> {
        let bytes = self.duel.as_mut().ok_or("TypeError: no duel")?.query_location(flags, controller, location);
        decode::query_location(&bytes).map_err(|e| e.to_string())
    }

    /// Runs the duel until it needs an answer or ends. The core keeps prompting after a win, so a win ends it here.
    fn run(&mut self) -> Result<(Vec<Json>, &'static str), String> {
        let (messages, status) = self.run_steps()?;
        for m in &messages {
            if m["type"] == MSG_HINT && m["hint_type"] == HINT_SELECTMSG {
                self.hints[m["player"].as_u64().unwrap_or(0) as usize & 1] = m["hint"].as_u64();
            }
        }
        self.pending = (status == "await").then(|| messages.last().cloned()).flatten();
        Ok((messages, status))
    }

    fn run_steps(&mut self) -> Result<(Vec<Json>, &'static str), String> {
        let mut messages = vec![];
        loop {
            let duel = self.duel.as_mut().ok_or("TypeError: no duel")?;
            let status = duel.process();
            let bytes = duel.messages();
            let mut batch = decode::messages(&bytes).map_err(|e| e.to_string())?;
            self.add_reasons(&mut batch)?;
            self.fix_shuffles(&mut batch)?;
            self.fix_grave_swaps(&mut batch)?;
            self.add_fits(&mut batch);
            let waiting = status == Status::Awaiting;
            // Before the prompt, which stays last.
            let field = if waiting && batch.is_empty() { None } else { self.field_report()? };
            if let Some(field) = field {
                let at = if waiting { batch.len() - 1 } else { batch.len() };
                batch.insert(at, field);
            }
            if let Some(win) = batch.iter().position(|m| m["type"] == MSG_WIN) {
                batch.truncate(win + 1);
                messages.extend(batch);
                return Ok((messages, "end"));
            }
            messages.extend(batch);
            match status {
                Status::End => return Ok((messages, "end")),
                Status::Awaiting => return Ok((messages, "await")),
                Status::Continue => {}
            }
        }
    }

    /// MSG_MOVE's `reason`, which the wrapper's parse drops: read back from the card at its new place, as the place
    /// reads (an Xyz material's is its Xyz Monster's: see decode::place); 0 when there is none to ask.
    fn add_reasons(&mut self, batch: &mut [Json]) -> Result<(), String> {
        for m in batch.iter_mut().filter(|m| m["type"] == MSG_MOVE) {
            let to = &m["to"];
            let location = to["location"].as_u64().unwrap_or(0) as u32;
            let reason = if location != 0 && location & decode::LOCATION_OVERLAY == 0 {
                let (c, s) = (to["controller"].as_u64().unwrap_or(0) as u8, to["sequence"].as_u64().unwrap_or(0) as u32);
                self.query(decode::QUERY_REASON, c, location, s)?.and_then(|q| q.get(decode::QUERY_REASON)).unwrap_or(0)
            } else {
                0
            };
            m["reason"] = reason.into();
        }
        Ok(())
    }

    /// MSG_SHUFFLE_SET_CARD gets `places`: each shuffled zone and the card it holds now (the core's lists give only the
    /// old places; the wrapper's parse pairs them up as {from, to}).
    fn fix_shuffles(&mut self, batch: &mut [Json]) -> Result<(), String> {
        for m in batch.iter_mut().filter(|m| m["type"] == MSG_SHUFFLE_SET_CARD) {
            let Some(cards) = m["cards"].as_array() else { continue };
            let zones: Vec<Json> = cards.iter().flat_map(|c| [c["from"].clone(), c["to"].clone()]).take(cards.len())
                .filter(|l| l["location"].as_u64().unwrap_or(0) != 0).collect();
            let mut places = vec![];
            for l in zones {
                let (c, loc, s) = (l["controller"].as_u64().unwrap_or(0), l["location"].as_u64().unwrap_or(0), l["sequence"].as_u64().unwrap_or(0));
                let q = self.query(decode::QUERY_CODE | decode::QUERY_POSITION, c as u8, loc as u32, s as u32)?;
                let get = |f| q.as_ref().and_then(|q| q.get(f)).unwrap_or(0);
                places.push(json!({ "controller": c, "location": loc, "sequence": s, "position": get(decode::QUERY_POSITION), "code": get(decode::QUERY_CODE) }));
            }
            m["places"] = Json::Array(places);
        }
        Ok(())
    }

    /// MSG_SWAP_GRAVE_DECK gets the player's `grave` and `extra` as they are after the step, in sequence order.
    fn fix_grave_swaps(&mut self, batch: &mut [Json]) -> Result<(), String> {
        for m in batch.iter_mut().filter(|m| m["type"] == MSG_SWAP_GRAVE_DECK) {
            let player = m["player"].as_u64().unwrap_or(0) as u8;
            for (key, location) in [("grave", LOCATION_GRAVE), ("extra", LOCATION_EXTRA)] {
                let codes: Vec<Json> = self.query_location(decode::QUERY_CODE, player, location)?.into_iter().flatten()
                    .map(|q| json!({ "code": q.get(decode::QUERY_CODE).unwrap_or(0) })).collect();
                m[key] = Json::Array(codes);
            }
        }
        Ok(())
    }

    /// ANNOUNCE_CARD gets `fits`: Konami ids of cards its filter allows, this duel's cards first, at most 30.
    fn add_fits(&mut self, batch: &mut [Json]) {
        for m in batch.iter_mut().filter(|m| m["type"] == MSG_ANNOUNCE_CARD) {
            let Some(opcodes) = m["opcodes"].as_array() else { continue };
            let opcodes: Vec<u64> = opcodes.iter().map(|o| o.as_u64().unwrap_or(0)).collect();
            let own = self.prints.own();
            let mut cards = self.cards.borrow_mut();
            let all = cards.declarables();
            let fit = |c: &&Declarable| declarable(c, &opcodes);
            let first: Vec<u32> = all.iter().filter(|c| own.contains(&c.code)).filter(fit).map(|c| c.code).collect();
            let room = 30usize.saturating_sub(first.len());
            let rest: Vec<u32> = all.iter().filter(|c| !own.contains(&c.code)).filter(fit).take(room).map(|c| c.code).collect();
            drop(cards);
            let cards = self.cards.borrow();
            let fits: Vec<u32> = first.into_iter().chain(rest).filter_map(|c| self.prints.cid(&cards, c)).filter(|c| *c != 0).collect();
            m["fits"] = json!(fits);
        }
    }

    /// Both players' monsters as they are now, `{type: "field", monsters: [...]}`, or None if nothing changed since
    /// the last report: effects change stats without a message of their own.
    fn field_report(&mut self) -> Result<Option<Json>, String> {
        let mut monsters = vec![];
        for controller in 0..2u8 {
            for (sequence, card) in self.query_location(MONSTER_STATS, controller, LOCATION_MZONE)?.into_iter().enumerate() {
                let Some(card) = card else { continue };
                let get = |f| card.get(f).map_or(Json::Null, Json::from);
                monsters.push(json!({ "controller": controller, "sequence": sequence, "position": get(decode::QUERY_POSITION),
                    "level": get(decode::QUERY_LEVEL), "attack": get(decode::QUERY_ATTACK), "defense": get(decode::QUERY_DEFENSE),
                    "base_attack": get(decode::QUERY_BASE_ATTACK), "base_defense": get(decode::QUERY_BASE_DEFENSE),
                    "attribute": get(decode::QUERY_ATTRIBUTE), "race": get(decode::QUERY_RACE) }));
            }
        }
        let monsters = Json::Array(monsters);
        if self.last_field.as_ref() == Some(&monsters) {
            return Ok(None);
        }
        self.last_field = Some(monsters.clone());
        Ok(Some(json!({ "type": "field", "monsters": monsters })))
    }

    /// The bot's answer to the pending prompt (`{"op":"think","style":{…}}`, see bot::Style), for the server to give
    /// with `respond`. With `"policy":"simple"`, the old placeholder CPU's answer instead (a baseline to measure by).
    fn think(&mut self, req: &Json) -> Answer {
        let prompt = self.pending.clone().ok_or("Error: no prompt to answer")?;
        let me = prompt["player"].as_u64().unwrap_or(0) as usize & 1;
        let started = std::time::Instant::now();
        let response = if req["policy"] == "simple" {
            crate::bot::policy::simple(&prompt)
        } else {
            let library = self.cards.borrow().library.clone();
            let style = Style::from_json(&req["style"]);
            let deadline = std::time::Instant::now() + crate::bot::BUDGET;
            let thinker = Thinker { library: &library, setup: &self.setup, history: &self.history, pool: &self.pool, style, deadline };
            let duel = self.duel.as_mut().ok_or("TypeError: no duel")?;
            let (response, declared) = thinker.think(duel, &prompt, self.hints[me], self.attacker);
            // Copies for the next decision, built while the duel goes on.
            self.pool.refill(&library, &self.setup, &self.history);
            self.attacker = declared.or(self.attacker.filter(|_| prompt["type"] != crate::bot::policy::MSG_SELECT_BATTLECMD));
            // The same prompt straight back after the same answer (an attack cancelled, an activation that came to
            // nothing): move on instead of trying it forever.
            let response = match &self.thought {
                Some((last, answer)) if *last == prompt && *answer == response => crate::bot::policy::move_on(&prompt),
                _ => response,
            };
            self.thought = Some((prompt, response.clone()));
            response
        };
        Ok(json!({ "status": "thought", "response": response, "ms": started.elapsed().as_millis() as u64 }))
    }

    /// Where every card is, by team (see the sidecar's `state()`).
    fn state(&mut self) -> Answer {
        let flags = decode::QUERY_CODE | MONSTER_STATS | decode::QUERY_COUNTERS;
        let field = self.duel.as_mut().ok_or("TypeError: no duel")?.query_field();
        // The core's LP is signed; a lethal attack may leave it negative before WIN.
        let players = decode::field(&mut Reader::new(&field)).map_err(|e| e.to_string())?.remove("players").unwrap_or_default();
        let lps: Vec<i32> = (0..2).map(|t| players[t]["lp"].as_u64().unwrap_or(0) as u32 as i32).collect();
        let mut teams = vec![];
        for t in 0..2u8 {
            let mut at = |location| -> Result<Vec<Json>, String> {
                Ok(self.query_location(flags, t, location)?.into_iter().map(|c| c.map_or(Json::Null, |c| {
                    let get = |f| c.get(f).unwrap_or(0);
                    let counters: Map<String, Json> = counter_counts(&c.counters).into_iter().map(|(k, v)| (k.to_string(), v.into())).collect();
                    json!({ "code": get(decode::QUERY_CODE), "position": get(decode::QUERY_POSITION), "level": get(decode::QUERY_LEVEL),
                        "attack": get(decode::QUERY_ATTACK), "defense": get(decode::QUERY_DEFENSE), "base_attack": get(decode::QUERY_BASE_ATTACK),
                        "base_defense": get(decode::QUERY_BASE_DEFENSE), "counters": counters })
                })).collect())
            };
            let list = |v: Vec<Json>| -> Vec<Json> { v.into_iter().filter(|c| !c.is_null()).collect() };
            let hand = list(at(LOCATION_HAND)?);
            let mzone = at(LOCATION_MZONE)?;
            let szone = at(LOCATION_SZONE)?;
            let grave = list(at(LOCATION_GRAVE)?);
            let removed = list(at(LOCATION_REMOVED)?);
            let extra = list(at(LOCATION_EXTRA)?);
            let deck = self.duel.as_mut().unwrap().count(t, LOCATION_DECK);
            teams.push(json!({ "lp": lps[t as usize].max(0), "hand": hand, "mzone": mzone, "szone": szone, "grave": grave,
                "removed": removed, "extra": extra, "deck": deck }));
        }
        let cards = self.cards.borrow();
        let teams = with_cids(Json::Array(teams), &cards, &self.prints);
        Ok(json!({ "status": "state", "teams": teams }))
    }

    /// What the core makes of each card `{cid, name}` (the server's card check): its passcode (null when the engine
    /// has no such card), the card data it reads, and the errors and missing scripts loading it reported. The cards
    /// are made in one throwaway duel; the running duel, if any, is left alone.
    fn check_cards(&mut self, req: &Json) -> Result<Json, String> {
        let log: Rc<RefCell<Log>> = Default::default();
        log.borrow_mut().missing = Some(vec![]);
        let mut scratch = self.make_duel([1, 2, 3, 4], 8000, 0, log.clone())?;
        let mut out = vec![];
        for card in req["cards"].as_array().into_iter().flatten() {
            let cid = card["cid"].as_u64().unwrap_or(0) as u32;
            let mut prints = Prints::default();
            let cards = self.cards.borrow();
            let Some(code) = prints.resolve(&cards, cid, card["name"].as_str()) else {
                out.push(json!({ "cid": cid, "passcode": null }));
                continue;
            };
            drop(cards);
            log.borrow_mut().lines.clear();
            log.borrow_mut().missing = Some(vec![]);
            scratch.new_card(0, 0, code, 0, LOCATION_DECK, 0, POSITION_FACEDOWN_DEFENSE);
            let cards = self.cards.borrow();
            let data = cards.data(code).map_or(Json::Null, |[kind, level, attack, defense, attribute, race, alias]| {
                let mut d = json!({ "type": kind, "level": level, "attack": attack, "defense": defense, "attribute": attribute, "race": race, "alias": alias });
                if let Some(name) = cards.name(code) {
                    d["name"] = name.into();
                }
                d
            });
            let mut log = log.borrow_mut();
            out.push(json!({ "cid": cid, "passcode": code, "data": data, "missing_scripts": log.missing.take().unwrap_or_default(),
                "errors": std::mem::take(&mut log.lines) }));
        }
        drop(scratch);
        Ok(Json::Array(out))
    }
}

fn with_bare(v: Json) -> Json {
    let mut v = v;
    v["_bare"] = true.into();
    v
}

/// A card's counters as {counter type: count}, the types with none dropped.
fn counter_counts(counters: &[(u16, u16)]) -> BTreeMap<u16, u16> {
    counters.iter().filter(|(_, n)| *n != 0).map(|(t, n)| (*t, *n)).collect()
}

/// Adds `cid` next to every numeric `code` (or `card`, as MOVE names it) and `cids` next to a list of codes, so the
/// server never sees passcodes (and `name` when there is no Konami id, as for tokens), `description_cid` next to a
/// `description` that is a card's script string (code << 20 | n), and `option_cids` next to a list of `options`.
pub fn with_cids(value: Json, cards: &Cards, prints: &Prints) -> Json {
    match value {
        Json::Array(items) => Json::Array(items.into_iter().map(|v| with_cids(v, cards, prints)).collect()),
        Json::Object(m) => {
            let mut out: Map<String, Json> = m.into_iter().map(|(k, v)| (k, with_cids(v, cards, prints))).collect();
            let code = out.get("code").filter(|c| c.is_number()).or_else(|| out.get("card")).and_then(Json::as_u64);
            if let Some(code) = code {
                let cid = prints.cid(cards, code as u32).unwrap_or(0);
                out.insert("cid".into(), cid.into());
                if code != 0 && cid == 0 && let Some(name) = cards.name(code as u32) {
                    out.insert("name".into(), name.into());
                }
            }
            let cids = out.get("cards").and_then(Json::as_array).filter(|c| c.iter().all(Json::is_number))
                .map(|c| c.iter().map(|c| prints.cid(cards, c.as_u64().unwrap_or(0) as u32).unwrap_or(0)).collect::<Vec<_>>());
            if let Some(cids) = cids {
                out.insert("cids".into(), json!(cids));
            }
            if let Some(owner) = out.get("description").and_then(Json::as_u64).and_then(|d| cards.cid_of.get(&((d >> 20) as u32))) {
                out.insert("description_cid".into(), (*owner).into());
            }
            let options = out.get("options").and_then(Json::as_array).filter(|o| o.iter().all(Json::is_number))
                .map(|o| o.iter().map(|o| cards.cid_of.get(&((o.as_u64().unwrap_or(0) >> 20) as u32)).copied().unwrap_or(0)).collect::<Vec<_>>());
            if let Some(options) = options {
                out.insert("option_cids".into(), json!(options));
                // What each option says (the card's string; empty for the core's own descriptions).
                let texts: Vec<String> = out["options"].as_array().into_iter().flatten()
                    .map(|o| o.as_u64().unwrap_or(0))
                    .map(|d| cards.string((d >> 20) as u32, (d & 0xF_FFFF) as u32).filter(|_| d >> 20 != 0).unwrap_or_default())
                    .collect();
                out.insert("option_texts".into(), json!(texts));
            }
            Json::Object(out)
        }
        v => v,
    }
}

const OP: u64 = 0x4000000000000000;
const fn op(n: u64) -> u64 {
    OP | n << 32
}
const TYPE_MONSTER: i128 = 0x1;
const TYPE_TOKEN: i128 = 0x4000;
const MARINE_DOLPHIN: u32 = 78734254;
const TWINKLE_MOSS: u32 = 13857930;

/// EDOPro's card-name filter (ANNOUNCE_CARD's `opcodes`, card_data::is_declarable): a stack program over a card.
pub fn declarable(card: &Declarable, opcodes: &[u64]) -> bool {
    let mut stack: Vec<i128> = vec![];
    let (mut aliases, mut tokens) = (false, false);
    let two = |s: &mut Vec<i128>, f: &dyn Fn(i128, i128) -> i128| {
        if s.len() >= 2 {
            let r = s.pop().unwrap();
            let l = s.pop().unwrap();
            s.push(f(l, r));
        }
    };
    let one = |s: &mut Vec<i128>, f: &dyn Fn(i128) -> i128| {
        if let Some(v) = s.pop() {
            s.push(f(v));
        }
    };
    let b = |x: bool| x as i128;
    let setcard = |set: i128| card.setcodes.iter().any(|s| (*s as i128 & 0xfff) == (set & 0xfff) && (*s as i128 & 0xf000 & set) == (set & 0xf000));
    let shift = |r: i128| if (0..128).contains(&r) { r as u32 } else { 127 };
    for &o in opcodes {
        match o {
            x if x == op(0x0) => two(&mut stack, &|l, r| l.wrapping_add(r)),
            x if x == op(0x1) => two(&mut stack, &|l, r| l.wrapping_sub(r)),
            x if x == op(0x2) => two(&mut stack, &|l, r| l.wrapping_mul(r)),
            x if x == op(0x3) => two(&mut stack, &|l, r| if r != 0 { l.wrapping_div(r) } else { 0 }),
            x if x == op(0x4) => two(&mut stack, &|l, r| b(l != 0 && r != 0)),
            x if x == op(0x5) => two(&mut stack, &|l, r| b(l != 0 || r != 0)),
            x if x == op(0x6) => one(&mut stack, &|v| v.wrapping_neg()),
            x if x == op(0x7) => one(&mut stack, &|v| b(v == 0)),
            x if x == op(0x8) => two(&mut stack, &|l, r| l & r),
            x if x == op(0x9) => two(&mut stack, &|l, r| l | r),
            x if x == op(0x10) => one(&mut stack, &|v| !v),
            x if x == op(0x11) => two(&mut stack, &|l, r| l ^ r),
            x if x == op(0x12) => two(&mut stack, &|l, r| l.checked_shl(shift(r)).unwrap_or(0)),
            x if x == op(0x13) => two(&mut stack, &|l, r| l >> shift(r)),
            x if x == op(0x14) => aliases = true,
            x if x == op(0x15) => tokens = true,
            x if x == op(0x100) => one(&mut stack, &|v| b(card.code as i128 == v)),
            x if x == op(0x101) => one(&mut stack, &|v| b(setcard(v))),
            x if x == op(0x102) => one(&mut stack, &|v| card.kind as i128 & v),
            x if x == op(0x103) => one(&mut stack, &|v| card.race as i128 & v),
            x if x == op(0x104) => one(&mut stack, &|v| card.attribute as i128 & v),
            x if x == op(0x105) => stack.push(card.code as i128),
            x if x == op(0x107) => stack.push(card.kind as i128),
            x if x == op(0x108) => stack.push(card.race as i128),
            x if x == op(0x109) => stack.push(card.attribute as i128),
            x => stack.push(x as i128),
        }
    }
    if stack.len() != 1 || stack[0] == 0 {
        return false;
    }
    let token = (card.kind as i128 & (TYPE_MONSTER | TYPE_TOKEN)) == (TYPE_MONSTER | TYPE_TOKEN);
    card.code == MARINE_DOLPHIN || card.code == TWINKLE_MOSS || ((aliases || card.alias == 0) && (tokens || !token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_drop_the_empty_types_and_keep_two_types_with_the_same_count() {
        assert_eq!(counter_counts(&[(0x1, 2), (0x2, 2), (0x3, 0)]), BTreeMap::from([(1, 2), (2, 2)]));
    }

    #[test]
    fn a_name_filter_allows_the_cards_it_names() {
        let card = Declarable { code: 89631139, alias: 0, setcodes: vec![0xdd], kind: 0x11, race: 0x2000, attribute: 0x10 };
        assert!(declarable(&card, &[89631139, op(0x100)]));
        assert!(!declarable(&card, &[46986414, op(0x100)]));
        // A Dragon (race 0x2000) that is a Monster.
        assert!(declarable(&card, &[0x2000, op(0x103), 0x1, op(0x102), op(0x4)]));
        assert!(declarable(&card, &[0xdd, op(0x101)]));
    }
}
