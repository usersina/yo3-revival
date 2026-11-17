//! Bounded, staff-only history and volatile logs. No simulated data or unbounded database scans in the browser.
use chrono::{DateTime,Utc};
use serde::{Deserialize,Serialize};
use utoipa::ToSchema;
use crate::{lock::Locked,logins::Role,store};
use super::{Api,Actor,MutationError,MutationErrorCode};
fn audit_limit()->u32{50}
fn log_limit()->u32{200}
#[derive(Debug,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditQuery {#[serde(default="audit_limit")]pub limit:u32,pub before:Option<String>,#[serde(default)]pub q:String}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct AuditEntry {
    /// Decimal SQLite row ID. Never convert this to a JavaScript number.
    pub id:String,pub at:DateTime<Utc>,pub actor:String,pub origin:String,pub operation:String,pub target:String,
    /// Pretty-printed JSON for display as text, not executable HTML or a business-operation response.
    pub before:Option<String>,pub after:Option<String>,pub detail:Option<String>,
}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct AuditPage {pub entries:Vec<AuditEntry>,pub next_before:Option<String>,pub checked_at:DateTime<Utc>}
#[derive(Debug,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogsQuery {#[serde(default="log_limit")]pub limit:u32,#[serde(default)]pub problems:bool}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
#[serde(rename_all="lowercase")]
pub enum LogLevel {Error,Warn,Info,Debug}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct LogEntry {pub at:DateTime<Utc>,pub level:LogLevel,pub text:String}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct LogPage {pub lines:Vec<LogEntry>,#[schema(maximum=4294967295_u32)]pub retention_lines:u32,pub checked_at:DateTime<Utc>}
fn invalid(message:&str)->MutationError{MutationError::new(MutationErrorCode::InvalidRequest,message)}
fn unreadable(error:impl std::fmt::Display)->MutationError{
    tracing::error!("Administrative history could not be read: {error}");
    MutationError::new(MutationErrorCode::PersistenceFailed,"The history could not be read. Check the server logs and try again.")
}
fn formatted(value:Option<String>)->Result<Option<String>,MutationError>{value.map(|v|serde_json::from_str::<serde_json::Value>(&v).and_then(|v|serde_json::to_string_pretty(&v)).map_err(unreadable)).transpose()}
impl Api {
    pub fn read_audit(&self,query:&AuditQuery,actor:Actor<'_>)->Result<AuditPage,MutationError>{
        actor.resolve(&self.accounts.locked(),Role::Moderator)?;
        if !(1..=200).contains(&query.limit) || query.q.chars().count()>80 || query.q.chars().any(char::is_control){return Err(invalid("Choose 1–200 audit entries and a search of at most 80 characters."))}
        let before=query.before.as_ref().map(|id|{
            if id.is_empty() || id.len()>19 || id.starts_with('0') || !id.bytes().all(|b|b.is_ascii_digit()){return Err(invalid("Invalid audit cursor."))}
            id.parse::<i64>().ok().filter(|id|*id>0).ok_or_else(||invalid("Invalid audit cursor."))
        }).transpose()?;
        let dir=self.data_dir.as_ref().ok_or_else(||MutationError::new(MutationErrorCode::NotFound,"This server has no persistent audit log."))?;
        let mut rows=store::audit_page(dir,query.limit+1,before,query.q.trim()).map_err(unreadable)?;
        let more=rows.len()>query.limit as usize;rows.truncate(query.limit as usize);
        let next_before=if more{rows.last().map(|r|r.id.to_string())}else{None};
        let entries=rows.into_iter().map(|r|Ok(AuditEntry{id:r.id.to_string(),at:r.at.parse::<DateTime<Utc>>().map_err(unreadable)?,actor:r.actor,origin:r.origin,operation:r.op,target:r.target,
            before:formatted(r.before)?,after:formatted(r.after)?,detail:formatted(r.detail)?})).collect::<Result<_,MutationError>>()?;
        Ok(AuditPage{entries,next_before,checked_at:Utc::now()})
    }
    pub fn read_logs(&self,query:&LogsQuery,actor:Actor<'_>)->Result<LogPage,MutationError>{
        actor.resolve(&self.accounts.locked(),Role::Moderator)?;
        if !(1..=500).contains(&query.limit){return Err(invalid("Choose 1–500 log lines."))}
        let lines=crate::recent::lines(query.limit as usize,query.problems).into_iter().map(|line|LogEntry{at:DateTime::from_timestamp(line.at,0).unwrap_or(DateTime::UNIX_EPOCH),
            level:match line.level{"ERROR"=>LogLevel::Error,"WARN"=>LogLevel::Warn,"DEBUG"=>LogLevel::Debug,_=>LogLevel::Info},text:line.text}).collect();
        Ok(LogPage{lines,retention_lines:crate::recent::KEEP as u32,checked_at:Utc::now()})
    }
}
