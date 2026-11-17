//! Account deletion commits every saved domain and retry receipt before disconnecting or notifying players.
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use utoipa::ToSchema;
use crate::{lock::Locked,logins::Role,store::{self,AuditRow,Receipt,EffectState}};
use super::{Api,Actor,MutationError,MutationErrorCode,Op,mutation::{validate_id,fingerprint}};
#[derive(Clone,Debug,Serialize,Deserialize,ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteAccountRequest {
    #[schema(maximum=4294967295_u32)] pub user_id:u32,
    /// Must match user_id. The dashboard asks the administrator to type this account number.
    #[schema(maximum=4294967295_u32)] pub confirm_user_id:u32,
}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize,ToSchema)]
pub struct DeleteAccountResult {
    #[schema(maximum=4294967295_u32)] pub user_id:u32,
    #[schema(maximum=4294967295_u32)] pub players:u32,
    pub request_id:String, pub message:String,
}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct DeleteAccountResponse { pub result:DeleteAccountResult, pub disconnect:Option<EffectState> }
#[derive(Serialize)]
enum Input { Numeric(DeleteAccountRequest), Named(Op) }
fn invalid(s:impl ToString)->MutationError {MutationError::new(MutationErrorCode::InvalidRequest,s.to_string())}
impl Api {
    pub fn delete_account(&self,request:&DeleteAccountRequest,id:&str,actor:Actor<'_>,origin:&str)->Result<DeleteAccountResponse,MutationError> {
        self.delete(Input::Numeric(request.clone()),id,actor,origin).map(|(r,_)|r)
    }
    pub(super) fn named_deletion(&self,op:&Op,id:&str,actor:Actor<'_>,origin:&str)->Result<Value,MutationError> {
        let (r,label)=self.delete(Input::Named(op.clone()),id,actor,origin)?;
        let mut value=json!(r.result);value["deleted"]=json!(label);value["disconnect"]=json!(r.disconnect);
        value["message"]=json!(format!("{label} is deleted with {} player(s).{}",r.result.players,match r.disconnect {
            Some(EffectState::Completed)=>" Its sessions were ended.",Some(_)=>" Disconnect outcome unknown; inspect current sessions.",None=>"",
        }));Ok(value)
    }
    fn delete(&self,input:Input,id:&str,actor:Actor<'_>,origin:&str)->Result<(DeleteAccountResponse,String),MutationError> {
        validate_id(id)?;
        // Same lock order as duel payments: Hub -> accounts -> database. Keep both through publication.
        let hub=self.hub.as_ref().map(|h|h.locked());
        let mut d=self.accounts.locked();let identity=actor.resolve(&d,Role::Admin)?;
        let hash=fingerprint(&input)?;
        if let Some(result)=self.replay::<DeleteAccountResult>(&identity.scope,id,"delete-account",&hash)? {
            let label=match &input {Input::Named(Op::DeleteAccount{who})=>who.clone(),_=>format!("#{}",result.user_id)};
            return Ok((DeleteAccountResponse{result,disconnect:self.disconnect_state(&identity.scope,id)?},label))
        }
        let user=match &input {
            Input::Numeric(r)=> {if r.user_id!=r.confirm_user_id {return Err(invalid("Type the matching account number to confirm deletion."))}r.user_id},
            Input::Named(Op::DeleteAccount{who})=>d.user_of(who).ok_or_else(||MutationError::new(MutationErrorCode::NotFound,"The account no longer exists."))?,
            _=>unreachable!(),
        };
        if d.login_by_id(user).is_some_and(|l|l.role==Role::Admin) && d.entries().filter(|(_,_,a)|d.login_by_id(a.user_id).is_some_and(|l|l.role==Role::Admin)).count()==1 {
            return Err(invalid("Create another administrator before deleting the last administrator account."))
        }
        let crate::accounts::AccountDeletion {mut next,label,players}=d.prepare_delete_account(user).map_err(|_|MutationError::new(MutationErrorCode::NotFound,"The account no longer exists."))?;
        let ids:Vec<u32>=players.iter().map(|(p,_)|*p).collect();
        // With no live Hub, load saved domains under the account lock so concurrent offline calls cannot stage stale copies.
        let stored=if hub.is_none() {
            self.data_dir.as_ref().map(|dir|crate::front::Hub::deletion_data(dir).map(std::sync::Mutex::new)).transpose().map_err(MutationError::persistence)?
        }else{None};
        let mut hub=hub.or_else(||stored.as_ref().map(|h|h.locked()));
        let cleanup=hub.as_ref().map(|h|h.prepare_account_deletion(&ids));
        let names:Vec<String>=std::iter::once(label.clone()).chain(players.iter().map(|(_,n)|n.clone())).collect();
        let disconnect=self.sessions.is_some();
        let result=DeleteAccountResult{user_id:user,players:players.len() as u32,request_id:id.into(),message:format!("Account #{user} was deleted. Backups and exported logs may retain older data.")};
        let row=AuditRow{at:chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),actor:identity.by,origin:origin.into(),op:"delete-account".into(),target:label.clone(),before:None,after:None,
            detail:Some(json!({"request_id":id,"receipt_actor":identity.scope,"disconnect":if disconnect {Some(EffectState::Pending)} else {None}}))};
        let receipt=Receipt{operation:"delete-account".into(),input:hash,result:json!(result)};
        match &self.data_dir {
            Some(dir)=>store::together(dir,|| {
                next.persist()?;if let (Some(cleanup),Some(hub))=(&cleanup,&hub){cleanup.persist(hub)?;}
                store::delete_duel_logs_of(dir,&ids)?;store::forget_in_audit(dir,&names)?;store::audit(dir,&row)?;
                store::put_receipt(dir,&identity.scope,id,&receipt)?;
                if disconnect {store::put_effect(dir,&identity.scope,id,user)?;}Ok(())
            }).map_err(MutationError::persistence)?,
            None=> {
                let mut audit=self.memory.locked();audit.retain(|r|!store::mentions(&names,&r.target,r.detail.as_ref()));audit.push(row);
                self.receipts.locked().insert((identity.scope.clone(),id.into()),receipt);
                if disconnect {self.effects.locked().insert((identity.scope.clone(),id.into()),EffectState::Pending);}
            }
        }
        *d=next;
        if let (Some(hub),Some(cleanup))=(hub.as_mut(),cleanup){hub.publish_account_deletion(cleanup);}
        if disconnect {
            self.sessions.as_ref().expect("sessions present").locked().logout_user(user);
            match &self.data_dir {
                Some(dir)=>if let Err(e)=store::complete_effect(dir,&identity.scope,id){tracing::error!("Deletion disconnect outcome not recorded: {e}; do not replay it");},
                None=> {
                    self.effects.locked().insert((identity.scope.clone(),id.into()),EffectState::Completed);
                    for row in self.memory.locked().iter_mut(){if row.detail.as_ref().is_some_and(|v|v["receipt_actor"]==identity.scope && v["request_id"]==id){row.detail.as_mut().unwrap()["disconnect"]=json!(EffectState::Completed);}}
                }
            }
        }
        Ok((DeleteAccountResponse{result,disconnect:self.disconnect_state(&identity.scope,id)?},label))
    }
}
