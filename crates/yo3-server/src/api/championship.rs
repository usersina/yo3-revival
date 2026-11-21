//! The Championship (#17): the host schedules, edits and cancels it, and the staff sees the entries and the result. A
//! running server's Championships are the hub's; with none running the data folder's. Each change is audited. Creating,
//! editing and cancelling is an admin's: the prizes are BP.
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use super::{Actor, Api, MutationError, MutationErrorCode};
use crate::championship::{Championship, Championships, Changes, New, Prize, State};
use crate::lock::Locked;
use crate::logins::Role;
use crate::store::AuditRow;

/// One entrant, in the order they entered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EntrantView {
    #[schema(maximum = 4294967295_u32)]
    pub player_id: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ChampionshipView {
    #[schema(maximum = 4294967295_u32)]
    pub championship_id: u32,
    /// The lobby's `event_id`, which the game's requests carry.
    #[schema(maximum = 4294967295_u32)]
    pub event_id: u32,
    pub name: String,
    /// The banner's year (Japan time) and the Championship's number in it.
    #[schema(maximum = 4294967295_u32)]
    pub year: u32,
    #[schema(maximum = 4294967295_u32)]
    pub season: u32,
    pub entry_open: DateTime<Utc>,
    pub entry_close: DateTime<Utc>,
    pub start: DateTime<Utc>,
    /// Duels a player must have played to enter.
    #[schema(maximum = 4294967295_u32)]
    pub min_duels: u32,
    pub prize: Prize,
    pub state: State,
    pub entrants: Vec<EntrantView>,
    pub winner: Option<EntrantView>,
    pub second: Option<EntrantView>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChampionshipList {
    pub championships: Vec<ChampionshipView>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateChampionshipRequest {
    #[schema(min_length = 1, max_length = 60)]
    pub name: String,
    /// When entries open (omitted: now); they close at `entry_close` and the draw is at `start`, all on one day.
    #[serde(default)]
    pub entry_open: Option<DateTime<Utc>>,
    pub entry_close: DateTime<Utc>,
    pub start: DateTime<Utc>,
    #[serde(default)]
    #[schema(maximum = 4294967295_u32)]
    pub min_duels: u32,
    #[serde(default)]
    pub prize: Prize,
}

/// What to change in a Championship still waiting for its start; what is left out stays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditChampionshipRequest {
    #[schema(maximum = 4294967295_u32)]
    pub championship_id: u32,
    #[serde(default)]
    #[schema(min_length = 1, max_length = 60)]
    pub name: Option<String>,
    #[serde(default)]
    pub entry_open: Option<DateTime<Utc>>,
    #[serde(default)]
    pub entry_close: Option<DateTime<Utc>>,
    #[serde(default)]
    pub start: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(maximum = 4294967295_u32)]
    pub min_duels: Option<u32>,
    #[serde(default)]
    pub prize: Option<Prize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelChampionshipRequest {
    #[schema(maximum = 4294967295_u32)]
    pub championship_id: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChampionshipResult {
    pub championship: ChampionshipView,
    pub message: String,
}

fn at(seconds: i64) -> DateTime<Utc> { Utc.timestamp_opt(seconds, 0).single().unwrap_or_default() }

fn view(c: &Championship) -> ChampionshipView {
    let who = |p: &(u32, String)| EntrantView { player_id: p.0, name: p.1.clone() };
    ChampionshipView { championship_id: c.id, event_id: c.event_id(), name: c.name.clone(), year: c.year, season: c.season, entry_open: at(c.entry_open),
        entry_close: at(c.entry_close), start: at(c.start), min_duels: c.min_duels, prize: c.prize, state: c.state,
        entrants: c.entrants.iter().map(who).collect(), winner: c.winner.as_ref().map(who), second: c.second.as_ref().map(who) }
}

fn refuse(why: String) -> MutationError {
    let code = if why.starts_with("no Championship") { MutationErrorCode::NotFound } else if why.contains("already") || why.contains("around then") { MutationErrorCode::RequestConflict } else { MutationErrorCode::InvalidRequest };
    MutationError::new(code, format!("{}{}", why[..1].to_uppercase(), &why[1..]))
}

impl Api {
    /// Runs `f` on the running server's Championships, or on the data folder's when none runs.
    fn with_championships<T>(&self, f: impl FnOnce(&mut Championships) -> T) -> T {
        match (&self.hub, &self.data_dir) {
            (Some(hub), _) => f(&mut hub.locked().championships),
            (None, Some(dir)) => f(&mut Championships::at(dir)),
            (None, None) => f(&mut self.championship_memory.locked()),
        }
    }

    /// The Championships, the newest first.
    pub fn list_championships(&self, actor: Actor<'_>) -> Result<ChampionshipList, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let mut all = self.with_championships(|c| c.list.clone());
        all.sort_by(|a, b| b.start.cmp(&a.start).then(b.id.cmp(&a.id)));
        Ok(ChampionshipList { championships: all.iter().map(view).collect(), checked_at: Utc::now() })
    }

    fn audit_championship(&self, by: String, origin: &str, op: &str, c: &Championship, before: Option<&Championship>) {
        self.audit_row(AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: by, origin: origin.into(), op: op.into(),
            target: format!("Championship {} ({})", c.id, c.name), before: before.map(|b| json!(view(b))), after: Some(json!(view(c))),
            detail: Some(json!({ "championship_id": c.id })) });
    }

    /// Schedules a Championship. The same one made again (a retry) is answered as done.
    pub fn create_championship(&self, request: &CreateChampionshipRequest, actor: Actor<'_>, origin: &str) -> Result<ChampionshipResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Admin)?;
        let new = New { name: request.name.clone(), entry_open: request.entry_open.unwrap_or_else(Utc::now).timestamp(), entry_close: request.entry_close.timestamp(),
            start: request.start.timestamp(), min_duels: request.min_duels, prize: request.prize };
        let (c, fresh) = self.with_championships(|all| {
            let same = all.list.iter().find(|c| c.state == State::Scheduled && c.name == new.name.trim() && c.entry_close == new.entry_close && c.start == new.start
                && c.min_duels == new.min_duels && c.prize == new.prize).cloned();
            match same { Some(c) => Ok((c, false)), None => all.create(new).map(|c| (c, true)) }
        }).map_err(refuse)?;
        if fresh { self.audit_championship(identity.by, origin, "create-championship", &c, None) }
        let message = format!("Championship \"{}\" (Y.O.C. {} season {}) takes entries from {} to {} and starts at {} (id {}).", c.name, c.year, c.season,
            at(c.entry_open).format("%Y-%m-%d %H:%M UTC"), at(c.entry_close).format("%H:%M"), at(c.start).format("%H:%M UTC"), c.id);
        Ok(ChampionshipResult { championship: view(&c), message })
    }

    pub fn edit_championship(&self, request: &EditChampionshipRequest, actor: Actor<'_>, origin: &str) -> Result<ChampionshipResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Admin)?;
        let changes = Changes { name: request.name.clone(), entry_open: request.entry_open.map(|t| t.timestamp()), entry_close: request.entry_close.map(|t| t.timestamp()),
            start: request.start.map(|t| t.timestamp()), min_duels: request.min_duels, prize: request.prize };
        let (before, after) = self.with_championships(|all| all.edit(request.championship_id, &changes)).map_err(refuse)?;
        if before != after { self.audit_championship(identity.by, origin, "edit-championship", &after, Some(&before)) }
        Ok(ChampionshipResult { championship: view(&after), message: format!("Championship {} is changed.", after.id) })
    }

    /// Cancels a Championship that has not been drawn: the game stops listing it.
    pub fn cancel_championship(&self, request: &CancelChampionshipRequest, actor: Actor<'_>, origin: &str) -> Result<ChampionshipResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Admin)?;
        let before = self.with_championships(|all| all.cancel(request.championship_id)).map_err(refuse)?;
        let after = Championship { state: State::Cancelled, ..before.clone() };
        self.audit_championship(identity.by, origin, "cancel-championship", &after, Some(&before));
        Ok(ChampionshipResult { championship: view(&after), message: format!("Championship {} is cancelled.", after.id) })
    }
}
