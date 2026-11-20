//! Official Events (the map's Stadium, OfficialEventScene `0x4B0110`, #18): events the host schedules, which players
//! join and play as Auto Duels against the other entrants of the same event. Five kinds, as the client names them
//! (`event_name_type` 1-5): Gunslinger (win streak), Duelist Trial (wins), Evolution Match (stars, matches), Ultimate
//! Match (stars, matches) and Anniversary Trial (wins). A player's score is kept per event in `server.db`
//! (`official_events`, `official_scores`); without a data folder (tests) in memory. The prizes are BP, paid when the
//! event ends (`Official::settle`, `Hub::tick_official`).
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use yo3_proto::{Message, Value};

pub const TABLES: &str = "CREATE TABLE IF NOT EXISTS official_events (id INTEGER PRIMARY KEY, kind TEXT NOT NULL, name TEXT NOT NULL,
    start INTEGER NOT NULL, finish INTEGER NOT NULL, regulation INTEGER, participation_bp INTEGER NOT NULL DEFAULT 0,
    threshold INTEGER NOT NULL DEFAULT 0, threshold_bp INTEGER NOT NULL DEFAULT 0, state TEXT NOT NULL DEFAULT 'active');
    CREATE TABLE IF NOT EXISTS official_scores (event_id INTEGER NOT NULL, player_id INTEGER NOT NULL, name TEXT NOT NULL,
    score INTEGER NOT NULL DEFAULT 0, best INTEGER NOT NULL DEFAULT 0, wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0, draws INTEGER NOT NULL DEFAULT 0, paid INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (event_id, player_id));";

/// The first official event's id: above Auto Duel's and the user events' (from 100). The id is what the lobby echoes.
pub const FIRST_EVENT: u32 = 10_000;
/// Events the server keeps scheduled or running at once, and the longest name (characters).
pub const MAX_ACTIVE: usize = 50;
pub const MAX_NAME: usize = 60;
/// What the list (`0x177B`) shows: the newest events, and a finished event only for this long after its end.
const LISTED: usize = 20;
const SHOWN_AFTER_END: i64 = 7 * 86_400;
/// The score the client shows (`event_score`, the result screen's star effect) goes up to 9999.
pub const SCORE_SHOWN: u32 = 9999;
/// An event's `status` (0x4B1060): 10 and 11 coming soon, 12 running and only to watch, 13 running and joinable, else
/// finished. 10 is more than a day before the start, 11 within it (**guess**: the client treats them alike).
pub const SOON: u32 = 10;
pub const STARTING: u32 = 11;
pub const OPEN: u32 = 13;
pub const FINISHED: u32 = 14;
/// `server_id` of the entry reply (0x1785): non-zero, or Participate never starts matching (0x4A3F70). Meaning unknown.
pub const SERVER_ID: u32 = 1;
/// Tournament Points (**guess**): taking part is worth this much, and each point of an event's best score one more.
pub const TP_FOR_PLAYING: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Gunslinger,
    DuelistTrial,
    Evolution,
    Ultimate,
    Anniversary,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Gunslinger, Kind::DuelistTrial, Kind::Evolution, Kind::Ultimate, Kind::Anniversary];

    /// The list's `event_name_type` (0x4B1060): 1 Gunslinger, 2 Duelist Trial, 3 Evolution, 4 Anniversary, 5 Ultimate.
    pub fn name_type(self) -> u32 {
        match self { Kind::Gunslinger => 1, Kind::DuelistTrial => 2, Kind::Evolution => 3, Kind::Anniversary => 4, Kind::Ultimate => 5 }
    }

    /// The `event_type` Konami's duel logs carry for the kind (**guess**, see the history): 11 for the match formats, 4
    /// Anniversary, 3 the single-duel trials. Never 10, which the Championship's scene looks for.
    pub fn event_type(self) -> u32 {
        match self { Kind::Gunslinger | Kind::DuelistTrial => 3, Kind::Anniversary => 4, Kind::Evolution | Kind::Ultimate => 11 }
    }

    /// Whether it is played as a match (best of three) rather than a single duel.
    pub fn is_match(self) -> bool { matches!(self, Kind::Evolution | Kind::Ultimate) }

    /// What the score counts, in the host's words.
    pub fn counts(self) -> &'static str {
        match self { Kind::Gunslinger => "win streak", Kind::DuelistTrial | Kind::Anniversary => "wins", Kind::Evolution | Kind::Ultimate => "stars" }
    }

    pub fn title(self) -> &'static str {
        match self {
            Kind::Gunslinger => "Gunslinger", Kind::DuelistTrial => "Duelist Trial", Kind::Evolution => "Evolution Match",
            Kind::Ultimate => "Ultimate Match", Kind::Anniversary => "Anniversary Trial",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Kind::Gunslinger => "gunslinger", Kind::DuelistTrial => "duelist-trial", Kind::Evolution => "evolution",
            Kind::Ultimate => "ultimate", Kind::Anniversary => "anniversary",
        }
    }

    /// A kind by its command-line name (`gunslinger`, `duelist-trial`, `evolution`, `ultimate`, `anniversary`).
    pub fn parse(text: &str) -> Option<Kind> {
        let text = text.trim().to_ascii_lowercase().replace([' ', '_'], "-");
        Kind::ALL.into_iter().find(|k| k.text() == text || k.title().to_ascii_lowercase().replace(' ', "-") == text)
    }
}

/// Where an event stands besides the clock: cancelled by the host, or ended and its prizes settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    Active,
    Cancelled,
    Finished,
}

impl State {
    fn text(self) -> &'static str { match self { State::Active => "active", State::Cancelled => "cancelled", State::Finished => "finished" } }
    fn of(text: &str) -> State { match text { "cancelled" => State::Cancelled, "finished" => State::Finished, _ => State::Active } }
}

/// What an event pays, in BP (the original's Special Packs don't exist here): to everyone who played a duel of it, and
/// besides to everyone whose best score reached the threshold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Prize {
    #[serde(default)]
    #[schema(maximum = 1000000)]
    pub participation_bp: u32,
    /// The best score that earns `threshold_bp` (0: no threshold).
    #[serde(default)]
    #[schema(maximum = 9999)]
    pub threshold: u32,
    #[serde(default)]
    #[schema(maximum = 1000000)]
    pub threshold_bp: u32,
}

/// A player's standing in one event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Entry {
    #[schema(maximum = 4294967295_u32)]
    pub player_id: u32,
    pub name: String,
    /// What the client shows as `event_score`: the current win streak (Gunslinger), the wins or the stars.
    pub score: u32,
    /// The best score reached: what the prize and the Tournament Points count (a streak ends, the best stays).
    pub best: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    /// The prize has been paid (or there was none).
    pub paid: bool,
}

impl Entry {
    fn new(player_id: u32, name: &str) -> Entry {
        Entry { player_id, name: name.into(), score: 0, best: 0, wins: 0, losses: 0, draws: 0, paid: false }
    }

    pub fn played(&self) -> u32 { self.wins + self.losses + self.draws }

    /// Counts a duel's result (1 won, 2 lost, 3 draw) for an event of `kind`: a win adds one (a win, a streak step or a
    /// star: **guess**, stars never fall), a loss or a draw ends a Gunslinger's streak.
    fn count(&mut self, kind: Kind, result: u32) {
        match result {
            1 => { self.wins += 1; self.score += 1; }
            2 => { self.losses += 1; if kind == Kind::Gunslinger { self.score = 0 } }
            3 => { self.draws += 1; if kind == Kind::Gunslinger { self.score = 0 } }
            _ => return,
        }
        self.best = self.best.max(self.score);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Event {
    #[schema(maximum = 4294967295_u32)]
    pub id: u32,
    pub kind: Kind,
    pub name: String,
    /// Unix seconds.
    pub start: i64,
    pub end: i64,
    /// The banlist entrants must log in with (a regulation id); none: any.
    #[schema(maximum = 4294967295_u32)]
    pub regulation: Option<u32>,
    pub prize: Prize,
    pub state: State,
    pub entries: Vec<Entry>,
}

impl Event {
    /// The `status` the client's list and lobby read.
    pub fn status(&self, now: i64) -> u32 {
        if self.state == State::Cancelled || now >= self.end || self.state == State::Finished { FINISHED }
        else if now < self.start { if self.start - now > 86_400 { SOON } else { STARTING } }
        else { OPEN }
    }

    pub fn is_open(&self, now: i64) -> bool { self.status(now) == OPEN }

    pub fn entry(&self, player_id: u32) -> Option<&Entry> { self.entries.iter().find(|e| e.player_id == player_id) }

    /// A row of the list (`0x177B`, `0x4B1060`) and the lobby's item (`0x1780`, `0x4B0B70`). The times are text, see [`jst`].
    pub fn row(&self, now: i64) -> Message {
        Message::record()
            .with("event_id", Value::U32(self.id))
            .with("official", Value::U32(1))
            .with("event_name_type", Value::U32(self.kind.name_type()))
            .with("event_type", Value::U32(self.kind.event_type()))
            .with("event_name", Value::text(&self.name))
            .with("status", Value::U32(self.status(now)))
            .with("event_start", Value::text(&jst(self.start)))
            .with("event_end", Value::text(&jst(self.end)))
            .with("player_num", Value::U32(self.entries.len() as u32))
    }

    /// What an entry is owed when the event ends, and why.
    pub fn owed(&self, entry: &Entry) -> (u32, String) {
        let mut bp = 0;
        let mut why = vec![];
        if entry.played() > 0 && self.prize.participation_bp > 0 {
            bp += self.prize.participation_bp;
            why.push(format!("{} BP for taking part", self.prize.participation_bp));
        }
        if self.prize.threshold > 0 && self.prize.threshold_bp > 0 && entry.best >= self.prize.threshold {
            bp += self.prize.threshold_bp;
            why.push(format!("{} BP for reaching {} ({})", self.prize.threshold_bp, self.prize.threshold, self.kind.counts()));
        }
        (bp, why.join(" and "))
    }

    /// The entries by score, the best first (ties by the earlier wins, then name).
    pub fn standings(&self) -> Vec<&Entry> {
        let mut all: Vec<&Entry> = self.entries.iter().collect();
        all.sort_by(|a, b| b.best.cmp(&a.best).then(b.wins.cmp(&a.wins)).then_with(|| a.name.cmp(&b.name)));
        all
    }
}

/// A time as the client reads `event_start` and `event_end` (`0x4B0CF0`, then `0x4AB490`): the text `YYYY-MM-DD HH:MM:SS`
/// (`%d-%d-%d %d:%d:%d`, then mktime and localtime), which a number crashes (null from localtime, live). Japan time like
/// every time we send; the client shows it in the player's own time zone.
pub fn jst(seconds: i64) -> String {
    let t = chrono::DateTime::from_timestamp(seconds, 0).unwrap_or_default() + chrono::Duration::hours(9);
    t.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// A prize to pay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payout {
    pub event_id: u32,
    pub event: String,
    pub player_id: u32,
    pub name: String,
    pub bp: u32,
    pub why: String,
    pub best: u32,
}

/// A new event, as the host gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewEvent {
    pub kind: Kind,
    /// The host's name for it, in D-Mails and the dashboard (the game shows the kind's own name).
    #[schema(min_length = 1, max_length = 60)]
    pub name: String,
    /// Unix seconds.
    pub start: i64,
    pub end: i64,
    #[serde(default)]
    pub regulation: Option<u32>,
    #[serde(default)]
    pub prize: Prize,
}

/// Changes to an event not yet finished; what is left out stays. `regulation` null is "no change", 0 is "any".
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Changes {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub end: Option<i64>,
    #[serde(default)]
    pub regulation: Option<u32>,
    #[serde(default)]
    pub prize: Option<Prize>,
}

/// The official events of one server.
#[derive(Clone, Debug, Default)]
pub struct Official {
    dir: Option<PathBuf>,
    pub events: Vec<Event>,
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<Event> {
    let kind: String = r.get(1)?;
    Ok(Event {
        id: r.get(0)?, kind: Kind::parse(&kind).unwrap_or(Kind::DuelistTrial), name: r.get(2)?, start: r.get(3)?, end: r.get(4)?,
        regulation: r.get(5)?, prize: Prize { participation_bp: r.get(6)?, threshold: r.get(7)?, threshold_bp: r.get(8)? },
        state: State::of(&r.get::<_, String>(9)?), entries: vec![],
    })
}

fn clean(name: &str) -> String { name.chars().filter(|c| !c.is_control()).take(MAX_NAME).collect::<String>().trim().to_string() }

impl Official {
    /// The events kept in `dir`'s database (none when it can't be read: the next change would then start afresh, so it
    /// is an error the caller sees: [`Official::load`]).
    pub fn at(dir: &Path) -> Official {
        Official::load(dir).unwrap_or_else(|e| {
            tracing::error!("official events not read from {}: {e}", dir.display());
            Official { dir: Some(dir.to_path_buf()), events: vec![] }
        })
    }

    pub fn load(dir: &Path) -> io::Result<Official> {
        if !dir.join("server.db").is_file() { return Ok(Official { dir: Some(dir.to_path_buf()), events: vec![] }) }
        let conn = crate::store::hold(dir)?;
        let mut events: Vec<Event> = {
            let mut q = conn.prepare("SELECT id, kind, name, start, finish, regulation, participation_bp, threshold, threshold_bp, state FROM official_events ORDER BY id")
                .map_err(crate::store::sql)?;
            q.query_map([], row).map_err(crate::store::sql)?.collect::<Result<_, _>>().map_err(crate::store::sql)?
        };
        let mut q = conn.prepare("SELECT event_id, player_id, name, score, best, wins, losses, draws, paid FROM official_scores ORDER BY rowid")
            .map_err(crate::store::sql)?;
        let scores = q.query_map([], |r| Ok((r.get::<_, u32>(0)?, Entry {
            player_id: r.get(1)?, name: r.get(2)?, score: r.get(3)?, best: r.get(4)?, wins: r.get(5)?, losses: r.get(6)?, draws: r.get(7)?,
            paid: r.get::<_, i64>(8)? != 0,
        }))).map_err(crate::store::sql)?;
        for s in scores {
            let (event, entry) = s.map_err(crate::store::sql)?;
            if let Some(e) = events.iter_mut().find(|e| e.id == event) { e.entries.push(entry) }
        }
        Ok(Official { dir: Some(dir.to_path_buf()), events })
    }

    pub fn get(&self, id: u32) -> Option<&Event> { self.events.iter().find(|e| e.id == id) }

    /// Writes one event with its entries.
    fn save(&self, id: u32) -> io::Result<()> {
        let (Some(dir), Some(e)) = (&self.dir, self.get(id)) else { return Ok(()) };
        let mut conn = crate::store::hold(dir)?;
        let tx = conn.transaction().map_err(crate::store::sql)?;
        tx.execute("INSERT INTO official_events (id, kind, name, start, finish, regulation, participation_bp, threshold, threshold_bp, state)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, name=excluded.name, start=excluded.start,
            finish=excluded.finish, regulation=excluded.regulation, participation_bp=excluded.participation_bp, threshold=excluded.threshold,
            threshold_bp=excluded.threshold_bp, state=excluded.state",
            rusqlite::params![e.id, e.kind.text(), e.name, e.start, e.end, e.regulation, e.prize.participation_bp, e.prize.threshold, e.prize.threshold_bp, e.state.text()])
            .map_err(crate::store::sql)?;
        for s in &e.entries { Self::put_score(&tx, e.id, s)?; }
        tx.commit().map_err(crate::store::sql)
    }

    fn put_score(conn: &rusqlite::Connection, event: u32, s: &Entry) -> io::Result<()> {
        conn.execute("INSERT INTO official_scores (event_id, player_id, name, score, best, wins, losses, draws, paid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(event_id, player_id) DO UPDATE SET name=excluded.name, score=excluded.score, best=excluded.best, wins=excluded.wins,
            losses=excluded.losses, draws=excluded.draws, paid=excluded.paid",
            rusqlite::params![event, s.player_id, s.name, s.score, s.best, s.wins, s.losses, s.draws, s.paid as i64]).map_err(crate::store::sql)?;
        Ok(())
    }

    fn save_entry(&self, id: u32, player: u32) -> io::Result<()> {
        let (Some(dir), Some(s)) = (&self.dir, self.get(id).and_then(|e| e.entry(player))) else { return Ok(()) };
        Self::put_score(&*crate::store::hold(dir)?, id, s)
    }

    fn check(&self, name: &str, start: i64, end: i64, prize: &Prize) -> Result<(), String> {
        if name.is_empty() { return Err("give the event a name".into()) }
        if end <= start { return Err("an event must end after it starts".into()) }
        if prize.threshold > SCORE_SHOWN { return Err(format!("the score threshold is at most {SCORE_SHOWN}")) }
        if prize.threshold == 0 && prize.threshold_bp > 0 { return Err("a threshold prize needs a threshold".into()) }
        Ok(())
    }

    /// Schedules an event; its id.
    pub fn create(&mut self, new: NewEvent) -> Result<Event, String> {
        let name = clean(&new.name);
        self.check(&name, new.start, new.end, &new.prize)?;
        if self.events.iter().filter(|e| e.state == State::Active).count() >= MAX_ACTIVE {
            return Err(format!("at most {MAX_ACTIVE} events may be scheduled at once"))
        }
        let id = self.events.iter().map(|e| e.id).max().map_or(FIRST_EVENT, |m| m.max(FIRST_EVENT - 1) + 1);
        self.events.push(Event { id, kind: new.kind, name, start: new.start, end: new.end, regulation: new.regulation.filter(|r| *r != 0),
            prize: new.prize, state: State::Active, entries: vec![] });
        self.save(id).map_err(|e| e.to_string())?;
        Ok(self.get(id).cloned().expect("just made"))
    }

    /// Changes an event that is not finished or cancelled: the event as it was and as it is now.
    pub fn edit(&mut self, id: u32, changes: &Changes) -> Result<(Event, Event), String> {
        let at = self.events.iter().position(|e| e.id == id).ok_or_else(|| format!("no official event {id}"))?;
        let before = self.events[at].clone();
        if before.state != State::Active { return Err(format!("event {id} is already {}", before.state.text())) }
        let name = changes.name.as_deref().map_or_else(|| before.name.clone(), clean);
        let (start, end) = (changes.start.unwrap_or(before.start), changes.end.unwrap_or(before.end));
        let prize = changes.prize.unwrap_or(before.prize);
        self.check(&name, start, end, &prize)?;
        let e = &mut self.events[at];
        (e.name, e.start, e.end, e.prize) = (name, start, end, prize);
        if let Some(r) = changes.regulation { e.regulation = Some(r).filter(|r| *r != 0) }
        self.save(id).map_err(|e| e.to_string())?;
        Ok((before, self.events[at].clone()))
    }

    /// Cancels an event: it leaves the list, nothing is paid. The event as it was.
    pub fn cancel(&mut self, id: u32) -> Result<Event, String> {
        let at = self.events.iter().position(|e| e.id == id).ok_or_else(|| format!("no official event {id}"))?;
        let before = self.events[at].clone();
        if before.state != State::Active { return Err(format!("event {id} is already {}", before.state.text())) }
        self.events[at].state = State::Cancelled;
        self.save(id).map_err(|e| e.to_string())?;
        Ok(before)
    }

    /// The events the Stadium's list shows (`0x177B`): not cancelled, the newest first, a finished one for a week.
    pub fn listed(&self, now: i64) -> Vec<&Event> {
        let mut shown: Vec<&Event> = self.events.iter().filter(|e| e.state != State::Cancelled && now - e.end < SHOWN_AFTER_END).collect();
        shown.sort_by(|a, b| b.start.cmp(&a.start).then(b.id.cmp(&a.id)));
        shown.truncate(LISTED);
        shown
    }

    /// A player entering the lobby (`0x1784`): their entry, made when there is none. None when the event is not open.
    pub fn join(&mut self, id: u32, player: u32, name: &str, now: i64) -> Option<Entry> {
        let at = self.events.iter().position(|e| e.id == id && e.is_open(now))?;
        if self.events[at].entry(player).is_none() {
            self.events[at].entries.push(Entry::new(player, name));
        }
        if let Err(e) = self.save_entry(id, player) { tracing::warn!("official event {id}: entry of player {player} not saved: {e}") }
        self.events[at].entry(player).cloned()
    }

    /// A duel of the event ended for `player`: their new score, none when it isn't counted (the event isn't running).
    pub fn record(&mut self, id: u32, player: u32, name: &str, result: u32, now: i64) -> Option<u32> {
        let at = self.events.iter().position(|e| e.id == id && e.state == State::Active && now >= e.start)?;
        let kind = self.events[at].kind;
        let entries = &mut self.events[at].entries;
        let i = match entries.iter().position(|e| e.player_id == player) {
            Some(i) => i,
            None => { entries.push(Entry::new(player, name)); entries.len() - 1 }
        };
        entries[i].count(kind, result);
        let score = entries[i].score;
        if let Err(e) = self.save_entry(id, player) { tracing::warn!("official event {id}: score of player {player} not saved: {e}") }
        Some(score)
    }

    /// Ends the events whose time is up and lists the prizes still to pay (also those of an event that ended earlier
    /// and whose payment did not finish). Entries owed nothing are settled here.
    pub fn settle(&mut self, now: i64) -> Vec<Payout> {
        let mut payouts = vec![];
        let ids: Vec<u32> = self.events.iter().filter(|e| e.state != State::Cancelled && now >= e.end).map(|e| e.id).collect();
        for id in ids {
            let at = self.events.iter().position(|e| e.id == id).expect("listed");
            let event = self.events[at].clone();
            let mut changed = event.state != State::Finished;
            self.events[at].state = State::Finished;
            for entry in &event.entries {
                if entry.paid { continue }
                let (bp, why) = event.owed(entry);
                if bp == 0 {
                    if let Some(e) = self.events[at].entries.iter_mut().find(|e| e.player_id == entry.player_id) { e.paid = true }
                    changed = true;
                } else {
                    payouts.push(Payout { event_id: id, event: event.name.clone(), player_id: entry.player_id, name: entry.name.clone(), bp, why, best: entry.best });
                }
            }
            if changed && let Err(e) = self.save(id) { tracing::warn!("official event {id} not saved: {e}") }
        }
        payouts
    }

    /// A prize was paid.
    pub fn mark_paid(&mut self, id: u32, player: u32) {
        if let Some(e) = self.events.iter_mut().find(|e| e.id == id).and_then(|e| e.entries.iter_mut().find(|e| e.player_id == player)) { e.paid = true }
        if let Err(e) = self.save_entry(id, player) { tracing::warn!("official event {id}: payment to player {player} not saved: {e}") }
    }

    /// Tournament Points by player (**guess**, our own rule: the client only shows the number): for each event that
    /// wasn't cancelled, [`TP_FOR_PLAYING`] for a player who played a duel of it and the best score they reached.
    pub fn points(&self) -> HashMap<u32, u32> {
        let mut tp: HashMap<u32, u32> = HashMap::new();
        for e in self.events.iter().filter(|e| e.state != State::Cancelled) {
            for s in e.entries.iter().filter(|s| s.played() > 0) {
                *tp.entry(s.player_id).or_default() += TP_FOR_PLAYING + s.best;
            }
        }
        tp
    }

    /// A player's Tournament Points and their rank among the players who have any (1 is the most; 0 when they have none).
    pub fn tp_of(&self, player: u32) -> (u32, u32) {
        let all = self.points();
        let Some(&mine) = all.get(&player).filter(|p| **p > 0) else { return (0, 0) };
        (mine, 1 + all.values().filter(|p| **p > mine).count() as u32)
    }

    /// Takes out `players`' standings (a deleted account's), in memory.
    pub fn forget_players(&mut self, players: &[u32]) {
        for e in &mut self.events { e.entries.retain(|s| !players.contains(&s.player_id)) }
    }
}

/// Deletes the standings of `players` from `dir`'s database (a deleted account's).
pub fn forget(dir: &Path, players: &[u32]) -> io::Result<usize> {
    if !dir.join("server.db").is_file() { return Ok(0) }
    let conn = crate::store::hold(dir)?;
    let mut gone = 0;
    for p in players {
        gone += conn.execute("DELETE FROM official_scores WHERE player_id=?1", [p]).map_err(crate::store::sql)?;
    }
    Ok(gone)
}

/// Whether a player's entry is stored: for tests of the store.
#[doc(hidden)]
pub fn stored_score(dir: &Path, event: u32, player: u32) -> io::Result<Option<u32>> {
    crate::store::hold(dir)?.query_row("SELECT score FROM official_scores WHERE event_id=?1 AND player_id=?2", [event, player], |r| r.get(0))
        .optional().map_err(crate::store::sql)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(kind: Kind, start: i64, end: i64) -> NewEvent {
        NewEvent { kind, name: "Test".into(), start, end, regulation: None, prize: Prize { participation_bp: 100, threshold: 3, threshold_bp: 500 } }
    }

    #[test]
    fn a_gunslingers_streak_ends_with_a_loss_but_its_best_stays() {
        let mut e = Entry::new(1, "Yugi");
        for r in [1, 1, 1, 2, 1] { e.count(Kind::Gunslinger, r) }
        assert_eq!((e.score, e.best, e.wins, e.losses), (1, 3, 4, 1));
    }

    #[test]
    fn trial_wins_and_stars_never_fall() {
        for kind in [Kind::DuelistTrial, Kind::Anniversary, Kind::Evolution, Kind::Ultimate] {
            let mut e = Entry::new(1, "Yugi");
            for r in [1, 2, 3, 1] { e.count(kind, r) }
            assert_eq!((e.score, e.best, e.played()), (2, 2, 4), "{kind:?}");
        }
    }

    #[test]
    fn the_status_follows_the_clock_and_the_host() {
        let mut o = Official::default();
        let e = o.create(new(Kind::Gunslinger, 1000 + 3 * 86_400, 1000 + 5 * 86_400)).unwrap();
        assert_eq!(e.status(1000), SOON);
        assert_eq!(e.status(1000 + 2 * 86_400 + 1), STARTING);
        assert_eq!(e.status(1000 + 3 * 86_400), OPEN);
        assert_eq!(e.status(1000 + 5 * 86_400), FINISHED);
        o.cancel(e.id).unwrap();
        assert_eq!(o.get(e.id).unwrap().status(1000 + 3 * 86_400), FINISHED);
        assert!(o.listed(1000).is_empty(), "a cancelled event isn't listed");
    }

    #[test]
    fn only_an_open_event_takes_entries_and_scores() {
        let mut o = Official::default();
        let e = o.create(new(Kind::DuelistTrial, 100, 200)).unwrap();
        assert!(o.join(e.id, 1, "Yugi", 50).is_none());
        assert!(o.join(e.id, 1, "Yugi", 150).is_some());
        assert_eq!(o.record(e.id, 1, "Yugi", 1, 150), Some(1));
        assert_eq!(o.record(e.id, 1, "Yugi", 1, 150), Some(2));
        assert_eq!(o.record(e.id, 1, "Yugi", 1, 50), None, "not before the start");
        assert_eq!(o.get(e.id).unwrap().entries.len(), 1);
    }

    #[test]
    fn settling_pays_each_entry_once_and_the_threshold_only_to_those_who_reached_it() {
        let mut o = Official::default();
        let e = o.create(new(Kind::DuelistTrial, 100, 200)).unwrap();
        for (p, wins) in [(1, 3), (2, 1), (3, 0)] {
            o.join(e.id, p, &format!("P{p}"), 150);
            for _ in 0..wins { o.record(e.id, p, "", 1, 150); }
        }
        o.record(e.id, 3, "", 2, 150);
        o.join(e.id, 4, "Idle", 150);
        assert!(o.settle(150).is_empty(), "nothing before the end");
        let payouts = o.settle(200);
        let by_player: Vec<(u32, u32)> = payouts.iter().map(|p| (p.player_id, p.bp)).collect();
        assert_eq!(by_player, [(1, 600), (2, 100), (3, 100)], "the idle entrant is owed nothing");
        assert_eq!(o.get(e.id).unwrap().state, State::Finished);
        o.mark_paid(e.id, 1);
        let again: Vec<u32> = o.settle(300).iter().map(|p| p.player_id).collect();
        assert_eq!(again, [2, 3], "a prize not marked paid comes again; a paid one doesn't");
    }

    #[test]
    fn tournament_points_rank_the_players_who_played() {
        let mut o = Official::default();
        let e = o.create(new(Kind::DuelistTrial, 100, 200)).unwrap();
        for (p, wins) in [(1, 3), (2, 1), (3, 1)] {
            o.join(e.id, p, "x", 150);
            for _ in 0..wins { o.record(e.id, p, "", 1, 150); }
        }
        o.join(e.id, 4, "Idle", 150);
        assert_eq!(o.tp_of(1), (8, 1));
        assert_eq!(o.tp_of(2), (6, 2));
        assert_eq!(o.tp_of(3), (6, 2));
        assert_eq!(o.tp_of(4), (0, 0));
    }

    #[test]
    fn the_events_and_their_scores_come_back_from_the_database() {
        let dir = std::env::temp_dir().join(format!("yo3-official-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        crate::store::initialize(&dir).unwrap();
        let mut o = Official::at(&dir);
        let e = o.create(new(Kind::Evolution, 100, 200)).unwrap();
        o.join(e.id, 7, "Yugi", 150);
        o.record(e.id, 7, "Yugi", 1, 150);
        assert_eq!(stored_score(&dir, e.id, 7).unwrap(), Some(1));
        let back = Official::load(&dir).unwrap();
        let again = back.get(e.id).unwrap();
        assert_eq!((again.kind, again.entries.len(), again.entry(7).map(|s| s.score)), (Kind::Evolution, 1, Some(1)));
        assert_eq!(forget(&dir, &[7]).unwrap(), 1);
        assert!(Official::load(&dir).unwrap().get(e.id).unwrap().entries.is_empty());
        crate::store::close(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_host_edits_an_active_event_and_not_a_cancelled_one() {
        let mut o = Official::default();
        let e = o.create(new(Kind::Anniversary, 100, 200)).unwrap();
        let (_, after) = o.edit(e.id, &Changes { end: Some(300), regulation: Some(4), ..Changes::default() }).unwrap();
        assert_eq!((after.end, after.regulation), (300, Some(4)));
        assert!(o.edit(e.id, &Changes { end: Some(50), ..Changes::default() }).is_err());
        let (_, any) = o.edit(e.id, &Changes { regulation: Some(0), ..Changes::default() }).unwrap();
        assert_eq!(any.regulation, None);
        o.cancel(e.id).unwrap();
        assert!(o.edit(e.id, &Changes::default()).is_err());
        assert!(o.cancel(e.id).is_err());
    }

    #[test]
    fn kinds_parse_by_their_command_line_names() {
        assert_eq!(Kind::parse("Duelist Trial"), Some(Kind::DuelistTrial));
        assert_eq!(Kind::parse("duelist-trial"), Some(Kind::DuelistTrial));
        assert_eq!(Kind::parse("ULTIMATE"), Some(Kind::Ultimate));
        assert_eq!(Kind::parse("evolution-match"), Some(Kind::Evolution));
        assert_eq!(Kind::parse("nope"), None);
    }
}
