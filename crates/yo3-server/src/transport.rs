//! The sole game listener: TLS, then a bounded credential/ticket handshake, then the stock game's frames.
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use yo3_tunnel::{tokio_rustls::{TlsAcceptor, server::TlsStream}, Role, MAGIC, KEY_LEN, HANDSHAKE_TIMEOUT};
use crate::accounts::Directory;
use crate::auth::{Identity, Sessions};
use crate::front::{FrontSession, Hub, World};
use crate::lock::Locked;
use crate::{log::PacketLog, net};

async fn handshake(mut tls: TlsStream<TcpStream>, sessions: &Mutex<Sessions>, accounts: &Mutex<Directory>, info: &yo3_tunnel::ServerInfo)
    -> anyhow::Result<Option<(TlsStream<TcpStream>, Identity, Instant, Role, u16)>>
{
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
                let user = accounts.locked().authenticate(&key).map(|a| a.user_id);
                let token = user.and_then(|u| sessions.locked().login(u, Instant::now()));
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
pub(crate) async fn accept_loop(listener: TcpListener, acceptor: TlsAcceptor, world: Arc<World>, hub: Arc<Mutex<Hub>>,
    accounts: Arc<Mutex<Directory>>, log: Arc<PacketLog>, quiet: net::Idle, heartbeats: net::Idle, http: SocketAddr)
{
    let sessions = Arc::new(Mutex::new(Sessions::default()));
    let slots = Arc::new(Semaphore::new(256));
    loop {
        let (tcp, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => { tracing::warn!("TLS accept: {e}"); tokio::time::sleep(std::time::Duration::from_millis(100)).await; continue }
        };
        let Ok(permit) = slots.clone().try_acquire_owned() else { continue };
        let (acceptor, world, hub, accounts, log, sessions) = (acceptor.clone(), world.clone(), hub.clone(), accounts.clone(), log.clone(), sessions.clone());
        tokio::spawn(async move {
            let _permit = permit;
            let mut stage = "TLS handshake";
            let hello = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
                let tls = acceptor.accept(tcp).await.context("TLS handshake")?;
                stage = "transport handshake";
                handshake(tls, &sessions, &accounts, &yo3_tunnel::ServerInfo::current(world.engine_problem.is_none())).await
            }).await;
            let (mut tls, identity, expires, role, port)=match hello {
                Ok(Ok(Some(connection)))=>connection,
                Ok(Ok(None))=>return,
                Ok(Err(error))=>{tracing::warn!("[tls] {peer} handshake failed: {error:#}");return},
                Err(_)=>{tracing::warn!("[tls] {peer} handshake timed out during {stage}");return},
            };
            let Some(mut lease) = sessions.locked().lease(identity) else { return };
            tracing::info!("[tls] {peer} user {} launcher {} {role:?}", identity.user_id, identity.session_id);
            let work = async move {
                match role {
                    Role::Front => {
                        let Some(account) = accounts.locked().account_of(identity.user_id) else { return Ok(()) };
                        Box::pin(net::secured(tls, "front", log, quiet, move |outbox| {
                            let mut front = FrontSession::new(account, world, hub, outbox);
                            front.set_identity(identity);
                            front.set_tunnel_duel_port(port);
                            front.set_directory(accounts);
                            move |p: &yo3_proto::Packet| front.handle(p)
                        })).await
                    }
                    Role::Duel => Box::pin(net::secured(tls, "duel", log, heartbeats, move |outbox| {
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
                        if !request.starts_with("GET ") || !(path.starts_with("/yo3/") || path.starts_with("/app/") || path.starts_with("/duellogs/") || path.starts_with("/ygoda/patch/")) {
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
            handshake(tls,&Mutex::new(Sessions::default()),&Mutex::new(Directory::new(vec![])),&yo3_tunnel::ServerInfo::current(true)).await.unwrap();
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
