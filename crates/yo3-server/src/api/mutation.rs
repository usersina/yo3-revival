//! Shared authenticated identity, errors and receipt lookup for the typed admin operations.
use std::fmt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use crate::{accounts::Directory, lock::Locked, logins::Role, store};
use super::Api;

/// Supplied by the authenticated transport, never by a request body or a host's audit display name.
#[derive(Clone, Copy)]
pub enum Actor<'a> { Host { by: &'a str }, Staff { user_id: u32 } }
pub(super) struct Identity { pub scope: String, pub by: String }
impl Actor<'_> {
    pub(super) fn resolve(self, accounts: &Directory, required: Role) -> Result<Identity, MutationError> {
        match self {
            Self::Host { by } => Ok(Identity { scope: "host".into(), by: by.into() }),
            Self::Staff { user_id } => {
                let login = accounts.login_by_id(user_id).filter(|l| l.staff() && l.refusal(crate::logins::now()).is_none()
                    && (required == Role::Moderator || l.role == Role::Admin))
                    .ok_or_else(|| MutationError::new(MutationErrorCode::Forbidden, "This account does not have permission for this operation."))?;
                Ok(Identity { scope: format!("staff:{user_id}"), by: login.user.clone() })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationErrorCode { InvalidRequest, Forbidden, NotFound, AlreadyExists, RequestConflict, PersistenceFailed }
#[derive(Debug, Serialize, Deserialize)]
pub struct MutationError { pub code: MutationErrorCode, pub message: String }
impl MutationError {
    pub(super) fn new(code: MutationErrorCode, message: impl Into<String>) -> Self { Self { code, message: message.into() } }
    pub(super) fn persistence(error: impl fmt::Display) -> Self {
        tracing::error!("Administrative change storage failed: {error}");
        Self::new(MutationErrorCode::PersistenceFailed, "The change could not be confirmed. Retry with the same request ID.")
    }
}
impl fmt::Display for MutationError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) } }
impl std::error::Error for MutationError {}

/// Safe in a header and as a receipt key. Successful receipts never expire or follow audit pruning.
pub fn valid_request_id(id: &str) -> bool {
    (16..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub(super) fn validate_id(id: &str) -> Result<(), MutationError> {
    valid_request_id(id).then_some(()).ok_or_else(|| MutationError::new(MutationErrorCode::InvalidRequest,
        "A request ID needs 16–128 letters, digits, hyphens or underscores."))
}
pub(super) fn fingerprint(input: &impl Serialize) -> Result<String, MutationError> {
    let canonical = serde_json::to_string(input).map_err(MutationError::persistence)?;
    serde_json::to_string(&crate::logins::digest(&canonical)).map_err(MutationError::persistence)
}
impl Api {
    /// The caller holds the domain lock and has rechecked permissions before asking for an old result.
    pub(super) fn replay<T: DeserializeOwned>(&self, scope: &str, id: &str, operation: &str, input: &str) -> Result<Option<T>, MutationError> {
        let previous = match &self.data_dir {
            Some(dir) => store::receipt(dir, scope, id).map_err(MutationError::persistence)?,
            None => self.receipts.locked().get(&(scope.into(), id.into())).cloned(),
        };
        let Some(receipt) = previous else { return Ok(None) };
        if receipt.operation != operation || receipt.input != input {
            return Err(MutationError::new(MutationErrorCode::RequestConflict, "This request ID was already used for a different change."))
        }
        serde_json::from_value(receipt.result).map(Some).map_err(MutationError::persistence)
    }
}
