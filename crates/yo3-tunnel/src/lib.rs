//! The revival launcher's TLS transport, separate from the unmodified game's wire protocol.
pub use tokio_rustls;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{rustls, TlsAcceptor, TlsConnector};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, pem::PemObject};
mod identity;
pub mod diagnostics;
pub use identity::{ObservedServer, discover, identity_from_pem};

pub type Stream = tokio_rustls::client::TlsStream<TcpStream>;
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub const MAGIC: &[u8; 8] = b"YO3TLS02";
pub const PROTOCOL_VERSION: u16 = 2;
pub const MOD_ABI: u16 = 2;
pub const BUILD_VERSION: &str = env!("YO3_VERSION");
pub const BUILD_COMMIT: &str = env!("YO3_COMMIT");
pub const MAX_INFO: usize = 4096;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ServerInfo {
    pub release: String,
    pub commit: String,
    pub protocol: u16,
    pub mod_abi: u16,
    pub game_version: String,
    pub ready: bool,
    pub problem: Option<String>,
}
impl ServerInfo {
    pub fn current(ready: bool) -> Self {
        Self { release: BUILD_VERSION.into(), commit: BUILD_COMMIT.into(), protocol: PROTOCOL_VERSION,
            mod_abi: MOD_ABI, game_version: "1.153".into(), ready,
            problem: (!ready).then(|| "The host's rules engine is unavailable.".into()) }
    }
    pub fn ensure_compatible(&self, protocol: u16, mod_abi: u16) -> anyhow::Result<()> {
        if self.protocol != protocol { bail!("Server {} requires protocol {}; this application uses {}. Install a compatible release.", self.release, self.protocol, protocol) }
        if self.mod_abi != mod_abi { bail!("Server {} requires connection DLL interface {}; installed components use {}. Repair or update the application.", self.release, self.mod_abi, mod_abi) }
        if self.game_version != "1.153" { bail!("Server requires unsupported game version {}.", self.game_version) }
        Ok(())
    }
}
/// TCP keepalive on every connection to the host: idle this long, then probes this often, until this many go
/// unanswered. A host that went away without closing (asleep, its network gone) otherwise left the game waiting for
/// good with no message; with keepalive the connection drops within about a minute and a half, and the game shows
/// its own "disconnected" dialogs (the connection DLL's logger keeps that from crashing, tools/mod/netlog.h).
pub const KEEPALIVE_IDLE: Duration = Duration::from_secs(45);
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
pub const KEEPALIVE_PROBES: u32 = 4;

pub fn keep_alive(tcp: &TcpStream) -> std::io::Result<()> {
    let keepalive = socket2::TcpKeepalive::new().with_time(KEEPALIVE_IDLE).with_interval(KEEPALIVE_INTERVAL);
    #[cfg(not(windows))]
    let keepalive = keepalive.with_retries(KEEPALIVE_PROBES);
    socket2::SockRef::from(tcp).set_tcp_keepalive(&keepalive)
}

pub const KEY_LEN: usize = 20;
pub const TOKEN_LEN: usize = 32;
pub type Token = [u8; TOKEN_LEN];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Role { Front = 1, Duel = 2, Http = 3 }

impl Role {
    pub fn decode(n: u8) -> Option<Self> {
        match n { 1 => Some(Self::Front), 2 => Some(Self::Duel), 3 => Some(Self::Http), _ => None }
    }
}

pub fn acceptor(cert: &Path, key: &Path) -> anyhow::Result<TlsAcceptor> {
    let certs = CertificateDer::pem_file_iter(cert)?.collect::<Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_file(key)?;
    let config = rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Only the host's supplied certificate is trusted; hostname/expiry checks remain enabled.
#[derive(Clone)]
pub struct Endpoint {
    address: String,
    name: ServerName<'static>,
    connector: TlsConnector,
}

impl Endpoint {
    /// A user/invitation-approved identity authenticates the public key independently of its changing address.
    pub fn pinned(address: String, name: String, identity: &str) -> anyhow::Result<Self> {
        Ok(Self { address, name: ServerName::try_from(name)?, connector: identity::connector(Some(identity))?.0 })
    }
    pub fn new(address: String, name: String, certificate: &Path) -> anyhow::Result<Self> {
        Self::from_pem(address, name, &std::fs::read(certificate)?)
    }

    pub fn from_pem(address: String, name: String, certificate: &[u8]) -> anyhow::Result<Self> {
        let mut roots = rustls::RootCertStore::empty();
        for cert in CertificateDer::pem_slice_iter(certificate) { roots.add(cert?)?; }
        if roots.is_empty() { bail!("host certificate is empty or invalid") }
        let config = rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
        Ok(Self { address, name: ServerName::try_from(name)?, connector: TlsConnector::from(Arc::new(config)) })
    }

    async fn open(&self, op: u8, stage: &mut &'static str) -> anyhow::Result<Stream> {
        *stage = "TCP connect";
        let tcp = TcpStream::connect(&self.address).await.context("connect to server")?;
        tcp.set_nodelay(true)?;
        keep_alive(&tcp)?;
        *stage = "TLS handshake";
        let mut tls = self.connector.connect(self.name.clone(), tcp).await.context("server certificate or TLS handshake failed")?;
        *stage = "transport write";
        tls.write_all(MAGIC).await.context("send transport opener")?;
        tls.write_u8(op).await.context("send transport operation")?;
        Ok(tls)
    }

    pub async fn login(&self, key: &str) -> anyhow::Result<Token> {
        if key.len() != KEY_LEN { bail!("account key must have {KEY_LEN} characters") }
        self.info().await?.ensure_compatible(PROTOCOL_VERSION, MOD_ABI)?;
        let mut stage = "TCP connect";
        let (token,mut tls)=tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(1,&mut stage).await?;
            tls.write_u16(PROTOCOL_VERSION).await?;
            tls.write_u16(MOD_ABI).await?;
            tls.write_all(key.as_bytes()).await?;
            let token = token_response(&mut tls,&mut stage).await?;
            Ok::<_,anyhow::Error>((token,tls))
        }).await.with_context(|| format!("login timed out during {stage}"))??;
        finish_response(token,&mut tls).await
    }

    /// Credential-free discovery, over the same authenticated TLS endpoint as game connections.
    pub async fn info(&self) -> anyhow::Result<ServerInfo> {
        let mut stage = "TCP connect";
        let (info,mut tls)=tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(5,&mut stage).await?;
            stage = "read server discovery response length";
            let length = tls.read_u16().await.context("read server discovery response length")? as usize;
            if length == 0 || length > MAX_INFO { bail!("invalid server discovery response length") }
            stage = "read server discovery response body";
            let mut body = vec![0; length]; tls.read_exact(&mut body).await.context("read server discovery response body")?;
            let info = serde_json::from_slice(&body).context("invalid server discovery response")?;
            Ok::<_,anyhow::Error>((info,tls))
        }).await.with_context(|| format!("server discovery timed out during {stage}"))?.context("Server discovery failed; verify its address, identity and revival version")?;
        finish_response(info,&mut tls).await
    }

    pub async fn ticket(&self, session: &Token, role: Role) -> anyhow::Result<Token> {
        let mut stage = "TCP connect";
        let (token,mut tls)=tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(2,&mut stage).await?;
            tls.write_all(session).await?;
            tls.write_u8(role as u8).await?;
            let token = token_response(&mut tls,&mut stage).await?;
            Ok::<_,anyhow::Error>((token,tls))
        }).await.with_context(|| format!("ticket request timed out during {stage}"))??;
        finish_response(token,&mut tls).await
    }

    pub async fn logout(&self, session: &Token) -> anyhow::Result<()> {
        let mut stage = "TCP connect";
        let mut tls=tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(4,&mut stage).await?;
            tls.write_all(session).await?;
            stage = "read logout acknowledgement";
            if tls.read_u8().await.context("read logout acknowledgement")? != 0 { bail!("logout refused") }
            Ok::<_,anyhow::Error>(tls)
        }).await.with_context(|| format!("logout timed out during {stage}"))??;
        finish_response((),&mut tls).await
    }

    pub async fn connect(&self, ticket: &Token, role: Role, local_duel_port: u16) -> anyhow::Result<Stream> {
        let mut stage = "TCP connect";
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(3,&mut stage).await?;
            tls.write_all(ticket).await?;
            tls.write_u8(role as u8).await?;
            tls.write_u16(local_duel_port).await?;
            stage = "read connection ticket acknowledgement";
            if tls.read_u8().await? != 0 { bail!("connection ticket refused") }
            Ok(tls)
        }).await.with_context(|| format!("connection timed out during {stage}"))?
    }
}

async fn finish_response<T,W: AsyncWrite+Unpin>(value: T, stream: &mut W) -> anyhow::Result<T> {
    // Each caller has already read and validated its entire framed reply. A peer may close/reset immediately
    // afterwards; failure to send our close_notify must not turn a successful operation into a startup failure.
    // Keep cleanup separate from the request deadline so a late complete reply remains successful too.
    let _=tokio::time::timeout(Duration::from_secs(1),stream.shutdown()).await;
    Ok(value)
}

async fn token_response(tls: &mut Stream, stage: &mut &'static str) -> anyhow::Result<Token> {
    *stage = "read login/session acknowledgement";
    if tls.read_u8().await.context("read login/session acknowledgement")? != 0 { bail!("login or session refused") }
    let mut token = [0; TOKEN_LEN];
    *stage = "read complete login/session token";
    tls.read_exact(&mut token).await.context("read complete login/session token")?;
    Ok(token)
}

#[cfg(test)]
mod response_finalization {
    use super::*;
    use std::{io,pin::Pin,task::{Context,Poll}};

    struct ClosingWriter { pending: bool, attempts: usize }
    impl AsyncWrite for ClosingWriter {
        fn poll_write(self: Pin<&mut Self>,_: &mut Context<'_>,bytes: &[u8]) -> Poll<io::Result<usize>> { Poll::Ready(Ok(bytes.len())) }
        fn poll_flush(self: Pin<&mut Self>,_: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
        fn poll_shutdown(mut self: Pin<&mut Self>,_: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.attempts+=1;
            if self.pending { Poll::Pending } else { Poll::Ready(Err(io::ErrorKind::ConnectionReset.into())) }
        }
    }

    #[tokio::test]
    async fn a_validated_reply_survives_a_reset_during_shutdown() {
        let value=[7;TOKEN_LEN];let mut writer=ClosingWriter{pending:false,attempts:0};
        let result=finish_response(value,&mut writer).await;
        assert!(writer.attempts>0,"attempt an orderly TLS close after reading the reply");
        assert_eq!(result.expect("cleanup cannot invalidate an already validated reply"),value);
    }

    #[tokio::test]
    async fn a_validated_reply_survives_a_shutdown_that_never_finishes() {
        let value=[8;TOKEN_LEN];let mut writer=ClosingWriter{pending:true,attempts:0};
        let result=tokio::time::timeout(Duration::from_secs(2),finish_response(value,&mut writer)).await
            .expect("response cleanup must retain its one-second bound");
        assert!(writer.attempts>0,"attempt an orderly TLS close after reading the reply");
        assert_eq!(result.expect("a stalled close cannot invalidate an already validated reply"),value);
    }
}

#[cfg(test)]
mod keepalive {
    #[tokio::test]
    async fn connections_to_the_host_keep_alive() {
        // A host gone without closing (asleep, network lost) left the game waiting with no message; keepalive makes
        // the connection drop, and the game then shows its own disconnect dialogs.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tcp = tokio::net::TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
        super::keep_alive(&tcp).unwrap();
        let sock = socket2::SockRef::from(&tcp);
        assert!(sock.keepalive().unwrap());
        #[cfg(target_os = "linux")]
        {
            assert_eq!(sock.tcp_keepalive_time().unwrap(), super::KEEPALIVE_IDLE);
            assert_eq!(sock.tcp_keepalive_interval().unwrap(), super::KEEPALIVE_INTERVAL);
            assert_eq!(sock.tcp_keepalive_retries().unwrap(), super::KEEPALIVE_PROBES);
        }
    }
}
