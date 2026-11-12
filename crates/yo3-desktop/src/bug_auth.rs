//! Shared in-memory credentials and ordered OS-vault mutations for GitHub reporting.
use std::sync::{Arc,Mutex,OnceLock,atomic::{AtomicBool,Ordering}};
use std::time::Duration;
use yo3_desktop::{github_credentials::{self,StoredCredential},github_report::{Client,Failure,Identity,Session,Stage,Token}};

type Gate=Arc<tokio::sync::Mutex<()>>;
#[derive(Clone)]
pub(super) struct GitHubAccount {
    pub client_id:String,pub login:String,pub warning:Option<String>,
    token:Arc<Mutex<Token>>,active:Arc<AtomicBool>,
}
impl GitHubAccount {
    pub(super) fn new(client_id:String,token:Token,login:String)->Self {Self {client_id,login,warning:None,token:Arc::new(Mutex::new(token)),active:Arc::new(AtomicBool::new(true))}}
    pub(super) fn token(&self)->Token {self.token.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()}
    pub(super) fn revoke(&self) {self.active.store(false,Ordering::Release);}
    pub(super) fn same_session(&self,other:&Self)->bool {Arc::ptr_eq(&self.token,&other.token)}
    fn active(&self)->bool {self.active.load(Ordering::Acquire)}
    fn replace_token(&self,token:Token) {*self.token.lock().unwrap_or_else(std::sync::PoisonError::into_inner)=token;}
}
trait Vault:Send+Sync+'static {
    fn load(&self,id:&str)->anyhow::Result<Option<StoredCredential>>;
    fn save(&self,id:&str,value:&StoredCredential)->anyhow::Result<()>;
    fn clear(&self,id:&str)->anyhow::Result<()>;
}
struct NativeVault;
impl Vault for NativeVault {
    fn load(&self,id:&str)->anyhow::Result<Option<StoredCredential>> {github_credentials::load(id)}
    fn save(&self,id:&str,value:&StoredCredential)->anyhow::Result<()> {github_credentials::save(id,value)}
    fn clear(&self,id:&str)->anyhow::Result<()> {github_credentials::clear(id)}
}
trait Gateway:Sync {
    fn refresh(&self,id:&str,token:&Token)->impl std::future::Future<Output=Result<Token,Failure>>+Send;
    fn identity(&self,token:&Token)->impl std::future::Future<Output=Result<Identity,Failure>>+Send;
}
impl Gateway for Client {
    async fn refresh(&self,id:&str,token:&Token)->Result<Token,Failure> {self.refresh_token(id,token).await}
    async fn identity(&self,token:&Token)->Result<Identity,Failure> {self.identity(token).await}
}
fn vault_gate()->Gate {static GATE:OnceLock<Gate>=OnceLock::new();GATE.get_or_init(||Arc::new(tokio::sync::Mutex::new(()))).clone()}
fn refresh_gate()->Gate {static GATE:OnceLock<Gate>=OnceLock::new();GATE.get_or_init(||Arc::new(tokio::sync::Mutex::new(()))).clone()}
const VAULT_TIMEOUT:Duration=Duration::from_secs(8);

pub(super) async fn restore_account()->Result<Option<GitHubAccount>,String> {
    let Some(id)=yo3_desktop::github_report::configured_client_id() else{return Ok(None)};
    restore_with(id,Arc::new(NativeVault),vault_gate(),VAULT_TIMEOUT).await
}
pub(super) async fn authorize_account(client_id:&str,session:Session,cancel:&AtomicBool)->Result<GitHubAccount,String> {
    let client=Client::new().map_err(|failure|failure.to_string())?;
    let token=client.poll_auth(&session,cancel).await.map_err(|failure|failure.to_string())?;
    if cancel.load(Ordering::Acquire){return Err("GitHub sign-in was cancelled.".into())}
    let identity=client.identity(&token).await.map_err(|failure|failure.to_string())?;
    if cancel.load(Ordering::Acquire){return Err("GitHub sign-in was cancelled.".into())}
    Ok(GitHubAccount::new(client_id.into(),token,identity.login))
}
pub(super) async fn save_account(account:GitHubAccount)->GitHubAccount {save_with(account,Arc::new(NativeVault),vault_gate(),VAULT_TIMEOUT).await}
pub(super) async fn refresh_account(account:GitHubAccount)->Result<GitHubAccount,Failure> {
    let client=Client::new()?;
    refresh_with(account,&client,Arc::new(NativeVault),vault_gate(),refresh_gate(),VAULT_TIMEOUT).await
}
/// Revoke synchronously, even before the returned future is polled.
pub(super) fn sign_out(account:GitHubAccount)->impl std::future::Future<Output=Result<(),String>>+Send {
    account.revoke();sign_out_with(account,Arc::new(NativeVault),vault_gate(),VAULT_TIMEOUT)
}
async fn vault_call<T:Send+'static>(gate:Gate,timeout:Duration,operation:impl FnOnce()->anyhow::Result<T>+Send+'static)->Result<T,String> {
    tokio::time::timeout(timeout,async move {
        let guard=gate.lock_owned().await;
        tokio::task::spawn_blocking(move||{let _guard=guard;operation()}).await
            .map_err(|_|"The system credential store could not be opened".to_string())?
            .map_err(|_|"The system credential store operation failed".to_string())
    }).await.map_err(|_|"The system credential store did not respond".to_string())?
}
async fn restore_with<V:Vault>(id:String,vault:Arc<V>,gate:Gate,timeout:Duration)->Result<Option<GitHubAccount>,String> {
    let lookup=id.clone();let Some(saved)=vault_call(gate,timeout,move||vault.load(&lookup)).await? else{return Ok(None)};
    let token=Token::from_secrets(saved.access_token,saved.expires_at,saved.refresh_token,saved.refresh_expires_at).map_err(|failure|failure.to_string())?;
    Ok(Some(GitHubAccount::new(id,token,saved.login)))
}
async fn save_with<V:Vault>(mut account:GitHubAccount,vault:Arc<V>,gate:Gate,timeout:Duration)->GitHubAccount {
    let saved=account.clone();
    let result=vault_call(gate,timeout,move||{
        if !saved.active(){return Ok(())}
        // Read after acquiring the vault guard, so a queued save uses the latest rotated pair.
        let token=saved.token();
        let credential=StoredCredential{access_token:token.access_token().into(),refresh_token:token.refresh_token().map(str::to_owned),
            expires_at:token.expires_at(),refresh_expires_at:token.refresh_expires_at(),login:saved.login.clone()};
        let result=vault.save(&saved.client_id,&credential);
        // A blocking credential call cannot be interrupted. Clean it up before any later vault job runs.
        if !saved.active(){vault.clear(&saved.client_id)?;}
        result
    }).await;
    if result.is_err(){account.warning=Some("Signed in for this session. Secure storage is unavailable; you may need to sign in again next time.".into());}
    account
}
async fn sign_out_with<V:Vault>(account:GitHubAccount,vault:Arc<V>,gate:Gate,timeout:Duration)->Result<(),String> {
    account.revoke();vault_call(gate,timeout,move||vault.clear(&account.client_id)).await
}
fn revoked()->Failure {Failure{stage:Stage::Identity,message:"You signed out of GitHub. Sign in again to publish this report.".into(),uncertain:false,auth_expired:true}}
async fn refresh_with<V:Vault,G:Gateway>(mut account:GitHubAccount,gateway:&G,vault:Arc<V>,gate:Gate,refresh_gate:Gate,timeout:Duration)->Result<GitHubAccount,Failure> {
    {
        // Re-read the shared token under this asynchronous guard: overlapping dialogs must not reuse a rotated refresh token.
        let _refresh=refresh_gate.lock().await;
        if !account.active(){return Err(revoked())}
        let token=account.token();
        if token.expires_at().is_some_and(|expires|expires<=yo3_tunnel::diagnostics::now().saturating_add(60)) {
            let rotated=gateway.refresh(&account.client_id,&token).await?;
            if !account.active(){return Err(revoked())}
            account.replace_token(rotated);
            // The shared memory update precedes both persistence and identity validation; neither failure may discard it.
            account=save_with(account,vault,gate,timeout).await;
        }
    }
    if !account.active(){return Err(revoked())}
    let identity=gateway.identity(&account.token()).await?;
    if !account.active(){return Err(revoked())}
    if !identity.login.eq_ignore_ascii_case(&account.login){return Err(Failure{stage:Stage::Identity,message:"GitHub returned a different account. Sign out and sign in again.".into(),uncertain:false,auth_expired:true})}
    Ok(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Condvar,atomic::AtomicUsize};
    #[derive(Default)]
    struct FakeVault {value:Mutex<Option<StoredCredential>>,entered:AtomicUsize,clears:AtomicUsize,block:Mutex<bool>,released:Condvar,fail:AtomicBool}
    impl Vault for FakeVault {
        fn load(&self,_id:&str)->anyhow::Result<Option<StoredCredential>> {Ok(self.value.lock().unwrap().clone())}
        fn save(&self,_id:&str,value:&StoredCredential)->anyhow::Result<()> {
            self.entered.fetch_add(1,Ordering::SeqCst);let mut blocked=self.block.lock().unwrap();while *blocked {blocked=self.released.wait(blocked).unwrap();}
            if self.fail.load(Ordering::SeqCst){return Err(anyhow::anyhow!("private ghu_secret"))}*self.value.lock().unwrap()=Some(value.clone());Ok(())
        }
        fn clear(&self,_id:&str)->anyhow::Result<()> {self.clears.fetch_add(1,Ordering::SeqCst);*self.value.lock().unwrap()=None;Ok(())}
    }
    struct FakeGateway {refreshes:AtomicUsize,fail_identity:bool}
    impl Gateway for FakeGateway {
        async fn refresh(&self,_id:&str,_token:&Token)->Result<Token,Failure> {self.refreshes.fetch_add(1,Ordering::SeqCst);tokio::time::sleep(Duration::from_millis(5)).await;Ok(Token::from_secrets("ghu_rotated".into(),Some(u64::MAX),Some("ghr_rotated".into()),None).unwrap())}
        async fn identity(&self,token:&Token)->Result<Identity,Failure> {assert_eq!(token.access_token(),"ghu_rotated");if self.fail_identity {Err(Failure{stage:Stage::Identity,message:"Connection interrupted".into(),uncertain:false,auth_expired:false})}else{Ok(Identity{login:"testduelist".into(),url:"https://github.com/testduelist".into()})}}
    }
    fn gate()->Gate {Arc::new(tokio::sync::Mutex::new(()))}
    fn account()->GitHubAccount {GitHubAccount::new("test-client".into(),Token::from_secrets("ghu_old".into(),Some(1),Some("ghr_old".into()),None).unwrap(),"TestDuelist".into())}
    struct Unblock(Arc<FakeVault>);
    impl Drop for Unblock {fn drop(&mut self){release(&self.0);}}
    fn release(vault:&FakeVault) {*vault.block.lock().unwrap()=false;vault.released.notify_all();}
    async fn entered(vault:&FakeVault) {tokio::time::timeout(Duration::from_secs(1),async {while vault.entered.load(Ordering::SeqCst)==0 {tokio::time::sleep(Duration::from_millis(1)).await;}}).await.unwrap();}
    #[tokio::test]
    async fn rotated_credentials_survive_a_later_identity_failure_in_all_account_clones() {
        let account=account();let retained=account.clone();let vault=Arc::new(FakeVault::default());let gateway=FakeGateway{refreshes:AtomicUsize::new(0),fail_identity:true};
        let result=refresh_with(account,&gateway,vault.clone(),gate(),gate(),Duration::from_secs(1)).await;
        assert!(result.is_err());assert_eq!(retained.token().access_token(),"ghu_rotated");assert_eq!(retained.token().refresh_token(),Some("ghr_rotated"));assert_eq!(vault.value.lock().unwrap().as_ref().unwrap().refresh_token.as_deref(),Some("ghr_rotated"));
    }
    #[tokio::test]
    async fn a_timed_out_save_keeps_the_lock_and_revocation_cannot_leave_a_late_credential() {
        let account=account();let vault=Arc::new(FakeVault::default());let _unblock=Unblock(vault.clone());*vault.block.lock().unwrap()=true;let serial=gate();
        let worker=tokio::spawn(save_with(account.clone(),vault.clone(),serial.clone(),Duration::from_millis(20)));entered(&vault).await;
        let saved=worker.await.unwrap();assert!(saved.warning.is_some());account.revoke();let signout=tokio::spawn(sign_out_with(account.clone(),vault.clone(),serial,Duration::from_secs(1)));
        tokio::time::sleep(Duration::from_millis(10)).await;assert_eq!(vault.clears.load(Ordering::SeqCst),0,"clear must wait until the timed-out blocking save releases its guard");release(&vault);signout.await.unwrap().unwrap();assert!(vault.value.lock().unwrap().is_none());assert!(!account.active());
    }
    #[tokio::test]
    async fn restoration_only_loads_the_vault_and_does_not_refresh_or_save() {
        let vault=Arc::new(FakeVault::default());*vault.value.lock().unwrap()=Some(StoredCredential{access_token:"ghu_old".into(),expires_at:Some(1),refresh_token:Some("ghr_old".into()),refresh_expires_at:None,login:"TestDuelist".into()});
        let restored=restore_with("test-client".into(),vault.clone(),gate(),Duration::from_secs(1)).await.unwrap().unwrap();assert_eq!(restored.token().access_token(),"ghu_old");assert_eq!(vault.entered.load(Ordering::SeqCst),0);assert_eq!(vault.clears.load(Ordering::SeqCst),0);
    }
    #[test]
    fn cloned_accounts_share_a_session_but_new_sign_ins_do_not() {let account=account();assert!(account.same_session(&account.clone()));assert!(!account.same_session(&GitHubAccount::new("test-client".into(),account.token(),"TestDuelist".into())));}
    #[tokio::test]
    async fn concurrent_account_clones_only_rotate_the_refresh_token_once() {
        let account=account();let vault=Arc::new(FakeVault::default());let gateway=FakeGateway{refreshes:AtomicUsize::new(0),fail_identity:false};let serial=gate();let refresh=gate();
        let (first,second)=tokio::join!(refresh_with(account.clone(),&gateway,vault.clone(),serial.clone(),refresh.clone(),Duration::from_secs(1)),refresh_with(account.clone(),&gateway,vault,serial,refresh,Duration::from_secs(1)));
        assert!(first.is_ok() && second.is_ok());assert_eq!(gateway.refreshes.load(Ordering::SeqCst),1);assert_eq!(account.token().refresh_token(),Some("ghr_rotated"));
    }
    #[tokio::test]
    async fn a_failed_vault_does_not_discard_the_rotated_session_token() {
        let account=account();let vault=Arc::new(FakeVault::default());vault.fail.store(true,Ordering::SeqCst);let gateway=FakeGateway{refreshes:AtomicUsize::new(0),fail_identity:false};
        let saved=refresh_with(account.clone(),&gateway,vault,gate(),gate(),Duration::from_secs(1)).await.unwrap();assert!(saved.warning.is_some());assert_eq!(account.token().access_token(),"ghu_rotated");assert!(saved.same_session(&account));
    }
    #[tokio::test]
    async fn revoked_sessions_cannot_refresh_or_save_again() {
        let account=account();account.revoke();let vault=Arc::new(FakeVault::default());let gateway=FakeGateway{refreshes:AtomicUsize::new(0),fail_identity:false};
        assert!(refresh_with(account.clone(),&gateway,vault.clone(),gate(),gate(),Duration::from_secs(1)).await.is_err());let _=save_with(account,vault.clone(),gate(),Duration::from_secs(1)).await;assert_eq!(gateway.refreshes.load(Ordering::SeqCst),0);assert_eq!(vault.entered.load(Ordering::SeqCst),0);
    }

    #[test]
    fn sign_out_revokes_the_shared_session_before_its_future_is_polled() {let account=account();let operation=sign_out(account.clone());assert!(!account.active());drop(operation);}
    #[tokio::test]
    async fn revocation_during_a_save_clears_it_before_releasing_the_vault_guard() {
        let account=account();let vault=Arc::new(FakeVault::default());let _unblock=Unblock(vault.clone());*vault.block.lock().unwrap()=true;
        let worker=tokio::spawn(save_with(account.clone(),vault.clone(),gate(),Duration::from_secs(1)));entered(&vault).await;account.revoke();release(&vault);let _=worker.await.unwrap();assert!(vault.value.lock().unwrap().is_none());assert_eq!(vault.clears.load(Ordering::SeqCst),1);
    }

}
