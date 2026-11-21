//! The Championship (the map's Tournament tile, #17): TournamentScene (`0x4CB570`, `+0x2D8` 0) takes the first `0x177B`
//! item with `event_type` 10 and `status` 12 as the Championship (`0x4CBD70`), asks `0x32C9` (`0x4D1770`; `status` 0 is the MC's
//! "not started yet" and the end of the setup), then joins its lobby (`0x1784`, `0x4D1B30`) and asks the bracket requests the
//! user events use (`0x32CA`, `0x32CC`, `0x32CD`, `0x32D5`, `0x32CB`). `0x32FB` carries
//! `tournament_id` (17 or more: the Final, 16 seats, four rounds, `0x4CE670`, `0x4CEBE0`), `championship_id`, `year`
//! and `season`. The entry draw is `0x32D0 {phase, status}` and `0x32D1 {lot}` (`0x4CC5B0`); duels are the match push, whose
//! `duelData` must not carry `event_type` 10 and `official` 1 (the client crashed at the end of the duel; the lobby sets the
//! mode and music itself, `0x4D1F90`). Duel Log's `0x1793` lists the past Championships.
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::Directory;
use yo3_server::duel::DuelSession;
use yo3_server::front::{Deck, DuelEntry, Ended, World};
use yo3_server::translate::CardStats;
use yo3_server::championship::{ChampionshipPrize as Prize, ChampionshipState as State, FIRST_EVENT, New};
use yo3_server::titles::TitleData;

fn int(m: &Message, k: &str) -> Option<u32> { m.get(k).and_then(Value::as_u32) }
fn requests(pushed: &[Message]) -> Vec<u32> { pushed.iter().filter_map(Message::request).collect() }
fn now() -> i64 { chrono::Utc::now().timestamp() }

/// A Championship whose entry window is open now and which starts in a few minutes.
fn schedule(server: &Server) -> (u32, i64) { schedule_with(server, 0, Prize::default()) }

fn schedule_with(server: &Server, min_duels: u32, prize: Prize) -> (u32, i64) {
    let start = now() + 200;
    let new = New { name: "Y.O.C. Test".into(), entry_open: now() - 100, entry_close: now() + 100, start, min_duels, prize };
    let c = server.hub.lock().unwrap().championships.create(new).expect("scheduled");
    (c.event_id(), start)
}

fn ask(c: &mut Client, id: u32, event: u32, extra: &[(&str, Value)]) -> Message {
    let mut fields = vec![("event_id", Value::U32(event))];
    fields.extend(extra.iter().map(|(k, v)| (*k, v.clone())));
    one(c.send(req_with(id, &fields)))
}

fn enter(c: &mut Client, event: u32) -> Message { ask(c, 0x1784, event, &[]) }

fn tick(server: &Server, start: i64) { server.hub.lock().unwrap().tick_championships(start + 1, Instant::now()) }

/// The seats `0x32CB` shows over the Final's three blocks (0, 1 and the final's 0x40): each block's player ids, 0 empty.
fn blocks(c: &mut Client, event: u32) -> Vec<(u32, Vec<u32>, Vec<u32>)> {
    let map = ask(c, 0x32D5, event, &[]);
    let flags = map.get("map_data").and_then(Value::as_bytes).expect("map_data").to_vec();
    (0..flags.len() as u32).filter(|i| flags[*i as usize] != 0).map(|i| {
        let block = one(vec![ask(c, 0x32CB, event, &[("block_index", Value::U32(i))])]);
        let b = &block.items[0];
        assert_eq!(int(b, "block_number"), Some(i));
        ((i), (1..=8).map(|n| int(b, &format!("player_id{n}")).unwrap()).collect(), (1..=8).map(|n| int(b, &format!("win_count{n}")).unwrap()).collect())
    }).collect()
}

fn players(blocks: &[(u32, Vec<u32>, Vec<u32>)]) -> Vec<u32> {
    blocks.iter().filter(|(i, ..)| *i < 0x40).flat_map(|(_, p, _)| p.iter().copied().filter(|p| *p != 0)).collect()
}

/// Ends the duels of the round under way (the first player of each wins; with no wait between rounds the next round is
/// paired at once): how many duels it had.
fn play_round(server: &Server, event: u32) -> usize {
    let duels = server.hub.lock().unwrap().event_duels(event);
    for duel in &duels {
        let winner = server.hub.lock().unwrap().match_players(*duel).map(|p| p[0]);
        server.hub.lock().unwrap().end_match(*duel, winner);
    }
    duels.len()
}

/// Plays rounds until the Championship is over; the duels of each.
fn play_out(server: &Server, event: u32) -> Vec<usize> {
    let mut rounds = vec![];
    loop {
        match play_round(server, event) {
            0 => return rounds,
            n => rounds.push(n),
        }
    }
}

#[test]
fn without_a_championship_the_list_has_no_event_type_10_item() {
    // 0x4CBD70 keeps the first item with event_type 10 and status 12; none is the announcer's "not started yet".
    let server = Server::new();
    let mut c = server.join(1001, "Yugi");
    let list = one(c.send(req(0x177A)));
    assert!(list.items.iter().all(|i| int(i, "event_type") != Some(10)));
    // Scheduled, but its entries not open yet: still not listed.
    let new = New { name: "Later".into(), entry_open: now() + 3600, entry_close: now() + 7200, start: now() + 7300, min_duels: 0, prize: Prize::default() };
    server.hub.lock().unwrap().championships.create(new).unwrap();
    assert!(one(c.send(req(0x177A))).items.iter().all(|i| int(i, "event_type") != Some(10)));
}

#[test]
fn an_open_championship_is_listed_as_event_type_10_status_12() {
    let server = Server::new();
    let mut c = server.join(1001, "Yugi");
    let (event, _) = schedule(&server);
    assert_eq!(event, FIRST_EVENT + 1);
    let list = one(c.send(req(0x177A)));
    let item = list.items.iter().find(|i| int(i, "event_type") == Some(10)).expect("the Championship");
    assert_eq!((int(item, "status"), int(item, "event_id"), int(item, "player_num")), (Some(12), Some(event), Some(0)));
    // The Stadium (0x4B1060) keeps only items with `official` set: the Championship is not one of its events.
    assert_eq!(int(item, "official"), Some(0));
}

#[test]
fn opening_the_lobby_while_entries_are_open_enters_and_the_lobby_shows_the_wait() {
    // 0x1784 -> 0x1785 {join_id, event_id echoed}; before the draw 0x32FB has the Final's id, the Championship's id,
    // year and season, status 0 (event coming up); 0x32CA lists the entrants, 0x32CC counts down to the start.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (event, start) = schedule(&server);
    let joined = enter(&mut yugi, event);
    assert_eq!((joined.request(), int(&joined, "event_id")), (Some(0x1785), Some(event)));
    assert!(int(&joined, "join_id").is_some_and(|j| j > 0) && int(&joined, "server_id").is_some_and(|s| s > 0));
    enter(&mut kaiba, event);
    // The first one's lobby is told to ask the entries and counts again.
    let told = requests(&yugi.pushed());
    assert!(told.iter().filter(|r| **r == 0x32D4).count() >= 2, "{told:?}");
    let state = ask(&mut kaiba, 0x32C9, event, &[]);
    assert_eq!(state.request(), Some(0x32FB));
    // Status 1 ("Tournament coming up"): with 0 the scene says "not started yet" and stops before joining (0x4D1770).
    assert_eq!((int(&state, "status"), int(&state, "tournament_id"), int(&state, "championship_id")), (Some(1), Some(17), Some(1)));
    let year = int(&state, "year").unwrap();
    assert!((2020..2100).contains(&year) && int(&state, "season") == Some(1));
    let entries = ask(&mut kaiba, 0x32CA, event, &[("start_index", Value::U32(0))]);
    assert_eq!((entries.request(), int(&entries, "total_num")), (Some(0x32FD), Some(2)));
    assert_eq!(entries.items.iter().map(|i| int(i, "player_id").unwrap()).collect::<Vec<_>>(), [1001, 1002]);
    let timer = int(&ask(&mut kaiba, 0x32CC, event, &[]), "timer").unwrap();
    assert!((100..=200).contains(&timer), "{timer} s to the start ({start})");
    assert_eq!(int(&ask(&mut kaiba, 0x32CD, event, &[]), "entry_num"), Some(2));
    // No bracket before the draw: an all-zero map (the lobby needs the 0x49 bytes) and no block.
    let map = ask(&mut kaiba, 0x32D5, event, &[]);
    assert_eq!(map.get("map_data").and_then(Value::as_bytes).map(|b| (b.len(), b.iter().all(|x| *x == 0))), Some((0x49, true)));
    assert!(ask(&mut kaiba, 0x32CB, event, &[("block_index", Value::U32(0))]).items.is_empty());
    // The lobby row (Event Menu).
    assert_eq!(one(kaiba.send(req_with(0x177F, &[("event_id", Value::U32(event))]))).items[0].get("event_name").and_then(Value::as_str).as_deref(), Some("Y.O.C. Test"));
}

#[test]
fn the_minimum_duels_are_the_hosts_rule_and_default_to_none() {
    // The client's text 15302 asks for a number of duels but never checks it (and premium, which every player has here).
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    let (event, _) = schedule_with(&server, 3, Prize::default());
    let refused = enter(&mut yugi, event);
    assert_eq!(refused.request(), Some(0x1786));
    assert!(server.hub.lock().unwrap().championships.get(1).unwrap().entrants.is_empty());
    let line = yugi.pushed().into_iter().filter(|m| m.request() == Some(0x2455)).filter_map(|m| m.items.first()?.get("chat_str")?.as_str().map(|s| s.to_string())).next();
    assert!(line.is_some_and(|l| l.contains("at least 3 duels")));
    // Three duels played (any opponent) and the way in is open.
    for n in 1..=3 {
        let played = Ended { player_id: 1001, opponent: 120, npc: true, duel_id: n, match_type: 1, result: 1, replay: None, stats: None, start_lp: 8000 };
        server.hub.lock().unwrap().finish_duel(&played);
    }
    assert_eq!(enter(&mut yugi, event).request(), Some(0x1785));
    assert_eq!(server.hub.lock().unwrap().championships.get(1).unwrap().entrants.len(), 1);
    // A player after the window looks in without entering.
    let mut late = server.join(1003, "Joey");
    server.hub.lock().unwrap().championships.edit(1, &yo3_server::championship::Changes { entry_close: Some(now() - 10), entry_open: Some(now() - 100), start: Some(now() + 50), ..Default::default() }).unwrap();
    assert_eq!(enter(&mut late, event).request(), Some(0x1785));
    assert_eq!(server.hub.lock().unwrap().championships.get(1).unwrap().entrants.len(), 1);
}

#[test]
fn fewer_than_two_entrants_cancel_it_with_a_dmail() {
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    let (event, start) = schedule(&server);
    enter(&mut yugi, event);
    yugi.pushed();
    tick(&server, start);
    assert_eq!(server.hub.lock().unwrap().championships.get(1).unwrap().state, State::Cancelled);
    assert!(requests(&yugi.pushed()).contains(&0x2581), "a new D-Mail");
    // A cancelled Championship leaves the map's list.
    assert!(one(yugi.send(req(0x177A))).items.iter().all(|i| int(i, "event_type") != Some(10)));
}

#[test]
fn the_draw_announces_then_draws_and_the_bracket_fits_the_entrants_with_byes() {
    // 0x32D0 {phase, status 1} shows "Selecting players for entry...", 0x32D0 status 0 closes it, 0x32D1 {lot} says in or
    // out. Three entrants fill 3 of the Final's 16 seats; the empty seats are byes (player_id 0).
    let server = Server::new();
    let mut c: Vec<Client> = [(1001, "Yugi"), (1002, "Kaiba"), (1003, "Joey")].iter().map(|(id, n)| server.join(*id, n)).collect();
    let (event, start) = schedule(&server);
    for client in &mut c { enter(client, event); }
    for client in &mut c { client.pushed(); }
    // The draw is announced and, the delay being up, made.
    server.hub.lock().unwrap().set_draw_delay(std::time::Duration::from_secs(5));
    let t = Instant::now();
    server.hub.lock().unwrap().tick_championships(start + 1, t);
    let announced = c[0].pushed();
    assert_eq!(requests(&announced), [0x32D0]);
    assert_eq!((int(&announced[0], "status"), int(&announced[0], "phase")), (Some(1), Some(1)));
    assert_eq!(server.hub.lock().unwrap().championships.get(1).unwrap().state, State::Scheduled);
    server.hub.lock().unwrap().tick_championships(start + 2, t + std::time::Duration::from_secs(5));
    let drawn = c[0].pushed();
    let reqs = requests(&drawn);
    assert_eq!(reqs[..2], [0x32D0, 0x32D1], "{reqs:?}");
    assert_eq!(int(&drawn[0], "status"), Some(0));
    assert_eq!(int(&drawn[1], "lot"), Some(1));
    // The Final: 16 seats = blocks 0 and 1 and the final block 0x40; three of the seats hold the players.
    let state = ask(&mut c[2], 0x32C9, event, &[]);
    assert_eq!((int(&state, "tournament_id"), int(&state, "championship_id")), (Some(17), Some(1)));
    let shown = blocks(&mut c[2], event);
    assert_eq!(shown.iter().map(|b| b.0).collect::<Vec<_>>(), [0, 1, 0x40]);
    let mut seated = players(&shown);
    seated.sort();
    assert_eq!(seated, [1001, 1002, 1003]);
    // 0x32CA lists the three (the empty seats are not entrants), the alive/entry counts say 3.
    let entries = ask(&mut c[2], 0x32CA, event, &[("start_index", Value::U32(0))]);
    assert_eq!(int(&entries, "total_num"), Some(3));
    assert!(entries.items.iter().all(|i| int(i, "remain") == Some(1)), "everyone is still in");
    let counts = ask(&mut c[2], 0x32CD, event, &[]);
    assert_eq!((int(&counts, "alive_num"), int(&counts, "entry_num")), (Some(3), Some(3)));
    // Three players play two duels: a bye in the first round for one of them (nobody gets two where it can be helped).
    assert_eq!(play_out(&server, event).iter().sum::<usize>(), 2);
    // It is not a user event: the Satellite Plaza's list does not show it.
    assert_eq!(int(&one(c[2].send(req_with(0x1789, &[("search_flag", Value::U32(1)), ("base_point", Value::U32(0)), ("front_num", Value::U32(30))]))), "result_number"), Some(0));
}

#[test]
fn more_than_sixteen_entrants_are_drawn_down_to_sixteen_and_the_rest_told() {
    let server = Server::new();
    let mut c: Vec<Client> = (0..18).map(|i| server.join(2000 + i, &format!("P{i}"))).collect();
    let (event, start) = schedule(&server);
    for client in &mut c { enter(client, event); client.pushed(); }
    tick(&server, start);
    let (mut lots, mut mails) = (vec![], 0);
    for client in &mut c {
        let pushed = client.pushed();
        lots.push(pushed.iter().filter(|m| m.request() == Some(0x32D1)).filter_map(|m| int(m, "lot")).next().expect("a lot each"));
        mails += requests(&pushed).iter().filter(|r| **r == 0x2581).count();
    }
    assert_eq!(lots.iter().filter(|l| **l == 1).count(), 16, "{lots:?}");
    assert_eq!(lots.iter().filter(|l| **l == 0).count(), 2, "unable to enter");
    assert_eq!(mails, 2, "a D-Mail to each who was not drawn");
    let shown = blocks(&mut c[0], event);
    assert_eq!(players(&shown).len(), 16);
}

#[test]
fn the_rounds_are_named_by_number_and_the_match_push_leaves_event_type_and_official_out() {
    // `total_round` in 0x32FB is the round's number 1-4 for the Final (0x4CE670 turns it into 1st Round, Quarter-Final,
    // Semi-Finals, Final); the banners 0x32D3 still take the name (text 0x3C05 + n: 1, 7, 8, 9). The match push's duelData
    // has neither event_type 10 nor official 1: with them the client crashed at the end of the duel (live), and the lobby
    // sets the duel's mode 6 and music 0x22 itself (0x4D1F90).
    let server = Server::new();
    let mut c: Vec<Client> = (0..16).map(|i| server.join(3000 + i, &format!("P{i}"))).collect();
    let (event, start) = schedule(&server);
    for client in &mut c { enter(client, event); client.pushed(); }
    tick(&server, start);
    let pushed = c[0].pushed();
    let data = pushed.iter().find(|m| m.request() == Some(0x1978)).and_then(|m| m.items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("duelData"))).expect("duelData");
    assert_eq!((int(data, "event_type"), int(data, "official"), int(data, "lp_start")), (None, None, Some(8000)));
    let banner = pushed.iter().find(|m| m.request() == Some(0x32D3)).expect("banner");
    assert_eq!((int(banner, "status"), int(banner, "total_round")), (Some(1), Some(1)));
    // 16 players: eight duels in the first round, nobody on a bye.
    let state = ask(&mut c[0], 0x32C9, event, &[]);
    assert_eq!((int(&state, "status"), int(&state, "total_round"), int(&state, "round")), (Some(6), Some(1), Some(1)));
    assert_eq!(server.hub.lock().unwrap().event_duels(event).len(), 8);
    // Played out: 8 + 4 + 2 + 1 duels in four rounds.
    assert_eq!(play_out(&server, event), [8, 4, 2, 1]);
    let state = ask(&mut c[0], 0x32C9, event, &[]);
    assert_eq!((int(&state, "status"), int(&state, "total_round")), (Some(8), Some(4)));
    assert!(int(&state, "winner").is_some_and(|w| w >= 3000));
    assert!(int(&state, "second").is_some_and(|w| w >= 3000));
}

#[test]
fn the_first_round_waits_after_the_draw_as_the_later_ones_do() {
    // The lot's dialog (0x32D1) and the bracket can be read before Select Deck takes the screen: the lobby says "Match-ups
    // coming up" (status 4) with the countdown (0x3301), and the first round is paired when it is up.
    let server = Server::new();
    server.hub.lock().unwrap().set_round_delay(std::time::Duration::from_secs(20));
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (event, start) = schedule(&server);
    enter(&mut yugi, event);
    enter(&mut kaiba, event);
    yugi.pushed();
    tick(&server, start);
    let pushed = yugi.pushed();
    assert!(requests(&pushed).contains(&0x32D1) && !requests(&pushed).contains(&0x1978), "{:?}", requests(&pushed));
    assert_eq!(int(pushed.iter().find(|m| m.request() == Some(0x3301)).expect("timer"), "timer"), Some(20));
    let state = ask(&mut kaiba, 0x32C9, event, &[]);
    assert_eq!((int(&state, "status"), int(&state, "total_round")), (Some(4), Some(4)), "two players: the byes go by and the Final is to come");
    assert!(server.hub.lock().unwrap().event_duels(event).is_empty());
    server.hub.lock().unwrap().tick_events(Instant::now() + std::time::Duration::from_secs(21));
    assert_eq!(server.hub.lock().unwrap().event_duels(event).len(), 1);
    assert!(requests(&yugi.pushed()).contains(&0x1978));
}

#[test]
fn a_seat_keeps_its_players_face_when_they_go_offline() {
    // The client draws a seat blank without the player's avatar (0x4CEBE0): a player gone offline keeps the face they had
    // at the draw in 0x32CA.
    let server = Server::new();
    let (mut yugi, kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (event, start) = schedule(&server);
    let mut kaiba = kaiba;
    enter(&mut yugi, event);
    enter(&mut kaiba, event);
    tick(&server, start);
    let faces = |c: &mut Client| { let mut v = ask(c, 0x32CA, event, &[("start_index", Value::U32(0))]).items.iter().map(|i| (int(i, "player_id").unwrap(), i.get("avatar1").is_some())).collect::<Vec<_>>(); v.sort(); v };
    assert_eq!(faces(&mut yugi), [(1001, true), (1002, true)]);
    drop(kaiba);
    assert_eq!(faces(&mut yugi), [(1001, true), (1002, true)], "Kaiba's face stays");
}

#[test]
fn byes_carry_two_players_through_to_a_final_and_three_play_two_duels() {
    // Two entrants meet in the Final: the bracket skips the rounds of byes (counted as wins) and has one duel.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (event, start) = schedule(&server);
    enter(&mut yugi, event);
    enter(&mut kaiba, event);
    tick(&server, start);
    assert_eq!(server.hub.lock().unwrap().event_duels(event).len(), 1, "one duel, in the Final");
    let state = ask(&mut yugi, 0x32C9, event, &[]);
    assert_eq!(int(&state, "total_round"), Some(4), "the Final");
    let shown = blocks(&mut yugi, event);
    // Both are in the final block (0x40) already: three byes each.
    let last = shown.iter().find(|b| b.0 == 0x40).unwrap();
    assert_eq!(last.1.iter().filter(|p| **p != 0).count(), 2);
    assert_eq!(play_out(&server, event), [1]);
    assert_eq!(server.hub.lock().unwrap().championships.get(1).unwrap().state, State::Finished);
}

#[test]
fn a_finished_championship_is_recorded_for_duel_log_and_its_bracket_stays_readable() {
    // 0x1793 -> 0x177B items {year, season, event_id}; the bracket requests still answer for it.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    assert_eq!(int(&one(yugi.send(req(0x1793))), "result_number"), Some(0), "none before");
    let (event, start) = schedule(&server);
    enter(&mut yugi, event);
    enter(&mut kaiba, event);
    tick(&server, start);
    play_out(&server, event);
    let past = one(yugi.send(req(0x1793)));
    assert_eq!((past.request(), int(&past, "result_number")), (Some(0x177B), Some(1)));
    let item = &past.items[0];
    assert_eq!((int(item, "season"), int(item, "event_id")), (Some(1), Some(event)));
    assert!(int(item, "year").is_some_and(|y| y >= 2020));
    let c = server.hub.lock().unwrap().championships.get(1).cloned().unwrap();
    assert_eq!(c.state, State::Finished);
    assert!(c.winner.is_some() && c.second.is_some() && c.bracket.is_some());
    // The state of the lobby after the end: Event Completed with the winner.
    let state = ask(&mut kaiba, 0x32C9, event, &[]);
    assert_eq!((int(&state, "status"), Some(c.winner.as_ref().unwrap().0)), (Some(8), int(&state, "winner")));
    // It stays on the map's list for a day so the scene can show the result.
    assert!(one(kaiba.send(req(0x177A))).items.iter().any(|i| int(i, "event_type") == Some(10)));
}

#[test]
fn titles_follow_the_rounds_gone_through_and_the_prize_is_paid() {
    // QUALIFIER when drawn; Q-FINALIST, S-FINALIST, FINALIST for the rounds gone through (a bye counts); WINNER for the
    // winner. The host's BP prize goes to the winner (and the finalists').
    let titles = TitleData {
        by_name: [("Y.O.C. QUALIFIER", 301), ("Y.O.C. Q-FINALIST", 302), ("Y.O.C. S-FINALIST", 303), ("Y.O.C. FINALIST", 304), ("Y.O.C WINNER", 305)]
            .iter().map(|(n, id)| (n.to_string(), *id)).collect(),
        ..TitleData::default()
    };
    let dir = std::env::temp_dir().join(format!("yo3-champ-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let directory = Arc::new(Mutex::new(Directory::open(vec!["Yugi".into()], &dir.join("accounts.json")).unwrap()));
    let server = Server::new();
    server.hub.lock().unwrap().set_directory(directory.clone());
    server.hub.lock().unwrap().set_titles(Arc::new(titles));
    let mut c: Vec<Client> = (1..=3).map(|i| server.join_from(&directory, IpAddr::V4(Ipv4Addr::new(10, 0, 0, i)))).collect();
    let ids: Vec<u32> = c.iter_mut().map(|c| int(&one(c.send(req(0x2CEE))).items[0], "player_id").unwrap()).collect();
    let prize = Prize { participation_bp: 10, finalist_bp: 100, winner_bp: 1000 };
    let (event, start) = schedule_with(&server, 0, prize);
    for client in &mut c { enter(client, event); }
    let before: Vec<u32> = ids.iter().map(|id| directory.lock().unwrap().player(*id).unwrap().items.len() as u32).collect();
    tick(&server, start);
    for id in &ids {
        assert!(directory.lock().unwrap().player(*id).unwrap().items.contains(&301), "QUALIFIER when drawn");
    }
    play_out(&server, event);
    let c1 = server.hub.lock().unwrap().championships.get(1).cloned().unwrap();
    let winner = c1.winner.as_ref().unwrap().0;
    let owned = |id: u32| directory.lock().unwrap().player(id).unwrap().items.clone();
    assert!(owned(winner).contains(&305) && owned(winner).contains(&304), "WINNER and FINALIST");
    let second = c1.second.as_ref().unwrap().0;
    assert!(owned(second).contains(&304) && !owned(second).contains(&305), "FINALIST only");
    for id in &ids {
        assert!(owned(*id).contains(&301) && owned(*id).len() as u32 >= before[ids.iter().position(|x| x == id).unwrap()]);
    }
    // The prize: the winner got 10 + 100 + 1000 BP, the loser of the final 10 + 100, and a D-Mail each.
    let mut bp_pushed = |at: usize| c[at].pushed().iter().filter_map(|m| int(m, "revival_bp")).next_back();
    let (w, s) = (ids.iter().position(|i| *i == winner).unwrap(), ids.iter().position(|i| *i == second).unwrap());
    let (won, lost) = (bp_pushed(w).expect("the winner's BP"), bp_pushed(s).expect("the runner-up's BP"));
    assert_eq!(won - lost, 1000, "{won} {lost}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_restart_ends_a_running_championship_and_brings_the_finished_brackets_back() {
    let dir = std::env::temp_dir().join(format!("yo3-champ-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let server = Server::new();
    server.hub.lock().unwrap().keep_championships(&dir);
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (event, start) = schedule(&server);
    enter(&mut yugi, event);
    enter(&mut kaiba, event);
    tick(&server, start);
    assert_eq!(server.hub.lock().unwrap().championships.get(1).unwrap().state, State::Running);
    // The server restarts mid-bracket: it ends, with no winner, and its entries are kept.
    let mut again = yo3_server::front::Hub::default();
    again.keep_championships(&dir);
    let c = again.championships.get(1).unwrap();
    assert_eq!((c.state, c.entrants.len(), c.winner.is_none()), (State::Finished, 2, true));
    assert_eq!(again.championships.past().len(), 1);
    // A finished one comes back with its bracket and winner.
    play_out(&server, event);
    let mut third = yo3_server::front::Hub::default();
    third.keep_championships(&dir);
    let c = third.championships.get(1).unwrap();
    assert!(c.winner.is_some() && c.bracket.is_some());
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_player_leaving_the_janken_loses_the_round_to_the_one_who_stayed() {
    // A leaver before the duel began used to end the match with no winner, which put both players out of the bracket.
    let stats = |cid, atk| CardStats { cid, atk, def: 0, level: 4, attribute: 1, race: 1 };
    let cards = [(1, stats(6960, 2000)), (2, stats(5004, 1900))].into_iter().collect();
    let server = Server::with_world(World { cards, ..World::new((1..=20).collect()) });
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (event, start) = schedule(&server);
    enter(&mut yugi, event);
    enter(&mut kaiba, event);
    tick(&server, start);
    let duel = server.hub.lock().unwrap().event_duels(event)[0];
    for (me, them, row) in [(1001, 1002, 1), (1002, 1001, 2)] {
        let deck = Deck { main: vec![row; 40], ..Deck::default() };
        server.hub.lock().unwrap().enter_duel(me, DuelEntry { deck, duel_id: Some(duel), opponent: Some(them), best_of: 1, ..DuelEntry::default() });
    }
    let seat = |seed| {
        let (tx, _inbox) = tokio::sync::mpsc::unbounded_channel();
        DuelSession::new(server.world.clone(), server.hub.clone(), seed).with_outbox(tx)
    };
    let (mut a, mut b) = (seat(3), seat(5));
    for (session, player) in [(&mut a, 1001), (&mut b, 1002)] {
        let m = Message::reply(0x200A).with("player_id", Value::U32(player)).with("autoSet", Value::U32(1));
        session.handle(&yo3_proto::Packet::Message(m));
    }
    drop(a);
    let c = server.hub.lock().unwrap().championships.get(1).cloned().unwrap();
    assert_eq!((c.state, c.winner.map(|w| w.0)), (State::Finished, Some(1002)), "Kaiba stayed");
    drop(b);
}
