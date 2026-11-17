//! Administrative snapshots and audited access. Intent commits before touching the filesystem.
use std::{fs,path::{Path,PathBuf},time::UNIX_EPOCH};
use chrono::{DateTime,Utc};
use serde::{Deserialize,Serialize};
use serde_json::json;
use utoipa::ToSchema;
use crate::{lock::Locked,logins::Role,store::{self,AuditRow,Receipt,EffectState}};
use super::{Api,Actor,MutationError,MutationErrorCode,mutation::{validate_id,fingerprint}};
#[derive(Debug,Serialize,Deserialize,ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateBackupRequest {}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize,ToSchema)]
pub struct CreateBackupResult {pub name:String,pub request_id:String,pub message:String}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct CreateBackupResponse {pub result:CreateBackupResult,pub backup:EffectState,pub available:bool}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct BackupEntry {pub name:String,pub at:Option<DateTime<Utc>>,pub bytes:String}
#[derive(Debug,Serialize,Deserialize,ToSchema)]
pub struct BackupList {pub backups:Vec<BackupEntry>,pub checked_at:DateTime<Utc>}
#[derive(Clone,Debug,Serialize,Deserialize,ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DownloadBackupRequest {#[schema(min_length=1,max_length=96,pattern="^[A-Za-z0-9][A-Za-z0-9._-]*$")]pub name:String}
#[derive(Serialize,Deserialize)]
struct DownloadReceipt {name:String,bytes:String,modified:String}
pub struct BackupDownload {pub file:fs::File,pub filename:String,pub bytes:u64}
fn invalid(message:&str)->MutationError{MutationError::new(MutationErrorCode::InvalidRequest,message)}
fn missing()->MutationError{MutationError::new(MutationErrorCode::NotFound,"This backup is no longer available. Refresh the backup list.")}
fn io_error(error:impl std::fmt::Display)->MutationError{tracing::error!("Backup operation failed: {error}");MutationError::new(MutationErrorCode::PersistenceFailed,"The backup operation could not be confirmed. Check the server logs and retry the same request.")}
fn snapshot(dir:&Path,name:&str)->Result<Option<PathBuf>,MutationError>{
    if !store::valid_backup_name(name){return Err(invalid("Choose a backup from this server's list."))}
    let root=dir.join("backups");let folder=root.join(name);let file=folder.join("server.db");
    for (path,is_dir) in [(&root,true),(&folder,true),(&file,false)] {
        let metadata=match fs::symlink_metadata(path){Ok(m)=>m,Err(e)if matches!(e.kind(),std::io::ErrorKind::NotFound|std::io::ErrorKind::NotADirectory)=>return Ok(None),Err(e)=>return Err(io_error(e))};
        if metadata.file_type().is_symlink() || metadata.is_dir()!=is_dir || (!is_dir && !metadata.is_file()){return Ok(None)}
    }
    Ok(Some(file))
}
impl Api {
    pub fn read_backups(&self,actor:Actor<'_>)->Result<BackupList,MutationError>{
        actor.resolve(&self.accounts.locked(),Role::Moderator)?;
        let dir=self.data_dir.as_ref().ok_or_else(missing)?;
        let root=dir.join("backups");let mut rows=vec![];
        if fs::symlink_metadata(&root).is_ok_and(|m|m.file_type().is_symlink()){return Err(io_error("the backup folder cannot be a symbolic link"))}
        let entries=match fs::read_dir(root){Ok(e)=>e,Err(e)if e.kind()==std::io::ErrorKind::NotFound=>return Ok(BackupList{backups:rows,checked_at:Utc::now()}),Err(e)=>return Err(io_error(e))};
        for entry in entries {let entry=entry.map_err(io_error)?;let Some(name)=entry.file_name().to_str().map(str::to_string)else{continue};
            if !store::valid_backup_name(&name){continue}
            let Some(path)=snapshot(dir,&name)? else{continue};let metadata=fs::metadata(path).map_err(io_error)?;
            rows.push(BackupEntry{name,at:metadata.modified().ok().map(DateTime::<Utc>::from),bytes:metadata.len().to_string()});
        }
        rows.sort_by(|a,b|b.name.cmp(&a.name));Ok(BackupList{backups:rows,checked_at:Utc::now()})
    }
    pub fn create_backup(&self,request:&CreateBackupRequest,id:&str,actor:Actor<'_>,origin:&str)->Result<CreateBackupResponse,MutationError>{
        validate_id(id)?;let d=self.accounts.locked();let identity=actor.resolve(&d,Role::Admin)?;let hash=fingerprint(request)?;
        let dir=self.data_dir.as_ref().ok_or_else(||invalid("This server has no persistent database to back up."))?;
        if let Some(result)=self.replay::<CreateBackupResult>(&identity.scope,id,"back-up",&hash)? {
            let backup=store::effect(dir,&identity.scope,id).map_err(io_error)?.ok_or_else(||io_error("backup intent missing"))?;
            let available=snapshot(dir,&result.name)?.is_some();return Ok(CreateBackupResponse{result,backup,available})
        }
        let name=format!("{}-{}",chrono::Local::now().format("%Y%m%d-%H%M%S%.9f"),&crate::logins::new_token()[..12]);
        let result=CreateBackupResult{name:name.clone(),request_id:id.into(),message:"Backup request recorded. Check its saved outcome before requesting another.".into()};
        let row=AuditRow{at:Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),actor:identity.by,origin:origin.into(),op:"back-up".into(),target:name.clone(),before:None,after:None,
            detail:Some(json!({"name":name,"request_id":id,"receipt_actor":identity.scope,"backup":EffectState::Pending}))};
        let receipt=Receipt{operation:"back-up".into(),input:hash,result:json!(result)};
        store::together(dir,||{store::audit(dir,&row)?;store::put_receipt(dir,&identity.scope,id,&receipt)?;store::put_backup_effect(dir,&identity.scope,id)?;Ok(())}).map_err(io_error)?;
        drop(d); // Never hold the Directory lock through a potentially slow SQLite snapshot.
        let work=store::back_up_as(dir,&name);
        if let Err(e)=&work{tracing::error!("Backup snapshot {name} failed: {e}");}
        let available=snapshot(dir,&name)?.is_some();
        // A file published before a directory-sync failure has an uncertain durability outcome, not a known absence.
        if (work.is_ok() || !available) && let Err(e)=store::finish_backup_effect(dir,&identity.scope,id,work.is_ok()){tracing::error!("Backup outcome not saved: {e}; never repeat this request automatically");}
        let backup=store::effect(dir,&identity.scope,id).map_err(io_error)?.ok_or_else(||io_error("backup intent missing"))?;
        Ok(CreateBackupResponse{result,backup,available})
    }
    /// Opens the snapshot and commits an access audit/receipt before the HTTP handler can stream any bytes.
    pub fn open_backup_download(&self,request:&DownloadBackupRequest,id:&str,actor:Actor<'_>,origin:&str)->Result<BackupDownload,MutationError>{
        validate_id(id)?;let d=self.accounts.locked();let identity=actor.resolve(&d,Role::Admin)?;let hash=fingerprint(request)?;
        let dir=self.data_dir.as_ref().ok_or_else(missing)?;let path=snapshot(dir,&request.name)?.ok_or_else(missing)?;
        let file=fs::File::open(path).map_err(io_error)?;let metadata=file.metadata().map_err(io_error)?;
        let modified=metadata.modified().map_err(io_error)?.duration_since(UNIX_EPOCH).map_err(io_error)?.as_nanos().to_string();
        if let Some(receipt)=self.replay::<DownloadReceipt>(&identity.scope,id,"download-backup",&hash)? {
            if receipt.bytes!=metadata.len().to_string() || receipt.modified!=modified{return Err(MutationError::new(MutationErrorCode::RequestConflict,"This backup changed. Refresh the list before downloading it again."))}
        }else{
            let receipt=Receipt{operation:"download-backup".into(),input:hash,result:json!(DownloadReceipt{name:request.name.clone(),bytes:metadata.len().to_string(),modified})};
            let row=AuditRow{at:Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),actor:identity.by,origin:origin.into(),op:"download-backup".into(),target:request.name.clone(),before:None,after:None,
                detail:Some(json!({"request_id":id,"bytes":metadata.len().to_string(),"issued":true}))};
            store::together(dir,||{store::audit(dir,&row)?;store::put_receipt(dir,&identity.scope,id,&receipt)?;Ok(())}).map_err(io_error)?;
        }
        Ok(BackupDownload{file,filename:format!("yo3-backup-{}.db",request.name),bytes:metadata.len()})
    }
}
