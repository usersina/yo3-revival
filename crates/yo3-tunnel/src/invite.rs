//! The invite link a host shares (`yo3revival://server?address=HOST:PORT&identity=HEX&name=NAME`): where the server is
//! and the identity to pin, so no first-use question follows. The server prints it (`yo3-server share`), Desktop's Host
//! for friends shows it, and Desktop's Other servers and the Scripts helper's `--sign-in` read it.
use anyhow::{Context, ensure};

/// A server an invite link names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invite {
    /// `HOST:PORT` (an IPv6 address in brackets).
    pub address: String,
    /// The name the TLS connection asks for: the host part of the address.
    pub server_name: String,
    /// The SHA-256 of the server's certificate key, lower-case hex.
    pub identity: String,
    /// The server's name, as the host set it.
    pub name: String,
}

/// The port when an address has none.
pub const DEFAULT_PORT: u16 = 7443;

/// `HOST`, `HOST:PORT`, `IP`, `[IPv6]:PORT`: the address to connect to and the name its certificate carries.
pub fn address(input: &str) -> anyhow::Result<(String, String)> {
    let input = input.trim();
    let input = input.strip_prefix("yo3://").unwrap_or(input).trim_end_matches('/');
    ensure!(!input.is_empty() && !input.contains(['/', '\\', '@']) && !input.chars().any(char::is_whitespace), "Enter a hostname or IP address, optionally followed by a port.");
    if let Ok(ip) = input.parse::<std::net::IpAddr>() {
        return Ok((match ip { std::net::IpAddr::V6(_) => format!("[{ip}]:{DEFAULT_PORT}"), _ => format!("{ip}:{DEFAULT_PORT}") }, ip.to_string()));
    }
    if let Ok(addr) = input.parse::<std::net::SocketAddr>() {
        ensure!(addr.port() != 0, "Port must be between 1 and 65535.");
        return Ok((addr.to_string(), addr.ip().to_string()));
    }
    let (host, port) = input.rsplit_once(':').unwrap_or((input, "7443"));
    let port: u16 = port.parse().context("Port must be between 1 and 65535.")?;
    ensure!(port > 0 && !host.is_empty() && host.bytes().all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b)), "Enter a valid hostname and port.");
    Ok((format!("{host}:{port}"), host.to_string()))
}

/// Whether `identity` is a SHA-256 in hex.
pub fn valid_identity(identity: &str) -> bool { identity.len() == 64 && identity.bytes().all(|b| b.is_ascii_hexdigit()) }

impl Invite {
    /// The server at `address` (see [`address`]) with `identity`, called `name` (its host when empty).
    pub fn new(address: &str, identity: &str, name: &str) -> anyhow::Result<Self> {
        let identity = identity.trim().to_ascii_lowercase();
        ensure!(valid_identity(&identity), "A server's identity is a SHA-256 in hex (64 characters).");
        let (address, server_name) = self::address(address)?;
        let name = if name.trim().is_empty() { server_name.clone() } else { name.trim().to_string() };
        Ok(Self { address, server_name, identity, name })
    }

    /// Reads an invite link.
    pub fn parse(link: &str) -> anyhow::Result<Self> {
        let rest = link.trim().strip_prefix("yo3revival://server?").context("an invite link starts with yo3revival://server?")?;
        let field = |key: &str| rest.split('&').find_map(|p| p.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| decode(v)));
        let address = field("address").context("the invite link has no address")?;
        let identity = field("identity").context("the invite link has no identity")?;
        ensure!(valid_identity(identity.trim()), "the invite link's identity is malformed");
        Self::new(&address, &identity, &field("name").unwrap_or_default())
    }

    /// The link.
    pub fn link(&self) -> String {
        format!("yo3revival://server?address={}&identity={}&name={}", encode(&self.address), self.identity, encode(&self.name))
    }
}

fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~:".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) { out.push(v); i += 3; continue }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_carries_the_address_identity_and_name_both_ways() {
        let identity = "D".repeat(64);
        let invite = Invite::parse(&format!("yo3revival://server?address=play.example%3A7444&identity={identity}&name=Kaiba%20Land")).unwrap();
        assert_eq!(invite, Invite { address: "play.example:7444".into(), server_name: "play.example".into(), identity: "d".repeat(64), name: "Kaiba Land".into() });
        assert_eq!(invite.link(), format!("yo3revival://server?address=play.example:7444&identity={}&name=Kaiba%20Land", "d".repeat(64)));
        assert_eq!(Invite::parse(&invite.link()).unwrap(), invite);
        let v6 = Invite::new("::1", &identity, "").unwrap();
        assert_eq!((v6.address.as_str(), v6.server_name.as_str(), v6.name.as_str()), ("[::1]:7443", "::1", "::1"));
        assert_eq!(Invite::parse(&v6.link()).unwrap(), v6);
    }

    #[test]
    fn a_link_without_a_good_identity_is_not_an_invite() {
        assert!(Invite::parse("yo3revival://server?address=play.example").is_err());
        assert!(Invite::parse("yo3revival://server?address=play.example&identity=abc").is_err());
        assert!(Invite::parse("https://play.example").is_err());
        assert!(Invite::new("bad host/x", &"a".repeat(64), "").is_err());
    }
}
