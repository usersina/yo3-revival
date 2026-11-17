//! The connected overview must preserve the private panel's checked operational information.
use std::{path::PathBuf,sync::{Arc,Mutex}};
use serde_json::Value;
use yo3_server::{accounts::Directory,api::{Api,Op},settings,store};
struct Fixture{dir:PathBuf,api:Api}
impl Fixture{
 fn new()->Self{let dir=std::env::temp_dir().join(format!("yo3-overview-{}",std::process::id()));let _=std::fs::remove_dir_all(&dir);store::initialize(&dir).unwrap();
 let api=Api::new(Arc::new(Mutex::new(Directory::open(vec![],&dir.join("accounts.json")).unwrap())),Arc::new(settings::Live::open(None,vec![]).unwrap()),Some(dir.clone()),Default::default(),"server");Self{dir,api}}
 fn view(&self)->Value{serde_json::to_value(self.api.overview()).unwrap()}
}
impl Drop for Fixture{fn drop(&mut self){store::close(&self.dir);let _=std::fs::remove_dir_all(&self.dir);}}
#[test]
fn overview_distinguishes_no_backup_a_checked_snapshot_and_unreadable_storage(){
 let f=Fixture::new();let first=f.view();assert_eq!(first["backup"]["kind"],"none","the real overview omits backup health");
 assert!(first["engines"].is_u64());assert_eq!(first["online_players"],serde_json::json!([]));
 if !first["memory_bytes"].is_null(){assert!(first["memory_bytes"].as_str().unwrap().parse::<u64>().is_ok(),"memory is a precision-safe decimal string");}
 f.api.apply(&Op::BackUp,"host").unwrap();let checked=f.view();assert_eq!(checked["backup"]["kind"],"available");
 assert!(chrono::DateTime::parse_from_rfc3339(checked["backup"]["at"].as_str().unwrap()).is_ok());
 std::fs::rename(f.dir.join("backups"),f.dir.join("saved-backups")).unwrap();std::fs::write(f.dir.join("backups"),b"blocked").unwrap();
 assert_eq!(f.view()["backup"]["kind"],"unavailable","an unreadable backup directory is not 'no backups'");
}
