//! BOOSTERPASS codes: prepare a copy, commit code/audit/receipt, then publish. No game push or external effect.
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{accounts::Directory, lock::Locked, logins::Role, store::{self, AuditRow, Receipt}};
use super::{Api, Actor, MutationError, MutationErrorCode, mutation::{Identity, fingerprint, validate_id}};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateBpCodeRequest {
    /// Omit for a generated code. Codes are case insensitive and stored in uppercase.
    #[schema(min_length = 16, max_length = 16, pattern = "^[A-Za-z0-9]{16}$")]
    pub code: Option<String>,
    #[schema(minimum = 1, maximum = 4294967295_u32)] pub bp: u32,
    /// Number of accounts that can claim it. Omit for unlimited; each account can claim it once.
    #[schema(minimum = 1, maximum = 4294967295_u32)] pub uses: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveBpCodeRequest {
    #[schema(min_length = 16, max_length = 16, pattern = "^[A-Za-z0-9]{16}$")] pub code: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateBpCodeResult {
    pub code: String, #[schema(maximum = 4294967295_u32)] pub bp: u32,
    #[schema(maximum = 4294967295_u32)] pub uses: Option<u32>, pub request_id: String, pub message: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RemoveBpCodeResult { pub removed: String, pub request_id: String, pub message: String }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BpCodeList { pub codes: Vec<BpCodeSummary> }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BpCodeSummary {
    pub code: String, #[schema(maximum = 4294967295_u32)] pub bp: u32,
    #[schema(maximum = 4294967295_u32)] pub uses: Option<u32>, pub claims: Vec<BpCodeClaim>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BpCodeClaim {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub players: Vec<String>, pub at: Option<chrono::DateTime<chrono::Utc>>,
}

fn normalize(code: &str) -> Result<String, MutationError> {
    let code = code.to_ascii_uppercase();
    if code.len() != crate::accounts::CODE_LEN || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(MutationError::new(MutationErrorCode::InvalidRequest, "A BOOSTERPASS code needs exactly 16 letters or digits."))
    }
    Ok(code)
}

impl Api {
    pub fn list_bp_codes(&self, actor: Actor<'_>) -> Result<BpCodeList, MutationError> {
        let accounts = self.accounts.locked();
        actor.resolve(&accounts, Role::Moderator)?;
        Ok(Self::bp_codes(&accounts))
    }

    pub(super) fn bp_codes(accounts: &Directory) -> BpCodeList {
        BpCodeList { codes: accounts.codes().iter().map(|c| BpCodeSummary {
            code: c.code.clone(), bp: c.bp, uses: c.uses,
            claims: c.used_by.iter().enumerate().map(|(i, &user_id)| BpCodeClaim { user_id, players: accounts.player_names(user_id),
                at: c.claimed_at.get(i).and_then(|t| t.as_ref()).and_then(|t| t.parse().ok()),
            }).collect(),
        }).collect() }
    }

    pub fn create_bp_code(&self, request: &CreateBpCodeRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<CreateBpCodeResult, MutationError> {
        validate_id(id)?;
        if request.bp == 0 || request.uses == Some(0) {
            return Err(MutationError::new(MutationErrorCode::InvalidRequest, "A code gives at least 1 BP, to at least one account."))
        }
        let request = CreateBpCodeRequest { code: request.code.as_deref().map(normalize).transpose()?, ..request.clone() };
        // Account -> store; this operation never acquires the hub, before or after taking the account lock.
        let mut accounts = self.accounts.locked();
        let identity = actor.resolve(&accounts, Role::Admin)?;
        let input = fingerprint(&request)?;
        if let Some(result) = self.replay(&identity.scope, id, "add-code", &input)? { return Ok(result) }
        let code = request.code.clone().unwrap_or_else(|| accounts.new_code());
        if accounts.codes().iter().any(|c| c.code.eq_ignore_ascii_case(&code)) {
            return Err(MutationError::new(MutationErrorCode::AlreadyExists, "This BOOSTERPASS code already exists."))
        }
        let mut next = accounts.prepare_add_code(&code, request.bp, request.uses)
            .map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?;
        let users = request.uses.map_or("any number of accounts".to_string(), |n| format!("{n} account(s)"));
        let result = CreateBpCodeResult { code: code.clone(), bp: request.bp, uses: request.uses, request_id: id.into(),
            message: format!("code {code}: {} BP, once per account, for {users}. Players enter it in Main Menu -> BOOSTERPASS.", request.bp) };
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by.clone(), origin: origin.into(),
            op: "add-code".into(), target: code.clone(), before: None, after: Some(json!({ "code": code, "bp": request.bp, "uses": request.uses })),
            detail: Some(json!({ "code": code, "bp": request.bp, "uses": request.uses, "request_id": id })) };
        self.commit_code(&mut next, &identity, id, input, &result, row)?;
        *accounts = next;
        tracing::info!(request_id = id, actor = %identity.scope, "[api] add-code committed");
        Ok(result)
    }

    pub fn remove_bp_code(&self, request: &RemoveBpCodeRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<RemoveBpCodeResult, MutationError> {
        validate_id(id)?;
        let request = RemoveBpCodeRequest { code: normalize(&request.code)? };
        let mut accounts = self.accounts.locked();
        let identity = actor.resolve(&accounts, Role::Admin)?;
        let input = fingerprint(&request)?;
        if let Some(result) = self.replay(&identity.scope, id, "remove-code", &input)? { return Ok(result) }
        let previous = accounts.codes().iter().find(|c| c.code.eq_ignore_ascii_case(&request.code))
            .ok_or_else(|| MutationError::new(MutationErrorCode::NotFound, "This BOOSTERPASS code does not exist."))?;
        let before = json!({ "code": previous.code, "bp": previous.bp, "uses": previous.uses, "claims": previous.used_by.len() });
        let mut next = accounts.prepare_remove_code(&request.code)
            .map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?;
        let result = RemoveBpCodeResult { removed: request.code.clone(), request_id: id.into(), message: format!("code {} removed", request.code) };
        let row = AuditRow { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by.clone(), origin: origin.into(),
            op: "remove-code".into(), target: request.code.clone(), before: Some(before), after: None,
            detail: Some(json!({ "code": request.code, "request_id": id })) };
        self.commit_code(&mut next, &identity, id, input, &result, row)?;
        *accounts = next;
        tracing::info!(request_id = id, actor = %identity.scope, "[api] remove-code committed");
        Ok(result)
    }

    fn commit_code(&self, next: &mut Directory, actor: &Identity, id: &str, input: String, result: &impl Serialize, row: AuditRow) -> Result<(), MutationError> {
        let receipt = Receipt { operation: row.op.clone(), input, result: serde_json::to_value(result).map_err(MutationError::persistence)? };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &actor.scope, id, &receipt) })
                .map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((actor.scope.clone(), id.into()), receipt); }
        }
        Ok(())
    }
}
