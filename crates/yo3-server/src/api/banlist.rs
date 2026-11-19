//! The banlist an account logs in with (#63): its own choice (`/banlist` in the chat) or the server's, and the staff's
//! override.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use super::{Actor, Api, MutationError, MutationErrorCode};
use crate::lock::Locked;
use crate::logins::Role;
use crate::store::AuditRow;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BanlistChoice {
    #[schema(maximum = 4294967295_u32)]
    pub id: u32,
    pub name: String,
}

/// The banlist an account's game logs in with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AccountBanlist {
    #[serde(flatten)]
    pub banlist: BanlistChoice,
    /// True when the account (or the staff) chose it; false when it is the server's `login-regulation`.
    pub chosen: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BanlistChoices {
    /// What an account without a choice logs in with.
    pub server_default: BanlistChoice,
    pub choices: Vec<BanlistChoice>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetBanlistRequest {
    #[schema(maximum = 4294967295_u32)]
    pub user_id: u32,
    /// A banlist's id from the choices; null or omitted goes back to the server's.
    #[schema(maximum = 4294967295_u32)]
    pub banlist: Option<u32>,
}

impl Api {
    /// The client's banlists (id, name): the running server's, else read from the game's files.
    pub(super) fn banlist_names_for_ops(&self) -> Vec<(u32, String)> { self.banlist_names() }

    fn banlist_names(&self) -> Vec<(u32, String)> {
        if let Some(world) = &self.world { return world.banlists.clone() }
        match yo3_data::GameData::load(&self.game_dir) {
            Ok(data) => data.regulations.iter().map(|r| (r.id, data.text(r.name_id).unwrap_or_default().to_string())).collect(),
            Err(_) => vec![],
        }
    }

    fn choice(names: &[(u32, String)], id: u32) -> BanlistChoice {
        BanlistChoice { id, name: names.iter().find(|(k, _)| *k == id).map_or_else(|| format!("Banlist {id}"), |(_, n)| n.clone()) }
    }

    /// The banlist account `user_id` logs in with now.
    pub(super) fn account_banlist(&self, own: Option<u32>) -> AccountBanlist {
        let names = self.world.as_ref().map(|w| w.banlists.clone()).unwrap_or_default();
        let known = own.filter(|id| names.is_empty() || names.iter().any(|(k, _)| k == id));
        match known {
            Some(id) => AccountBanlist { banlist: Self::choice(&names, id), chosen: true },
            None => AccountBanlist { banlist: Self::choice(&names, self.settings.now().login_regulation), chosen: false },
        }
    }

    pub fn banlist_choices(&self, actor: Actor<'_>) -> Result<BanlistChoices, MutationError> {
        actor.resolve(&self.accounts.locked(), Role::Moderator)?;
        let names = self.banlist_names();
        Ok(BanlistChoices { server_default: Self::choice(&names, self.settings.now().login_regulation),
            choices: names.iter().map(|(id, _)| Self::choice(&names, *id)).collect(), checked_at: Utc::now() })
    }

    /// The staff's override: the account logs in with this banlist from its next login (or the server's). Audited.
    pub fn set_banlist(&self, request: &SetBanlistRequest, actor: Actor<'_>, origin: &str) -> Result<AccountBanlist, MutationError> {
        let names = self.banlist_names();
        if let Some(id) = request.banlist && !names.iter().any(|(k, _)| *k == id) {
            return Err(MutationError::new(MutationErrorCode::InvalidRequest, "Choose one of the game's banlists."))
        }
        let mut d = self.accounts.locked();
        let identity = actor.resolve(&d, Role::Moderator)?;
        let before = d.account_of(request.user_id).ok_or_else(|| MutationError::new(MutationErrorCode::NotFound, "The account no longer exists."))?.banlist;
        if before != request.banlist {
            d.set_banlist_of(request.user_id, request.banlist).map_err(MutationError::persistence)?;
        }
        let label = d.player_names(request.user_id).first().cloned().unwrap_or_else(|| format!("#{}", request.user_id));
        drop(d);
        if before != request.banlist {
            self.audit_row(AuditRow { at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), actor: identity.by, origin: origin.into(),
                op: "set-banlist".into(), target: label, before: Some(json!(before)), after: Some(json!(request.banlist)), detail: Some(json!({ "user_id": request.user_id, "banlist": request.banlist })) });
        }
        Ok(self.account_banlist(request.banlist))
    }
}
