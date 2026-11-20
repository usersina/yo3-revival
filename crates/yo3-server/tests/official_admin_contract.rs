//! The host's official events (#18): `create-event`, `events`, `edit-event` and `cancel-event` (the same operations the
//! dashboard's Events page calls), audited, on a running server (the Stadium's list follows at once) and on a stopped
//! server's data folder.
mod common;

use std::sync::{Arc, Mutex};

use common::{Server, one, req};
use yo3_proto::Value;
use yo3_server::accounts::Directory;
use yo3_server::api::{Api, Op};
use yo3_server::front::World;
use yo3_server::logins::Role;

fn api(server: &Server, dir: Option<&std::path::Path>) -> Api {
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let mut api = Api::new(d, Arc::new(yo3_server::settings::Live::open(None, vec![(3, "Unlimited".into()), (5, "Season 16".into())]).unwrap()),
        dir.map(|p| p.to_path_buf()), Default::default(), "server");
    api.hub = Some(server.hub.clone());
    api.world = Some(server.world.clone());
    api
}

fn create(name: &str, kind: &str) -> Op {
    Op::CreateEvent { kind: kind.into(), name: name.into(), start: "now".into(), end: "+2h".into(), regulation: None,
        participation_bp: 100, threshold: 3, threshold_bp: 400 }
}

#[test]
fn a_host_schedules_edits_and_cancels_an_event_and_the_stadium_follows() {
    let server = Server::with_world(World { banlists: vec![(3, "Unlimited".into()), (5, "Season 16".into())], ..World::new((1..=20).collect()) });
    let api = api(&server, None);
    let mut c = server.join(1001, "Yugi");
    assert!(one(c.send(req(0x177A))).items.is_empty());

    let made = api.apply(&create("Opening Duelist Trial", "Duelist Trial"), "host").unwrap();
    assert_eq!(made["event"]["kind"], "duelist-trial");
    assert_eq!(made["event"]["phase"], "running");
    assert_eq!(made["event"]["prize"]["threshold_bp"], 400);
    let id = made["event"]["event_id"].as_u64().unwrap() as u32;
    let list = one(c.send(req(0x177A)));
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].get("event_id").and_then(Value::as_u32), Some(id));
    assert_eq!(list.items[0].get("event_name_type").and_then(Value::as_u32), Some(2));

    // A retry of the same request is answered as done, not made twice.
    let again = api.apply(&create("Opening Duelist Trial", "duelist-trial"), "host").unwrap();
    assert_eq!(again["event"]["event_id"], id);
    assert_eq!(one(c.send(req(0x177A))).items.len(), 1);

    let edit = Op::EditEvent { id, name: Some("Renamed".into()), start: None, end: Some("+5h".into()), regulation: Some("Season 16".into()),
        participation_bp: Some(250), threshold: None, threshold_bp: None };
    let edited = api.apply(&edit, "host").unwrap();
    assert_eq!((edited["event"]["name"].as_str(), edited["event"]["regulation"].as_u64()), (Some("Renamed"), Some(5)));
    assert_eq!((edited["event"]["prize"]["participation_bp"].as_u64(), edited["event"]["prize"]["threshold"].as_u64()), (Some(250), Some(3)), "what was left out stays");
    let any = Op::EditEvent { id, name: None, start: None, end: None, regulation: Some("any".into()), participation_bp: None, threshold: None, threshold_bp: None };
    assert!(api.apply(&any, "host").unwrap()["event"]["regulation"].is_null());
    let bad = Op::EditEvent { id, name: None, start: None, end: Some("2001-01-01 00:00".into()), regulation: None, participation_bp: None, threshold: None, threshold_bp: None };
    assert!(api.apply(&bad, "host").unwrap_err().contains("must end after"));
    assert!(api.apply(&create("x", "no-such-kind"), "host").unwrap_err().contains("no event kind"));

    let listed = api.apply(&Op::Events, "host").unwrap();
    assert!(listed["message"].as_str().unwrap().contains("Renamed"), "{listed}");

    api.apply(&Op::CancelEvent { id }, "host").unwrap();
    assert!(one(c.send(req(0x177A))).items.is_empty(), "a cancelled event leaves the game's list");
    assert!(api.apply(&Op::CancelEvent { id }, "host").unwrap_err().contains("already cancelled"));
    assert!(api.apply(&Op::CancelEvent { id: 1 }, "host").unwrap_err().contains("No official event"));

    let audit: Vec<String> = api.audit(20).unwrap().into_iter().rev().map(|r| r.op).filter(|op| op.ends_with("-event")).collect();
    assert_eq!(audit, ["create-event", "edit-event", "edit-event", "cancel-event"], "each change once; the retry and the refusals none");
}

#[test]
fn a_moderator_may_look_at_the_events_and_not_change_them() {
    let server = Server::new();
    let api = api(&server, None);
    api.apply(&create("Opening", "gunslinger"), "host").unwrap();
    // The staff roles resolve through the accounts; without a login the role is refused (only admins and moderators).
    assert!(api.apply_as(&create("Mod", "gunslinger"), "Croquet", "panel", Some(Role::Moderator)).unwrap_err().contains("only an admin"));
    assert!(api.apply_as(&Op::Events, "Plain", "panel", Some(Role::Player)).is_err());
}

#[test]
fn a_stopped_servers_data_folder_takes_the_same_commands() {
    let dir = std::env::temp_dir().join(format!("yo3-official-admin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    let offline = Api::new(d, Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "offline");
    offline.apply(&create("Saved", "anniversary"), "host").unwrap();
    // The next server start reads it from the database.
    let mut hub = yo3_server::front::Hub::default();
    hub.keep_official(&dir);
    assert_eq!(hub.official.events.len(), 1);
    assert_eq!(hub.official.events[0].name, "Saved");
    assert!(offline.apply(&Op::Events, "host").unwrap()["message"].as_str().unwrap().contains("Saved"));
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}
