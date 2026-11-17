//! Offline, one-time administrator provisioning for unattended deployments. Never an HTTP capability.
use std::path::Path;
use serde_json::json;
use crate::{accounts::Directory,logins::{self,Login,Role},store::{self,AuditRow,Receipt}};
const RECEIPT:&str="bootstrap-first-admin-v1";
pub async fn admin(dir:&Path,user:&str,password:String)->anyhow::Result<bool>{
    let _exclusive=store::lock(dir)?;
    store::initialize(dir)?;
    let d=Directory::open(vec![],&dir.join("accounts.json"))?;
    let input=json!({"user_digest":logins::digest(&user.to_ascii_lowercase())}).to_string();
    if let Some(receipt)=store::receipt(dir,"host",RECEIPT)? {
        let id=receipt.result["user_id"].as_u64().and_then(|n|u32::try_from(n).ok());
        anyhow::ensure!(receipt.operation=="bootstrap-admin" && receipt.input==input
            && id.and_then(|id|d.login_by_id(id)).is_some_and(|l|l.user.eq_ignore_ascii_case(user)&&l.role==Role::Admin),
            "Initial administrator provisioning is already closed. Use the local account commands for recovery; bootstrap never resets or re-promotes an account");
        return Ok(false)
    }
    anyhow::ensure!(d.entries().next().is_none(),"Bootstrap requires an empty account directory. Use the existing administrator or local account commands");
    anyhow::ensure!(logins::valid_user_id(user),"Admin User ID must be 3–16 letters or digits");
    anyhow::ensure!(logins::password_problem(user,&password).is_none(),"Admin password must be 8–16 letters or digits, uncommon and different from the User ID");
    let hash=logins::hash_password(password).await?;
    let (next,id)=d.prepare_password_account(user,logins::digest(&logins::new_token()),logins::now()).map_err(anyhow::Error::msg)?;
    let mut login=Login::new(user,hash,logins::digest(&logins::new_token()));login.role=Role::Admin;
    let mut next=next.prepare_login(id,|l|{*l=login;Ok(())}).map_err(anyhow::Error::msg)?;
    let row=AuditRow{at:chrono::Utc::now().to_rfc3339(),actor:"host".into(),origin:"offline-bootstrap".into(),op:"bootstrap-admin".into(),target:id.to_string(),before:None,after:Some(json!({"role":"admin"})),detail:None};
    let receipt=Receipt{operation:"bootstrap-admin".into(),input,result:json!({"user_id":id})};
    store::together(dir,||{next.persist()?;store::audit(dir,&row)?;store::put_receipt(dir,"host",RECEIPT,&receipt)?;Ok(())})?;
    Ok(true)
}
