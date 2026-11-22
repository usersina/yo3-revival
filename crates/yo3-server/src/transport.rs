//! The sole game listener: TLS, then a bounded credential/ticket handshake, then the stock game's frames.
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use yo3_tunnel::{tokio_rustls::{TlsAcceptor, server::TlsStream}, Role, MAGIC, KEY_LEN, HANDSHAKE_TIMEOUT};
use crate::accounts::Directory;
use crate::auth::{Identity, Sessions};
use crate::front::{FrontSession, Hub, World};
use crate::lock::Locked;
use crate::{log::PacketLog, net};

/// What every connection's handshake shares: sessions, accounts, the login throttle and the server's access settings.
pub(crate) struct Gate {
    /// Shared with the admin API, which ends them (a kick, a ban, maintenance, #26).
    pub sessions: Arc<Mutex<Sessions>>,
    pub accounts: Arc<Mutex<Directory>>,
    pub throttle: Mutex<crate::logins::Throttle>,
    /// The live settings (#24): the account rules and the server list's name and headline.
    pub settings: Arc<crate::settings::Live>,
    /// Players' problem reports (operation 7), shared with the admin API.
    pub problems: Arc<crate::problems::Problems>,
}

impl Gate {
    pub fn new(accounts: Arc<Mutex<Directory>>, settings: Arc<crate::settings::Live>, sessions: Arc<Mutex<Sessions>>,
        problems: Arc<crate::problems::Problems>) -> Self {
        Gate { sessions, accounts, throttle: Mutex::default(), settings, problems }
    }
    /// Discovery's answer: the build, and for the server list its name, headline, who is online and how accounts
    /// are made, from the settings now.
    fn info(&self, ready: bool, online: Option<u32>) -> yo3_tunnel::ServerInfo {
        let settings = self.settings.now();
        let mut info = yo3_tunnel::ServerInfo::current(ready);
        info.name = settings.server_name.clone();
        info.headline = settings.headline.clone();
        info.online = online;
        info.registration = settings.registration.as_str().into();
        info.min_launcher = yo3_tunnel::minimum_launcher(&info.release,settings.min_launcher.as_deref()).map(str::to_owned);
        info.maintenance = settings.in_maintenance(crate::logins::now());
        info.clean_game = settings.clean_game;
        info.max_online = u32::try_from(settings.max_players).ok();
        // The news, each body shortened, and as many entries as fit in discovery's answer.
        let short = |text: &str, n: usize| if text.chars().count() > n { text.chars().take(n - 1).collect::<String>() + "…" } else { text.to_owned() };
        info.news = settings.news.iter().map(|(title, body)| yo3_tunnel::News { title: short(title, 128), body: short(body, 400) }).collect();
        while !info.news.is_empty() && serde_json::to_vec(&info).map_or(true, |b| b.len() > yo3_tunnel::MAX_INFO) { info.news.pop(); }
        info
    }
}

/// A game's front connection ending: its place on the device is free again (#64).
struct GameClosed { sessions: Arc<Mutex<crate::auth::Sessions>>, identity: crate::auth::Identity }
impl Drop for GameClosed { fn drop(&mut self) { self.sessions.locked().set_playing(self.identity, false, usize::MAX); } }

/// Files a problem report under `session`'s account: its id, or why not.
fn problem_report(gate: &Gate, session: &yo3_tunnel::Token, body: &[u8]) -> Result<String, yo3_tunnel::ReportRefused> {
    use yo3_tunnel::ReportRefused;
    if !gate.settings.now().problem_reports { return Err(ReportRefused::Off) }
    let user_id = gate.sessions.locked().user_of(session, Instant::now()).ok_or(ReportRefused::Session)?;
    let request = crate::problems::inflate(body)?;
    let (account, names) = {
        let d = gate.accounts.locked();
        let names = d.player_names(user_id);
        let account = d.login_by_id(user_id).map(|l| l.user.clone()).unwrap_or_else(|| names.join(", "));
        (account, names)
    };
    let now = crate::logins::now();
    let report = gate.problems.accept(user_id, &account, &request, crate::problems::server_lines(user_id, &names, now), now)?;
    tracing::info!("[report] user {user_id} ({account}) sent problem report {report}: {}", request["kind"].as_str().unwrap_or_default());
    Ok(report)
}

/// `info(with_online)`: discovery's answer. Only discovery (operation 5) counts who is online, which takes the hub
/// lock: that is the server list's and the launcher's liveness check of a hung hub, and the other operations (a
/// sign-in, a ticket, a game connection) need not wait behind a busy hub.
async fn handshake(mut tls: TlsStream<TcpStream>, gate: &Gate, info: &(dyn Fn(bool) -> yo3_tunnel::ServerInfo + Sync), peer: std::net::IpAddr)
    -> anyhow::Result<Option<(TlsStream<TcpStream>, Identity, Instant, Role, u16)>>
{
    let (sessions, accounts) = (&*gate.sessions, &*gate.accounts);
    let mut magic = [0; 8]; tls.read_exact(&mut magic).await.context("read transport opener")?;
    anyhow::ensure!(&magic == MAGIC, "unsupported transport version");
    let operation=tls.read_u8().await.context("read transport operation")?;
    let result=async {
        let info = &info(operation == 5);
        match operation {
            1 => {
                let protocol = tls.read_u16().await?;
                let abi = tls.read_u16().await?;
                if info.ensure_compatible(protocol, abi).is_err() { tls.write_u8(1).await?; tls.shutdown().await?; return Ok(None) }
                let length=tls.read_u8().await? as usize;
                anyhow::ensure!((1..=64).contains(&length),"invalid launcher version length");
                let mut version=vec![0;length];tls.read_exact(&mut version).await?;
                if std::str::from_utf8(&version).ok().is_none_or(|v|info.ensure_launcher(v).is_err()) {
                    tls.write_u8(1).await?;tls.shutdown().await?;return Ok(None)
                }
                let mut key = [0; KEY_LEN]; tls.read_exact(&mut key).await?;
                // While saves fail, nobody new comes in: their progress could not be kept (store::healthy).
                if !crate::store::healthy() { tracing::warn!("login refused: the server can't save right now"); tls.write_u8(1).await?; tls.shutdown().await?; return Ok(None) }
                // A key doesn't get past what a password wouldn't (#26): an account that also has a User ID is refused
                // while banned or restricted, and during maintenance only the staff comes in (a key-only account has
                // no role, so it waits like any player).
                let now = crate::logins::now();
                let maintenance = gate.settings.now().in_maintenance(now);
                let max_players = gate.settings.now().max_players;
                let token = {
                    let d = accounts.locked();
                    let user = d.authenticate(&key).map(|a| a.user_id).filter(|&u| {
                        let staff = d.login_by_id(u).is_some_and(|l| l.staff());
                        let full = { let s = sessions.locked(); !s.signed_in(u) && s.users() >= max_players };
                        let allowed = match d.login_by_id(u) { Some(login) => login.refusal(now).is_none(), None => true };
                        allowed && (!maintenance || staff) && (!full || staff)
                    });
                    // Match moderation: account -> sessions. A ban cannot finish between admission and issuance.
                    user.and_then(|u| sessions.locked().login(u, Instant::now()))
                };
                tls.write_u8(if token.is_some() { 0 } else { 1 }).await?;
                if let Some(token) = token { tls.write_all(&token).await?; }
            }
            2 => {
                let mut session = [0; 32]; tls.read_exact(&mut session).await?;
                let role = Role::decode(tls.read_u8().await?);
                let token = role.and_then(|r| sessions.locked().ticket(&session, r, Instant::now()));
                tls.write_u8(if token.is_some() { 0 } else { 1 }).await?;
                if let Some(token) = token { tls.write_all(&token).await?; }
            }
            3 => {
                let mut ticket = [0; 32]; tls.read_exact(&mut ticket).await?;
                let role = Role::decode(tls.read_u8().await?);
                let port = tls.read_u16().await?;
                let claimed = role.and_then(|r| sessions.locked().consume(&ticket, r, Instant::now()).map(|(i, e)| (i, e, r)))
                    .filter(|(i, _, r)| (*r != Role::Front || port != 0) && accounts.locked().account_of(i.user_id).is_some());
                tls.write_u8(if claimed.is_some() { 0 } else { 1 }).await?;
                if let Some((identity, expires, role)) = claimed { return Ok(Some((tls, identity, expires, role, port))) }
            }
            4 => {
                let mut session = [0; 32]; tls.read_exact(&mut session).await?;
                sessions.locked().logout(&session);
                tls.write_u8(0).await?;
            }
            5 => {
                let body = serde_json::to_vec(info)?;
                anyhow::ensure!(body.len() <= yo3_tunnel::MAX_INFO, "discovery response too large");
                tls.write_u16(body.len() as u16).await?;
                tls.write_all(&body).await?;
            }
            7 => {
                // A problem report (problems.rs): the session it is filed under, then its deflated JSON.
                let mut session = [0; 32]; tls.read_exact(&mut session).await?;
                let length = tls.read_u32().await? as usize;
                anyhow::ensure!(length <= yo3_tunnel::MAX_PROBLEM_REPORT, "problem report too large");
                let mut body = vec![0; length]; tls.read_exact(&mut body).await?;
                let answer = problem_report(gate, &session, &body);
                match &answer {
                    Ok(id) => tracing::info!("[report] {peer} problem report {id}"),
                    Err(refused) => tracing::info!("[report] {peer} problem report refused: {refused}"),
                }
                match answer {
                    Ok(id) => { tls.write_u8(0).await?; tls.write_u8(id.len() as u8).await?; tls.write_all(id.as_bytes()).await?; }
                    Err(refused) => tls.write_u8(refused.code()).await?,
                }
            }
            6 => {
                // Account requests (account_ops): a length and a JSON object each way. Never logged as they come.
                let length = tls.read_u16().await? as usize;
                anyhow::ensure!(length <= yo3_tunnel::MAX_ACCOUNT_REQUEST, "account request too large");
                let mut body = vec![0; length]; tls.read_exact(&mut body).await?;
                let reply = match serde_json::from_slice::<serde_json::Value>(&body) {
                    Ok(request) if request.is_object() => {
                        let settings = gate.settings.now();
                        let ctx = crate::account_ops::Context { accounts, sessions, throttle: &gate.throttle, settings: &settings, peer };
                        crate::account_ops::handle(&request, &ctx).await
                    }
                    _ => serde_json::json!({ "ok": false, "error": "bad_request" }),
                };
                let body = serde_json::to_vec(&reply)?;
                tls.write_u16(body.len() as u16).await?;
                tls.write_all(&body).await?;
            }
            _ => anyhow::bail!("unsupported transport operation"),
        }
        tls.shutdown().await.context("send response close_notify")?;
        // A one-shot caller still needs to return its close_notify after reading the reply. Dropping TCP here
        // with that alert unread can reset Windows sockets before the caller finishes receiving the response.
        let mut trailing=[0;1];
        match tokio::time::timeout(std::time::Duration::from_secs(1),tls.read(&mut trailing)).await {
            Ok(Ok(0))=>{},
            Ok(Ok(_))=>tracing::debug!("TLS short-response peer sent trailing application data"),
            Ok(Err(error))=>tracing::debug!("TLS short-response peer close failed: {error}"),
            Err(_)=>tracing::debug!("TLS short-response peer close timed out"),
        }
        Ok(None)
    }.await;
    result.with_context(||format!("transport operation {operation}"))
}

#[expect(clippy::too_many_arguments, reason = "Each authenticated stream shares the server state, framing limits and HTTP endpoint.")]
pub(crate) async fn accept_loop(listener: TcpListener, acceptor: Arc<std::sync::RwLock<TlsAcceptor>>, world: Arc<World>, hub: Arc<Mutex<Hub>>,
    accounts: Arc<Mutex<Directory>>, log: Arc<PacketLog>, idle: net::Idle, http: SocketAddr,
    sessions: Arc<Mutex<Sessions>>, problems: Arc<crate::problems::Problems>)
{
    let gate = Arc::new(Gate::new(accounts.clone(), world.settings.clone(), sessions, problems));
    // The connections open now, against the `max-connections` setting, and each address's handshake allowance (#27).
    let open = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut buckets = crate::logins::Buckets::default();
    loop {
        let (tcp, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => { tracing::warn!("TLS accept: {e}"); tokio::time::sleep(std::time::Duration::from_millis(100)).await; continue }
        };
        // A connected client may stay silent for good (`net::IDLE_TIMEOUT`), so a machine that went away without
        // closing (asleep, its network gone) is found by keepalive, as the launcher finds a host that is gone: the
        // connection drops within about a minute and a half, and a duel it was in ends (`Table::leave`).
        if let Err(e) = yo3_tunnel::keep_alive(&tcp) { tracing::debug!("[tls] {peer} keepalive: {e}"); }
        let settings = gate.settings.now();
        if open.load(std::sync::atomic::Ordering::Relaxed) >= settings.max_connections {
            tracing::warn!("[tls] {peer} refused: {} connections open (max-connections)", settings.max_connections);
            continue;
        }
        // This PC's connections (a launcher's own, the host's tools) aren't counted: a game makes many.
        if !peer.ip().is_loopback() && !handshake_allowed(&mut buckets, peer.ip(), Instant::now(), settings.handshake_burst, settings.handshakes_per_second) {
            tracing::warn!("[tls] {peer} refused: too many connections from this address (handshake-burst)");
            continue;
        }
        let permit = Slot::take(&open);
        // The current certificate: a renewal while running swaps it for the connections after (`lib.rs`).
        let acceptor = acceptor.read().unwrap_or_else(|e| e.into_inner()).clone();
        let (world, hub, accounts, log, gate) = (world.clone(), hub.clone(), accounts.clone(), log.clone(), gate.clone());
        tokio::spawn(async move {
            let _permit = permit;
            let mut stage = "TLS handshake";
            let hello = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
                let tls = acceptor.accept(tcp).await.context("TLS handshake")?;
                stage = "transport handshake";
                handshake(tls, &gate, &|with_online: bool| {
                    let online = if with_online { u32::try_from(hub.locked().online()).ok() } else { None };
                    let mut info = gate.info(world.engine_problem.is_none(), online);
                    info.card_profile = world.profiles.current_hash(world.settings.now().anime_cards);
                    info
                }, peer.ip()).await
            }).await;
            let (mut tls, identity, expires, role, port)=match hello {
                Ok(Ok(Some(connection)))=>connection,
                Ok(Ok(None))=>return,
                Ok(Err(error))=>{tracing::warn!("[tls] {peer} handshake failed: {error:#}");return},
                Err(_)=>{tracing::warn!("[tls] {peer} handshake timed out during {stage}");return},
            };
            let Some(mut lease) = gate.sessions.locked().lease(identity) else { return };
            tracing::info!("[tls] {peer} user {} launcher {} {role:?}", identity.user_id, identity.session_id);
            let pace = gate.settings.now().pace;
            let trail = Arc::new(net::Trail::default());
            let carried = trail.clone();
            let work = async move {
                match role {
                    Role::Front => {
                        let Some(account) = accounts.locked().account_of(identity.user_id) else { return Ok(()) };
                        // Linked accounts (#64): where this account plays from, and the cap of games per device. A game
                        // over the cap (it was started before the launcher's check, or by another route) is closed.
                        accounts.locked().note_seen(identity.user_id, None, Some(peer.ip()), crate::logins::now());
                        let cap = gate.settings.now().games_per_device as usize;
                        if !gate.sessions.locked().set_playing(identity, true, cap) {
                            tracing::warn!("[tls] {peer} user {}: more than {cap} game(s) on one device; closed", identity.user_id);
                            return Ok(());
                        }
                        let _closed = GameClosed { sessions: gate.sessions.clone(), identity };
                        Box::pin(net::secured(tls, "front", log, idle, pace, trail, move |outbox| {
                            let mut front = FrontSession::new(account, world, hub, outbox);
                            front.set_identity(identity);
                            front.set_tunnel_duel_port(port);
                            front.set_directory(accounts);
                            move |p: &yo3_proto::Packet| front.handle(p)
                        })).await
                    }
                    Role::Duel => Box::pin(net::secured(tls, "duel", log, idle, pace, trail, move |outbox| {
                        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
                        let mut duel = crate::duel::DuelSession::new(world, hub, seed).with_outbox(outbox).with_identity(identity);
                        move |p: &yo3_proto::Packet| duel.handle(p)
                    })).await,
                    Role::Http => {
                        let mut header = vec![];
                        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
                            while header.len() < 8192 && !header.ends_with(b"\r\n\r\n") { header.push(tls.read_u8().await?); }
                            Ok::<_, std::io::Error>(())
                        }).await.map_err(std::io::Error::other)??;
                        let request = String::from_utf8_lossy(&header);
                        let path = request.split_whitespace().nth(1).unwrap_or_default();
                        if !request.starts_with("GET ") || !public_http(path) {
                            tls.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
                            return tls.shutdown().await;
                        }
                        // The card profile (#60): what the connection DLL rewrites the client's card table with at start. The
                        // server notes which one this account's game loaded; a game that runs another is refused every duel (`FrontSession::stale_game`).
                        if path.split('?').next() == Some(CARD_PROFILE_PATH) {
                            let body = world.profiles.file(world.settings.now().anime_cards);
                            let loaded = crate::profile::Profile::identify(&body).unwrap_or_default();
                            // The launcher asks too, to place the local decks (`?for=decks`): that proves nothing about the game.
                            let for_decks = launcher_read(path);
                            tracing::info!("[tls] {peer} user {} {} the card profile {} {}", identity.user_id, if for_decks { "(launcher) read" } else { "fetched" }, loaded.0, loaded.1);
                            if !for_decks { world.profiles.note_launch(identity, loaded); }
                            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                            tls.write_all(head.as_bytes()).await?;
                            tls.write_all(body.as_bytes()).await?;
                            return tls.shutdown().await;
                        }
                        // The added cards (#75): the launcher extends the game's card files with them before releasing the game.
                        if path.split('?').next() == Some(ADDED_CARDS_PATH) {
                            let body = world.added.render();
                            tracing::info!("[tls] {peer} user {} read the added cards ({})", identity.user_id, world.added.cards.len());
                            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                            tls.write_all(head.as_bytes()).await?;
                            tls.write_all(body.as_bytes()).await?;
                            return tls.shutdown().await;
                        }
                        let mut local = TcpStream::connect(http).await?;
                        local.write_all(&header).await?;
                        tokio::io::copy_bidirectional(&mut tls, &mut local).await.map(|_| ())
                    }
                }
            };
            // Why a connection ended is said in the log: a player who reports being disconnected can then be answered from it.
            // A game connection's line also says what it carried and its last messages each way (net::Trail): a player's
            // problem report brings this line along (problems.rs).
            let user = identity.user_id;
            let carried = |role: Role| if role == Role::Http { String::new() } else { format!("; {}", carried.summary()) };
            tokio::select! {
                _ = lease.changed() => tracing::info!("[tls] {peer} user {user} {role:?} closed by the server: the session ended (signed out, or the account signed in again: the newest sign-in wins){}", carried(role)),
                result = tokio::time::timeout(expires.saturating_duration_since(Instant::now()), work) => match result {
                    Err(_) => tracing::info!("[tls] {peer} user {user} {role:?} closed by the server: the session's 12 hours are over"),
                    Ok(Err(e)) => tracing::info!("[tls] {peer} user {user} {role:?} ended with an error: {e}{}", carried(role)),
                    Ok(Ok(())) if role == Role::Http => tracing::debug!("[tls] {peer} user {user} {role:?} ended"),
                    Ok(Ok(())) => tracing::info!("[tls] {peer} user {user} {role:?} ended by the game or its helper{}", carried(role)),
                },
            };
        });
    }
}

#[cfg(test)]
mod close_tests {
    use super::*;
    use std::time::Duration;
    use yo3_tunnel::tokio_rustls::{TlsConnector,rustls};

    #[tokio::test]
    async fn short_response_waits_for_peer_close_notify_before_dropping_tcp() {
        let rcgen::CertifiedKey{cert,signing_key}=rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let mut roots=rustls::RootCertStore::empty();roots.add(cert.der().clone()).unwrap();
        let client=rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
        let server=rustls::ServerConfig::builder().with_no_client_auth()
            .with_single_cert(vec![cert.der().clone()],rustls::pki_types::PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into()).unwrap();
        let acceptor=TlsAcceptor::from(Arc::new(server));
        let listener=TcpListener::bind(("127.0.0.1",0)).await.unwrap();let address=listener.local_addr().unwrap();
        let mut peer=tokio::spawn(async move {
            let (tcp,_)=listener.accept().await.unwrap();let tls=acceptor.accept(tcp).await.unwrap();
            let gate=Gate::new(Arc::new(Mutex::new(Directory::new(vec![]))),Arc::new(crate::settings::Live::open(None,vec![]).unwrap()),Arc::default(),Arc::new(crate::problems::Problems::new(None)));
            handshake(tls,&gate,&|_| yo3_tunnel::ServerInfo::current(true),std::net::Ipv4Addr::LOCALHOST.into()).await.unwrap();
        });
        let tcp=TcpStream::connect(address).await.unwrap();
        let mut tls=TlsConnector::from(Arc::new(client)).connect("localhost".try_into().unwrap(),tcp).await.unwrap();
        tls.write_all(MAGIC).await.unwrap();tls.write_u8(5).await.unwrap();
        let length=tls.read_u16().await.unwrap();let mut body=vec![0;length as usize];tls.read_exact(&mut body).await.unwrap();
        let _:yo3_tunnel::ServerInfo=serde_json::from_slice(&body).unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(25),&mut peer).await.is_err(),
            "keep the read half alive until the client can return its TLS close_notify");
        tls.shutdown().await.unwrap();
        tokio::time::timeout(Duration::from_secs(1),peer).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn only_discovery_counts_who_is_online() {
        // Counting takes the hub lock: a ticket, a sign-in or a game connection waiting behind a busy hub (a slow save,
        // the engine) only slowed every player's connections; discovery keeps it, as the launchers' liveness check.
        let rcgen::CertifiedKey{cert,signing_key}=rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let mut roots=rustls::RootCertStore::empty();roots.add(cert.der().clone()).unwrap();
        let client=Arc::new(rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth());
        let server=rustls::ServerConfig::builder().with_no_client_auth()
            .with_single_cert(vec![cert.der().clone()],rustls::pki_types::PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into()).unwrap();
        let acceptor=TlsAcceptor::from(Arc::new(server));
        for (operation, counted) in [(2u8, false), (4, false), (5, true)] {
            let listener=TcpListener::bind(("127.0.0.1",0)).await.unwrap();let address=listener.local_addr().unwrap();
            let acceptor=acceptor.clone();
            let peer=tokio::spawn(async move {
                let (tcp,_)=listener.accept().await.unwrap();let tls=acceptor.accept(tcp).await.unwrap();
                let gate=Gate::new(Arc::new(Mutex::new(Directory::new(vec![]))),Arc::new(crate::settings::Live::open(None,vec![]).unwrap()),Arc::default(),Arc::new(crate::problems::Problems::new(None)));
                let asked=std::sync::atomic::AtomicBool::new(false);
                let info=|online: bool| { asked.fetch_or(online, std::sync::atomic::Ordering::SeqCst); yo3_tunnel::ServerInfo::current(true) };
                handshake(tls,&gate,&info,std::net::Ipv4Addr::LOCALHOST.into()).await.unwrap();
                asked.into_inner()
            });
            let tcp=TcpStream::connect(address).await.unwrap();
            let mut tls=TlsConnector::from(client.clone()).connect("localhost".try_into().unwrap(),tcp).await.unwrap();
            tls.write_all(MAGIC).await.unwrap();tls.write_u8(operation).await.unwrap();
            if operation != 5 { tls.write_all(&[0;32]).await.unwrap(); }
            if operation == 2 { tls.write_u8(Role::Front as u8).await.unwrap(); }
            let mut answer=Vec::new();tls.read_to_end(&mut answer).await.ok();
            tls.shutdown().await.ok();
            assert_eq!(tokio::time::timeout(Duration::from_secs(2),peer).await.unwrap().unwrap(), counted, "operation {operation}");
        }
    }
}

/// One open connection, counted until it ends.
struct Slot(Arc<std::sync::atomic::AtomicUsize>);
impl Slot {
    fn take(open: &Arc<std::sync::atomic::AtomicUsize>) -> Slot { open.fetch_add(1, std::sync::atomic::Ordering::Relaxed); Slot(open.clone()) }
}
impl Drop for Slot {
    fn drop(&mut self) { self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed); }
}

/// Takes one handshake from `addr`'s bucket (`burst` deep, `per_second` back each second); whether there was one.
fn handshake_allowed(buckets: &mut crate::logins::Buckets, addr: std::net::IpAddr, now: Instant, burst: u32, per_second: u32) -> bool {
    buckets.take(addr, now, f64::from(burst.max(1)), f64::from(per_second.max(1))).is_ok()
}

#[cfg(test)]
mod handshake_tests {
    #[test]
    fn an_address_gets_its_burst_then_its_rate() {
        let (mut buckets, now) = (crate::logins::Buckets::default(), std::time::Instant::now());
        let addr: std::net::IpAddr = [203, 0, 113, 9].into();
        assert!((0..5).all(|_| super::handshake_allowed(&mut buckets, addr, now, 5, 2)));
        assert!(!super::handshake_allowed(&mut buckets, addr, now, 5, 2), "the sixth at once waits");
        let later = now + std::time::Duration::from_millis(500);
        assert!(super::handshake_allowed(&mut buckets, addr, later, 5, 2), "one back after half a second at 2 a second");
        assert!(super::handshake_allowed(&mut buckets, [203, 0, 113, 10].into(), now, 5, 2), "another address has its own");
    }
}

/// The path of the card profile (`profile.rs`), under the pages the connection DLL may ask for (`/web/`).
pub const CARD_PROFILE_PATH: &str = "/web/card-profile";

/// The path of the added cards (#75, `yo3_data::added`), read by the launcher through its relay.
pub const ADDED_CARDS_PATH: &str = "/web/added-cards";

/// Whether a profile request is the launcher's own read to place the local decks (`?for=decks`), not the game's fetch.
fn launcher_read(path: &str) -> bool {
    path.split_once('?').is_some_and(|(_, query)| query.split('&').any(|p| p == "for=decks"))
}

#[cfg(test)]
mod profile_read_tests {
    #[test]
    fn only_the_launchers_marked_read_is_not_a_launch() {
        assert!(super::launcher_read("/web/card-profile?for=decks"));
        assert!(!super::launcher_read("/web/card-profile"));
        assert!(!super::launcher_read("/web/card-profile?for=decks2") && !super::launcher_read("/web/card-profile?x=for=decks"));
    }
}

/// The local server's HTTP paths an authenticated launcher may reach: the game's patch and data downloads, duel logs, and
/// the pages behind its old website links (`/web/`, crate::web); never the admin API (`/api/admin/v1/`, api.rs).
fn public_http(path: &str) -> bool {
    // No way out of those folders either (#39): the local server routes by prefix, so `..` must never pass (it
    // decodes no `%`, and a duel log's name may carry a player's name, so those stay).
    let path = path.split('?').next().unwrap_or_default();
    !path.contains("..") && !path.contains('\\')
        && ["/yo3/", "/app/", "/duellogs/", "/ygoda/patch/", "/web/"].iter().any(|p| path.starts_with(p))
}

#[cfg(test)]
mod public_http_tests {
    #[test]
    fn launchers_reach_the_games_files_logs_and_web_pages_only() {
        for ok in ["/yo3/yo3.idx", "/app/", "/duellogs/a.dlog", "/ygoda/patch/x", "/web/v3/bpass/index/"] {
            assert!(super::public_http(ok), "{ok}");
        }
        for no in ["/admin", "/manager", "/api/admin/v1/status", "/web", "/", "/session/x/web/", "/web/../api/admin/v1/status",
            "/yo3/../panel/", "/duellogs/..\\x"] {
            assert!(!super::public_http(no), "{no}");
        }
    }
}
