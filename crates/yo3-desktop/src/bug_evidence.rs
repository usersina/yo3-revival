//! Private report receipts. OAuth credentials belong in the system credential store, never this payload.
use std::path::Path;
use serde::{Deserialize,Serialize};
use anyhow::{Context,ensure};
use rusqlite::{Connection,OptionalExtension};
pub const MAX_PAYLOAD_BYTES: usize = 10*1024*1024;
const MAX_RECEIPT_BYTES: usize = MAX_PAYLOAD_BYTES+16384;

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum PendingPost { Gist, Issue }
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Receipt {
    pub report_id: String,
    pub account_login: Option<String>,
    pub gist_id: Option<String>,
    pub gist_url: Option<String>,
    pub issue_number: Option<u64>,
    pub issue_url: Option<String>,
    pub pending: Option<PendingPost>,
    pub payload: Option<serde_json::Value>,
}
impl Receipt {
    pub fn new(report_id: impl Into<String>) -> Self {Self {report_id:report_id.into(),account_login:None,gist_id:None,gist_url:None,issue_number:None,issue_url:None,pending:None,payload:None}}
    fn unfinished(&self)->bool {self.pending.is_some() || ((self.gist_id.is_some() || self.gist_url.is_some()) && self.issue_number.is_none() && self.issue_url.is_none())}
    fn completed(&self)->bool {self.pending.is_none() && (self.issue_number.is_some() || self.issue_url.is_some())}
    fn validate(&self)->anyhow::Result<()> {
        ensure!(yo3_tunnel::diagnostics::valid_id(&self.report_id),"The report ID is invalid.");
        if let Some(payload)=&self.payload {ensure!(serde_json::to_vec(payload)?.len()<=MAX_PAYLOAD_BYTES,"The saved report payload exceeds the 10 MiB limit. Existing report state was kept.");}
        Ok(())
    }
}
/// Hold this handle for the entire publisher run. Persistent files avoid unlink/recreation inode races.
pub fn lock(root:&Path,report_id:&str)->anyhow::Result<std::fs::File> {
    ensure!(yo3_tunnel::diagnostics::valid_id(report_id),"The report ID is invalid.");
    let reports=root.join("reports");yo3_tunnel::diagnostics::private_dir(&reports)?;let directory=reports.join("locks");yo3_tunnel::diagnostics::private_dir(&directory)?;
    let path=directory.join(format!("{report_id}.lock"));
    if let Ok(meta)=std::fs::symlink_metadata(&path) {ensure!(meta.is_file() && !meta.file_type().is_symlink(),"The report lock is not a regular private file: {}",path.display());}
    let mut options=std::fs::OpenOptions::new();options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    let file=options.open(&path).with_context(||format!("Could not open the report lock: {}",path.display()))?;
    match file.try_lock() {
        Ok(())=>Ok(file),
        Err(std::fs::TryLockError::WouldBlock)=>anyhow::bail!("This report is already being handled by another launcher window. Wait for it to finish, then check again."),
        Err(std::fs::TryLockError::Error(error))=>Err(error).with_context(||format!("Could not lock the report: {}",path.display())),
    }
}
fn database(root:&Path,create:bool)->anyhow::Result<Option<Connection>> {
    let directory=root.join("reports");let path=directory.join("bug-reports.db");
    if create {yo3_tunnel::diagnostics::private_dir(&directory)?;}
    match std::fs::symlink_metadata(&path) {
        Ok(meta)=>ensure!(meta.is_file() && !meta.file_type().is_symlink(),"The report database is not a regular private file: {}",path.display()),
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{if !create {return Ok(None)}yo3_tunnel::diagnostics::private_create(&path)?;},
        Err(error)=>return Err(error).with_context(||format!("Could not inspect the report database: {}",path.display())),
    }
    let conn=Connection::open_with_flags(&path,rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE).with_context(||format!("Could not open the report database: {}",path.display()))?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    let version:u32=conn.pragma_query_value(None,"user_version",|row|row.get(0))?;
    ensure!(version<=1,"The report database belongs to a newer application version.");
    conn.pragma_update(None,"synchronous","FULL")?;
    if version==0 {conn.execute_batch("BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS receipts (report_id TEXT PRIMARY KEY,body TEXT NOT NULL,updated INTEGER NOT NULL,revision INTEGER NOT NULL,unfinished INTEGER NOT NULL,completed INTEGER NOT NULL); PRAGMA user_version=1; COMMIT;")?;}
    Ok(Some(conn))
}
/// Commit the intent before making a create request. Payload contains sanitized report data, never OAuth tokens.
pub fn save(root: &Path, receipt: &Receipt) -> anyhow::Result<()> {
    receipt.validate()?;let body=serde_json::to_string(receipt)?;ensure!(body.len()<=MAX_RECEIPT_BYTES,"The saved report receipt is too large. Existing report state was kept.");
    let mut conn=database(root,true)?.context("The report database is unavailable.")?;let transaction=conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let revision:i64=transaction.query_row("SELECT COALESCE(MAX(revision),0)+1 FROM receipts",[],|row|row.get(0))?;let now=i64::try_from(yo3_tunnel::diagnostics::now()).context("The report timestamp is out of range.")?;
    transaction.execute("INSERT INTO receipts(report_id,body,updated,revision,unfinished,completed) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(report_id) DO UPDATE SET body=excluded.body,updated=excluded.updated,revision=excluded.revision,unfinished=excluded.unfinished,completed=excluded.completed",
        rusqlite::params![receipt.report_id,body,now,revision,receipt.unfinished(),receipt.completed()])?;
    transaction.execute("DELETE FROM receipts WHERE (completed=1 AND (updated<?1 OR report_id IN (SELECT report_id FROM receipts WHERE completed=1 ORDER BY revision DESC LIMIT -1 OFFSET 20))) OR (completed=0 AND unfinished=0 AND updated<?1)",[now.saturating_sub(14*86400)])?;
    transaction.commit()?;Ok(())
}
fn read(conn:&Connection,report_id:&str)->anyhow::Result<Option<Receipt>> {
    let size:Option<i64>=conn.query_row("SELECT length(CAST(body AS BLOB)) FROM receipts WHERE report_id=?1",[report_id],|row|row.get(0)).optional()?;
    let Some(size)=size else{return Ok(None)};ensure!(size>=0 && size<=MAX_RECEIPT_BYTES as i64,"The saved report receipt is too large. It was not loaded or changed.");
    let body:Option<String>=conn.query_row("SELECT body FROM receipts WHERE report_id=?1",[report_id],|row|row.get(0)).optional()?;
    let Some(body)=body else{return Ok(None)};let receipt:Receipt=serde_json::from_str(&body).context("The saved report receipt is invalid.")?;
    ensure!(receipt.report_id==report_id,"The saved report ID does not match its receipt.");receipt.validate()?;Ok(Some(receipt))
}
pub fn load(root: &Path, report_id: &str) -> anyhow::Result<Option<Receipt>> {
    ensure!(yo3_tunnel::diagnostics::valid_id(report_id),"The report ID is invalid.");
    let Some(conn)=database(root,false)? else{return Ok(None)};read(&conn,report_id)
}
pub fn latest_pending(root: &Path) -> anyhow::Result<Option<Receipt>> {
    let Some(conn)=database(root,false)? else{return Ok(None)};
    let report_id:Option<String>=conn.query_row("SELECT report_id FROM receipts WHERE unfinished=1 ORDER BY revision DESC LIMIT 1",[],|row|row.get(0)).optional()?;
    report_id.map(|id|read(&conn,&id)).transpose().map(Option::flatten)
}
