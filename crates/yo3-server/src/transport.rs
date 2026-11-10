//! The sole game listener: TLS, then a bounded credential/ticket handshake, then the stock game's frames.
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use yo3_tunnel::{tokio_rustls::{TlsAcceptor, server::TlsStream}, Role, MAGIC, KEY_LEN, HANDSHAKE_TIMEOUT};
use crate::accounts::Directory;
use crate::auth::{Identity, Sessions};
use crate::front::{FrontSession, Hub, World};
use crate::lock::Locked;
use crate::{log::PacketLog, net};

async fn handshake(mut tls: TlsStream<TcpStream>, sessions: &Mutex<Sessions>, accounts: &Mutex<Directory>)
    -> anyhow::Result<Option<(TlsStream<TcpStream>, Identity, Instant, Role, u16)>>
{
    let mut magic = [0; 8]; tls.read_exact(&mut magic).await?;
    anyhow::ensure!(&magic == MAGIC, "unsupported transport version");
    match tls.read_u8().await? {
        1 => {
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
        _ => anyhow::bail!("unsupported transport operation"),
    }
    tls.shutdown().await?;
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
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
            let hello = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
                let tls = acceptor.accept(tcp).await?;
                handshake(tls, &sessions, &accounts).await
            }).await;
            let Ok(Ok(Some((mut tls, identity, expires, role, port)))) = hello else { return };
            let Some(mut lease) = sessions.locked().lease(identity) else { return };
            tracing::info!("[tls] {peer} user {} launcher {} {role:?}", identity.user_id, identity.session_id);
            let work = async move {
                match role {
                    Role::Front => {
                        let Some(account) = accounts.locked().account_of(identity.user_id) else { return Ok(()) };
                        net::secured(tls, "front", log, quiet, move |outbox| {
                            let mut front = FrontSession::new(account, world, hub, outbox);
                            front.set_identity(identity);
                            front.set_tunnel_duel_port(port);
                            front.set_directory(accounts);
                            move |p: &yo3_proto::Packet| front.handle(p)
                        }).await
                    }
                    Role::Duel => net::secured(tls, "duel", log, heartbeats, move |outbox| {
                        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
                        let mut duel = crate::duel::DuelSession::new(world, hub, seed).with_outbox(outbox).with_identity(identity);
                        move |p: &yo3_proto::Packet| duel.handle(p)
                    }).await,
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
