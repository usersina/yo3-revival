//! Invite creation: prepare, commit invite/audit/receipt together, then publish. No external effects.
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{lock::Locked, logins::Role, store::{self, AuditRow, Receipt}};
use super::{Api, Actor, MutationError, MutationErrorCode, mutation::{fingerprint, validate_id}};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInviteRequest {
    /// Number of sign-ups. Omit for unlimited.
    #[schema(minimum = 1, maximum = 4294967295_u32)] pub uses: Option<u32>,
    /// Days from the first successful creation. Omit for no expiry; retries keep the original expiry.
    #[schema(minimum = 1, maximum = 4294967295_u32)] pub days: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateInviteResult {
    pub code: String, #[schema(maximum = 4294967295_u32)] pub uses: Option<u32>,
    pub expires_at: Option<DateTime<Utc>>, pub request_id: String, pub message: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InviteList { pub invites: Vec<InviteSummary>, pub checked_at: DateTime<Utc> }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InviteSummary {
    pub code: String,
    /// Remaining sign-ups, not the original limit. Null means unlimited.
    #[schema(maximum = 4294967295_u32)] pub uses: Option<u32>,
    pub expires_at: Option<DateTime<Utc>>, pub issued_by: String, pub claims: Vec<InviteClaim>, pub state: InviteState,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InviteClaim { #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub players: Vec<String> }
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InviteState { Usable, Exhausted, Expired }

// RFC 3339 and the generated browser validators require four-digit years, unlike chrono's extended range.
fn expiry(timestamp: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp, 0).filter(|t| (0..=9999).contains(&t.year()))
}

impl Api {
    pub fn list_invites(&self, actor: Actor<'_>) -> Result<InviteList, MutationError> {
        let accounts = self.accounts.locked();
        actor.resolve(&accounts, Role::Moderator)?;
        let checked_at = Utc::now();
        let invites = accounts.invites().iter().map(|i| Ok(InviteSummary {
            code: i.code.clone(), uses: i.uses, expires_at: i.expires.map(|t| expiry(t).ok_or_else(|| MutationError::new(
                MutationErrorCode::PersistenceFailed, "An invite has an unsupported expiry. Ask the host to review the stored invite data."))).transpose()?, issued_by: i.issued_by.clone(),
            claims: i.used_by.iter().map(|&user_id| InviteClaim { user_id, players: accounts.player_names(user_id) }).collect(),
            state: if i.expires.is_some_and(|t| t <= checked_at.timestamp()) { InviteState::Expired }
                else if i.uses == Some(0) { InviteState::Exhausted } else { InviteState::Usable },
        })).collect::<Result<Vec<_>, MutationError>>()?;
        Ok(InviteList { checked_at, invites })
    }

    pub fn create_invite(&self, request: &CreateInviteRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<CreateInviteResult, MutationError> {
        validate_id(id)?;
        if request.uses == Some(0) || request.days == Some(0) {
            return Err(MutationError::new(MutationErrorCode::InvalidRequest, "An invite needs at least one sign-up and one day; omit either limit for unlimited."))
        }
        // Account -> store; never acquire the hub while holding this lock.
        let mut accounts = self.accounts.locked();
        let identity = actor.resolve(&accounts, Role::Moderator)?;
        let input = fingerprint(request)?;
        // Check the receipt before calculating relative expiry. A retry never extends an invite.
        if let Some(result) = self.replay(&identity.scope, id, "invite", &input)? { return Ok(result) }
        let expires_at = request.days.map(|days| expiry(crate::logins::now() + i64::from(days) * 86400)
            .ok_or_else(|| MutationError::new(MutationErrorCode::InvalidRequest, "The invite expiry is outside the supported date range."))).transpose()?;
        let (mut next, code) = accounts.prepare_invite(request.uses, expires_at.map(|t| t.timestamp()), &identity.by);
        let users = request.uses.map_or("any number of sign-ups".to_string(), |n| format!("{n} sign-up(s)"));
        let until = expires_at.map_or("with no end date".to_string(), |t| format!("until {}", t.to_rfc3339()));
        let result = CreateInviteResult { code: code.clone(), uses: request.uses, expires_at, request_id: id.into(),
            message: format!("invite code {code}: {users}, {until}. Players enter it when they make their account in the launcher.") };
        let row = AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by.clone(), origin: origin.into(),
            op: "invite".into(), target: code.clone(), before: None,
            after: Some(json!({ "code": code, "uses": request.uses, "expires_at": expires_at, "issued_by": identity.by })),
            detail: Some(json!({ "uses": request.uses, "days": request.days, "request_id": id })) };
        let receipt = Receipt { operation: "invite".into(), input, result: serde_json::to_value(&result).map_err(MutationError::persistence)? };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) })
                .map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope.clone(), id.into()), receipt); }
        }
        *accounts = next;
        tracing::info!(request_id = id, actor = %identity.scope, "[api] invite committed");
        Ok(result)
    }
}
