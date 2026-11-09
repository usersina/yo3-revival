//! Country flags: the client reads `country` (an ISO code it looks up in countrytbl.bin; a code it doesn't have, or
//! none, shows Germany) in the login item (the leaderboard link, 0x46FC14), the player records (the list 0x2260,
//! Player Information 0x4B7350, a duel's vsPlayerData 0x4A2D30), the search entries (0x2CF7) and the bracket's seats
//! (0x4CEBE0). No request carries the client's country: it is the account's setting, or follows the login's language.
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Server, one, req, req_with};
use yo3_proto::{Message, Value};
use yo3_server::accounts::Directory;

fn text(m: &Message, k: &str) -> Option<String> {
    m.get(k).and_then(Value::as_str)
}

fn addr() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7))
}

fn directory() -> Arc<Mutex<Directory>> {
    Arc::new(Mutex::new(Directory::new(vec!["Yugi".into(), "Kaiba".into()])))
}

fn login(c: &mut common::Client, lang: u32) -> Message {
    // The login answers 0x2201, then pushes the 0x23F0 that fills premium_expire.
    let replies = c.send(req_with(0x1F48, &[("lang", Value::U32(lang))]));
    assert_eq!(replies.len(), 2, "{replies:?}");
    replies.into_iter().next().unwrap()
}

#[test]
fn without_a_setting_the_flag_follows_the_logins_language() {
    let s = Server::new();
    let d = directory();
    let mut c = s.join_from(&d, addr());
    // French (3): FR in the login item and in the player list.
    let item = login(&mut c, 3).items.remove(0);
    assert_eq!(text(&item, "country").as_deref(), Some("FR"));
    let list = one(c.send(req(0x1F49)));
    assert!(list.items.iter().all(|p| text(p, "country").as_deref() == Some("FR")), "{list:?}");
}

#[test]
fn the_accounts_setting_wins_over_the_language() {
    let s = Server::new();
    let d = directory();
    d.lock().unwrap().account_for(addr());
    d.lock().unwrap().set_country(addr(), "jp").unwrap();
    let mut c = s.join_from(&d, addr());
    let item = login(&mut c, 3).items.remove(0);
    assert_eq!(text(&item, "country").as_deref(), Some("JP"));
    let list = one(c.send(req(0x1F49)));
    assert!(list.items.iter().all(|p| text(p, "country").as_deref() == Some("JP")));
}

#[test]
fn the_flag_reaches_search_entries() {
    let s = Server::new();
    let d = directory();
    d.lock().unwrap().account_for(addr());
    d.lock().unwrap().set_country(addr(), "US").unwrap();
    let mut yugi = s.join_from(&d, addr());
    login(&mut yugi, 1);
    // The other account (another address): its language, German, gives DE.
    let mut kaiba = s.join_from(&d, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)));
    login(&mut kaiba, 2);
    // The players are online from the join, which happened before the login's language; join again the way the
    // client does (login, then select).
    let id_at = |last: u8| d.lock().unwrap().account_for(IpAddr::V4(Ipv4Addr::new(10, 0, 0, last))).players[0].player_id;
    let (yugi_id, kaiba_id) = (id_at(7), id_at(8));
    yugi.send(req_with(0x1F4C, &[("player_id", Value::U32(yugi_id))]));
    kaiba.send(req_with(0x1F4C, &[("player_id", Value::U32(kaiba_id))]));
    let found = one(kaiba.send(req_with(
        0x2CF0,
        &[("search_flag", Value::U32(1)), ("search_type", Value::U32(1)), ("base_point", Value::U32(0)), ("front_num", Value::U32(100)), ("back_num", Value::U32(0))],
    )));
    let flags: Vec<(String, String)> = found.items.iter().map(|i| (text(i, "pname").unwrap(), text(i, "country").unwrap())).collect();
    assert!(flags.contains(&("Yugi".to_string(), "US".to_string())), "{flags:?}");
    assert!(flags.contains(&("Yugi2".to_string(), "DE".to_string())), "{flags:?}");
}

#[test]
fn the_setting_is_kept_and_cleared_with_auto() {
    let dir = std::env::temp_dir().join(format!("yo3-country-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("accounts.json");
    let mut d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    d.account_for(addr());
    assert!(d.set_country(addr(), "Q").is_err(), "a code is two letters");
    assert!(d.set_country(IpAddr::V4(Ipv4Addr::new(10, 9, 9, 9)), "FR").is_err(), "no account there");
    d.set_country(addr(), "fr").unwrap();
    let d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    let account = d.accounts().next().unwrap().1;
    assert_eq!((account.country.as_str(), account.players[0].country.as_str()), ("FR", "FR"));
    let mut d = d;
    d.set_country(addr(), "").unwrap();
    let d = Directory::open(vec!["Yugi".into()], &file).unwrap();
    assert_eq!(d.accounts().next().unwrap().1.country, "");
}

#[test]
fn search_by_country_matches_the_flag() {
    // search_flag 0x8: `country` (text, an ISO code of countrytbl.bin; Detailed Search 0x45F6AB) against the flag each
    // player shows.
    let s = Server::new();
    let d = directory();
    d.lock().unwrap().account_for(addr());
    d.lock().unwrap().set_country(addr(), "JP").unwrap();
    let mut yugi = s.join_from(&d, addr());
    let mut kaiba = s.join_from(&d, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)));
    let ask = |c: &mut common::Client, code: &str| {
        one(c.send(req_with(
            0x2CF0,
            &[("search_flag", Value::U32(0x8)), ("search_type", Value::U32(1)), ("base_point", Value::U32(0)), ("front_num", Value::U32(100)), ("back_num", Value::U32(0)), ("country", Value::text(code))],
        )))
        .items
        .iter()
        .map(|i| text(i, "pname").unwrap())
        .collect::<Vec<_>>()
    };
    // Kaiba (the other account, language English: GB) never logged in with a language here, so it follows the
    // default English; the first account was set to Japan.
    assert_eq!(ask(&mut kaiba, "JP"), ["Yugi"]);
    assert_eq!(ask(&mut yugi, "GB"), ["Yugi2"]);
    assert!(ask(&mut yugi, "FR").is_empty());
}
