//! Player reports (#44): the staff lists them and marks them handled or dismissed. A moderator may.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use super::{Actor, Api, MutationError, MutationErrorCode};
use crate::lock::Locked;
use crate::logins::Role;
use crate::reports::{Report, Reports, Status};
use crate::store::AuditRow;

/// A report with the accounts of the two players, for the page's links.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReportView {
    #[serde(flatten)]
    pub report: Report,
    /// The account of the reporting player (none once deleted).
    #[schema(maximum = 4294967295_u32)]
    pub reporter_user_id: Option<u32>,
    #[schema(maximum = 4294967295_u32)]
    pub target_user_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReportList {
    pub reports: Vec<ReportView>,
    /// Open reports in all, whatever the list shows.
    #[schema(maximum = 4294967295_u32)]
    pub open: u32,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ReportQuery {
    /// open, handled or dismissed; all when omitted.
    pub status: Option<Status>,
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HandleReportRequest {
    #[schema(maximum = 4294967295_u32)]
    pub report_id: u32,
    /// handled or dismissed.
    pub status: Status,
    /// What was done, shown with the report (optional).
    #[schema(max_length = 300)]
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HandleReportResult {
    pub report: ReportView,
    pub message: String,
}

impl Api {
    /// Runs `f` on the running server's reports, or on the data folder's when none runs.
    fn with_reports<T>(&self, f: impl FnOnce(&mut Reports) -> T) -> T {
        match (&self.hub, &self.data_dir) {
            (Some(hub), _) => f(&mut hub.locked().reports),
            (None, Some(dir)) => f(&mut Reports::at(dir)),
            (None, None) => f(&mut Reports::default()),
        }
    }

    fn view(&self, report: Report) -> ReportView {
        let d = self.accounts.locked();
        let user_of = |player: u32| d.entries().find(|(_, _, a)| a.players.iter().any(|p| p.player_id == player)).map(|(_, _, a)| a.user_id);
        ReportView { reporter_user_id: user_of(report.reporter_id), target_user_id: user_of(report.target_id), report }
    }

    pub fn list_reports(&self, query: &ReportQuery, actor: Actor<'_>) -> Result<ReportList, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let limit = query.limit.unwrap_or(100).clamp(1, 500);
        let (reports, open) = self.with_reports(|r| (r.list(query.status, limit), r.open()));
        let reports = reports.map_err(MutationError::persistence)?;
        Ok(ReportList { reports: reports.into_iter().map(|r| self.view(r)).collect(), open, checked_at: Utc::now() })
    }

    /// Marks a report handled or dismissed, with a note, and audits it. The same change again is answered as done.
    pub fn handle_report(&self, request: &HandleReportRequest, actor: Actor<'_>, origin: &str) -> Result<HandleReportResult, MutationError> {
        let identity = actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let done = self.with_reports(|r| {
            if let Ok(Some(old)) = r.get(request.report_id)
                && old.status == request.status && old.handled_by.as_deref() == Some(identity.by.as_str())
            {
                return Ok((old.clone(), old, true));
            }
            r.resolve(request.report_id, request.status, &identity.by, &request.note).map(|(b, a)| (b, a, false))
        });
        let (before, after, repeat) = done.map_err(|e| {
            let code = if e.starts_with("no report") { MutationErrorCode::NotFound } else if e.contains("already") { MutationErrorCode::RequestConflict } else { MutationErrorCode::InvalidRequest };
            MutationError::new(code, format!("{}{}", e[..1].to_uppercase(), &e[1..]))
        })?;
        if !repeat {
            self.audit_row(AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by.clone(), origin: origin.into(),
                op: "handle-report".into(), target: after.target.clone(), before: Some(json!({ "report": before.id, "status": before.status })),
                after: Some(json!({ "report": after.id, "status": after.status })), detail: Some(json!({ "id": request.report_id, "status": request.status, "note": after.note })) });
        }
        let message = format!("Report #{} about {} is {}.", after.id, after.target, serde_json::to_value(after.status).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default());
        Ok(HandleReportResult { report: self.view(after), message })
    }

    /// Writes one audit row, to the database or (tests, no data folder) the in-memory log.
    pub(super) fn audit_row(&self, row: AuditRow) {
        match &self.data_dir {
            Some(dir) => if let Err(e) = crate::store::audit(dir, &row) { tracing::error!("audit row not written ({}): {e}", row.op) },
            None => self.memory.locked().push(row),
        }
    }
}
