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
use yo3_server::admin::{self, Op};
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
        let user = d.login(ADDR, std::time::Instant::now()).user_id;
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
fn give_bp_pays_an_account_by_a_players_name_or_address() {
    let d = directory();
    let mut d = d.lock().unwrap();
    d.login(ADDR, std::time::Instant::now());
    assert_eq!(d.give_bp("Kaiba", 2500).map(|(_, total)| total), Ok(START_BP + 2500), "the account's second player");
    assert_eq!(d.give_bp(&ADDR.to_string(), 500).map(|(_, total)| total), Ok(START_BP + 3000));
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
        let made = admin::send(&live2, &Op::AddCode { code: CODE.into(), bp: 700, uses: None });
        let listed = admin::send(&live2, &Op::Codes);
        // A wrong token is refused.
        let forged = admin::send(&admin::Live { token: "x".repeat(20), ..live2.clone() }, &Op::Codes);
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
async fn the_commands_reach_a_running_server_and_the_file_editing_ones_wait_for_it_to_stop() {
    let dir = std::env::temp_dir().join(format!("yo3-bp-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _running = start(Config { data_dir: Some(dir.clone()), players: vec!["Yugi".into()], ..Config::ephemeral(dir.join("logs")) })
        .await
        .unwrap();
    // An account for the running server to hold (as a friend connecting would make).
    let data = dir.clone();
    let cli = move |args: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).args(args).arg("--data-dir").arg(&data).output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
    };
    let (ok, out) = tokio::task::spawn_blocking(move || {
        let made = cli(&["add-code", CODE, "250", "--uses", "3"]);
        let listed = cli(&["codes"]);
        let renamed = cli(&["rename", "Yugi", "Atem"]);
        (vec![made.0, listed.0, !renamed.0], vec![made.1, listed.1, renamed.1])
    })
    .await
    .unwrap();
    assert_eq!(ok, [true, true, true], "{out:#?}");
    assert!(out[1].contains(CODE) && out[1].contains("3 of 3 left"), "{}", out[1]);
    assert!(out[2].contains("stop it first"), "{}", out[2]);
    let _ = std::fs::remove_dir_all(&dir);
}
