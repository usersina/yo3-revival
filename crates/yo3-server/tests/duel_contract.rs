//! Duels. Every duel goes through DuelMatchingAgent (0x4A0B50). The NPC duel (SelectNpcScene) lists
//! the NPC roster, and once an opponent is picked (decideNpc 0x4C0C80) its duel start step
//! (0x4BE390) hands the opponent's roster item to the agent in mode 3. The agent checks the deck
//! (0x4A0E90), asks for an entry (0x1784, step 0x4A1090) and a duel (0x17AC, step 0x4A1790); after
//! 0x17AD the scene opens DuelManager (0x4BE4DA).
mod common;

use common::{Server, one, req, req_with};
use yo3_proto::{Message, Packet, TAG_MESSAGE, Value};
use yo3_server::front::World;

fn world() -> World {
    World { npcs: (120..126).collect(), duel_server: ("10.0.0.5".into(), 4321), ..World::new((1..=20).collect()) }
}

#[test]
fn the_npc_roster_lists_every_level_of_every_npc() {
    // 0x4BEB40 walks the 0x2711 items three at a time: the first gives npc_type (the tab, 0-2) and
    // npc_id; each of the three gives need_ex, the experience (npcbin entry 3) its level needs.
    // Picking level L looks up the item whose npc_id is the first id + L (0x4BE454), and that item
    // becomes the agent's record and the opponent's PlayerData. NPC names are text 0x5208 + 2 *
    // ((npc_id - 120) / 3) (0x4BFE34), so the first id of a triple is 120 + 3k.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let r = one(c.send(req(0x2710)));
    assert_eq!(r.request(), Some(0x2711));
    let ids: Vec<_> = r.items.iter().map(|i| i.get("npc_id").and_then(Value::as_u32)).collect();
    assert_eq!(ids, (120..126).map(Some).collect::<Vec<_>>());
    for item in &r.items {
        // DuelManager (0x50F450) makes an NPC opponent only for npc_type 0-2.
        assert!(item.get("npc_type").and_then(Value::as_u32).is_some_and(|t| t <= 2), "{item:?}");
    }
    let need_ex: Vec<_> = r.items.iter().map(|i| i.get("need_ex").and_then(Value::as_u32).expect("need_ex")).collect();
    for levels in need_ex.chunks(3) {
        // Level j is mastered once the experience with its own id (npcbin entry 3, absent = 0) reaches its need_ex,
        // counting up from the first (0x4BEB40); the offered level is the number mastered, at most 2. A need_ex of 0
        // is mastered from the start, so every level needs some. Hovering an NPC divides by the three summed
        // (0x4BFDC4, the friendship bar per mille). 100/200/300 is a **guess**.
        assert_eq!(levels, [100, 200, 300], "{need_ex:?}");
    }
}

#[test]
fn the_npcs_are_on_their_series_tab() {
    // npc_type is the tab (kaiba_main.swf: Yu-Gi-Oh!, GX, 5D's, texts 15504-15506). The free NPCs (0x79039C: first
    // ids 120, 123, 126, 147, 150, 153, 171, 174, 177) are three per series, so NPCs 0-8 are tab 0, 9-16 tab 1 and
    // 17-25 tab 2 (**guess** from the names).
    let world = World { npcs: (120..198).collect(), ..world() };
    let mut c = Server::with_world(world).join(1001, "Yugi");
    let r = one(c.send(req(0x2710)));
    let tab = |id: u32| r.items.iter().find(|i| i.get("npc_id") == Some(&Value::U32(id))).and_then(|i| i.get("npc_type")).cloned();
    for (id, expected) in [(120, 0), (144, 0), (147, 1), (168, 1), (171, 2), (195, 2)] {
        assert_eq!(tab(id), Some(Value::U32(expected)), "npc_id {id}");
    }
}

#[test]
fn without_game_data_the_roster_is_empty() {
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x2710)));
    assert_eq!(r.request(), Some(0x2711));
    assert!(r.items.is_empty());
}

#[test]
fn a_duel_entry_gets_a_join_id() {
    // 0x1785 (0x4A1090): join_id is kept (agent+0x58) and sent back in 0x17AC. The global
    // dispatcher (0x4F4AE7) also stores event_id, join_id, server_id and event_score, all ints.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let r = one(c.send(req_with(0x1784, &[("event_id", Value::U32(0))])));
    assert_eq!(r.request(), Some(0x1785));
    assert!(r.get("join_id").and_then(Value::as_u32).is_some_and(|id| id > 0));
}

#[test]
fn each_duel_gets_its_own_duel_id() {
    // 0x17AD (0x4A1B0C): duel_id is kept (agent+0x5C); for an NPC duel the reply is copied into
    // MainGamen+0x88C with the agent's regulation_id, and the agent is done.
    let server = Server::with_world(world());
    let ask = |c: &mut common::Client| {
        let fields = [("event_id", Value::U32(0)), ("join_id", Value::U32(1)), ("npc_id", Value::U32(120))];
        let r = one(c.send(req_with(0x17AC, &fields)));
        assert_eq!(r.request(), Some(0x17AD));
        r.get("duel_id").and_then(Value::as_u32).expect("duel_id")
    };
    let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
    let (first, second) = (ask(&mut a), ask(&mut b));
    assert!(first > 0 && second > 0 && first != second);
}

#[test]
fn the_duel_deck_is_acknowledged() {
    // After 0x17AD, DuelManager's deck selection (0x50ECF0) sends the chosen deck as 0x1F4F
    // {duel_id, event_id, update_type 1, item: the deck (tag 6)}. 0x22CA lets it go on to the duel
    // (0x50EFC1 -> 0x50F030), 0x22CB fails with text 0x2919. Nobody reads a field of 0x22CA (the
    // deck editor 0x43FC60 and the shop 0x48AA60 take it too).
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let deck = Message::deck().with("deck_name", Value::text("Starter Deck"));
    let fields = [("duel_id", Value::U32(1)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))];
    let update = fields.iter().fold(Message::reply(0x1F4F), |m, (k, v)| m.with(k, v.clone())).item(deck);
    let r = one(c.send(Packet::Message(update)));
    assert_eq!(r.request(), Some(0x22CA));
    assert!(r.fields.iter().all(|(k, _)| k == "request"), "{r:?}");
}

#[test]
fn the_duel_report_is_acknowledged_and_the_duel_server_announced() {
    // DuelManager 0x50F030: once its duel object (0x4DC320) reports state 2 or -1, it sends 0x17D4
    // {event_id, join_id, duel_id} unless MainGamen+0x8BC is set, and goes on at 0x17D5 (0x50F2B0);
    // 0x17D6 fails with text 0x2919. Neither reply is read further, nor has a global case.
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let duel_id = start_npc_duel(&mut c);
    let fields = [("event_id", Value::U32(0)), ("join_id", Value::U32(1)), ("duel_id", Value::U32(duel_id))];
    let replies = c.send(req_with(0x17D4, &fields));
    assert_eq!(replies[0].request(), Some(0x17D5));
    assert!(replies[0].fields.iter().all(|(k, _)| k == "request"), "{replies:?}");
    // Its next state 0x50F690 then waits for front flag 0x8000, which the global dispatcher sets on
    // 0x22C4 (0x4F4F34) while keeping the reply in slot 20. It reads the item named "duelData"
    // (0x531EB0 finds items by their "name" field): host and port go to DuelServerConnection
    // (+0x78, +0x94), which connects; its sub-item "rule" replaces MainGamen+0x88C, and match_type
    // is read from it. (Other duel paths also read duel_id from duelData: 0x4A2D30, 0x4A3650.)
    assert_eq!(replies.len(), 2, "{replies:?}");
    let push = &replies[1];
    assert_eq!(push.request(), Some(0x22C4));
    let data = &push.items[0];
    assert_eq!(data.get("name").and_then(Value::as_str).as_deref(), Some("duelData"));
    assert_eq!(data.get("host").and_then(Value::as_str).as_deref(), Some("10.0.0.5"));
    assert_eq!(data.get("port"), Some(&Value::U32(4321)));
    let rule = &data.items[0];
    assert_eq!(rule.get("name").and_then(Value::as_str).as_deref(), Some("rule"));
    // The rule is the one the client asked for in 0x17AC.
    assert_eq!(rule.get("match_type"), Some(&Value::U32(4)));
    assert_eq!(rule.get("regulation_id"), Some(&Value::U32(1)));
}

#[test]
fn a_report_for_an_unknown_duel_only_gets_its_ack() {
    let mut c = Server::with_world(world()).join(1001, "Yugi");
    let replies = c.send(req_with(0x17D4, &[("duel_id", Value::U32(99))]));
    assert_eq!(replies.iter().map(Message::request).collect::<Vec<_>>(), [Some(0x17D5)]);
}

/// 0x1784 then 0x17AC as the NPC duel sends them (live: Seto Kaiba, level 2); returns the duel_id.
fn start_npc_duel(c: &mut common::Client) -> u32 {
    let join_id = one(c.send(req_with(0x1784, &[("event_id", Value::U32(1))]))).get("join_id").cloned().unwrap();
    let rule = [("cpu_id", 124), ("duel_server_id", 0), ("match_type", 4), ("regulation_id", 1), ("surrender", 1)]
        .into_iter()
        .chain([("time_total", 0), ("time_turn", 0)])
        .fold(Message { tag: TAG_MESSAGE, ..Message::default() }, |m, (k, v)| m.with(k, Value::U32(v)));
    let duel = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", join_id).item(rule);
    one(c.send(Packet::Message(duel))).get("duel_id").and_then(Value::as_u32).unwrap()
}

#[test]
fn the_duel_deck_and_the_npc_are_kept_for_the_duel_server() {
    // The duel connection only names the player (0x200A {player_id}), so the deck picked at Select
    // Deck and the NPC of the 0x17AC rule (cpu_id) are kept by player for the duel server.
    let server = Server::with_world(world());
    let mut c = server.join(1001, "Yugi");
    let duel_id = start_npc_duel(&mut c);
    let deck = Message::deck()
        .with("deck_name", Value::text("Starter Deck"))
        .with("main", Value::u16_be_array(&[10, 10, 20]))
        .with("extra", Value::u16_be_array(&[30]))
        .with("side", Value::u16_be_array(&[]));
    let fields = [("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))];
    let update = fields.iter().fold(Message::reply(0x1F4F), |m, (k, v)| m.with(k, v.clone())).item(deck);
    one(c.send(Packet::Message(update)));
    let entry = server.hub.lock().unwrap().duel_entry(1001).expect("entry");
    assert_eq!((entry.deck.main, entry.deck.extra, entry.npc_id), (vec![10, 10, 20], vec![30], Some(124)));
}

#[test]
fn the_duels_end_reaches_the_front_connection_as_duel_data() {
    // The front dispatcher (0x4F4C9E) takes push 0x2009's item named "duelData", keeps it in slot 0x15 and sets flag
    // 0x20000 (it also reads a top-level score_<player_id>, which we leave out). The item must have tag 7: the
    // factory (0x531B90) makes a YO3DuelData, which finds its children by name (0x5315C0): "players" at +0x78 and
    // "result" at +0x7C. DuelScene (0x515BA3) reads result.duel_end (2 or 3: an abnormal end with a dialog);
    // DuelResult's first step (0x512070) finds the player's record in players by player_id (0x510830), notes a
    // child "resultData", and takes result.duel_result (1 won, 2 lost, 3 draw) as the match result (MainGamen
    // +0x820), which lets DuelManager (0x50FC00) leave the duel.
    let push = yo3_server::front::duel_over(1002, 2, [0, 1, 0]);
    assert_eq!(push.request(), Some(0x2009));
    let data = &push.items[0];
    assert_eq!(data.tag, yo3_proto::TAG_DUEL_DATA);
    assert_eq!(data.get("name").and_then(Value::as_str).as_deref(), Some("duelData"));
    let child = |name: &str| data.items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some(name));
    let result = child("result").expect("result");
    assert_eq!(result.get("duel_result").and_then(Value::as_u32), Some(2));
    assert_eq!(result.get("duel_end").and_then(Value::as_u32), Some(1));
    let players = child("players").expect("players");
    let me = players.items.iter().find(|p| p.get("player_id").and_then(Value::as_u32) == Some(1002)).expect("player");
    assert!(me.items.iter().any(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("resultData")));
}

#[test]
fn the_result_screen_shows_the_record_against_this_opponent() {
    // DuelResult's getResult (0x511200) reads the player's resultData: win, lose, draw and their *_add (0x511934..
    // 0x511B19) for the player's panel "Wins 1 (+001)", then lose / win swapped for the opponent's panel.
    let push = yo3_server::front::duel_over(1002, 2, [3, 2, 1]);
    let players = push.items[0].items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("players"));
    let result = &players.expect("players").items[0].items[0];
    let int = |k| result.get(k).and_then(Value::as_u32);
    assert_eq!([int("win"), int("lose"), int("draw")], [Some(3), Some(2), Some(1)]);
    assert_eq!([int("win_add"), int("lose_add"), int("draw_add")], [Some(0), Some(1), Some(0)]);
}

#[test]
fn the_hub_keeps_each_players_record_against_each_opponent() {
    let mut hub = yo3_server::front::Hub::default();
    assert_eq!(hub.record_result(1002, 124, 1), [1, 0, 0]);
    assert_eq!(hub.record_result(1002, 124, 2), [1, 1, 0]);
    assert_eq!(hub.record_result(1002, 125, 3), [0, 0, 1]);
    assert_eq!(hub.record_result(1003, 124, 1), [1, 0, 0]);
}

#[test]
fn the_duel_server_is_announced_at_the_address_the_client_reached() {
    // Without an advertised host, 0x22C4's duelData host is the front connection's own address.
    let world = World { duel_server: (String::new(), 4321), ..world() };
    let mut c = Server::with_world(world).join(1001, "Yugi");
    c.session.set_reached_at("192.168.1.2".parse().unwrap());
    let duel_id = start_npc_duel(&mut c);
    let r = c.send(req_with(0x17D4, &[("duel_id", Value::U32(duel_id))]));
    let data = r[1].items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("duelData")).unwrap();
    assert_eq!(data.get("host").and_then(Value::as_str).as_deref(), Some("192.168.1.2"));
}

#[test]
fn records_are_kept_across_restarts() {
    let dir = std::env::temp_dir().join(format!("yo3-records-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("records.json");
    let mut hub = yo3_server::front::Hub::with_records(&path);
    hub.record_result(1001, 1002, 1);
    let mut hub = yo3_server::front::Hub::with_records(&path);
    assert_eq!(hub.record_result(1001, 1002, 1), [2, 0, 0]);
    assert_eq!(hub.record_result(1002, 1001, 2), [0, 1, 0]);
}

#[test]
fn a_duels_result_carries_the_url_of_its_log() {
    // DuelResult (0x512070) reads dlogURL from the same item as duel_result, and a non-empty text starts the client's
    // duel log download (0x498700) into players\<name>\duellogs\myduels (0x498B10).
    let push = yo3_server::front::duel_over_with_log(1002, 1, [1, 0, 0], Some("http://127.0.0.1:8080/duellogs/a.dlog"));
    let data = &push.items[0];
    let result = data.items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some("result")).unwrap();
    assert_eq!(result.get("dlogURL").and_then(Value::as_str).as_deref(), Some("http://127.0.0.1:8080/duellogs/a.dlog"));
    assert_eq!(result.get("duel_result").and_then(Value::as_u32), Some(1));
}

#[test]
fn a_finished_duel_leaves_a_log_the_duel_log_screen_reads() {
    // The .dlog header the screen's loader (0x538AB0, then 0x524CA0) reads: magic 0x9E7, version 1, duel_id, player_1,
    // player_2, match_type, dlv_1, dlv_2, end_time, pname_1, pname_2, dlog_comment.
    use yo3_server::front::Ended;
    let dir = std::env::temp_dir().join(format!("yo3-dlogs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut hub = yo3_server::front::Hub::default();
    hub.keep_duel_logs(&dir, 8080);
    let ended = Ended { player_id: 1001, opponent: 1002, npc: false, duel_id: 9, match_type: 1, result: 1, replay: None };
    let name = hub.finish_duel(&ended).expect("a log file");
    let bytes = std::fs::read(dir.join(&name)).unwrap();
    let int = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(&bytes[..4], [0xE7, 0x09, 1, 0]);
    assert_eq!([int(8), int(12), int(16), int(20)], [9, 1001, 1002, 1]);
    assert_eq!(bytes[100], 0);
    assert!(bytes.len() >= 360);
    assert_eq!(hub.record_result(1001, 1002, 1), [2, 0, 0], "the duel was counted");
    // A duel against an NPC leaves no log: the client downloads none of those (DuelResult 0x512070, MainGamen +0x804).
    let npc = Ended { player_id: 1001, opponent: 125, npc: true, duel_id: 10, match_type: 1, result: 1, replay: None };
    assert_eq!(hub.finish_duel(&npc), None);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn a_replay_runs_from_the_janken_to_the_matchs_end() {
    // The commands of a duel log (duellog.rs): a watcher's pre-duel screen first (1500-1503, which take it to the
    // field), then the card table, the duel's start and LP, the engine's events, the duel's end (command 2) and 1505,
    // which takes a watching client's DuelResult out of the duel.
    use serde_json::json;
    use yo3_server::duel::{lead_janken, replay_commands};
    use yo3_server::front::Deck;
    let deck = |first: u16| Deck { main: (first..first + 5).collect(), ..Deck::default() };
    let world = yo3_server::front::World::new(vec![]);
    let history = [json!({ "type": 5, "player": 0, "reason": 1 })];
    let commands = replay_commands(&world, [&deck(100), &deck(200)], [1, 2, 0], 8000, &history);
    let ids: Vec<u32> = commands.iter().map(|c| c[0]).collect();
    assert_eq!(&ids[..4], [1500, 1501, 1502, 1503]);
    assert_eq!(commands[1][2..5], [1, 2, 0], "the janken as the first player saw it: won with rock against scissors... ");
    assert!(ids.contains(&1) && ids.iter().filter(|&&i| i == 1000).count() >= 10, "the card table and the duel's start");
    assert_eq!(&ids[ids.len() - 2..], [2, 1505]);
    assert_eq!(commands[commands.len() - 2][2], 1, "player 0 won");
    assert_eq!(commands[commands.len() - 1][2], 1, "the match too");
    // A janken that the other seat's player saw: the result and the hands swap.
    assert_eq!(lead_janken([1, 2, 0], true), [1, 2, 0]);
    assert_eq!(lead_janken([1, 2, 0], false), [2, 0, 2]);
    assert_eq!(lead_janken([0, 1, 1], false), [0, 1, 1]);
}
