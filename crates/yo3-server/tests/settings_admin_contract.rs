//! Settings and maintenance must be saved and audited before publication.
mod common;
use std::{path::PathBuf, sync::{Arc,Mutex}};
use yo3_server::{accounts::Directory, api::{Api,Op}, settings,store};
struct Fixture { dir:PathBuf, api:Api }
impl Fixture {
    fn new(name:&str)->Self {
        let dir=std::env::temp_dir().join(format!("yo3-settings-admin-{name}-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir); store::initialize(&dir).unwrap();
        let accounts=Arc::new(Mutex::new(Directory::open(vec![],&dir.join("accounts.json")).unwrap()));
        let live=Arc::new(settings::Live::open(Some(&dir.join("settings.json")),vec![]).unwrap());
        Self { api:Api::new(accounts,live,Some(dir.clone()),Default::default(),"server"),dir }
    }
    fn sql(&self,sql:&str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _=std::fs::remove_dir_all(&self.dir); } }
#[test]
fn settings_and_maintenance_do_not_publish_without_audit() {
    for (name,op) in [
        ("setting",Op::Set { name:"server-name".into(),value:"Changed".into() }),
        ("maintenance",Op::Maintenance { minutes:Some(5),message:Some("Changed".into()),off:false }),
    ] {
        let f=Fixture::new(name); let before=f.api.settings.rows();
        f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;");
        assert!(f.api.apply(&op,"host").is_err(),"{name} reported success without audit");
        assert_eq!(f.api.settings.rows(),before);
        assert_eq!(settings::Live::open(Some(&f.dir.join("settings.json")),vec![]).unwrap().rows(),before);
    }
}
#[test]
fn invalid_maintenance_time_cannot_save_its_message() {
    let f=Fixture::new("invalid"); let before=f.api.settings.rows();
    assert!(f.api.apply(&Op::Maintenance { minutes:Some(u32::MAX),message:Some("Changed".into()),off:false },"host").is_err());
    assert_eq!(f.api.settings.rows(),before);
}

use yo3_server::{api::{settings::SettingsRequest, Actor, MutationErrorCode}, logins::{Login,Role,Registration}};
const ID:&str="settings-change-00000001";
fn setting(name:&str,value:&str)->SettingsRequest { SettingsRequest::Set { name:name.into(),value:value.into() } }
#[test]
fn save_receipt_audit_and_commit_failures_leave_live_and_saved_state_unchanged() {
    for failure in ["save","receipt","audit","commit"] { for kind in ["set","reset","schedule","cancel"] {
        let f=Fixture::new(&format!("{failure}-{kind}"));
        f.api.settings.set("server-name","Before").unwrap();
        f.api.settings.set("maintenance-at",&(yo3_server::logins::now()+600).to_string()).unwrap();
        let before=f.api.settings.rows();
        match failure {
            "save"=>store::fail_domain(&f.dir,"settings.json",true).unwrap(),
            "receipt"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL,'no receipt'); END;"),
            "audit"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;"),
            _=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;"),
        }
        let request=match kind {
            "set"=>setting("server-name","After"), "reset"=>SettingsRequest::Reset { name:"server-name".into() },
            "schedule"=>SettingsRequest::ScheduleMaintenance { minutes:5,message:Some("After".into()) }, _=>SettingsRequest::CancelMaintenance,
        };
        assert_eq!(f.api.change_settings(&request,ID,Actor::Host { by:"host" },"server").unwrap_err().code,MutationErrorCode::PersistenceFailed);
        assert_eq!(f.api.settings.rows(),before,"{failure}-{kind}: memory changed");
        assert_eq!(settings::Live::open(Some(&f.dir.join("settings.json")),vec![]).unwrap().rows(),before,"{failure}-{kind}: disk changed");
        assert!(f.api.audit(100).unwrap().is_empty());
        store::fail_domain(&f.dir,"settings.json",false).unwrap(); if failure!="save" { f.sql("DROP TRIGGER refuse_change;"); }
        f.api.settings.set("headline","Unrelated successful save").unwrap();
        assert_eq!(f.api.settings.now().server_name,"Before");
        assert_eq!(settings::Live::open(Some(&f.dir.join("settings.json")),vec![]).unwrap().now().maintenance_at,f.api.settings.now().maintenance_at);
        f.api.change_settings(&request,ID,Actor::Host { by:"host" },"server").unwrap();
        assert_eq!(f.api.audit(100).unwrap().len(),1);
    } }
}
#[test]
fn retries_do_not_overwrite_later_settings_or_move_maintenance_and_survive_restart_and_audit_pruning() {
    let f=Fixture::new("retry"); let request=SettingsRequest::ScheduleMaintenance { minutes:5,message:Some("Saved together".into()) };
    let results=std::thread::scope(|s| {
        let jobs:Vec<_>=(0..8).map(|_|s.spawn(||f.api.change_settings(&request,ID,Actor::Host { by:"host" },"server").unwrap())).collect();
        jobs.into_iter().map(|j|j.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(results.iter().all(|r|r==&results[0])); assert_eq!(f.api.audit(100).unwrap().len(),1);
    let at=f.api.settings.now().maintenance_at.unwrap();
    assert_eq!(f.api.settings.now().maintenance_message,"Saved together");
    let cancel=f.api.change_settings(&SettingsRequest::CancelMaintenance,"settings-cancel-0001",Actor::Host { by:"host" },"server").unwrap();
    assert!(cancel.maintenance_at.is_none());
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';"); store::prune_audit(&f.dir,1).unwrap();
    store::close(&f.dir); store::initialize(&f.dir).unwrap();
    let live=Arc::new(settings::Live::open(Some(&f.dir.join("settings.json")),vec![]).unwrap());
    let api=Api::new(f.api.accounts.clone(),live,Some(f.dir.clone()),Default::default(),"offline");
    assert_eq!(api.change_settings(&request,ID,Actor::Host { by:"different display name" },"offline").unwrap(),results[0]);
    assert!(api.settings.now().maintenance_at.is_none(),"retry restored a cancelled countdown");
    assert!(results[0].maintenance_at.as_ref().unwrap().contains(&chrono::DateTime::from_timestamp(at,0).unwrap().format("%H:%M").to_string()));
    assert_eq!(api.change_settings(&setting("server-name","Conflict"),ID,Actor::Host { by:"host" },"offline").unwrap_err().code,MutationErrorCode::RequestConflict);
}
#[test]
fn independent_settings_requests_preserve_both_edits_and_replays_recheck_permissions() {
    let f=Fixture::new("concurrent");
    std::thread::scope(|s| { for (name,value,id) in [("server-name","New name","independent-setting-0001"),("headline","New headline","independent-setting-0002")] {
        let f=&f; s.spawn(move||f.api.change_settings(&setting(name,value),id,Actor::Host { by:"host" },"server").unwrap());
    } });
    assert_eq!(f.api.settings.now().server_name,"New name"); assert_eq!(f.api.settings.now().headline,"New headline");
    let admin=f.api.accounts.lock().unwrap().register(Login::new("Admin","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();
    f.api.accounts.lock().unwrap().set_role("Admin",Role::Admin).unwrap();
    let actor=Actor::Staff { user_id:admin }; let request=setting("headline","Staff edit");
    f.api.change_settings(&request,ID,actor,"dashboard").unwrap();
    f.api.accounts.lock().unwrap().set_role("Admin",Role::Moderator).unwrap();
    assert_eq!(f.api.change_settings(&request,ID,actor,"dashboard").unwrap_err().code,MutationErrorCode::Forbidden);
    let list=f.api.live_settings(actor).unwrap(); assert!(list.settings.iter().any(|s|s.name=="headline" && s.value=="Staff edit" && s.overridden));
}

#[test]
fn failed_changes_push_nothing_and_retries_never_repeat_countdowns_or_cancellation_chat() {
    let mut f=Fixture::new("push");
    f.api.accounts.lock().unwrap().add_account(&["Yugi".into()]).unwrap();
    let user=f.api.accounts.lock().unwrap().user_of("Yugi").unwrap();
    let server=common::Server::new(); server.hub.lock().unwrap().set_directory(f.api.accounts.clone());
    let mut client=server.join_saved(&f.api.accounts,user); client.pushed(); f.api.hub=Some(server.hub);
    let request=SettingsRequest::ScheduleMaintenance { minutes:5,message:Some("Back soon".into()) };
    f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;");
    assert!(f.api.change_settings(&request,ID,Actor::Host { by:"host" },"server").is_err()); assert!(client.pushed().is_empty());
    f.sql("DROP TRIGGER refuse_change;");
    f.api.change_settings(&request,ID,Actor::Host { by:"host" },"server").unwrap();
    assert_eq!(client.pushed().iter().filter(|m|m.request()==Some(0x21FC)).count(),1);
    f.api.change_settings(&request,ID,Actor::Host { by:"host" },"server").unwrap(); assert!(client.pushed().is_empty());
    f.api.change_settings(&SettingsRequest::CancelMaintenance,"cancel-with-chat-0001",Actor::Host { by:"host" },"server").unwrap();
    let pushed=client.pushed(); assert_eq!(pushed.len(),2); assert!(pushed.iter().any(|m|m.request()==Some(0x21FC)));
    f.api.change_settings(&SettingsRequest::CancelMaintenance,"cancel-with-chat-0001",Actor::Host { by:"host" },"server").unwrap(); assert!(client.pushed().is_empty());
}
