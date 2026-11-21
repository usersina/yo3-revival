//! The Championship (the map's Tournament tile, TournamentScene `0x4CB570`, #17): a knockout the host schedules. Players
//! enter while its entry window is open (opening the Championship's lobby, `0x1784`); at the start time the server draws
//! the entrants (`0x32D0`, `0x32D1`) and plays the client's Final: 16 seats, four rounds (`0x32FB.tournament_id` 17 or
//! more), byes filling the empty seats. The bracket itself is a [`crate::events::Event`] the hub runs, as it runs user
//! events; this module keeps the schedule, the entries, the seating and the record of each Championship (`server.db`
//! tables `championships` and `championship_entries`; without a data folder, in memory).
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const TABLES: &str = "CREATE TABLE IF NOT EXISTS championships (id INTEGER PRIMARY KEY, name TEXT NOT NULL, year INTEGER NOT NULL,
    season INTEGER NOT NULL, entry_open INTEGER NOT NULL, entry_close INTEGER NOT NULL, start INTEGER NOT NULL,
    min_duels INTEGER NOT NULL DEFAULT 0, participation_bp INTEGER NOT NULL DEFAULT 0, finalist_bp INTEGER NOT NULL DEFAULT 0,
    winner_bp INTEGER NOT NULL DEFAULT 0, state TEXT NOT NULL DEFAULT 'scheduled', finished INTEGER NOT NULL DEFAULT 0,
    winner_id INTEGER, winner_name TEXT, second_id INTEGER, second_name TEXT, bracket TEXT);
    CREATE TABLE IF NOT EXISTS championship_entries (championship_id INTEGER NOT NULL, player_id INTEGER NOT NULL,
    name TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (championship_id, player_id));";

/// The Championship's lobby id (`event_id`, what every bracket request carries): above Auto Duel's, the user events' (from
/// 100) and the official events' (from 10000).
pub const FIRST_EVENT: u32 = 20_000;
/// The Final's seats (`0x32FB.tournament_id` of 17 or more: 16 seats, four rounds).
pub const SEATS: usize = 16;
/// `tournament_id` of the Final (0x4CE670 and 0x4CEBE0 compare it with 0x11).
pub const FINAL_ID: u32 = 17;
/// Championships the server keeps scheduled at once, and the longest name (characters).
pub const MAX_ACTIVE: usize = 10;
pub const MAX_NAME: usize = 60;
/// The Championship stays on the map's list (so the scene shows its result) this long after it ended (**guess**).
pub const LISTED_AFTER_END: i64 = 24 * 3600;
/// The past Championships Duel Log lists.
pub const PAST_LISTED: usize = 20;
/// Tournament titles (by the client's names; the winner's has no full stop after Y.O.C, as the client spells it).
pub const QUALIFIER: &str = "Y.O.C. QUALIFIER";
pub const Q_FINALIST: &str = "Y.O.C. Q-FINALIST";
pub const S_FINALIST: &str = "Y.O.C. S-FINALIST";
pub const FINALIST: &str = "Y.O.C. FINALIST";
pub const WINNER: &str = "Y.O.C WINNER";

/// Where a Championship is: waiting for its start time (entries open from `entry_open`), being played, or over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ChampionshipState {
    Scheduled,
    Cancelled,
    Running,
    Finished,
}

impl ChampionshipState {
    pub fn text(self) -> &'static str {
        match self { ChampionshipState::Scheduled => "scheduled", ChampionshipState::Cancelled => "cancelled", ChampionshipState::Running => "running", ChampionshipState::Finished => "finished" }
    }
    fn of(text: &str) -> ChampionshipState {
        match text { "cancelled" => ChampionshipState::Cancelled, "running" => ChampionshipState::Running, "finished" => ChampionshipState::Finished, _ => ChampionshipState::Scheduled }
    }
}

/// What a Championship pays, in BP: to everyone drawn into the Final, to the finalists (both) and to the winner, which add up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChampionshipPrize {
    #[serde(default)]
    #[schema(maximum = 1000000)]
    pub participation_bp: u32,
    #[serde(default)]
    #[schema(maximum = 1000000)]
    pub finalist_bp: u32,
    #[serde(default)]
    #[schema(maximum = 1000000)]
    pub winner_bp: u32,
}

impl ChampionshipPrize {
    /// BP owed for having got to `wins` rounds won (a bye counts) of the four; `drawn`: in the bracket at all.
    pub fn owed(&self, drawn: bool, wins: u32) -> u32 {
        if !drawn { return 0 }
        self.participation_bp + if wins >= 3 { self.finalist_bp } else { 0 } + if wins >= 4 { self.winner_bp } else { 0 }
    }
}

/// The title (by name) a player has earned at `wins` rounds won, a bye counting as a win: drawn into the bracket
/// QUALIFIER; through the first round (the quarter-final stage, eight left) Q-FINALIST; through the second (the
/// semi-final) S-FINALIST; through the third (the final) FINALIST; the fourth WINNER.
pub fn titles_for(wins: u32) -> Vec<&'static str> {
    [QUALIFIER, Q_FINALIST, S_FINALIST, FINALIST, WINNER].into_iter().take(1 + wins.min(4) as usize).collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Championship {
    pub id: u32,
    pub name: String,
    /// The year (Japan time) of the start and the number of the Championship in it: the banner's and Duel Log's
    /// `year` and `season`.
    pub year: u32,
    pub season: u32,
    /// Unix seconds: entries open, entries close, and the draw and first round.
    pub entry_open: i64,
    pub entry_close: i64,
    pub start: i64,
    /// Duels a player must have played to enter (text 15302; the client doesn't check).
    pub min_duels: u32,
    pub prize: ChampionshipPrize,
    pub state: ChampionshipState,
    /// When it ended (unix seconds), 0 until then.
    pub finished: i64,
    /// The entrants in the order they entered, and the winner and runner-up once it is over.
    pub entrants: Vec<(u32, String)>,
    pub winner: Option<(u32, String)>,
    pub second: Option<(u32, String)>,
    /// The bracket as it ended (`Event::to_json`), kept for Duel Log's list.
    pub bracket: Option<serde_json::Value>,
}

impl Championship {
    /// The lobby's `event_id`.
    pub fn event_id(&self) -> u32 { FIRST_EVENT + self.id }

    pub fn has_entered(&self, player: u32) -> bool { self.entrants.iter().any(|(p, _)| *p == player) }

    /// Whether entries are taken now.
    pub fn is_open(&self, now: i64) -> bool { self.state == ChampionshipState::Scheduled && now >= self.entry_open && now < self.entry_close }

    /// Whether the map lists it (`0x177A`): from the start of its entry window until a day after it ended; never a
    /// cancelled one.
    pub fn listed(&self, now: i64) -> bool {
        match self.state {
            ChampionshipState::Cancelled => false,
            ChampionshipState::Scheduled | ChampionshipState::Running => now >= self.entry_open,
            ChampionshipState::Finished => now - self.finished < LISTED_AFTER_END,
        }
    }
}

/// A new Championship, as the host gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct New {
    pub name: String,
    /// Unix seconds.
    pub entry_open: i64,
    pub entry_close: i64,
    pub start: i64,
    #[serde(default)]
    pub min_duels: u32,
    #[serde(default)]
    pub prize: ChampionshipPrize,
}

/// Changes to a Championship still waiting for its start; what is left out stays.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Changes {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub entry_open: Option<i64>,
    #[serde(default)]
    pub entry_close: Option<i64>,
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub min_duels: Option<u32>,
    #[serde(default)]
    pub prize: Option<ChampionshipPrize>,
}

/// What entering the lobby did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Entered now, or already in.
    In,
    /// Not open (before the window, after it, or not scheduled).
    Closed,
    /// Fewer duels played than the host asks for.
    TooFewDuels(u32),
}

#[derive(Clone, Debug, Default)]
pub struct Championships {
    dir: Option<PathBuf>,
    pub list: Vec<Championship>,
}

fn clean(name: &str) -> String { name.chars().filter(|c| !c.is_control()).take(MAX_NAME).collect::<String>().trim().to_string() }

/// Japan time's year of a unix time (every date the client sees is JST).
pub fn jst_year(seconds: i64) -> u32 {
    use chrono::Datelike;
    (chrono::DateTime::from_timestamp(seconds, 0).unwrap_or_default() + chrono::Duration::hours(9)).year().max(0) as u32
}

impl Championships {
    pub fn at(dir: &Path) -> Championships {
        Championships::load(dir).unwrap_or_else(|e| {
            tracing::error!("championships not read from {}: {e}", dir.display());
            Championships { dir: Some(dir.to_path_buf()), list: vec![] }
        })
    }

    pub fn load(dir: &Path) -> io::Result<Championships> {
        if !dir.join("server.db").is_file() { return Ok(Championships { dir: Some(dir.to_path_buf()), list: vec![] }) }
        let conn = crate::store::hold(dir)?;
        let mut q = conn.prepare("SELECT id, name, year, season, entry_open, entry_close, start, min_duels, participation_bp, finalist_bp, winner_bp,
            state, finished, winner_id, winner_name, second_id, second_name, bracket FROM championships ORDER BY id").map_err(crate::store::sql)?;
        let mut list: Vec<Championship> = q.query_map([], |r| {
            let pair = |id: Option<u32>, name: Option<String>| id.map(|i| (i, name.unwrap_or_default()));
            Ok(Championship {
                id: r.get(0)?, name: r.get(1)?, year: r.get(2)?, season: r.get(3)?, entry_open: r.get(4)?, entry_close: r.get(5)?, start: r.get(6)?,
                min_duels: r.get(7)?, prize: ChampionshipPrize { participation_bp: r.get(8)?, finalist_bp: r.get(9)?, winner_bp: r.get(10)? },
                state: ChampionshipState::of(&r.get::<_, String>(11)?), finished: r.get(12)?, entrants: vec![],
                winner: pair(r.get(13)?, r.get(14)?), second: pair(r.get(15)?, r.get(16)?),
                bracket: r.get::<_, Option<String>>(17)?.and_then(|t| serde_json::from_str(&t).ok()),
            })
        }).map_err(crate::store::sql)?.collect::<Result<_, _>>().map_err(crate::store::sql)?;
        let mut q = conn.prepare("SELECT championship_id, player_id, name FROM championship_entries ORDER BY championship_id, position").map_err(crate::store::sql)?;
        let rows = q.query_map([], |r| Ok((r.get::<_, u32>(0)?, (r.get::<_, u32>(1)?, r.get::<_, String>(2)?)))).map_err(crate::store::sql)?;
        for row in rows {
            let (id, entrant) = row.map_err(crate::store::sql)?;
            if let Some(c) = list.iter_mut().find(|c| c.id == id) { c.entrants.push(entrant) }
        }
        Ok(Championships { dir: Some(dir.to_path_buf()), list })
    }

    pub fn get(&self, id: u32) -> Option<&Championship> { self.list.iter().find(|c| c.id == id) }

    /// A Championship by its lobby id.
    pub fn by_event(&self, event_id: u32) -> Option<&Championship> { self.list.iter().find(|c| c.event_id() == event_id) }

    pub fn save(&self, id: u32) -> io::Result<()> {
        let (Some(dir), Some(c)) = (&self.dir, self.get(id)) else { return Ok(()) };
        let mut conn = crate::store::hold(dir)?;
        let tx = conn.transaction().map_err(crate::store::sql)?;
        tx.execute("INSERT INTO championships (id, name, year, season, entry_open, entry_close, start, min_duels, participation_bp, finalist_bp, winner_bp,
            state, finished, winner_id, winner_name, second_id, second_name, bracket)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(id) DO UPDATE SET name=excluded.name, year=excluded.year, season=excluded.season, entry_open=excluded.entry_open,
            entry_close=excluded.entry_close, start=excluded.start, min_duels=excluded.min_duels, participation_bp=excluded.participation_bp,
            finalist_bp=excluded.finalist_bp, winner_bp=excluded.winner_bp, state=excluded.state, finished=excluded.finished,
            winner_id=excluded.winner_id, winner_name=excluded.winner_name, second_id=excluded.second_id, second_name=excluded.second_name,
            bracket=excluded.bracket",
            rusqlite::params![c.id, c.name, c.year, c.season, c.entry_open, c.entry_close, c.start, c.min_duels, c.prize.participation_bp,
                c.prize.finalist_bp, c.prize.winner_bp, c.state.text(), c.finished, c.winner.as_ref().map(|w| w.0), c.winner.as_ref().map(|w| w.1.clone()),
                c.second.as_ref().map(|w| w.0), c.second.as_ref().map(|w| w.1.clone()), c.bracket.as_ref().map(|b| b.to_string())])
            .map_err(crate::store::sql)?;
        tx.execute("DELETE FROM championship_entries WHERE championship_id=?1", [c.id]).map_err(crate::store::sql)?;
        for (i, (p, name)) in c.entrants.iter().enumerate() {
            tx.execute("INSERT INTO championship_entries (championship_id, player_id, name, position) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![c.id, p, name, i as u32])
                .map_err(crate::store::sql)?;
        }
        tx.commit().map_err(crate::store::sql)
    }

    fn check(&self, name: &str, new: &New) -> Result<(), String> {
        if name.is_empty() { return Err("give the Championship a name".into()) }
        if new.entry_close <= new.entry_open { return Err("entries must close after they open".into()) }
        if new.start < new.entry_close { return Err("the Championship starts when entries have closed, not before".into()) }
        // One day (the Final's own rule): the entry window and the start are within it.
        if new.start - new.entry_open > 24 * 3600 { return Err("the entry window and the start must fall within one day".into()) }
        Ok(())
    }

    /// Schedules one; at most [`MAX_ACTIVE`] waiting, and no two whose days overlap (the map's scene follows one at a time).
    pub fn create(&mut self, new: New) -> Result<Championship, String> {
        let name = clean(&new.name);
        self.check(&name, &new)?;
        let waiting = self.list.iter().filter(|c| matches!(c.state, ChampionshipState::Scheduled | ChampionshipState::Running)).collect::<Vec<_>>();
        if waiting.len() >= MAX_ACTIVE { return Err(format!("at most {MAX_ACTIVE} Championships may be scheduled at once")) }
        if waiting.iter().any(|c| new.entry_open < c.start + 3600 && c.entry_open < new.start + 3600) {
            return Err("another Championship is scheduled around then (an hour apart at least)".into())
        }
        let id = self.list.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        let year = jst_year(new.start);
        let season = 1 + self.list.iter().filter(|c| c.year == year && c.state != ChampionshipState::Cancelled).count() as u32;
        self.list.push(Championship { id, name, year, season, entry_open: new.entry_open, entry_close: new.entry_close, start: new.start, min_duels: new.min_duels,
            prize: new.prize, state: ChampionshipState::Scheduled, finished: 0, entrants: vec![], winner: None, second: None, bracket: None });
        self.save(id).map_err(|e| e.to_string())?;
        Ok(self.get(id).cloned().expect("just made"))
    }

    pub fn edit(&mut self, id: u32, changes: &Changes) -> Result<(Championship, Championship), String> {
        let at = self.list.iter().position(|c| c.id == id).ok_or_else(|| format!("no Championship {id}"))?;
        let before = self.list[at].clone();
        if before.state != ChampionshipState::Scheduled { return Err(format!("Championship {id} is already {}", before.state.text())) }
        let name = changes.name.as_deref().map_or_else(|| before.name.clone(), clean);
        let new = New { name: name.clone(), entry_open: changes.entry_open.unwrap_or(before.entry_open), entry_close: changes.entry_close.unwrap_or(before.entry_close),
            start: changes.start.unwrap_or(before.start), min_duels: changes.min_duels.unwrap_or(before.min_duels), prize: changes.prize.unwrap_or(before.prize) };
        self.check(&name, &new)?;
        let c = &mut self.list[at];
        (c.name, c.entry_open, c.entry_close, c.start, c.min_duels, c.prize) = (name, new.entry_open, new.entry_close, new.start, new.min_duels, new.prize);
        self.save(id).map_err(|e| e.to_string())?;
        Ok((before, self.list[at].clone()))
    }

    /// Cancels one still waiting (once drawn, it is played out). The Championship as it was.
    pub fn cancel(&mut self, id: u32) -> Result<Championship, String> {
        let at = self.list.iter().position(|c| c.id == id).ok_or_else(|| format!("no Championship {id}"))?;
        let before = self.list[at].clone();
        if before.state != ChampionshipState::Scheduled { return Err(format!("Championship {id} is already {}", before.state.text())) }
        self.list[at].state = ChampionshipState::Cancelled;
        self.save(id).map_err(|e| e.to_string())?;
        Ok(before)
    }

    /// The Championship the map shows (`0x177A`): the newest of those [`Championship::listed`].
    pub fn current(&self, now: i64) -> Option<&Championship> {
        self.list.iter().filter(|c| c.listed(now)).max_by_key(|c| (c.start, c.id))
    }

    /// Duel Log's past Championships (`0x1793`): the finished ones, the newest first.
    pub fn past(&self) -> Vec<&Championship> {
        let mut past: Vec<&Championship> = self.list.iter().filter(|c| c.state == ChampionshipState::Finished).collect();
        past.sort_by(|a, b| b.start.cmp(&a.start).then(b.id.cmp(&a.id)));
        past.truncate(PAST_LISTED);
        past
    }

    /// A player opening the Championship's lobby while entries are open enters it. `duels` is how many they have played.
    pub fn enter(&mut self, event_id: u32, player: u32, name: &str, duels: u32, now: i64) -> Option<Entry> {
        let at = self.list.iter().position(|c| c.event_id() == event_id)?;
        let c = &mut self.list[at];
        if c.has_entered(player) { return Some(Entry::In) }
        if !c.is_open(now) { return Some(Entry::Closed) }
        if duels < c.min_duels { return Some(Entry::TooFewDuels(c.min_duels)) }
        c.entrants.push((player, name.to_string()));
        let id = c.id;
        if let Err(e) = self.save(id) { tracing::warn!("Championship {id}: entry of player {player} not saved: {e}") }
        Some(Entry::In)
    }

    /// Takes out `players`' entries, in memory (a deleted account's).
    pub fn forget_players(&mut self, players: &[u32]) {
        for c in &mut self.list {
            c.entrants.retain(|(p, _)| !players.contains(p));
            for w in [&mut c.winner, &mut c.second] { if w.as_ref().is_some_and(|(p, _)| players.contains(p)) { *w = None } }
        }
    }
}

/// Deletes `players`' entries from `dir`'s database (a deleted account's).
pub fn forget(dir: &Path, players: &[u32]) -> io::Result<usize> {
    if !dir.join("server.db").is_file() { return Ok(0) }
    let conn = crate::store::hold(dir)?;
    let mut gone = 0;
    for p in players {
        gone += conn.execute("DELETE FROM championship_entries WHERE player_id=?1", [p]).map_err(crate::store::sql)?;
        conn.execute("UPDATE championships SET winner_id=NULL, winner_name=NULL WHERE winner_id=?1", [p]).map_err(crate::store::sql)?;
        conn.execute("UPDATE championships SET second_id=NULL, second_name=NULL WHERE second_id=?1", [p]).map_err(crate::store::sql)?;
    }
    Ok(gone)
}

/// Which of the Final's 16 seats take players when `n` (2 to 16) are drawn, spreading the byes (an empty seat in a
/// pair, or a half with no player, sends the other side on unopposed): the plan with the fewest byes any one player
/// can end up with, then the fewest the winner has, then the evenest halves. A player gets a second bye only when the
/// bracket's fixed four rounds leave no other way (below nine players).
pub fn plan(n: usize) -> [bool; SEATS] {
    // (worst byes of anyone in the subtree, byes of its eventual winner) by tree height (0..=4) and players placed.
    fn best(height: u32, k: usize, memo: &mut std::collections::HashMap<(u32, usize), Option<(u32, u32, Vec<bool>)>>) -> Option<(u32, u32, Vec<bool>)> {
        let seats = 1usize << height;
        if k > seats { return None }
        if let Some(hit) = memo.get(&(height, k)) { return hit.clone() }
        let out = if height == 0 {
            Some((0, 0, vec![k == 1]))
        } else {
            let half = seats / 2;
            let mut chosen: Option<((u32, u32, usize, usize), Vec<bool>)> = None;
            for left in 0..=k.min(half) {
                let right = k - left;
                let (Some(l), Some(r)) = (best(height - 1, left, memo), best(height - 1, right, memo)) else { continue };
                let (worst, wins) = match (left, right) {
                    (0, 0) => (0, 0),
                    (0, _) => (r.0.max(r.1 + 1), r.1 + 1),
                    (_, 0) => (l.0.max(l.1 + 1), l.1 + 1),
                    _ => (l.0.max(r.0), l.1.max(r.1)),
                };
                let key = (worst, wins, left.abs_diff(right), half - left.min(half));
                if chosen.as_ref().is_none_or(|(k0, _)| key < *k0) {
                    let mut layout = l.2;
                    layout.extend(r.2);
                    chosen = Some((key, layout));
                }
            }
            chosen.map(|((worst, wins, ..), layout)| (worst, wins, layout))
        };
        memo.insert((height, k), out.clone());
        out
    }
    let layout = best(4, n.min(SEATS), &mut Default::default()).map(|b| b.2).unwrap_or_default();
    let mut seats = [false; SEATS];
    for (s, taken) in seats.iter_mut().zip(layout) { *s = taken }
    seats
}

/// A random number below `n` (the OS's randomness; the draw and the seating must not be guessable).
fn below(n: usize) -> usize {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("OS randomness unavailable");
    (u64::from_le_bytes(bytes) % n.max(1) as u64) as usize
}

/// Shuffles in place (Fisher-Yates).
pub fn shuffle<T>(items: &mut [T]) {
    for i in (1..items.len()).rev() { items.swap(i, below(i + 1)) }
}

/// The entry draw: up to [`SEATS`] of the entrants, at random, and the seats they take in the bracket (16 slots, 0 an
/// empty seat). The ones not drawn are the second list.
pub fn draw(entrants: &[(u32, String)]) -> (Vec<(u32, String)>, Vec<(u32, String)>) {
    let mut all = entrants.to_vec();
    shuffle(&mut all);
    let out = all.split_off(all.len().min(SEATS));
    (all, out)
}

/// The 16 seats of the bracket for the drawn players: `(player_id, name)`, `(0, "")` for an empty seat.
pub fn seat(drawn: &[(u32, String)]) -> Vec<(u32, String)> {
    let mut players = drawn.to_vec();
    shuffle(&mut players);
    let mut players = players.into_iter();
    plan(drawn.len()).iter().map(|taken| if *taken { players.next().unwrap_or((0, String::new())) } else { (0, String::new()) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(open: i64, close: i64, start: i64) -> New { New { name: "Y.O.C.".into(), entry_open: open, entry_close: close, start, min_duels: 0, prize: ChampionshipPrize::default() } }

    /// The byes each player gets in a 16-seat bracket with `seats` taken, the worst case (the higher count wins a duel).
    fn byes(seats: &[bool; SEATS]) -> u32 {
        fn walk(slice: &[bool]) -> Option<(u32, u32)> {
            if slice.len() == 1 { return slice[0].then_some((0, 0)) }
            let (l, r) = (walk(&slice[..slice.len() / 2]), walk(&slice[slice.len() / 2..]));
            match (l, r) {
                (None, None) => None,
                (Some(a), None) | (None, Some(a)) => Some((a.0.max(a.1 + 1), a.1 + 1)),
                (Some(a), Some(b)) => Some((a.0.max(b.0), a.1.max(b.1))),
            }
        }
        walk(seats).map_or(0, |w| w.0)
    }

    #[test]
    fn the_plan_seats_every_player_and_spreads_the_byes() {
        for n in 2..=16 {
            let seats = plan(n);
            assert_eq!(seats.iter().filter(|s| **s).count(), n, "{n} players");
        }
        // Nine or more players: nobody gets a second bye. Fewer, the four fixed rounds leave a bye or two to each.
        for n in 9..=16 {
            assert!(byes(&plan(n)) <= 1, "{n} players: {:?}", plan(n));
        }
        assert_eq!(byes(&plan(16)), 0);
        // Two players meet in the Final, one in each half.
        assert_eq!(plan(2).iter().position(|s| *s), Some(0));
        assert_eq!(plan(2).iter().rposition(|s| *s), Some(8));
    }

    #[test]
    fn the_draw_takes_sixteen_at_random_and_leaves_the_rest_out() {
        let entrants: Vec<(u32, String)> = (1..=20).map(|i| (i, format!("P{i}"))).collect();
        let (drawn, out) = draw(&entrants);
        assert_eq!((drawn.len(), out.len()), (16, 4));
        let mut all: Vec<u32> = drawn.iter().chain(&out).map(|(p, _)| *p).collect();
        all.sort();
        assert_eq!(all, (1..=20).collect::<Vec<_>>());
        let seats = seat(&drawn);
        assert_eq!(seats.len(), 16);
        assert_eq!(seats.iter().filter(|(p, _)| *p != 0).count(), 16);
        let (few, none) = draw(&entrants[..5]);
        assert_eq!((few.len(), none.len()), (5, 0));
        assert_eq!(seat(&few).iter().filter(|(p, _)| *p != 0).count(), 5);
    }

    #[test]
    fn titles_follow_the_rounds_won() {
        assert_eq!(titles_for(0), [QUALIFIER]);
        assert_eq!(titles_for(1), [QUALIFIER, Q_FINALIST]);
        assert_eq!(titles_for(3), [QUALIFIER, Q_FINALIST, S_FINALIST, FINALIST]);
        assert_eq!(titles_for(4).last(), Some(&WINNER));
        let prize = ChampionshipPrize { participation_bp: 10, finalist_bp: 100, winner_bp: 1000 };
        assert_eq!((prize.owed(false, 4), prize.owed(true, 0), prize.owed(true, 3), prize.owed(true, 4)), (0, 10, 110, 1110));
    }

    #[test]
    fn scheduling_checks_the_window_and_the_day() {
        let mut all = Championships::default();
        assert!(all.create(new(100, 100, 200)).is_err(), "entries close after they open");
        assert!(all.create(new(100, 200, 150)).is_err(), "no start before entries close");
        assert!(all.create(new(100, 200, 100 + 25 * 3600)).is_err(), "one day");
        let c = all.create(new(100, 200, 300)).unwrap();
        assert_eq!((c.id, c.event_id(), c.season), (1, FIRST_EVENT + 1, 1));
        assert!(all.create(new(150, 250, 350)).is_err(), "no two at once");
        assert!(all.current(50).is_none(), "not listed before its entries open");
        assert_eq!(all.current(100).map(|c| c.id), Some(1));
        all.cancel(1).unwrap();
        assert!(all.current(150).is_none());
        let again = all.create(new(150, 250, 350)).unwrap();
        assert_eq!((again.id, again.season), (2, 1), "a cancelled one has no season");
    }

    #[test]
    fn entering_needs_the_window_and_the_minimum_duels() {
        let mut all = Championships::default();
        let mut n = new(100, 200, 300);
        n.min_duels = 3;
        let id = all.create(n).unwrap().event_id();
        assert_eq!(all.enter(id, 7, "Yugi", 5, 50), Some(Entry::Closed));
        assert_eq!(all.enter(id, 7, "Yugi", 2, 150), Some(Entry::TooFewDuels(3)));
        assert_eq!(all.enter(id, 7, "Yugi", 3, 150), Some(Entry::In));
        assert_eq!(all.enter(id, 7, "Yugi", 0, 250), Some(Entry::In), "already in");
        assert_eq!(all.enter(id, 8, "Kaiba", 9, 250), Some(Entry::Closed), "the window is shut");
        assert_eq!(all.enter(1, 8, "Kaiba", 9, 150), None, "not a Championship's id");
    }
}
