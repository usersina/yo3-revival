//! Transactional moderation, with durable disconnect intent/outcome. Receipts never replay external effects.
use chrono::{Datelike,DateTime,Utc};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use utoipa::ToSchema;
use crate::{accounts::Directory,lock::Locked,logins::{Role,Standing,Mute},store::{self,AuditRow,Receipt,EffectState}};
use super::{Api,Op,mutation::{Actor,MutationError,MutationErrorCode,fingerprint,validate_id}};

#[derive(Clone,Debug,Serialize,Deserialize,ToSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum ModerationRequest {
    Kick { #[schema(maximum=4294967295_u32)] user_id:u32 },
    Ban { #[schema(maximum=4294967295_u32)] user_id:u32, #[schema(max_length=200)] reason:String, #[schema(minimum=1,maximum=4294967295_u32)] days:Option<u32> },
    Restrict { #[schema(maximum=4294967295_u32)] user_id:u32, #[schema(max_length=200)] reason:String, #[schema(minimum=1,maximum=4294967295_u32)] days:Option<u32> },
    Lift { #[schema(maximum=4294967295_u32)] user_id:u32 },
    Mute { #[schema(maximum=4294967295_u32)] user_id:u32, #[schema(max_length=200)] reason:String, #[schema(minimum=1,maximum=4294967295_u32)] minutes:Option<u32> },
    Unmute { #[schema(maximum=4294967295_u32)] user_id:u32 },
    SignOut { #[schema(maximum=4294967295_u32)] user_id:u32 },
}
impl ModerationRequest {
    fn user(&self)->u32 { match self { Self::Kick{user_id}|Self::Ban{user_id,..}|Self::Restrict{user_id,..}|Self::Lift{user_id}|Self::Mute{user_id,..}|Self::Unmute{user_id}|Self::SignOut{user_id}=>*user_id } }
    fn operation(&self)->&'static str { match self { Self::Kick{..}=>"kick",Self::Ban{..}=>"ban",Self::Restrict{..}=>"restrict",Self::Lift{..}=>"lift",Self::Mute{..}=>"mute",Self::Unmute{..}=>"unmute",Self::SignOut{..}=>"sign-out" } }
}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize,ToSchema)]
pub struct ModerationResult {
    #[schema(maximum=4294967295_u32)] pub user_id:u32, pub request_id:String, pub operation:String,
    pub until:Option<DateTime<Utc>>, pub message:String,
}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct ModerationResponse {
    pub result:ModerationResult,
    /// Null: no live disconnect was needed. Pending/interrupted is an uncertain outcome, never an instruction to repeat it.
    pub disconnect:Option<EffectState>,
}
#[derive(Serialize)]
enum Input { Numeric(ModerationRequest), Named(Op) }
impl Input {
    fn operation(&self)->String { match self { Self::Numeric(r)=>r.operation().into(),Self::Named(op)=>op.name() } }
    fn resolve(&self,d:&Directory)->Result<ModerationRequest,MutationError> {
        if let Self::Numeric(r)=self { return Ok(r.clone()) }
        let Self::Named(op)=self else { unreachable!() };
        let user=d.user_of(op.account().ok_or_else(missing)?).ok_or_else(missing)?;
        Ok(match op {
            Op::Kick{..}=>ModerationRequest::Kick{user_id:user}, Op::Ban{reason,days,..}=>ModerationRequest::Ban{user_id:user,reason:reason.clone(),days:*days},
            Op::Restrict{reason,days,..}=>ModerationRequest::Restrict{user_id:user,reason:reason.clone(),days:*days}, Op::Lift{..}=>ModerationRequest::Lift{user_id:user},
            Op::Mute{reason,minutes,..}=>ModerationRequest::Mute{user_id:user,reason:reason.clone(),minutes:*minutes}, Op::Unmute{..}=>ModerationRequest::Unmute{user_id:user},
            Op::SignOut{..}=>ModerationRequest::SignOut{user_id:user}, _=>unreachable!(),
        })
    }
}
fn invalid(e:impl ToString)->MutationError { MutationError::new(MutationErrorCode::InvalidRequest,e.to_string()) }
fn missing()->MutationError { MutationError::new(MutationErrorCode::NotFound,"The account no longer exists.") }
fn permission(actor:Actor<'_>,d:&Directory,user:u32)->Result<(),MutationError> {
    actor.resolve(d,Role::Moderator)?;
    if let Actor::Staff{user_id}=actor && d.login_by_id(user_id).is_some_and(|l|l.role==Role::Moderator)
        && user_id!=user && d.login_by_id(user).is_some_and(|l|l.staff()) {
        return Err(MutationError::new(MutationErrorCode::Forbidden,"Moderators cannot act on another staff account."))
    }
    Ok(())
}
fn deadline(amount:Option<u32>,seconds:i64)->Result<Option<DateTime<Utc>>,MutationError> {
    amount.map(|n| {
        if n==0 { return Err(invalid("Choose a positive duration, or leave it empty for no expiry.")) }
        DateTime::from_timestamp(crate::logins::now()+i64::from(n)*seconds,0).filter(|t|t.year()<=9999)
            .ok_or_else(||invalid("This duration is too long; choose a date before year 10000."))
    }).transpose()
}
impl Api {
    pub fn moderate(&self,request:&ModerationRequest,id:&str,actor:Actor<'_>,origin:&str)->Result<ModerationResponse,MutationError> {
        self.moderation(Input::Numeric(request.clone()),id,actor,origin)
    }
    pub(super) fn named_moderation(&self,op:&Op,id:&str,actor:Actor<'_>,origin:&str)->Result<Value,MutationError> {
        let response=self.moderation(Input::Named(op.clone()),id,actor,origin)?;
        let mut value=json!(response.result); value["disconnect"]=json!(response.disconnect);
        let label=self.accounts.locked().login_by_id(response.result.user_id).map(|l|l.user.clone()).unwrap_or_else(||op.account().unwrap_or("account").into());
        let until=response.result.until.map(|t|format!("until {}",t.format("%Y-%m-%d %H:%M UTC")));
        let message=match op {
            Op::Kick{..}=>if response.disconnect==Some(EffectState::Completed) {format!("{label} was disconnected")} else {format!("{label} disconnect request saved")},
            Op::Ban{..}=>format!("{label} is banned (devices signed out, game disconnect requested) {}",until.unwrap_or_else(||"until lifted".into())),
            Op::Restrict{..}=>format!("{label} is restricted (refused at the next sign-in) {}",until.unwrap_or_else(||"until lifted".into())),
            Op::Lift{..}=>format!("{label} may sign in again"), Op::Mute{..}=>format!("{label} is muted {}",until.unwrap_or_else(||"until unmuted".into())),
            Op::Unmute{..}=>format!("{label} may chat again"), Op::SignOut{..}=>format!("{label} is signed out everywhere"), _=>unreachable!(),
        };
        let suffix=match response.disconnect { Some(EffectState::Pending|EffectState::Interrupted)=>"; disconnect outcome unknown: inspect current sessions before requesting another disconnect", _=>"" };
        value["message"]=json!(format!("{message}{suffix}")); Ok(value)
    }
    fn disconnect_state(&self,scope:&str,id:&str)->Result<Option<EffectState>,MutationError> {
        match &self.data_dir {
            Some(dir)=>store::effect(dir,scope,id).map_err(MutationError::persistence),
            None=>Ok(self.effects.locked().get(&(scope.into(),id.into())).copied()),
        }
    }
    fn moderation(&self,input:Input,id:&str,actor:Actor<'_>,origin:&str)->Result<ModerationResponse,MutationError> {
        validate_id(id)?;
        // Account -> session lock matches sign-out/recovery. No Hub lock is held while waking disconnected clients.
        let mut d=self.accounts.locked(); let identity=actor.resolve(&d,Role::Moderator)?;
        let operation=input.operation(); let hash=fingerprint(&input)?;
        if let Some(result)=self.replay::<ModerationResult>(&identity.scope,id,&operation,&hash)? {
            permission(actor,&d,result.user_id)?;
            return Ok(ModerationResponse { result,disconnect:self.disconnect_state(&identity.scope,id)? })
        }
        let request=input.resolve(&d)?; let user=request.user(); permission(actor,&d,user)?;
        if d.account_of(user).is_none() { return Err(missing()) }
        let reason=match &request { ModerationRequest::Ban{reason,..}|ModerationRequest::Restrict{reason,..}|ModerationRequest::Mute{reason,..}=>reason.as_str(), _=>"" };
        if reason.chars().count()>200 || reason.chars().any(char::is_control) { return Err(invalid("The reason is at most 200 characters, on one line.")) }
        let until=match &request { ModerationRequest::Ban{days,..}|ModerationRequest::Restrict{days,..}=>deadline(*days,86400)?,ModerationRequest::Mute{minutes,..}=>deadline(*minutes,60)?,_=>None };
        let disconnect=matches!(request,ModerationRequest::Kick{..}|ModerationRequest::Ban{..}|ModerationRequest::SignOut{..}) && self.sessions.is_some();
        if matches!(request,ModerationRequest::Kick{..}) && !disconnect { return Err(invalid("Disconnect is available only on a running server.")) }
        let audit_value=|l:&crate::logins::Login|->Option<Value> {match &request {
            ModerationRequest::Ban{..}|ModerationRequest::Restrict{..}|ModerationRequest::Lift{..}=>Some(json!(l.standing)),
            ModerationRequest::Mute{..}|ModerationRequest::Unmute{..}=>l.muted.as_ref().map(|m|json!(m)),
            ModerationRequest::SignOut{..}=>Some(json!(l.devices.len())),_=>None,
        }};
        let before=d.login_by_id(user).and_then(audit_value);
        let mut next=if matches!(request,ModerationRequest::Kick{..}) { None } else { Some(d.prepare_login(user,|l| {
            match &request {
                ModerationRequest::Ban{..}|ModerationRequest::Restrict{..}=> {
                    l.staff_epoch=l.staff_epoch.checked_add(1).ok_or("Session revision limit reached")?;
                    l.standing=if matches!(request,ModerationRequest::Ban{..}) { l.devices.clear(); Standing::Banned{reason:reason.into(),until:until.map(|t|t.timestamp())} }
                        else { Standing::Restricted{reason:reason.into(),until:until.map(|t|t.timestamp())} };
                }
                ModerationRequest::Lift{..}=>l.standing=Standing::Active,
                ModerationRequest::Mute{..}=>l.muted=Some(Mute{reason:reason.into(),until:until.map(|t|t.timestamp())}),
                ModerationRequest::Unmute{..}=>l.muted=None,
                ModerationRequest::SignOut{..}=> {l.staff_epoch=l.staff_epoch.checked_add(1).ok_or("Session revision limit reached")?;l.devices.clear();}
                _=>unreachable!(),
            }
            Ok(())
        }).map_err(invalid)?) };
        let after=next.as_ref().and_then(|n|n.login_by_id(user)).and_then(audit_value);
        let result=ModerationResult {user_id:user,request_id:id.into(),operation:operation.clone(),until,message:format!("Saved {operation} for account #{user}.")};
        let label=d.login_by_id(user).map_or_else(||format!("#{user}"),|l|l.user.clone());
        let detail=json!({"who":label,"user_id":user,"request_id":id,"receipt_actor":identity.scope,"disconnect":if disconnect {Some(EffectState::Pending)} else {None}});
        let row=AuditRow {at:Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),actor:identity.by,origin:origin.into(),op:operation.clone(),target:match &input {Input::Named(op)=>op.target(),_=>format!("#{user}")},before,after,detail:Some(detail)};
        let receipt=Receipt{operation,input:hash,result:json!(result)};
        match &self.data_dir {
            Some(dir)=>store::together(dir,|| {if let Some(n)=next.as_mut(){n.persist()?;} store::audit(dir,&row)?;store::put_receipt(dir,&identity.scope,id,&receipt)?;if disconnect {store::put_effect(dir,&identity.scope,id,user)?;} Ok(())}).map_err(MutationError::persistence)?,
            None=> { self.memory.locked().push(row);self.receipts.locked().insert((identity.scope.clone(),id.into()),receipt);if disconnect {self.effects.locked().insert((identity.scope.clone(),id.into()),EffectState::Pending);} }
        }
        if let Some(next)=next { *d=next; }
        if disconnect {
            self.sessions.as_ref().expect("running sessions").locked().logout_user(user);
            // Keep the account lock through the effect, so a later lift/reconnect cannot be caught by an old ban.
            match &self.data_dir {
                Some(dir)=>if let Err(e)=store::complete_effect(dir,&identity.scope,id) {tracing::error!("Disconnect outcome not recorded: {e}; never replay it automatically");},
                None=> {self.effects.locked().insert((identity.scope.clone(),id.into()),EffectState::Completed);
                    for row in self.memory.locked().iter_mut() {if row.detail.as_ref().is_some_and(|v|v["receipt_actor"]==identity.scope && v["request_id"]==id) {row.detail.as_mut().unwrap()["disconnect"]=json!(EffectState::Completed);}}
                }
            }
        }
        Ok(ModerationResponse { result,disconnect:self.disconnect_state(&identity.scope,id)? })
    }
}
