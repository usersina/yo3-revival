//! Live settings and maintenance: one validated snapshot, audit and retry receipt, then game notifications.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use crate::{lock::Locked, logins::Role, settings::{self, Kind}, store::{self, AuditRow, Receipt}};
use super::{Api, Op, mutation::{Actor, MutationError, MutationErrorCode, fingerprint, validate_id}};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettingsRequest {
    Set { name: String, value: String }, Reset { name: String },
    ScheduleMaintenance { #[schema(maximum = 4294967295_u32)] minutes: u32, message: Option<String> },
    CancelMaintenance,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SettingsResult {
    pub request_id: String, pub message: String,
    /// Committed deadline, not a live status. A retry returns this original deadline even after later changes.
    #[schema(value_type = Option<String>, format = DateTime)] pub maintenance_at: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SettingKind {
    Text { max_length: u32 }, Number { min: u32, max: u32 }, Choice { choices: Vec<String> },
    Version, Regulation { choices: Vec<RegulationChoice> }, News,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegulationChoice { pub id: u32, pub name: String }
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SettingRow {
    pub name: String, pub help: String, pub value: String, pub default_value: String, pub overridden: bool,
    pub applies: String, pub input: SettingKind,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SettingsList {
    pub settings: Vec<SettingRow>,
    #[schema(value_type = String, format = DateTime)] pub checked_at: String,
    #[schema(value_type = Option<String>, format = DateTime)] pub maintenance_at: Option<String>,
    pub maintenance_active: bool,
}
fn invalid(e: impl ToString) -> MutationError { MutationError::new(MutationErrorCode::InvalidRequest, e.to_string()) }
fn time(at: i64) -> String { chrono::DateTime::from_timestamp(at,0).expect("validated timestamp").to_rfc3339_opts(chrono::SecondsFormat::Secs,true) }
fn text(value: &Value) -> String { match value { Value::String(s) => s.clone(), other => other.to_string() } }
impl Api {
    pub fn live_settings(&self, actor: Actor<'_>) -> Result<SettingsList, MutationError> {
        let accounts = self.accounts.locked(); actor.resolve(&accounts, Role::Moderator)?;
        let (now, overrides) = self.settings.snapshot();
        let settings = settings::REGISTRY.iter().map(|s| SettingRow {
            name: s.name.into(), help: s.help.into(), value: text(now.value(s.name)), default_value: text(&(s.default)()),
            overridden: overrides.contains_key(s.name), applies: s.applies.describe().into(), input: match s.kind {
                Kind::Text { max } => SettingKind::Text { max_length: max as u32 }, Kind::Number { min,max } => SettingKind::Number { min,max },
                Kind::Choice(c) => SettingKind::Choice { choices: c.iter().map(|s| (*s).into()).collect() }, Kind::Version => SettingKind::Version,
                Kind::Regulation => SettingKind::Regulation { choices: self.settings.banlists().iter().map(|(id,name)| RegulationChoice { id:*id,name:name.clone() }).collect() },
                Kind::News => SettingKind::News,
            },
        }).collect();
        let at = crate::logins::now();
        Ok(SettingsList { settings, checked_at: time(at), maintenance_at: now.maintenance_at.map(time), maintenance_active: now.in_maintenance(at) })
    }
    pub(super) fn named_setting(&self, op: &Op, id: &str, actor: Actor<'_>, origin: &str) -> Result<Value, MutationError> {
        let request = match op {
            Op::Set { name,value } => SettingsRequest::Set { name:name.clone(),value:value.clone() },
            Op::Reset { name } => SettingsRequest::Reset { name:name.clone() },
            Op::Maintenance { minutes, message, off } => {
                if *off && (minutes.is_some() || message.is_some()) { return Err(invalid("Use --off alone, or --in MINUTES with an optional --message.")) }
                if *off { SettingsRequest::CancelMaintenance } else { SettingsRequest::ScheduleMaintenance {
                    minutes: minutes.ok_or_else(|| invalid("maintenance needs --in MINUTES (0: now), or --off"))?, message:message.clone(),
                } }
            }
            _ => unreachable!(),
        };
        self.change_settings(&request,id,actor,origin).map(|r| json!(r))
    }
    pub fn change_settings(&self, request: &SettingsRequest, id: &str, actor: Actor<'_>, origin: &str) -> Result<SettingsResult, MutationError> {
        validate_id(id)?;
        // Match the other mutations: Hub -> accounts -> settings -> store. Workers release snapshots before locking Hub.
        let mut hub = self.hub.as_ref().map(|h| h.locked());
        let accounts = self.accounts.locked(); let identity = actor.resolve(&accounts,Role::Admin)?;
        let hash = fingerprint(request)?;
        let (operation,target) = match request {
            SettingsRequest::Set { name,.. } => ("set",name.as_str()), SettingsRequest::Reset { name } => ("reset",name.as_str()),
            _ => ("maintenance",""),
        };
        if let Some(result) = self.replay(&identity.scope,id,operation,&hash)? { return Ok(result) }
        let edits = match request {
            SettingsRequest::Set { name,value } => vec![(name.clone(),Some(value.clone()))],
            SettingsRequest::Reset { name } => vec![(name.clone(),None)],
            SettingsRequest::CancelMaintenance => vec![("maintenance-at".into(),None)],
            SettingsRequest::ScheduleMaintenance { minutes,message } => {
                let at = crate::logins::now() + i64::from(*minutes)*60;
                let mut edits = vec![("maintenance-at".into(),Some(at.to_string()))];
                if let Some(message) = message { edits.push(("maintenance-message".into(),Some(message.clone()))); }
                edits
            }
        };
        let next = self.settings.prepare(&edits).map_err(invalid)?;
        let cancel = matches!(request,SettingsRequest::CancelMaintenance) && next.before.maintenance_at.is_some_and(|t| t > crate::logins::now());
        let message = match request {
            SettingsRequest::Set { name,.. } | SettingsRequest::Reset { name } => next.message(name),
            SettingsRequest::CancelMaintenance => "maintenance is off".into(),
            SettingsRequest::ScheduleMaintenance { .. } => {
                let at = next.settings.maintenance_at.expect("scheduled timestamp");
                let when = chrono::DateTime::from_timestamp(at,0).expect("validated timestamp").format("%Y-%m-%d %H:%M UTC");
                format!("maintenance starts at {when}: the game counts down until then; then only admins and moderators stay")
            }
        };
        let result = SettingsResult { request_id:id.into(),message,maintenance_at:next.settings.maintenance_at.map(time) };
        let values = |s: &settings::Settings| -> Value { edits.iter().map(|(name,_)| {
            let name = settings::find(name).expect("validated").name; (name.to_string(),s.value(name).clone())
        }).collect::<serde_json::Map<_,_>>().into() };
        let row = AuditRow { at:time(crate::logins::now()),actor:identity.by,origin:origin.into(),op:operation.into(),target:target.into(),
            before:Some(values(&next.before)),after:Some(values(&next.settings)),detail:Some(json!({ "request_id":id })) };
        let receipt = Receipt { operation:operation.into(),input:hash,result:json!(result) };
        match &self.data_dir {
            Some(dir) => store::together(dir,|| { next.persist()?; store::audit(dir,&row)?; store::put_receipt(dir,&identity.scope,id,&receipt) }).map_err(MutationError::persistence)?,
            None => { next.persist().map_err(MutationError::persistence)?; self.memory.locked().push(row); self.receipts.locked().insert((identity.scope,id.into()),receipt); }
        }
        let snapshot = next.settings.clone(); next.publish();
        // Post-commit effects run once. A missing response must never replay a countdown, cancellation or pruning.
        if edits.iter().any(|(n,_)| ["news","maintenance-at","maintenance-message"].iter().any(|s| n.trim().eq_ignore_ascii_case(s)))
            && let Some(h) = hub.as_mut() {
            h.push_everyone(&crate::front::information(&snapshot,self.status.engine_problem.is_some(),crate::logins::now()));
            if cancel { h.push_everyone(&crate::front::chat_note("Moderator",crate::front::MAINTENANCE_CANCELLED)); }
        }
        if edits.iter().any(|(n,_)| n.trim().eq_ignore_ascii_case("audit-keep-days")) && let Some(dir) = &self.data_dir
            && let Err(e) = store::prune_audit(dir,snapshot.audit_keep_days) { tracing::error!("audit log not pruned: {e}"); }
        Ok(result)
    }
}
