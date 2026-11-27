//! Official Events (the map's Stadium, #18): OfficialEventScene (`0x4B0110`) asks for the list (`0x177A`, reply `0x177B`,
//! read by `0x4B1060`, which keeps the items with `official` set and `event_name_type` 1-5), polls the lobby
//! (`0x177F` -> `0x1780`, `0x4B0B70`), joins it (`0x1784` -> `0x1785` with `join_id`, a non-zero `server_id`, `event_score`
//! and the echoed `event_id`), Participate sends `0x1798 {event_id, join_id}` (no format: the event's kind decides it),
//! cancel `0x17A2`, leave `0x178E`. The match is the usual `0x1978`; a duel's end `0x2009` carries `score_<player_id>`.
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Packet, Value};
use yo3_server::accounts::Directory;
use yo3_server::front::{Ended, World};
use yo3_server::official::{Kind, NewEvent, Prize};

const STADIUM_LIST: u32 = 0x177A;

fn now() -> i64 { chrono::Utc::now().timestamp() }

/// An event running now for an hour.
fn schedule(server: &Server, kind: Kind, prize: Prize) -> u32 {
    schedule_at(server, kind, now() - 60, now() + 3600, prize)
}

fn schedule_at(server: &Server, kind: Kind, start: i64, end: i64, prize: Prize) -> u32 {
    let new = NewEvent { kind, name: "Test event".into(), start, end, regulation: None, prize };
    server.hub.lock().unwrap().official.create(new).expect("scheduled").id
}

fn int(m: &Message, k: &str) -> Option<u32> { m.get(k).and_then(Value::as_u32) }

/// 0x1784 {event_id}: the join reply.
fn join(c: &mut Client, event: u32) -> Message {
    one(c.send(req_with(0x1784, &[("event_id", Value::U32(event))])))
}

/// 0x1784 then 0x1798 {event_id, join_id}: the Participate reply.
fn participate(c: &mut Client, event: u32) -> Message {
    let joined = join(c, event);
    assert_eq!(joined.request(), Some(0x1785), "{joined:?}");
    let join_id = int(&joined, "join_id").expect("join_id");
    one(c.send(req_with(0x1798, &[("event_id", Value::U32(event)), ("join_id", Value::U32(join_id))])))
}

fn pushed_within(c: &mut Client, wait: Duration) -> Vec<Message> {
    let start = Instant::now();
    let mut out = c.pushed();
    while out.is_empty() && start.elapsed() < wait {
        std::thread::sleep(Duration::from_millis(20));
        out = c.pushed();
    }
    out
}

fn named<'a>(m: &'a Message, name: &str) -> &'a Message {
    m.items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some(name)).unwrap_or_else(|| panic!("no {name} in {m:?}"))
}

#[test]
fn the_stadium_lists_the_hosts_events_as_official_items() {
    // 0x4B1060: official != 0, event_name_type 1-5, status, event_start/event_end (text times), player_num, event_id.
    let server = Server::new();
    let mut c = server.join(1001, "Yugi");
    assert!(one(c.send(req(STADIUM_LIST))).items.is_empty(), "no events scheduled");
    let mut ids = vec![];
    for kind in Kind::ALL {
        ids.push(schedule(&server, kind, Prize::default()));
    }
    let list = one(c.send(req(STADIUM_LIST)));
    assert_eq!(list.request(), Some(0x177B));
    assert_eq!(int(&list, "result_number"), Some(5));
    let mut types: Vec<u32> = list.items.iter().map(|i| int(i, "event_name_type").unwrap()).collect();
    types.sort();
    assert_eq!(types, [1, 2, 3, 4, 5], "Gunslinger, Duelist Trial, Evolution, Anniversary, Ultimate");
    for item in &list.items {
        assert_eq!(int(item, "official"), Some(1));
        assert_eq!(int(item, "status"), Some(13), "running and joinable");
        assert!(ids.contains(&int(item, "event_id").unwrap()));
        // The client parses both as text, "%d-%d-%d %d:%d:%d", with mktime (0x4AB490); a number gives a null time and a crash.
        let time = |k: &str| chrono::NaiveDateTime::parse_from_str(&item.get(k).and_then(Value::as_str).expect(k), "%Y-%m-%d %H:%M:%S").expect(k);
        assert!(time("event_start") < time("event_end"));
        assert_eq!(int(item, "player_num"), Some(0));
        assert_ne!(int(item, "event_type"), Some(10), "10 is the Championship, which the Tournament scene looks for");
    }
}

#[test]
fn an_event_is_coming_soon_then_running_then_finished_and_a_cancelled_one_is_gone() {
    // status 10 or 11 "coming soon", 13 running and joinable, anything else finished (0x4B1060 and the lobby).
    let server = Server::new();
    let mut c = server.join(1001, "Yugi");
    let soon = schedule_at(&server, Kind::Gunslinger, now() + 5 * 86_400, now() + 6 * 86_400, Prize::default());
    let past = schedule_at(&server, Kind::DuelistTrial, now() - 7200, now() - 3600, Prize::default());
    let doomed = schedule(&server, Kind::Anniversary, Prize::default());
    server.hub.lock().unwrap().official.cancel(doomed).unwrap();
    let list = one(c.send(req(STADIUM_LIST)));
    let status = |id: u32| list.items.iter().find(|i| int(i, "event_id") == Some(id)).and_then(|i| int(i, "status"));
    assert_eq!(status(soon), Some(10));
    assert_eq!(status(past), Some(14));
    assert_eq!(status(doomed), None);
    // Entering one that isn't running is refused (0x1786), so Participate can't start.
    assert_eq!(join(&mut c, soon).request(), Some(0x1786));
    assert_eq!(join(&mut c, past).request(), Some(0x1786));
}

#[test]
fn the_lobby_poll_returns_the_event_and_the_players_score() {
    // 0x4B0B70 reads item[0]: event_id, status, event_start, event_end, player_num.
    let server = Server::new();
    let mut c = server.join(1001, "Yugi");
    let id = schedule(&server, Kind::DuelistTrial, Prize::default());
    let lobby = one(c.send(req_with(0x177F, &[("event_id", Value::U32(id))])));
    assert_eq!(lobby.request(), Some(0x1780));
    let item = &lobby.items[0];
    for key in ["event_id", "status", "event_start", "event_end", "player_num"] {
        assert!(item.get(key).is_some(), "{key}");
    }
    assert_eq!((int(item, "event_id"), int(item, "official"), int(item, "player_num")), (Some(id), Some(1), Some(0)));
    join(&mut c, id);
    let lobby = one(c.send(req_with(0x177F, &[("event_id", Value::U32(id))])));
    assert_eq!(int(&lobby.items[0], "player_num"), Some(1), "the player entered the lobby");
    let missing = one(c.send(req_with(0x177F, &[("event_id", Value::U32(99_999))])));
    assert_eq!(missing.request(), Some(0x1781));
}

#[test]
fn joining_gives_a_join_id_a_server_id_the_score_and_the_event_id_back() {
    // 0x1785 (0x4A1090): join_id; the dispatcher (0x4F4AE7) keeps event_id, join_id, server_id, event_score. Without a
    // non-zero server_id Participate never starts matching (0x4A3F70); without the echoed event_id the lobby shows
    // "This event is now finished" after a duel.
    let server = Server::new();
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let id = schedule(&server, Kind::Gunslinger, Prize::default());
    let ja = join(&mut a, id);
    let jb = join(&mut b, id);
    assert_eq!(ja.request(), Some(0x1785));
    assert!(int(&ja, "join_id").is_some_and(|j| j > 0));
    assert_ne!(int(&ja, "join_id"), int(&jb, "join_id"));
    assert!(int(&ja, "server_id").is_some_and(|s| s != 0));
    assert_eq!((int(&ja, "event_id"), int(&ja, "event_score")), (Some(id), Some(0)));
    // Entering again doesn't count the player twice.
    join(&mut a, id);
    let lobby = one(a.send(req_with(0x177F, &[("event_id", Value::U32(id))])));
    assert_eq!(int(&lobby.items[0], "player_num"), Some(2));
    // Auto Duel's own entry is unchanged: no server_id, no echo.
    let auto = one(a.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
    assert_eq!((auto.get("event_id"), auto.get("server_id")), (None, None));
}

#[test]
fn participants_of_one_event_are_matched_and_nobody_else() {
    let server = Server::new();
    let (mut a, mut b, mut c, mut d) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"), server.join(1004, "Mai"));
    let (one_id, other_id) = (schedule(&server, Kind::DuelistTrial, Prize::default()), schedule(&server, Kind::Anniversary, Prize::default()));
    assert_eq!(participate(&mut a, one_id).request(), Some(0x1799));
    // Another event's entrant and an Auto Duel waiter don't match it.
    assert_eq!(participate(&mut c, other_id).request(), Some(0x1799));
    let auto = one(d.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
    let join_id = int(&auto, "join_id").unwrap();
    d.send(req_with(0x1798, &[("event_id", Value::U32(2)), ("join_id", Value::U32(join_id)), ("match_type", Value::U32(1)), ("level", Value::U32(0))]));
    assert!(pushed_within(&mut a, Duration::from_millis(200)).is_empty());
    assert_eq!(participate(&mut b, one_id).request(), Some(0x1799));
    let to_a = one(pushed_within(&mut a, Duration::from_secs(3)));
    let to_b = one(pushed_within(&mut b, Duration::from_secs(3)));
    assert_eq!(to_a.request(), Some(0x1978));
    assert_eq!(int(named(&to_a, "vsPlayerData"), "player_id"), Some(1002));
    assert_eq!(int(named(&to_b, "vsPlayerData"), "player_id"), Some(1001));
    assert_eq!(int(named(&to_a, "duelData"), "match_type"), Some(1), "Duelist Trial is single duels");
    assert!(c.pushed().is_empty() && d.pushed().is_empty());
}

#[test]
fn the_match_formats_are_played_as_matches() {
    for kind in [Kind::Evolution, Kind::Ultimate] {
        let server = Server::new();
        let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
        let id = schedule(&server, kind, Prize::default());
        participate(&mut a, id);
        participate(&mut b, id);
        let to_a = one(pushed_within(&mut a, Duration::from_secs(3)));
        assert_eq!(int(named(&to_a, "duelData"), "match_type"), Some(3), "{kind:?}");
    }
}

#[test]
fn cancel_and_leaving_take_the_player_out_of_the_queue() {
    let server = Server::new();
    let (mut a, mut b, mut c) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    let id = schedule(&server, Kind::DuelistTrial, Prize::default());
    participate(&mut a, id);
    participate(&mut b, id);
    let _ = pushed_within(&mut a, Duration::from_secs(3));
    // a and b are matched. Now c waits, cancels (0x17A2), and a later entrant isn't paired with them.
    participate(&mut c, id);
    assert_eq!(one(c.send(req_with(0x17A2, &[("event_id", Value::U32(id))]))).request(), Some(0x17A3));
    let mut d = server.join(1004, "Mai");
    participate(&mut d, id);
    assert!(pushed_within(&mut c, Duration::from_millis(300)).is_empty());
    // Leaving the lobby (0x178E) ends the wait too.
    let mut e = server.join(1005, "Tea");
    participate(&mut e, id);
    assert_eq!(one(d.send(req_with(0x178E, &[("event_id", Value::U32(id))]))).request(), Some(0x178F));
    assert!(pushed_within(&mut e, Duration::from_millis(300)).is_empty(), "nobody is left to match e with");
}

#[test]
fn an_event_under_another_banlist_is_joined_and_its_duels_carry_it() {
    // #77: everyone logs in with the server's banlist, so an event under another one no longer asks the entrant to change
    // theirs: they wait as usual, the chat says the event's banlist, and the match's duelData (and rule) carries it as
    // regulation_id, which the game's Select Deck and the server's deck check read.
    let server = Server::new();
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let new = NewEvent { kind: Kind::DuelistTrial, name: "Banlist".into(), start: now() - 60, end: now() + 3600, regulation: Some(77), prize: Prize::default() };
    let id = server.hub.lock().unwrap().official.create(new).unwrap().id;
    for c in [&mut a, &mut b] {
        let joined = join(c, id);
        let join_id = int(&joined, "join_id").unwrap();
        let r = one(c.send(req_with(0x1798, &[("event_id", Value::U32(id)), ("join_id", Value::U32(join_id))])));
        assert_eq!(r.request(), Some(0x1799), "waiting, not refused");
    }
    server.hub.lock().unwrap().match_waiting();
    let said = pushed_within(&mut a, Duration::from_secs(3));
    assert!(said.iter().flat_map(|m| &m.items).any(|i| i.get("chat_str").and_then(Value::as_str).is_some_and(|t| t.contains("played under the banlist 77"))), "{said:?}");
    let to_a = said.iter().find(|m| m.request() == Some(0x1978)).expect("the match");
    assert_eq!(int(named(to_a, "duelData"), "regulation_id"), Some(77));
}

/// Two entrants matched, and the duel id of their match.
fn matched(server: &Server, kind: Kind, prize: Prize) -> (Client, Client, u32, u32) {
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let id = schedule(server, kind, prize);
    participate(&mut a, id);
    participate(&mut b, id);
    let to_a = one(pushed_within(&mut a, Duration::from_secs(3)));
    let _ = pushed_within(&mut b, Duration::from_secs(3));
    let duel = int(named(&to_a, "duelData"), "duel_id").unwrap();
    (a, b, id, duel)
}

fn ended(player: u32, opponent: u32, duel: u32, result: u32, match_type: u32) -> Ended {
    Ended { player_id: player, opponent, npc: false, duel_id: duel, match_type, result, replay: None, stats: None, start_lp: 8000 }
}

fn score_of(pushed: &[Message], player: u32) -> Option<u32> {
    pushed.iter().find(|m| m.request() == Some(0x2009)).and_then(|m| int(m, &format!("score_{player}")))
}

#[test]
fn a_duels_result_updates_the_event_score_pushed_with_the_result() {
    // The dispatcher (0x4F4C9E) copies the 0x2009 push's `score_<player_id>` into `event_score`.
    let server = Server::new();
    let (mut a, mut b, id, duel) = matched(&server, Kind::DuelistTrial, Prize::default());
    {
        let mut hub = server.hub.lock().unwrap();
        hub.finish_duel(&ended(1001, 1002, duel, 1, 1));
        hub.finish_duel(&ended(1002, 1001, duel, 2, 1));
    }
    assert_eq!(score_of(&a.pushed(), 1001), Some(1), "a win is a point");
    assert_eq!(score_of(&b.pushed(), 1002), Some(0), "a loss adds none, but the score is still sent");
    // The lobby shows it too.
    let lobby = one(a.send(req_with(0x177F, &[("event_id", Value::U32(id))])));
    assert_eq!(int(&lobby.items[0], "event_score"), Some(1));
    assert_eq!(int(&one(a.send(req_with(0x1784, &[("event_id", Value::U32(id))]))), "event_score"), Some(1));
}

#[test]
fn a_gunslingers_score_is_its_win_streak() {
    let server = Server::new();
    let (mut a, _b, _id, duel) = matched(&server, Kind::Gunslinger, Prize::default());
    for (result, expected) in [(1, 1), (1, 2), (2, 0), (1, 1)] {
        server.hub.lock().unwrap().finish_duel(&ended(1001, 1002, duel, result, 1));
        assert_eq!(score_of(&a.pushed(), 1001), Some(expected), "after result {result}");
    }
}

#[test]
fn a_duel_outside_an_event_and_an_npc_duel_send_no_score() {
    let server = Server::new();
    let (mut a, _b, _id, _duel) = matched(&server, Kind::DuelistTrial, Prize::default());
    {
        let mut hub = server.hub.lock().unwrap();
        hub.finish_duel(&ended(1001, 1002, 4242, 1, 1));
        hub.finish_duel(&Ended { npc: true, opponent: 125, ..ended(1001, 125, 4242, 1, 1) });
    }
    assert_eq!(score_of(&a.pushed(), 1001), None);
}

#[test]
fn tournament_points_and_their_rank_show_on_my_profile() {
    // My Profile (ProfileGet 0x4705D0) reads `tp`; player records carry `tprank`.
    let server = Server::new();
    let (mut a, mut b, _id, duel) = matched(&server, Kind::DuelistTrial, Prize::default());
    let profile = |c: &mut Client| {
        let list = one(c.send(req(0x1F49)));
        let me = list.items.iter().find(|i| int(i, "pindex").is_some()).cloned().expect("own record");
        (int(&me, "tp"), int(&me, "tprank"))
    };
    assert_eq!(profile(&mut a), (Some(0), Some(0)), "no points before a duel of an event");
    {
        let mut hub = server.hub.lock().unwrap();
        hub.finish_duel(&ended(1001, 1002, duel, 1, 1));
        hub.finish_duel(&ended(1002, 1001, duel, 2, 1));
    }
    // 5 for playing and the best score: Yugi 5 + 1, Kaiba 5.
    assert_eq!(profile(&mut a), (Some(6), Some(1)));
    assert_eq!(profile(&mut b), (Some(5), Some(2)));
}

const ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));

#[test]
fn prizes_are_paid_in_bp_by_d_mail_when_the_event_ends() {
    let server = Server::new();
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into(), "Kaiba".into()])));
    server.hub.lock().unwrap().set_directory(d.clone());
    let mut c = server.join_from(&d, ADDR);
    let list = one(c.send(req(0x1F49)));
    let (player, name) = (int(&list.items[0], "player_id").unwrap(), list.items[0].get("pname").and_then(Value::as_str).unwrap());
    one(c.send(req_with(0x1F4C, &[("player_id", Value::U32(player))])));
    let end = now() + 3600;
    let id = schedule_at(&server, Kind::DuelistTrial, now() - 60, end, Prize { participation_bp: 100, threshold: 2, threshold_bp: 400 });
    join(&mut c, id);
    {
        let mut hub = server.hub.lock().unwrap();
        hub.official.record(id, player, &name, 1, now());
        hub.official.record(id, player, &name, 1, now());
    }
    let user = d.lock().unwrap().user_of(&name).unwrap();
    let before = d.lock().unwrap().dpoint(user).unwrap();
    c.pushed();
    server.hub.lock().unwrap().tick_official(end - 1);
    assert_eq!(d.lock().unwrap().dpoint(user), Some(before), "nothing before the end");
    server.hub.lock().unwrap().tick_official(end);
    assert_eq!(d.lock().unwrap().dpoint(user), Some(before + 500), "participation and the threshold");
    assert!(c.pushed().iter().any(|m| m.request() == Some(0x2581)), "the bell's mail icon");
    let mail = one(c.send(req_with(0x2582, &[("count", Value::U32(1))])));
    let item = &mail.items[0];
    assert_eq!(item.get("from_name").and_then(Value::as_str).as_deref(), Some("Host"));
    assert!(item.get("body").and_then(Value::as_str).is_some_and(|b| b.contains("Test event") && b.contains("400 BP")), "{item:?}");
    // Paid once.
    server.hub.lock().unwrap().tick_official(end + 60);
    assert_eq!(d.lock().unwrap().dpoint(user), Some(before + 500));
}

// The Ultimate Match against the computer (#18). The client's own text for the event (15261) calls it a "PvP or PvCPU
// Match Duel event"; nothing says when the computer plays, so it is the fallback of a wait (a guess, the live setting
// `ultimate-cpu-wait`), and its match is one duel pushed like a player's (`0x1978`, the NPC's record as `vsPlayerData`).

/// A server with six NPCs and the computer's wait set to `wait` seconds (0: never).
fn with_computers(wait: u32) -> Server {
    let names = [(120, "Seto Kaiba"), (121, "Seto Kaiba"), (122, "Seto Kaiba"), (123, "Joey Wheeler"), (124, "Joey Wheeler"), (125, "Joey Wheeler")];
    let world = World { npcs: (120..126).collect(), npc_names: names.iter().map(|(i, n)| (*i, n.to_string())).collect(), ..World::new((1..=20).collect()) };
    let server = Server::with_world(world);
    let live = yo3_server::settings::Live::open(None, vec![]).unwrap();
    live.set("ultimate-cpu-wait", &wait.to_string()).unwrap();
    server.hub.lock().unwrap().set_settings(Arc::new(live));
    server
}

fn computers(server: &Server) { server.hub.lock().unwrap().match_computers(&server.world); }

#[test]
fn an_ultimate_entrant_nobody_joins_gets_a_computer_after_the_wait() {
    let server = with_computers(1);
    let mut a = server.join(1001, "Yugi");
    let id = schedule(&server, Kind::Ultimate, Prize::default());
    assert_eq!(participate(&mut a, id).request(), Some(0x1799));
    computers(&server);
    assert!(pushed_within(&mut a, Duration::from_millis(200)).is_empty(), "not before the wait");
    std::thread::sleep(Duration::from_millis(1100));
    computers(&server);
    let push = one(pushed_within(&mut a, Duration::from_secs(3)));
    assert_eq!(push.request(), Some(0x1978));
    let vs = named(&push, "vsPlayerData");
    let npc = int(vs, "player_id").unwrap();
    assert!((120..126).contains(&npc), "an NPC of the client's roster, not a player: {npc}");
    let name = vs.get("pname").and_then(Value::as_str).unwrap();
    assert_eq!(name, if npc < 123 { "Seto Kaiba" } else { "Joey Wheeler" }, "its name from the client's text table");
    let data = named(&push, "duelData");
    assert!(int(data, "duel_id").is_some_and(|d| d > 0));
    assert_eq!(int(data, "match_type"), Some(1), "one duel counts as the match (guess)");
    // Paired once: no second push, and the player no longer waits.
    std::thread::sleep(Duration::from_millis(1100));
    computers(&server);
    assert!(pushed_within(&mut a, Duration::from_millis(200)).is_empty());
}

#[test]
fn the_computer_plays_only_in_the_ultimate_match_and_only_when_the_wait_is_on() {
    let server = with_computers(1);
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (evolution, trial) = (schedule(&server, Kind::Evolution, Prize::default()), schedule(&server, Kind::DuelistTrial, Prize::default()));
    participate(&mut a, evolution);
    participate(&mut b, trial);
    std::thread::sleep(Duration::from_millis(1100));
    computers(&server);
    assert!(pushed_within(&mut a, Duration::from_millis(300)).is_empty() && b.pushed().is_empty());
    // A wait of 0 turns the computer off.
    let off = with_computers(0);
    let mut c = off.join(1003, "Joey");
    let id = schedule(&off, Kind::Ultimate, Prize::default());
    participate(&mut c, id);
    std::thread::sleep(Duration::from_millis(100));
    computers(&off);
    assert!(pushed_within(&mut c, Duration::from_millis(300)).is_empty());
}

#[test]
fn a_player_who_arrives_first_is_matched_before_the_computer() {
    let server = with_computers(1);
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let id = schedule(&server, Kind::Ultimate, Prize::default());
    participate(&mut a, id);
    participate(&mut b, id);
    let to_a = one(pushed_within(&mut a, Duration::from_secs(3)));
    assert_eq!(int(named(&to_a, "vsPlayerData"), "player_id"), Some(1002));
    std::thread::sleep(Duration::from_millis(1100));
    computers(&server);
    assert!(pushed_within(&mut a, Duration::from_millis(200)).is_empty() && b.pushed().len() == 1, "only the player match's push");
}

#[test]
fn the_computer_match_brings_the_npc_to_the_duel_and_counts_a_star_like_a_player_match() {
    let server = with_computers(1);
    let mut a = server.join(1001, "Yugi");
    let id = schedule(&server, Kind::Ultimate, Prize::default());
    participate(&mut a, id);
    std::thread::sleep(Duration::from_millis(1100));
    computers(&server);
    let push = one(pushed_within(&mut a, Duration::from_secs(3)));
    let (duel, npc) = (int(named(&push, "duelData"), "duel_id").unwrap(), int(named(&push, "vsPlayerData"), "player_id").unwrap());
    // Select Deck for the match's duel_id (0x1F4F) is accepted and the duel connection is told to play the NPC, not to sit
    // at a table.
    let deck = Message::deck().with("deck_name", Value::text("Starter Deck"));
    let fields = [("duel_id", Value::U32(duel)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))];
    let update = fields.iter().fold(Message::reply(0x1F4F), |m, (k, v)| m.with(k, v.clone())).item(deck);
    assert_eq!(one(a.send(Packet::Message(update))).request(), Some(0x22CA));
    let entry = server.hub.lock().unwrap().duel_entry(1001).expect("the deck was brought");
    assert_eq!((entry.npc_id, entry.cpu_match, entry.duel_id), (Some(npc), Some(duel), None));
    // The computer match does not count as a duel that never started: no engine counted before it begins, and the
    // 180 s "nobody came" ending leaves it alone.
    server.hub.lock().unwrap().tick_matches(Instant::now() + Duration::from_secs(600));
    assert!(server.hub.lock().unwrap().match_ids().contains(&duel));
    // Its end: a win is a star, shown in the result and the lobby; the match is over.
    server.hub.lock().unwrap().finish_duel(&Ended { npc: true, opponent: npc, ..ended(1001, npc, duel, 1, 1) });
    assert_eq!(score_of(&a.pushed(), 1001), Some(1));
    assert!(!server.hub.lock().unwrap().match_ids().contains(&duel));
    assert_eq!(int(&one(a.send(req_with(0x177F, &[("event_id", Value::U32(id))]))).items[0], "event_score"), Some(1));
    // An ordinary NPC duel afterwards is not the event's.
    server.hub.lock().unwrap().finish_duel(&Ended { npc: true, opponent: npc, ..ended(1001, npc, 0, 1, 1) });
    assert_eq!(score_of(&a.pushed(), 1001), None);
}
