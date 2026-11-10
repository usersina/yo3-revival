//! Interface selection is explicit; a missing saved interface never widens the listening scope.
use std::net::IpAddr;
use anyhow::{Context, ensure};
use crate::Settings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindAddress { pub interface_id: Option<String>, pub label: String, pub address: IpAddr }
impl std::fmt::Display for BindAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{} — {}", self.label, self.address) }
}
pub fn defaults() -> Vec<BindAddress> {
    [("All IPv4 interfaces", "0.0.0.0"), ("Local computer only (IPv4)", "127.0.0.1"),
        ("All IPv6 interfaces", "::"), ("Local computer only (IPv6)", "::1")].into_iter()
        .map(|(label, ip)| BindAddress { interface_id: None, label: label.into(), address: ip.parse().unwrap() }).collect()
}
pub fn bindings() -> anyhow::Result<Vec<BindAddress>> {
    let mut choices = defaults();
    for interface in if_addrs::get_if_addrs().context("Cannot enumerate network interfaces")? {
        if interface.oper_status == if_addrs::IfOperStatus::Down { continue }
        let address = interface.ip();
        if address.is_loopback() || address.is_unspecified() || matches!(address, IpAddr::V6(ip) if ip.is_unicast_link_local()) { continue }
        #[cfg(windows)] let id = interface.adapter_name.clone();
        #[cfg(not(windows))] let id = format!("{:?}:{}", interface.index, interface.name);
        let choice = BindAddress { interface_id: Some(id), label: interface.name, address };
        if !choices.contains(&choice) { choices.push(choice); }
    }
    choices[4..].sort_by_key(|c| (c.label.clone(), c.address));
    Ok(choices)
}
pub fn resolve_bind(settings: &Settings, available: &[BindAddress]) -> anyhow::Result<IpAddr> {
    let requested: IpAddr = settings.bind_address.parse().context("Choose a valid listening IP address")?;
    if let Some(id) = &settings.bind_interface {
        let candidates: Vec<_> = available.iter().filter(|b| b.interface_id.as_ref() == Some(id) && b.address.is_ipv4() == requested.is_ipv4()).collect();
        return candidates.iter().find(|b| b.address == requested).or_else(|| candidates.first()).map(|b| b.address)
            .context("The selected interface is unavailable. Refresh and choose an interface before starting; the server was not exposed on another interface.");
    }
    ensure!(requested.is_unspecified() || requested.is_loopback() || available.iter().any(|b| b.address == requested), "The selected listening address is not assigned to this computer");
    Ok(requested)
}
pub fn advertised(settings: &Settings) -> anyhow::Result<(String, String)> {
    let input = settings.host_address.trim().strip_prefix("yo3://").unwrap_or(settings.host_address.trim());
    let (address, name) = crate::server_address(input)?;
    let explicit_port = input.parse::<std::net::SocketAddr>().is_ok() || (input.parse::<IpAddr>().is_err() && input.contains(':'));
    if explicit_port { return Ok((address, name)) }
    let host = if name.parse::<std::net::Ipv6Addr>().is_ok() { format!("[{name}]") } else { name.clone() };
    Ok((format!("{host}:{}", settings.listen_port), name))
}

/// A bind address or localhost can serve local play but cannot be presented as an invitation address for friends.
pub fn share_address(settings: &Settings) -> anyhow::Result<Option<(String, String)>> {
    let (address, name) = advertised(settings)?;
    if name.eq_ignore_ascii_case("localhost") || name.eq_ignore_ascii_case("localhost.localdomain") || name.to_ascii_lowercase().ends_with(".localhost") { return Ok(None) }
    if let Ok(ip) = name.parse::<IpAddr>() {
        if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || ip == IpAddr::V4(std::net::Ipv4Addr::BROADCAST) { return Ok(None) }
        if let IpAddr::V6(v6) = ip && let Some(v4) = v6.to_ipv4_mapped() && (v4.is_loopback() || v4.is_unspecified()) { return Ok(None) }
    }
    Ok(Some((address, name)))
}
