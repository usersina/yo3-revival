//! The host's Championship (#17): `create-championship`, `championships`, `edit-championship` and `cancel-championship` (the
//! same operations the dashboard's Championships page calls), audited, on a running server (the map's list follows at once)
//! and on a stopped server's data folder.
mod common;

use std::sync::{Arc, Mutex};

use common::{Server, one, req};
use yo3_proto::Value;
use yo3_server::accounts::Directory;
use yo3_server::api::{Api, Op};

fn api(server: &Server, dir: Option<&std::path::Path>) -> Api {
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let mut api = Api::new(d, Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()), dir.map(|p| p.to_path_buf()), Default::default(), "server");
    api.hub = Some(server.hub.clone());
    api.world = Some(server.world.clone());
    api
}

fn create(name: &str) -> Op {
    Op::CreateChampionship { name: name.into(), entry_open: None, entry_close: "+2h".into(), start: "+3h".into(), min_duels: 2,
        participation_bp: 10, finalist_bp: 100, winner_bp: 1000 }
}

#[test]
fn a_host_schedules_edits_and_cancels_the_championship_and_the_map_follows() {
    let server = Server::new();
    let api = api(&server, None);
    let mut c = server.join(1001, "Yugi");
    let has_championship = |c: &mut common::Client| one(c.send(req(0x177A))).items.iter().any(|i| i.get("event_type").and_then(Value::as_u32) == Some(10));
    assert!(!has_championship(&mut c));

    let made = api.apply(&create("Winter Cup"), "host").unwrap();
    let id = made["championship"]["championship_id"].as_u64().unwrap() as u32;
    assert_eq!((made["championship"]["state"].as_str(), made["championship"]["min_duels"].as_u64()), (Some("scheduled"), Some(2)));
    assert_eq!(made["championship"]["prize"]["winner_bp"], 1000);
    assert!(has_championship(&mut c), "entries open now, so it is on the map");

    // A retry of the same request is answered as done; a second one that day is refused.
    assert_eq!(api.apply(&create("Winter Cup"), "host").unwrap()["championship"]["championship_id"], id);
    assert!(api.apply(&create("Another"), "host").unwrap_err().contains("around then"));
    assert!(api.apply(&Op::CreateChampionship { name: "Late".into(), entry_open: None, entry_close: "+2h".into(), start: "+3d".into(), min_duels: 0,
        participation_bp: 0, finalist_bp: 0, winner_bp: 0 }, "host").unwrap_err().contains("one day"));

    let edit = Op::EditChampionship { id, name: Some("Spring Cup".into()), entry_open: None, entry_close: None, start: Some("+4h".into()), min_duels: Some(0),
        participation_bp: None, finalist_bp: Some(300), winner_bp: None };
    let edited = api.apply(&edit, "host").unwrap();
    assert_eq!((edited["championship"]["name"].as_str(), edited["championship"]["min_duels"].as_u64()), (Some("Spring Cup"), Some(0)));
    assert_eq!((edited["championship"]["prize"]["finalist_bp"].as_u64(), edited["championship"]["prize"]["winner_bp"].as_u64()), (Some(300), Some(1000)), "what was left out stays");
    assert!(api.apply(&Op::Championships, "host").unwrap()["message"].as_str().unwrap().contains("Spring Cup"));

    api.apply(&Op::CancelChampionship { id }, "host").unwrap();
    assert!(!has_championship(&mut c), "a cancelled Championship leaves the map");
    assert!(api.apply(&Op::CancelChampionship { id }, "host").unwrap_err().contains("already cancelled"));
    assert!(api.apply(&Op::CancelChampionship { id: 99 }, "host").unwrap_err().contains("No Championship"));

    let audit: Vec<String> = api.audit(20).unwrap().into_iter().rev().map(|r| r.op).filter(|op| op.ends_with("-championship")).collect();
    assert_eq!(audit, ["create-championship", "edit-championship", "cancel-championship"], "each change once; the retry and the refusals none");
}

#[test]
fn a_stopped_servers_data_folder_takes_the_same_commands() {
    let dir = std::env::temp_dir().join(format!("yo3-champ-admin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let offline = Api::new(d, Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "offline");
    offline.apply(&create("Saved"), "host").unwrap();
    // The next server start reads it from the database.
    let mut hub = yo3_server::front::Hub::default();
    hub.keep_championships(&dir);
    assert_eq!(hub.championships.list.len(), 1);
    assert_eq!((hub.championships.list[0].name.as_str(), hub.championships.list[0].min_duels), ("Saved", 2));
    assert!(offline.apply(&Op::Championships, "host").unwrap()["message"].as_str().unwrap().contains("Saved"));
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}
