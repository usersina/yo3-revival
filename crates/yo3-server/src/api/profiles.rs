//! Safe account projections and profile edits. Persist/audit/receipt before publishing, with no live game push.
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{accounts::Directory, lock::Locked, logins::{Role, Standing}, store::{self, AuditRow, Receipt}};
use super::{Api, Actor, Op, MutationError, MutationErrorCode, mutation::{fingerprint, validate_id}};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AccountEditRequest {
    SetRole { #[schema(maximum = 4294967295_u32)] user_id: u32, role: Role },
    AddPlayer { #[schema(maximum = 4294967295_u32)] user_id: u32, #[schema(min_length = 1, max_length = 16)] name: String },
    RenamePlayer { #[schema(maximum = 4294967295_u32)] user_id: u32, #[schema(maximum = 4294967295_u32)] player_id: u32,
        #[schema(min_length = 1, max_length = 16)] name: String },
    SetCountry { #[schema(maximum = 4294967295_u32)] user_id: u32,
        /// Null or omitted: derive from the game's language. Other codes must occur in the client's table.
        #[schema(min_length = 2, max_length = 2, pattern = "^[A-Za-z]{2}$")] country: Option<String> },
}
impl AccountEditRequest {
    fn user(&self) -> u32 { match self { Self::AddPlayer { user_id, .. } | Self::RenamePlayer { user_id, .. } | Self::SetCountry { user_id, .. } | Self::SetRole { user_id, .. } => *user_id } }
    fn operation(&self) -> &'static str { match self { Self::AddPlayer { .. } => "add-player", Self::RenamePlayer { .. } => "rename", Self::SetCountry { .. } => "set-country", Self::SetRole { .. } => "set-role" } }
    fn normalize(&mut self) {
        match self {
            Self::AddPlayer { name, .. } | Self::RenamePlayer { name, .. } => *name = name.trim().into(),
            Self::SetCountry { country, .. } => *country = country.as_ref().map(|c| c.to_ascii_uppercase()),
            Self::SetRole { .. } => {}
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AccountEditResult {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32,
    #[schema(maximum = 4294967295_u32)] pub player_id: Option<u32>, pub request_id: String, pub message: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountDetail {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub login: Option<String>, pub has_key: bool, pub password_ready: Option<bool>, pub role: Role,
    #[schema(maximum = 4294967295_u32)] pub bp: u32, pub country: Option<String>, pub standing: Option<AccountStanding>, pub muted: Option<AccountMute>,
    #[schema(maximum = 4294967295_u32)] pub device_count: u32, pub reset_pending: bool, pub players: Vec<PlayerDetail>,
    /// The banlist the account's game logs in with (its own choice with `/banlist`, a staff override, or the server's).
    pub banlist: super::banlist::AccountBanlist, pub checked_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AccountStanding { Active, Restricted { reason: String, until: Option<DateTime<Utc>> }, Banned { reason: String, until: Option<DateTime<Utc>> } }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountMute { pub reason: String, pub until: Option<DateTime<Utc>> }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlayerDetail {
    #[schema(maximum = 4294967295_u32)] pub player_id: u32, pub name: String,
    #[schema(maximum = 4294967295_u32)] pub slot: u32,
    #[schema(maximum = 4294967295_u32)] pub level: u32,
    #[schema(maximum = 4294967295_u32)] pub title_id: u32,
    /// Additional card copies, excluding the starter deck. u16 card rows * u32 counts fit below 2^48, exactly in JS.
    #[schema(maximum = 9007199254740991_u64)] pub additional_cards: u64,
    #[schema(maximum = 4294967295_u32)] pub card_kinds: u32,
    #[schema(maximum = 4294967295_u32)] pub purchased_decks: u32,
    #[schema(maximum = 4294967295_u32)] pub uploaded_decks: u32,
    #[schema(maximum = 4294967295_u32)] pub owned_items: u32,
    /// Live presence is unknown without a running hub. Never infer it from saved accounts.
    pub online: Option<bool>, pub record: Option<DuelRecord>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DuelRecord {
    #[schema(maximum = 4294967295_u32)] pub wins: u32, #[schema(maximum = 4294967295_u32)] pub losses: u32, #[schema(maximum = 4294967295_u32)] pub draws: u32,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CountryList { pub codes: Vec<String> }

fn missing() -> MutationError { MutationError::new(MutationErrorCode::NotFound, "The account or player no longer exists.") }
fn timestamp(value: Option<i64>) -> Result<Option<DateTime<Utc>>, MutationError> {
    value.map(|t| DateTime::from_timestamp(t, 0).filter(|d| (0..=9999).contains(&d.year()))
        .ok_or_else(|| MutationError::new(MutationErrorCode::PersistenceFailed, "An account has an unsupported date. Ask the host to review its data."))).transpose()
}
fn permission(actor: Actor<'_>, d: &Directory, user: u32, operation: &str) -> Result<(), MutationError> {
    let required = if operation == "rename" { Role::Moderator } else { Role::Admin };
    actor.resolve(d, required)?;
    if let Actor::Staff { user_id } = actor && required == Role::Moderator && d.login_by_id(user_id).is_some_and(|l| l.role == Role::Moderator)
        && user_id != user && d.login_by_id(user).is_some_and(|l| l.staff()) {
        return Err(MutationError::new(MutationErrorCode::Forbidden, "Moderators cannot change another staff account."))
    }
    Ok(())
}

#[derive(Serialize)]
enum Input { Numeric(AccountEditRequest), Named(Op) }
impl Input {
    fn operation(&self) -> String { match self { Self::Numeric(r) => r.operation().into(), Self::Named(op) => op.name() } }
    fn resolve(&self, d: &Directory) -> Result<AccountEditRequest, MutationError> {
        Ok(match self {
            Self::Numeric(r) => r.clone(),
            Self::Named(Op::AddPlayer { who, name }) => AccountEditRequest::AddPlayer { user_id: d.user_of(who).ok_or_else(missing)?, name: name.clone() },
            Self::Named(Op::Rename { old, new }) => {
                let player_id = d.player_id_named(old).ok_or_else(missing)?;
                let user_id = d.entries().find(|(_, _, a)| a.players.iter().any(|p| p.player_id == player_id)).map(|(_, _, a)| a.user_id).ok_or_else(missing)?;
                AccountEditRequest::RenamePlayer { user_id, player_id, name: new.clone() }
            }
            Self::Named(Op::SetCountry { who, code }) => AccountEditRequest::SetCountry { user_id: d.user_of(who).ok_or_else(missing)?, country: (!code.is_empty()).then(|| code.clone()) },
            Self::Named(Op::SetRole { user, role }) => AccountEditRequest::SetRole { user_id: d.login_of(user).map(|(id, _)| id).ok_or_else(missing)?,
                role: role.parse().map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))? },
            _ => return Err(MutationError::new(MutationErrorCode::InvalidRequest, "Unsupported profile edit.")),
        })
    }
}
pub(crate) fn badges(login:Option<&crate::logins::Login>,now:i64)->Result<(Option<AccountStanding>,Option<AccountMute>),MutationError>{
        let standing = login.map(|l| -> Result<AccountStanding, MutationError> { Ok(if l.refusal(now).is_none() { AccountStanding::Active } else {
            match &l.standing {
                Standing::Active => AccountStanding::Active,
                Standing::Restricted { reason, until } => AccountStanding::Restricted { reason: reason.clone(), until: timestamp(*until)? },
                Standing::Banned { reason, until } => AccountStanding::Banned { reason: reason.clone(), until: timestamp(*until)? },
            }
        }) }).transpose()?;
        let muted = login.and_then(|l| l.mute(now)).map(|m| -> Result<AccountMute, MutationError> { Ok(AccountMute { reason: m.reason.clone(), until: timestamp(m.until)? }) }).transpose()?;
    Ok((standing,muted))
}
impl Api {
    pub fn account_detail(&self, user: u32, actor: Actor<'_>) -> Result<AccountDetail, MutationError> {
        // Live hub -> stored accounts, matching the mutation/duel reward lock order.
        let hub = self.hub.as_ref().map(|h| h.locked());
        let d = self.accounts.locked();
        actor.resolve(&d, Role::Moderator)?;
        let a = d.account_of(user).ok_or_else(missing)?;
        let login = d.login_by_id(user);
        let now = crate::logins::now();
        let (standing,muted)=badges(login,now)?;
        Ok(AccountDetail { user_id: user, login: login.map(|l| l.user.clone()), has_key: d.entries().any(|(_, k, a)| a.user_id == user && k.is_some()),
            password_ready: login.map(|l| !l.password.is_empty()), role: login.map_or(Role::Player, |l| l.role), bp: a.dpoint, country: (!a.country.is_empty()).then_some(a.country), standing, muted,
            device_count: login.map_or(0, |l| l.devices.len() as u32), reset_pending: login.and_then(|l| l.reset.as_ref()).is_some_and(|(_, t)| *t > now),
            banlist: self.account_banlist(a.banlist),
            players: a.players.iter().map(|p| PlayerDetail { player_id: p.player_id, name: p.name.clone(), slot: p.index, level: p.level, title_id: p.title,
                additional_cards: p.cards.values().map(|&n| u64::from(n)).sum(), card_kinds: p.cards.len() as u32, purchased_decks: p.decks.len() as u32,
                uploaded_decks: p.uploaded.len() as u32, owned_items: p.items.len() as u32, online: hub.as_ref().map(|h| h.is_online(p.player_id)),
                record: hub.as_ref().map(|h| { let [wins, losses, draws] = h.record_totals(p.player_id); DuelRecord { wins, losses, draws } }),
            }).collect(), checked_at: Utc::now() })
    }

    pub fn account_countries(&self, actor: Actor<'_>) -> Result<CountryList, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let codes = yo3_data::countries(&self.game_dir).map_err(|error| { tracing::warn!("Country choices unavailable: {error}");
            MutationError::new(MutationErrorCode::PersistenceFailed, "Country choices are unavailable. Ask the host to check the game data files; automatic country is still available.") })?;
        Ok(CountryList { codes })
    }

    pub fn edit_account(&self, request: &AccountEditRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<AccountEditResult, MutationError> {
        let mut request = request.clone(); request.normalize();
        self.edit_profile(Input::Numeric(request), id, actor, origin)
    }

    pub(super) fn edit_named_account(&self, op: &Op, id: &str, actor: Actor<'_>, origin: &str) -> Result<serde_json::Value, MutationError> {
        let mut op = op.clone();
        match &mut op {
            Op::AddPlayer { name, .. } => *name = name.trim().into(),
            Op::Rename { new, .. } => *new = new.trim().into(),
            Op::SetCountry { code, .. } => *code = if code.eq_ignore_ascii_case("auto") { String::new() } else { code.to_ascii_uppercase() },
            Op::SetRole { role, .. } => *role = role.to_ascii_lowercase(),
            _ => {}
        }
        let result = self.edit_profile(Input::Named(op.clone()), id, actor, origin)?;
        let mut value = json!(result);
        // Human labels are rendered from the caller's input, never retained in permanent receipts.
        value["message"] = json!(match &op {
            Op::Rename { old, new } => format!("{old} is now {new} (online, at the player's next login)"),
            Op::AddPlayer { who, name } => { value["created"] = json!(name); format!("{who}'s account has a player named {name}") },
            Op::SetCountry { who, code } => format!("{who}'s country is now {}", if code.is_empty() { "auto (the game's language)" } else { code }),
            Op::SetRole { user, role } => format!("{user} is now {role}"),
            _ => unreachable!(),
        });
        Ok(value)
    }

    fn edit_profile(&self, input: Input, id: &str, actor: Actor<'_>, origin: &str) -> Result<AccountEditResult, MutationError> {
        validate_id(id)?;
        let operation = input.operation();
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, if operation == "rename" { Role::Moderator } else { Role::Admin })?;
        let fingerprint = fingerprint(&input)?;
        if let Some(result) = self.replay::<AccountEditResult>(&identity.scope, id, &operation, &fingerprint)? {
            permission(actor, &d, result.user_id, &operation)?;
            return Ok(result)
        }
        let request = input.resolve(&d)?;
        let user = request.user();
        permission(actor, &d, user, &operation)?;
        let account = d.account_of(user).ok_or_else(missing)?;
        let target = match &input { Input::Named(op) => op.target(), Input::Numeric(_) => format!("#{user}") };
        let (mut next, player_id, before, after) = match request {
            AccountEditRequest::SetRole { role, .. } => {
                let login = d.login_by_id(user).ok_or_else(|| MutationError::new(MutationErrorCode::InvalidRequest, "Only password accounts can have a staff role."))?;
                (d.prepare_role(user, role).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?, None, Some(json!(login.role)), Some(json!(role)))
            }
            AccountEditRequest::AddPlayer { name, .. } => {
                if d.player_id_named(&name).is_some() { return Err(MutationError::new(MutationErrorCode::AlreadyExists, "That player name is already taken.")) }
                if account.players.len() >= crate::accounts::MAX_PLAYERS { return Err(MutationError::new(MutationErrorCode::RequestConflict, "This account already has four players.")) }
                let (next, player_id) = d.prepare_add_player(user, &name).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?;
                (next, Some(player_id), None, Some(json!(name)))
            }
            AccountEditRequest::RenamePlayer { player_id, name, .. } => {
                let old = account.players.iter().find(|p| p.player_id == player_id).ok_or_else(missing)?;
                if d.player_id_named(&name).is_some_and(|p| p != player_id) { return Err(MutationError::new(MutationErrorCode::AlreadyExists, "That player name is already taken.")) }
                (d.prepare_rename(player_id, &name).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?, Some(player_id), Some(json!(old.name)), Some(json!(name)))
            }
            AccountEditRequest::SetCountry { country, .. } => {
                if let Some(code) = &country {
                    if code.len() != 2 || !code.bytes().all(|b| b.is_ascii_uppercase()) { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "Choose a two-letter country code, or automatic.")) }
                    let known = yo3_data::countries(&self.game_dir).map_err(|_| MutationError::new(MutationErrorCode::PersistenceFailed, "Country choices are unavailable. Ask the host to check the game data files."))?;
                    if !known.contains(code) { return Err(MutationError::new(MutationErrorCode::InvalidRequest, "This country is not supported by the game.")) }
                }
                (d.prepare_country(user, country.as_deref().unwrap_or_default()).map_err(|e| MutationError::new(MutationErrorCode::InvalidRequest, e))?, None, Some(json!(account.country)), Some(json!(country.unwrap_or_default())))
            }
        };
        let message = if operation == "set-role" { format!("Saved role for account #{user}. If it changed, its staff sessions must sign in again.") }
            else { format!("Saved {operation} for account #{user}. In-game changes appear after the next sign-in.") };
        let result = AccountEditResult { user_id: user, player_id, request_id: id.into(), message };
        // Keep the account's stable login label in prunable audit metadata, so deletion also removes old aliases.
        let label = d.login_by_id(user).map_or_else(|| format!("#{user}"), |l| l.user.clone());
        let row = AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by, origin: origin.into(),
            op: operation.clone(), target, before, after, detail: Some(json!({ "who": label, "user_id": user, "player_id": player_id, "request_id": id })) };
        let receipt = Receipt { operation, input: fingerprint, result: json!(result) };
        match &self.data_dir {
            Some(dir) => store::together(dir, || { next.persist()?; store::audit(dir, &row)?; store::put_receipt(dir, &identity.scope, id, &receipt) }).map_err(MutationError::persistence)?,
            None => { self.memory.locked().push(row); self.receipts.locked().insert((identity.scope, id.into()), receipt); }
        }
        *d = next;
        Ok(result)
    }
}

#[cfg(test)]
mod badge_tests {
    use super::*;
    #[test]
    fn expired_moderation_is_not_shown_as_active_and_bad_dates_are_reported() {
        let now=crate::logins::now();let mut login=crate::logins::Login::new("Player","HASH".into(),"RECOVERY".into());
        login.standing=Standing::Restricted{reason:"expired".into(),until:Some(now-1)};
        login.muted=Some(crate::logins::Mute{reason:"expired".into(),until:Some(now-1)});
        let (standing,muted)=badges(Some(&login),now).unwrap();assert!(matches!(standing,Some(AccountStanding::Active)));assert!(muted.is_none());
        login.standing=Standing::Banned{reason:"invalid date".into(),until:Some(i64::MAX)};
        assert_eq!(badges(Some(&login),now).unwrap_err().code,MutationErrorCode::PersistenceFailed);
    }
}
