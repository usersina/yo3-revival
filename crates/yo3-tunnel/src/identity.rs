//! Explicit key trust. Discovery is credential-free and does not save or approve the observed identity.
//! Pinned connections check the exact SPKI hash, certificate lifetime and TLS proof of key ownership.
use std::sync::{Arc, Mutex};
use base64::Engine;
use anyhow::{Context, ensure};
use super::{Endpoint, ServerInfo, HANDSHAKE_TIMEOUT, rustls, TlsConnector, CertificateDer, ServerName};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{UnixTime, pem::PemObject};

#[derive(Clone, Debug)]
pub struct ObservedServer { pub info: ServerInfo, pub identity: String, pub certificate: String }

fn certificate_key(der: &[u8]) -> anyhow::Result<String> {
    let (rest, certificate) = x509_parser::parse_x509_certificate(der).map_err(|_| anyhow::anyhow!("invalid host certificate"))?;
    ensure!(rest.is_empty(), "trailing host certificate data");
    let digest = ring::digest::digest(&ring::digest::SHA256, certificate.public_key().raw);
    Ok(digest.as_ref().iter().map(|b| format!("{b:02x}")).collect())
}
pub fn identity_from_pem(pem: &[u8]) -> anyhow::Result<String> {
    let cert = CertificateDer::pem_slice_iter(pem).next().context("host certificate is empty")??;
    certificate_key(cert.as_ref())
}

#[derive(Debug)]
struct IdentityVerifier { expected: Option<String>, observed: Arc<Mutex<Option<Vec<u8>>>> }
impl ServerCertVerifier for IdentityVerifier {
    fn verify_server_cert(&self, cert: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], now: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        let fail = |why: &str| rustls::Error::General(why.into());
        let identity = certificate_key(cert.as_ref()).map_err(|_| fail("Invalid host identity certificate"))?;
        let (_, parsed) = x509_parser::parse_x509_certificate(cert.as_ref()).map_err(|_| fail("Invalid host certificate"))?;
        let at = x509_parser::time::ASN1Time::from_timestamp(now.as_secs() as i64).map_err(|_| fail("Invalid verification time"))?;
        if !parsed.validity().is_valid_at(at) { return Err(fail("Host certificate is expired or not yet valid")) }
        if self.expected.as_ref().is_some_and(|expected| expected != &identity) { return Err(fail("Server identity changed. Confirm the new identity with the host before reconnecting.")) }
        *self.observed.lock().map_err(|_| fail("Identity observation failed"))? = Some(cert.as_ref().to_vec());
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, signature: &rustls::DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, signature, &rustls::crypto::ring::default_provider().signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, signature: &rustls::DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, signature, &rustls::crypto::ring::default_provider().signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> { rustls::crypto::ring::default_provider().signature_verification_algorithms.supported_schemes() }
}

type CertificateObservation = Arc<Mutex<Option<Vec<u8>>>>;
pub(super) fn connector(expected: Option<&str>) -> anyhow::Result<(TlsConnector, CertificateObservation)> {
    if let Some(id) = expected { ensure!(id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()), "invalid server identity code"); }
    let observed = Arc::new(Mutex::new(None));
    let verifier = IdentityVerifier { expected: expected.map(str::to_ascii_lowercase), observed: observed.clone() };
    let config = rustls::ClientConfig::builder().dangerous().with_custom_certificate_verifier(Arc::new(verifier)).with_no_client_auth();
    Ok((TlsConnector::from(Arc::new(config)), observed))
}

/// Observe a host without giving it account credentials. This result is UNTRUSTED until explicitly approved
/// or compared with an identity already supplied by the host. It cannot be used as an authenticated Endpoint.
pub async fn discover(address: &str, name: &str) -> anyhow::Result<ObservedServer> {
    tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        let (connector, observation) = connector(None)?;
        let probe = Endpoint { address: address.into(), name: ServerName::try_from(name.to_string())?, connector, identity: None };
        let info = probe.info().await?;
        let der = observation.lock().map_err(|_| anyhow::anyhow!("identity observation failed"))?.clone().context("server supplied no identity")?;
        let identity = certificate_key(&der)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&der);
        let lines = encoded.as_bytes().chunks(64).map(|s| std::str::from_utf8(s).unwrap()).collect::<Vec<_>>().join("\n");
        Ok(ObservedServer { info, identity, certificate: format!("-----BEGIN CERTIFICATE-----\n{lines}\n-----END CERTIFICATE-----\n") })
    }).await.context("server discovery timed out")?
}
