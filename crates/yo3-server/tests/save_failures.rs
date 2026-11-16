//! A change to the accounts that cannot be saved is not made: memory goes back to what is on disk, the disk keeps
//! what it had, and nobody is told it worked. Before, only purchases and the host's BP commands worked this way; a
//! code redeemed, a duel's reward or a player's change stayed in memory, reported as done, and was gone at the next
//! start (review, 2026-11-15).
mod common;

use std::path::{Path, PathBuf};

use common::{Server, one, req_with};
use yo3_proto::Value;
use yo3_server::accounts::{Directory, START_BP};

/// A stored account with two players, the directory, its folder and the players' ids.
fn stored(name: &str) -> (Directory, PathBuf, u32, u32) {
    let dir = std::env::temp_dir().join(format!("yo3-save-failures-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let mut d = Directory::open(vec!["Yugi".into(), "Kaiba".into()], &dir.join("accounts.json")).unwrap();
    d.add_account(&[]).unwrap();
    d.add_code("ABCDEFGHIJKLMNOP", 300, None).unwrap();
    let players = d.entries().next().unwrap().2.players.iter().map(|p| p.player_id).collect::<Vec<_>>();
    (d, dir, players[0], players[1])
}

/// What the directory's disk holds now.
fn on_disk(dir: &Path) -> String {
    let d = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    format!("{:?}", d.entries().map(|(_, _, a)| a.clone()).collect::<Vec<_>>()) + &format!("{:?}", d.codes())
}

/// What the directory holds in memory.
fn in_memory(d: &Directory) -> String {
    format!("{:?}", d.entries().map(|(_, _, a)| a.clone()).collect::<Vec<_>>()) + &format!("{:?}", d.codes())
}

/// Runs `change` with the database out of reach and checks that it reported failure and changed nothing. The
/// database comes back afterwards.
fn refused(name: &str, change: impl FnOnce(&mut Directory, u32, u32) -> bool) {
    let (mut d, dir, yugi, kaiba) = stored(name);
    let (memory, disk) = (in_memory(&d), on_disk(&dir));
    yo3_server::store::fail_saves(&dir, true).unwrap(); // nothing can be written, as on a full disk
    let reported_success = change(&mut d, yugi, kaiba);
    yo3_server::store::fail_saves(&dir, false).unwrap();
    assert!(!reported_success, "{name}: reported as done");
    assert_eq!(in_memory(&d), memory, "{name}: memory changed");
    assert_eq!(on_disk(&dir), disk, "{name}: disk changed");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_code_redeemed_without_a_save_is_not_redeemed() {
    refused("redeem", |d, _, _| {
        let user = d.entries().next().unwrap().2.user_id;
        d.redeem(user, "ABCDEFGHIJKLMNOP").is_ok()
    });
}

#[test]
fn a_duel_reward_without_a_save_is_not_paid() {
    refused("reward", |d, yugi, _| d.reward(yugi, 100).is_some());
}

#[test]
fn bp_charged_without_a_save_are_not_taken() {
    refused("charge", |d, _, _| {
        let user = d.entries().next().unwrap().2.user_id;
        d.charge(user, 100).is_some()
    });
}

#[test]
fn a_player_change_without_a_save_is_not_made() {
    refused("update-player", |d, yugi, _| d.update_player(yugi, |p| { p.items.insert(2305); }).is_ok());
}

#[test]
fn a_trade_without_a_save_moves_no_card() {
    refused("trade", |d, yugi, kaiba| {
        d.update_players(vec![(yugi, Box::new(|p| { p.cards.insert(10, 1); })), (kaiba, Box::new(|p| { p.cards.insert(11, 1); }))])
    });
}

#[test]
fn an_account_change_without_a_save_is_not_made() {
    refused("update-account", |d, _, _| {
        let user = d.entries().next().unwrap().2.user_id;
        d.update_account(user, |a| a.dpoint = 1).is_ok()
    });
}

#[test]
fn players_made_renamed_or_deleted_without_a_save_are_not() {
    refused("create-player", |d, _, _| {
        let user = d.entries().next().unwrap().2.user_id;
        d.create_player(user, 3, "Joey").is_ok()
    });
    refused("delete-player", |d, _, kaiba| {
        let user = d.entries().next().unwrap().2.user_id;
        d.delete_player(user, kaiba).is_ok()
    });
    refused("rename", |d, _, _| d.rename("Yugi", "Atem").is_ok());
    refused("add-player-to", |d, _, _| {
        let user = d.entries().next().unwrap().2.user_id;
        d.add_player_to(user, "Joey").is_ok()
    });
    refused("set-country", |d, _, _| {
        let user = d.entries().next().unwrap().2.user_id;
        d.set_country_of(user, "JP").is_ok()
    });
    refused("add-account", |d, _, _| d.add_account(&["Joey".to_string()]).is_ok());
}

#[test]
fn the_boosterpass_window_says_no_when_the_code_cannot_be_saved() {
    // 0x20D0 {dpcode} -> 0x23F0 with dpauth_result 1 ("Incorrect BOOSTERPASS CODE.") and no BP gained.
    let (d, dir, _, _) = stored("window");
    let user = d.entries().next().unwrap().2.user_id;
    let directory = std::sync::Arc::new(std::sync::Mutex::new(d));
    let mut c = Server::new().join_saved(&directory, user);
    yo3_server::store::fail_saves(&dir, true).unwrap();
    let r = one(c.send(req_with(0x20D0, &[("dpcode", Value::text("ABCDEFGHIJKLMNOP"))])));
    assert_eq!(r.get("dpauth_result").and_then(Value::as_u32), Some(1), "{r:?}");
    assert_eq!(directory.lock().unwrap().dpoint(user), Some(START_BP));
    std::fs::remove_dir_all(dir).unwrap();
}

/// The BP of `player`'s account on disk.
fn bp_on_disk(dir: &Path, player: u32) -> u32 {
    let d = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    d.entries().find(|(_, _, a)| a.players.iter().any(|p| p.player_id == player)).unwrap().2.dpoint
}

/// `player`'s wins, losses and draws against `opponent` on disk.
fn record_on_disk(dir: &Path, player: u32, opponent: u32) -> Option<[u64; 3]> {
    let saved = yo3_server::store::read(&dir.join("records.json")).unwrap()?;
    saved["records"].as_array()?.iter().find(|r| r["player_id"] == player && r["opponent"] == opponent)
        .map(|r| [r["win"].as_u64().unwrap(), r["lose"].as_u64().unwrap(), r["draw"].as_u64().unwrap()])
}

#[test]
fn a_duels_bp_and_record_are_saved_together_or_not_at_all() {
    // #22: one transaction for a duel's end. When the record can't be saved (a failure on its rows only), the BP the
    // duel paid is not kept either, on disk or in memory, and the record stays as it was.
    let (d, dir, yugi, kaiba) = stored("duel-together");
    let directory = std::sync::Arc::new(std::sync::Mutex::new(d));
    let mut hub = yo3_server::front::Hub::with_records(&dir.join("records.json"));
    hub.set_directory(directory.clone());
    assert_eq!(hub.record_result(yugi, kaiba, 1), [1, 0, 0]);
    let paid = START_BP + yo3_server::shop::duel_bp(1);
    assert_eq!((bp_on_disk(&dir, yugi), record_on_disk(&dir, yugi, kaiba)), (paid, Some([1, 0, 0])), "a duel's end saved");
    let memory = in_memory(&directory.lock().unwrap());
    yo3_server::store::fail_domain(&dir, "records.json", true).unwrap();
    assert_eq!(hub.record_result(yugi, kaiba, 1), [1, 0, 0], "the record as it stands");
    yo3_server::store::fail_domain(&dir, "records.json", false).unwrap();
    assert_eq!(in_memory(&directory.lock().unwrap()), memory, "the BP went back in memory");
    assert_eq!((bp_on_disk(&dir, yugi), record_on_disk(&dir, yugi, kaiba)), (paid, Some([1, 0, 0])), "and on disk");
    // Saving works again: the next duel's end saves both.
    assert_eq!(hub.record_result(yugi, kaiba, 1), [2, 0, 0]);
    assert_eq!((bp_on_disk(&dir, yugi), record_on_disk(&dir, yugi, kaiba)), (paid + yo3_server::shop::duel_bp(1), Some([2, 0, 0])));
    yo3_server::store::close(&dir);
    std::fs::remove_dir_all(dir).unwrap();
}
