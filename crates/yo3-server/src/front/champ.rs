//! The Championship's part of the hub (#17, [`crate::championship`]): its entry on the map's list (`0x177A`), entering its
//! lobby (`0x1784`), the lobby before the draw, the entry draw (`0x32D0`, `0x32D1`), the Final's rounds, the titles,
//! the prizes and the record Duel Log lists (`0x1793`). The bracket under way is an [`Event`] with a [`Tag`], run by the
//! same code as the user events (`events.rs`, `next_round`, `end_match`), with its 16 seats fixed at the draw.
use std::time::{Duration, Instant};

use yo3_proto::{Message, Value};

use super::{Hub, Match, SINGLE_MODE, chat_note};
use crate::championship::{self as champ, Championship, ChampionshipState as State, Entry, FINAL_ID, SEATS};
use crate::events::{Event, FINISHED, RUNNING, Tag};
use crate::lock::Locked;
use crate::mail::Mail;

/// `status` of the Championship's item on the map's list (TournamentScene 0x4CBD70 keeps the first with `event_type` 10
/// and this status).
const LISTED: u32 = 12;
/// `0x32D1`'s `lot`: drawn into the bracket, and not (**guess**: the client shows its text 15361 for 1 and 15362 for any
/// other value; what 15363 and 15364 are for is unknown, see docs/protocol.md).
const LOT_IN: u32 = 1;
const LOT_OUT: u32 = 0;
/// A Championship the server could not start within this long of its start time (it was down) is cancelled (**guess**).
const MISSED: i64 = 3600;

fn jst_now() -> String {
    (chrono::Utc::now() + chrono::Duration::hours(9)).format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Hub {
    pub fn set_draw_delay(&mut self, delay: Duration) {
        self.champ_draw_delay = delay;
    }

    /// Keeps the Championships in `dir`'s database. A bracket that was running when the server stopped cannot go on (its
    /// duels and lobbies died with it): it ends without a winner. The finished ones' brackets come back for Duel Log.
    pub fn keep_championships(&mut self, dir: &std::path::Path) {
        self.championships = champ::Championships::at(dir);
        let now = chrono::Utc::now().timestamp();
        for id in self.championships.list.iter().filter(|c| c.state == State::Running).map(|c| c.id).collect::<Vec<_>>() {
            if let Some(c) = self.championships.list.iter_mut().find(|c| c.id == id) {
                (c.state, c.finished) = (State::Finished, now);
            }
            if let Err(e) = self.championships.save(id) { tracing::warn!("Championship {id} not saved: {e}") }
        }
        let brackets: Vec<Event> = self.championships.list.iter().filter_map(|c| c.bracket.as_ref().and_then(Event::from_json)).collect();
        self.events.retain(|e| e.champ.is_none());
        self.events.extend(brackets.into_iter().map(|mut e| { e.status = FINISHED; e }));
    }

    /// The Championship's item in `0x177B` (read by 0x4CBD70: the first with `event_type` 10 and `status` 12). `official` 0, so the
    /// Stadium (which keeps `official` items only) does not list it.
    pub(super) fn championship_item(&self, now: i64) -> Option<Message> {
        let c = self.championships.current(now)?;
        Some(self.champ_fields(c, now))
    }

    fn champ_fields(&self, c: &Championship, _now: i64) -> Message {
        Message::record()
            .with("event_id", Value::U32(c.event_id()))
            .with("official", Value::U32(0))
            .with("event_type", Value::U32(10))
            .with("status", Value::U32(LISTED))
            .with("event_name", Value::text(&c.name))
            .with("event_start", Value::text(&crate::official::jst(c.entry_open)))
            .with("event_end", Value::text(&crate::official::jst(c.start)))
            .with("player_num", Value::U32(c.entrants.len() as u32))
            .with("year", Value::U32(c.year))
            .with("season", Value::U32(c.season))
    }

    /// `0x177F` for a Championship: its row, with the host as nobody (player 0).
    pub(super) fn champ_row(&self, event_id: u32) -> Message {
        match self.championships.by_event(event_id) {
            Some(c) => Message::reply(0x1780).item(self.champ_fields(c, 0).with("player_id", Value::U32(0)).with("pname", Value::text("Host"))),
            None => Message::reply(0x1781).with("error_code", Value::U32(0)),
        }
    }

    /// Duel Log's past Championships (`0x1793` -> `0x177B`, `{year, season, event_id}`): the finished ones, newest first.
    pub(super) fn championship_past(&self) -> Message {
        let past = self.championships.past();
        past.iter().fold(Message::reply(0x177B).with("result_number", Value::U32(past.len() as u32)), |m, c| {
            m.item(Message::record().with("year", Value::U32(c.year)).with("season", Value::U32(c.season)).with("event_id", Value::U32(c.event_id())).with("event_name", Value::text(&c.name)))
        })
    }

    /// How many duels a player has played (their records against players and NPCs together).
    fn duels_played(&self, player: u32) -> u32 {
        self.records.iter().filter(|((me, _), _)| *me == player).map(|(_, r)| r.iter().sum::<u32>()).sum()
    }

    /// `0x1784` for a Championship's id: opening its lobby. While entries are open that enters the player (the client
    /// offers no other way in, see docs/protocol.md); later anyone may look in. None when `event_id` is not a Championship's.
    pub(super) fn championship_join(&mut self, event_id: u32, player: u32, name: &str) -> Option<Message> {
        let c = self.championships.by_event(event_id)?;
        if c.state == State::Cancelled { return Some(Message::reply(0x1786).with("error_code", Value::U32(0))) }
        let duels = self.duels_played(player);
        match self.championships.enter(event_id, player, name, duels, chrono::Utc::now().timestamp()) {
            Some(Entry::TooFewDuels(need)) => {
                let text = format!("The Championship asks for at least {need} duels played; you have played {duels}.");
                self.push(player, &chat_note("Championship", &text));
                return Some(Message::reply(0x1786).with("error_code", Value::U32(0)));
            }
            Some(Entry::In) => self.champ_entered(event_id, player),
            Some(Entry::Closed) | None => {}
        }
        self.last_join += 1;
        Some(Message::reply(0x1785)
            .with("join_id", Value::U32(self.last_join))
            .with("server_id", Value::U32(crate::official::SERVER_ID))
            .with("event_score", Value::U32(0))
            .with("event_id", Value::U32(event_id)))
    }

    /// A player entered (or looked in): the others' lobbies ask for the entries and counts again.
    fn champ_entered(&self, event_id: u32, player: u32) {
        let Some(c) = self.championships.by_event(event_id).filter(|c| c.state == State::Scheduled) else { return };
        for (p, _) in c.entrants.iter().filter(|(p, _)| *p != player) {
            for flag in [2, 5] {
                self.push(*p, &Message::reply(0x32D4).with("flag", Value::U32(flag)));
            }
        }
    }

    /// Whether a player is in a Championship still to be played (they may not create a user event meanwhile).
    pub(super) fn champ_busy(&self, player: u32) -> bool {
        self.championships.list.iter().any(|c| matches!(c.state, State::Scheduled | State::Running) && c.has_entered(player))
    }

    /// The lobby's requests before the draw: the bracket does not exist yet, so the entries, the countdown to the
    /// start and an empty map (the lobby shows the entrants; the seats come with the draw).
    pub(super) fn champ_waiting(&self, _me: u32, id: u32, event_id: u32, start: u32) -> Message {
        let Some(c) = self.championships.by_event(event_id) else { return Message::reply(id + 1).with("error_code", Value::U32(0)) };
        let tournament = |m: Message| m.with("tournament_id", Value::U32(FINAL_ID)).with("championship_id", Value::U32(c.id)).with("year", Value::U32(c.year)).with("season", Value::U32(c.season));
        match id {
            0x177F => self.champ_row(event_id),
            0x32C9 => tournament(Message::reply(0x32FB)
                .with("event_id", Value::U32(event_id))
                .with("status", Value::U32(if c.state == State::Cancelled { 8 } else { 0 }))
                .with("phase", Value::U32(0))
                .with("round", Value::U32(0))
                .with("total_round", Value::U32(0))
                .with("winner", Value::U32(0))
                .with("winner_name", Value::text(""))
                .with("second", Value::U32(0))
                .with("second_name", Value::text(""))),
            0x32CA => c.entrants.iter().skip(start as usize).fold(Message::reply(0x32FD).with("total_num", Value::U32(c.entrants.len() as u32)), |m, (p, name)| {
                let online = self.online.values().find(|o| o.player_id == *p);
                let field = |k: &str| online.and_then(|o| o.record.get(k).cloned());
                let item = Message::record().with("player_id", Value::U32(*p)).with("player_name", Value::text(name)).with("remain", Value::U32(1));
                let item = field("avatar1").into_iter().fold(item, |i, a| i.with("avatar1", a));
                m.item(field("country").into_iter().fold(item, |i, v| i.with("country", v)))
            }),
            0x32CC => Message::reply(0x3301).with("timer", Value::U32((c.start - chrono::Utc::now().timestamp()).clamp(0, 86_400) as u32)),
            0x32CD => Message::reply(0x3303).with("alive_num", Value::U32(0)).with("entry_num", Value::U32(c.entrants.len() as u32)).with("watch_num", Value::U32(0)),
            0x32CE => ["phase_block1", "phase_block2", "phase_block3"].into_iter().fold(Message::reply(0x3305).with("remain", Value::U32(0)), |m, k| m.with(k, Value::U32(u32::MAX))),
            0x32D5 => Message::reply(0x3309).with("map_data", Value::Bin(vec![0; crate::events::MAP_BLOCKS])),
            0x32CB => Message::reply(0x32FF),
            0x178E => Message::reply(0x178F),
            _ => Message::reply(id + 1).with("error_code", Value::U32(0)),
        }
    }

    /// The Championships whose time has come (called every second): the draw is announced (`0x32D0` status 1, "Selecting
    /// players for entry..."), and a moment later made. Fewer than two entrants cancels the Championship.
    pub fn tick_championships(&mut self, now: i64, at: Instant) {
        let due: Vec<u32> = self.championships.list.iter()
            .filter(|c| c.state == State::Scheduled && now >= c.start && !self.champ_drawing.contains_key(&c.id)).map(|c| c.id).collect();
        for id in due {
            let Some(c) = self.championships.get(id).cloned() else { continue };
            if now - c.start > MISSED {
                self.champ_cancel(id, "The server was not running when it was to start.");
            } else if c.entrants.len() < 2 {
                self.champ_cancel(id, "Fewer than two players entered.");
            } else {
                let announce = Message::reply(0x32D0).with("phase", Value::U32(1)).with("status", Value::U32(1));
                for (p, _) in &c.entrants {
                    self.push(*p, &announce);
                }
                self.champ_drawing.insert(id, at + self.champ_draw_delay);
            }
        }
        let ready: Vec<u32> = self.champ_drawing.iter().filter(|(_, t)| **t <= at).map(|(id, _)| *id).collect();
        for id in ready {
            self.champ_drawing.remove(&id);
            self.champ_draw(id);
        }
    }

    fn champ_mail(&mut self, to: (u32, String), title: String, body: String) {
        self.mail.post(Mail { from: (0, "Host".into()), to: (to.0, to.1), title, body, sent: jst_now(), ..Mail::default() });
        self.push(to.0, &Message::reply(0x2581));
    }

    /// Cancels a Championship waiting for its start, telling its entrants in a D-Mail.
    fn champ_cancel(&mut self, id: u32, why: &str) {
        let Some(at) = self.championships.list.iter().position(|c| c.id == id) else { return };
        let c = &mut self.championships.list[at];
        c.state = State::Cancelled;
        let (name, entrants) = (c.name.clone(), c.entrants.clone());
        if let Err(e) = self.championships.save(id) { tracing::warn!("Championship {id} not saved: {e}") }
        tracing::info!("Championship {id} \"{name}\" is cancelled: {why}");
        for entrant in entrants {
            self.champ_mail(entrant, format!("{name} cancelled"), format!("The Championship \"{name}\" was cancelled. {why}"));
        }
    }

    /// The entry draw: the lots (`0x32D1`), then the 16-seat bracket and the first round. More than 16 entrants: 16 are
    /// drawn at random and the others are told they could not enter.
    fn champ_draw(&mut self, id: u32) {
        let Some(c) = self.championships.get(id).cloned().filter(|c| c.state == State::Scheduled) else { return };
        let (drawn, out) = champ::draw(&c.entrants);
        let closed = Message::reply(0x32D0).with("phase", Value::U32(1)).with("status", Value::U32(0));
        for (p, _) in &c.entrants {
            self.push(*p, &closed);
        }
        for (p, _) in &drawn {
            self.push(*p, &Message::reply(0x32D1).with("lot", Value::U32(LOT_IN)));
        }
        for entrant in out {
            self.push(entrant.0, &Message::reply(0x32D1).with("lot", Value::U32(LOT_OUT)));
            let body = format!("The Championship \"{}\" had more entrants than the Final's {SEATS} seats, and the draw did not pick you.", c.name);
            self.champ_mail(entrant, format!("{} entry draw", c.name), body);
        }
        let settings = self.settings.as_ref().map(|s| s.now());
        let number = |v: Option<u32>, default: u32| Value::U32(v.filter(|v| *v > 0).unwrap_or(default));
        let fields = vec![
            ("match_type".to_string(), Value::U32(SINGLE_MODE)),
            ("regulation_id".to_string(), Value::U32(settings.as_ref().map_or(0, |s| s.login_regulation))),
            ("duel_lp_start".to_string(), number(settings.as_ref().map(|s| s.start_lp), 8000)),
            ("duel_surrender".to_string(), Value::U32(1)),
            ("duel_time_turn".to_string(), number(settings.as_ref().map(|s| s.turn_seconds), 180)),
            ("duel_time_total".to_string(), number(settings.as_ref().map(|s| s.total_seconds), 600)),
            ("duel_watch".to_string(), Value::U32(1)),
        ];
        let mut event = Event::new(c.event_id(), (0, "Host".into()), fields);
        event.champ = Some(Tag { championship: id, year: c.year, season: c.season });
        event.status = RUNNING;
        event.slots = champ::seat(&drawn);
        event.alive = drawn.iter().map(|(p, _)| *p).collect();
        event.entrants = drawn;
        self.events.retain(|e| e.id != c.event_id());
        self.events.push(event);
        if let Some(c) = self.championships.list.iter_mut().find(|c| c.id == id) { c.state = State::Running }
        if let Err(e) = self.championships.save(id) { tracing::warn!("Championship {id} not saved: {e}") }
        let at = self.events.len() - 1;
        self.next_round(at, true);
    }

    /// Pairs the Final's next round. The seats are a tree: in round r the seats form groups of 2^(r-1), neighbours
    /// meeting, and a group's player is the one of its seats still in. A pair with one side empty is a bye (the other
    /// goes on unopposed and counts a round won); two players get the match push `0x1978` as in a user event, with
    /// `event_type` 10 and `official` 1 in `duelData` (watch mode 6, BGM 0x22). A player no longer online forfeits.
    pub(super) fn start_champ_round(&mut self, at: usize) {
        self.events[at].round_number += 1;
        self.events[at].dueling = false;
        let round = self.events[at].round_number;
        if round > self.events[at].total_rounds() {
            self.finish_event(at);
            return;
        }
        let event = self.events[at].clone();
        let size = 1usize << (round - 1);
        let occupant = |alive: &[u32], group: usize| event.slots[group * size..(group + 1) * size].iter().map(|(p, _)| *p).find(|p| *p != 0 && alive.contains(p));
        let mut pairs = vec![];
        for pair in 0..SEATS / size / 2 {
            let alive = self.events[at].alive.clone();
            match (occupant(&alive, 2 * pair), occupant(&alive, 2 * pair + 1)) {
                (Some(a), Some(b)) => pairs.push((a, b)),
                (Some(p), None) | (None, Some(p)) => self.events[at].advance(p),
                (None, None) => {}
            }
        }
        let rule: Vec<(String, Value)> = [("match_type", "match_type"), ("regulation_id", "regulation_id"), ("lp_start", "duel_lp_start"), ("surrender", "duel_surrender"), ("time_turn", "duel_time_turn"), ("time_total", "duel_time_total")]
            .into_iter().map(|(k, from)| (k.to_string(), Value::U32(event.int(from)))).chain([("event_type".to_string(), Value::U32(10)), ("official".to_string(), Value::U32(1))]).collect();
        let times = Some((event.int("duel_time_turn"), event.int("duel_time_total"))).filter(|(turn, _)| *turn > 0);
        let mut duels = vec![];
        for (a, b) in pairs {
            let records = [a, b].map(|p| self.online.values().find(|o| o.player_id == p).map(|o| o.record.clone()));
            let [Some(ra), Some(rb)] = records else {
                // A player gone forfeits; the other goes through (neither here: both are out).
                match (records[0].is_some(), records[1].is_some()) {
                    (true, false) => { self.events[at].alive.retain(|p| *p != b); self.events[at].advance(a); }
                    (false, true) => { self.events[at].alive.retain(|p| *p != a); self.events[at].advance(b); }
                    _ => self.events[at].alive.retain(|p| *p != a && *p != b),
                }
                continue;
            };
            self.last_duel += 1;
            let duel_id = self.last_duel;
            let push = |vs: &Message| {
                let data = rule.iter().fold(Message::record().with("name", Value::text("duelData")), |m, (k, v)| m.with(k, v.clone()));
                Message::reply(0x1978).item(vs.clone().with("name", Value::text("vsPlayerData"))).item(data.with("duel_id", Value::U32(duel_id)))
            };
            self.push(a, &push(&rb));
            self.push(b, &push(&ra));
            self.matches.insert(duel_id, Match { players: [a, b], rule: rule.clone(), times, watchable: true, made: Instant::now(), event: None, cpu: false });
            duels.push((duel_id, [a, b]));
        }
        self.events[at].played = !duels.is_empty();
        self.events[at].round = duels;
        if self.events[at].round.is_empty() {
            // Byes only (or forfeits): nothing to wait for, no banner.
            self.next_round(at, true);
            return;
        }
        self.round_banner(at, true);
        self.refresh_lobby(at, &[6, 5, 2, 3]);
        self.save_events();
    }

    /// A round is over: the titles of the rounds the players have now gone through.
    pub(super) fn champ_round_over(&mut self, at: usize) {
        let event = &self.events[at];
        let wins: Vec<(u32, u32)> = event.slots.iter().filter(|(p, _)| *p != 0).map(|(p, _)| (*p, event.won(*p))).collect();
        for (p, w) in wins {
            // Only those still in or just out of this round keep climbing: a title is for the rounds gone through.
            self.champ_titles(p, w);
        }
    }

    /// Grants a player the Y.O.C. titles a number of rounds won earns (a bye counts), each once.
    fn champ_titles(&mut self, player_id: u32, wins: u32) {
        let (Some(titles), Some(directory)) = (self.titles.clone(), self.directory.clone()) else { return };
        let ids: Vec<u16> = champ::titles_for(wins).into_iter().filter_map(|n| crate::titles::by_name(&titles, n)).collect();
        let mut new = vec![];
        let saved = directory.locked().update_player(player_id, |p| {
            for id in &ids {
                if p.items.insert(*id) { new.push(*id) }
            }
        });
        if let Err(why) = saved {
            tracing::warn!("Championship titles of player {player_id} not kept: {why}");
            return;
        }
        for id in new {
            if let Some(name) = titles.name(id) { self.push(player_id, &crate::titles::note(name)); }
        }
    }

    /// The Final is over (`finish_event` told the lobbies): the record for Duel Log, and each player's prize.
    pub(super) fn champ_finished(&mut self, at: usize) {
        let event = self.events[at].clone();
        let Some(tag) = event.champ else { return };
        let name_of = |p: u32| (p != 0).then(|| (p, event.name(p)));
        let winner = event.alive.first().copied().and_then(name_of);
        let second = event.second.and_then(name_of);
        let Some(c) = self.championships.list.iter_mut().find(|c| c.id == tag.championship) else { return };
        (c.state, c.finished, c.winner, c.second, c.bracket) = (State::Finished, chrono::Utc::now().timestamp(), winner.clone(), second, Some(event.to_json()));
        let (id, title, prize) = (c.id, c.name.clone(), c.prize);
        if let Err(e) = self.championships.save(id) { tracing::warn!("Championship {id} not saved: {e}") }
        let champion = winner.as_ref().map_or_else(|| "nobody".to_string(), |(_, n)| n.clone());
        for (p, name) in event.slots.iter().filter(|(p, _)| *p != 0) {
            let bp = prize.owed(true, event.won(*p));
            if bp == 0 { continue }
            let paid = self.directory.as_ref().map(|d| d.locked().pay(*p, bp));
            let total = match paid {
                Some(Ok(Some(total))) => Some(total),
                Some(Ok(None)) | None => None,
                Some(Err(e)) => { tracing::warn!("prize of player {p} in Championship {id} not paid: {e}"); continue }
            };
            if let Some(total) = total {
                let players = self.directory.as_ref().map(|d| d.locked().account_players(*p)).unwrap_or_default();
                self.push_bp(&players, total);
            }
            let body = format!("The Championship \"{title}\" is over; {champion} won it. Prize: {bp} BP.{}", total.map_or(String::new(), |t| format!(" You have {t} BP now.")));
            self.champ_mail((*p, name.clone()), format!("{title} prize"), body);
        }
    }
}
