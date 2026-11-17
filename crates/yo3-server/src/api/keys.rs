//! Explicit admin-only key disclosure. Credentials and player names never enter durable receipts.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{accounts::Directory, lock::Locked, logins::{self, Role}, store::{self, AuditRow, Receipt}};
use super::{Api, Actor, Op, MutationError, MutationErrorCode, mutation::{fingerprint, validate_id}};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KeyAccountRequest {
    /// No names: use the host's starting players, made unique. Explicit names must be available.
    Create { #[schema(max_items = 4)] names: Vec<String> },
    /// Reveal the existing key, or assign one. The key allows passwordless game access.
    Reveal { #[schema(maximum = 4294967295_u32)] user_id: u32 },
}
#[derive(Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String, min_length = 20, max_length = 20)]
pub struct AccountKey(String);
impl AccountKey { pub fn expose(&self) -> &str { &self.0 } }
impl std::fmt::Debug for AccountKey { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("[REDACTED]") } }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KeyPlayer { #[schema(maximum = 4294967295_u32)] pub player_id: u32, pub name: String }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KeyAccountResult {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub request_id: String,
    pub key: AccountKey, pub players: Vec<KeyPlayer>,
}
#[derive(Serialize, Deserialize)]
struct Confirmation { user_id: u32, request_id: String, key_digest: String }
#[derive(Serialize)]
enum Input { Create { names: Vec<String> }, Reveal { user_id: u32 }, Named { who: String } }
impl Input { fn operation(&self) -> &'static str { if matches!(self, Self::Create { .. }) { "create-account" } else { "key" } } }
fn missing() -> MutationError { MutationError::new(MutationErrorCode::NotFound, "The account no longer exists. A retry cannot recreate it.") }
fn result(d: &Directory, receipt: Confirmation) -> Result<KeyAccountResult, MutationError> {
    let a = d.account_of(receipt.user_id).ok_or_else(missing)?;
    let key = d.key_for_user(receipt.user_id).ok_or_else(missing)?;
    if logins::digest(key) != receipt.key_digest { return Err(MutationError::new(MutationErrorCode::RequestConflict, "The account key has changed. Start a new key request.")) }
    Ok(KeyAccountResult { user_id: a.user_id, request_id: receipt.request_id, key: AccountKey(key.into()),
        players: a.players.iter().map(|p| KeyPlayer { player_id: p.player_id, name: p.name.clone() }).collect() })
}
impl Api {
    pub fn key_account(&self, request: &KeyAccountRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<KeyAccountResult, MutationError> {
        let input = match request { KeyAccountRequest::Create { names } => Input::Create { names: names.iter().map(|s| s.trim().to_owned()).collect() },
            KeyAccountRequest::Reveal { user_id } => Input::Reveal { user_id: *user_id } };
        self.key_change(input, id, actor, origin)
    }
    pub(super) fn named_key_account(&self, op: &Op, id: &str, actor: Actor<'_>, origin: &str) -> Result<serde_json::Value, MutationError> {
        let input = match op { Op::CreateAccount { names } => Input::Create { names: names.iter().map(|s| s.trim().to_owned()).collect() },
            Op::Key { who } => Input::Named { who: who.to_ascii_lowercase() }, _ => unreachable!() };
        let response = self.key_change(input, id, actor, origin)?;
        let names: Vec<_> = response.players.iter().map(|p| p.name.clone()).collect();
        let mut value = json!(response);
        // Preserve CLI/profile presentation, reconstructed now rather than stored in a receipt.
        value["created"] = json!(names.first()); value["players"] = json!(names);
        Ok(value)
    }
    fn key_change(&self, input: Input, id: &str, actor: Actor<'_>, origin: &str) -> Result<KeyAccountResult, MutationError> {
        validate_id(id)?;
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, Role::Admin)?;
        let operation = input.operation();
        let hash = fingerprint(&input)?;
        if let Some(receipt) = self.replay::<Confirmation>(&identity.scope, id, operation, &hash)? { return result(&d, receipt) }
        let (mut next, user) = match &input {
            Input::Create { names } => {
                if names.iter().any(|n| d.player_id_named(n).is_some()) { return Err(MutationError::new(MutationErrorCode::AlreadyExists, "That player name is already taken.")) }
                d.prepare_key_account(names).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?
            }
            Input::Reveal { .. } | Input::Named { .. } => {
                let user = match &input { Input::Reveal { user_id } => *user_id, Input::Named { who } => d.user_of(who).ok_or_else(missing)?, _ => unreachable!() };
                (d.prepare_key(user).map_err(|_| missing())?, user)
            }
        };
        let key = next.key_for_user(user).expect("prepared key");
        let confirmation = Confirmation { user_id: user, request_id: id.into(), key_digest: logins::digest(key) };
        let response = result(&next, confirmation)?;
        let label = next.login_by_id(user).map_or_else(|| format!("#{user}"), |l| l.user.clone());
        let row = AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by,
            origin: origin.into(), op: operation.into(), target: label.clone(), before: None,
            after: Some(json!({ "user_id": user, "has_key": true })), detail: Some(json!({ "who": label, "request_id": id })) };
        let receipt = Receipt { operation: operation.into(), input: hash,
            result: json!(Confirmation { user_id: user, request_id: id.into(), key_digest: logins::digest(response.key.expose()) }) };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) }).map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope, id.into()), receipt); }
        }
        *d = next;
        Ok(response)
    }
}
