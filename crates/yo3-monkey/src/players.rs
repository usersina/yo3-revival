//! Simulated players for the soak (#27): each its own password account, sign-in and launcher proxy, as each real
//! player's launcher has, so the load reaches the server as that many players rather than as one shared session.
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::json;
use yo3_tunnel::Endpoint;

use crate::wire::{Ports, authenticate_proxy, unhex};

const PASSWORD: &str = "monkey passphrase";

/// One simulated player: the local ports its plans and duels use, and the proxy that keeps them open.
pub struct Player {
    pub ports: Ports,
    _proxy: yo3_launcher::Proxy,
}

/// The `i`th player's address: 127.3.0.1 onwards (accounts mode uses 127.2.x.y), so each signs up and connects from
/// its own address, as players do.
fn address(i: usize) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(127, 3, (i / 250) as u8, (i % 250) as u8 + 1))
}

/// `n` players on the server at `endpoint`, whose data folder is `data` (its `add-player` command gives each account
/// its game player). Registration must be open. A player already made by an earlier run signs in again.
pub async fn sign_up(server_bin: &Path, data: &Path, endpoint: &Endpoint, host: &str, n: usize) -> anyhow::Result<Vec<Player>> {
    let mut players = Vec::with_capacity(n);
    for i in 0..n {
        let (user, name) = (format!("player{i}"), format!("Duelist{i}"));
        let endpoint = endpoint.clone().bound_to(address(i));
        let made = endpoint.account(&json!({ "op": "register", "user": user, "password": PASSWORD, "rules": true })).await?;
        anyhow::ensure!(made["ok"] == true || made["error"] == "user_id_taken", "register {user}: {made}");
        // A new account has no player until the game's Create Player; the server's command makes one (once).
        let _ = Command::new(server_bin).args(["add-player", &user, &name, "--data-dir"]).arg(data)
            .stdout(Stdio::null()).stderr(Stdio::null()).status();
        let signed = endpoint.account(&json!({ "op": "login", "user": user, "password": PASSWORD })).await?;
        let session: yo3_tunnel::Token = signed["session"].as_str().and_then(unhex).and_then(|b| b.try_into().ok())
            .ok_or_else(|| anyhow::anyhow!("login {user}: {signed}"))?;
        let proxy = yo3_launcher::Proxy::start(endpoint, session, yo3_launcher::Ports { gate: 0, front: 0, duel: 0, http: 0 }).await?;
        let p = proxy.ports;
        authenticate_proxy(&[p.gate, p.front, p.duel], proxy.cookie);
        let mut ports = Ports { host: host.into(), gate: p.gate, front: p.front, duel: p.duel, http: p.http, pids: [0, 0] };
        let pid = crate::discover_players(&ports, 1).await.ok_or_else(|| anyhow::anyhow!("{user} has no player"))?[0];
        ports.pids = [pid, pid];
        players.push(Player { ports, _proxy: proxy });
    }
    Ok(players)
}
