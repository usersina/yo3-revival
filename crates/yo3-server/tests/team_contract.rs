//! Teams: the Team window (TeamInfoWindow 0x4BBEC0, main_teaminfo.swf) and PlayerInfoWindow's team buttons start the
//! community agent (modes 5-12); the global dispatcher keeps the team info (0x2B13, slot 17) and members (0x2B17, slot
//! 9), and asks for both again after every team push (0x2B07-0x2B0B, 0x4A9100).
mod common;

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn text(m: &Message, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or_default()
}

fn send(c: &mut Client, id: u32, fields: &[(&str, Value)]) -> Message {
    one(c.send(req_with(id, fields)))
}

fn u(n: u32) -> Value {
    Value::U32(n)
}

/// The team's members as (id, name, authority).
fn members(c: &mut Client) -> Vec<(u32, String, u32)> {
    let r = one(c.send(req(0x2AFD)));
    assert_eq!(r.request(), Some(0x2B17));
    r.items.iter().map(|i| (int(i, "member_id").unwrap(), text(i, "member_name"), int(i, "authority").unwrap())).collect()
}

/// Yugi (1001) forms "Millennium"; its id.
fn formed(yugi: &mut Client) -> u32 {
    let r = send(yugi, 0x2AF9, &[("team_name", Value::text("Millennium"))]);
    assert_eq!((r.request(), text(&r, "team_name")), (Some(0x2B0F), "Millennium".to_string()));
    int(&one(yugi.send(req(0x2AFB))), "team_id").expect("team_id")
}

/// Yugi forms a team and Kaiba (1002) joins it: (server, yugi, kaiba, team_id), pushes cleared.
fn team_of_two() -> (Server, Client, Client, u32) {
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let team = formed(&mut yugi);
    send(&mut yugi, 0x2AFE, &[("to_id", u(1002))]);
    kaiba.pushed();
    send(&mut kaiba, 0x2B00, &[("to_id", u(1001)), ("team_id", u(team)), ("answer", u(2))]);
    yugi.pushed();
    kaiba.pushed();
    (server, yugi, kaiba, team)
}

#[test]
fn without_a_team_the_team_replies_say_so() {
    // 0x2B14 {error_code 404} clears slot 17 (0x4F5CCA): no team. A bare 0x2B13 would store an empty team, which
    // enables the chat's Team tab (0x436CA0) and makes the Team window think there is one.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x2AFB)));
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2B14), Some(404)));
    assert_eq!(one(c.send(req(0x2AFD))).request(), Some(0x2B18));
}

#[test]
fn forming_a_team_makes_the_player_its_leader() {
    // 0x2AF9 {team_name} -> 0x2B0F {team_name} ("You have formed the team", 0x3DCF). 0x2B13 carries team_id, team_name,
    // level, leader_name, created, season_score, score; 0x2B17's rows {member_id, member_name, login_flag, authority}
    // must hold the player's own (its authority goes to front +0xE4; without it the Team window stays blank).
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    let team = formed(&mut yugi);
    let info = one(yugi.send(req(0x2AFB)));
    assert_eq!((int(&info, "team_id"), text(&info, "team_name"), text(&info, "leader_name")), (Some(team), "Millennium".into(), "Yugi".into()));
    assert_eq!(int(&info, "level"), Some(1));
    assert!(!text(&info, "created").is_empty());
    assert_eq!(members(&mut yugi), [(1001, "Yugi".to_string(), 2)]);
    // The record shows it (0x46C1D0: team_level below 1 is no team; team_authority + 1 picks the icon frame).
    let me = one(yugi.send(req(0x2CEE))).items.remove(0);
    assert_eq!((int(&me, "team_level"), text(&me, "team_name"), int(&me, "team_authority")), (Some(1), "Millennium".into(), Some(2)));
}

#[test]
fn a_team_name_is_taken_once() {
    // 0x2B10 {error_code}: 409 the name is taken (0x3DCD), 808 bad characters (0x3DCE).
    let server = Server::new();
    formed(&mut server.join(1001, "Yugi"));
    let r = send(&mut server.join(1002, "Kaiba"), 0x2AF9, &[("team_name", Value::text("millennium"))]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2B10), Some(409)));
}

#[test]
fn an_invited_player_joins_the_team() {
    // 0x2AFE {to_id} -> 0x2B19 ("Inviting %s…"); the invitee gets 0x2AFF {from_id, from_name, team_id, team_name} on the
    // bell (mode 7). 0x2B00 {to_id: the inviter, team_id, answer 2} -> 0x2B1D; the inviter gets 0x2B01 {answer} ("has
    // joined", 0x3DD9) and the members 0x2B08 {member_name}.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let team = formed(&mut yugi);
    kaiba.pushed();
    assert_eq!(send(&mut yugi, 0x2AFE, &[("to_id", u(1002))]).request(), Some(0x2B19));
    let invite = one(kaiba.pushed());
    assert_eq!(invite.request(), Some(0x2AFF));
    assert_eq!((int(&invite, "from_id"), text(&invite, "from_name")), (Some(1001), "Yugi".into()));
    assert_eq!((int(&invite, "team_id"), text(&invite, "team_name")), (Some(team), "Millennium".into()));
    let r = send(&mut kaiba, 0x2B00, &[("to_id", u(1001)), ("team_id", u(team)), ("answer", u(2))]);
    assert_eq!(r.request(), Some(0x2B1D));
    let pushed = yugi.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(0x2B01) && int(m, "answer") == Some(2)), "{pushed:?}");
    assert!(pushed.iter().any(|m| m.request() == Some(0x2B08) && text(m, "member_name") == "Kaiba"), "{pushed:?}");
    assert_eq!(members(&mut kaiba), [(1001, "Yugi".to_string(), 2), (1002, "Kaiba".to_string(), 0)]);
    // Player Information shows a player's team (0x4B6AF0 / 0x4B7350: team_id, team_name, team_authority), and the
    // team tab of the search (search_flag 0x20) lists the team's members.
    let found = one(yugi.send(req_with(0x2CF0, &[("search_flag", u(0x20)), ("base_point", u(0))])));
    let ids: Vec<_> = found.items.iter().filter_map(|i| int(i, "player_id")).collect();
    assert_eq!(ids, [1001, 1002]);
    let kaiba_row = found.items.iter().find(|i| int(i, "player_id") == Some(1002)).unwrap();
    assert_eq!((int(kaiba_row, "team_id"), text(kaiba_row, "team_name"), int(kaiba_row, "team_authority")), (Some(team), "Millennium".into(), Some(0)));
}

#[test]
fn a_player_in_a_team_cant_be_invited() {
    // 0x2B1A {error_code}: 106 "This player cannot receive another request." (0x296C); other codes "A transmission
    // error has occurred." (0x290C), as for a player who went offline.
    let (server, mut yugi, _kaiba, _team) = team_of_two();
    let mut seto = server.join(1004, "Seto");
    send(&mut seto, 0x2AF9, &[("team_name", Value::text("KC"))]);
    let r = send(&mut seto, 0x2AFE, &[("to_id", u(1002))]);
    assert_eq!((r.request(), int(&r, "error_code")), (Some(0x2B1A), Some(106)));
    let r = send(&mut yugi, 0x2AFE, &[("to_id", u(1003))]);
    assert_eq!(r.request(), Some(0x2B1A));
    assert_ne!(int(&r, "error_code"), Some(106));
}

#[test]
fn team_chat_reaches_the_members() {
    // Chat type 3 goes to the team: to_id is slot 17's team_id (0x439D20); members get the 0x2455 line.
    let (_server, mut yugi, mut kaiba, team) = team_of_two();
    let line = Message::record().with("chat_str", Value::text("gg")).with("chat_type", u(3)).with("to_id", u(team));
    let r = yugi.send(yo3_proto::Packet::Message(Message::reply(0x2134).item(line)));
    assert_eq!(r.first().and_then(Message::request), Some(0x2456));
    let got = one(kaiba.pushed());
    assert_eq!((got.request(), text(&got.items[0], "chat_str")), (Some(0x2455), "gg".into()));
}

#[test]
fn a_member_leaves() {
    // 0x2B04 -> 0x2B23; the others get 0x2B09 {member_name} ("… has left the team", 0x3DFF).
    let (_server, mut yugi, mut kaiba, _team) = team_of_two();
    assert_eq!(one(kaiba.send(req(0x2B04))).request(), Some(0x2B23));
    let left = one(yugi.pushed());
    assert_eq!((left.request(), text(&left, "member_name")), (Some(0x2B09), "Kaiba".into()));
    assert_eq!(one(kaiba.send(req(0x2AFB))).request(), Some(0x2B14));
    assert_eq!(members(&mut yugi).len(), 1);
}

#[test]
fn the_leader_promotes_kicks_and_disbands() {
    // 0x2B06 {to_id, authority}: 1 subleader, 0 back to member, 2 the leadership -> 0x2B27, and everyone gets 0x2B0B
    // {authority, to_name}. 0x2B05 {to_id} -> 0x2B25, everyone (the one kicked too, so their refresh clears the team)
    // gets 0x2B0A {admin_name, member_name}. 0x2AFA {player_id} -> 0x2B11, the others get 0x2B07 {team_name}.
    let (_server, mut yugi, mut kaiba, team) = team_of_two();
    assert_eq!(send(&mut yugi, 0x2B06, &[("to_id", u(1002)), ("authority", u(1))]).request(), Some(0x2B27));
    let promoted = one(kaiba.pushed());
    assert_eq!((promoted.request(), int(&promoted, "authority"), text(&promoted, "to_name")), (Some(0x2B0B), Some(1), "Kaiba".into()));
    yugi.pushed();
    assert_eq!(members(&mut yugi)[1].2, 1);
    // A subleader can't kick the leader.
    assert_eq!(send(&mut kaiba, 0x2B05, &[("to_id", u(1001))]).request(), Some(0x2B26));
    // The client sends some of these ids as text (live: 0x2B00's to_id "1001" and team_id "1").
    assert_eq!(send(&mut yugi, 0x2B05, &[("to_id", Value::text("1002"))]).request(), Some(0x2B25));
    let kicked = one(kaiba.pushed());
    assert_eq!((kicked.request(), text(&kicked, "admin_name"), text(&kicked, "member_name")), (Some(0x2B0A), "Yugi".into(), "Kaiba".into()));
    yugi.pushed();
    assert_eq!(one(kaiba.send(req(0x2AFB))).request(), Some(0x2B14));
    // Back in, then the team is disbanded.
    send(&mut yugi, 0x2AFE, &[("to_id", u(1002))]);
    kaiba.pushed();
    send(&mut kaiba, 0x2B00, &[("to_id", u(1001)), ("team_id", u(team)), ("answer", u(2))]);
    yugi.pushed();
    assert_eq!(send(&mut yugi, 0x2AFA, &[("player_id", u(1001))]).request(), Some(0x2B11));
    let gone = one(kaiba.pushed());
    assert_eq!((gone.request(), text(&gone, "team_name")), (Some(0x2B07), "Millennium".into()));
    assert_eq!(one(yugi.send(req(0x2AFB))).request(), Some(0x2B14));
}

#[test]
fn team_mail_reaches_every_member() {
    // MailWindow's Send to Team (mail_type 3, team_id and team_name from slot 17, no to_id; 0x4AE8E0).
    let (_server, mut yugi, mut kaiba, team) = team_of_two();
    let fields = [
        ("mail_type", u(3)),
        ("title", Value::text("Meeting")),
        ("body", Value::text("Tonight")),
        ("team_id", u(team)),
        ("team_name", Value::text("Millennium")),
    ];
    assert_eq!(send(&mut yugi, 0x2580, &fields).request(), Some(0x2584));
    assert_eq!(one(kaiba.pushed()).request(), Some(0x2581));
    let count = int(&one(kaiba.send(req(0x2583))), "count").unwrap();
    let mail = one(kaiba.send(req_with(0x2582, &[("count", u(count))])));
    let m = &mail.items[0];
    assert_eq!((int(m, "mail_type"), int(m, "team_id"), text(m, "team_name")), (Some(3), Some(team), "Millennium".into()));
    assert_eq!(one(yugi.send(req(0x2583))).get("count"), Some(&u(0)), "not to the sender");
}

#[test]
fn teams_are_kept_across_restarts() {
    let dir = std::env::temp_dir().join(format!("yo3-teams-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("teams.json");
    let server = Server::new();
    server.hub.lock().unwrap().keep_teams(&path);
    let team = formed(&mut server.join(1001, "Yugi"));
    let server = Server::new();
    server.hub.lock().unwrap().keep_teams(&path);
    let info = one(server.join(1001, "Yugi").send(req(0x2AFB)));
    assert_eq!((int(&info, "team_id"), text(&info, "team_name")), (Some(team), "Millennium".into()));
}
