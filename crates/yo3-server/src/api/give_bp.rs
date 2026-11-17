//! Give BP: prepare copies, commit balance/mail/audit/receipt together, then publish under the original locks.
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Api, Actor, MutationError, MutationErrorCode, mutation::{fingerprint, validate_id}};
use crate::{lock::Locked, logins::Role, mail::Mailbox, store::{self, AuditRow, Receipt}};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GiveBpRequest { #[schema(min_length = 1)] pub who: String, #[schema(minimum = 1, maximum = 4294967295_u32)] pub amount: u32 }

/// The dashboard always targets the stable account number, never a renameable player or login label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GiveBpByIdRequest {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32,
    #[schema(minimum = 1, maximum = 4294967295_u32)] pub amount: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GiveBpResult { #[schema(maximum = 4294967295_u32)] pub user_id: u32, #[schema(maximum = 4294967295_u32)] pub bp: u32, pub message: String, pub request_id: String }

impl Api {
    pub fn give_bp(&self, request: &GiveBpRequest, request_id: &str, actor: Actor<'_>, origin: &str) -> Result<GiveBpResult, MutationError> {
        self.grant_bp(request, None, request_id, actor, origin)
    }

    pub fn give_bp_to_account(&self, request: &GiveBpByIdRequest, request_id: &str, actor: Actor<'_>, origin: &str) -> Result<GiveBpResult, MutationError> {
        self.grant_bp(&GiveBpRequest { who: format!("#{}", request.user_id), amount: request.amount }, Some(request.user_id), request_id, actor, origin)
    }

    fn grant_bp(&self, request: &GiveBpRequest, target: Option<u32>, request_id: &str, actor: Actor<'_>, origin: &str) -> Result<GiveBpResult, MutationError> {
        validate_id(request_id)?;
        if request.amount == 0 { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "give at least 1 BP")) }
        // Hub -> accounts -> store, as duel results and other live changes do. Keep these guards through publishing;
        // another request cannot see a prepared copy or race a receipt check with its balance change.
        let mut hub = self.hub.as_ref().map(|h| h.locked());
        let mut accounts = self.accounts.locked();
        let identity = actor.resolve(&accounts, Role::Admin)?;
        let (scope, by) = (identity.scope, identity.by);
        let span = tracing::info_span!("give-bp", request_id, actor = %scope);
        let _entered = span.enter();
        // Retained receipts need the input's fingerprint, not a player's name (account deletion scrubs the audit).
        let input = match target {
            Some(user_id) => fingerprint(&GiveBpByIdRequest { user_id, amount: request.amount }),
            None => fingerprint(request),
        }?;
        if let Some(result) = self.replay(&scope, request_id, "give-bp", &input)? { return Ok(result) }
        let user = target.or_else(|| accounts.user_of(&request.who)).ok_or_else(|| MutationError::new(MutationErrorCode::NotFound,
            format!("no account with the User ID or a player named {:?}", request.who)))?;
        let before = accounts.dpoint(user).ok_or_else(|| MutationError::new(MutationErrorCode::NotFound, "The account is gone."))?;
        if before.checked_add(request.amount).is_none() {
            return Err(MutationError::new(MutationErrorCode::InvalidRequest, "This grant would exceed the BP balance limit."))
        }
        let players = accounts.account_of(user).ok_or_else(|| MutationError::new(MutationErrorCode::NotFound, "The account is gone."))?
            .players.into_iter().map(|p| (p.player_id, p.name)).collect::<Vec<_>>();
        let (mut next, total) = accounts.prepare_bp(user, request.amount)
            .map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?;
        let source = match hub.as_ref() {
            Some(h) => h.bp_mail(),
            None => match &self.data_dir {
                Some(dir) => Mailbox::try_load(&dir.join("mail.json")).map_err(MutationError::persistence)?, None => Mailbox::default(),
            },
        };
        let mail = source.prepare_bp(&players, request.amount, total);
        let result = GiveBpResult { user_id: user, bp: total, message: format!("gave {} BP to account #{user}: {total} BP now", request.amount), request_id: request_id.into() };
        let receipt = Receipt { operation: "give-bp".into(), input, result: serde_json::to_value(&result).map_err(MutationError::persistence)? };
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: by, origin: origin.into(),
            op: "give-bp".into(), target: request.who.clone(), before: Some(json!(before)), after: Some(json!(total)),
            detail: Some(json!({ "who": request.who, "amount": request.amount, "request_id": request_id })) };
        match &self.data_dir {
            Some(dir) => store::together(dir, || {
                next.persist()?;
                mail.persist()?;
                store::audit(dir, &row)?;
                store::put_receipt(dir, &scope, request_id, &receipt)
            }).map_err(MutationError::persistence)?,
            None => {
                self.memory.locked().push(row);
                self.receipts.locked().insert((scope, request_id.into()), receipt);
            }
        }
        *accounts = next;
        if let Some(hub) = hub.as_mut() { hub.publish_bp(mail, &players, total) }
        tracing::info!(user_id = user, amount = request.amount, bp = total, "[api] give-bp committed");
        Ok(result)
    }
}
