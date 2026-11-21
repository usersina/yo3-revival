//! Players' problem reports (problems.rs) for the dashboard: read only, a moderator may.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{Actor, Api, MutationError, MutationErrorCode};
use crate::lock::Locked;
use crate::logins::Role;
use crate::problems::{Listed, Problems};

/// A report in a list: everything but its logs. (Own schema names: `Report` is the in-game player report.)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProblemReportRow {
    /// R-XXXX-XXXX
    pub id: String,
    /// When it came, in Unix seconds.
    pub at: i64,
    #[schema(maximum = 4294967295_u32)]
    pub user_id: u32,
    /// The account's User ID, or its players' names for a key account.
    pub account: String,
    /// network, relay, server-lost, game-error or report.
    pub kind: String,
    /// The launcher's version.
    pub launcher: String,
    /// One line on what happened, from the launcher.
    pub summary: String,
    /// The size of its logs in bytes.
    pub bytes: u64,
}

/// One report with the server's lines and the launcher's log.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProblemReport {
    #[serde(flatten)]
    pub row: ProblemReportRow,
    /// The server's own lines about this account from the minutes before.
    pub server_log: Vec<String>,
    /// The launcher's session log, credentials scrubbed.
    pub logs: String,
    /// The whole report as the text file staff download.
    pub text: String,
}

impl From<Listed> for ProblemReportRow {
    fn from(l: Listed) -> Self {
        Self { id: l.id, at: l.at, user_id: l.user_id, account: l.account, kind: l.kind, launcher: l.launcher, summary: l.summary, bytes: l.bytes }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProblemReportList {
    /// Newest first, without their logs.
    pub reports: Vec<ProblemReportRow>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ProblemReportQuery {
    /// A User ID or a player's name: only that account's reports.
    #[param(max_length = 64)]
    pub who: Option<String>,
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
}

/// `R-XXXX-XXXX` in any case.
pub fn valid_id(id: &str) -> bool {
    let id = id.trim();
    id.len() == 11 && id.bytes().enumerate().all(|(i, b)| if i == 0 { b.eq_ignore_ascii_case(&b'R') } else if i == 1 || i == 6 { b == b'-' } else { b.is_ascii_alphanumeric() })
}

impl Api {
    pub fn list_problem_reports(&self, query: &ProblemReportQuery, actor: Actor<'_>) -> Result<ProblemReportList, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let limit = query.limit.unwrap_or(100);
        if !(1..=500).contains(&limit) { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "Choose 1–500 reports.")) }
        let user = match query.who.as_deref().map(str::trim).filter(|w| !w.is_empty()) {
            Some(who) => Some(self.accounts.locked().user_of(who)
                .ok_or_else(|| MutationError::new(MutationErrorCode::NotFound, format!("No account with the User ID or a player named {who:?}.")))?),
            None => None,
        };
        let reports = Problems::new(self.data_dir.as_deref()).list(user, limit as usize);
        Ok(ProblemReportList { reports: reports.into_iter().map(Into::into).collect(), checked_at: Utc::now() })
    }

    pub fn read_problem_report(&self, id: &str, actor: Actor<'_>) -> Result<ProblemReport, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        if !valid_id(id) { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "A report ID looks like R-7K3Q-M2PA.")) }
        let report = Problems::new(self.data_dir.as_deref()).get(id)
            .ok_or_else(|| MutationError::new(MutationErrorCode::NotFound, "No problem report has that ID."))?;
        let text = report.text();
        let row = ProblemReportRow::from(report.listed());
        Ok(ProblemReport { row, server_log: report.server_log, logs: report.logs, text })
    }
}
