//! Accounts. The client never sends a user id or password (0x1F47 is a bare request), so
//! the server tells accounts apart by source address; players are picked at Select Player.
use std::net::{IpAddr, Ipv4Addr};

use yo3_server::accounts::Directory;

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
fn the_host_names_a_friends_player_ahead_and_renames_players() {
    // The admin commands: a friend's address gets an account with just the chosen player; a player can be renamed,
    // never to a name in use.
    let path = accounts_file("admin");
    let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
    d.add_player(ip(7), "Anis").unwrap();
    assert!(d.add_player(ip(8), "Anis").is_err(), "names are unique");
    d.account_for(ip(1));
    d.rename("Yugi", "Joey").unwrap();
    assert!(d.rename("Joey", "Anis").is_err());
    let mut d = Directory::open(vec!["Yugi".into()], &path).unwrap();
    assert_eq!(names(&d.account_for(ip(7))), ["Anis"]);
    assert_eq!(names(&d.account_for(ip(1))), ["Joey"]);
    let listed: Vec<_> = d.accounts().map(|(addr, a)| (addr, names(a).join(","))).collect();
    assert_eq!(listed, [(ip(1), "Joey".to_string()), (ip(7), "Anis".to_string())]);
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
        d.update_account(a.user_id, |a| a.dpoint = 123);
        d.update_player(a.players[0].player_id, |p| {
            p.cards.insert(38, 2);
        });
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
        d.update_player(id, |p| p.decks = vec![502, 4]);
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
    d.update_player(id, |p| (p.eval_num, p.eval_total) = (3, -15));
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

#[test]
fn the_host_moves_an_account_to_a_friends_new_address() {
    // Accounts are keyed by address only (friends, mail, records and rentals by player and user id), so a friend whose
    // address changed gets their account back by moving it: players, BP and cards go along, kept in the file.
    let file = accounts_file("move");
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let old = d.account_for(ip(1));
    d.update_player(old.players[0].player_id, |p| {
        p.cards.insert(10, 2);
    });
    d.move_account(ip(1), ip(9), false).unwrap();
    let d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let moved: Vec<_> = d.accounts().map(|(addr, a)| (addr, a.user_id, a.players[0].player_id)).collect();
    assert_eq!(moved, [(ip(9), old.user_id, old.players[0].player_id)]);
    assert_eq!(d.player(old.players[0].player_id).unwrap().cards.get(&10), Some(&2));
}

#[test]
fn moving_onto_an_address_with_an_account_needs_replace() {
    // The friend may have connected from the new address first, which made a fresh account there: refused unless the
    // host says to discard it. Moving from an address with no account, or onto itself, is refused.
    let mut d = Directory::new(vec!["Yugi".into()]);
    let old = d.account_for(ip(1));
    let fresh = d.account_for(ip(2));
    assert!(d.move_account(ip(1), ip(2), false).unwrap_err().contains("--replace"));
    assert!(d.move_account(ip(3), ip(4), false).is_err());
    assert!(d.move_account(ip(1), ip(1), true).is_err());
    d.move_account(ip(1), ip(2), true).unwrap();
    let left: Vec<_> = d.accounts().map(|(addr, a)| (addr, a.user_id)).collect();
    assert_eq!(left, [(ip(2), old.user_id)]);
    assert!(d.player(fresh.players[0].player_id).is_none(), "the discarded account's players are gone");
}

// Accounts by key: the key rides in the patch URL (`/yo3/<key>/yo3.idx`), which the client fetches before connecting.

use std::time::{Duration, Instant};
use yo3_server::accounts::{KEY_LEN, KEY_WINDOW};

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
fn the_account_follows_the_key_to_any_address() {
    let mut d = Directory::new(vec!["Yugi".into()]);
    let key = d.add_account(&["Anis".into()]).unwrap();
    let now = Instant::now();
    assert!(d.present_key(ip(1), &key, now));
    let a = d.login(ip(1), now);
    assert_eq!(names(&a), ["Anis"]);
    // The same friend from another address: the same account, not a new one.
    assert!(d.present_key(ip(2), &key, now));
    assert_eq!(d.login(ip(2), now).user_id, a.user_id);
    assert_eq!(d.entries().count(), 1, "no address account was made");
}

#[test]
fn friends_behind_one_address_get_their_own_accounts() {
    // Each starts the game (fetching its key) just before connecting, so the most recent key wins.
    let mut d = Directory::new(vec!["Yugi".into()]);
    let (ka, kb) = (d.add_account(&["Anis".into()]).unwrap(), d.add_account(&["Joey".into()]).unwrap());
    let now = Instant::now();
    d.present_key(ip(1), &ka, now);
    assert_eq!(names(&d.login(ip(1), now)), ["Anis"]);
    d.present_key(ip(1), &kb, now + Duration::from_secs(5));
    assert_eq!(names(&d.login(ip(1), now + Duration::from_secs(6))), ["Joey"]);
}

#[test]
fn an_unknown_key_or_none_falls_back_to_the_address_account() {
    let mut d = Directory::new(vec!["Yugi".into()]);
    let now = Instant::now();
    let by_address = d.account_for(ip(1));
    assert!(!d.present_key(ip(1), "notAKeyOfAnyAccount1", now));
    assert_eq!(d.login(ip(1), now).user_id, by_address.user_id, "unknown key");
    assert_eq!(d.login(ip(3), now).user_id, d.account_for(ip(3)).user_id, "no key: today's behaviour, made on first contact");
    assert_eq!(d.entries().count(), 2);
}

#[test]
fn a_presented_key_counts_for_ten_minutes_and_a_login_renews_it() {
    let mut d = Directory::new(vec!["Yugi".into()]);
    let key = d.add_account(&["Anis".into()]).unwrap();
    let by_address = d.account_for(ip(1));
    let t0 = Instant::now();
    d.present_key(ip(1), &key, t0);
    let almost = t0 + KEY_WINDOW - Duration::from_secs(1);
    assert_eq!(names(&d.login(ip(1), almost)), ["Anis"]);
    // The login renewed it: a reconnect another nine minutes on still finds the key.
    assert_eq!(names(&d.login(ip(1), almost + KEY_WINDOW - Duration::from_secs(1))), ["Anis"]);
    // Left alone past the window it lapses to the address account.
    let late = almost + KEY_WINDOW - Duration::from_secs(1) + KEY_WINDOW + Duration::from_secs(1);
    assert_eq!(d.login(ip(1), late).user_id, by_address.user_id);
    assert_eq!(d.login(ip(1), late).user_id, by_address.user_id, "and stays lapsed");
}

#[test]
fn keys_are_kept_in_the_file_and_old_files_without_keys_load() {
    let file = accounts_file("keys");
    let (key, user) = {
        let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
        let by_address = d.account_for(ip(1));
        let key = d.add_account(&["Anis".into()]).unwrap();
        (key, by_address.user_id)
    };
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    assert!(d.present_key(ip(5), &key, Instant::now()));
    assert_eq!(names(&d.login(ip(5), Instant::now())), ["Anis"]);
    assert_eq!(d.account_for(ip(1)).user_id, user, "the address account is untouched");
    // An old file: addresses only.
    let old = serde_json::json!({ "next_user": 1002, "next_player": 1002, "accounts": [
        { "address": "10.0.0.1", "user_id": 1001, "dpoint": 5000, "players": [{ "player_id": 1001, "name": "Old" }] }] });
    std::fs::write(&file, old.to_string()).unwrap();
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    assert_eq!(names(&d.login(ip(1), Instant::now())), ["Old"]);
    assert_eq!(d.entries().next().unwrap().1, None);
}

#[test]
fn the_host_gives_an_address_account_a_key_without_losing_anything() {
    let file = accounts_file("key-of");
    let mut d = Directory::open(vec!["Duelist".into()], &file).unwrap();
    let before = d.account_for(ip(1));
    d.update_player(before.players[0].player_id, |p| {
        p.cards.insert(10, 2);
    });
    let key = d.key_of("10.0.0.1").unwrap();
    assert_eq!(d.key_of("duelist").unwrap(), key, "by a player's name, ignoring case; the same key again");
    assert!(d.key_of("10.0.0.9").is_err() && d.key_of("Nobody").is_err());
    let mut d = Directory::open(vec!["Duelist".into()], &file).unwrap();
    d.present_key(ip(7), &key, Instant::now());
    let after = d.login(ip(7), Instant::now());
    assert_eq!((after.user_id, after.players[0].player_id), (before.user_id, before.players[0].player_id));
    assert_eq!(d.player(after.players[0].player_id).unwrap().cards.get(&10), Some(&2));
    assert_eq!(d.account_for(ip(1)).user_id, before.user_id, "and the address still works without the key");
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
    assert_eq!(d.accounts().count(), 0, "move-account and the address listing only see address accounts");
}

#[test]
fn a_fetch_without_a_known_key_forgets_the_previous_key_of_that_address() {
    // A friend who removes the key from yo3.ini, or a second friend behind the same address without one, must not
    // land in the account of the key presented minutes ago.
    let mut d = Directory::new(vec!["Yugi".into()]);
    let key = d.add_account(&["Anis".into()]).unwrap();
    let by_address = d.account_for(ip(1));
    let now = Instant::now();
    d.present_key(ip(1), &key, now);
    d.present_no_key(ip(1));
    assert_eq!(d.login(ip(1), now).user_id, by_address.user_id);
    d.present_key(ip(1), &key, now);
    assert!(!d.present_key(ip(1), "wrongwrongwrongwrong", now));
    assert_eq!(d.login(ip(1), now).user_id, by_address.user_id);
}

#[test]
fn the_official_decks_sent_are_kept_and_the_host_can_send_them_again() {
    // Which official decks a player's deck list sent is kept across restarts; `redeliver` forgets it, for a new PC.
    let file = accounts_file("official");
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let id = d.account_for(ip(1)).players[0].player_id;
    d.update_player(id, |p| p.official = vec![20, 28]);
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    assert_eq!(d.player(id).unwrap().official, [20, 28]);
    d.redeliver("yugi").unwrap();
    assert!(Directory::open(vec!["Yugi".into()], &file).unwrap().player(id).unwrap().official.is_empty());
    assert!(d.redeliver("Nobody").is_err());
}
