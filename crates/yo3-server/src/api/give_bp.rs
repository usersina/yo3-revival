//! Give BP: prepare copies, commit balance/mail/audit/receipt together, then publish under the original locks.
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::Api;
use crate::{lock::Locked, logins::Role, mail::Mailbox, store::{self, AuditRow, Receipt}};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiveBpRequest { pub who: String, pub amount: u32 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GiveBpResult { pub user_id: u32, pub bp: u32, pub message: String, pub request_id: String }

/// Identity comes from the authenticated transport. A host's display name never selects its receipt scope.
pub enum GrantActor<'a> { Host { by: &'a str }, Staff { user_id: u32 } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantErrorCode { InvalidRequest, Forbidden, NotFound, RequestConflict, PersistenceFailed }

#[derive(Debug, Serialize, Deserialize)]
pub struct GrantError { pub code: GrantErrorCode, pub message: String }

impl GrantError {
    fn new(code: GrantErrorCode, message: impl Into<String>) -> Self { Self { code, message: message.into() } }
    fn persistence(error: impl fmt::Display) -> Self {
        tracing::error!("BP grant storage failed: {error}");
        Self::new(GrantErrorCode::PersistenceFailed, "The grant could not be confirmed. Retry with the same request ID.")
    }
}
impl fmt::Display for GrantError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) } }
impl std::error::Error for GrantError {}

/// Safe both in a header and as a receipt key. Receipts have no expiry and IDs are never reused.
pub fn valid_request_id(id: &str) -> bool {
    (16..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl Api {
    pub fn give_bp(&self, request: &GiveBpRequest, request_id: &str, actor: GrantActor<'_>, origin: &str) -> Result<GiveBpResult, GrantError> {
        if !valid_request_id(request_id) {
            return Err(GrantError::new(GrantErrorCode::InvalidRequest, "A request ID needs 16–128 letters, digits, hyphens or underscores."))
        }
        if request.amount == 0 { return Err(GrantError::new(GrantErrorCode::InvalidRequest, "give at least 1 BP")) }
        // Hub -> accounts -> store, as duel results and other live changes do. Keep these guards through publishing;
        // another request cannot see a prepared copy or race a receipt check with its balance change.
        let mut hub = self.hub.as_ref().map(|h| h.locked());
        let mut accounts = self.accounts.locked();
        let (scope, by) = match actor {
            GrantActor::Host { by } => ("host".into(), by.to_string()),
            GrantActor::Staff { user_id } => {
                let login = accounts.login_by_id(user_id).filter(|l| l.role == Role::Admin && l.refusal(crate::logins::now()).is_none())
                    .ok_or_else(|| GrantError::new(GrantErrorCode::Forbidden, "only an admin may give BP"))?;
                (format!("staff:{user_id}"), login.user.clone())
            }
        };
        let span = tracing::info_span!("give-bp", request_id, actor = %scope);
        let _entered = span.enter();
        // Retained receipts need the input's fingerprint, not a player's name (account deletion scrubs the audit).
        let canonical = serde_json::to_string(request).map_err(GrantError::persistence)?;
        let input = serde_json::to_string(&crate::logins::digest(&canonical)).map_err(GrantError::persistence)?;
        let previous = match &self.data_dir {
            Some(dir) => store::receipt(dir, &scope, request_id).map_err(GrantError::persistence)?,
            None => self.grant_receipts.locked().get(&(scope.clone(), request_id.into())).cloned(),
        };
        if let Some(receipt) = previous {
            if receipt.operation != "give-bp" || receipt.input != input {
                return Err(GrantError::new(GrantErrorCode::RequestConflict, "This request ID was already used for a different change."))
            }
            return serde_json::from_value(receipt.result).map_err(GrantError::persistence)
        }
        let user = accounts.user_of(&request.who).ok_or_else(|| GrantError::new(GrantErrorCode::NotFound,
            format!("no account with the User ID or a player named {:?}", request.who)))?;
        let before = accounts.dpoint(user).ok_or_else(|| GrantError::new(GrantErrorCode::NotFound, "The account is gone."))?;
        if before.checked_add(request.amount).is_none() {
            return Err(GrantError::new(GrantErrorCode::InvalidRequest, "This grant would exceed the BP balance limit."))
        }
        let players = accounts.named_players_of(&request.who);
        let (mut next, _, total) = accounts.prepare_bp(&request.who, request.amount)
            .map_err(|e| GrantError::new(GrantErrorCode::InvalidRequest, e))?;
        let source = match hub.as_ref() {
            Some(h) => h.bp_mail(),
            None => match &self.data_dir {
                Some(dir) => Mailbox::try_load(&dir.join("mail.json")).map_err(GrantError::persistence)?, None => Mailbox::default(),
            },
        };
        let mail = source.prepare_bp(&players, request.amount, total);
        let result = GiveBpResult { user_id: user, bp: total, message: format!("gave {} BP to account #{user}: {total} BP now", request.amount), request_id: request_id.into() };
        let receipt = Receipt { operation: "give-bp".into(), input, result: serde_json::to_value(&result).map_err(GrantError::persistence)? };
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: by, origin: origin.into(),
            op: "give-bp".into(), target: request.who.clone(), before: Some(json!(before)), after: Some(json!(total)),
            detail: Some(json!({ "who": request.who, "amount": request.amount, "request_id": request_id })) };
        match &self.data_dir {
            Some(dir) => store::together(dir, || {
                next.persist()?;
                mail.persist()?;
                store::audit(dir, &row)?;
                store::put_receipt(dir, &scope, request_id, &receipt)
            }).map_err(GrantError::persistence)?,
            None => {
                self.memory.locked().push(row);
                self.grant_receipts.locked().insert((scope, request_id.into()), receipt);
            }
        }
        *accounts = next;
        if let Some(hub) = hub.as_mut() { hub.publish_bp(mail, &players, total) }
        tracing::info!(user_id = user, amount = request.amount, bp = total, "[api] give-bp committed");
        Ok(result)
    }
}
