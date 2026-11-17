//! BOOSTERPASS Points from the host: BOOSTERPASS codes the host makes (`add-code`), entered in the game's BOOSTERPASS
//! window, and `give-bp`, both working on a running server from the host's own PC (`admin.rs`).
//!
//! BpInputWindow (0x40F7B0) sends 0x20D0 {dpcode} (16 characters); the reply 0x23F0 {dpauth_result, dpPoints,
//! dpGetPoints, premium} with result 0 adds `dpGetPoints` to the shown BP; anything else shows "Incorrect BOOSTERPASS
//! CODE." The global dispatcher keeps `premium` from it (0x4F5349), so it goes with 1.
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Client, Server, one, req, req_with};
use yo3_proto::Value;
use yo3_server::accounts::{Directory, START_BP};
use yo3_server::admin;
use yo3_server::api::{self, Op};
use yo3_server::{Config, start};

const ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));
const OTHER: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8));
const CODE: &str = "WELCOME2YO3DUEL1";

/// A client of the account at `addr`, its first player selected.
fn client(server: &Server, d: &Arc<Mutex<Directory>>, addr: IpAddr) -> Client {
    let mut c = server.join_from(d, addr);
    let list = one(c.send(req(0x1F49)));
    let id = list.items[0].get("player_id").and_then(Value::as_u32).unwrap();
    one(c.send(req_with(0x1F4C, &[("player_id", Value::U32(id))])));
    c
}

/// A code entered in the window: (dpauth_result, dpPoints, dpGetPoints).
fn enter(c: &mut Client, code: &str) -> (u32, u32, u32) {
    let r = one(c.send(req_with(0x20D0, &[("dpcode", Value::text(code))])));
    assert_eq!(r.request(), Some(0x23F0));
    assert_eq!(r.get("premium"), Some(&Value::U32(1)), "premium kept");
    // On success the window parses premium_expire ("%4d-%2d-%2d %2d:%2d:%2d", 0x485B50) and localtime()s it (0x485C00):
    // missing, mktime failed and the copy from localtime's null crashed the client (0x485CA1, live).
    let expire = r.get("premium_expire").and_then(Value::as_str).expect("premium_expire");
    assert!(chrono::NaiveDateTime::parse_from_str(&expire, "%Y-%m-%d %H:%M:%S").is_ok_and(|t| t.and_utc().timestamp() < i32::MAX as i64), "{expire}");
    let int = |k| r.get(k).and_then(Value::as_u32).unwrap();
    (int("dpauth_result"), int("dpPoints"), int("dpGetPoints"))
}

fn directory() -> Arc<Mutex<Directory>> {
    Arc::new(Mutex::new(Directory::new(vec!["Yugi".into(), "Kaiba".into()])))
}

#[test]
fn a_host_code_gives_its_bp_once_per_account() {
    let server = Server::new();
    let d = directory();
    d.lock().unwrap().add_code(CODE, 5000, None).unwrap();
    let mut c = client(&server, &d, ADDR);
    // Typed in any case: the window has both.
    assert_eq!(enter(&mut c, &CODE.to_lowercase()), (0, START_BP + 5000, 5000));
    let user = d.lock().unwrap().user_of("Yugi").unwrap();
    assert_eq!(d.lock().unwrap().dpoint(user), Some(START_BP + 5000));
    // Again: refused, the BP as it is.
    assert_eq!(enter(&mut c, CODE), (1, START_BP + 5000, 0));
    // A code nobody made.
    assert_eq!(enter(&mut c, "AAAAAAAAAAAAAAAA"), (1, START_BP + 5000, 0));
    // Another account may use it too.
    let mut other = client(&server, &d, OTHER);
    assert_eq!(enter(&mut other, CODE).0, 0);
}

#[test]
fn a_code_with_uses_runs_out() {
    let server = Server::new();
    let d = directory();
    d.lock().unwrap().add_code(CODE, 100, Some(1)).unwrap();
    assert_eq!(enter(&mut client(&server, &d, ADDR), CODE).0, 0);
    assert_eq!(enter(&mut client(&server, &d, OTHER), CODE).0, 1);
}

#[test]
fn codes_fit_the_window() {
    let d = directory();
    let mut d = d.lock().unwrap();
    for bad in ["SHORT", "WELCOME2YO3DUEL12", "WELCOME-YO3DUEL1", "WELCOME YO3DUEL1"] {
        assert!(d.add_code(bad, 100, None).is_err(), "{bad}");
    }
    assert!(d.add_code(CODE, 0, None).is_err(), "no BP");
    d.add_code(CODE, 100, None).unwrap();
    assert!(d.add_code(&CODE.to_lowercase(), 100, None).is_err(), "the same code in another case");
    assert_eq!(d.codes().len(), 1);
    d.remove_code(&CODE.to_lowercase()).unwrap();
    assert!(d.codes().is_empty());
    assert!(d.remove_code(CODE).is_err());
}

#[test]
fn codes_and_their_uses_are_kept_across_restarts() {
    let dir = std::env::temp_dir().join(format!("yo3-bp-keep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("accounts.json");
    let user = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        d.add_code(CODE, 300, Some(5)).unwrap();
        let user = d.account_for(ADDR).user_id;
        assert_eq!(d.redeem(user, CODE), Ok((300, START_BP + 300)));
        user
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let code = &d.codes()[0];
    assert_eq!((code.bp, code.uses, code.used_by.clone()), (300, Some(5), vec![user]));
    assert!(d.redeem(user, CODE).is_err(), "still used by that account");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn give_bp_pays_an_account_by_a_players_name() {
    let d = directory();
    let mut d = d.lock().unwrap();
    d.account_for(ADDR);
    assert_eq!(d.give_bp("Kaiba", 2500).map(|(_, total)| total), Ok(START_BP + 2500), "the account's second player");
    assert_eq!(d.give_bp("yugi", 500).map(|(_, total)| total), Ok(START_BP + 3000), "ignoring case");
    assert!(d.give_bp("Nobody", 1).is_err());
    assert!(d.give_bp("Kaiba", 0).is_err());
}

#[test]
fn the_bp_a_player_sees_follows_a_gift_on_a_running_server() {
    // give-bp changes the stored account; the session reads it again at the next login reply or purchase.
    let server = Server::new();
    let d = directory();
    let mut c = client(&server, &d, ADDR);
    d.lock().unwrap().give_bp("Yugi", 1000).unwrap();
    let r = one(c.send(req(0x1F47)));
    assert_eq!(r.get("dpoint"), Some(&Value::U32(START_BP + 1000)));
}

#[tokio::test]
async fn the_running_server_takes_admin_commands_from_its_own_pc_with_its_token() {
    let dir = std::env::temp_dir().join(format!("yo3-bp-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let running = start(Config { data_dir: Some(dir.clone()), ..Config::ephemeral(dir.join("logs")) }).await.unwrap();
    // The server says where it listens and with which token, in a file only the host's user can read.
    let live = admin::running(&dir).expect("the server's admin file, and the server answering");
    assert_eq!(live.port, running.http.port());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join(admin::LIVE_FILE)).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "admin file readable by others: {mode:o}");
    }
    let (live2, dir2) = (live.clone(), dir.clone());
    let out = tokio::task::spawn_blocking(move || {
        let made = api::call(&live2, &Op::AddCode { code: CODE.into(), bp: 700, uses: None });
        let listed = api::call(&live2, &Op::Codes).map(|l| l.to_string());
        // A wrong token is refused.
        let forged = api::call(&admin::Live { token: "x".repeat(20), ..live2.clone() }, &Op::Codes);
        let _ = dir2;
        (made, listed, forged)
    })
    .await
    .unwrap();
    assert!(out.0.is_ok(), "{:?}", out.0);
    assert!(out.1.as_ref().is_ok_and(|l| l.contains(CODE) && l.contains("700")), "{:?}", out.1);
    assert!(out.2.is_err(), "a wrong token: {:?}", out.2);
    // Kept: the accounts file has the code (the running server saved it).
    let d = Directory::open(vec!["test".into()], &dir.join("accounts.json")).unwrap();
    assert_eq!(d.codes().len(), 1);
    drop(running);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn every_command_reaches_a_running_server() {
    let dir = std::env::temp_dir().join(format!("yo3-bp-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _running = start(Config { data_dir: Some(dir.clone()), players: vec!["Yugi".into()], ..Config::ephemeral(dir.join("logs")) })
        .await
        .unwrap();
    // The running server holds the accounts; every command goes to it (#25), the ones that rename players too.
    let data = dir.clone();
    let cli = move |args: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).args(args).arg("--data-dir").arg(&data).output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
    };
    let (ok, out) = tokio::task::spawn_blocking(move || {
        let made = cli(&["add-code", CODE, "250", "--uses", "3"]);
        let listed = cli(&["codes"]);
        assert!(cli(&["add-account", "Yugi"]).0);
        let renamed = cli(&["rename", "Yugi", "Atem"]);
        let accounts = cli(&["accounts"]);
        let audit = cli(&["audit"]);
        (vec![made.0, listed.0, renamed.0, accounts.0, audit.0], vec![made.1, listed.1, renamed.1, accounts.1, audit.1])
    })
    .await
    .unwrap();
    assert_eq!(ok, [true; 5], "{out:#?}");
    assert!(out[1].contains(CODE) && out[1].contains("3 of 3 left"), "{}", out[1]);
    assert!(out[2].contains("Yugi is now Atem"), "{}", out[2]);
    assert!(out[3].contains("Atem (#"), "{}", out[3]);
    // The newest change first, each from the running server.
    let rows: Vec<&str> = out[4].lines().collect();
    assert!(rows[0].contains("host (server)\trename\tYugi\t\"Yugi\" -> \"Atem\""), "{}", out[4]);
    assert!(rows[1].contains("create-account") && rows[2].contains("add-code"), "{}", out[4]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_claim_records_the_account_and_when_and_older_claims_load_without_a_time() {
    let dir = std::env::temp_dir().join(format!("yo3-bp-claims-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("accounts.json");
    // A code saved before claim times were kept: one claim, no time.
    std::fs::write(&file, r#"{"next_user":1002,"next_player":1002,"accounts":[],"bp_codes":[{"code":"OLDCODE123456789","bp":10,"used_by":[1001]}]}"#).unwrap();
    let user = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        assert_eq!(d.codes()[0].claimed_at, vec![None]);
        d.add_code(CODE, 300, None).unwrap();
        let user = d.account_for(ADDR).user_id;
        d.redeem(user, CODE).unwrap();
        user
    };
    let d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let code = d.codes().iter().find(|c| c.code == CODE).unwrap();
    assert_eq!(code.used_by, vec![user]);
    let at = code.claimed_at[0].as_deref().expect("claim time kept");
    assert!(chrono::DateTime::parse_from_rfc3339(at).is_ok(), "{at}");
    assert_eq!(d.player_names(user), ["Yugi"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_random_code_fits_the_window_and_is_new() {
    let d = directory();
    let mut d = d.lock().unwrap();
    let code = d.new_code();
    assert_eq!(code.len(), yo3_server::accounts::CODE_LEN);
    assert!(code.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()), "{code}");
    d.add_code(&code, 100, None).unwrap();
    assert_ne!(d.new_code(), code);
}

#[test]
fn bp_paid_for_a_duel_reaches_the_players_game_at_once() {
    // The client shows BP only from the login, a purchase, a rental and the BOOSTERPASS window; the server pushes
    // REVIVAL_BP {revival_bp} for the connection DLL to apply (tools/mod/bp.h).
    let server = Server::new();
    let d = directory();
    server.hub.lock().unwrap().set_directory(d.clone());
    let mut c = client(&server, &d, ADDR);
    c.pushed();
    let players: Vec<u32> = d.lock().unwrap().accounts().flat_map(|(_, a)| a.players.iter().map(|p| p.player_id)).collect();
    server.hub.lock().unwrap().record_result(players[0], players[1], 1);
    let bp = c.pushed().into_iter().find(|m| m.request() == Some(yo3_server::front::REVIVAL_BP)).expect("BP pushed");
    assert_eq!(bp.get("revival_bp"), Some(&Value::U32(START_BP + 100)));
}

#[test]
fn a_host_gift_arrives_as_a_d_mail_with_the_bell_icon() {
    // 0x2581 only sets the connection's mail flag (global case 0x4F5565), so it is safe on any screen; the mail
    // waits for the D-Mail window (0x2583 count, 0x2582 fetch).
    let server = Server::new();
    let d = directory();
    server.hub.lock().unwrap().set_directory(d.clone());
    let mut c = client(&server, &d, ADDR);
    c.pushed();
    let named = d.lock().unwrap().named_players_of("Yugi");
    assert_eq!(named.len(), 2, "both players of the account");
    let mut api = yo3_server::api::Api::new(d.clone(), std::sync::Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()),
        None, std::path::PathBuf::new(), "server");
    api.hub = Some(server.hub.clone());
    api.apply(&yo3_server::api::Op::GiveBp { who: "Yugi".into(), amount: 2000 }, "host").unwrap();
    assert!(c.pushed().iter().any(|m| m.request() == Some(0x2581)), "the bell's mail icon");
    let count = one(c.send(req(0x2583)));
    assert_eq!(count.get("count"), Some(&Value::U32(1)));
    let mail = one(c.send(req_with(0x2582, &[("count", Value::U32(1))])));
    let item = &mail.items[0];
    assert_eq!(item.get("title").and_then(Value::as_str).as_deref(), Some("BP received"));
    assert_eq!(item.get("from_name").and_then(Value::as_str).as_deref(), Some("Host"));
    assert!(item.get("body").and_then(Value::as_str).is_some_and(|b| b.contains("2000 BP") && b.contains(&(START_BP + 2000).to_string())));
}

/// The host's gift of cards or a deck recipe (#19) on a running server: a D-Mail from "Host" with the bell's icon; for
/// cards the player's new trunk at once (as after a trade, 0x2BC9), so no screen waits for its next trunk request; and a
/// present (0x2774 sets the flag the main screen's startup step 0x473AE0 tests, then PresentWindow asks 0x2775): the
/// cards with their pictures, or a recipe with its cards as a structure deck (type 1, which sets the recipe's item in the
/// game itself). Live: back on the map from Kaiba Corporation, the window showed both Blue-Eyes of artwork 5.
#[test]
fn a_host_gift_of_cards_or_a_recipe_is_mailed_and_the_cards_arrive_at_once() {
    use yo3_server::gifts::{Card, Gifts, Recipe};
    let mut world = yo3_server::front::World::new((1..=20).collect());
    world.card_rows = 5000;
    let server = Server::with_world(world);
    let d = directory();
    server.hub.lock().unwrap().set_directory(d.clone());
    let mut c = client(&server, &d, ADDR);
    c.pushed();
    let settings = Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let gifts = Gifts {
        cards: vec![Card { row: 816, cid: 4007, art: 1, name: "Blue-Eyes White Dragon".into() }, Card { row: 817, cid: 4007, art: 2, name: "Blue-Eyes White Dragon".into() }],
        recipes: vec![Recipe { id: 7, title: "E-HERO EVOLUTION DECK".into(), item: 769, cards: vec![816] }],
    };
    let mut a = api::Api::new(d.clone(), settings, None, std::path::PathBuf::new(), "server").with_gifts(gifts);
    a.hub = Some(server.hub.clone());
    a.world = Some(server.world.clone());
    a.apply(&Op::GiveCards { player: "Yugi".into(), card: "4007".into(), art: Some(2), count: Some(3) }, "host").unwrap();
    let pushed = c.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(0x2581)), "the bell's mail icon");
    let trunk = pushed.iter().find(|m| m.request() == Some(9000)).expect("the new trunk");
    let cards = trunk.items[0].get("cards").and_then(Value::as_bytes).unwrap();
    assert_eq!((cards.len(), cards[817 - 1]), (5000, 3), "row 817 at index 816");
    let mail = one(c.send(req_with(0x2582, &[("count", Value::U32(1))])));
    let item = &mail.items[0];
    assert_eq!(item.get("title").and_then(Value::as_str).as_deref(), Some("Cards received"));
    assert!(item.get("body").and_then(Value::as_str).is_some_and(|b| b.contains("3 × Blue-Eyes White Dragon (artwork 2)")));
    let waiting = pushed.iter().find(|m| m.request() == Some(0x2774)).expect("the present flag");
    assert_eq!(waiting.get("present_count"), Some(&Value::U32(1)));
    let present = one(c.send(req(0x2775)));
    assert_eq!((present.request(), present.get("type"), present.get("pack_id")), (Some(0x2777), Some(&Value::U32(0)), Some(&Value::U32(0))));
    assert_eq!(present.get("item").and_then(Value::as_bytes), Some(&[3, 49, 3, 49, 3, 49][..]), "row 817 three times");
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778), "no more");
    // A recipe alone: the mail, no trunk and no present (its window would show the deck's cards as given).
    a.apply(&Op::GiveRecipe { player: "Yugi".into(), deck: "7".into(), cards: false }, "host").unwrap();
    let pushed = c.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(0x2581)), "{pushed:?}");
    assert!(!pushed.iter().any(|m| m.request() == Some(9000) || m.request() == Some(0x2774)), "{pushed:?}");
    // With its cards: the trunk, and the deck as a present of type 1, its id as item's first u16.
    a.apply(&Op::GiveRecipe { player: "Yugi".into(), deck: "7".into(), cards: true }, "host").unwrap();
    let pushed = c.pushed();
    assert!(pushed.iter().any(|m| m.request() == Some(9000)) && pushed.iter().any(|m| m.request() == Some(0x2774)), "{pushed:?}");
    let present = one(c.send(req(0x2775)));
    assert_eq!((present.get("type"), present.get("item").and_then(Value::as_bytes)), (Some(&Value::U32(1)), Some(&[0, 7][..])));
    let mail = one(c.send(req_with(0x2582, &[("count", Value::U32(3))])));
    assert!(mail.items.iter().any(|m| m.get("title").and_then(Value::as_str).as_deref() == Some("Recipe received")), "{mail:?}");
}

#[test]
fn a_gift_to_a_player_who_is_away_waits_for_the_present_window_at_their_login() {
    // The login's 0x2201 sets the present flag (startup step 0x473AE0 opens PresentWindow), so it asks 0x2775 at once.
    use yo3_server::gifts::{Card, Gifts};
    let server = Server::new();
    let d = directory();
    server.hub.lock().unwrap().set_directory(d.clone());
    let settings = Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap());
    let gifts = Gifts { cards: vec![Card { row: 816, cid: 4007, art: 1, name: "Blue-Eyes White Dragon".into() }], recipes: vec![] };
    let mut a = api::Api::new(d.clone(), settings, None, std::path::PathBuf::new(), "server").with_gifts(gifts);
    a.hub = Some(server.hub.clone());
    drop(client(&server, &d, ADDR)); // the account's first visit makes its players
    a.apply(&Op::GiveCards { player: "Yugi".into(), card: "4007".into(), art: None, count: Some(9) }, "host").unwrap();
    let mut c = client(&server, &d, ADDR);
    let present = one(c.send(req(0x2775)));
    assert_eq!(present.get("item").and_then(Value::as_bytes).map(<[u8]>::len), Some(10), "five copies at most, a pack's worth");
    assert_eq!(one(c.send(req(0x2775))).request(), Some(0x2778));
}
