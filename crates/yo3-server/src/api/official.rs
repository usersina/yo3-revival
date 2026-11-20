//! Official Events (#18): the host schedules, edits and cancels the Stadium's events, and the staff sees the standings.
//! A running server's events are the hub's; with none running the data folder's. Each change is audited. Creating,
//! editing and cancelling is an admin's: the prizes are BP.
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use super::{Actor, Api, MutationError, MutationErrorCode};
use crate::lock::Locked;
use crate::logins::Role;
use crate::official::{Changes, Entry, Event, Kind, NewEvent, Official, Prize, State};
use crate::store::AuditRow;

/// Where an event is in time, for the staff (the game sees only its `status` number).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Scheduled,
    Running,
    Finished,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventView {
    #[schema(maximum = 4294967295_u32)]
    pub event_id: u32,
    pub kind: Kind,
    /// What the kind's score counts: win streak, wins or stars.
    pub counts: String,
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// The banlist entrants must log in with (a regulation id); none: any.
    #[schema(maximum = 4294967295_u32)]
    pub regulation: Option<u32>,
    pub prize: Prize,
    pub phase: Phase,
    /// The players who entered its lobby.
    #[schema(maximum = 4294967295_u32)]
    pub entrants: u32,
    /// The ten best standings.
    pub standings: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EventList {
    pub events: Vec<EventView>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateEventRequest {
    pub kind: Kind,
    #[schema(min_length = 1, max_length = 60)]
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// A banlist's id (`/panel/api/v1/banlists`); omitted: any.
    #[serde(default)]
    #[schema(maximum = 4294967295_u32)]
    pub regulation: Option<u32>,
    #[serde(default)]
    pub prize: Prize,
}

/// What to change in an event that hasn't finished; what is left out stays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditEventRequest {
    #[schema(maximum = 4294967295_u32)]
    pub event_id: u32,
    #[serde(default)]
    #[schema(min_length = 1, max_length = 60)]
    pub name: Option<String>,
    #[serde(default)]
    pub start: Option<DateTime<Utc>>,
    #[serde(default)]
    pub end: Option<DateTime<Utc>>,
    /// A banlist's id; 0 for any.
    #[serde(default)]
    #[schema(maximum = 4294967295_u32)]
    pub regulation: Option<u32>,
    #[serde(default)]
    pub prize: Option<Prize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelEventRequest {
    #[schema(maximum = 4294967295_u32)]
    pub event_id: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EventResult {
    pub event: EventView,
    pub message: String,
}

fn at(seconds: i64) -> DateTime<Utc> { Utc.timestamp_opt(seconds, 0).single().unwrap_or_default() }

fn view(e: &Event, now: i64) -> EventView {
    let phase = match e.state {
        State::Cancelled => Phase::Cancelled,
        _ if now >= e.end || e.state == State::Finished => Phase::Finished,
        _ if now < e.start => Phase::Scheduled,
        _ => Phase::Running,
    };
    EventView { event_id: e.id, kind: e.kind, counts: e.kind.counts().into(), name: e.name.clone(), start: at(e.start), end: at(e.end),
        regulation: e.regulation, prize: e.prize, phase, entrants: e.entries.len() as u32,
        standings: e.standings().into_iter().take(10).cloned().collect() }
}

fn refuse(why: String) -> MutationError {
    let code = if why.starts_with("no official event") { MutationErrorCode::NotFound } else if why.contains("already") { MutationErrorCode::RequestConflict } else { MutationErrorCode::InvalidRequest };
    MutationError::new(code, format!("{}{}", why[..1].to_uppercase(), &why[1..]))
}

impl Api {
    /// Runs `f` on the running server's official events, or on the data folder's when none runs.
    fn with_official<T>(&self, f: impl FnOnce(&mut Official) -> T) -> T {
        match (&self.hub, &self.data_dir) {
            (Some(hub), _) => f(&mut hub.locked().official),
            (None, Some(dir)) => f(&mut Official::at(dir)),
            (None, None) => f(&mut self.official_memory.locked()),
        }
    }

    /// The events, the newest start first; the staff's view (cancelled and long finished ones included).
    pub fn list_events(&self, actor: Actor<'_>) -> Result<EventList, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let now = Utc::now().timestamp();
        let mut events = self.with_official(|o| o.events.clone());
        events.sort_by(|a, b| b.start.cmp(&a.start).then(b.id.cmp(&a.id)));
        Ok(EventList { events: events.iter().map(|e| view(e, now)).collect(), checked_at: Utc::now() })
    }

    fn audit_event(&self, by: String, origin: &str, op: &str, event: &Event, before: Option<&Event>) {
        let now = Utc::now().timestamp();
        self.audit_row(AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: by, origin: origin.into(), op: op.into(),
            target: format!("event {} ({})", event.id, event.name), before: before.map(|b| json!(view(b, now))), after: Some(json!(view(event, now))),
            detail: Some(json!({ "event_id": event.id, "kind": event.kind })) });
    }

    /// Schedules an event. The same event made again (a retry) is answered as done.
    pub fn create_event(&self, request: &CreateEventRequest, actor: Actor<'_>, origin: &str) -> Result<EventResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Admin)?;
        let new = NewEvent { kind: request.kind, name: request.name.clone(), start: request.start.timestamp(), end: request.end.timestamp(),
            regulation: request.regulation, prize: request.prize };
        let (event, fresh) = self.with_official(|o| {
            let same = o.events.iter().find(|e| e.state == State::Active && e.kind == new.kind && e.start == new.start && e.end == new.end
                && e.name == new.name.trim()).cloned();
            match same { Some(e) => Ok((e, false)), None => o.create(new).map(|e| (e, true)) }
        }).map_err(refuse)?;
        if fresh { self.audit_event(identity.by, origin, "create-event", &event, None) }
        let message = format!("{} \"{}\" is scheduled from {} to {} (event {}).", event.kind.title(), event.name, at(event.start).format("%Y-%m-%d %H:%M UTC"), at(event.end).format("%Y-%m-%d %H:%M UTC"), event.id);
        Ok(EventResult { event: view(&event, Utc::now().timestamp()), message })
    }

    pub fn edit_event(&self, request: &EditEventRequest, actor: Actor<'_>, origin: &str) -> Result<EventResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Admin)?;
        let changes = Changes { name: request.name.clone(), start: request.start.map(|t| t.timestamp()), end: request.end.map(|t| t.timestamp()),
            regulation: request.regulation, prize: request.prize };
        let (before, after) = self.with_official(|o| o.edit(request.event_id, &changes)).map_err(refuse)?;
        if before != after { self.audit_event(identity.by, origin, "edit-event", &after, Some(&before)) }
        Ok(EventResult { event: view(&after, Utc::now().timestamp()), message: format!("Event {} is changed.", after.id) })
    }

    /// Cancels an event: the game stops listing it and nothing is paid. Players already matched finish their duels.
    pub fn cancel_event(&self, request: &CancelEventRequest, actor: Actor<'_>, origin: &str) -> Result<EventResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Admin)?;
        let before = self.with_official(|o| o.cancel(request.event_id)).map_err(refuse)?;
        let after = Event { state: State::Cancelled, ..before.clone() };
        self.audit_event(identity.by, origin, "cancel-event", &after, Some(&before));
        Ok(EventResult { event: view(&after, Utc::now().timestamp()), message: format!("Event {} is cancelled; nothing is paid.", after.id) })
    }
}

/// A time as the command line gives it: Unix seconds, `YYYY-MM-DD HH:MM` or `YYYY-MM-DDTHH:MM` (UTC), RFC 3339, `now`, or
/// `+N` minutes, hours (`h`), days (`d`) from now.
pub fn parse_time(text: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("now") { return Ok(now) }
    if let Some(rest) = text.strip_prefix('+') {
        let (digits, unit) = rest.split_at(rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len()));
        let n: i64 = digits.parse().map_err(|_| format!("{text:?} is not a time"))?;
        let span = match unit.trim() { "" | "m" => chrono::Duration::minutes(n), "h" => chrono::Duration::hours(n), "d" => chrono::Duration::days(n),
            _ => return Err(format!("{text:?}: use +30m, +2h or +7d")) };
        return Ok(now + span);
    }
    if let Ok(seconds) = text.parse::<i64>() { return Utc.timestamp_opt(seconds, 0).single().ok_or_else(|| format!("{text:?} is not a time")) }
    if let Ok(t) = DateTime::parse_from_rfc3339(text) { return Ok(t.with_timezone(&Utc)) }
    for format in ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(t) = chrono::NaiveDateTime::parse_from_str(text, format) { return Ok(t.and_utc()) }
    }
    Err(format!("{text:?} is not a time: use 2026-12-01 18:00 (UTC), an RFC 3339 time, +2h, or now"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_parse_as_the_command_line_gives_them() {
        let now = Utc.with_ymd_and_hms(2026, 12, 1, 12, 0, 0).unwrap();
        assert_eq!(parse_time("now", now).unwrap(), now);
        assert_eq!(parse_time("+2h", now).unwrap(), now + chrono::Duration::hours(2));
        assert_eq!(parse_time("+7d", now).unwrap(), now + chrono::Duration::days(7));
        assert_eq!(parse_time("2026-12-25 18:30", now).unwrap(), Utc.with_ymd_and_hms(2026, 12, 25, 18, 30, 0).unwrap());
        assert_eq!(parse_time("2026-12-25T18:30:00+09:00", now).unwrap(), Utc.with_ymd_and_hms(2026, 12, 25, 9, 30, 0).unwrap());
        assert!(parse_time("tomorrow", now).is_err());
    }
}
