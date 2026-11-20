//! Linked accounts (#64): which accounts have been seen on the same device or from the same address.
//!
//! Each account keeps the devices (a launcher's install id, kept as a digest) and the addresses it signed in or played
//! from, with when it was last seen. Two accounts that share one are *linked*: a host's fair-play rules (no BP, records
//! or titles from a duel between them, no trade or rental, `linked-accounts` setting) and the dashboard's account view
//! use it. It is a tidiness rule for friends' servers, not anti-cheat: a device id is the launcher's to send and an
//! address can be changed, and a household or a school behind one address shares it.
use std::collections::BTreeMap;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// How long a device counts as the account's.
pub const DEVICE_KEEP: i64 = 90 * 24 * 3600;
/// How long an address does (addresses move: a shorter memory means fewer false links).
pub const ADDRESS_KEEP: i64 = 14 * 24 * 3600;
/// A sighting of something already known is only written again after this long.
const REFRESH: i64 = 6 * 3600;
/// More accounts than this seen at one address and it isn't a PC or a home: a gateway, a proxy, a campus.
pub const CROWD: usize = 8;
/// The most each account keeps of either.
const MOST: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Link { Device, Address }

impl Link {
    pub fn word(self) -> &'static str { match self { Link::Device => "device", Link::Address => "address" } }
}

/// What one account has been seen on, by digest or address text, and when (Unix seconds).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    #[serde(default)]
    pub devices: BTreeMap<String, i64>,
    #[serde(default)]
    pub addresses: BTreeMap<String, i64>,
}

/// The text an address is kept as (an IPv4 address in IPv6 dress is the IPv4 address).
pub fn address_text(ip: IpAddr) -> String {
    match ip { IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or_else(|| v6.to_string(), |v4| v4.to_string()), v4 => v4.to_string() }
}

fn note_in(map: &mut BTreeMap<String, i64>, key: &str, keep: i64, now: i64) -> bool {
    let mut changed = false;
    match map.get(key) {
        Some(&at) if now - at < REFRESH => {}
        _ => { map.insert(key.to_owned(), now); changed = true }
    }
    let before = map.len();
    map.retain(|_, at| now - *at < keep);
    while map.len() > MOST {
        let oldest = map.iter().min_by_key(|(_, at)| **at).map(|(k, _)| k.clone());
        match oldest { Some(k) => { map.remove(&k); } None => break }
    }
    changed || map.len() != before
}

impl Seen {
    /// Notes a sighting; whether anything is worth saving.
    pub fn note(&mut self, device: Option<&str>, address: Option<IpAddr>, now: i64) -> bool {
        let mut changed = false;
        if let Some(device) = device.filter(|d| !d.is_empty()) { changed |= note_in(&mut self.devices, device, DEVICE_KEEP, now) }
        if let Some(address) = address { changed |= note_in(&mut self.addresses, &address_text(address), ADDRESS_KEEP, now) }
        changed
    }

    /// What this account and `other` have in common, at `now`. `crowded` says an address too many accounts share to
    /// mean anything (a gateway or a proxy in front of the server): it links nobody.
    pub fn shared(&self, other: &Seen, now: i64, crowded: &dyn Fn(&str) -> bool) -> Vec<Link> {
        let both = |a: &BTreeMap<String, i64>, b: &BTreeMap<String, i64>, keep: i64, skip: &dyn Fn(&str) -> bool|
            a.iter().any(|(k, at)| now - at < keep && b.get(k).is_some_and(|t| now - t < keep) && !skip(k));
        let mut out = vec![];
        if both(&self.devices, &other.devices, DEVICE_KEEP, &|_| false) { out.push(Link::Device) }
        if both(&self.addresses, &other.addresses, ADDRESS_KEEP, crowded) { out.push(Link::Address) }
        out
    }

    pub fn is_empty(&self) -> bool { self.devices.is_empty() && self.addresses.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> Option<IpAddr> { Some(text.parse().unwrap()) }

    #[test]
    fn two_accounts_on_one_device_or_address_are_linked() {
        let (mut a, mut b, mut c) = (Seen::default(), Seen::default(), Seen::default());
        assert!(a.note(Some("pc-1"), ip("10.0.0.5"), 1000));
        b.note(Some("pc-1"), ip("192.0.2.9"), 1000);
        c.note(Some("pc-2"), ip("10.0.0.5"), 1000);
        assert_eq!(a.shared(&b, 1000, &|_| false), [Link::Device]);
        assert_eq!(a.shared(&c, 1000, &|_| false), [Link::Address]);
        assert!(b.shared(&c, 1000, &|_| false).is_empty());
        assert_eq!(a.shared(&a, 1000, &|_| false), [Link::Device, Link::Address]);
    }

    #[test]
    fn an_address_is_forgotten_sooner_than_a_device_and_an_ipv4_in_ipv6_dress_is_the_same() {
        let (mut a, mut b) = (Seen::default(), Seen::default());
        a.note(Some("pc-1"), ip("10.0.0.5"), 0);
        b.note(Some("pc-1"), ip("::ffff:10.0.0.5"), 0);
        assert_eq!(a.shared(&b, 0, &|_| false), [Link::Device, Link::Address]);
        assert_eq!(a.shared(&b, ADDRESS_KEEP + 1, &|_| false), [Link::Device], "a moved address no longer links");
        assert!(a.shared(&b, DEVICE_KEEP + 1, &|_| false).is_empty());
    }

    #[test]
    fn an_address_everyone_shares_links_nobody() {
        let (mut a, mut b) = (Seen::default(), Seen::default());
        a.note(Some("pc-1"), ip("172.18.0.1"), 0);
        b.note(Some("pc-2"), ip("172.18.0.1"), 0);
        assert_eq!(a.shared(&b, 0, &|_| false), [Link::Address]);
        assert!(a.shared(&b, 0, &|address| address == "172.18.0.1").is_empty(), "a gateway's address");
    }

    #[test]
    fn a_known_sighting_is_not_saved_again_until_it_is_old_and_nothing_unknown_links() {
        let mut a = Seen::default();
        assert!(a.note(Some("d"), ip("10.0.0.1"), 100));
        assert!(!a.note(Some("d"), ip("10.0.0.1"), 200), "nothing new");
        assert!(a.note(Some("d"), ip("10.0.0.1"), 100 + REFRESH), "old enough to move on");
        assert!(!a.note(None, None, 300), "no device and no address (a test client): nothing to remember");
        assert!(a.shared(&Seen::default(), 300, &|_| false).is_empty());
        for n in 0..40 { a.note(Some(&format!("d{n}")), None, 1000 + n); }
        assert!(a.devices.len() <= MOST);
    }
}
