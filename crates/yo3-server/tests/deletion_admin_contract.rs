//! Deletion must be all-or-nothing, including related domains and removal of old audit rows.
use std::{path::PathBuf,sync::{Arc,Mutex},time::Instant};
use yo3_server::{accounts::Directory,api::{Api,Op},auth::Sessions,front::Hub,logins::{Login,Registration},settings,store};
struct Fixture { dir:PathBuf,api:Api,user:u32 }
impl Fixture {
    fn new(name:&str)->Self {
        let dir=std::env::temp_dir().join(format!("yo3-deletion-admin-{name}-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir);store::initialize(&dir).unwrap();
        let mut d=Directory::open(vec![],&dir.join("accounts.json")).unwrap();
        let user=d.register(Login::new("Yugi","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();
        d.add_player_to(user,"Yugi").unwrap();
        let mut hub=Hub::with_records(&dir.join("records.json"));
        hub.keep_social(&dir.join("friends.json"));hub.keep_mail(&dir.join("mail.json"));
        hub.keep_teams(&dir.join("teams.json"));hub.keep_events(&dir.join("events.json"));hub.keep_rentals(&dir.join("rentals.json"));
        let mut api=Api::new(Arc::new(Mutex::new(d)),Arc::new(settings::Live::open(None,vec![]).unwrap()),Some(dir.clone()),Default::default(),"server");
        api.hub=Some(Arc::new(Mutex::new(hub)));
        let mut sessions=Sessions::default();sessions.login(user,Instant::now()).unwrap();api.sessions=Some(Arc::new(Mutex::new(sessions)));
        Self{dir,api,user}
    }
    fn sql(&self,sql:&str) {rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap();}
}
impl Drop for Fixture {fn drop(&mut self){store::close(&self.dir);let _=std::fs::remove_dir_all(&self.dir);}}
#[test]
fn deletion_does_not_report_success_or_remove_an_account_without_its_audit_row() {
    let f=Fixture::new("audit-red");
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;");
    assert!(f.api.apply(&Op::DeleteAccount{who:"Yugi".into()},"host").is_err(),"deletion reported success without audit");
    assert!(f.api.accounts.lock().unwrap().account_of(f.user).is_some());
    assert!(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap().account_of(f.user).is_some());
    assert!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user));
}

use yo3_server::{api::{Actor,MutationErrorCode,deletion::DeleteAccountRequest},logins::Role};
const ID:&str="delete-account-request-0001";
fn request(user_id:u32)->DeleteAccountRequest {DeleteAccountRequest{user_id,confirm_user_id:user_id}}
#[test]
fn every_domain_cleanup_audit_receipt_intent_and_commit_failure_rolls_back() {
    for failure in ["accounts.json","friends.json","mail.json","teams.json","rentals.json","records.json","events.json","duel_logs","audit_prune","audit","receipt","intent","commit"] {
        let f=Fixture::new(failure);
        f.api.apply(&Op::GiveBp{who:"Yugi".into(),amount:25},"host").unwrap();
        let player=f.api.accounts.lock().unwrap().account_of(f.user).unwrap().players[0].player_id;
        f.sql(&format!("INSERT INTO duel_logs VALUES('example-{player}.dlog',X'0102',1);"));
        let before:Vec<_>=["accounts.json","friends.json","mail.json","teams.json","rentals.json","records.json","events.json"].into_iter()
            .map(|name|(name,store::read(&f.dir.join(name)).unwrap())).collect();
        let audit=f.api.audit(100).unwrap();
        match failure {
            name if name.ends_with(".json")=>store::fail_domain(&f.dir,name,true).unwrap(),
            "audit_prune"=>f.sql("CREATE TRIGGER refuse_change BEFORE DELETE ON audit BEGIN SELECT RAISE(FAIL,'no prune'); END;"),
            "duel_logs"=>f.sql("CREATE TRIGGER refuse_change BEFORE DELETE ON duel_logs BEGIN SELECT RAISE(FAIL,'no deletion'); END;"),
            "audit"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;"),
            "receipt"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL,'no receipt'); END;"),
            "intent"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_effects BEGIN SELECT RAISE(FAIL,'no intent'); END;"),
            _=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;"),
        }
        let result=f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server");
        assert!(result.is_err(),"{failure}: deletion unexpectedly succeeded");
        assert!(f.api.accounts.lock().unwrap().account_of(f.user).is_some(),"{failure}");
        assert!(f.api.sessions.as_ref().unwrap().lock().unwrap().signed_in(f.user),"{failure}: premature disconnect");
        assert_eq!(f.api.audit(100).unwrap(),audit,"{failure}: audit changed before commit");
        for (name,value) in &before {assert_eq!(store::read(&f.dir.join(name)).unwrap(),*value,"{failure}: {name} changed");}
        let conn=rusqlite::Connection::open(f.dir.join("server.db")).unwrap();
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM duel_logs",[],|r|r.get::<_,u32>(0)).unwrap(),1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM operation_receipts WHERE request_id=?1",[ID],|r|r.get::<_,u32>(0)).unwrap(),0);
        if failure.ends_with(".json") {store::fail_domain(&f.dir,failure,false).unwrap();}else{f.sql("DROP TRIGGER refuse_change;");}
        // A later save must not inherit a failed prepared copy or poisoned incremental-save bookkeeping.
        f.api.accounts.lock().unwrap().add_player_to(f.user,"Unrelated").unwrap();
        assert_eq!(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap().account_of(f.user).unwrap().players.len(),2);
        f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server").unwrap();
        assert!(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap().account_of(f.user).is_none());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM duel_logs",[],|r|r.get::<_,u32>(0)).unwrap(),0);
    }
}
#[test]
fn duplicates_restart_audit_pruning_and_name_reuse_never_delete_a_new_account() {
    let f=Fixture::new("retries");let op=Op::DeleteAccount{who:"Yugi".into()};
    let replies=std::thread::scope(|s|{
        let jobs:Vec<_>=(0..8).map(|_|s.spawn(||f.api.apply_request(&op,"host","server",None,ID).unwrap())).collect();
        jobs.into_iter().map(|j|j.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(replies.iter().all(|r|r["user_id"]==f.user && r["players"]==1));
    assert_eq!(f.api.audit(100).unwrap().len(),1);
    let new=f.api.accounts.lock().unwrap().register(Login::new("Yugi","NEW HASH".into(),"NEW RECOVERY".into()),None,Registration::Open).unwrap();
    assert_ne!(new,f.user);f.api.sessions.as_ref().unwrap().lock().unwrap().login(new,Instant::now()).unwrap();
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");store::prune_audit(&f.dir,1).unwrap();store::close(&f.dir);store::initialize(&f.dir).unwrap();
    // A new API and Directory stand in for a process restart, not just the old in-memory receipt cache.
    let mut restarted=Api::new(Arc::new(Mutex::new(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap())),f.api.settings.clone(),Some(f.dir.clone()),Default::default(),"offline");
    restarted.sessions=f.api.sessions.clone();restarted.hub=f.api.hub.clone();
    let retry=restarted.apply_request(&op,"new display name","offline",None,ID).unwrap();assert_eq!(retry["user_id"],f.user);
    assert!(restarted.accounts.lock().unwrap().account_of(new).is_some());assert!(restarted.sessions.as_ref().unwrap().lock().unwrap().signed_in(new));
    let conn=rusqlite::Connection::open(f.dir.join("server.db")).unwrap();
    let raw:String=conn.query_row("SELECT result FROM operation_receipts WHERE request_id=?1",[ID],|r|r.get(0)).unwrap();
    for secret in ["Yugi","HASH","RECOVERY"] {assert!(!raw.contains(secret),"{raw}");}
    assert!(restarted.apply_request(&Op::DeleteAccount{who:"Another".into()},"host","offline",None,ID).is_err());
}
#[test]
fn uncertain_disconnect_is_observable_and_never_repeated_after_recovery() {
    let f=Fixture::new("outcome");
    f.sql("CREATE TRIGGER refuse_change BEFORE UPDATE ON operation_effects BEGIN SELECT RAISE(FAIL,'outcome lost'); END;");
    let done=f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server").unwrap();
    assert_eq!(done.disconnect,Some(store::EffectState::Pending));assert!(f.api.accounts.lock().unwrap().account_of(f.user).is_none());
    f.sql("DROP TRIGGER refuse_change;");assert_eq!(store::interrupt_effects(&f.dir).unwrap(),1);
    assert_eq!(f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server").unwrap().disconnect,Some(store::EffectState::Interrupted));
    assert_eq!(f.api.audit(100).unwrap()[0].detail.as_ref().unwrap()["disconnect"],"interrupted");
}
#[test]
fn deletion_rechecks_permissions_even_on_retry_and_keeps_the_last_administrator() {
    let f=Fixture::new("roles");let staff={let mut d=f.api.accounts.lock().unwrap();
        let id=d.register(Login::new("Admin","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();d.set_role("Admin",Role::Admin).unwrap();id};
    let mismatch=DeleteAccountRequest{user_id:f.user,confirm_user_id:staff};
    assert_eq!(f.api.delete_account(&mismatch,ID,Actor::Staff{user_id:staff},"dashboard").unwrap_err().code,MutationErrorCode::InvalidRequest);
    assert!(f.api.delete_account(&request(staff),ID,Actor::Host{by:"host"},"server").is_err());
    f.api.delete_account(&request(f.user),ID,Actor::Staff{user_id:staff},"dashboard").unwrap();
    f.api.accounts.lock().unwrap().set_role("Admin",Role::Moderator).unwrap();
    assert_eq!(f.api.delete_account(&request(f.user),ID,Actor::Staff{user_id:staff},"dashboard").unwrap_err().code,MutationErrorCode::Forbidden);
    assert!(f.api.apply_as(&Op::DeleteAccount{who:"Admin".into()},"Admin","panel",Some(Role::Moderator)).is_err());
}

mod common;
#[test]
fn failed_cleanup_never_notifies_team_members_or_changes_their_live_mail_and_friends() {
    use common::{one,req,req_with};use yo3_proto::Value;
    for failure in ["teams","audit","commit"] {
        let f=Fixture::new(&format!("push-{failure}"));
        let other={let mut d=f.api.accounts.lock().unwrap();let id=d.register(Login::new("Kaiba","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();d.add_player_to(id,"Kaiba").unwrap();id};
        let s=common::Server{world:Arc::new(yo3_server::front::World::new((1..=20).collect())),hub:f.api.hub.as_ref().unwrap().clone()};
        let (mut a,mut b)=(s.join_saved(&f.api.accounts,f.user),s.join_saved(&f.api.accounts,other));
        let yp=f.api.accounts.lock().unwrap().account_of(f.user).unwrap().players[0].player_id;
        let kp=f.api.accounts.lock().unwrap().account_of(other).unwrap().players[0].player_id;
        let u=Value::U32;
        one(a.send(req_with(0x24B9,&[("to_id",u(kp))])));one(b.send(req_with(0x24BB,&[("to_id",u(yp)),("answer",u(2))])));
        one(a.send(req_with(0x2580,&[("mail_type",u(0)),("title",Value::text("Hi")),("body",Value::text("Still here")),("to_id",u(kp))])));
        one(a.send(req_with(0x2AF9,&[("team_name",Value::text("Millennium"))])));
        let team=one(a.send(req(0x2AFB))).get("team_id").and_then(Value::as_u32).unwrap();
        a.send(req_with(0x2AFE,&[("to_id",u(kp))]));b.send(req_with(0x2B00,&[("to_id",u(yp)),("team_id",u(team)),("answer",u(2))]));
        a.pushed();b.pushed();
        match failure {
            "teams"=>store::fail_domain(&f.dir,"teams.json",true).unwrap(),
            "audit"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit'); END;"),
            _=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY);CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1);END;"),
        }
        assert!(f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server").is_err());
        assert!(a.pushed().is_empty() && b.pushed().is_empty(),"{failure}: premature push");
        assert_eq!(s.hub.lock().unwrap().friends_of(kp).iter().map(|(id,_)|*id).collect::<Vec<_>>(),vec![yp]);
        assert_eq!(one(b.send(req(0x2583))).get("count"),Some(&u(1)));
        assert_eq!(one(b.send(req(0x2AFB))).get("team_id"),Some(&u(team)));
        if failure=="teams"{store::fail_domain(&f.dir,"teams.json",false).unwrap();}else{f.sql("DROP TRIGGER refuse_change;");}
        f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server").unwrap();
        assert!(b.pushed().iter().any(|m|m.request()==Some(0x2B07)),"only committed deletion notifies survivors");
        assert_eq!(one(b.send(req(0x2583))).get("count"),Some(&u(0)));
        assert!(s.hub.lock().unwrap().friends_of(kp).is_empty());
    }
}

#[test]
fn independent_concurrent_deletions_both_commit_without_resurrecting_each_other() {
    let mut f=Fixture::new("independent");
    let other=f.api.accounts.lock().unwrap().register(Login::new("Kaiba","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();
    f.api.accounts.lock().unwrap().add_player_to(other,"Kaiba").unwrap();
    for who in ["Yugi","Kaiba"] {f.api.apply(&Op::GiveBp{who:who.into(),amount:10},"host").unwrap();}
    f.api.hub=None; // The stopped-server path loads domains from disk while holding the Directory lock.
    std::thread::scope(|s| {
        let a=s.spawn(||f.api.delete_account(&request(f.user),ID,Actor::Host{by:"host"},"server").unwrap());
        let b=s.spawn(||f.api.delete_account(&request(other),"independent-delete-0001",Actor::Host{by:"host"},"server").unwrap());
        a.join().unwrap();b.join().unwrap();
    });
    let disk=Directory::open(vec![],&f.dir.join("accounts.json")).unwrap();
    assert!(disk.account_of(f.user).is_none() && disk.account_of(other).is_none());
    assert!(store::read(&f.dir.join("mail.json")).unwrap().unwrap()["waiting"].as_array().unwrap().is_empty(),"neither deletion resurrects the other's mail");
    assert_eq!(f.api.audit(100).unwrap().iter().filter(|r|r.op=="delete-account").count(),2);
}
