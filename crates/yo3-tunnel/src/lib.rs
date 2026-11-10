//! The revival launcher's TLS transport, separate from the unmodified game's wire protocol.
pub use tokio_rustls;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{rustls, TlsAcceptor, TlsConnector};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, pem::PemObject};
mod identity;
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

    async fn open(&self, op: u8) -> anyhow::Result<Stream> {
        let tcp = TcpStream::connect(&self.address).await?;
        tcp.set_nodelay(true)?;
        let mut tls = self.connector.connect(self.name.clone(), tcp).await.context("server certificate or TLS handshake failed")?;
        tls.write_all(MAGIC).await?;
        tls.write_u8(op).await?;
        Ok(tls)
    }

    pub async fn login(&self, key: &str) -> anyhow::Result<Token> {
        if key.len() != KEY_LEN { bail!("account key must have {KEY_LEN} characters") }
        self.info().await?.ensure_compatible(PROTOCOL_VERSION, MOD_ABI)?;
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(1).await?;
            tls.write_u16(PROTOCOL_VERSION).await?;
            tls.write_u16(MOD_ABI).await?;
            tls.write_all(key.as_bytes()).await?;
            let token = token_response(&mut tls).await?;
            tls.shutdown().await?;
            Ok(token)
        }).await.context("login timed out")?
    }

    /// Credential-free discovery, over the same authenticated TLS endpoint as game connections.
    pub async fn info(&self) -> anyhow::Result<ServerInfo> {
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(5).await?;
            let length = tls.read_u16().await? as usize;
            if length == 0 || length > MAX_INFO { bail!("invalid server discovery response length") }
            let mut body = vec![0; length]; tls.read_exact(&mut body).await?;
            let info = serde_json::from_slice(&body).context("invalid server discovery response")?;
            tls.shutdown().await?;
            Ok(info)
        }).await.context("server discovery timed out")?.context("Server discovery failed; verify its address, identity and revival version")
    }

    pub async fn ticket(&self, session: &Token, role: Role) -> anyhow::Result<Token> {
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(2).await?;
            tls.write_all(session).await?;
            tls.write_u8(role as u8).await?;
            let token = token_response(&mut tls).await?;
            tls.shutdown().await?;
            Ok(token)
        }).await.context("ticket request timed out")?
    }

    pub async fn logout(&self, session: &Token) -> anyhow::Result<()> {
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(4).await?;
            tls.write_all(session).await?;
            if tls.read_u8().await? != 0 { bail!("logout refused") }
            tls.shutdown().await?;
            Ok(())
        }).await.context("logout timed out")?
    }

    pub async fn connect(&self, ticket: &Token, role: Role, local_duel_port: u16) -> anyhow::Result<Stream> {
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(3).await?;
            tls.write_all(ticket).await?;
            tls.write_u8(role as u8).await?;
            tls.write_u16(local_duel_port).await?;
            if tls.read_u8().await? != 0 { bail!("connection ticket refused") }
            Ok(tls)
        }).await.context("connection timed out")?
    }
}

async fn token_response(tls: &mut Stream) -> anyhow::Result<Token> {
    if tls.read_u8().await? != 0 { bail!("login or session refused") }
    let mut token = [0; TOKEN_LEN];
    tls.read_exact(&mut token).await?;
    Ok(token)
}
