//! Live process status and durable, run-bound graceful-stop requests.
use chrono::{DateTime,Utc};
use serde::{Deserialize,Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{lock::Locked,logins::Role,store::{self,AuditRow,Receipt,EffectState}};
use super::{Api,Actor,MutationError,MutationErrorCode,mutation::{validate_id,fingerprint}};
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct ServerOverview {
    pub run_id:String, pub version:String, pub can_stop:bool, pub checked_at:DateTime<Utc>,
    #[schema(maximum=4294967295_u32)] pub accounts:u32,
    #[schema(maximum=4294967295_u32)] pub players:u32,
    #[schema(maximum=4294967295_u32)] pub online:u32,
    #[schema(maximum=4294967295_u32)] pub duels:u32,
    pub uptime_seconds:Option<u32>, pub persistence_healthy:bool, pub engine_problem:Option<String>,
}
#[derive(Clone,Debug,Serialize,Deserialize,ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StopRequest { #[schema(min_length=64,max_length=64,pattern="^[a-f0-9]{64}$")] pub run_id:String }
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize,ToSchema)]
pub struct StopResult { pub run_id:String, pub request_id:String, pub message:String }
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct StopResponse {pub result:StopResult,pub stop:EffectState}
impl Api {
    pub fn run_id(&self)->&str {&self.run_id}
    /// Presence, duels and uptime describe this process, not persisted rows.
    pub fn overview(&self)->ServerOverview {
        let hub=self.hub.as_ref().map(|h|h.locked());let d=self.accounts.locked();
        let (online,duels)=hub.as_ref().map(|h|(h.online(),h.tables.len())).unwrap_or_default();
        ServerOverview {run_id:self.run_id.clone(),version:yo3_tunnel::BUILD_VERSION.into(),can_stop:self.shutdown.is_some(),checked_at:Utc::now(),
            accounts:d.entries().count().try_into().unwrap_or(u32::MAX),players:d.entries().map(|(_,_,a)|a.players.len()).sum::<usize>().try_into().unwrap_or(u32::MAX),
            online:online.try_into().unwrap_or(u32::MAX),duels:duels.try_into().unwrap_or(u32::MAX),
            uptime_seconds:self.status.started.map(|t|t.elapsed().as_secs().try_into().unwrap_or(u32::MAX)),persistence_healthy:store::healthy(),engine_problem:self.status.engine_problem.clone()}
    }
    pub fn request_stop(&self,request:&StopRequest,id:&str,actor:Actor<'_>,origin:&str)->Result<StopResponse,MutationError> {
        validate_id(id)?;let d=self.accounts.locked();let identity=actor.resolve(&d,Role::Admin)?;let hash=fingerprint(request)?;
        if let Some(result)=self.replay::<StopResult>(&identity.scope,id,"shutdown",&hash)? {
            let stop=self.disconnect_state(&identity.scope,id)?.ok_or_else(||MutationError::persistence("stop intent missing"))?;
            return Ok(StopResponse{result,stop})
        }
        if request.run_id!=self.run_id {return Err(MutationError::new(MutationErrorCode::RequestConflict,"This server restarted. Refresh its status before requesting a new stop."))}
        let signal=self.shutdown.as_ref().ok_or_else(||MutationError::new(MutationErrorCode::InvalidRequest,"This API does not control a running server."))?;
        let result=StopResult{run_id:self.run_id.clone(),request_id:id.into(),message:"Graceful stop requested. Connections will close; final saves and backup continue during shutdown.".into()};
        let row=AuditRow{at:Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),actor:identity.by,origin:origin.into(),op:"shutdown".into(),target:"server".into(),before:None,after:None,
            detail:Some(json!({"run_id":self.run_id,"request_id":id,"receipt_actor":identity.scope,"stop":EffectState::Pending}))};
        let receipt=Receipt{operation:"shutdown".into(),input:hash,result:json!(result)};
        match &self.data_dir {
            Some(dir)=>store::together(dir,||{store::audit(dir,&row)?;store::put_receipt(dir,&identity.scope,id,&receipt)?;store::put_stop_effect(dir,&identity.scope,id)?;Ok(())}).map_err(MutationError::persistence)?,
            None=>{self.memory.locked().push(row);self.receipts.locked().insert((identity.scope.clone(),id.into()),receipt);self.effects.locked().insert((identity.scope.clone(),id.into()),EffectState::Pending);}
        }
        signal.send_replace(true);
        match &self.data_dir {
            Some(dir)=>if let Err(e)=store::complete_effect(dir,&identity.scope,id){tracing::error!("Stop outcome not recorded: {e}; never repeat it automatically");},
            None=>{self.effects.locked().insert((identity.scope.clone(),id.into()),EffectState::Completed);
                for row in self.memory.locked().iter_mut(){if row.detail.as_ref().is_some_and(|v|v["receipt_actor"]==identity.scope && v["request_id"]==id){row.detail.as_mut().unwrap()["stop"]=json!(EffectState::Completed);}}}
        }
        let stop=self.disconnect_state(&identity.scope,id)?.ok_or_else(||MutationError::persistence("stop intent missing"))?;
        Ok(StopResponse{result,stop})
    }
}
