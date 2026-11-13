//! User events: Satellite Plaza's list and Create (UserEventScene 0x4D9BB0), the lobby (TournamentScene in user mode,
//! 0x4CBD70), and the event's duels, which the server pairs round by round and starts with the match push 0x1978.
mod common;

use common::{Client, Server, one, req, req_with};
use yo3_proto::{Message, Value};

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

fn text(m: &Message, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or_default()
}

fn requests(pushed: &[Message]) -> Vec<u32> {
    pushed.iter().filter_map(Message::request).collect()
}

/// Create (0x339B -> 0x3400 {create_status 0}, then the form 0x3390 -> 0x33F4 {event_id}) and the host's entry
/// (0x3397 {event_id} -> 0x33FA); the event's id.
fn create(c: &mut Client, name: &str) -> u32 {
    let r = one(c.send(req(0x339B)));
    assert_eq!((r.request(), int(&r, "create_status")), (Some(0x3400), Some(0)));
    let form = [
        ("event_name", Value::text(name)),
        ("comment", Value::text("Friday cup")),
        ("entry_max", Value::U32(4)),
        ("player_limit", Value::U32(1)),
        ("password1", Value::text("")),
        ("match_type", Value::U32(1)),
        ("regulation_id", Value::U32(1)),
        ("duel_lp_start", Value::U32(8000)),
        ("duel_time_turn", Value::U32(180)),
        ("duel_time_total", Value::U32(600)),
    ];
    let r = one(c.send(req_with(0x3390, &form)));
    assert_eq!(r.request(), Some(0x33F4));
    let event_id = int(&r, "event_id").expect("event_id");
    assert_eq!(one(c.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33FA));
    event_id
}

fn search(c: &mut Client) -> Message {
    let fields = [("search_flag", Value::U32(1)), ("base_point", Value::U32(0)), ("front_num", Value::U32(30))];
    one(c.send(req_with(0x1789, &fields)))
}

#[test]
fn a_created_event_is_listed_with_its_host_entered() {
    // 0x339A -> 0x33FE {remain, event_id}: the player's own event (kept while remain isn't 0; it turns Create off).
    // The list 0x1789 -> 0x177B {result_number} with rows {event_id, event_name, status 12 open, join_flag, password1,
    // prize_type, entry_max, entry_num, match_type, regulation_id, ...} (0x4D66A0, the detail view 0x4D75F0).
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    assert_eq!(int(&one(yugi.send(req(0x339A))), "remain"), Some(0));
    let event_id = create(&mut yugi, "Duelist Cup");
    let own = one(yugi.send(req(0x339A)));
    assert_eq!((int(&own, "remain"), int(&own, "event_id")), (Some(1), Some(event_id)));
    let list = search(&mut server.join(1002, "Kaiba"));
    assert_eq!(int(&list, "result_number"), Some(1));
    let row = &list.items[0];
    assert_eq!((int(row, "event_id"), text(row, "event_name"), int(row, "status")), (Some(event_id), "Duelist Cup".into(), Some(12)));
    assert_eq!((int(row, "entry_num"), int(row, "entry_max"), int(row, "match_type")), (Some(1), Some(4), Some(1)));
    // join_flag 0: open to all (live: 1 crossed the row out and the details offered no Participate).
    assert_eq!(int(row, "join_flag"), Some(0));
}

#[test]
fn the_lobby_names_the_event_and_its_entrants() {
    // TournamentScene in user mode asks 0x177F {event_id} -> 0x1780 item[0] {official 0, event_name, player_id: the
    // host, which it compares with its own}, and 0x32CA {event_id, start_index} -> 0x32FD {total_num, items
    // {player_id}}.
    let server = Server::new();
    let mut yugi = server.join(1001, "Yugi");
    let event_id = create(&mut yugi, "Duelist Cup");
    let info = one(yugi.send(req_with(0x177F, &[("event_id", Value::U32(event_id))])));
    assert_eq!(info.request(), Some(0x1780));
    let item = &info.items[0];
    assert_eq!((int(item, "official"), text(item, "event_name"), int(item, "player_id")), (Some(0), "Duelist Cup".into(), Some(1001)));
    let entries = one(yugi.send(req_with(0x32CA, &[("event_id", Value::U32(event_id)), ("start_index", Value::U32(0))])));
    assert_eq!((entries.request(), int(&entries, "total_num")), (Some(0x32FD), Some(1)));
    assert_eq!(int(&entries.items[0], "player_id"), Some(1001));
    // The lobby asks with the event_id the global dispatcher kept from 0x1785 (live: 0 until it was echoed), then for
    // its state (0x4CC5B0): 0x32CC -> 0x3301 {timer}, 0x32CD -> 0x3303 {alive_num, entry_num, watch_num}, 0x32CE ->
    // 0x3305 {phase_block1-3, remain}, 0x32C9 -> 0x32FB {event_id, status, phase, round, total_round, winner...}.
    let joined = one(yugi.send(req_with(0x1784, &[("event_id", Value::U32(event_id))])));
    assert_eq!(int(&joined, "event_id"), Some(event_id));
    // Only a user event's id goes back, and theirs start above the client's own (1, 2): echoing Auto Duel's 2 made
    // both clients queue again after their match (live).
    assert!(event_id >= 100);
    let auto = one(yugi.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
    assert_eq!(auto.get("event_id"), None);
    let ask = |c: &mut Client, id: u32| one(c.send(req_with(id, &[("event_id", Value::U32(event_id))])));
    assert_eq!(ask(&mut yugi, 0x32CC).request(), Some(0x3301));
    let counts = ask(&mut yugi, 0x32CD);
    assert_eq!((counts.request(), int(&counts, "entry_num")), (Some(0x3303), Some(1)));
    assert_eq!(ask(&mut yugi, 0x32CE).request(), Some(0x3305));
    // 0x32D5 -> 0x3309 {map_data}: 0x49 raw bytes the lobby copies unchecked (0x4D0F40; live, a crash without them).
    let map = ask(&mut yugi, 0x32D5);
    assert!(matches!(map.get("map_data"), Some(Value::Bin(b)) if b.len() == 0x49));
    let state = ask(&mut yugi, 0x32C9);
    // Its status is the lobby's state (+0x2F8), not the list's: above 1 the host's Start and Cancel go (0x4CE670;
    // live, 12 left only Close in the Event Menu).
    assert_eq!((state.request(), int(&state, "event_id"), int(&state, "status")), (Some(0x32FB), Some(event_id), Some(0)));
}

#[test]
fn players_enter_and_leave_and_the_lobby_hears_of_it() {
    // An entry (0x3397) or a leave (0x178E {event_id} -> 0x178F) reaches the others as 0x3399 {entry_name, entry_type:
    // 1 joined "… has entered", else left}.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    yugi.pushed();
    assert_eq!(one(kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33FA));
    // Each is followed by the lobby's refresh (0x32D4 flags 2 entries, 3 bracket, 5 counts).
    let pushed = yugi.pushed();
    assert_eq!(requests(&pushed), [0x3399, 0x32D4, 0x32D4, 0x32D4]);
    let joined = pushed[0].clone();
    assert_eq!((joined.request(), text(&joined, "entry_name"), int(&joined, "entry_type")), (Some(0x3399), "Kaiba".into(), Some(1)));
    assert_eq!(one(kaiba.send(req_with(0x178E, &[("event_id", Value::U32(event_id))]))).request(), Some(0x178F));
    let left = yugi.pushed().remove(0);
    assert_eq!((left.request(), int(&left, "entry_type")), (Some(0x3399), Some(2)));
    assert_eq!(int(&search(&mut kaiba).items[0], "entry_num"), Some(1));
}

#[test]
fn starting_pairs_the_entrants_and_the_winners_go_on() {
    // The host's start 0x3395 {event_id} -> 0x33F8; everyone gets 0x3396 ("The event has started"), and each pair the
    // match push 0x1978 {vsPlayerData, duelData {duel_id, the event's rule}}, which TournamentScene takes like the
    // matching agent (0x4CC60C) to open the duel. A duel's winner goes on; the last one standing ends the event.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    yugi.pushed();
    assert_eq!(one(yugi.send(req_with(0x3395, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33F8));
    let (to_yugi, to_kaiba) = (yugi.pushed(), kaiba.pushed());
    // Then the round's banner and the lobby's refresh (see the lobby test).
    assert_eq!(requests(&to_yugi), [0x3396, 0x1978, 0x32D3, 0x32D4, 0x32D4, 0x32D4, 0x32D4]);
    assert_eq!(requests(&to_kaiba)[..3], [0x3396, 0x1978, 0x32D3]);
    let data = to_kaiba[1].items.iter().find(|i| text(i, "name") == "duelData").expect("duelData");
    let duel_id = int(data, "duel_id").expect("duel_id");
    assert_eq!((int(data, "match_type"), int(data, "lp_start")), (Some(1), Some(8000)));
    assert_eq!(int(&search(&mut kaiba).items[0], "status"), Some(13));
    // Live: both picked their decks, Yugi won the duel, and Next took both back to the lobby.
    server.hub.lock().unwrap().end_match(duel_id, Some(1002));
    let state = one(kaiba.send(req_with(0x32C9, &[("event_id", Value::U32(event_id))])));
    assert_eq!((int(&state, "status"), int(&state, "winner"), text(&state, "winner_name")), (Some(8), Some(1002), "Kaiba".into()));
    // A finished event leaves the list, and its host may create another.
    assert_eq!(int(&search(&mut kaiba), "result_number"), Some(0));
    assert_eq!(int(&one(yugi.send(req(0x339A))), "remain"), Some(0));
}

#[test]
fn the_host_cancels_the_event() {
    // 0x3393 {event_id} -> 0x33F6; the others get 0x3394 ("The event has been cancelled", the lobby closes).
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    assert_eq!(one(yugi.send(req_with(0x3393, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33F6));
    assert_eq!(requests(&kaiba.pushed()), [0x3394]);
    assert_eq!(int(&search(&mut kaiba), "result_number"), Some(0));
}

fn ask(c: &mut Client, id: u32, event_id: u32, extra: &[(&str, Value)]) -> Message {
    let mut fields = vec![("event_id", Value::U32(event_id))];
    fields.extend(extra.iter().cloned());
    one(c.send(req_with(id, &fields)))
}

/// An event with Yugi (host), Kaiba and Joey entered and started; the three clients, and the event's id.
fn started_three(server: &Server) -> (Client, Client, Client, u32) {
    let (mut yugi, mut kaiba, mut joey) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"), server.join(1003, "Joey"));
    let event_id = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    joey.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    assert_eq!(one(yugi.send(req_with(0x3395, &[("event_id", Value::U32(event_id))]))).request(), Some(0x33F8));
    (yugi, kaiba, joey, event_id)
}

#[test]
fn the_bracket_names_its_entrants_and_their_wins() {
    // TournamentScene's loaders: the entries 0x32CA -> 0x32FD {total_num, items {player_id, player_name, remain}}
    // (0x4CEBE0 looks each seat's player up there), the map 0x32D5 -> 0x3309 {map_data} (a flag per block, 0x4CF4A0
    // shows block 0 first when it is set) and the block 0x32CB {block_index} -> 0x32FF with an item {block_number,
    // start_level, player_id1-8, win_count1-8} (0x4D10F0 matches `block_number`; 0x4CEBE0 reads the seats).
    let server = Server::new();
    let (mut yugi, mut kaiba, _joey, event_id) = started_three(&server);
    let entries = ask(&mut yugi, 0x32CA, event_id, &[("start_index", Value::U32(0))]);
    assert_eq!(int(&entries, "total_num"), Some(3));
    let names: Vec<String> = entries.items.iter().map(|i| text(i, "player_name")).collect();
    assert_eq!(names, ["Yugi", "Kaiba", "Joey"]);
    let map = ask(&mut yugi, 0x32D5, event_id, &[]);
    let map = map.get("map_data").and_then(Value::as_bytes).expect("map_data").to_vec();
    assert_eq!((map.len(), map[0], map[1], map[0x40]), (0x49, 1, 0, 0));
    let block = ask(&mut yugi, 0x32CB, event_id, &[("block_index", Value::U32(0))]);
    let seat = &block.items[0];
    assert_eq!((int(seat, "block_number"), int(seat, "start_level")), (Some(0), Some(0)));
    let seats: Vec<_> = (1..=8).map(|n| int(seat, &format!("player_id{n}")).unwrap()).collect();
    assert_eq!(seats, [1001, 1002, 1003, 0, 0, 0, 0, 0]);
    // No wins yet: Joey (odd one out) went through the first round, which counts.
    let wins: Vec<_> = (1..=8).map(|n| int(seat, &format!("win_count{n}")).unwrap()).collect();
    assert_eq!(wins, [0, 0, 1, 0, 0, 0, 0, 0]);
    // A block that doesn't exist has no item (the lobby then clears the bracket).
    assert!(ask(&mut yugi, 0x32CB, event_id, &[("block_index", Value::U32(5))]).items.is_empty());
    // Kaiba wins the first duel: he has won a round as well.
    let duel_id = {
        let hub = server.hub.lock().unwrap();
        hub.event_duels(event_id)[0]
    };
    server.hub.lock().unwrap().end_match(duel_id, Some(1002));
    let block = ask(&mut kaiba, 0x32CB, event_id, &[("block_index", Value::U32(0))]);
    let wins: Vec<_> = (1..=3).map(|n| int(&block.items[0], &format!("win_count{n}")).unwrap()).collect();
    assert_eq!(wins, [0, 1, 1]);
}

#[test]
fn the_bracket_entries_carry_each_players_flag() {
    // 0x4CEBE0 reads each seat's `country` from the entries (0x32FD) as well as the name and face: the flag of the
    // account, or the one its language implies (players without a setting here: none sent, Germany shown).
    let server = Server::new();
    let (mut yugi, _kaiba, _joey, event_id) = started_three(&server);
    let entries = ask(&mut yugi, 0x32CA, event_id, &[("start_index", Value::U32(0))]);
    assert!(entries.items.iter().all(|i| text(i, "country") == "GB"), "{entries:?}");
}

#[test]
fn the_lobby_shows_the_round_and_the_status() {
    // 0x32FB {status, round, total_round}: status 6 "Prepare to Duel" (0x3BDF) once the round is paired, 7 "Duel in
    // Progress" when a player sat at a duel's table, 8 "Event Completed" at the end (0x4CE670); total_round names the
    // round for the lobby: 8 "Semi-Finals" with two rounds to go (0x3BCD + n, 0x4CE670), 9 "Final" at the last.
    let server = Server::new();
    let (mut yugi, mut kaiba, _joey, event_id) = started_three(&server);
    let state = ask(&mut kaiba, 0x32C9, event_id, &[]);
    assert_eq!((int(&state, "status"), int(&state, "total_round")), (Some(6), Some(8)));
    let duel_id = server.hub.lock().unwrap().event_duels(event_id)[0];
    let before = yugi.pushed();
    kaiba.pushed();
    server.hub.lock().unwrap().duel_began(duel_id);
    // A duel's start tells the lobbies to ask for the state (flag 6).
    assert_eq!(requests(&yugi.pushed()), [0x32D4]);
    assert_eq!(int(&ask(&mut kaiba, 0x32C9, event_id, &[]), "status"), Some(7));
    assert_eq!(int(&ask(&mut kaiba, 0x32CD, event_id, &[]), "alive_num"), Some(3));
    // The map's current blocks (0x3305): block 0 is in play, the others -1.
    let phase = ask(&mut kaiba, 0x32CE, event_id, &[]);
    assert_eq!((int(&phase, "phase_block1"), int(&phase, "phase_block2")), (Some(0), Some(u32::MAX)));
    // The banner pushes: the round starting (0x32D3 status 1, total_round the round's name), then the lobby is told to
    // ask again (0x32D4 flags 6 state, 5 counts, 2 entries, 3 bracket, 0x4CC5B0).
    let pushed = before;
    let banner = pushed.iter().find(|m| m.request() == Some(0x32D3)).expect("round banner");
    assert_eq!((int(banner, "status"), int(banner, "total_round")), (Some(1), Some(8)));
    let flags: Vec<_> = pushed.iter().filter(|m| m.request() == Some(0x32D4)).filter_map(|m| int(m, "flag")).collect();
    assert!(flags.ends_with(&[6, 5, 2, 3]), "{flags:?}");
    // Kaiba beats Yugi; the second round (the final: Kaiba against Joey) starts after the first's banner "finished".
    server.hub.lock().unwrap().end_match(duel_id, Some(1002));
    let pushed = kaiba.pushed();
    let banners: Vec<_> = pushed.iter().filter(|m| m.request() == Some(0x32D3)).map(|m| (int(m, "status"), int(m, "total_round"))).collect();
    assert_eq!(banners, [(Some(0), Some(8)), (Some(1), Some(9))]);
    let state = ask(&mut kaiba, 0x32C9, event_id, &[]);
    assert_eq!((int(&state, "status"), int(&state, "total_round")), (Some(6), Some(9)));
    // Joey beats Kaiba in the final: winner and runner-up (0x3307, 0x32FB), status 8.
    let final_duel = server.hub.lock().unwrap().event_duels(event_id)[0];
    server.hub.lock().unwrap().end_match(final_duel, Some(1003));
    let pushed = kaiba.pushed();
    let done = pushed.iter().find(|m| m.request() == Some(0x3307)).expect("winner push");
    assert_eq!((int(done, "winner"), text(done, "winner_name"), int(done, "second"), text(done, "second_name")), (Some(1003), "Joey".into(), Some(1002), "Kaiba".into()));
    let state = ask(&mut kaiba, 0x32C9, event_id, &[]);
    assert_eq!((int(&state, "status"), int(&state, "winner"), int(&state, "second")), (Some(8), Some(1003), Some(1002)));
    // The champion went three rounds in a 3-seat bracket... two (a bye and the final).
    let block = ask(&mut kaiba, 0x32CB, event_id, &[("block_index", Value::U32(0))]);
    assert_eq!(int(&block.items[0], "win_count3"), Some(2));
}

#[test]
fn open_events_are_kept_across_restarts() {
    // The data directory keeps the open events (events.json) with their entrants and the id counter; a running
    // event can't come back (its duels died with the server), so it ends there.
    let dir = std::env::temp_dir().join(format!("yo3-events-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("events.json");
    let server = Server::new();
    server.hub.lock().unwrap().keep_events(&path);
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let open = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(open))]));
    let mut joey = server.join(1003, "Joey");
    let running = create(&mut joey, "Joey's Cup");
    server.join(1004, "Mai").send(req_with(0x3397, &[("event_id", Value::U32(running))]));
    joey.send(req_with(0x3395, &[("event_id", Value::U32(running))]));

    let server = Server::new();
    server.hub.lock().unwrap().keep_events(&path);
    let mut yugi = server.join(1001, "Yugi");
    let list = search(&mut yugi);
    assert_eq!(int(&list, "result_number"), Some(1));
    let row = &list.items[0];
    assert_eq!((int(row, "event_id"), text(row, "event_name"), int(row, "entry_num"), int(row, "status")), (Some(open), "Duelist Cup".into(), Some(2), Some(12)));
    assert_eq!((int(row, "entry_max"), int(row, "duel_lp_start"), text(row, "comment")), (Some(4), Some(8000), "Friday cup".into()));
    // The host still hosts it, and a new event's id follows on.
    assert_eq!(int(&one(yugi.send(req(0x339A))), "event_id"), Some(open));
    let mut mai = server.join(1004, "Mai");
    assert!(int(&one(mai.send(req(0x339B))), "create_status") == Some(0));
    let next = create(&mut mai, "Mai's Cup");
    assert!(next > running);
    // The host can start it with the entrants kept.
    let mut kaiba = server.join(1002, "Kaiba");
    assert_eq!(one(yugi.send(req_with(0x3395, &[("event_id", Value::U32(open))]))).request(), Some(0x33F8));
    assert_eq!(requests(&kaiba.pushed())[..2], [0x3396, 0x1978]);
}

#[test]
fn the_lobby_waits_between_rounds() {
    // Between rounds the winners are still on their result screens: the lobby says "Match-ups coming up" (status 4,
    // 0x4CE670) with the time to the next round (0x3301 {timer}, 0x32CC; 0x4CEAD0 shows it), and the next round's
    // banner and match push follow when the time is up (0x3302 clears the timer).
    let server = Server::new();
    let delay = std::time::Duration::from_secs(20);
    server.hub.lock().unwrap().set_round_delay(delay);
    let (mut yugi, mut kaiba, _joey, event_id) = started_three(&server);
    let duel_id = server.hub.lock().unwrap().event_duels(event_id)[0];
    yugi.pushed();
    kaiba.pushed();
    server.hub.lock().unwrap().end_match(duel_id, Some(1001));
    let pushed = yugi.pushed();
    let timer = pushed.iter().find(|m| m.request() == Some(0x3301)).expect("timer");
    assert_eq!(int(timer, "timer"), Some(20));
    assert!(!requests(&pushed).contains(&0x1978));
    let state = ask(&mut kaiba, 0x32C9, event_id, &[]);
    // The round to come is named: the final.
    assert_eq!((int(&state, "status"), int(&state, "total_round")), (Some(4), Some(9)));
    assert!(int(&ask(&mut kaiba, 0x32CC, event_id, &[]), "timer").is_some_and(|t| (1..=20).contains(&t)));
    // Nothing happens early; when the time is up the final is paired.
    server.hub.lock().unwrap().tick_events(std::time::Instant::now());
    assert!(!requests(&yugi.pushed()).contains(&0x1978));
    server.hub.lock().unwrap().tick_events(std::time::Instant::now() + delay);
    let pushed = yugi.pushed();
    assert_eq!(requests(&pushed)[..3], [0x3302, 0x1978, 0x32D3]);
    assert_eq!(int(&ask(&mut kaiba, 0x32C9, event_id, &[]), "status"), Some(6));
}

#[test]
fn a_round_match_nobody_joins_ends_on_its_own() {
    // A match push that reaches a client on another screen (a result screen) is lost (0x1978 is taken only by the
    // matching agent or TournamentScene). A player who came waits at the table and is sent back (table::NO_SHOW); but
    // when neither came, no table existed and the round never ended. A match nobody joined ends after NO_JOIN, both out.
    let server = Server::new();
    let (mut yugi, mut kaiba) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let event_id = create(&mut yugi, "Duelist Cup");
    kaiba.send(req_with(0x3397, &[("event_id", Value::U32(event_id))]));
    yugi.send(req_with(0x3395, &[("event_id", Value::U32(event_id))]));
    let duel_id = server.hub.lock().unwrap().event_duels(event_id)[0];
    let now = std::time::Instant::now();
    server.hub.lock().unwrap().tick_matches(now + std::time::Duration::from_secs(30));
    assert!(!server.hub.lock().unwrap().ended(duel_id), "players still have time to come");
    server.hub.lock().unwrap().tick_matches(now + yo3_server::front::NO_JOIN + std::time::Duration::from_secs(1));
    assert!(server.hub.lock().unwrap().ended(duel_id));
    assert!(server.hub.lock().unwrap().event_duels(event_id).is_empty(), "the round goes on");
}
