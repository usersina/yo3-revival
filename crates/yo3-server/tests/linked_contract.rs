//! Fair play between linked accounts (#64): accounts seen on one device or address. A duel between them is played but
//! pays no BP and counts no record or title; they can't trade or rent to each other; staff see them linked; and the
//! `games-per-device` setting caps the games one device runs.
mod common;

use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use common::{Client, Server, one, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::{Directory, START_BP};
use yo3_server::auth::Sessions;
use yo3_server::front::{Ended, World};
use yo3_server::links::Link;
use yo3_server::settings::Live;

struct Pair { server: Server, dir: Arc<Mutex<Directory>>, a: Client, b: Client, a_id: u32, b_id: u32, a_user: u32, b_user: u32 }

fn address(n: u8) -> IpAddr { IpAddr::from([10, 0, 0, n]) }

/// Two accounts (players of their own), seen at `seen`: each (device, address).
fn pair(seen: [(Option<&str>, IpAddr); 2]) -> Pair {
    let dir = Arc::new(Mutex::new(Directory::new(vec!["Duelist".into()])));
    let (x, y) = (address(1), address(2));
    let (a_id, b_id) = {
        let mut d = dir.lock().unwrap();
        (d.account_for(x).players[0].player_id, d.account_for(y).players[0].player_id)
    };
    let (a_user, b_user) = { let d = dir.lock().unwrap(); (d.user_of_player(a_id).unwrap(), d.user_of_player(b_id).unwrap()) };
    {
        let mut d = dir.lock().unwrap();
        let now = yo3_server::logins::now();
        d.note_seen(a_user, seen[0].0, Some(seen[0].1), now);
        d.note_seen(b_user, seen[1].0, Some(seen[1].1), now);
        d.update_player(a_id, |p| { p.cards.insert(10, 2); }).unwrap();
    }
    let server = Server::with_world(World { card_rows: 50, ..World::new((1..=20).collect()) });
    server.hub.lock().unwrap().set_directory(dir.clone());
    let (a, b) = (server.join_from(&dir, x), server.join_from(&dir, y));
    Pair { server, dir, a, b, a_id, b_id, a_user, b_user }
}

fn ended(p: &Pair, result: u32) -> Ended {
    Ended { player_id: p.a_id, opponent: p.b_id, npc: false, duel_id: 1, match_type: 1, result, replay: None, stats: None, start_lp: 8000 }
}

fn bp(p: &Pair, user: u32) -> u32 { p.dir.lock().unwrap().dpoint(user).unwrap() }

fn notes(c: &mut Client) -> Vec<String> {
    c.pushed().iter().filter(|m| m.request() == Some(0x2455)).filter_map(|m| m.items.first()?.get("chat_str")?.as_str()).collect()
}

#[test]
fn a_duel_between_strangers_pays_and_counts() {
    let p = pair([(Some("pc-a"), address(1)), (Some("pc-b"), address(2))]);
    p.server.hub.lock().unwrap().finish_duel(&ended(&p, 1));
    assert!(bp(&p, p.a_user) > START_BP, "the winner is paid");
    assert_eq!(p.server.hub.lock().unwrap().record_totals(p.a_id), [1, 0, 0]);
}

#[test]
fn a_duel_between_accounts_on_one_device_pays_no_bp_and_counts_no_record() {
    let mut p = pair([(Some("same-pc"), address(1)), (Some("same-pc"), address(2))]);
    assert_eq!(p.dir.lock().unwrap().links_between(p.a_user, p.b_user, yo3_server::logins::now()), [Link::Device]);
    p.server.hub.lock().unwrap().finish_duel(&ended(&p, 1));
    assert_eq!(bp(&p, p.a_user), START_BP, "no BP, win or lose");
    assert_eq!(p.server.hub.lock().unwrap().record_totals(p.a_id), [0, 0, 0], "no record");
    assert_eq!(p.dir.lock().unwrap().player(p.a_id).unwrap().counters.get("duels"), None, "no title progress");
    assert!(notes(&mut p.a).iter().any(|l| l.contains("doesn't count")), "the player is told why");
}

#[test]
fn a_duel_between_accounts_on_one_address_pays_nothing_either() {
    let p = pair([(Some("pc-a"), address(9)), (Some("pc-b"), address(9))]);
    p.server.hub.lock().unwrap().finish_duel(&ended(&p, 1));
    assert_eq!((bp(&p, p.a_user), p.server.hub.lock().unwrap().record_totals(p.a_id)), (START_BP, [0, 0, 0]));
}

#[test]
fn an_npc_duel_pays_for_a_player_whatever_their_accounts_share() {
    let p = pair([(Some("same-pc"), address(1)), (Some("same-pc"), address(2))]);
    let npc = Ended { opponent: 120, npc: true, ..ended(&p, 1) };
    p.server.hub.lock().unwrap().finish_duel(&npc);
    assert!(bp(&p, p.a_user) > START_BP);
}

#[test]
fn the_host_can_turn_the_rule_off() {
    let p = pair([(Some("same-pc"), address(1)), (Some("same-pc"), address(2))]);
    let live = Live::open(None, vec![]).unwrap();
    live.set("linked-accounts", "off").unwrap();
    p.server.hub.lock().unwrap().set_settings(Arc::new(live));
    p.server.hub.lock().unwrap().finish_duel(&ended(&p, 1));
    assert!(bp(&p, p.a_user) > START_BP, "linked-accounts off: strangers");
}

#[test]
fn linked_accounts_cannot_trade_or_rent_to_each_other() {
    let mut p = pair([(Some("same-pc"), address(1)), (Some("same-pc"), address(2))]);
    p.b.pushed();
    let ask = |c: &mut Client, id: u32, to: u32| one(c.send(req_with(id, &[("to_id", Value::U32(to))])));
    let trade = ask(&mut p.a, 0x2BC0, p.b_id);
    assert_eq!((trade.request(), trade.get("error_code").and_then(Value::as_u32)), (Some(0x2BD8), Some(106)), "{trade:?}");
    let rental = ask(&mut p.a, 0x2C24, p.b_id);
    assert_eq!((rental.request(), rental.get("error_code").and_then(Value::as_u32)), (Some(0x2D51), Some(106)), "{rental:?}");
    assert!(p.b.pushed().iter().all(|m| !matches!(m.request(), Some(0x2BC1 | 0x2C25))), "the other player is never asked");
    assert!(notes(&mut p.a).iter().any(|l| l.contains("can't trade or rent")));
}

#[test]
fn strangers_still_trade() {
    let mut p = pair([(Some("pc-a"), address(1)), (Some("pc-b"), address(2))]);
    let r: Message = one(p.a.send(req_with(0x2BC0, &[("to_id", Value::U32(p.b_id))])));
    assert_eq!(r.request(), Some(0x2BD7));
}

#[test]
fn what_an_account_was_seen_on_is_kept_and_linked_accounts_are_listed() {
    let dir = std::env::temp_dir().join(format!("yo3-links-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("accounts.json");
    let (x, y, z) = {
        let mut d = Directory::open(vec!["Duelist".into()], &file).unwrap();
        let ids: Vec<u32> = (1..=3).map(|n| d.account_for(address(n)).user_id).collect();
        let now = yo3_server::logins::now();
        d.note_seen(ids[0], Some("shared"), Some(address(1)), now);
        d.note_seen(ids[1], Some("shared"), Some(address(2)), now);
        d.note_seen(ids[2], Some("other"), Some(address(2)), now);
        (ids[0], ids[1], ids[2])
    };
    let d = Directory::open(vec!["Duelist".into()], &file).unwrap();
    let now = yo3_server::logins::now();
    assert_eq!(d.linked_to(x, now), [(y, vec![Link::Device])], "kept across a restart");
    assert_eq!(d.linked_to(y, now), [(x, vec![Link::Device]), (z, vec![Link::Address])]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_device_runs_at_most_the_cap_of_games() {
    let mut sessions = Sessions::default();
    let now = Instant::now();
    let one_session = |s: &mut Sessions, user: u32, device: Option<&str>| {
        let token = s.login_from(user, device, now).unwrap();
        let ticket = s.ticket(&token, yo3_tunnel::Role::Front, now).unwrap();
        s.consume(&ticket, yo3_tunnel::Role::Front, now).unwrap().0
    };
    let (a, b, c) = (one_session(&mut sessions, 1, Some("pc")), one_session(&mut sessions, 2, Some("pc")), one_session(&mut sessions, 3, Some("elsewhere")));
    assert!(sessions.set_playing(a, true, 1));
    assert_eq!(sessions.games_on("pc", 2), 1);
    assert!(!sessions.set_playing(b, true, 1), "the second game on the PC is over the cap");
    assert!(sessions.set_playing(c, true, 1), "another PC has its own");
    assert!(sessions.set_playing(b, true, 2), "a host who raised the cap allows it");
    sessions.set_playing(a, false, 1);
    assert_eq!(sessions.games_on("pc", 9), 1, "only b plays on the PC now");
    assert!(sessions.set_playing(b, true, 1), "the same account again takes no second place");
}

#[test]
fn the_default_cap_is_one_game_per_device_and_the_two_fair_play_settings_are_on() {
    let s = Live::open(None, vec![]).unwrap().now();
    assert_eq!((s.games_per_device, s.linked_fair, s.clean_game), (1, true, false));
}


/// The launcher's `device_id` in a sign-in: kept as a digest, linking the accounts that send the same one, and the
/// per-device cap refusing a second game on it.
#[tokio::test]
async fn a_sign_in_with_a_device_id_links_accounts_and_the_cap_refuses_a_second_game() {
    use serde_json::json;
    use yo3_server::account_ops::{Context, handle};
    use yo3_server::logins::{self, Login, Registration, Throttle};
    logins::use_fast_hashing();
    let mut d = Directory::new(vec!["Duelist".into()]);
    for user in ["alice", "bob"] {
        d.register(Login::new(user, logins::hash_password_now("correct horse 1").unwrap(), logins::digest(user)), None, Registration::Open).unwrap();
    }
    let (accounts, sessions, throttle) = (Mutex::new(d), Mutex::new(Sessions::default()), Mutex::new(Throttle::default()));
    let live = Live::open(None, vec![]).unwrap();
    let ask = |user: &str, device: &str, peer: &str| {
        let settings = live.now();
        let request = json!({ "op": "login", "user": user, "password": "correct horse 1", "device_id": device, "launcher": "99.0.0" });
        let (accounts, sessions, throttle, peer) = (&accounts, &sessions, &throttle, peer.parse().unwrap());
        async move { handle(&request, &Context { accounts, sessions, throttle, settings: &settings, peer }).await }
    };
    let a = ask("alice", "install-id-0001", "10.1.0.1").await;
    assert_eq!(a["ok"], true, "{a}");
    let b = ask("bob", "install-id-0001", "10.1.0.2").await;
    assert_eq!(b["ok"], true, "signing in is not playing: no game runs yet");
    let now = logins::now();
    let (ua, ub) = { let d = accounts.lock().unwrap(); (d.user_of("alice").unwrap(), d.user_of("bob").unwrap()) };
    assert_eq!(accounts.lock().unwrap().links_between(ua, ub, now), [Link::Device], "the same install id");
    assert!(accounts.lock().unwrap().seen(ua).unwrap().devices.keys().all(|k| !k.contains("install-id")), "kept as a digest");
    // Alice's game runs: with the default cap of 1 Bob can't take a second place on that PC, a stranger's PC can.
    let identity = { let mut s = sessions.lock().unwrap(); let t = s.login_from(ua, accounts.lock().unwrap().seen(ua).unwrap().devices.keys().next().map(String::as_str), Instant::now()).unwrap();
        let ticket = s.ticket(&t, yo3_tunnel::Role::Front, Instant::now()).unwrap(); s.consume(&ticket, yo3_tunnel::Role::Front, Instant::now()).unwrap().0 };
    assert!(sessions.lock().unwrap().set_playing(identity, true, 1));
    let refused = ask("bob", "install-id-0001", "10.1.0.2").await;
    assert_eq!((refused["ok"].as_bool(), refused["error"].as_str(), refused["limit"].as_u64()), (Some(false), Some("device_full"), Some(1)), "{refused}");
    assert_eq!(ask("bob", "another-install-02", "10.1.0.3").await["ok"], true);
    live.set("games-per-device", "2").unwrap();
    assert_eq!(ask("bob", "install-id-0001", "10.1.0.2").await["ok"], true, "a host raised the cap");
}

#[test]
fn an_address_many_accounts_share_is_a_gateway_and_links_nobody() {
    let mut d = Directory::new(vec!["Duelist".into()]);
    let now = yo3_server::logins::now();
    let ids: Vec<u32> = (1..=12).map(|n| d.account_for(address(n)).user_id).collect();
    for (n, &id) in ids.iter().enumerate() { d.note_seen(id, Some(&format!("pc-{n}")), Some(address(200)), now) }
    assert!(d.links_between(ids[0], ids[1], now).is_empty(), "twelve accounts behind one address: a proxy, not a household");
    let mut small = Directory::new(vec!["Duelist".into()]);
    let few: Vec<u32> = (1..=3).map(|n| small.account_for(address(n)).user_id).collect();
    for (n, &id) in few.iter().enumerate() { small.note_seen(id, Some(&format!("pc-{n}")), Some(address(200)), now) }
    assert_eq!(small.links_between(few[0], few[1], now), [Link::Address]);
}

#[test]
fn deleting_an_account_forgets_what_it_was_seen_on() {
    let mut d = Directory::new(vec!["Duelist".into()]);
    let now = yo3_server::logins::now();
    let (a, b) = (d.account_for(address(1)).user_id, d.account_for(address(2)).user_id);
    d.note_seen(a, Some("pc"), Some(address(1)), now);
    d.note_seen(b, Some("pc"), Some(address(2)), now);
    assert_eq!(d.linked_to(a, now).len(), 1);
    d.delete_account(b).unwrap();
    assert!(d.seen(b).is_none() && d.linked_to(a, now).is_empty(), "no trace of the deleted account");
}
