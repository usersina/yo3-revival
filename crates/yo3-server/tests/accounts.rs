//! Accounts. The client never sends a user id or password (0x1F47 is a bare request), so
//! the server tells accounts apart by source address; players are picked at Select Player.
use std::net::{IpAddr, Ipv4Addr};

use yo3_server::accounts::Directory;

#[test]
fn a_failed_manager_commit_restores_the_live_directory() {
    let dir = std::env::temp_dir().join(format!("yo3-account-transaction-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let mut d = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    d.add_account(&["Original".into()]).unwrap();
    yo3_server::store::fail_saves(&dir, true).unwrap();
    let failed = d.atomic(|d| d.add_account(&["MustNotExist".into()]));
    assert!(failed.is_err());
    assert_eq!(d.entries().count(), 1, "failed write cannot publish an in-memory account");
    let fresh = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    assert_eq!(fresh.entries().count(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, last))
}

#[test]
fn one_account_per_address() {
    let mut d = Directory::new(vec!["Yugi".into()]);
    let a1 = d.account_for(ip(1));
    let a2 = d.account_for(ip(1));
    assert_eq!(a1.user_id, a2.user_id);
    assert_eq!(a1.players[0].player_id, a2.players[0].player_id);
}

#[test]
fn different_addresses_get_different_accounts_and_players() {
    let mut d = Directory::new(vec!["Yugi".into(), "Kaiba".into()]);
    let a = d.account_for(ip(1));
    let b = d.account_for(ip(2));
    assert_ne!(a.user_id, b.user_id);
    let ids: Vec<u32> = a.players.iter().chain(&b.players).map(|p| p.player_id).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "player ids are unique across accounts: {ids:?}");
    assert_eq!(a.players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Yugi", "Kaiba"]);
    assert_eq!(a.players[1].index, 1, "pindex follows the slot");
}

#[test]
fn an_account_holds_at_most_four_players() {
    let names = (1..=6).map(|i| format!("P{i}")).collect();
    assert_eq!(Directory::new(names).account_for(ip(1)).players.len(), 4);
}

fn names(a: &yo3_server::front::Account) -> Vec<&str> {
    a.players.iter().map(|p| p.name.as_str()).collect()
}

/// A fresh accounts file in the temp dir.
fn accounts_file(test: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("yo3-accounts-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("accounts.json")
}

#[test]
fn player_names_are_unique_across_accounts() {
    // Friends get the same starting names; a number tells them apart in chat, search and duels.
    let mut d = Directory::new(vec!["Yugi".into(), "Kaiba".into()]);
    assert_eq!(names(&d.account_for(ip(1))), ["Yugi", "Kaiba"]);
    assert_eq!(names(&d.account_for(ip(2))), ["Yugi2", "Kaiba2"]);
    assert_eq!(names(&d.account_for(ip(3))), ["Yugi3", "Kaiba3"]);
}

#[test]
fn accounts_are_kept_in_their_file() {
    let path = accounts_file("kept");
    let (a, b) = {
        let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
        (d.account_for(ip(1)), d.account_for(ip(2)))
    };
    // A restart reads them back; new accounts continue the numbering.
    let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
    let again = d.account_for(ip(1));
    assert_eq!((again.user_id, again.players[0].player_id, names(&again)), (a.user_id, a.players[0].player_id, vec!["Yugi"]));
    let c = d.account_for(ip(3));
    assert!(c.user_id > b.user_id && c.players[0].player_id > b.players[0].player_id);
    assert_eq!(names(&c), ["Yugi3"]);
}

#[test]
fn a_deleted_starter_deck_is_kept_in_the_file_issue_85() {
    let path = accounts_file("starter");
    let id = {
        let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
        let id = d.account_for(ip(1)).players[0].player_id;
        d.update_player(id, |p| p.starter_deleted = true).unwrap();
        id
    };
    let d = Directory::open(vec!["Yugi".into()], &path).unwrap();
    assert!(d.player(id).unwrap().starter_deleted);
}

#[test]
fn the_host_names_a_friends_player_ahead_and_renames_players() {
    // The admin commands: a friend's account made with just the chosen player; a player can be renamed, never to a
    // name in use.
    let path = accounts_file("admin");
    let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
    d.add_account(&["Anis".into()]).unwrap();
    assert!(d.add_account(&["Anis".into()]).is_err(), "names are unique");
    d.account_for(ip(1));
    d.rename("Yugi", "Joey").unwrap();
    assert!(d.rename("Joey", "Anis").is_err());
    let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
    assert_eq!(names(&d.account_for(ip(1))), ["Joey"]);
    let listed: Vec<_> = d.entries().map(|(_, _, a)| names(a).join(",")).collect();
    assert_eq!(listed, ["Anis", "Joey"]);
}

#[test]
fn bp_and_bought_cards_are_kept_with_the_account() {
    // New accounts start with BOOSTERPASS Points (the shop's BP, the account's dpoint); BP and a player's bought cards
    // (CARD_Prop row -> count) survive a restart.
    let dir = std::env::temp_dir().join(format!("yo3-bp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("accounts.json");
    let addr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));
    let (user_id, player_id) = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        let a = d.account_for(addr);
        assert_eq!(a.dpoint, yo3_server::accounts::START_BP);
        d.update_account(a.user_id, |a| a.dpoint = 123).unwrap();
        d.update_player(a.players[0].player_id, |p| {
            p.cards.insert(38, 2);
        }).unwrap();
        (a.user_id, a.players[0].player_id)
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let a = d.account_for(addr);
    assert_eq!((a.user_id, a.dpoint), (user_id, 123));
    let p = a.players.into_iter().find(|p| p.player_id == player_id).unwrap();
    assert_eq!(p.cards.get(&38), Some(&2));
}

#[test]
fn bought_structure_decks_are_kept_with_the_player() {
    let dir = std::env::temp_dir().join(format!("yo3-decks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("accounts.json");
    let addr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8));
    let player_id = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        let id = d.account_for(addr).players[0].player_id;
        d.update_player(id, |p| p.decks = vec![502, 4]).unwrap();
        id
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let p = d.account_for(addr).players.into_iter().find(|p| p.player_id == player_id).unwrap();
    assert_eq!(p.decks, [502, 4]);
}

#[test]
fn manner_ratings_are_kept_across_restarts() {
    let dir = std::env::temp_dir().join(format!("yo3-ratings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("accounts.json");
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let id = d.account_for(ip(1)).players[0].player_id;
    d.update_player(id, |p| (p.eval_num, p.eval_total) = (3, -15)).unwrap();
    let d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let p = d.player(id).unwrap();
    assert_eq!((p.eval_num, p.eval_total), (3, -15));
}

#[test]
fn player_ids_are_never_reused_after_a_delete_and_a_restart() {
    // next_player came from the highest surviving id: delete the newest player, restart, and the next one got its id
    // (and with it the old one's friends, mail and rentals).
    let file = std::env::temp_dir().join(format!("yo3-ids-{}", std::process::id())).join("accounts.json");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    let deleted = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        let account = d.account_for(ip(1));
        d.create_player(account.user_id, 1, "Kaiba").unwrap();
        let kaiba = d.account_of(account.user_id).unwrap().players[1].player_id;
        d.delete_player(account.user_id, kaiba).unwrap();
        kaiba
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let account = d.account_for(ip(1));
    d.create_player(account.user_id, 1, "Joey").unwrap();
    let joey = d.account_of(account.user_id).unwrap().players[1].player_id;
    assert_ne!(joey, deleted);
    // And a new account's user id keeps counting too.
    let next = d.account_for(ip(2));
    assert!(next.user_id > account.user_id);
}

#[test]
fn a_charge_takes_from_the_stored_bp_not_from_a_copy() {
    // A purchase wrote the session's cached BP back as an absolute amount, undoing a reward paid since (a duel). The
    // debit now runs on the stored value, under the lock, and refuses what is not there.
    let mut d = Directory::new(vec!["Yugi".into()]);
    let account = d.account_for(ip(1));
    let stale = account.dpoint;
    assert_eq!(d.reward(account.players[0].player_id, 180), Some(stale + 180));
    assert_eq!(d.charge(account.user_id, 100), Some(stale + 80));
    assert_eq!(d.dpoint(account.user_id), Some(stale + 80));
    assert_eq!(d.charge(account.user_id, stale + 81), None, "not enough left");
    assert_eq!(d.dpoint(account.user_id), Some(stale + 80), "and nothing was taken");
}

#[test]
fn several_players_change_together_or_not_at_all() {
    // A trade hands cards both ways: four saves meant a crash or a missing player left it half done.
    let mut d = Directory::new(vec!["Yugi".into(), "Kaiba".into()]);
    let players = d.account_for(ip(1)).players;
    let (a, b) = (players[0].player_id, players[1].player_id);
    let both: Vec<yo3_server::accounts::PlayerChange> =
        vec![(a, Box::new(|p| p.level = 5)), (b, Box::new(|p| p.level = 6))];
    assert!(d.update_players(both));
    assert_eq!((d.player(a).unwrap().level, d.player(b).unwrap().level), (5, 6));
    let with_a_ghost: Vec<yo3_server::accounts::PlayerChange> =
        vec![(a, Box::new(|p| p.level = 9)), (4242, Box::new(|p| p.level = 9))];
    assert!(!d.update_players(with_a_ghost));
    assert_eq!(d.player(a).unwrap().level, 5, "the first was not changed either");
}

// Accounts by key: the launcher's own Singleplayer account and the host's tools sign in with one (TLS operation 1).

use yo3_server::accounts::KEY_LEN;

#[test]
fn a_key_is_random_and_url_safe() {
    let mut d = Directory::new(vec!["Yugi".into()]);
    let keys: Vec<String> = (0..20).map(|i| d.add_account(&[format!("P{i}")]).unwrap()).collect();
    for k in &keys {
        assert_eq!(k.len(), KEY_LEN);
        assert!(k.bytes().all(|b| b.is_ascii_alphanumeric()), "{k}");
    }
    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len());
}

#[test]
fn keys_are_kept_in_the_file_and_address_only_accounts_are_dropped() {
    let file = accounts_file("keys");
    let key = Directory::open(vec!["Yugi".into()], &file).unwrap().add_account(&["Anis".into()]).unwrap();
    let d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    assert_eq!(names(&d.authenticate(key.as_bytes()).unwrap()), ["Anis"]);
    // An account found only by the address it connected from has no way in any more (#23): it is not loaded.
    let old = serde_json::json!({ "next_user": 1002, "next_player": 1002, "accounts": [
        { "address": "10.0.0.1", "user_id": 1001, "dpoint": 5000, "players": [{ "player_id": 1001, "name": "Old" }] }] });
    std::fs::write(&file, old.to_string()).unwrap();
    assert_eq!(Directory::open(vec!["Yugi".into()], &file).unwrap().entries().count(), 0);
}

#[test]
fn the_host_prints_an_accounts_key_by_a_player_name() {
    let file = accounts_file("key-of");
    let mut d = Directory::open(vec!["Duelist".into()], &file).unwrap();
    let key = d.add_account(&["Anis".into()]).unwrap();
    assert_eq!(d.key_of("anis").unwrap(), key, "by a player's name, ignoring case; the same key again");
    assert!(d.key_of("10.0.0.9").is_err() && d.key_of("Nobody").is_err());
}

#[test]
fn add_account_takes_one_to_four_unique_players() {
    let mut d = Directory::new(vec!["Yugi".into(), "Kaiba".into()]);
    d.account_for(ip(1));
    d.add_account(&[]).unwrap();
    let starters: Vec<_> = d.entries().map(|(_, _, a)| names(a).join("+")).collect();
    assert_eq!(starters, ["Yugi+Kaiba", "Yugi2+Kaiba2"], "no names: the starting players, made unique");
    assert!(d.add_account(&["yugi".into()]).is_err(), "taken, ignoring case");
    assert!(d.add_account(&["A".into(), "a".into()]).is_err(), "twice in one account");
    assert!(d.add_account(&(1..=5).map(|i| format!("Q{i}")).collect::<Vec<_>>()).is_err());
    assert!(d.add_account(&["".into()]).is_err());
    assert!(d.add_account(&["A".into(), "B".into(), "C".into(), "D".into()]).is_ok());
}

#[test]
fn admin_commands_work_on_key_accounts_by_player_name() {
    let mut d = Directory::new(vec!["Yugi".into()]);
    d.add_account(&["Anis".into()]).unwrap();
    let user = d.user_of("anis").unwrap();
    assert_eq!(d.user_of("Nobody"), None);
    d.add_player_to(user, "Anis2").unwrap();
    assert!(d.add_player_to(user, "Anis").is_err(), "taken");
    d.set_country_of(user, "tn").unwrap();
    let account = d.account_of(user).unwrap();
    assert_eq!(names(&account), ["Anis", "Anis2"]);
    assert_eq!(account.country, "TN");
    assert!(account.players.iter().all(|p| p.country == "TN"));
    d.rename("Anis2", "Joey").unwrap();
    assert_eq!(names(&d.account_of(user).unwrap()), ["Anis", "Joey"]);
    assert!(d.set_country_of(user, "Tunisia").is_err());
}

#[test]
#[ignore = "a measurement, not a check"]
fn save_cost() {
    let dir = std::env::temp_dir().join(format!("yo3-save-cost-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    yo3_server::store::initialize(&dir).unwrap();
    let mut d = Directory::open(vec!["A".into(), "B".into(), "C".into(), "D".into()], &dir.join("accounts.json")).unwrap();
    for _ in 0..300 { d.add_account(&[]).unwrap(); }
    let players: Vec<u32> = d.entries().flat_map(|(_, _, a)| a.players.iter().map(|p| p.player_id).collect::<Vec<_>>()).collect();
    let started = std::time::Instant::now();
    for &p in &players { d.purchase(p, 0, None, |player| { for c in 0..1500u16 { player.cards.insert(c * 3, 2); } }).unwrap(); }
    let filled = started.elapsed();
    let mut one = Vec::new();
    for (n, &p) in players.iter().step_by(60).take(20).enumerate() {
        let started = std::time::Instant::now();
        d.purchase(p, 1, None, |player| { player.cards.insert(9000 + n as u16, 1); }).unwrap();
        one.push(started.elapsed());
    }
    one.sort();
    println!("{} players with 1,500 cards each (filled in {filled:?}); one purchase: median {:?}, worst {:?}", players.len(), one[10], one[19]);
    let fresh = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    assert_eq!(fresh.entries().count(), 300, "everything was kept");
    std::fs::remove_dir_all(dir).unwrap();
}
