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
