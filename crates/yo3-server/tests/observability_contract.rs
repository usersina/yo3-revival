//! History must paginate real rows without precision loss, and failure is never an empty success.
use std::{path::PathBuf,sync::{Arc,Mutex}};
use yo3_server::{api::{Api,Actor,MutationErrorCode,observability::AuditQuery},accounts::Directory,settings,store::{self,AuditRow}};
struct Fixture{dir:PathBuf,api:Api}
impl Fixture {
 fn new(name:&str)->Self{let dir=std::env::temp_dir().join(format!("yo3-history-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&dir);store::initialize(&dir).unwrap();
 let api=Api::new(Arc::new(Mutex::new(Directory::open(vec![],&dir.join("accounts.json")).unwrap())),Arc::new(settings::Live::open(None,vec![]).unwrap()),Some(dir.clone()),Default::default(),"server");Self{dir,api}}
 fn sql(&self,sql:&str){rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap();}
 fn note(&self,target:&str){store::audit(&self.dir,&AuditRow{at:"2026-11-17T12:00:00Z".into(),actor:"Admin".into(),origin:"test".into(),op:"test".into(),target:target.into(),before:None,after:None,detail:Some(serde_json::json!({"request_id":"history-proof-00001"}))}).unwrap();}
}
impl Drop for Fixture{fn drop(&mut self){store::close(&self.dir);let _=std::fs::remove_dir_all(&self.dir);}}
fn query()->AuditQuery{AuditQuery{limit:2,before:None,q:String::new()}}
#[test]
fn cursor_ids_above_javascript_precision_remain_exact_and_new_inserts_do_not_shift_older_pages(){
 let f=Fixture::new("cursor");f.note("first");f.sql("UPDATE audit SET id=9007199254740993;");f.note("second");f.note("third");
 let first=f.api.read_audit(&query(),Actor::Host{by:"host"}).unwrap();assert_eq!(first.entries[0].id,"9007199254740995");assert_eq!(first.next_before.as_deref(),Some("9007199254740994"));
 f.note("newest");let mut next=query();next.before=first.next_before;
 let page=f.api.read_audit(&next,Actor::Host{by:"host"}).unwrap();assert_eq!(page.entries.len(),1);assert_eq!(page.entries[0].id,"9007199254740993");assert!(page.next_before.is_none());
 next.before=None;next.q="HISTORY-proof".into();assert_eq!(f.api.read_audit(&next,Actor::Host{by:"host"}).unwrap().entries.len(),2);
 next.q="x' OR 1=1--".into();assert!(f.api.read_audit(&next,Actor::Host{by:"host"}).unwrap().entries.is_empty());
}
#[test]
fn invalid_stored_metadata_and_database_failure_are_errors_instead_of_empty_history(){
 let f=Fixture::new("failure");f.note("target");f.sql("UPDATE audit SET at='invalid';");
 assert_eq!(f.api.read_audit(&query(),Actor::Host{by:"host"}).unwrap_err().code,MutationErrorCode::PersistenceFailed);
 f.sql("UPDATE audit SET at='2026-11-17T12:00:00Z',detail='invalid json';");assert!(f.api.read_audit(&query(),Actor::Host{by:"host"}).is_err());
 f.sql("DROP TABLE audit;");assert!(f.api.read_audit(&query(),Actor::Host{by:"host"}).is_err());
}

#[test]
fn cursor_validation_matches_the_generated_contract(){
 let f=Fixture::new("cursor-validation");for invalid in ["+1","01","0","-1","00000000000000000000000001","9223372036854775808"]{
  let mut q=query();q.before=Some(invalid.into());assert_eq!(f.api.read_audit(&q,Actor::Host{by:"host"}).unwrap_err().code,MutationErrorCode::InvalidRequest,"{invalid}");
 }
}
