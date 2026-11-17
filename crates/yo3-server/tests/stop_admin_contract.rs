//! A stop must be audited before signalling, and an old request cannot stop a different server run.
use std::{path::PathBuf,sync::{Arc,Mutex}};
use yo3_server::{accounts::Directory,api::{Api,Op},settings,store};
struct Fixture {dir:PathBuf,api:Api}
impl Fixture {
    fn new(name:&str)->Self {
        let dir=std::env::temp_dir().join(format!("yo3-stop-admin-{name}-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir);store::initialize(&dir).unwrap();
        let d=Directory::open(vec![],&dir.join("accounts.json")).unwrap();
        let mut api=Api::new(Arc::new(Mutex::new(d)),Arc::new(settings::Live::open(None,vec![]).unwrap()),Some(dir.clone()),Default::default(),"server");
        api.shutdown=Some(tokio::sync::watch::channel(false).0);Self{dir,api}
    }
    fn sql(&self,sql:&str){rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap();}
}
impl Drop for Fixture{fn drop(&mut self){store::close(&self.dir);let _=std::fs::remove_dir_all(&self.dir);}}
#[test]
fn a_failed_audit_must_not_stop_the_server_or_report_success() {
    let f=Fixture::new("audit-red");
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit');END;");
    assert!(f.api.apply(&Op::Shutdown{run_id:f.api.run_id().into()},"host").is_err(),"stop falsely reports success when audit fails");
    assert!(!*f.api.shutdown.as_ref().unwrap().borrow(),"a failed operation stopped the server");
}

use yo3_server::{api::{Actor,MutationErrorCode,server::StopRequest},logins::{Login,Registration,Role}};
const ID:&str="stop-this-server-0001";
fn request(api:&Api)->StopRequest{StopRequest{run_id:api.run_id().into()}}
#[test]
fn audit_receipt_intent_and_commit_failures_never_publish_a_stop() {
    for failure in ["audit","receipt","intent","commit"] {
        let f=Fixture::new(failure);
        match failure {
            "audit"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;"),
            "receipt"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL,'no receipt'); END;"),
            "intent"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_effects BEGIN SELECT RAISE(FAIL,'no intent'); END;"),
            _=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY);CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1);END;"),
        }
        assert!(f.api.request_stop(&request(&f.api),ID,Actor::Host{by:"host"},"server").is_err());
        assert!(!*f.api.shutdown.as_ref().unwrap().borrow());assert!(f.api.audit(10).unwrap().is_empty());
        f.sql("DROP TRIGGER refuse_change;");
        let done=f.api.request_stop(&request(&f.api),ID,Actor::Host{by:"host"},"server").unwrap();
        assert_eq!(done.stop,store::EffectState::Completed);assert!(*f.api.shutdown.as_ref().unwrap().borrow());
    }
}
#[test]
fn old_requests_and_lost_response_retries_cannot_stop_a_new_process() {
    let f=Fixture::new("restart");let old=request(&f.api);
    let replies=std::thread::scope(|s|{let jobs:Vec<_>=(0..8).map(|_|s.spawn(||f.api.request_stop(&old,ID,Actor::Host{by:"host"},"server").unwrap())).collect();jobs.into_iter().map(|j|j.join().unwrap()).collect::<Vec<_>>()});
    assert!(replies.iter().all(|r|r.result==replies[0].result));assert_eq!(f.api.audit(100).unwrap().len(),1);
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");store::prune_audit(&f.dir,1).unwrap();store::close(&f.dir);store::initialize(&f.dir).unwrap();
    let mut next=Api::new(Arc::new(Mutex::new(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap())),f.api.settings.clone(),Some(f.dir.clone()),Default::default(),"server");next.shutdown=Some(tokio::sync::watch::channel(false).0);
    assert_ne!(next.run_id(),f.api.run_id());
    assert_eq!(next.request_stop(&old,ID,Actor::Host{by:"changed host label"},"server").unwrap().result,replies[0].result);
    assert!(!*next.shutdown.as_ref().unwrap().borrow(),"receipt replay cannot signal the next process");
    assert_eq!(next.request_stop(&old,"never-committed-old-request",Actor::Host{by:"host"},"server").unwrap_err().code,MutationErrorCode::RequestConflict);
    assert!(!*next.shutdown.as_ref().unwrap().borrow(),"even an uncommitted request is bound to its old run");
    assert!(next.request_stop(&request(&next),ID,Actor::Host{by:"host"},"server").is_err(),"changing input with an old ID conflicts");
}
#[test]
fn unfinished_stop_intents_become_interrupted_without_signalling_and_replay_rechecks_staff() {
    let f=Fixture::new("outcome");let staff={let mut d=f.api.accounts.lock().unwrap();let id=d.register(Login::new("Admin","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();d.set_role("Admin",Role::Admin).unwrap();id};
    f.sql("CREATE TRIGGER refuse_change BEFORE UPDATE ON operation_effects BEGIN SELECT RAISE(FAIL,'outcome lost');END;");
    let original=request(&f.api);let done=f.api.request_stop(&original,ID,Actor::Staff{user_id:staff},"dashboard").unwrap();assert_eq!(done.stop,store::EffectState::Pending);
    f.sql("DROP TRIGGER refuse_change;");f.api.shutdown.as_ref().unwrap().send_replace(false);
    assert_eq!(store::interrupt_effects(&f.dir).unwrap(),1);assert!(!*f.api.shutdown.as_ref().unwrap().borrow());
    let retry=f.api.request_stop(&original,ID,Actor::Staff{user_id:staff},"dashboard").unwrap();assert_eq!(retry.stop,store::EffectState::Interrupted);
    assert_eq!(f.api.audit(100).unwrap()[0].detail.as_ref().unwrap()["stop"],"interrupted");
    f.api.accounts.lock().unwrap().set_role("Admin",Role::Moderator).unwrap();
    assert_eq!(f.api.request_stop(&original,ID,Actor::Staff{user_id:staff},"dashboard").unwrap_err().code,MutationErrorCode::Forbidden);
}
#[test]
fn schema_four_disconnect_history_is_preserved_when_stop_intents_are_added() {
    let f=Fixture::new("schema4");
    // A real version-4 table, including a receipt and a pending disconnect. Migration must preserve it verbatim.
    f.sql("DROP TABLE operation_effects;CREATE TABLE operation_effects(actor TEXT NOT NULL,request_id TEXT NOT NULL,user_id INTEGER NOT NULL,state TEXT NOT NULL CHECK(state IN('pending','completed','interrupted')),created INTEGER NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(actor,request_id),FOREIGN KEY(actor,request_id) REFERENCES operation_receipts(actor,request_id)) WITHOUT ROWID;
        INSERT INTO operation_receipts VALUES('host','old-disconnect-request','kick','{}','{}',1);
        INSERT INTO operation_effects VALUES('host','old-disconnect-request',1001,'pending',1,1);PRAGMA user_version=4;");
    store::close(&f.dir);store::initialize(&f.dir).unwrap();
    let conn=rusqlite::Connection::open(f.dir.join("server.db")).unwrap();
    assert_eq!(conn.query_row("SELECT kind FROM operation_effects",[],|r|r.get::<_,String>(0)).unwrap(),"disconnect");
    assert_eq!(store::interrupt_effects(&f.dir).unwrap(),1);
    assert_eq!(conn.query_row("SELECT state FROM operation_effects",[],|r|r.get::<_,String>(0)).unwrap(),"interrupted");
    assert!(f.api.request_stop(&request(&f.api),ID,Actor::Host{by:"host"},"server").is_ok());
    assert_eq!(conn.query_row("SELECT COUNT(*) FROM operation_effects",[],|r|r.get::<_,u32>(0)).unwrap(),2);
}
