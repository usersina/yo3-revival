//! Caller-generated, one-time setup/reset codes. Durable receipts never hold usable credentials.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{accounts::Directory, lock::Locked, logins::{self, Role}, store::{self, AuditRow, Receipt}};
use super::{Api, Actor, MutationError, MutationErrorCode, Op, mutation::{fingerprint, validate_id}};

#[derive(Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String, min_length = 16, max_length = 19, pattern = "^([A-Za-z0-9]{16}|[A-Za-z0-9]{4}(-[A-Za-z0-9]{4}){3})$")]
pub struct AccessCode(String);
impl From<String> for AccessCode { fn from(value: String) -> Self { Self(value) } }
impl std::fmt::Debug for AccessCode { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("[REDACTED]") } }
impl AccessCode {
    pub fn display(&self) -> &str { &self.0 }
    pub fn validate(&self) -> Result<(), MutationError> { self.canonical().map(|_| ()) }
    fn canonical(&self) -> Result<Self, MutationError> {
        let s = self.0.as_bytes();
        let valid = match s.len() {
            16 => s.iter().all(u8::is_ascii_alphanumeric),
            19 => s.iter().enumerate().all(|(i, c)| if [4, 9, 14].contains(&i) { *c == b'-' } else { c.is_ascii_alphanumeric() }),
            _ => false,
        };
        if !valid { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "Use a generated 16-character setup or reset code.")) }
        Ok(Self(logins::normalize_code(&self.0)))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IssueAccountCodeRequest {
    CreateAccount { #[schema(min_length = 3, max_length = 16, pattern = "^[A-Za-z0-9]+$")] user: String,
        #[schema(write_only)] code: AccessCode },
    ResetPassword { #[schema(maximum = 4294967295_u32)] user_id: u32, #[schema(write_only)] code: AccessCode },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountCodeKind { Setup, Reset }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AccountCodeConfirmation {
    pub kind: AccountCodeKind, #[schema(maximum = 4294967295_u32)] pub user_id: u32,
    pub expires_at: DateTime<Utc>, pub request_id: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountCodeState { Ready, Expired, UsedOrReplaced }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AccountCodeResponse {
    /// Immutable issuance receipt. A retry never creates or extends a code.
    pub confirmation: AccountCodeConfirmation,
    /// Live availability, checked now; not part of the stored receipt.
    pub state: AccountCodeState,
}
#[derive(Serialize)]
enum Input { Create { user: String, code: AccessCode }, Reset { user_id: u32, code: AccessCode }, NamedReset { user: String, code: AccessCode } }
impl Input {
    fn operation(&self) -> &'static str { if matches!(self, Self::Create { .. }) { "add-user" } else { "reset-code" } }
    fn code(&self) -> &AccessCode { match self { Self::Create { code, .. } | Self::Reset { code, .. } | Self::NamedReset { code, .. } => code } }
}
fn missing() -> MutationError { MutationError::new(MutationErrorCode::NotFound, "The password account no longer exists.") }
fn permission(d: &Directory, actor: Actor<'_>, target: u32) -> Result<(), MutationError> {
    actor.resolve(d, Role::Moderator)?;
    if let Actor::Staff { user_id } = actor && d.login_by_id(user_id).is_some_and(|l| l.role == Role::Moderator)
        && user_id != target && d.login_by_id(target).is_some_and(|l| l.staff()) {
        return Err(MutationError::new(MutationErrorCode::Forbidden, "Moderators cannot reset another staff account's password."))
    }
    Ok(())
}
fn response(d: &Directory, confirmation: AccountCodeConfirmation, code: &AccessCode) -> AccountCodeResponse {
    let now = logins::now();
    let ready = d.login_by_id(confirmation.user_id).and_then(|l| l.reset.as_ref()).is_some_and(|(digest, until)|
        *digest == logins::digest(code.display()) && *until == confirmation.expires_at.timestamp());
    let state = if confirmation.expires_at.timestamp() <= now { AccountCodeState::Expired }
        else if ready { AccountCodeState::Ready } else { AccountCodeState::UsedOrReplaced };
    AccountCodeResponse { confirmation, state }
}
impl Api {
    pub fn issue_account_code(&self, request: &IssueAccountCodeRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<AccountCodeResponse, MutationError> {
        let input = match request {
            // The User ID is kept as typed, like sign-up's: lookups ignore case (`Directory::login_at`).
            IssueAccountCodeRequest::CreateAccount { user, code } => Input::Create { user: user.trim().to_owned(), code: code.canonical()? },
            IssueAccountCodeRequest::ResetPassword { user_id, code } => Input::Reset { user_id: *user_id, code: code.canonical()? },
        };
        self.issue_code(input, id, actor, origin)
    }
    pub(super) fn issue_named_code(&self, op: &Op, id: &str, actor: Actor<'_>, origin: &str) -> Result<serde_json::Value, MutationError> {
        let (user, code, create) = match op {
            Op::AddUser { user, code } => (user, code, true), Op::ResetCode { user, code } => (user, code, false),
            _ => return Err(MutationError::new(MutationErrorCode::InvalidRequest, "Unsupported credential operation.")),
        };
        let display_user = user.trim();
        let user = display_user.to_owned();
        let input = if create { Input::Create { user: user.clone(), code: code.canonical()? } } else { Input::NamedReset { user: user.clone(), code: code.canonical()? } };
        let result = self.issue_code(input, id, actor, origin)?;
        let mut value = json!(result);
        // Reconstruct this caller's code for existing CLI/HTML presentation. It is never stored in a receipt.
        value["code"] = json!(code.display());
        let state = match result.state { AccountCodeState::Ready => "ready", AccountCodeState::Expired => "expired", AccountCodeState::UsedOrReplaced => "used, replaced or invalidated" };
        value["message"] = json!(format!("{} code for {display_user}: {}. State: {state}; expires {}. Use Forgot password in the launcher to choose a password.",
            if create { "Setup" } else { "Reset" }, code.display(), result.confirmation.expires_at.to_rfc3339()));
        Ok(value)
    }
    fn issue_code(&self, input: Input, id: &str, actor: Actor<'_>, origin: &str) -> Result<AccountCodeResponse, MutationError> {
        validate_id(id)?;
        let create = matches!(input, Input::Create { .. });
        let operation = input.operation();
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, if create { Role::Admin } else { Role::Moderator })?;
        let hash = fingerprint(&input)?;
        if let Some(original) = self.replay::<AccountCodeConfirmation>(&identity.scope, id, operation, &hash)? {
            if !create { permission(&d, actor, original.user_id)?; }
            return Ok(response(&d, original, input.code()))
        }
        let expires_at = DateTime::from_timestamp(logins::now() + logins::RESET_TTL, 0).expect("24-hour expiry");
        let digest = logins::digest(input.code().display());
        let (mut next, user_id, label, kind) = match &input {
            Input::Create { user, .. } => {
                if !logins::valid_user_id(user) { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "User IDs need 3–16 letters or digits.")) }
                if d.login_of(user).is_some() { return Err(MutationError::new(MutationErrorCode::AlreadyExists, "That User ID is already taken.")) }
                let (next, user_id) = d.prepare_password_account(user, digest, expires_at.timestamp()).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?;
                (next, user_id, user.clone(), AccountCodeKind::Setup)
            }
            Input::Reset { .. } | Input::NamedReset { .. } => {
                let user_id = match &input { Input::Reset { user_id, .. } => *user_id, Input::NamedReset { user, .. } => d.login_of(user).map(|(id, _)| id).ok_or_else(missing)?, _ => unreachable!() };
                if d.account_of(user_id).is_none() { return Err(missing()) }
                permission(&d, actor, user_id)?;
                let label = d.login_by_id(user_id).ok_or_else(|| MutationError::new(MutationErrorCode::InvalidRequest, "Key accounts do not have passwords."))?.user.clone();
                (d.prepare_reset(user_id, digest, expires_at.timestamp()).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?, user_id, label, AccountCodeKind::Reset)
            }
        };
        let confirmation = AccountCodeConfirmation { kind, user_id, expires_at, request_id: id.into() };
        let row = AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by,
            origin: origin.into(), op: operation.into(), target: label.clone(), before: None,
            after: Some(json!({ "user_id": user_id, "expires_at": expires_at })), detail: Some(json!({ "who": label, "request_id": id })) };
        let receipt = Receipt { operation: operation.into(), input: hash, result: json!(confirmation) };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) }).map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope, id.into()), receipt); }
        }
        *d = next;
        Ok(response(&d, confirmation, input.code()))
    }
}
