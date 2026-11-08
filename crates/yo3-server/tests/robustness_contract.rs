//! What keeps one failure from stalling or killing the server (the 2026-11-08 robustness review): an engine that
//! dies mid-duel, a panic under a lock, a player leaving before the duel started, a crafted connection.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{Message, Packet, Value};
use yo3_server::duel::{DuelSession, SERVER_DUEL};
use yo3_server::front::{Deck, DuelEntry, Hub, World};
use yo3_server::translate::CardStats;

mod common;

const DUEL: u32 = 7;

fn real_engine() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    dir.join("vendor/cardinfo.json").is_file().then_some(dir)
}

/// An engine directory whose sidecar is the real one behind a proxy that dies on its next call once the returned
/// flag file exists.
fn dying_engine(name: &str) -> Option<(PathBuf, PathBuf)> {
    let real = real_engine()?;
    let dir = std::env::temp_dir().join(format!("yo3-dying-engine-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flag = dir.join("die");
    let _ = std::fs::remove_file(&flag);
    let script = format!(
        "import {{ spawn }} from 'node:child_process';\n\
         import {{ existsSync }} from 'node:fs';\n\
         const real = spawn('node', [{:?}], {{ stdio: ['pipe', 'inherit', 'inherit'] }});\n\
         process.stdin.on('data', (d) => {{ if (existsSync({:?})) process.exit(1); real.stdin.write(d); }});\n\
         process.stdin.on('end', () => real.kill());\n",
        real.join("duelcore.ts").display().to_string(),
        flag.display().to_string()
    );
    std::fs::write(dir.join("duelcore.ts"), script).unwrap();
    Some((dir, flag))
}

fn world(engine_dir: Option<PathBuf>) -> World {
    let stats = |cid, atk| CardStats { cid, atk, def: 0, level: 4, attribute: 1, race: 1 };
    let cards: HashMap<u16, CardStats> = [(1, stats(6960, 2000)), (2, stats(5004, 1900))].into_iter().collect();
    World { cards, engine_dir, ..World::new((1..=20).collect()) }
}

struct Seat {
    session: DuelSession,
    inbox: UnboundedReceiver<Message>,
}

impl Seat {
    fn send(&mut self, name: u32, fields: &[(&str, u32)]) -> Vec<Message> {
        let m = Message::reply(0x200C).with("duelRequestName", Value::U32(name));
        self.session.handle(&Packet::Message(fields.iter().fold(m, |m, (k, v)| m.with(k, Value::U32(*v)))))
    }

    fn pushed(&mut self) -> Vec<Message> {
        std::iter::from_fn(|| self.inbox.try_recv().ok()).collect()
    }
}

fn table_for(world: World) -> (Seat, Seat, Arc<Mutex<Hub>>) {
    let world = Arc::new(world);
    let hub = Arc::new(Mutex::new(Hub::default()));
    for (me, them, row) in [(1001, 1002, 1), (1002, 1001, 2)] {
        let deck = Deck { main: vec![row; 40], ..Deck::default() };
        hub.lock().unwrap().enter_duel(me, DuelEntry { deck, duel_id: Some(DUEL), opponent: Some(them), best_of: 1, ..DuelEntry::default() });
    }
    let seat = |seed| {
        let (tx, inbox) = unbounded_channel();
        Seat { session: DuelSession::new(world.clone(), hub.clone(), seed).with_outbox(tx), inbox }
    };
    (seat(3), seat(5), hub)
}

fn enter(seat: &mut Seat, player_id: u32) -> Vec<Message> {
    let m = Message::reply(0x200A).with("player_id", Value::U32(player_id)).with("autoSet", Value::U32(1));
    seat.session.handle(&Packet::Message(m))
}

fn commands(replies: &[Message]) -> Vec<(u32, [u32; 4])> {
    replies
        .iter()
        .filter(|m| m.request() == Some(SERVER_DUEL) && m.get("duelRequestName") == Some(&Value::U32(0x11)))
        .map(|m| {
            let int = |k| m.get(k).and_then(Value::as_u32).unwrap();
            (int("id"), [int("param1"), int("param2"), int("param3"), int("param4")])
        })
        .collect()
}

/// Both seated, 1001 won the janken (rock against scissors).
fn after_janken(world: World) -> (Seat, Seat, Arc<Mutex<Hub>>) {
    let (mut a, mut b, hub) = table_for(world);
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    a.send(0xC, &[("selectHand", 0)]);
    b.send(0xC, &[("selectHand", 1)]);
    a.pushed();
    (a, b, hub)
}

/// A draw with no reason (DuelScene 0x514290 shows duel_draw.swf, no reason text).
const NO_RESULT: (u32, [u32; 4]) = (2, [3, 0, 0, 0]);

#[test]
fn an_engine_that_dies_mid_duel_ends_it_as_a_draw_for_both() {
    // The sidecar answers the opening, then dies on its next call (a phase button): both clients get a draw end
    // instead of waiting forever, the match is over and counted once.
    let Some((dir, flag)) = dying_engine("mid") else { return };
    let (mut a, mut b, hub) = after_janken(world(Some(dir)));
    a.send(0xD, &[("choice", 1)]);
    b.pushed();
    std::fs::write(flag, "").unwrap();
    let out = a.send(3, &[("phase", 5)]);
    assert_eq!(commands(&out).last(), Some(&NO_RESULT), "{out:?}");
    assert_eq!(commands(&b.pushed()).last(), Some(&NO_RESULT));
    assert_eq!(hub.lock().unwrap().record_result(1001, 1002, 0), [0, 0, 1], "counted once, as a draw");
    assert!(a.send(3, &[("phase", 5)]).is_empty(), "nothing more once it ended");
}

#[test]
fn an_engine_that_cannot_open_the_duel_ends_it_as_a_draw_too() {
    let Some((dir, flag)) = dying_engine("open") else { return };
    std::fs::write(flag, "").unwrap();
    let (mut a, mut b, hub) = after_janken(world(Some(dir)));
    let out = a.send(0xD, &[("choice", 1)]);
    assert_eq!(commands(&out).last(), Some(&NO_RESULT), "{out:?}");
    assert_eq!(commands(&b.pushed()).last(), Some(&NO_RESULT));
    assert_eq!(hub.lock().unwrap().record_result(1001, 1002, 0), [0, 0, 1]);
}

#[test]
fn an_engine_that_dies_in_an_npc_duel_ends_it_as_a_draw() {
    let Some((dir, flag)) = dying_engine("npc") else { return };
    let mut world = world(Some(dir));
    world.npc_decks.insert(124, Deck { main: vec![2; 40], ..Deck::default() });
    let mut hub = Hub::default();
    hub.enter_duel(1001, DuelEntry { deck: Deck { main: vec![1; 40], ..Deck::default() }, npc_id: Some(124), ..DuelEntry::default() });
    let mut session = DuelSession::new(Arc::new(world), Arc::new(Mutex::new(hub)), 9);
    enter_session(&mut session, 1001);
    let send = |s: &mut DuelSession, name: u32, fields: &[(&str, u32)]| {
        let m = Message::reply(0x200C).with("duelRequestName", Value::U32(name));
        s.handle(&Packet::Message(fields.iter().fold(m, |m, (k, v)| m.with(k, Value::U32(*v)))))
    };
    send(&mut session, 0xD, &[("choice", 1)]);
    std::fs::write(flag, "").unwrap();
    let out = send(&mut session, 3, &[("phase", 5)]);
    assert_eq!(commands(&out).last(), Some(&NO_RESULT), "{out:?}");
    assert!(send(&mut session, 3, &[("phase", 5)]).is_empty(), "nothing more once it ended");
}

fn enter_session(s: &mut DuelSession, player_id: u32) -> Vec<Message> {
    let m = Message::reply(0x200A).with("player_id", Value::U32(player_id)).with("autoSet", Value::U32(1));
    s.handle(&Packet::Message(m))
}

mod wire {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use yo3_proto::{Message, Packet, decrypt, frame, frame_len};
    use yo3_server::{Config, start};

    pub async fn recv(s: &mut TcpStream) -> std::io::Result<Message> {
        let mut head = [0; 4];
        s.read_exact(&mut head).await?;
        let mut body = vec![0; frame_len(head)];
        s.read_exact(&mut body).await?;
        match Packet::decode(&decrypt(&body).unwrap()).unwrap() {
            Packet::Message(m) => Ok(m),
            other => panic!("unexpected {other:?}"),
        }
    }

    fn version() -> Vec<u8> {
        [b"\x00\x122010.03.23 skelton".as_slice(), &[0; 4]].concat()
    }

    pub async fn front(server: &yo3_server::Running) -> TcpStream {
        let mut s = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], server.front.port()))).await.unwrap();
        s.write_all(&frame(&version())).await.unwrap();
        s
    }

    pub async fn running(idle: Duration) -> yo3_server::Running {
        let log_dir = std::env::temp_dir().join(format!("yo3-robust-{}-{idle:?}", std::process::id()));
        start(Config { idle_timeout: idle, ..Config::ephemeral(log_dir) }).await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_crafted_deeply_nested_frame_does_not_kill_the_server() {
        let server = running(Duration::from_secs(30)).await;
        // 200,000 nested empty messages in one frame just under the 1 MiB limit: a stack overflow, once, for a
        // decoder that recursed without a limit, and the Blowfish key is public.
        let plain: Vec<u8> = (0..200_000).flat_map(|_| [4u8, 0, 0, 0, 1]).collect();
        let mut evil = front(&server).await;
        assert_eq!(recv(&mut evil).await.unwrap().request(), Some(0x219F));
        evil.write_all(&frame(&plain)).await.unwrap();
        // Still alive: a second client logs in as usual.
        let mut ok = front(&server).await;
        assert_eq!(recv(&mut ok).await.unwrap().request(), Some(0x219F));
        ok.write_all(&frame(&Message::reply(0x1F47).encode())).await.unwrap();
        assert_eq!(recv(&mut ok).await.unwrap().request(), Some(0x219A));
    }

    #[tokio::test]
    async fn a_connection_that_goes_silent_is_closed_but_a_heartbeating_one_stays() {
        let server = running(Duration::from_millis(400)).await;
        let mut silent = front(&server).await;
        let mut beating = front(&server).await;
        assert_eq!(recv(&mut silent).await.unwrap().request(), Some(0x219F));
        assert_eq!(recv(&mut beating).await.unwrap().request(), Some(0x219F));
        for _ in 0..4 {
            tokio::time::sleep(Duration::from_millis(250)).await;
            beating.write_all(&frame(&Message::reply(0x2724).encode())).await.unwrap();
        }
        let closed = tokio::time::timeout(Duration::from_secs(5), recv(&mut silent)).await.expect("closed in time");
        assert!(closed.is_err(), "the silent connection was closed");
        beating.write_all(&frame(&Message::reply(0x1F47).encode())).await.unwrap();
        assert_eq!(recv(&mut beating).await.unwrap().request(), Some(0x219A));
    }
}

/// An engine directory whose sidecar answers every call with the same prompt (a MSG_SELECT_SUM no card can satisfy),
/// as a core does that keeps refusing the server's answer; and the file counting its calls.
fn nagging_engine(name: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yo3-nagging-engine-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let calls = dir.join("calls");
    let _ = std::fs::remove_file(&calls);
    let card = serde_json::json!({ "code": 1, "controller": 0, "location": 2, "sequence": 0, "amount": 1 });
    let prompt = serde_json::json!({ "type": 23, "player": 0, "select_max": 1, "amount": 100, "min": 1, "max": 1,
                                     "selects_must": [], "selects": [card] });
    let answer = serde_json::json!({ "status": "await", "messages": [prompt], "errors": [] });
    let script = format!(
        "import {{ appendFileSync }} from 'node:fs';\n\
         process.stdin.on('data', () => {{ appendFileSync({:?}, 'x'); process.stdout.write({:?} + '\\n'); }});\n",
        calls.display().to_string(),
        answer.to_string()
    );
    std::fs::write(dir.join("duelcore.ts"), script).unwrap();
    (dir, calls)
}

#[test]
fn a_prompt_the_core_keeps_refusing_ends_the_duel_instead_of_looping() {
    let (dir, calls) = nagging_engine("table");
    let (mut a, mut b, hub) = after_janken(world(Some(dir)));
    let started = std::time::Instant::now();
    let out = a.send(0xD, &[("choice", 1)]);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(commands(&out).last(), Some(&NO_RESULT), "{:?}", commands(&out).len());
    assert_eq!(commands(&b.pushed()).last(), Some(&NO_RESULT));
    assert_eq!(hub.lock().unwrap().record_result(1001, 1002, 0), [0, 0, 1]);
    let asked = std::fs::read_to_string(calls).unwrap().len();
    assert!(asked < 50, "the core was asked {asked} times");
}

#[test]
fn the_same_goes_for_an_npc_duel() {
    let (dir, calls) = nagging_engine("npc");
    let mut world = world(Some(dir));
    world.npc_decks.insert(124, Deck { main: vec![2; 40], ..Deck::default() });
    let mut hub = Hub::default();
    hub.enter_duel(1001, DuelEntry { deck: Deck { main: vec![1; 40], ..Deck::default() }, npc_id: Some(124), ..DuelEntry::default() });
    let mut session = DuelSession::new(Arc::new(world), Arc::new(Mutex::new(hub)), 9);
    enter_session(&mut session, 1001);
    let m = Message::reply(0x200C).with("duelRequestName", Value::U32(0xD)).with("choice", Value::U32(1));
    let out = session.handle(&Packet::Message(m));
    assert_eq!(commands(&out).last(), Some(&NO_RESULT));
    assert!(std::fs::read_to_string(calls).unwrap().len() < 50);
}

#[test]
fn a_player_leaving_before_the_duel_started_sends_the_opponent_out() {
    // Left in the janken, the opponent waited forever: only a started duel notified it. Now it gets the end at once
    // (command 2 {1 won, 14 disconnected}), and nothing is counted for a duel that never began.
    let (mut a, mut b, hub) = table_for(world(None));
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    b.pushed();
    drop(a);
    assert_eq!(commands(&b.pushed()), [(2, [1, 14, 0, 0])]);
    assert_eq!(hub.lock().unwrap().record_result(1002, 1001, 0), [0, 0, 0], "nothing counted");
}

#[test]
fn a_player_leaving_in_the_turn_order_choice_sends_the_opponent_out_too() {
    let (a, mut b, hub) = after_janken(world(None));
    b.pushed();
    drop(a);
    assert_eq!(commands(&b.pushed()), [(2, [1, 14, 0, 0])]);
    let _ = hub;
}

#[test]
fn a_second_player_arriving_after_the_first_left_is_not_given_an_orphan_table() {
    let (mut a, mut b, hub) = table_for(world(None));
    enter(&mut a, 1001);
    drop(a);
    // The match is over: 1002 is told, and nothing waits for a partner who never comes.
    assert_eq!(commands(&enter(&mut b, 1002)), [(2, [1, 14, 0, 0])]);
    assert!(b.send(0xC, &[("selectHand", 1)]).is_empty());
    assert!(hub.lock().unwrap().ended(DUEL));
}

#[test]
fn the_turn_order_is_taken_once() {
    // A second 0xD (a replayed or hostile client) opened another engine process each time.
    let (dir, calls) = nagging_engine("once-table");
    let (mut a, _b, _hub) = after_janken(world(Some(dir)));
    a.send(0xD, &[("choice", 1)]);
    let asked = std::fs::read_to_string(&calls).unwrap().len();
    assert!(asked > 0);
    assert!(a.send(0xD, &[("choice", 1)]).is_empty());
    assert!(a.send(0xD, &[("choice", 2)]).is_empty());
    assert_eq!(std::fs::read_to_string(&calls).unwrap().len(), asked, "no new engine");

    let (dir, calls) = nagging_engine("once-npc");
    let mut world = world(Some(dir));
    world.npc_decks.insert(124, Deck { main: vec![2; 40], ..Deck::default() });
    let mut hub = Hub::default();
    hub.enter_duel(1001, DuelEntry { deck: Deck { main: vec![1; 40], ..Deck::default() }, npc_id: Some(124), ..DuelEntry::default() });
    let mut session = DuelSession::new(Arc::new(world), Arc::new(Mutex::new(hub)), 9);
    enter_session(&mut session, 1001);
    let choose = |s: &mut DuelSession| {
        let m = Message::reply(0x200C).with("duelRequestName", Value::U32(0xD)).with("choice", Value::U32(1));
        s.handle(&Packet::Message(m))
    };
    choose(&mut session);
    let asked = std::fs::read_to_string(&calls).unwrap().len();
    assert!(asked > 0);
    assert!(choose(&mut session).is_empty());
    assert_eq!(std::fs::read_to_string(&calls).unwrap().len(), asked, "no new engine");
}

mod front {
    use std::time::{Duration, Instant};
    use yo3_proto::{Message, Value};
    use yo3_server::front::World;

    use super::common::{Client, Server, req_with};

    pub fn world() -> World {
        World { duel_server: ("10.0.0.5".into(), 4321), ..World::new((1..=20).collect()) }
    }

    pub fn auto(c: &mut Client) {
        let join = c.send(req_with(0x1784, &[("event_id", Value::U32(2))])).remove(0);
        let fields = [("event_id", Value::U32(2)), ("join_id", join.get("join_id").cloned().unwrap()), ("match_type", Value::U32(1)), ("level", Value::U32(0))];
        c.send(req_with(0x1798, &fields));
    }

    pub fn wait(c: &mut Client) -> Vec<Message> {
        let start = Instant::now();
        let mut out = c.pushed();
        while out.is_empty() && start.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(20));
            out = c.pushed();
        }
        out
    }

    pub fn matched() -> (Server, Client, Client, u32) {
        let server = Server::with_world(world());
        let (mut a, mut b) = (server.join(1001, "Yugi"), server.join(1002, "Kaiba"));
        auto(&mut a);
        auto(&mut b);
        let push = wait(&mut a).remove(0);
        wait(&mut b);
        let duel_id = push.items.iter().find_map(|i| i.get("duel_id").and_then(Value::as_u32)).expect("duel_id");
        (server, a, b, duel_id)
    }
}

#[test]
fn a_matched_player_whose_client_dies_before_the_duel_frees_the_match() {
    // Hub::leave never ended the match: a client that died between the match push and its duel connection left the
    // match (and an event's round) stuck, both players shown as duelling for good. Now the match ends, the other
    // player is sent back out (0x2009, duel_end 3, as for a refused deck) and a later duel connection is turned away.
    let (server, a, mut b, duel_id) = front::matched();
    assert!(!server.hub.lock().unwrap().ended(duel_id));
    drop(a);
    assert!(server.hub.lock().unwrap().ended(duel_id), "the match is over");
    let pushed = b.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(0x2009)), "{pushed:?}");
}

#[test]
fn a_corrupt_side_file_is_kept_aside_and_not_overwritten_by_the_next_save() {
    // friends.json (and mail, teams, rentals, records, events) that wouldn't parse logged a warning, started empty and
    // was overwritten by the next save, though the docs invite hand-editing it.
    let dir = std::env::temp_dir().join(format!("yo3-corrupt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let broken = "{ \"friends\": [[1001, 1002]], oops";
    std::fs::write(dir.join("friends.json"), broken).unwrap();
    let mut hub = Hub::default();
    hub.keep_social(&dir.join("friends.json"));
    hub.befriend((1001, "Yugi"), (1003, "Joey"));
    let kept: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("friends.json.corrupt-"))
        .collect();
    assert_eq!(kept.len(), 1, "the bad file is still there");
    assert_eq!(std::fs::read_to_string(kept[0].path()).unwrap(), broken);
    assert!(dir.join("friends.json").is_file(), "and the new one holds the new friendship");
}

#[test]
fn a_failed_reselect_drops_the_old_registration_and_player_requests_wait_for_a_player() {
    // A failed 0x1F4C left the old registration online (a ghost in searches and matching), and player() fell back to
    // the first player, so requests with no selection acted as them. Now both are refused.
    let server = common::Server::with_world(front::world());
    let mut a = server.join(1001, "Yugi");
    assert_eq!(server.hub.lock().unwrap().online(), 1);
    let bad = a.send(common::req_with(0x1F4C, &[("player_id", Value::U32(4242))]));
    assert!(bad.is_empty());
    assert_eq!(server.hub.lock().unwrap().online(), 0, "the old registration is gone");
    // Matching, hosting, trade, mail and the like act for nobody.
    let auto = [("event_id", Value::U32(2)), ("join_id", Value::U32(1)), ("match_type", Value::U32(1)), ("level", Value::U32(0))];
    for id in [0x1784, 0x1798, 0x17AC, 0x2BC0, 0x2C24, 0x2AFE, 0x2CF0] {
        assert!(a.send(common::req_with(id, &auto)).is_empty(), "{id:#x} answered with no player selected");
    }
    // A client that never selected anyone gets nothing from them either.
    let mut c = {
        let (tx, _inbox) = tokio::sync::mpsc::unbounded_channel();
        let session = yo3_server::front::FrontSession::new(yo3_server::front::Account::test(), std::sync::Arc::new(front::world()), server.hub.clone(), tx);
        common::Client { session, inbox: _inbox }
    };
    c.send(Packet::Version("2010.03.23 skelton".into()));
    assert!(c.send(common::req_with(0x1798, &auto)).is_empty());
    assert_eq!(server.hub.lock().unwrap().online(), 0);
    // The login and Select Player's own requests still answer.
    assert_eq!(c.send(common::req(0x1F49)).len(), 1);
}

#[test]
fn a_duel_entry_is_claimed_only_from_the_address_that_brought_it() {
    // 0x200A {player_id} is taken as sent: anyone could name a player and play (or spoil) their duel. The entry now
    // remembers the front connection's address and the duel connection must come from the same one.
    let peer = |last: u8| std::net::IpAddr::from([10, 0, 0, last]);
    let world = Arc::new(world(None));
    let hub = Arc::new(Mutex::new(Hub::default()));
    let entry = DuelEntry { duel_id: Some(DUEL), opponent: Some(1002), best_of: 1, peer: Some(peer(1)), ..DuelEntry::default() };
    hub.lock().unwrap().enter_duel(1001, entry);
    let (tx, _rx) = unbounded_channel();
    let mut thief = DuelSession::new(world.clone(), hub.clone(), 3).with_outbox(tx).with_peer(peer(9));
    assert!(enter_session(&mut thief, 1001).is_empty());
    assert!(thief.handle(&Packet::Message(Message::reply(0x200C).with("duelRequestName", Value::U32(0xC)).with("selectHand", Value::U32(0)))).is_empty());
    let (tx, _rx) = unbounded_channel();
    let (owner_world, owner_hub) = (world.clone(), hub.clone());
    let mut owner = DuelSession::new(world, hub, 3).with_outbox(tx).with_peer(peer(1));
    assert!(enter_session(&mut owner, 1001).is_empty(), "seated, waiting for the opponent");
    let theirs = DuelEntry { duel_id: Some(DUEL), opponent: Some(1001), best_of: 1, peer: Some(peer(2)), ..DuelEntry::default() };
    owner_hub.lock().unwrap().enter_duel(1002, theirs);
    let (tx, _rx) = unbounded_channel();
    let mut other = DuelSession::new(owner_world, owner_hub, 5).with_outbox(tx).with_peer(peer(2));
    assert_eq!(commands(&enter_session(&mut other, 1002)), [(1500, [0; 4])], "the janken begins: the thief took no seat");
}

#[test]
fn what_a_client_can_make_the_server_keep_is_capped() {
    let server = common::Server::with_world(front::world());
    let mut c = server.join(1001, "Yugi");
    // An avatar blob of a megabyte is refused; a speech line of a megabyte is cut.
    let huge = |avatar: usize, speech: usize| {
        Packet::Message(
            Message::reply(0x2CEF).with("avatar1", Value::Bin(vec![1; avatar])).with("speech00", Value::text(&"あ".repeat(speech))).with("title", Value::U32(0)),
        )
    };
    assert_eq!(common::one(c.send(huge(1 << 20, 1))).request(), Some(0x2CF6));
    assert_eq!(common::one(c.send(huge(82, 300_000))).request(), Some(0x2CF5));
    let me = common::one(c.send(common::req(0x2CEE))).items.remove(0);
    assert!(me.get("speech00").and_then(Value::as_str).unwrap().len() <= 200);
    // A user event: one unfinished event per host, and a form's fields are cut.
    let form = |name: &str| {
        let mut m = Message::reply(0x3390).with("event_name", Value::text(name)).with("comment", Value::text(&"x".repeat(100_000)));
        for i in 0..500 {
            m = m.with(&format!("extra{i}"), Value::U32(i));
        }
        Packet::Message(m)
    };
    assert_eq!(common::one(c.send(form("First"))).request(), Some(0x33F4));
    assert_eq!(common::one(c.send(form("Second"))).request(), Some(0x33F5), "a second event while the first is unfinished");
    // Create Duel's settings replace the player's earlier hosted duel instead of piling up.
    let host = || {
        let rule = Message::record().with("status", Value::U32(1)).with("match_type", Value::U32(1));
        Packet::Message(Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(rule))
    };
    for _ in 0..5 {
        c.send(host());
    }
    assert_eq!(server.hub.lock().unwrap().hosted_duels(), 1);
}
