//! Review, device sign-in and explicit publication. Opening this dialog never uploads diagnostics.
use super::*;
use std::sync::atomic::{AtomicBool,Ordering};
use yo3_desktop::{bug_evidence::{self,Receipt},bug_publish::{self,Payload,PublishError},bug_report::{Context,Draft},github_report::{self,Client,Session},reports::Report,system_info::SystemInfo};

pub(super) struct BugReport {
    pub description: widget::text_editor::Content,
    pub steps: widget::text_editor::Content,
    pub context: Context,
    pub system: Option<SystemInfo>,
    pub details: bool,
    pub log_options: bool,
    pub warning: Option<String>,
    pub auth_warning: Option<String>,
    pub device: Option<DevicePrompt>,
    pub auth_busy: bool,
    pub receipt: Option<Receipt>,
    pub recovery_checked: bool,
    pub phase: &'static str,
    preview_revision: u64,
    auth_attempt: u64,
    cancel: Arc<AtomicBool>,
}
pub(super) use super::bug_auth::GitHubAccount;
use super::bug_auth::{restore_account,authorize_account,save_account,refresh_account,sign_out};
#[derive(Clone)]
pub(super) struct DevicePrompt {pub user_code:String,pub verification_uri:String}
#[derive(Clone)]
pub(super) struct BugPreview {pub system:SystemInfo,pub report:Option<Report>,pub warning:Option<String>,pub receipt:Option<Receipt>,pub recovery_error:Option<String>}
#[derive(Clone)]
pub(super) enum BugEvent {
    Description(widget::text_editor::Action),Steps(widget::text_editor::Action),Details(bool),LogOptions(bool),
    SignIn,SignOut,StopSignIn,CopyCode,OpenSignIn,RetryPreview,Publish(bool),Copy,CopyLink,OpenIssue,OpenGist,Export,
    Previewed(u64,u64,Result<BugPreview,String>),Restored(u64,Result<Option<GitHubAccount>,String>),
    AuthStarted(u64,u64,Result<Session,String>),Authorized(u64,u64,Result<GitHubAccount,String>),
    BrowserOpened(u64,Result<(),String>),Prepared(u64,Result<Receipt,String>),
    Published(u64,Option<GitHubAccount>,Result<Receipt,PublishError>),SignedOut(u64,Result<(),String>),AccountStored(u64,u64,GitHubAccount),
}
impl BugReport {
    pub(super) fn can_submit(&self)->bool {self.system.is_some() && self.recovery_checked && (self.receipt.is_some() || !self.description.text().trim().is_empty() || !self.context.error.trim().is_empty())}
    pub(super) fn frozen(&self)->bool {self.receipt.is_some()}
    fn draft(&self)->Draft {Draft {description:self.description.text(),steps:self.steps.text()}}
    fn current_auth(&self,attempt:u64)->bool {self.auth_attempt==attempt}
}
impl App {
    pub(super) fn open_bug_report(&mut self)->Task<Msg> {
        if self.bug_report.is_some(){return Task::none()}
        let context=Context {connection:if self.selected==ServerChoice::Local{"Local play"}else{"Remote server"}.into(),
            server_version:self.checked.as_ref().map(|server|server.release.clone()),
            readiness:match &self.setup_result {Some(yo3_desktop::Readiness::Ready)=>"Ready",Some(yo3_desktop::Readiness::GameProblem(_))=>"Game folder needs attention",
                Some(yo3_desktop::Readiness::ComponentProblem(_))=>"Revival components need attention",Some(yo3_desktop::Readiness::ValidationProblem(_))=>"Validation failed",None=>"Not checked"}.into(),
            error:if self.failed{self.error_report()}else{self.monitor_error.clone().unwrap_or_default()},
            missing_files:self.setup_missing_files.iter().map(|path|path.to_string_lossy().into_owned()).collect()};
        self.support=false;self.report_options=true;self.report_remove_personal=false;self.report_feedback=None;self.report_busy=false;self.report_generation+=1;
        let restore=self.github_account.is_none();let generation=self.report_generation;
        self.bug_report=Some(BugReport {description:Default::default(),steps:Default::default(),context,system:None,details:false,log_options:false,
            warning:None,auth_warning:None,device:None,auth_busy:restore,receipt:None,recovery_checked:false,phase:"Collecting system details and logs…",preview_revision:0,auth_attempt:0,cancel:Arc::new(AtomicBool::new(false))});
        let preview=self.start_bug_preview(false);
        let auth=if restore {Task::perform(restore_account(),move|result|Msg::Bug(BugEvent::Restored(generation,result)))}else{Task::none()};
        Task::batch([preview,auth])
    }
    pub(super) fn update_bug_report(&mut self,event:BugEvent)->Task<Msg> {
        if let BugEvent::AccountStored(generation,attempt,account)=event {
            if self.github_account.as_ref().is_some_and(|current|current.same_session(&account)){self.github_account=Some(account);}
            if self.current_auth(generation,attempt){self.bug_report.as_mut().unwrap().auth_busy=false;}
            return Task::none()
        }
        if !self.report_options || self.bug_report.is_none(){return Task::none()}
        match event {
            BugEvent::Description(action)=>if !self.report_busy && !self.bug_report.as_ref().unwrap().frozen(){let edited=action.is_edit();self.bug_report.as_mut().unwrap().description.perform(action);if edited{self.invalidate_bug_files();}},
            BugEvent::Steps(action)=>if !self.report_busy && !self.bug_report.as_ref().unwrap().frozen(){let edited=action.is_edit();self.bug_report.as_mut().unwrap().steps.perform(action);if edited{self.invalidate_bug_files();}},
            BugEvent::Details(value)=>self.bug_report.as_mut().unwrap().details=value,
            BugEvent::LogOptions(value)=>self.bug_report.as_mut().unwrap().log_options=value,
            BugEvent::Previewed(generation,revision,result)=>{
                if !self.current_bug(generation) || self.bug_report.as_ref().unwrap().preview_revision!=revision{return Task::none()}
                self.report_busy=false;let bug=self.bug_report.as_mut().unwrap();
                match result {
                    Ok(preview)=>{bug.system=Some(preview.system);bug.warning=preview.warning;bug.recovery_checked=preview.recovery_error.is_none();self.report=preview.report;
                        if let Some(error)=preview.recovery_error{bug.warning=Some(format!("Could not check the previous report: {error}. Retry preview before publishing to avoid a duplicate."));}
                        if let Some(receipt)=preview.receipt {bug.receipt=Some(receipt);bug.warning=Some("An unfinished report was restored. Its saved details will be reused; GitHub will be checked before any retry.".into());}},
                    Err(error)=>{bug.system=Some(minimal_system());bug.warning=Some(format!("Could not collect diagnostics: {error}. Retry preview before publishing."));self.report=None;}
                }
            }
            BugEvent::Restored(generation,result)=>{
                if !self.current_bug(generation) || self.bug_report.as_ref().unwrap().auth_attempt!=0{return Task::none()}
                let bug=self.bug_report.as_mut().unwrap();bug.auth_busy=false;
                match result {Ok(account)=>self.github_account=account,Err(error)=>bug.auth_warning=Some(error)}
            }
            BugEvent::SignIn=>{
                if self.report_busy || self.bug_report.as_ref().unwrap().auth_busy{return Task::none()}
                let Some(client_id)=github_report::configured_client_id() else {self.bug_report.as_mut().unwrap().auth_warning=Some("GitHub reporting is not configured in this build. Use the complete current Desktop release.".into());return Task::none()};
                let bug=self.bug_report.as_mut().unwrap();bug.cancel.store(true,Ordering::Relaxed);bug.cancel=Arc::new(AtomicBool::new(false));bug.auth_attempt+=1;bug.auth_busy=true;bug.device=None;bug.auth_warning=None;
                let attempt=bug.auth_attempt;let generation=self.report_generation;
                return Task::perform(async move {Client::new().map_err(|error|error.to_string())?.start_auth(&client_id).await.map_err(|error|error.to_string())},move|result|Msg::Bug(BugEvent::AuthStarted(generation,attempt,result)))
            }
            BugEvent::AuthStarted(generation,attempt,result)=>{
                if !self.current_auth(generation,attempt){return Task::none()}
                match result {
                    Ok(session)=>{
                        let url=session.verification_uri.clone();let bug=self.bug_report.as_mut().unwrap();bug.device=Some(DevicePrompt{user_code:session.user_code.clone(),verification_uri:session.verification_uri.clone()});let cancel=bug.cancel.clone();
                        let Some(client_id)=github_report::configured_client_id() else {bug.auth_busy=false;bug.auth_warning=Some("GitHub reporting is not configured in this build.".into());return Task::none()};
                        return Task::batch([Task::perform(async move {authorize_account(&client_id,session,&cancel).await},move|result|Msg::Bug(BugEvent::Authorized(generation,attempt,result))),open_report_link(generation,url)])
                    }
                    Err(error)=>{let bug=self.bug_report.as_mut().unwrap();bug.auth_busy=false;bug.auth_warning=Some(error);}
                }
            }
            BugEvent::Authorized(generation,attempt,result)=>{
                if !self.current_auth(generation,attempt){return Task::none()}
                let bug=self.bug_report.as_mut().unwrap();bug.auth_busy=false;bug.device=None;
                match result {Ok(account)=>{self.github_account=Some(account.clone());bug.auth_warning=None;bug.auth_busy=true;self.report_feedback=None;
                    return Task::perform(save_account(account),move|account|Msg::Bug(BugEvent::AccountStored(generation,attempt,account)))},Err(error)=>bug.auth_warning=Some(error)}
            }
            BugEvent::StopSignIn=>{
                let bug=self.bug_report.as_mut().unwrap();if self.report_busy || (self.github_account.is_some() && bug.device.is_none()){return Task::none()}bug.cancel.store(true,Ordering::Relaxed);bug.auth_attempt+=1;bug.auth_busy=false;bug.device=None;bug.auth_warning=None;
            }
            BugEvent::SignOut=>{
                if self.report_busy || self.bug_report.as_ref().unwrap().auth_busy{return Task::none()}
                let Some(account)=self.github_account.take() else{return Task::none()};account.revoke();let generation=self.report_generation;
                self.bug_report.as_mut().unwrap().auth_busy=true;
                return Task::perform(async move {sign_out(account).await},move|result|Msg::Bug(BugEvent::SignedOut(generation,result)))
            }
            BugEvent::SignedOut(generation,result)=>if self.current_bug(generation){let bug=self.bug_report.as_mut().unwrap();bug.auth_busy=false;if let Err(error)=result{bug.auth_warning=Some(format!("Signed out of this session. {error}. Revoke access in GitHub Settings → Applications if needed."));}},
            BugEvent::CopyCode=>if let Some(device)=&self.bug_report.as_ref().unwrap().device{return iced::clipboard::write(device.user_code.clone())},
            BugEvent::OpenSignIn=>if let Some(device)=&self.bug_report.as_ref().unwrap().device{return open_report_link(self.report_generation,device.verification_uri.clone())},
            BugEvent::RetryPreview=>return self.start_bug_preview(self.report_all),
            BugEvent::Publish(without_logs)=>{
                let bug=self.bug_report.as_ref().unwrap();
                if self.report_busy || bug.auth_busy || !bug.can_submit() || self.github_account.is_none(){return Task::none()}
                if bug.receipt.as_ref().is_some_and(|receipt|receipt.issue_url.is_some()){return Task::none()}
                if bug.receipt.is_some(){return self.publish_bug_receipt()}
                let draft=bug.draft();let context=bug.context.clone();let system=bug.system.clone().unwrap();let root=self.root.clone();let generation=self.report_generation;
                let report=if without_logs{None}else{self.report.clone()};let privacy=self.report_remove_personal;let cancel=bug.cancel.clone();
                self.report_busy=true;self.report_feedback=None;self.bug_report.as_mut().unwrap().phase="Preparing your report…";
                return Task::perform(async move {tokio::task::spawn_blocking(move||prepare_report(&root,report.as_ref(),&draft,&context,&system,privacy,&cancel)).await
                    .map_err(|_|"Report preparation was interrupted. Try again or export logs.".to_string())?.map_err(|error|format!("Could not prepare the report: {error:#}"))},move|result|Msg::Bug(BugEvent::Prepared(generation,result)))
            }
            BugEvent::Prepared(generation,result)=>{
                if !self.current_bug(generation){return Task::none()}
                self.report_busy=false;
                match result {Ok(receipt)=>{self.bug_report.as_mut().unwrap().receipt=Some(receipt);return self.publish_bug_receipt()},Err(error)=>self.report_feedback=Some(Err(error))}
            }
            BugEvent::Published(generation,account,result)=>{
                if !self.current_bug(generation){return Task::none()}
                self.report_busy=false;if let Some(account)=account{self.github_account=Some(account);}
                match result {
                    Ok(receipt)=>{self.bug_report.as_mut().unwrap().receipt=Some(receipt);self.report_feedback=Some(Ok("Your bug report was published with its diagnostic logs. Thank you!".into()));},
                    Err(error)=>{if error.auth_expired{self.github_account=None;self.bug_report.as_mut().unwrap().auth_warning=Some("GitHub sign-in expired. Sign in again to resume this report.".into());}
                        if let Some(backend)=&self.backend {backend.diagnostics.line(&format!("Bug-report publication failed: {}",error.message));}
                        self.bug_report.as_mut().unwrap().receipt=Some(*error.receipt);self.report_feedback=Some(Err(error.message));}
                }
            }
            BugEvent::BrowserOpened(generation,result)=>if self.current_bug(generation) && result.is_err(){self.report_feedback=Some(Err(if self.bug_report.as_ref().unwrap().device.is_some(){"Could not open your browser. Visit github.com/login/device and enter the code shown here."}else{"Could not open your browser. Copy the link below and open it yourself; your report remains saved."}.into()));},
            BugEvent::Copy=>{
                let bug=self.bug_report.as_ref().unwrap();
                let content=bug.receipt.as_ref().and_then(|receipt|receipt.payload.as_ref()).and_then(|value|serde_json::from_value::<Payload>(value.clone()).ok()).map(|payload|payload.content)
                    .unwrap_or_else(||bug.draft().issue_content(&bug.context,bug.system.as_ref().unwrap_or(&minimal_system()),self.report_remove_personal));
                let content=bug.receipt.as_ref().and_then(|receipt|receipt.gist_url.as_ref().map(|url|(receipt,url)))
                    .and_then(|(receipt,url)|yo3_desktop::bug_report::with_gist(&content,url,&receipt.report_id).ok()).unwrap_or(content);
                return iced::clipboard::write(content)
            }
            BugEvent::CopyLink=>if let Some(receipt)=&self.bug_report.as_ref().unwrap().receipt && let Some(url)=receipt.issue_url.as_ref().or(receipt.gist_url.as_ref()){return iced::clipboard::write(url.clone())},
            BugEvent::OpenIssue=>if let Some(url)=self.bug_report.as_ref().unwrap().receipt.as_ref().and_then(|receipt|receipt.issue_url.clone()){return open_report_link(self.report_generation,url)},
            BugEvent::OpenGist=>if let Some(url)=self.bug_report.as_ref().unwrap().receipt.as_ref().and_then(|receipt|receipt.gist_url.clone()){return open_report_link(self.report_generation,url)},
            BugEvent::AccountStored(..)=>unreachable!("handled before dialog guards"),
            BugEvent::Export=>{if self.report_busy{return Task::none()}self.cancel_bug_task();self.bug_report=None;self.report_options=false;self.report_feedback=None;self.report_generation+=1;return self.update(Msg::ExportLogs)},
        }
        Task::none()
    }
    fn current_bug(&self,generation:u64)->bool {self.report_options && self.bug_report.is_some() && generation==self.report_generation}
    fn current_auth(&self,generation:u64,attempt:u64)->bool {self.current_bug(generation) && self.bug_report.as_ref().unwrap().current_auth(attempt)}
    pub(super) fn cancel_bug_task(&self) {if let Some(bug)=&self.bug_report{bug.cancel.store(true,Ordering::Relaxed);}}
    pub(super) fn start_bug_preview(&mut self,all:bool)->Task<Msg> {
        if self.report_busy || self.bug_report.as_ref().is_none_or(BugReport::frozen){return Task::none()}
        self.report_all=all;self.report=None;self.invalidate_bug_files();self.report_busy=true;
        let bug=self.bug_report.as_mut().unwrap();bug.preview_revision+=1;bug.phase="Collecting system details and logs…";
        let revision=bug.preview_revision;let check_recovery=!bug.recovery_checked;let generation=self.report_generation;let backend=self.backend.clone();let root=self.root.clone();
        Task::perform(async move {tokio::task::spawn_blocking(move||{
            let system=SystemInfo::collect();
            let (report,warning)=match backend.map(|backend|backend.report(all)) {
                Some(Ok(report))=>(Some(report),None),Some(Err(error))=>(None,Some(format!("Session logs are unavailable: {error:#}"))),None=>(None,Some("App setup is unavailable. Session logs could not be collected.".into()))};
            let (receipt,recovery_error)=if check_recovery {match bug_evidence::latest_pending(&root){Ok(receipt)=>(receipt,None),Err(error)=>(None,Some(format!("{error:#}")))}}else{(None,None)};
            BugPreview{system,report,warning,receipt,recovery_error}
        }).await.map_err(|_|"Diagnostic collection was interrupted".to_string())},move|result|Msg::Bug(BugEvent::Previewed(generation,revision,result)))
    }
    pub(super) fn invalidate_bug_files(&mut self) {if self.bug_report.as_ref().is_some_and(|bug|!bug.frozen()){self.report_feedback=None;}}
    fn publish_bug_receipt(&mut self)->Task<Msg> {
        let Some(account)=self.github_account.clone() else{return Task::none()};let bug=self.bug_report.as_mut().unwrap();let Some(receipt)=bug.receipt.clone() else{return Task::none()};
        if receipt.issue_url.is_some(){return Task::none()}
        bug.phase=if receipt.pending.is_some(){"Checking whether GitHub received this report…"}else{"Uploading diagnostics and publishing your issue…"};
        let cancel=bug.cancel.clone();let root=self.root.clone();let generation=self.report_generation;
        self.report_busy=true;self.report_feedback=None;
        Task::perform(async move {
            // Dropping an Iced task must not discard the result of an already-sent POST.
            let backup=receipt.clone();let recovery_root=root.clone();let recovery_login=account.login.clone();let result=tokio::spawn(async move {
                let account=match refresh_account(account).await {
                    Ok(account)=>account,Err(error)=>return (None,Err(PublishError{receipt:Box::new(receipt),message:error.message,auth_expired:error.auth_expired,uncertain:false}))};
                let result=match Client::new(){Ok(client)=>bug_publish::publish(&root,&client,&account.token(),&account.login,receipt,&cancel).await,
                    Err(error)=>Err(PublishError{receipt:Box::new(receipt),message:error.message,auth_expired:error.auth_expired,uncertain:false})};
                (Some(account),result)
            }).await;
            match result {
                Ok(result)=>result,
                Err(_)=>{
                    let id=backup.report_id.clone();let saved=tokio::task::spawn_blocking(move||bug_evidence::load(&recovery_root,&id)).await.ok().and_then(Result::ok).flatten();
                    let mut receipt=saved.unwrap_or(backup);
                    if receipt.issue_url.is_none() && receipt.pending.is_none(){receipt.pending=Some(if receipt.gist_url.is_some(){bug_evidence::PendingPost::Issue}else{bug_evidence::PendingPost::Gist});receipt.account_login.get_or_insert(recovery_login);}
                    (None,Err(PublishError{receipt:Box::new(receipt),message:"Report publication was interrupted. Check report status before retrying.".into(),auth_expired:false,uncertain:true}))
                }
            }
        },move|(account,result)|Msg::Bug(BugEvent::Published(generation,account,result)))
    }
}
fn minimal_system()->SystemInfo {SystemInfo{os:std::env::consts::OS.into(),arch:std::env::consts::ARCH.into(),wine:None}}
fn open_report_link(generation:u64,url:String)->Task<Msg> {
    Task::perform(async move {if !safe_report_url(&url){return Err("Invalid GitHub report link".to_string())}tokio::task::spawn_blocking(move||open(&url)).await.map_err(|_|"Browser launch was interrupted".to_string())?.map_err(|_|"Browser launch failed".to_string())},move|result|Msg::Bug(BugEvent::BrowserOpened(generation,result)))
}
fn safe_report_url(value:&str)->bool {
    let Ok(url)=reqwest::Url::parse(value) else{return false};
    if url.scheme()!="https" || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() || url.port().is_some(){return false}
    if url.host_str()==Some("gist.github.com"){return yo3_desktop::bug_report::with_gist("report",value,"report-1234").is_ok()}
    if url.host_str()!=Some("github.com"){return false}
    url.path()=="/login/device" || url.path().strip_prefix("/usersina/yo3-revival/issues/").is_some_and(|number|!number.is_empty() && number.bytes().all(|byte|byte.is_ascii_digit()))
}
fn prepare_report(root:&std::path::Path,report:Option<&Report>,draft:&Draft,context:&Context,system:&SystemInfo,privacy:bool,cancel:&AtomicBool)->anyhow::Result<Receipt> {
    use ring::rand::SecureRandom;
    let mut random=[0u8;16];ring::rand::SystemRandom::new().fill(&mut random).map_err(|_|anyhow::anyhow!("Could not create a unique report ID"))?;
    let id=random.iter().map(|byte|format!("{byte:02x}")).collect::<String>();
    let content=draft.issue_content(context,system,privacy);
    let longest_link=format!("https://gist.github.com/{}","a".repeat(80));
    anyhow::ensure!(yo3_desktop::bug_report::with_gist(&content,&longest_link,&id)?.len()<=64*1024,
        "The report text exceeds GitHub's 64 KiB issue limit. Shorten the description or steps and try again; nothing was uploaded.");
    let files=if let Some(report)=report {report.gist_files(privacy,&serde_json::to_value(context)?,&content,system)?}
        else {std::collections::BTreeMap::from([("bug-report.md".into(),content.clone()),("report.json".into(),serde_json::json!({"version":yo3_tunnel::BUILD_VERSION,"build":yo3_tunnel::BUILD_COMMIT,"system":system,"session_logs":"unavailable or explicitly excluded"}).to_string())])};
    let title=yo3_desktop::reports::filter_text(&draft.issue_title(context),privacy).split_whitespace().collect::<Vec<_>>().join(" ").chars().take(100).collect::<String>();
    let title=if title.is_empty(){"YO3 Revival problem".into()}else{title};
    let payload=Payload{title,content,files};
    anyhow::ensure!(!cancel.load(Ordering::Acquire),"Report preparation was cancelled; nothing was uploaded.");
    let mut receipt=Receipt::new(id);receipt.payload=Some(serde_json::to_value(payload)?);bug_evidence::save(root,&receipt)?;Ok(receipt)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn app(name:&str)->App {let root=std::env::temp_dir().join(format!("yo3-bug-ui-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);App::load(root)}
    fn system()->SystemInfo {SystemInfo{os:"Windows 11 (build 26100)".into(),arch:"x86_64".into(),wine:None}}
    fn account()->GitHubAccount {GitHubAccount::new("test-client".into(),github_report::Token::new("ghu_fake-token".into()).unwrap(),"TestDuelist".into())}
    fn ready(app:&mut App){let generation=app.report_generation;let _=app.update(Msg::Bug(BugEvent::Previewed(generation,1,Ok(BugPreview{system:system(),report:None,warning:None,receipt:None,recovery_error:None}))));let _=app.update(Msg::Bug(BugEvent::Restored(generation,Ok(Some(account())))));}
    #[test]
    fn automatic_report_can_use_the_captured_error_without_forcing_a_description() {
        let mut app=app("automatic-captured");app.feedback(Err("Game could not start: missing yo3-auth.dll".into()));let _=app.update(Msg::ReportBug);ready(&mut app);
        assert!(app.bug_report.as_ref().unwrap().can_submit());assert_eq!(app.notice,"Game could not start: missing yo3-auth.dll");std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn automatic_report_still_needs_either_description_or_error_and_system_metadata() {
        let mut app=app("automatic-empty");let _=app.update(Msg::ReportBug);let bug=app.bug_report.as_mut().unwrap();assert!(!bug.can_submit());bug.system=Some(system());bug.recovery_checked=true;assert!(!bug.can_submit());
        bug.description=widget::text_editor::Content::with_text("The deck editor is blank");assert!(bug.can_submit());std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn reporting_never_publishes_on_open_or_sign_in_and_late_results_cannot_reopen_a_dialog() {
        let mut app=app("consent");let _=app.update(Msg::ReportBug);let generation=app.report_generation;assert!(app.bug_report.as_ref().unwrap().receipt.is_none());ready(&mut app);
        assert!(app.bug_report.as_ref().unwrap().receipt.is_none());assert!(!app.report_busy);assert_eq!(app.update(Msg::Bug(BugEvent::Publish(false))).units(),0);
        let cancel=app.bug_report.as_ref().unwrap().cancel.clone();let _=app.update(Msg::CancelClose);assert!(cancel.load(Ordering::Relaxed));
        let _=app.update(Msg::Bug(BugEvent::Restored(generation,Ok(Some(account())))));assert!(app.bug_report.is_none());assert!(app.report_feedback.is_none());std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn saved_publication_is_frozen_and_auth_failures_preserve_original_game_error() {
        let mut app=app("freeze");app.feedback(Err("Original native failure stage=mod-load".into()));let _=app.update(Msg::ReportBug);ready(&mut app);let generation=app.report_generation;
        let receipt=prepare_report(&app.root,None,&Draft{description:"The game closes".into(),steps:String::new()},&Context::default(),&system(),false,&AtomicBool::new(false)).unwrap();
        app.bug_report.as_mut().unwrap().receipt=Some(receipt.clone());assert!(app.bug_report.as_ref().unwrap().frozen());
        let _=app.update(Msg::Bug(BugEvent::Published(generation,None,Err(PublishError{receipt:Box::new(receipt),message:"GitHub denied access".into(),auth_expired:true,uncertain:false}))));
        assert_eq!(app.notice,"Original native failure stage=mod-load");assert!(app.github_account.is_none());assert!(app.bug_report.as_ref().unwrap().receipt.is_some());assert!(!app.report_busy);std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn preparation_keeps_exact_sanitized_evidence_and_contains_no_token_credentials() {
        let app=app("payload");let backend=app.backend.as_ref().unwrap();backend.diagnostics.line("Native failure stage=game-exit process_exit_code=0xC0000005 key=ABCDEFGHIJKLMNOPQRST");let report=backend.report(false).unwrap();
        let receipt=prepare_report(&app.root,Some(&report),&Draft{description:"My player cannot summon Stardust Dragon".into(),steps:String::new()},&Context{error:"key=ABCDEFGHIJKLMNOPQRST".into(),..Default::default()},&system(),false,&AtomicBool::new(false)).unwrap();
        let saved=bug_evidence::load(&app.root,&receipt.report_id).unwrap().unwrap();let payload:Payload=serde_json::from_value(saved.payload.unwrap()).unwrap();assert!(payload.content.contains("Stardust Dragon"));assert!(payload.files.contains_key("bug-report.md"));
        let full=serde_json::to_string(&payload).unwrap();assert!(!full.contains("ABCDEFGHIJKLMNOPQRST"));assert!(!full.contains("Nothing is uploaded automatically"));assert!(receipt.gist_id.is_none() && receipt.issue_number.is_none());std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn window_close_cancels_sign_in_before_game_close_confirmation() {
        let mut app=app("close");let _=app.update(Msg::ReportBug);let generation=app.report_generation;let cancel=app.bug_report.as_ref().unwrap().cancel.clone();app.game_running=true;
        let _=app.update(Msg::Close);assert!(app.closing);assert!(app.bug_report.is_none());assert!(cancel.load(Ordering::Relaxed));assert!(app.report_generation>generation);assert!(!app.report_busy);std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn frozen_reports_keep_their_original_privacy_and_scope_and_ignore_duplicate_publish_clicks() {
        let mut app=app("frozen-options");let _=app.update(Msg::ReportBug);ready(&mut app);
        let receipt=prepare_report(&app.root,None,&Draft{description:"The game closes".into(),steps:String::new()},&Context::default(),&system(),false,&AtomicBool::new(false)).unwrap();
        app.bug_report.as_mut().unwrap().receipt=Some(receipt);let _=app.update(Msg::RemovePersonal(true));let _=app.update(Msg::PrepareReport(true));
        assert!(!app.report_remove_personal);assert!(!app.report_all);app.report_busy=true;
        assert_eq!(app.update(Msg::Bug(BugEvent::Publish(false))).units(),0);std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn cancelled_sign_in_cannot_restore_an_account_or_save_credentials() {
        let mut app=app("stale-sign-in");let _=app.update(Msg::ReportBug);ready(&mut app);app.github_account=None;
        let generation=app.report_generation;let bug=app.bug_report.as_mut().unwrap();bug.auth_attempt=1;bug.auth_busy=true;
        let _=app.update(Msg::Bug(BugEvent::StopSignIn));
        assert_eq!(app.update(Msg::Bug(BugEvent::Authorized(generation,1,Ok(account())))).units(),0);
        assert!(app.github_account.is_none());assert!(!app.bug_report.as_ref().unwrap().auth_busy);std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn report_browser_links_cannot_open_local_files_or_unrelated_hosts() {
        for url in ["https://github.com/login/device","https://github.com/usersina/yo3-revival/issues/42","https://gist.github.com/abc123"]{assert!(safe_report_url(url),"{url}");}
        for url in ["file:///tmp/report","https://evil.example/report","https://github.com/usersina/yo3-revival/issues/42?secret=abc","https://github.com/other/repo/issues/42","https://github.com@evil.example/login/device"]{assert!(!safe_report_url(url),"{url}");}
    }
    #[test]
    fn preflight_title_is_a_single_valid_github_line() {
        let app=app("title-preflight");let receipt=prepare_report(&app.root,None,&Draft{description:"Game closes".into(),steps:String::new()},&Context::default(),&system(),false,&AtomicBool::new(false)).unwrap();
        let payload:Payload=serde_json::from_value(receipt.payload.unwrap()).unwrap();assert_eq!(payload.title,"Game closes");assert!(!payload.title.chars().any(char::is_control));std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn preflight_oversized_issue_content_fails_before_freezing_the_draft_or_uploading() {
        let app=app("long-preflight");let result=prepare_report(&app.root,None,&Draft{description:"Full detail ".repeat(7000),steps:String::new()},&Context::default(),&system(),false,&AtomicBool::new(false));
        assert!(result.is_err(),"reject an oversized issue before uploading its evidence");assert!(!app.root.join("reports/bug-reports.db").exists());std::fs::remove_dir_all(app.root).unwrap();
    }

    #[test]
    fn recovery_read_failure_blocks_fresh_publication_until_the_previous_report_is_checked() {
        let mut app=app("recovery-failure");app.feedback(Err("Game closes".into()));let _=app.update(Msg::ReportBug);let generation=app.report_generation;
        let _=app.update(Msg::Bug(BugEvent::Restored(generation,Ok(Some(account())))));
        let _=app.update(Msg::Bug(BugEvent::Previewed(generation,1,Ok(BugPreview{system:system(),report:None,warning:None,receipt:None,recovery_error:Some("Report database is temporarily unreadable".into())}))));
        assert!(!app.bug_report.as_ref().unwrap().can_submit(),"a new report could duplicate an unresolved previous post");assert_eq!(app.update(Msg::Bug(BugEvent::Publish(false))).units(),0);std::fs::remove_dir_all(app.root).unwrap();
    }

    #[test]
    fn waiting_for_github_keeps_the_loading_indicator_animated() {
        let mut app=app("auth-animation");let _=app.update(Msg::ReportBug);ready(&mut app);assert!(!app.animating());
        app.bug_report.as_mut().unwrap().auth_busy=true;assert!(app.animating());std::fs::remove_dir_all(app.root).unwrap();
    }
    #[test]
    fn cancelled_preparation_does_not_leave_an_unstarted_saved_payload() {
        let app=app("cancel-prepare");let result=prepare_report(&app.root,None,&Draft{description:"The game closes".into(),steps:String::new()},&Context::default(),&system(),false,&AtomicBool::new(true));
        assert!(result.is_err());assert!(!app.root.join("reports/bug-reports.db").exists());std::fs::remove_dir_all(app.root).unwrap();
    }

    #[test]
    fn reporting_from_help_remains_available_during_a_busy_game_operation() {
        let mut app=app("report-during-startup");app.busy=true;app.support=true;let task=app.update(Msg::ReportBug);
        assert!(app.bug_report.is_some(),"Help's enabled report action must work during startup");assert!(app.busy,"reporting must preserve the running operation");assert!(task.units()>0);app.cancel_bug_task();std::fs::remove_dir_all(app.root).unwrap();
    }

}
