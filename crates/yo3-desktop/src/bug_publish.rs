//! Durable, explicit report publication. Unknown create results are reconciled, never blindly retried.
use std::{collections::BTreeMap,path::Path,sync::atomic::{AtomicBool,Ordering}};
use serde::{Deserialize,Serialize};
use crate::{bug_evidence::{self,PendingPost,Receipt},github_report::{self,Failure,Gist,Published,Token}};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Payload {pub title:String,pub content:String,pub files:BTreeMap<String,String>}
#[derive(Clone,Debug)]
pub struct PublishError {pub receipt:Box<Receipt>,pub message:String,pub auth_expired:bool,pub uncertain:bool}
impl std::fmt::Display for PublishError {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str(&self.message)}}
impl std::error::Error for PublishError {}

trait Gateway {
    fn upload_gist(&self,files:&BTreeMap<String,String>,report_id:&str)->impl std::future::Future<Output=Result<Gist,Failure>>+Send;
    fn find_gist(&self,report_id:&str)->impl std::future::Future<Output=Result<Option<Gist>,Failure>>+Send;
    fn find_issue(&self,report_id:&str)->impl std::future::Future<Output=Result<Option<Published>,Failure>>+Send;
    fn publish_issue(&self,title:&str,body:&str,report_id:&str)->impl std::future::Future<Output=Result<Published,Failure>>+Send;
}
struct GitHub<'a>{client:&'a github_report::Client,token:&'a Token}
impl Gateway for GitHub<'_> {
    async fn upload_gist(&self,files:&BTreeMap<String,String>,report_id:&str)->Result<Gist,Failure>{self.client.upload_gist(self.token,files,report_id).await}
    async fn find_gist(&self,report_id:&str)->Result<Option<Gist>,Failure>{self.client.find_gist(self.token,report_id).await}
    async fn find_issue(&self,report_id:&str)->Result<Option<Published>,Failure>{self.client.find_issue(self.token,report_id).await}
    async fn publish_issue(&self,title:&str,body:&str,report_id:&str)->Result<Published,Failure>{self.client.publish_issue(self.token,title,body,report_id).await}
}
/// The UI runs this worker in a detached Tokio task so closing a dialog cannot cancel post-result persistence.
pub async fn publish(root:&Path,client:&github_report::Client,token:&Token,login:&str,receipt:Receipt,cancel:&AtomicBool)->Result<Receipt,PublishError> {
    publish_with(root,&GitHub{client,token},login,receipt,cancel).await
}
fn error(receipt:&Receipt,message:impl Into<String>,uncertain:bool)->PublishError {PublishError{receipt:Box::new(receipt.clone()),message:message.into(),auth_expired:false,uncertain}}
fn persist(root:&Path,receipt:&Receipt,uncertain:bool)->Result<(),PublishError> {
    bug_evidence::save(root,receipt).map_err(|failure|error(receipt,format!("Could not save report progress: {failure:#}. Keep this report and retry after repairing the application data folder."),uncertain))
}
fn before_post(root:&Path,receipt:&mut Receipt,pending:PendingPost)->Result<(),PublishError> {
    let previous=receipt.pending;receipt.pending=Some(pending);
    if let Err(failure)=bug_evidence::save(root,receipt) {receipt.pending=previous;return Err(error(receipt,format!("Could not save the report before posting: {failure:#}. Nothing was posted; repair the application data folder and retry."),false))}
    Ok(())
}
fn request_failure(root:&Path,mut receipt:Receipt,failure:Failure)->PublishError {
    let mut message=failure.message;
    if !failure.uncertain {
        receipt.pending=None;
        if let Err(storage)=bug_evidence::save(root,&receipt) {message.push_str(&format!(" Report progress could not be saved: {storage:#}."));}
    }
    PublishError{receipt:Box::new(receipt),message,auth_expired:failure.auth_expired,uncertain:failure.uncertain}
}
fn check_cancel(receipt:&Receipt,cancel:&AtomicBool)->Result<(),PublishError> {
    if cancel.load(Ordering::Relaxed) {return Err(error(receipt,"Report posting was cancelled. Any completed upload was saved; no further post was started.",receipt.pending.is_some()))}
    Ok(())
}
fn cancel_unstarted_post(root:&Path,receipt:&mut Receipt,cancel:&AtomicBool)->Result<(),PublishError> {
    if !cancel.load(Ordering::Relaxed) {return Ok(())}
    let pending=receipt.pending;receipt.pending=None;
    if let Err(failure)=bug_evidence::save(root,receipt) {receipt.pending=pending;return Err(error(receipt,format!("Posting was cancelled, but its saved intent could not be cleared: {failure:#}. No new post was started; keep this report to check its status safely."),true))}
    Err(error(receipt,"Report posting was cancelled before the request started. Completed uploads remain saved.",false))
}
async fn publish_with<G:Gateway+Sync>(root:&Path,gateway:&G,login:&str,mut receipt:Receipt,cancel:&AtomicBool)->Result<Receipt,PublishError> {
    let _lock=bug_evidence::lock(root,&receipt.report_id).map_err(|failure|error(&receipt,format!("{failure:#}"),receipt.pending.is_some()))?;
    if let Some(saved)=bug_evidence::load(root,&receipt.report_id).map_err(|failure|error(&receipt,format!("Could not read saved report progress: {failure:#}. No new post was made."),true))? {
        if saved.payload!=receipt.payload {return Err(error(&saved,"The saved report content differs from this draft. Resume the original report, or prepare a new report with a new ID; no new post was made.",saved.pending.is_some()))}
        receipt=saved;
    }
    if login.is_empty() || receipt.account_login.as_ref().is_some_and(|owner|!owner.eq_ignore_ascii_case(login)) {
        return Err(error(&receipt,"This report belongs to a different GitHub account. Sign in with the original account to resume it.",receipt.pending.is_some()))
    }
    if receipt.account_login.is_none() && (receipt.pending.is_some() || receipt.gist_id.is_some() || receipt.gist_url.is_some() || receipt.issue_number.is_some() || receipt.issue_url.is_some()) {
        return Err(error(&receipt,"The saved report's GitHub account is missing. Its previous posting status must be checked before any new post.",true))
    }
    check_cancel(&receipt,cancel)?;receipt.account_login=Some(login.into());
    if receipt.issue_number.is_some() || receipt.issue_url.is_some() {receipt.pending=None;persist(root,&receipt,true)?;return Ok(receipt)}
    let payload:Payload=receipt.payload.as_ref().ok_or_else(||error(&receipt,"The saved report content is missing. Prepare a new report; no new post was made.",receipt.pending.is_some()))
        .and_then(|value|Payload::deserialize(value).map_err(|_|error(&receipt,"The saved report content is invalid. No new post was made.",receipt.pending.is_some())))?;
    if payload.title.trim().is_empty() || payload.content.trim().is_empty() || payload.files.is_empty() {
        return Err(error(&receipt,"The report needs a title, description and diagnostic evidence before it can be posted.",false))
    }
    if receipt.pending==Some(PendingPost::Issue) {
        let found=gateway.find_issue(&receipt.report_id).await.map_err(|failure|PublishError{receipt:Box::new(receipt.clone()),message:failure.message,auth_expired:failure.auth_expired,uncertain:true})?;
        let Some(issue)=found else{return Err(error(&receipt,"GitHub has not confirmed whether the earlier issue was created. Check again later; it will not be posted a second time.",true))};
        receipt.issue_number=Some(issue.number);receipt.issue_url=Some(issue.url);receipt.pending=None;persist(root,&receipt,true)?;return Ok(receipt)
    }
    if receipt.pending==Some(PendingPost::Gist) {
        let found=gateway.find_gist(&receipt.report_id).await.map_err(|failure|PublishError{receipt:Box::new(receipt.clone()),message:failure.message,auth_expired:failure.auth_expired,uncertain:true})?;
        let Some(gist)=found else{return Err(error(&receipt,"GitHub has not confirmed whether the earlier evidence upload completed. Check again later; it will not be uploaded a second time.",true))};
        receipt.gist_id=Some(gist.id);receipt.gist_url=Some(gist.url);receipt.pending=None;persist(root,&receipt,true)?;
    }
    if receipt.gist_id.is_none() && receipt.gist_url.is_none() {
        check_cancel(&receipt,cancel)?;before_post(root,&mut receipt,PendingPost::Gist)?;cancel_unstarted_post(root,&mut receipt,cancel)?;
        let gist=match gateway.upload_gist(&payload.files,&receipt.report_id).await {Ok(gist)=>gist,Err(failure)=>return Err(request_failure(root,receipt,failure))};
        receipt.gist_id=Some(gist.id);receipt.gist_url=Some(gist.url);receipt.pending=None;persist(root,&receipt,true)?;
    }
    let gist_url=receipt.gist_url.as_deref().ok_or_else(||error(&receipt,"The uploaded evidence URL is missing. The evidence will not be uploaded again; check its saved report state.",true))?;
    let body=crate::bug_report::with_gist(&payload.content,gist_url,&receipt.report_id).map_err(|failure|error(&receipt,format!("The uploaded evidence link is invalid: {failure:#}. No issue was posted."),false))?;
    check_cancel(&receipt,cancel)?;before_post(root,&mut receipt,PendingPost::Issue)?;cancel_unstarted_post(root,&mut receipt,cancel)?;
    let issue=match gateway.publish_issue(&payload.title,&body,&receipt.report_id).await {Ok(issue)=>issue,Err(failure)=>return Err(request_failure(root,receipt,failure))};
    receipt.issue_number=Some(issue.number);receipt.issue_url=Some(issue.url);receipt.pending=None;persist(root,&receipt,true)?;Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf,sync::{Arc,Mutex}};
    use github_report::Stage;
    #[derive(Default)]
    struct State {uploads:usize,issues:usize,gist_lookups:usize,issue_lookups:usize,gist:Option<Gist>,issue:Option<Published>,gist_error:Option<Failure>,issue_error:Option<Failure>,cancel_after_gist:Option<Arc<AtomicBool>>,cancel_after_issue:Option<Arc<AtomicBool>>,upload_started:Option<Arc<tokio::sync::Notify>>,release_upload:Option<Arc<tokio::sync::Notify>>}
    struct Fake {root:PathBuf,state:Mutex<State>}
    impl Gateway for Fake {
        async fn upload_gist(&self,_files:&BTreeMap<String,String>,report_id:&str)->Result<Gist,Failure> {
            let disk=bug_evidence::load(&self.root,report_id).unwrap().unwrap();assert_eq!(disk.pending,Some(PendingPost::Gist));assert!(disk.payload.is_some());assert_eq!(disk.account_login.as_deref(),Some("TestDuelist"));
            let (release,result)={let mut state=self.state.lock().unwrap();state.uploads+=1;let gist=Gist{id:"abc123".into(),url:"https://gist.github.com/abc123".into()};
                if state.gist_error.as_ref().is_none_or(|error|error.uncertain){state.gist=Some(gist.clone());}
                if let Some(cancel)=&state.cancel_after_gist {cancel.store(true,Ordering::Relaxed);}
                if let Some(started)=&state.upload_started {started.notify_one();}
                (state.release_upload.clone(),state.gist_error.take().map_or(Ok(gist),Err))};
            if let Some(release)=release {release.notified().await;}result
        }
        async fn find_gist(&self,_report_id:&str)->Result<Option<Gist>,Failure>{let mut state=self.state.lock().unwrap();state.gist_lookups+=1;Ok(state.gist.clone())}
        async fn find_issue(&self,_report_id:&str)->Result<Option<Published>,Failure>{let mut state=self.state.lock().unwrap();state.issue_lookups+=1;Ok(state.issue.clone())}
        async fn publish_issue(&self,_title:&str,body:&str,report_id:&str)->Result<Published,Failure> {
            let disk=bug_evidence::load(&self.root,report_id).unwrap().unwrap();assert_eq!(disk.pending,Some(PendingPost::Issue));assert_eq!(disk.gist_id.as_deref(),Some("abc123"));assert!(body.contains("https://gist.github.com/abc123"));
            let mut state=self.state.lock().unwrap();state.issues+=1;let issue=Published{number:42,url:"https://github.com/usersina/yo3-revival/issues/42".into()};
            if state.issue_error.as_ref().is_none_or(|error|error.uncertain){state.issue=Some(issue.clone());}
            if let Some(cancel)=&state.cancel_after_issue {cancel.store(true,Ordering::Relaxed);}
            state.issue_error.take().map_or(Ok(issue),Err)
        }
    }
    fn fixture(name:&str)->(PathBuf,Fake,Receipt) {
        let root=std::env::temp_dir().join(format!("yo3-publish-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut receipt=Receipt::new("report-1234");receipt.payload=Some(serde_json::to_value(Payload{title:"Game failure".into(),content:"Manual description".into(),files:BTreeMap::from([("report.json".into(),"sanitized evidence".into())])}).unwrap());
        let fake=Fake{root:root.clone(),state:Mutex::new(State::default())};(root,fake,receipt)
    }
    fn failure(stage:Stage,uncertain:bool)->Failure{Failure{stage,message:"GitHub request failed".into(),auth_expired:false,uncertain}}
    #[tokio::test]
    async fn definite_issue_failure_reuses_uploaded_evidence_after_restart() {
        let (root,fake,receipt)=fixture("definite");fake.state.lock().unwrap().issue_error=Some(failure(Stage::Publish,false));
        let failed=publish_with(&root,&fake,"TestDuelist",receipt,&AtomicBool::new(false)).await.unwrap_err();assert!(!failed.uncertain);assert_eq!(failed.receipt.pending,None);assert_eq!(failed.receipt.gist_id.as_deref(),Some("abc123"));
        let resumed=bug_evidence::latest_pending(&root).unwrap().unwrap();let completed=publish_with(&root,&fake,"TestDuelist",resumed,&AtomicBool::new(false)).await.unwrap();
        assert_eq!(completed.issue_number,Some(42));let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues),(1,2));drop(state);assert!(bug_evidence::latest_pending(&root).unwrap().is_none());std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn uncertain_creates_reconcile_after_restart_without_duplicate_posts() {
        for stage in [Stage::Gist,Stage::Publish] {
            let (root,fake,receipt)=fixture(&format!("uncertain-{stage:?}"));{let mut state=fake.state.lock().unwrap();if stage==Stage::Gist{state.gist_error=Some(failure(stage,true));}else{state.issue_error=Some(failure(stage,true));}}
            let failed=publish_with(&root,&fake,"TestDuelist",receipt,&AtomicBool::new(false)).await.unwrap_err();assert!(failed.uncertain);
            let resumed=bug_evidence::latest_pending(&root).unwrap().unwrap();let completed=publish_with(&root,&fake,"TestDuelist",resumed,&AtomicBool::new(false)).await.unwrap();assert_eq!(completed.issue_number,Some(42));
            let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues),(1,1));assert_eq!(state.gist_lookups,usize::from(stage==Stage::Gist));assert_eq!(state.issue_lookups,usize::from(stage==Stage::Publish));drop(state);std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[tokio::test]
    async fn empty_reconciliation_does_not_retry_a_possibly_successful_create() {
        for pending in [PendingPost::Gist,PendingPost::Issue] {
            let (root,fake,mut receipt)=fixture(&format!("missing-{pending:?}"));receipt.account_login=Some("TestDuelist".into());receipt.pending=Some(pending);bug_evidence::save(&root,&receipt).unwrap();
            for _ in 0..2 {let current=bug_evidence::load(&root,&receipt.report_id).unwrap().unwrap();let failed=publish_with(&root,&fake,"TestDuelist",current,&AtomicBool::new(false)).await.unwrap_err();assert!(failed.uncertain);assert_eq!(failed.receipt.pending,Some(pending));}
            let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues),(0,0));drop(state);std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[tokio::test]
    async fn stale_ui_receipts_cannot_bypass_durable_pending_state_or_replace_frozen_payloads() {
        let (root,fake,stale)=fixture("stale-receipt");let mut saved=stale.clone();saved.account_login=Some("TestDuelist".into());saved.pending=Some(PendingPost::Gist);bug_evidence::save(&root,&saved).unwrap();
        let failed=publish_with(&root,&fake,"TestDuelist",stale.clone(),&AtomicBool::new(false)).await.unwrap_err();assert!(failed.uncertain);assert_eq!(failed.receipt.pending,Some(PendingPost::Gist));
        let mut changed=stale.clone();changed.payload.as_mut().unwrap()["content"]="A changed report".into();let failed=publish_with(&root,&fake,"TestDuelist",changed,&AtomicBool::new(false)).await.unwrap_err();assert!(failed.message.contains("content"));
        assert_eq!(bug_evidence::load(&root,&saved.report_id).unwrap(),Some(saved));
        std::fs::write(root.join("reports/bug-reports.db"),b"not a SQLite database").unwrap();assert!(publish_with(&root,&fake,"TestDuelist",stale,&AtomicBool::new(false)).await.is_err(),"unreadable durable state must fail closed");
        let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues),(0,0));assert_eq!(state.gist_lookups,1);drop(state);std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn two_workers_for_one_report_cannot_post_concurrently() {
        let (root,fake,receipt)=fixture("concurrent");let fake=Arc::new(fake);let started=Arc::new(tokio::sync::Notify::new());let release=Arc::new(tokio::sync::Notify::new());
        {let mut state=fake.state.lock().unwrap();state.upload_started=Some(started.clone());state.release_upload=Some(release.clone());}
        let first={let root=root.clone();let fake=fake.clone();let receipt=receipt.clone();tokio::spawn(async move{publish_with(&root,&*fake,"TestDuelist",receipt,&AtomicBool::new(false)).await})};
        tokio::time::timeout(std::time::Duration::from_secs(2),started.notified()).await.unwrap();
        let second=publish_with(&root,&*fake,"TestDuelist",receipt,&AtomicBool::new(false)).await;release.notify_one();first.await.unwrap().unwrap();
        let failed=second.unwrap_err();assert!(failed.message.contains("already") || failed.message.contains("another"));
        let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues),(1,1));drop(state);
        assert!(root.join("reports/locks/report-1234.lock").is_file(),"lock files stay in place so another process cannot lock a different inode");std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn account_mismatch_and_cancel_before_post_have_no_remote_effects() {
        let (root,fake,mut receipt)=fixture("guard");receipt.account_login=Some("AnotherDuelist".into());
        let failed=publish_with(&root,&fake,"TestDuelist",receipt,&AtomicBool::new(false)).await.unwrap_err();assert!(failed.message.contains("account"));
        let (_,_,receipt)=fixture("guard-cancel");let failed=publish_with(&root,&fake,"TestDuelist",receipt,&AtomicBool::new(true)).await.unwrap_err();assert!(!failed.uncertain);
        let (_,_,mut missing_owner)=fixture("guard-owner");missing_owner.gist_url=Some("https://gist.github.com/abc123".into());
        let failed=publish_with(&root,&fake,"TestDuelist",missing_owner,&AtomicBool::new(false)).await.unwrap_err();assert!(failed.message.contains("account"),"a saved remote URL without an owner cannot be adopted by a different login");
        assert!(!root.join("reports/bug-reports.db").exists());
        let blocked=root.join("blocked");std::fs::create_dir_all(&blocked).unwrap();std::fs::write(blocked.join("reports"),b"not a report directory").unwrap();let (_,_,mut receipt)=fixture("guard-storage");
        let failed=before_post(&blocked,&mut receipt,PendingPost::Gist).unwrap_err();assert!(failed.message.contains("before posting"));assert_eq!(failed.receipt.pending,None);assert!(!failed.uncertain);
        let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues,state.gist_lookups,state.issue_lookups),(0,0,0,0));drop(state);std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn cancellation_after_a_post_still_persists_its_result() {
        for during_issue in [false,true] {
            let (root,fake,receipt)=fixture(&format!("cancel-after-{during_issue}"));let cancel=Arc::new(AtomicBool::new(false));{let mut state=fake.state.lock().unwrap();if during_issue{state.cancel_after_issue=Some(cancel.clone());}else{state.cancel_after_gist=Some(cancel.clone());}}
            let result=publish_with(&root,&fake,"TestDuelist",receipt,&cancel).await;let saved=bug_evidence::load(&root,"report-1234").unwrap().unwrap();assert_eq!(saved.gist_id.as_deref(),Some("abc123"));assert_eq!(saved.pending,None);
            if during_issue {assert_eq!(result.unwrap().issue_number,Some(42));assert_eq!(saved.issue_number,Some(42));}else{assert!(!result.unwrap_err().uncertain);assert_eq!(fake.state.lock().unwrap().issues,0);}
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn cancellation_after_intent_commit_clears_only_the_unstarted_post_and_fails_closed_on_storage_error() {
        let (root,fake,mut receipt)=fixture("cancel-intent");receipt.account_login=Some("TestDuelist".into());before_post(&root,&mut receipt,PendingPost::Gist).unwrap();
        let failed=cancel_unstarted_post(&root,&mut receipt,&AtomicBool::new(true)).unwrap_err();assert!(!failed.uncertain);assert_eq!(receipt.pending,None);assert_eq!(bug_evidence::load(&root,&receipt.report_id).unwrap().unwrap().pending,None);
        before_post(&root,&mut receipt,PendingPost::Gist).unwrap();let conn=rusqlite::Connection::open(root.join("reports/bug-reports.db")).unwrap();conn.execute_batch("CREATE TRIGGER reject_receipt_update BEFORE UPDATE ON receipts BEGIN SELECT RAISE(ABORT,'fixture storage failure'); END;").unwrap();drop(conn);
        let failed=cancel_unstarted_post(&root,&mut receipt,&AtomicBool::new(true)).unwrap_err();assert!(failed.uncertain);assert_eq!(receipt.pending,Some(PendingPost::Gist));assert_eq!(bug_evidence::load(&root,&receipt.report_id).unwrap().unwrap().pending,Some(PendingPost::Gist));
        let state=fake.state.lock().unwrap();assert_eq!((state.uploads,state.issues),(0,0));drop(state);std::fs::remove_dir_all(root).unwrap();
    }
}
