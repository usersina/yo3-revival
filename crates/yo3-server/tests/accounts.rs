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
