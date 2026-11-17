//! Moderation must commit state and audit before revoking devices or live sessions.
use std::{path::PathBuf,sync::{Arc,Mutex},time::Instant};
use yo3_server::{accounts::Directory,api::{Api,Op},auth::Sessions,logins::{Login,Registration},settings,store};
struct Fixture { dir:PathBuf, api:Api,user:u32 }
impl Fixture {
    fn new(name:&str)->Self {
        let dir=std::env::temp_dir().join(format!("yo3-moderation-admin-{name}-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir); store::initialize(&dir).unwrap();
        let mut d=Directory::open(vec![],&dir.join("accounts.json")).unwrap();
        let user=d.register(Login::new("Yugi","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();
        let mut api=Api::new(Arc::new(Mutex::new(d)),Arc::new(settings::Live::open(None,vec![]).unwrap()),Some(dir.clone()),Default::default(),"server");
        let mut sessions=Sessions::default(); sessions.login(user,Instant::now()).unwrap(); api.sessions=Some(Arc::new(Mutex::new(sessions)));
        Self { dir,api,user }
    }
    fn sql(&self,sql:&str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _=std::fs::remove_dir_all(&self.dir); } }
fn op(kind:&str)->Op { match kind {
    "ban"=>Op::Ban {who:"Yugi".into(),reason:"Spam".into(),days:Some(1)},
    "restrict"=>Op::Restrict {who:"Yugi".into(),reason:"Spam".into(),days:Some(1)},
    "mute"=>Op::Mute {who:"Yugi".into(),reason:"Spam".into(),minutes:Some(30)},
    "lift"=>Op::Lift {who:"Yugi".into()}, "unmute"=>Op::Unmute {who:"Yugi".into()},
    "kick"=>Op::Kick {who:"Yugi".into()}, _=>Op::SignOut {who:"Yugi".into()},
} }
#[test]
fn no_moderation_reports_success_changes_accounts_or_disconnects_without_audit() {
    for kind in ["ban","restrict","mute","lift","unmute","kick","sign-out"] {
        let f=Fixture::new(kind); let before=f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().clone();
        f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;");
        assert!(f.api.apply(&op(kind),"host").is_err(),"{kind} reported success without audit");
        let d=f.api.accounts.lock().unwrap(); let after=d.login_by_id(f.user).unwrap();
        assert_eq!(serde_json::to_value(after).unwrap(),serde_json::to_value(&before).unwrap());
        assert!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user));
        let disk=Directory::open(vec![],&f.dir.join("accounts.json")).unwrap();
        assert_eq!(serde_json::to_value(disk.login_by_id(f.user)).unwrap(),serde_json::to_value(Some(&before)).unwrap());
    }
}

use yo3_server::{api::{moderation::ModerationRequest,Actor,MutationErrorCode},logins::{Device,Role,Standing,Mute},store::EffectState};
const ID:&str="moderation-change-0001";
fn request(user:u32,kind:&str)->ModerationRequest {match kind {
    "ban"=>ModerationRequest::Ban{user_id:user,reason:"Spam".into(),days:Some(1)},
    "restrict"=>ModerationRequest::Restrict{user_id:user,reason:"Spam".into(),days:Some(1)},
    "mute"=>ModerationRequest::Mute{user_id:user,reason:"Spam".into(),minutes:Some(30)},
    "lift"=>ModerationRequest::Lift{user_id:user},"unmute"=>ModerationRequest::Unmute{user_id:user},
    "kick"=>ModerationRequest::Kick{user_id:user},_=>ModerationRequest::SignOut{user_id:user},
} }
#[test]
fn save_receipt_intent_audit_and_commit_failures_preserve_accounts_devices_and_sessions() {
    for failure in ["save","receipt","intent","audit","commit"] { for kind in ["ban","restrict","mute","lift","unmute","sign-out","kick"] {
        if failure=="intent" && !matches!(kind,"ban"|"sign-out"|"kick") || failure=="save" && kind=="kick" {continue}
        let f=Fixture::new(&format!("{failure}-{kind}"));
        {let mut d=f.api.accounts.lock().unwrap();
            d.add_device(f.user,Device{token:"DEVICE-DIGEST".into(),label:"Desktop".into(),created:1,last_used:1,expires:i64::MAX}).unwrap();
            d.set_mute(f.user,Some(Mute{reason:"Before".into(),until:None})).unwrap();
            d.set_standing_of(f.user,Standing::Restricted{reason:"Before".into(),until:Some(1)}).unwrap();
        }
        let before=f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().clone();
        match failure {
            "save"=>store::fail_domain(&f.dir,"accounts.json",true).unwrap(),
            "receipt"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL,'no receipt'); END;"),
            "intent"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_effects BEGIN SELECT RAISE(FAIL,'no intent'); END;"),
            "audit"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;"),
            _=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;"),
        }
        assert_eq!(f.api.moderate(&request(f.user,kind),ID,Actor::Host{by:"host"},"server").unwrap_err().code,MutationErrorCode::PersistenceFailed,"{failure}-{kind}");
        assert_eq!(f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap(),&before);
        assert_eq!(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap().login_by_id(f.user).unwrap(),&before);
        assert!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user)); assert!(f.api.audit(100).unwrap().is_empty());
        store::fail_domain(&f.dir,"accounts.json",false).unwrap();if failure!="save" {f.sql("DROP TRIGGER refuse_change;");}
        f.api.accounts.lock().unwrap().add_player_to(f.user,"Unrelated").unwrap();
        assert_eq!(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap().login_by_id(f.user).unwrap(),&before,"failed state leaked into the next save");
        f.api.moderate(&request(f.user,kind),ID,Actor::Host{by:"host"},"server").unwrap();
        assert_eq!(f.api.audit(100).unwrap().len(),1);
    } }
}
#[test]
fn concurrent_duplicates_and_retries_after_reconnect_never_repeat_disconnects_or_old_state() {
    for kind in ["ban","restrict","mute","lift","unmute","sign-out","kick"] {
        let f=Fixture::new(&format!("retry-{kind}"));let request=request(f.user,kind);
        let replies=std::thread::scope(|s| {
            let jobs:Vec<_>=(0..8).map(|_|s.spawn(||f.api.moderate(&request,ID,Actor::Host{by:"host"},"server").unwrap())).collect();
            jobs.into_iter().map(|j|j.join().unwrap()).collect::<Vec<_>>()
        });
        assert!(replies.iter().all(|r|r.result==replies[0].result));assert_eq!(f.api.audit(100).unwrap().len(),1);
        let disconnected=matches!(kind,"ban"|"sign-out"|"kick");
        assert_eq!(replies[0].disconnect,disconnected.then_some(EffectState::Completed));
        assert_eq!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user),!disconnected);
        f.api.accounts.lock().unwrap().set_standing_of(f.user,Standing::Active).unwrap();f.api.accounts.lock().unwrap().set_mute(f.user,None).unwrap();
        f.api.sessions.as_ref().unwrap().lock().unwrap().login(f.user,Instant::now()).unwrap();
        f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");store::prune_audit(&f.dir,1).unwrap();store::close(&f.dir);store::initialize(&f.dir).unwrap();
        assert_eq!(f.api.moderate(&request,ID,Actor::Host{by:"renamed host"},"offline").unwrap().result,replies[0].result);
        assert!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user));
        let d=f.api.accounts.lock().unwrap();let login=d.login_by_id(f.user).unwrap();assert_eq!(login.standing,Standing::Active);assert!(login.muted.is_none());
        let raw:String=rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT result FROM operation_receipts",[],|r|r.get(0)).unwrap();
        assert!(!raw.contains("Yugi")&&!raw.contains("Spam")&&!raw.contains("HASH"));
    }
}
#[test]
fn outcome_write_failure_and_restart_report_uncertainty_without_repeating_the_disconnect() {
    for failure in ["state","audit","commit"] {
    let f=Fixture::new(&format!("unknown-{failure}"));let request=request(f.user,"kick");
    match failure {
        "audit"=>f.sql("CREATE TRIGGER refuse_outcome BEFORE UPDATE ON audit BEGIN SELECT RAISE(FAIL,'no audit outcome'); END;"),
        "commit"=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_outcome AFTER UPDATE ON operation_effects BEGIN INSERT INTO child VALUES(-1); END;"),
        _=>f.sql("CREATE TRIGGER refuse_outcome BEFORE UPDATE ON operation_effects BEGIN SELECT RAISE(FAIL,'no outcome'); END;"),
    }
    let first=f.api.moderate(&request,ID,Actor::Host{by:"host"},"server").unwrap();
    assert_eq!(first.disconnect,Some(EffectState::Pending));assert!(!f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user));
    assert_eq!(f.api.audit(100).unwrap()[0].detail.as_ref().unwrap()["disconnect"],"pending");
    f.api.sessions.as_ref().unwrap().lock().unwrap().login(f.user,Instant::now()).unwrap();
    assert_eq!(f.api.moderate(&request,ID,Actor::Host{by:"host"},"server").unwrap().disconnect,Some(EffectState::Pending));
    f.sql("DROP TRIGGER refuse_outcome;");store::close(&f.dir);store::initialize(&f.dir).unwrap();assert_eq!(store::interrupt_effects(&f.dir).unwrap(),1);
    let replay=f.api.moderate(&request,ID,Actor::Host{by:"host"},"server").unwrap();assert_eq!(replay.result,first.result);assert_eq!(replay.disconnect,Some(EffectState::Interrupted));
    assert!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user));
    assert_eq!(f.api.audit(100).unwrap()[0].detail.as_ref().unwrap()["disconnect"],"interrupted");assert_eq!(store::interrupt_effects(&f.dir).unwrap(),0);
    }
}
#[test]
fn current_permissions_target_roles_and_durations_are_checked_before_any_change_or_replay() {
    let f=Fixture::new("permissions");
    let mut d=f.api.accounts.lock().unwrap();let moderator=d.register(Login::new("Croquet","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();
    d.set_role("Croquet",Role::Moderator).unwrap();let admin=d.register(Login::new("Pegasus","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();d.set_role("Pegasus",Role::Admin).unwrap();drop(d);
    let actor=Actor::Staff{user_id:moderator};
    assert_eq!(f.api.moderate(&request(admin,"kick"),ID,actor,"dashboard").unwrap_err().code,MutationErrorCode::Forbidden);
    let original=request(f.user,"mute");f.api.moderate(&original,ID,actor,"dashboard").unwrap();
    f.api.accounts.lock().unwrap().set_role("Yugi",Role::Admin).unwrap();
    assert_eq!(f.api.moderate(&original,ID,actor,"dashboard").unwrap_err().code,MutationErrorCode::Forbidden);
    f.api.accounts.lock().unwrap().set_role("Croquet",Role::Player).unwrap();
    assert_eq!(f.api.moderate(&original,ID,actor,"dashboard").unwrap_err().code,MutationErrorCode::Forbidden);
    for days in [Some(0),Some(u32::MAX)] {
        assert_eq!(f.api.moderate(&ModerationRequest::Ban{user_id:f.user,reason:"Spam".into(),days},"invalid-duration-0001",Actor::Host{by:"host"},"server").unwrap_err().code,MutationErrorCode::InvalidRequest);
    }
    assert_eq!(f.api.moderate(&request(f.user,"kick"),ID,Actor::Staff{user_id:admin},"dashboard").unwrap().disconnect,Some(EffectState::Completed),"IDs are scoped per staff account");
}
#[test]
fn schema_three_migrates_without_losing_accounts_audit_or_receipts() {
    let f=Fixture::new("schema3");let original=f.api.moderate(&request(f.user,"mute"),ID,Actor::Host{by:"host"},"server").unwrap();
    f.sql("DROP TABLE operation_effects;PRAGMA user_version=3;");store::close(&f.dir);store::initialize(&f.dir).unwrap();
    let conn=rusqlite::Connection::open(f.dir.join("server.db")).unwrap();assert_eq!(conn.pragma_query_value(None,"user_version",|r|r.get::<_,u32>(0)).unwrap(),6);
    assert_eq!(f.api.moderate(&request(f.user,"mute"),ID,Actor::Host{by:"host"},"server").unwrap().result,original.result);assert_eq!(f.api.audit(100).unwrap().len(),1);
    assert!(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap().login_by_id(f.user).unwrap().muted.is_some());
}

#[tokio::test]
async fn real_startup_marks_unfinished_effects_interrupted_and_retry_does_not_end_a_new_session() {
    let f=Fixture::new("startup");
    let key=f.api.apply(&Op::Key{who:"Yugi".into()},"host").unwrap()["key"].as_str().unwrap().to_string();
    f.sql("CREATE TRIGGER refuse_outcome BEFORE UPDATE ON operation_effects BEGIN SELECT RAISE(FAIL,'no outcome'); END;");
    let first=f.api.apply_request(&op("kick"),"host","server",None,ID).unwrap();assert_eq!(first["disconnect"],"pending");
    f.sql("DROP TRIGGER refuse_outcome;");
    // A real restart resets process health; this same-process test clears the injected storage failure with a save.
    f.api.accounts.lock().unwrap().add_player_to(f.user,"Yugi").unwrap();
    let mut cfg=yo3_server::Config::ephemeral(f.dir.clone());cfg.data_dir=Some(f.dir.clone());cfg.game_dir=f.dir.join("no-game-files");
    let cert=cfg.tls.certificate.clone();let server=yo3_server::start(cfg).await.unwrap();
    let endpoint=yo3_tunnel::Endpoint::new(server.auth.to_string(),"localhost".into(),&cert).unwrap();
    let root=endpoint.login(&key).await.unwrap();
    let admin=yo3_server::admin::running(&f.dir).unwrap();
    let retry=tokio::task::spawn_blocking(move||yo3_server::api::call_as_request(&admin,&op("kick"),"another host label",ID)).await.unwrap().unwrap();
    assert_eq!(retry["disconnect"],"interrupted");assert_eq!(retry["request_id"],first["request_id"]);
    assert!(endpoint.ticket(&root,yo3_tunnel::Role::Front).await.is_ok(),"old kick receipt disconnected a new game session");
    assert_eq!(f.api.audit(100).unwrap().iter().find(|r|r.op=="kick").unwrap().detail.as_ref().unwrap()["disconnect"],"interrupted");
    server.stopper().send_replace(true);server.wait().await;
}

#[test]
fn independent_moderation_requests_preserve_both_changes() {
    let f=Fixture::new("independent");
    std::thread::scope(|s| {for(kind,id)in[("ban","independent-ban-00001"),("mute","independent-mute-0001")] {
        let f=&f;s.spawn(move||f.api.moderate(&request(f.user,kind),id,Actor::Host{by:"host"},"server").unwrap());
    }});
    let d=f.api.accounts.lock().unwrap();let login=d.login_by_id(f.user).unwrap();
    assert!(matches!(login.standing,Standing::Banned{..}));assert!(login.muted.is_some());assert_eq!(f.api.audit(100).unwrap().len(),2);
}
