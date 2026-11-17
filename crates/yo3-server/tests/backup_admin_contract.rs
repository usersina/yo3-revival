//! Backups are filesystem effects: durable intent before creation, immutable retry receipt, audited downloads.
use std::{path::PathBuf,sync::{Arc,Mutex}};
use yo3_server::{accounts::Directory,api::{Api,Op},settings,store};
struct Fixture{dir:PathBuf,api:Api}
impl Fixture{
 fn new(name:&str)->Self{let dir=std::env::temp_dir().join(format!("yo3-backup-admin-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&dir);store::initialize(&dir).unwrap();
 let api=Api::new(Arc::new(Mutex::new(Directory::open(vec![],&dir.join("accounts.json")).unwrap())),Arc::new(settings::Live::open(None,vec![]).unwrap()),Some(dir.clone()),Default::default(),"server");Self{dir,api}}
 fn sql(&self,sql:&str){rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap();}
}
impl Drop for Fixture{fn drop(&mut self){store::close(&self.dir);let _=std::fs::remove_dir_all(&self.dir);}}
#[test]
fn a_backup_is_not_created_or_reported_successful_when_its_audit_cannot_be_saved(){
 let f=Fixture::new("red");f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit');END;");
 assert!(f.api.apply(&Op::BackUp,"host").is_err(),"backup falsely reports success without audit");assert!(store::backups(&f.dir).is_empty());
}
use yo3_server::{api::{Actor,MutationErrorCode,backups::{CreateBackupRequest,DownloadBackupRequest}},store::EffectState,logins::{Login,Registration,Role}};
const ID:&str="backup-request-000001";
fn create(f:&Fixture)->yo3_server::api::backups::CreateBackupResponse{f.api.create_backup(&CreateBackupRequest{},ID,Actor::Host{by:"host"},"server").unwrap()}
#[test]
fn intent_is_atomic_on_audit_receipt_effect_and_commit_failures(){
 for failure in ["audit","receipt","intent","commit"]{
  let f=Fixture::new(failure);match failure{
   "audit"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL,'no audit');END;"),
   "receipt"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL,'no receipt');END;"),
   "intent"=>f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_effects BEGIN SELECT RAISE(FAIL,'no effect');END;"),
   _=>f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY);CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1);END;")}
  assert!(f.api.create_backup(&CreateBackupRequest{},ID,Actor::Host{by:"host"},"server").is_err());
  assert!(store::backups(&f.dir).is_empty());assert!(f.api.audit(100).unwrap().is_empty());
  let conn=rusqlite::Connection::open(f.dir.join("server.db")).unwrap();
  for table in ["operation_receipts","operation_effects"]{assert_eq!(conn.query_row(&format!("SELECT COUNT(*) FROM {table}"),[],|r|r.get::<_,u32>(0)).unwrap(),0);}
  f.sql("DROP TRIGGER refuse_change;");assert_eq!(create(&f).backup,EffectState::Completed);
 }
}
#[test]
fn concurrent_duplicate_requests_snapshot_once_and_restart_retries_survive_audit_pruning(){
 let f=Fixture::new("concurrent");
 let results=std::thread::scope(|s|{let jobs:Vec<_>=(0..6).map(|_|s.spawn(||create(&f))).collect();jobs.into_iter().map(|j|j.join().unwrap()).collect::<Vec<_>>()});
 assert!(results.iter().all(|r|r.result==results[0].result));assert_eq!(store::backups(&f.dir).len(),1);assert_eq!(f.api.audit(100).unwrap().len(),1);
 let name=&results[0].result.name;let db=f.dir.join("backups").join(name).join("server.db");let check=rusqlite::Connection::open_with_flags(db,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
 assert_eq!(check.query_row("PRAGMA integrity_check",[],|r|r.get::<_,String>(0)).unwrap(),"ok");drop(check);
 f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");store::prune_audit(&f.dir,1).unwrap();store::close(&f.dir);store::initialize(&f.dir).unwrap();
 let next=Api::new(Arc::new(Mutex::new(Directory::open(vec![],&f.dir.join("accounts.json")).unwrap())),f.api.settings.clone(),Some(f.dir.clone()),Default::default(),"server");
 assert_eq!(next.create_backup(&CreateBackupRequest{},ID,Actor::Host{by:"different label"},"server").unwrap().result,results[0].result);
 std::fs::remove_dir_all(f.dir.join("backups").join(name)).unwrap();
 let replay=next.create_backup(&CreateBackupRequest{},ID,Actor::Host{by:"host"},"server").unwrap();assert!(!replay.available);assert_eq!(replay.backup,EffectState::Completed);assert!(store::backups(&f.dir).is_empty(),"a rotated backup must not be recreated on retry");
}
#[test]
fn filesystem_failure_is_saved_and_an_uncertain_outcome_never_creates_a_second_snapshot(){
 let f=Fixture::new("filesystem");std::fs::write(f.dir.join("backups"),b"blocked").unwrap();
 let failed=create(&f);assert_eq!(failed.backup,EffectState::Failed);assert!(!failed.available);
 std::fs::remove_file(f.dir.join("backups")).unwrap();assert_eq!(create(&f).backup,EffectState::Failed);assert!(store::backups(&f.dir).is_empty());
 assert!(f.api.read_backups(Actor::Host{by:"host"}).unwrap().backups.is_empty());
 f.sql("CREATE TRIGGER refuse_outcome BEFORE UPDATE ON operation_effects BEGIN SELECT RAISE(FAIL,'lost outcome');END;");
 let next=f.api.create_backup(&CreateBackupRequest{},"backup-request-000002",Actor::Host{by:"host"},"server").unwrap();assert_eq!(next.backup,EffectState::Pending);assert!(next.available);
 f.sql("DROP TRIGGER refuse_outcome;");assert_eq!(store::interrupt_effects(&f.dir).unwrap(),1);
 let replay=f.api.create_backup(&CreateBackupRequest{},"backup-request-000002",Actor::Host{by:"host"},"server").unwrap();assert_eq!(replay.backup,EffectState::Interrupted);assert_eq!(store::backups(&f.dir).len(),1);
}
#[test]
fn downloads_commit_access_audit_before_handing_out_the_file_and_recheck_permissions_on_retry(){
 let f=Fixture::new("download");let created=create(&f);let request=DownloadBackupRequest{name:created.result.name.clone()};
 let staff={let mut d=f.api.accounts.lock().unwrap();let id=d.register(Login::new("Admin","HASH".into(),"RECOVERY".into()),None,Registration::Open).unwrap();d.set_role("Admin",Role::Admin).unwrap();id};
 f.sql("CREATE TRIGGER refuse_download BEFORE INSERT ON audit WHEN NEW.op='download-backup' BEGIN SELECT RAISE(FAIL,'no audit');END;");
 assert!(f.api.open_backup_download(&request,"download-request-0001",Actor::Staff{user_id:staff},"dashboard").is_err());
 f.sql("DROP TRIGGER refuse_download;");let mut download=f.api.open_backup_download(&request,"download-request-0001",Actor::Staff{user_id:staff},"dashboard").unwrap();
 let mut header=[0u8;16];std::io::Read::read_exact(&mut download.file,&mut header).unwrap();assert_eq!(&header,b"SQLite format 3\0");assert!(download.bytes>16);drop(download);
 f.api.open_backup_download(&request,"download-request-0001",Actor::Staff{user_id:staff},"dashboard").unwrap();
 assert_eq!(f.api.audit(100).unwrap().iter().filter(|r|r.op=="download-backup").count(),1);
 assert_eq!(f.api.create_backup(&CreateBackupRequest{},"download-request-0001",Actor::Staff{user_id:staff},"dashboard").unwrap_err().code,MutationErrorCode::RequestConflict);
 f.api.accounts.lock().unwrap().set_role("Admin",Role::Moderator).unwrap();
 assert_eq!(f.api.open_backup_download(&request,"download-request-0001",Actor::Staff{user_id:staff},"dashboard").err().unwrap().code,MutationErrorCode::Forbidden);
 assert_eq!(f.api.read_backups(Actor::Staff{user_id:staff}).unwrap().backups.len(),1);
 for name in ["../server.db","/etc/passwd","a/b",".","","file\r\nHeader"]{assert!(f.api.open_backup_download(&DownloadBackupRequest{name:name.into()},"download-invalid-001",Actor::Host{by:"host"},"server").is_err());}
}
#[test]
fn schema_five_stop_and_disconnect_effects_survive_adding_backup_outcomes(){
 let f=Fixture::new("schema5");
 f.sql("DROP TABLE operation_effects;CREATE TABLE operation_effects(actor TEXT NOT NULL,request_id TEXT NOT NULL,user_id INTEGER,kind TEXT NOT NULL CHECK(kind IN('disconnect','stop')),state TEXT NOT NULL CHECK(state IN('pending','completed','interrupted')),created INTEGER NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(actor,request_id),FOREIGN KEY(actor,request_id) REFERENCES operation_receipts(actor,request_id)) WITHOUT ROWID;
 INSERT INTO operation_receipts VALUES('host','old-stop-request','shutdown','{}','{}',1);INSERT INTO operation_receipts VALUES('host','old-kick-request','kick','{}','{}',1);
 INSERT INTO operation_effects VALUES('host','old-stop-request',NULL,'stop','completed',1,1);INSERT INTO operation_effects VALUES('host','old-kick-request',1001,'disconnect','pending',1,1);PRAGMA user_version=5;");
 store::close(&f.dir);store::initialize(&f.dir).unwrap();assert_eq!(store::interrupt_effects(&f.dir).unwrap(),1);
 let conn=rusqlite::Connection::open(f.dir.join("server.db")).unwrap();assert_eq!(conn.query_row("SELECT state FROM operation_effects WHERE kind='stop'",[],|r|r.get::<_,String>(0)).unwrap(),"completed");
 assert_eq!(create(&f).backup,EffectState::Completed);
}
#[cfg(unix)]
#[test]
fn partial_and_symlinked_snapshots_are_never_listed_or_downloaded(){
 let f=Fixture::new("links");let created=create(&f);let backups=f.dir.join("backups");
 std::fs::create_dir(backups.join(".pending-unfinished")).unwrap();std::fs::write(backups.join(".pending-unfinished/server.db"),b"partial").unwrap();
 std::os::unix::fs::symlink(backups.join(&created.result.name),backups.join("external")).unwrap();
 assert_eq!(f.api.read_backups(Actor::Host{by:"host"}).unwrap().backups.len(),1);
 assert!(f.api.open_backup_download(&DownloadBackupRequest{name:"external".into()},"download-request-0002",Actor::Host{by:"host"},"server").is_err());
}
