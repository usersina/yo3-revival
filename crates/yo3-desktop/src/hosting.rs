//! Host for friends (#37): a server of your own that friends join, like a game's "Open to LAN". It is the same
//! `yo3-server` as Singleplayer's, with its own data folder (`<app data>/hosting/`), listening on every interface on
//! the hosting port, with password accounts made with invite codes. It lives while the launcher is open (`--owned`).
//! Friends get an invite link that carries the address and the server's identity (`multiplayer::invite_link`).
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use anyhow::{Context, ensure};
use serde_json::Value;
use yo3_server::{admin, api::{self, Op}};
use crate::{Backend, Settings, multiplayer::{self, Server}};

/// The hosted server's name when the host gave none.
pub const DEFAULT_NAME: &str = "Friends' server";
/// Where the admin panel (#36) is served on the server's loopback port: its sign-in page, which answers before any
/// sign-in (the panel's other pages redirect there).
pub fn panel_path() -> String { format!("{}/login", yo3_server::panel::PREFIX) }

/// A hosted server, as friends and the host see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hosted {
    /// The server in friends' lists: the address to share and the identity.
    pub server: Server,
    /// The invite link friends paste under Other servers.
    pub link: String,
    /// Whether the address is this PC's network address (friends on the same network only), not one the host typed.
    pub local_network: bool,
}

/// The hosted server's state, for the host.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Activity { pub online: usize, pub players: Vec<String> }

/// The address friends dial, without the port: the one the host typed, else this PC's network address.
pub fn share_host(settings: &Settings) -> Option<(String, bool)> {
    if !settings.host_share.trim().is_empty() { return Some((settings.host_share.trim().to_string(), false)) }
    lan_address().map(|ip| (ip.to_string(), true))
}

/// This PC's address on its local network: the first private IPv4 address of an interface that is up.
pub fn lan_address() -> Option<IpAddr> {
    let mut found: Vec<IpAddr> = if_addrs::get_if_addrs().ok()?.into_iter()
        .filter(|i| i.oper_status != if_addrs::IfOperStatus::Down)
        .map(|i| i.ip()).filter(|ip| matches!(ip, IpAddr::V4(v4) if v4.is_private())).collect();
    found.sort_by_key(|ip| !matches!(ip, IpAddr::V4(v4) if v4.octets()[0] == 192));
    found.into_iter().next()
}

/// The server friends see, for `identity` at `host`:`port`.
pub fn hosted_server(name: &str, host: &str, port: u16, identity: &str) -> anyhow::Result<Server> {
    let address = if host.parse::<std::net::Ipv6Addr>().is_ok() { format!("[{host}]:{port}") } else { format!("{host}:{port}") };
    let (address, server_name) = crate::server_address(&address)?;
    Ok(Server { name: name.into(), address, server_name, identities: vec![identity.into()], official: false, description: String::new(), saved_user: None, last_used: 0 })
}

/// The server's name: the host's, else the default.
pub fn name_of(settings: &Settings) -> &str { if settings.host_name.trim().is_empty() { DEFAULT_NAME } else { settings.host_name.trim() } }

impl Backend {
    /// The hosted server's data, apart from Singleplayer's.
    pub fn hosting_dir(&self) -> PathBuf { self.root.join("hosting") }

    /// The hosted server, when it runs.
    pub fn hosting_live(&self) -> Option<admin::Live> { admin::running(&self.hosting_dir()) }

    /// Starts the hosted server (or finds it running) and returns what to share.
    pub fn start_hosting(&self, settings: &Settings) -> anyhow::Result<Hosted> {
        self.check_server_resources()?;
        let mut owned = self.hosted.lock().unwrap();
        let data = self.hosting_dir();
        let (share, local_network) = share_host(settings).context("This PC has no network address to share: connect it to a network, or type the address friends use.")?;
        if self.hosting_live().is_none() {
            ensure!(owned.as_mut().is_none_or(|p| p.try_wait().ok().flatten().is_some()), "Your hosted server is still starting.");
            let port = settings.host_port;
            // Your own Singleplayer server on the same port gives way when nobody plays on it.
            if self.live(settings).is_some() && settings.listen_port == port {
                ensure!(!self.game_running(), "Your Singleplayer game uses port {port}. Close it first, or change the hosting port.");
                self.stop_server(settings)?;
            }
            ensure!(!crate::local_port_answers(port), "Port {port} is already used by another program. Close it, or change the hosting port.");
            crate::validate_game(std::path::Path::new(&settings.game_dir))?;
            yo3_server::auth::maintain_identity(&data, vec!["localhost".into(), "127.0.0.1".into(), share.clone()])?;
            yo3_server::store::initialize(&data)?;
            let live = yo3_server::settings::Live::open(Some(&data.join("settings.json")), crate::banlists(std::path::Path::new(&settings.game_dir)))?;
            live.set("server-name", name_of(settings)).map_err(anyhow::Error::msg)?;
            // Accounts are made with the invite codes the host hands out: the server's default registration ("invite").
            drop(live);
            let mut cmd = self.hosting_command(settings)?;
            crate::detach_server(&mut cmd);
            cmd.arg("--owned").stdin(Stdio::piped());
            let (mut child, errors) = crate::process::spawn(&mut cmd, &self.diagnostics, "Hosted server")?;
            *self.hosted_owner.lock().unwrap() = child.stdin.take();
            *owned = Some(child);
            let started = std::time::Instant::now();
            while self.hosting_live().is_none() {
                if let Some(status) = owned.as_mut().unwrap().try_wait()? {
                    *owned = None;
                    anyhow::bail!("Your hosted server stopped during startup ({status}).\n{}", errors.finish());
                }
                if started.elapsed() > std::time::Duration::from_secs(15) {
                    let _ = owned.as_mut().unwrap().kill();
                    *owned = None;
                    anyhow::bail!("Your hosted server did not become ready within 15 seconds.\n{}", errors.finish());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            self.diagnostics.line(&format!("Hosting for friends on port {port}"));
        } else if let Some(live) = self.hosting_live() {
            // Already running (this launcher started it): a new name applies at once, as a live setting.
            let _ = api::call(&live, &Op::Set { name: "server-name".into(), value: name_of(settings).into() });
        }
        let identity = yo3_tunnel::identity_from_pem(&std::fs::read(data.join("server.crt")).context("Your hosted server's identity is missing")?)?;
        let server = hosted_server(name_of(settings), &share, settings.host_port, &identity)?;
        Ok(Hosted { link: multiplayer::link_for(&server), server, local_network })
    }

    /// The hosted server's command: every interface on the hosting port, no packet logs (players' privacy).
    pub fn hosting_command(&self, settings: &Settings) -> anyhow::Result<Command> {
        let mut cmd = Command::new(&self.runtime.server);
        cmd.args(["--bind", "0.0.0.0", "--auth-port", &settings.host_port.to_string(), "--http-port", "0", "--no-packet-log"])
            .arg("--data-dir").arg(self.hosting_dir()).arg("--game-dir").arg(&settings.game_dir).arg("--log-dir").arg(self.root.join("logs/hosting"))
            .arg("--diagnostics-dir").arg(self.root.join("logs/sessions")).arg("--diagnostic-parent").arg(self.diagnostics.id())
            .env("YO3_ENGINE_DIR", &self.runtime.engine).env("YO3_ENGINE", &self.runtime.engine_program).env("NO_COLOR", "1")
            .stdout(Stdio::null()).stderr(Stdio::null());
        if crate::options::detailed(settings) { cmd.env("RUST_LOG", "debug"); }
        Ok(cmd)
    }

    /// Stops the hosted server (friends' duels end as draws) and waits for its port to close.
    pub fn stop_hosting(&self) -> anyhow::Result<()> {
        let mut owned = self.hosted.lock().unwrap();
        let Some(live) = self.hosting_live() else { *owned = None; return Ok(()) };
        api::call(&live, &Op::Shutdown).map_err(anyhow::Error::msg)?;
        *self.hosted_owner.lock().unwrap() = None;
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(10) {
            if let Some(child) = owned.as_mut() && child.try_wait()?.is_some() { *owned = None }
            if owned.is_none() && self.hosting_live().is_none() { return Ok(()) }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        anyhow::bail!("Your hosted server is still stopping. Wait a few seconds, then try again.")
    }

    /// Who is on the hosted server now.
    pub fn hosting_activity(&self) -> anyhow::Result<Activity> {
        let live = self.hosting_live().context("Your hosted server isn't running.")?;
        let status = api::call(&live, &Op::Status).map_err(anyhow::Error::msg)?;
        let players: Vec<String> = status["players"].as_array().into_iter().flatten()
            .filter_map(|p| p["name"].as_str().or_else(|| p.as_str()).map(str::to_owned)).collect();
        Ok(Activity { online: status["online"].as_u64().map_or(players.len(), |n| n as usize), players })
    }

    /// A new invite code for one friend's account, good for a week.
    pub fn hosting_invite(&self) -> anyhow::Result<String> {
        let live = self.hosting_live().context("Start hosting first.")?;
        let reply: Value = api::call(&live, &Op::Invite { uses: Some(1), days: Some(7) }).map_err(anyhow::Error::msg)?;
        reply["code"].as_str().map(str::to_owned).context("The server made no invite code")
    }

    /// The admin panel's address on this PC (#36), when the hosted server serves one.
    pub fn hosting_panel(&self) -> Option<String> {
        use std::io::{Read, Write};
        let live = self.hosting_live()?;
        let mut stream = std::net::TcpStream::connect_timeout(&([127, 0, 0, 1], live.port).into(), std::time::Duration::from_secs(1)).ok()?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).ok()?;
        let path = panel_path();
        write!(stream, "GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").ok()?;
        let mut head = [0u8; 64];
        let n = stream.read(&mut head).ok()?;
        String::from_utf8_lossy(&head[..n]).split_whitespace().nth(1).filter(|code| *code == "200")
            .map(|_| format!("http://127.0.0.1:{}{path}", live.port))
    }
}

/// Whether the hosted server answers at the address friends use, with its own identity. From this PC that also needs
/// the router to loop back to it, so a failure says "maybe", not "no".
pub async fn reachable(hosted: &Hosted) -> bool {
    match yo3_tunnel::discover(&hosted.server.address, &hosted.server.server_name).await {
        Ok(seen) => hosted.server.identities.contains(&seen.identity),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hosted_server_is_shared_by_a_link_that_names_it_and_its_identity() {
        let identity = "ab".repeat(32);
        let server = hosted_server("Friday duels", "203.0.113.7", 7444, &identity).unwrap();
        let link = multiplayer::link_for(&server);
        let back = multiplayer::invite_link(&link).unwrap();
        assert_eq!((back.name.as_str(), back.address.as_str(), back.key()), ("Friday duels", "203.0.113.7:7444", identity.as_str()));
        assert_eq!(hosted_server("x", "::1", 7443, &identity).unwrap().address, "[::1]:7443");
    }

    #[test]
    fn the_address_to_share_is_the_hosts_own_else_this_pcs_network_address() {
        let typed = Settings { host_share: " play.example.com ".into(), ..Settings::default() };
        assert_eq!(share_host(&typed), Some(("play.example.com".into(), false)));
        if let Some((lan, local)) = share_host(&Settings::default()) {
            assert!(local && lan.parse::<IpAddr>().is_ok_and(|ip| matches!(ip, IpAddr::V4(v4) if v4.is_private())));
        }
        assert_eq!(name_of(&Settings::default()), DEFAULT_NAME);
    }

    #[test]
    fn the_hosted_server_has_its_own_data_listens_for_friends_and_keeps_no_packet_logs() {
        let root = std::env::temp_dir().join(format!("yo3-hosting-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let backend = Backend::new(root.clone()).unwrap();
        let settings = Settings { host_port: 7450, ..Settings::default() };
        assert_ne!(backend.hosting_dir(), backend.data_dir(&settings), "apart from Singleplayer's");
        let cmd = backend.hosting_command(&settings).unwrap();
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        for pair in [["--bind", "0.0.0.0"], ["--auth-port", "7450"]] { assert!(args.windows(2).any(|w| w == pair), "{pair:?} in {args:?}") }
        assert!(args.contains(&"--no-packet-log".to_string()));
        assert!(args.windows(2).any(|w| w[0] == "--data-dir" && w[1] == backend.hosting_dir().to_string_lossy()));
        drop(backend);
        std::fs::remove_dir_all(root).unwrap();
    }
}
