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
}

impl Gate {
    pub fn new(accounts: Arc<Mutex<Directory>>, settings: Arc<crate::settings::Live>, sessions: Arc<Mutex<Sessions>>) -> Self {
        Gate { sessions, accounts, throttle: Mutex::default(), settings }
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
        info.min_launcher = settings.min_launcher.clone();
        info.maintenance = settings.in_maintenance(crate::logins::now());
        info.max_online = u32::try_from(settings.max_players).ok();
        // The news, each body shortened, and as many entries as fit in discovery's answer.
        let short = |text: &str, n: usize| if text.chars().count() > n { text.chars().take(n - 1).collect::<String>() + "…" } else { text.to_owned() };
        info.news = settings.news.iter().map(|(title, body)| yo3_tunnel::News { title: short(title, 128), body: short(body, 400) }).collect();
        while !info.news.is_empty() && serde_json::to_vec(&info).map_or(true, |b| b.len() > yo3_tunnel::MAX_INFO) { info.news.pop(); }
        info
    }
}

async fn handshake(mut tls: TlsStream<TcpStream>, gate: &Gate, info: &yo3_tunnel::ServerInfo, peer: std::net::IpAddr)
    -> anyhow::Result<Option<(TlsStream<TcpStream>, Identity, Instant, Role, u16)>>
{
    let (sessions, accounts) = (&*gate.sessions, &*gate.accounts);
    let mut magic = [0; 8]; tls.read_exact(&mut magic).await.context("read transport opener")?;
    anyhow::ensure!(&magic == MAGIC, "unsupported transport version");
    let operation=tls.read_u8().await.context("read transport operation")?;
    let result=async {
        match operation {
            1 => {
                let protocol = tls.read_u16().await?;
                let abi = tls.read_u16().await?;
                if info.ensure_compatible(protocol, abi).is_err() { tls.write_u8(1).await?; tls.shutdown().await?; return Ok(None) }
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
    accounts: Arc<Mutex<Directory>>, log: Arc<PacketLog>, quiet: net::Idle, heartbeats: net::Idle, http: SocketAddr,
    sessions: Arc<Mutex<Sessions>>)
{
    let gate = Arc::new(Gate::new(accounts.clone(), world.settings.clone(), sessions));
    // The connections open now, against the `max-connections` setting, and each address's handshake allowance (#27).
    let open = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut buckets = crate::logins::Buckets::default();
    loop {
        let (tcp, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => { tracing::warn!("TLS accept: {e}"); tokio::time::sleep(std::time::Duration::from_millis(100)).await; continue }
        };
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
                let online = u32::try_from(hub.locked().online()).ok();
                handshake(tls, &gate, &gate.info(world.engine_problem.is_none(), online), peer.ip()).await
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
            let work = async move {
                match role {
                    Role::Front => {
                        let Some(account) = accounts.locked().account_of(identity.user_id) else { return Ok(()) };
                        Box::pin(net::secured(tls, "front", log, quiet, pace, move |outbox| {
                            let mut front = FrontSession::new(account, world, hub, outbox);
                            front.set_identity(identity);
                            front.set_tunnel_duel_port(port);
                            front.set_directory(accounts);
                            move |p: &yo3_proto::Packet| front.handle(p)
                        })).await
                    }
                    Role::Duel => Box::pin(net::secured(tls, "duel", log, heartbeats, pace, move |outbox| {
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
                        let mut local = TcpStream::connect(http).await?;
                        local.write_all(&header).await?;
                        tokio::io::copy_bidirectional(&mut tls, &mut local).await.map(|_| ())
                    }
                }
            };
            tokio::select! {
                _ = lease.changed() => {},
                result = tokio::time::timeout(expires.saturating_duration_since(Instant::now()), work) => {
                    if let Ok(Err(e)) = result { tracing::debug!("authenticated connection ended: {e}"); }
                }
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
            let gate=Gate::new(Arc::new(Mutex::new(Directory::new(vec![]))),Arc::new(crate::settings::Live::open(None,vec![]).unwrap()),Arc::default());
            handshake(tls,&gate,&yo3_tunnel::ServerInfo::current(true),std::net::Ipv4Addr::LOCALHOST.into()).await.unwrap();
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
