//! Chat commands (#63 banlist, #58 draw, #55 where, #73 pack, #44 report). The game keeps its own `/` commands and answers an
//! unknown one itself ("That command is not available."), but `/p NAME MESSAGE` goes to the server as a whisper
//! (`chat_type` 0, `to_name`, `chat_str`; seen live): the server is the player "Server". A chat room line that starts
//! with `!` and a command's name is one too. Each is answered to the sender alone, as lines in their chat window.
mod common;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use yo3_data::Regulation;
use yo3_proto::{Message, Packet, Value};
use yo3_server::accounts::Directory;
use yo3_server::api::{Api, Op};
use yo3_server::front::{Deck, World};
use yo3_server::logins::{self, Login, Registration, Role};
use yo3_server::reports::{Reports, Status};
use yo3_server::shop::PackList;
use yo3_server::sources::{CardSource, Kind};
use yo3_server::translate::CardStats;

/// Rows 1-100 are cards with Konami ids 1001-1100 named "Card 1001"...; banlist 3 "Unlimited" is open, 5 "Season 16"
/// forbids 1003 (the starter deck's third card), 20 "Season 17" forbids 1004. The server's login banlist is Season 16.
fn world() -> World {
    let stats = |row: u16| CardStats { cid: 1000 + row, atk: 0, def: 0, level: 4, attribute: 1, race: 1 };
    let reg = |id, lists: [Vec<u16>; 4]| Regulation { id, kind: 0, status: 0, name_id: 0, lists, default_limit: 3, record: Message::record() };
    let banlists: Vec<(u32, String)> = vec![(3, "Unlimited".into()), (5, "Season 16".into()), (20, "Season 17".into())];
    let settings = yo3_server::settings::Live::open(None, banlists.clone()).unwrap();
    settings.set("login-regulation", "Season 16").unwrap();
    let starter = Deck { id: 1, name: "Starter Deck".into(), main: (1..=20).chain(1..=20).collect(), extra: vec![], side: vec![] };
    World {
        card_rows: 100,
        starter_deck: Some(starter),
        cards: (1..=100).map(|row| (row, stats(row))).collect::<HashMap<_, _>>(),
        card_names: (1..=100).map(|row| (1000 + row, format!("Card {}", 1000 + row))).collect(),
        regulations: [reg(3, Default::default()), reg(5, [vec![1003], vec![], vec![], vec![]]), reg(20, [vec![1004], vec![], vec![], vec![]])]
            .into_iter().map(|r| (r.id, r)).collect(),
        banlists,
        settings: Arc::new(settings),
        card_sources: vec![
            CardSource { kind: Kind::Pack, name: "Booster Pack 3".into(), cards: [1007, 1008].into() },
            CardSource { kind: Kind::TradePool, name: "Blister's Trade Card (cards from Booster Packs 1-14)".into(), cards: [1007].into() },
            CardSource { kind: Kind::Rental, name: "Blister's Rental".into(), cards: [1008].into() },
        ],
        pack_lists: vec![
            PackList::new(3, "Booster Pack 3".into(), String::new(), &[vec![1, 2], vec![7], vec![8]], |row| Some(format!("Card {}", 1000 + row))),
            PackList::new(4, "Booster Pack 4".into(), String::new(), &[vec![9]], |row| Some(format!("Card {}", 1000 + row))),
        ],
        npcs: vec![124],
        ..World::new(vec![3, 5, 20])
    }
}

fn directory() -> Arc<Mutex<Directory>> {
    Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])))
}

fn addr(n: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, n))
}

/// A line the real client sends for `/p Server TEXT` (seen live): a whisper in one tag-10 item.
fn whisper(to: &str, text: &str) -> Packet {
    let body = Message { tag: 10, fields: vec![("chat_str".into(), Value::text(text)), ("chat_type".into(), Value::U32(0)), ("to_name".into(), Value::text(to))], items: vec![] };
    Packet::Message(Message::reply(0x2134).item(body))
}

fn say(room: u32, text: &str) -> Packet {
    let body = Message { tag: 10, fields: vec![("chat_str".into(), Value::text(text)), ("chat_type".into(), Value::U32(1)), ("to_id".into(), Value::U32(room))], items: vec![] };
    Packet::Message(Message::reply(0x2134).item(body))
}

/// What the server answered: the lines of its notes, after the 0x2456 success.
fn answer(replies: &[Message]) -> Vec<String> {
    assert_eq!(replies[0].request(), Some(0x2456), "send success first: {replies:?}");
    replies[1..].iter().map(|m| {
        assert_eq!(m.request(), Some(0x2455));
        assert_eq!(m.items[0].get("pname").and_then(Value::as_str).as_deref(), Some("Server"));
        m.items[0].get("chat_str").and_then(Value::as_str).unwrap()
    }).collect()
}

fn ask(c: &mut Client, text: &str) -> String {
    answer(&c.send(whisper("Server", text))).join("\n")
}

fn login_regulation(c: &mut Client) -> u32 {
    let r = c.send(req_with(0x1F48, &[("lang", Value::U32(1))])).remove(0);
    assert_eq!(r.request(), Some(0x2201));
    r.get("regulation_id").and_then(Value::as_u32).unwrap()
}

#[test]
fn a_whisper_to_the_server_is_answered_to_the_sender_alone() {
    let s = Server::with_world(world());
    let d = directory();
    let (mut a, mut b) = (s.join_from(&d, addr(1)), s.join_from(&d, addr(2)));
    let text = ask(&mut a, "help");
    for command in ["banlist", "draw", "where", "pack", "report"] {
        assert!(text.contains(command), "{command} in the help: {text}");
    }
    assert!(ask(&mut a, "nonsense").contains("banlist"), "an unknown command gets the help");
    assert!(ask(&mut a, "/help").contains("report"), "a leading / is allowed");
    assert!(b.pushed().is_empty(), "nobody else hears");
    // A line in a chat room starting with ! and a command's name is a command too; the room hears nothing.
    a.send(req_with(0x2136, &[("room_id", Value::U32(1))]));
    b.send(req_with(0x2136, &[("room_id", Value::U32(1))]));
    b.pushed();
    assert!(answer(&a.send(say(1, "!help"))).join(" ").contains("banlist"));
    assert!(b.pushed().is_empty());
    // Anything else is ordinary chat: "!" alone doesn't make a command.
    let r = a.send(say(1, "!lol that was close"));
    assert_eq!((r[0].request(), r[1].request()), (Some(0x2456), Some(0x2455)));
    assert_eq!(r[1].items[0].get("pname").and_then(Value::as_str).as_deref(), Some("Yugi"));
    assert_eq!(b.pushed().len(), 1);
}

#[test]
fn a_banlist_is_chosen_kept_and_sent_at_the_next_login() {
    let s = Server::with_world(world());
    let d = directory();
    let mut c = s.join_from(&d, addr(1));
    let user = d.lock().unwrap().account_for(addr(1)).user_id;
    assert_eq!(login_regulation(&mut c), 5, "the server's login-regulation until the player picks");
    let shown = ask(&mut c, "banlist");
    assert!(shown.contains("Season 16 (the server's)") && shown.contains("Unlimited") && shown.contains("Season 17"), "{shown}");

    let done = ask(&mut c, "banlist unlimited");
    assert!(done.contains("Unlimited") && done.contains("next log in"), "applies at the next login, and says so: {done}");
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, Some(3));
    assert!(ask(&mut c, "banlist").contains("Unlimited (your choice)"));
    // Names the way login-regulation reads them: any case, or an id.
    assert!(ask(&mut c, "banlist SEASON 17").contains("now Season 17"));
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, Some(20));
    ask(&mut c, "banlist 3");
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, Some(3));

    // An unknown name is refused, and what was kept stays.
    let refused = ask(&mut c, "banlist season 99");
    assert!(refused.contains("no banlist") && refused.contains("Unlimited"), "{refused}");
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, Some(3));

    // The next login (a new session of the same account) carries it, in both login replies.
    let mut again = s.join_saved(&d, user);
    let auth = one(again.send(req(0x1F47)));
    assert_eq!((auth.request(), auth.get("regulation_id")), (Some(0x219A), Some(&Value::U32(3))));
    assert_eq!(login_regulation(&mut again), 3);
    // `default` goes back to the server's.
    assert!(ask(&mut again, "banlist default").contains("the server's"));
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, None);
    assert_eq!(login_regulation(&mut s.join_saved(&d, user)), 5);
    // Another account is unaffected.
    assert_eq!(login_regulation(&mut s.join_from(&d, addr(2))), 5);
}

#[test]
fn a_banlist_the_client_no_longer_has_falls_back_to_the_servers() {
    let s = Server::with_world(world());
    let d = directory();
    let user = d.lock().unwrap().account_for(addr(1)).user_id;
    d.lock().unwrap().set_banlist_of(user, Some(99)).unwrap();
    assert_eq!(login_regulation(&mut s.join_saved(&d, user)), 5);
}

fn npc_duel(c: &mut Client) -> u32 {
    let rule = Message::record().with("cpu_id", Value::U32(124)).with("match_type", Value::U32(4));
    let duel = Message::reply(0x17AC).with("event_id", Value::U32(1)).with("join_id", Value::U32(1)).item(rule);
    one(c.send(Packet::Message(duel))).get("duel_id").and_then(Value::as_u32).unwrap()
}

fn pick(c: &mut Client, duel_id: u32, name: &str, main: &[u16]) -> u32 {
    let deck = Message::deck().with("deck_name", Value::text(name)).with("main", Value::u16_be_array(main))
        .with("extra", Value::u16_be_array(&[])).with("side", Value::u16_be_array(&[]));
    let Packet::Message(m) = req_with(0x1F4F, &[("duel_id", Value::U32(duel_id)), ("event_id", Value::U32(0)), ("update_type", Value::U32(1))]) else { unreachable!() };
    one(c.send(Packet::Message(m.item(deck)))).request().unwrap()
}

#[test]
fn select_deck_follows_the_players_banlist() {
    let s = Server::with_world(world());
    let d = directory();
    let user = d.lock().unwrap().account_for(addr(1)).user_id;
    let starter: Vec<u16> = (1..=20).chain(1..=20).collect();
    // Under Season 16 the starter deck's third card is forbidden: refused (0x22CB). Under Unlimited it is accepted.
    let mut c = s.join_from(&d, addr(1));
    let id = npc_duel(&mut c);
    assert_eq!(pick(&mut c, id, "Mine", &starter), 0x22CB);
    ask(&mut c, "banlist unlimited");
    let mut c = s.join_saved(&d, user);
    let id = npc_duel(&mut c);
    assert_eq!(pick(&mut c, id, "Mine", &starter), 0x22CA);
    // A player on the server's list still meets it, whatever this one chose.
    let mut other = s.join_from(&d, addr(2));
    let id = npc_duel(&mut other);
    assert_eq!(pick(&mut other, id, "Mine", &starter), 0x22CB);
}

fn auto_match(c: &mut Client) -> Vec<Message> {
    let join = one(c.send(req_with(0x1784, &[("event_id", Value::U32(2))])));
    let join_id = join.get("join_id").and_then(Value::as_u32).unwrap();
    c.send(req_with(0x1798, &[("event_id", Value::U32(2)), ("join_id", Value::U32(join_id)), ("match_type", Value::U32(1)), ("level", Value::U32(0))]))
}

#[test]
fn auto_duel_pairs_only_players_on_the_same_banlist() {
    let s = Server::with_world(World { duel_server: ("10.0.0.5".into(), 4321), ..world() });
    let d = directory();
    let ua = d.lock().unwrap().account_for(addr(1)).user_id;
    let ub = d.lock().unwrap().account_for(addr(2)).user_id;
    d.lock().unwrap().set_banlist_of(ua, Some(3)).unwrap();
    let (mut a, mut b) = (s.join_saved(&d, ua), s.join_saved(&d, ub));
    auto_match(&mut a);
    auto_match(&mut b);
    s.hub.lock().unwrap().match_waiting();
    assert!(a.pushed().is_empty() && b.pushed().is_empty(), "Unlimited and Season 16 are not paired");
    // A third on Unlimited meets the first; the Season 16 player keeps waiting.
    let uc = d.lock().unwrap().account_for(addr(3)).user_id;
    d.lock().unwrap().set_banlist_of(uc, Some(3)).unwrap();
    let mut c = s.join_saved(&d, uc);
    auto_match(&mut c);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let (mut got_a, mut got_c) = (false, false);
    while std::time::Instant::now() < deadline && !(got_a && got_c) {
        got_a |= a.pushed().iter().any(|m| m.request() == Some(0x1978));
        got_c |= c.pushed().iter().any(|m| m.request() == Some(0x1978));
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(got_a && got_c, "the two on Unlimited are matched");
    assert!(b.pushed().iter().all(|m| m.request() != Some(0x1978)), "the Season 16 player is not");
}

#[test]
fn draw_deals_cards_from_the_newest_deck() {
    let s = Server::with_world(world());
    let d = directory();
    let mut c = s.join_from(&d, addr(1));
    // Before any upload it draws from the starter deck (rows 1-20: "Card 1001"-"Card 1020").
    let starter = ask(&mut c, "draw5");
    assert!(starter.starts_with("Draw 5 from \"Starter Deck\" (40 cards): "), "{starter}");
    let names = |text: &str, from: &str| -> Vec<String> {
        text.split_once(from).unwrap().1.split(',').map(|n| n.trim().to_string()).collect()
    };
    let hand = names(&starter, "): ");
    assert_eq!(hand.len(), 5);
    assert!(hand.iter().all(|n| n.strip_prefix("Card 10").and_then(|r| r.parse::<u32>().ok()).is_some_and(|r| (1..=20).contains(&r))), "{hand:?}");

    // A deck the player picked is the newest (the cards it holds must be owned, and legal: Unlimited here).
    s.world.settings.set("login-regulation", "Unlimited").unwrap();
    let user = d.lock().unwrap().account_for(addr(1)).user_id;
    let mut c = s.join_saved(&d, user);
    let mine: Vec<u16> = (1..=20).rev().chain((1..=20).rev()).collect();
    let id = npc_duel(&mut c);
    assert_eq!(pick(&mut c, id, "Mine", &mine), 0x22CA);
    let seven = ask(&mut c, "draw 7");
    assert!(seven.starts_with("Draw 7 from \"Mine\" (40 cards): "), "{seven}");
    let hand = names(&seven, "): ");
    assert_eq!(hand.len(), 7);
    assert!(hand.iter().all(|n| (1001..=1020).contains(&n.strip_prefix("Card ").and_then(|r| r.parse::<u32>().ok()).unwrap())), "{hand:?}");
    // By (part of) a name, even an older deck, and `!draw5` from a chat room; a number is capped.
    assert!(ask(&mut c, "draw 5 starter").contains("\"Starter Deck\""));
    assert!(ask(&mut c, "draw 99").starts_with("Draw 10 from"));
    assert!(ask(&mut c, "draw5 nothing like it").contains("no deck named like"));
    // Random: ten draws are not all the same hand.
    let hands: std::collections::HashSet<String> = (0..10).map(|_| ask(&mut c, "draw5")).collect();
    assert!(hands.len() > 1);
}

#[test]
fn where_lists_the_places_a_card_comes_from() {
    let s = Server::with_world(world());
    let mut c = s.join(1001, "Yugi");
    let two = ask(&mut c, "where card 1007");
    assert_eq!(two, "Card 1007: Booster Pack 3; Blister's Trade Card (cards from Booster Packs 1-14)", "any case, a part of the name");
    assert_eq!(ask(&mut c, "where CARD 1008"), "Card 1008: Booster Pack 3; Blister's Rental");
    assert!(ask(&mut c, "where card 1009").contains("no normal route"));
    assert!(ask(&mut c, "where card 10").contains("Be more specific"), "many cards match: a few are named");
    assert!(ask(&mut c, "where zzz").contains("No card is named like"));
    assert!(ask(&mut c, "where").starts_with("Usage"));
}

/// #73: before buying, `pack NAME` lists every card of a pack, rarest first, at the shop's price; `pack` lists the packs.
#[test]
fn pack_lists_a_packs_cards_by_rarity() {
    let s = Server::with_world(world());
    let mut c = s.join(1001, "Yugi");
    let three = ask(&mut c, "pack booster pack 3");
    assert_eq!(three, "Booster Pack 3 (100 BP for 5 cards):\nSuper Rare: Card 1008\nRare: Card 1007\nCommon: Card 1001, Card 1002\n\
        Every card is on the Packs page: Main Menu, Leaderboard, then Packs in the bar.", "an exact name, any case");
    assert!(ask(&mut c, "pack").starts_with("Packs: Booster Pack 3, Booster Pack 4"), "the list");
    assert!(ask(&mut c, "pack booster").contains("2 packs match"), "a part of two names");
    assert!(ask(&mut c, "!pack 4").contains("Common: Card 1009"), "one match");
    assert!(ask(&mut c, "pack zzz").contains("No pack is named like"));
    // A tier too long for the chat window is counted, and the page lists it.
    let mut big = world();
    big.pack_lists = vec![PackList::new(9, "Beginner's Pack".into(), String::new(), &[(1..=40).collect(), vec![50]], |row| Some(format!("Card {}", 1000 + row)))];
    let s = Server::with_world(big);
    let mut c = s.join(1001, "Yugi");
    let beginner = ask(&mut c, "pack beginner");
    assert!(beginner.contains("Rare: Card 1050\nCommon: 40 cards (see the Packs page)") && !beginner.contains("Card 1001"), "{beginner}");
}

fn report_world() -> (Server, Arc<Mutex<Directory>>) {
    let s = Server::with_world(world());
    let d = Arc::new(Mutex::new(Directory::new(vec!["Yugi".into()])));
    (s, d)
}

#[test]
fn a_report_reaches_the_staff_with_a_limit_per_sender() {
    let (s, d) = report_world();
    let (mut a, mut b) = (s.join_from(&d, addr(1)), s.join_from(&d, addr(2)));
    // A player is found by name wherever they are, even offline, ignoring case.
    let other = d.lock().unwrap().account_for(addr(2)).user_id;
    d.lock().unwrap().create_player(other, 1, "Joey Wheeler").unwrap();
    let said = ask(&mut a, "report kaiba swears at everyone");
    // Both accounts' first player is "Yugi" in this fixture: the second one is renamed by the directory ("Yugi2").
    assert!(said.contains("No player is named"), "{said}");
    let names: Vec<String> = d.lock().unwrap().entries().flat_map(|(_, _, a)| a.players.iter().map(|p| p.name.clone()).collect::<Vec<_>>()).collect();
    assert!(names.contains(&"Yugi".to_string()) && names.contains(&"Joey Wheeler".to_string()), "{names:?}");

    assert!(ask(&mut a, "report \"joey wheeler\" cheats in duels").contains("sent to the staff (#1)"));
    assert!(ask(&mut a, "report Joey Wheeler spams the chat").contains("sent to the staff (#2)"), "the longest name that is a player's");
    assert!(ask(&mut a, "report Joey").contains("Usage") || ask(&mut a, "report Joey").contains("No player"), "a reason is needed");
    assert!(ask(&mut a, "report Yugi because").contains("can't report yourself"));
    assert!(b.pushed().is_empty(), "the reported player is not told");
    let list = s.hub.lock().unwrap().reports.list(None, 10).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!((list[1].reporter.as_str(), list[1].target.as_str(), list[1].reason.as_str(), list[1].status), ("Yugi", "Joey Wheeler", "cheats in duels", Status::Open));

    // The limit is per sender per hour (5 by default); the others still report.
    for n in 3..=5 {
        assert!(ask(&mut a, &format!("report \"Joey Wheeler\" again {n}")).contains(&format!("(#{n})")));
    }
    let limited = ask(&mut a, "report \"Joey Wheeler\" once more");
    assert!(limited.contains("5 reports in the last hour"), "{limited}");
    assert_eq!(s.hub.lock().unwrap().reports.list(None, 10).unwrap().len(), 5);
    assert!(ask(&mut b, "report Yugi he is rude").contains("sent to the staff (#6)"));
}

#[test]
fn reports_can_be_turned_off() {
    let (s, d) = report_world();
    let mut a = s.join_from(&d, addr(1));
    s.world.settings.set("report-limit", "0").unwrap();
    assert!(ask(&mut a, "report Joey rude").contains("turned off"));
}

fn staff(hub: &Arc<Mutex<yo3_server::front::Hub>>, dir: Option<&std::path::Path>) -> (Api, Arc<Mutex<Directory>>) {
    logins::use_fast_hashing();
    let mut d = Directory::new(vec!["Yugi".into()]);
    let hash = logins::hash_password_now("LongPassword123").unwrap();
    d.register(Login::new("Croquet", hash.clone(), "RECOVERY_ONE".into()), None, Registration::Open).unwrap();
    d.set_role("Croquet", Role::Moderator).unwrap();
    d.register(Login::new("Plain", hash, "RECOVERY_TWO".into()), None, Registration::Open).unwrap();
    let d = Arc::new(Mutex::new(d));
    let mut api = Api::new(d.clone(), Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()), dir.map(|p| p.to_path_buf()), Default::default(), "server");
    api.hub = Some(hub.clone());
    (api, d)
}

#[test]
fn a_moderator_lists_and_handles_reports_and_it_is_audited() {
    let s = Server::with_world(world());
    {
        let mut hub = s.hub.lock().unwrap();
        hub.reports.add((1, "Yugi"), (2, "Kaiba"), "cheats", 5).unwrap();
        hub.reports.add((1, "Yugi"), (3, "Joey"), "rude", 5).unwrap();
    }
    let (api, _) = staff(&s.hub, None);
    let moderator = Some(Role::Moderator);
    let listed = api.apply_as(&Op::Reports { status: Some("open".into()), limit: None }, "Croquet", "panel", moderator).unwrap();
    assert_eq!((listed["open"].as_u64(), listed["reports"].as_array().unwrap().len()), (Some(2), 2));
    assert_eq!(listed["reports"][0]["target"], "Joey", "newest first");
    assert_eq!(listed["reports"][0]["status"], "open");
    assert!(listed["message"].as_str().unwrap().contains("Yugi -> Joey"));

    // A player (no staff role) may not; a moderator may.
    let handle = |id, status: &str| Op::HandleReport { id, status: status.into(), note: "warned".into() };
    assert!(api.apply_as(&handle(1, "handled"), "Plain", "panel", Some(Role::Player)).is_err());
    let done = api.apply_as(&handle(1, "handled"), "Croquet", "panel", moderator).unwrap();
    assert_eq!((done["report"]["status"].as_str(), done["report"]["handled_by"].as_str(), done["report"]["note"].as_str()), (Some("handled"), Some("Croquet"), Some("warned")));
    // The same change again is answered as done and audited once; another status on a closed one is refused.
    api.apply_as(&handle(1, "handled"), "Croquet", "panel", moderator).unwrap();
    assert!(api.apply_as(&handle(1, "dismissed"), "Croquet", "panel", moderator).unwrap_err().contains("already"));
    assert!(api.apply_as(&handle(99, "handled"), "Croquet", "panel", moderator).unwrap_err().contains("No report 99"));
    assert!(api.apply_as(&handle(2, "open"), "Croquet", "panel", moderator).is_err());
    api.apply_as(&handle(2, "dismissed"), "Croquet", "panel", moderator).unwrap();

    let open = api.apply_as(&Op::Reports { status: Some("open".into()), limit: None }, "Croquet", "panel", moderator).unwrap();
    assert_eq!(open["reports"].as_array().unwrap().len(), 0);
    let all = api.apply_as(&Op::Reports { status: None, limit: Some(10) }, "Croquet", "panel", moderator).unwrap();
    assert_eq!(all["reports"][0]["status"], "dismissed");
    let audit: Vec<_> = api.audit(20).unwrap().into_iter().filter(|r| r.op == "handle-report").collect();
    assert_eq!(audit.len(), 2, "{audit:?}");
    assert_eq!((audit[1].actor.as_str(), audit[1].origin.as_str(), audit[1].target.as_str()), ("Croquet", "panel", "Kaiba"));
}

#[test]
fn reports_are_kept_in_the_database_and_go_with_a_deleted_account() {
    let dir = std::env::temp_dir().join(format!("yo3-reports-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let s = Server::with_world(world());
    s.hub.lock().unwrap().reports = Reports::at(&dir);
    let d = directory();
    let mut a = s.join_from(&d, addr(1));
    let user = d.lock().unwrap().account_for(addr(1)).user_id;
    d.lock().unwrap().create_player(user, 1, "Kaiba").unwrap();
    assert!(ask(&mut a, "report Kaiba cheats").contains("(#1)"));
    // A new Reports over the same folder (a restart, or the CLI on a stopped server) sees it.
    let fresh = Reports::at(&dir);
    let list = fresh.list(Some(Status::Open), 10).unwrap();
    assert_eq!((list.len(), list[0].reporter.as_str(), list[0].target.as_str()), (1, "Yugi", "Kaiba"));
    // The rate limit counts what was kept.
    let mut reports = Reports::at(&dir);
    for _ in 0..4 { reports.add((list[0].reporter_id, "Yugi"), (list[0].target_id, "Kaiba"), "more", 5).unwrap(); }
    assert!(reports.add((list[0].reporter_id, "Yugi"), (list[0].target_id, "Kaiba"), "more", 5).is_err());
    assert_eq!(yo3_server::reports::forget(&dir, &[list[0].target_id]).unwrap(), 5);
    assert!(Reports::at(&dir).list(None, 10).unwrap().is_empty());
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn staff_set_an_accounts_banlist_and_the_account_shows_it() {
    let s = Server::with_world(world());
    let (api, d) = staff(&s.hub, None);
    let moderator = Some(Role::Moderator);
    let user = d.lock().unwrap().user_of("Plain").unwrap();
    let detail = api.apply_as(&Op::Account { who: "Plain".into() }, "Croquet", "panel", moderator).unwrap();
    assert_eq!(detail["banlist"]["chosen"], false);
    // Without the game's data the names are unknown: only `default` is accepted, an id is refused.
    assert!(api.apply_as(&Op::SetBanlist { who: "Plain".into(), banlist: "Unlimited".into() }, "Croquet", "panel", moderator).is_err());
    assert!(api.apply_as(&Op::SetBanlist { who: "Plain".into(), banlist: "default".into() }, "Croquet", "panel", moderator).is_ok());
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, None);

    // With the running server's world: the override is audited and the player's next login carries it.
    let mut api = api;
    api.world = Some(Arc::new(world()));
    let done = api.apply_as(&Op::SetBanlist { who: "Plain".into(), banlist: "unlimited".into() }, "Croquet", "panel", moderator).unwrap();
    assert!(done["message"].as_str().unwrap().contains("Unlimited"), "{done}");
    assert_eq!(d.lock().unwrap().account_of(user).unwrap().banlist, Some(3));
    let detail = api.apply_as(&Op::Account { who: "Plain".into() }, "Croquet", "panel", moderator).unwrap();
    assert_eq!((detail["banlist"]["id"].as_u64(), detail["banlist"]["name"].as_str(), detail["banlist"]["chosen"].as_bool()), (Some(3), Some("Unlimited"), Some(true)));
    assert!(api.apply_as(&Op::SetBanlist { who: "Plain".into(), banlist: "season 99".into() }, "Croquet", "panel", moderator).is_err());
    let audit: Vec<_> = api.audit(20).unwrap().into_iter().filter(|r| r.op == "set-banlist").collect();
    assert_eq!(audit.len(), 1);
    assert_eq!((audit[0].actor.as_str(), audit[0].before.clone(), audit[0].after.clone()), ("Croquet", Some(serde_json::json!(null)), Some(serde_json::json!(3))));
    // The same again changes nothing and is not audited twice; a moderator can't act on another staff account.
    api.apply_as(&Op::SetBanlist { who: "Plain".into(), banlist: "unlimited".into() }, "Croquet", "panel", moderator).unwrap();
    assert_eq!(api.audit(20).unwrap().iter().filter(|r| r.op == "set-banlist").count(), 1);
    let choices = api.apply_as(&Op::Banlists, "Croquet", "panel", moderator).unwrap();
    assert_eq!(choices["banlists"]["choices"].as_array().unwrap().len(), 3);
    assert_eq!(choices["banlists"]["server_default"]["name"], "Season 17", "the default login banlist is the built-in id 20");
}

#[test]
fn no_player_can_take_the_servers_name() {
    // Commands are whispers to "Server": a player of that name would get them instead of the server.
    let d = directory();
    let user = d.lock().unwrap().account_for(addr(1)).user_id;
    for name in ["Server", "server", " SERVER "] {
        assert!(d.lock().unwrap().create_player(user, 1, name).is_err(), "{name:?}");
        assert!(d.lock().unwrap().add_player_to(user, name).is_err(), "{name:?}");
        assert!(d.lock().unwrap().rename("Yugi", name).is_err(), "{name:?}");
    }
    assert!(d.lock().unwrap().create_player(user, 1, "Servers").is_ok());
}

/// `bug what happened` sends the player's last duel to the staff: a report about the opponent with the duel's
/// recording id, pinned so the daily pruning keeps it. Without a recent duel the note goes alone.
#[test]
fn bug_sends_the_players_last_duel_pinned_with_the_note() {
    use yo3_server::recordings::{Recordings, Seat};
    let dir = std::env::temp_dir().join(format!("yo3-bug-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let recordings = Arc::new(Recordings::at(&dir));
    let s = Server::with_world(World { recordings: Some(recordings.clone()), ..world() });
    s.hub.lock().unwrap().reports = Reports::at(&dir);
    let d = directory();
    let mut a = s.join_from(&d, addr(1));
    assert!(ask(&mut a, "bug").contains("Usage: bug"));
    assert!(ask(&mut a, "help").contains("bug what happened"));
    let alone = ask(&mut a, "bug my trap did nothing");
    assert!(alone.contains("(#1)") && alone.contains("No recent duel"), "{alone}");
    let me = Reports::at(&dir).list(None, 10).unwrap()[0].reporter_id;

    let duel = recordings.start("npc", &[Seat { player_id: Some(me), name: "Yugi".into() }, Seat { player_id: None, name: "Pegasus".into() }], false, None).unwrap();
    let id = duel.id().to_string();
    // During the duel (the recording still open) as after it.
    let sent = ask(&mut a, "!bug Mirror Force destroyed nothing");
    assert!(sent.contains("against Pegasus") && sent.contains("(#2)"), "{sent}");
    drop(duel);
    let list = Reports::at(&dir).list(None, 10).unwrap();
    let report = &list[0];
    assert_eq!((report.recording.as_deref(), report.duel_problem, report.target.as_str(), report.target_id), (Some(id.as_str()), true, "Pegasus", 0));
    assert_eq!(report.reason, "Duel problem: Mirror Force destroyed nothing");
    assert_eq!((list[1].recording.as_deref(), list[1].duel_problem, list[1].target.as_str()), (None, true, "(no recent duel)"));
    assert!(dir.join(format!("recordings/pinned/{id}.jsonl.gz")).is_file());
    recordings.prune(0).unwrap();
    assert!(recordings.find(&id).is_some(), "pinned: the pruning keeps it");
    // An ordinary report has no recording and isn't a duel problem.
    let staff_view = Reports::at(&dir);
    assert!(staff_view.list(None, 10).unwrap().iter().all(|r| r.duel_problem));
    s.world.settings.set("report-limit", "0").unwrap();
    assert!(ask(&mut a, "bug again").contains("turned off"));
}

/// Nothing should depend on a server staying up: a player on this PC (Singleplayer, or a host playing on their own
/// server: a loopback address) is told where the pinned recording is on their disk, to send it to the developer if they
/// like; a player elsewhere is told the host keeps it.
#[test]
fn bug_tells_a_player_on_this_pc_where_the_recording_is_and_others_that_the_host_keeps_it() {
    use yo3_server::recordings::{Recordings, Seat};
    let dir = std::env::temp_dir().join(format!("yo3-bug-local-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let recordings = Arc::new(Recordings::at(&dir));
    let s = Server::with_world(World { recordings: Some(recordings.clone()), ..world() });
    s.hub.lock().unwrap().reports = Reports::at(&dir);
    let d = directory();
    let mut here = s.join_at(&d, IpAddr::V4(Ipv4Addr::LOCALHOST));
    let mut there = s.join_at(&d, addr(2));
    let ids: Vec<u32> = d.lock().unwrap().accounts().flat_map(|(_, a)| a.players.iter().map(|p| p.player_id)).collect();
    for me in ids {
        drop(recordings.start("npc", &[Seat { player_id: Some(me), name: "Yugi".into() }, Seat { player_id: None, name: "Pegasus".into() }], false, None).unwrap());
    }
    let local = ask(&mut here, "bug the card did nothing");
    let pinned = dir.join("recordings/pinned");
    assert!(local.contains("Saved on this PC") && local.contains(&pinned.display().to_string()), "{local}");
    let remote = ask(&mut there, "bug the card did nothing");
    assert!(remote.contains("kept on this server") && !remote.contains(&pinned.display().to_string()), "{remote}");
}

/// A database made before duel problems gets the reports' `recording` column at the next start.
#[test]
fn an_older_database_gets_the_recording_column() {
    let dir = std::env::temp_dir().join(format!("yo3-bug-schema-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    rusqlite::Connection::open(dir.join("server.db")).unwrap().execute_batch("ALTER TABLE reports DROP COLUMN recording;").unwrap();
    yo3_server::store::initialize(&dir).unwrap();
    let mut reports = Reports::at(&dir);
    reports.add_duel_problem((1, "Yugi"), (0, "Pegasus"), "it froze", Some("20261122-120000-001"), 5).unwrap();
    assert_eq!(reports.list(None, 1).unwrap()[0].recording.as_deref(), Some("20261122-120000-001"));
}
