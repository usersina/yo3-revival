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

pub type Stream = tokio_rustls::client::TlsStream<TcpStream>;
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub const MAGIC: &[u8; 8] = b"YO3TLS01";
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
        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut tls = self.open(1).await?;
            tls.write_all(key.as_bytes()).await?;
            let token = token_response(&mut tls).await?;
            tls.shutdown().await?;
            Ok(token)
        }).await.context("login timed out")?
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
