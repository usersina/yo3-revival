//! A stalled authenticated connection reports which phase consumed the existing request deadline.
use std::{sync::Arc,time::Duration};
use tokio::{io::{AsyncReadExt,AsyncWriteExt},net::TcpListener};
use yo3_tunnel::{Endpoint,HANDSHAKE_TIMEOUT,MAGIC,tokio_rustls::{TlsAcceptor,rustls}};

enum Stall { Tls, DiscoveryLength, DiscoveryBody }
async fn stalled_peer(stage: Stall) -> (Endpoint,tokio::task::JoinHandle<()>) {
    let rcgen::CertifiedKey{cert,signing_key}=rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let certificate=cert.pem();
    let config=rustls::ServerConfig::builder().with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()],rustls::pki_types::PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into()).unwrap();
    let acceptor=TlsAcceptor::from(Arc::new(config));
    let listener=TcpListener::bind(("127.0.0.1",0)).await.unwrap();
    let endpoint=Endpoint::from_pem(listener.local_addr().unwrap().to_string(),"localhost".into(),certificate.as_bytes()).unwrap();
    let peer=tokio::spawn(async move {
        let (mut tcp,_)=listener.accept().await.unwrap();
        if matches!(stage,Stall::Tls) {
            tcp.read_u8().await.unwrap();
            std::future::pending::<()>().await; drop(tcp); return
        }
        let mut tls=acceptor.accept(tcp).await.unwrap();
        let mut magic=[0;8];tls.read_exact(&mut magic).await.unwrap();assert_eq!(&magic,MAGIC);
        assert_eq!(tls.read_u8().await.unwrap(),5);
        if matches!(stage,Stall::DiscoveryBody) { tls.write_u16(8).await.unwrap();tls.write_all(b"{").await.unwrap();tls.flush().await.unwrap(); }
        std::future::pending::<()>().await; drop(tls);
    });
    (endpoint,peer)
}

async fn assert_timeout(stage: Stall, expected: &str) {
    let (endpoint,peer)=stalled_peer(stage).await;
    let error=tokio::time::timeout(HANDSHAKE_TIMEOUT+Duration::from_secs(2),endpoint.info()).await.unwrap().unwrap_err();
    peer.abort();let _=peer.await;
    let message=format!("{error:#}");
    assert!(message.contains("timed out"),"{message}");
    assert!(message.contains(expected),"identify the phase that actually stalled: {message}");
}

#[tokio::test]
async fn discovery_timeout_identifies_a_stalled_tls_handshake() { assert_timeout(Stall::Tls,"TLS handshake").await }
#[tokio::test]
async fn discovery_timeout_identifies_the_missing_response_length() { assert_timeout(Stall::DiscoveryLength,"discovery response length").await }
#[tokio::test]
async fn discovery_timeout_identifies_an_incomplete_response_body() { assert_timeout(Stall::DiscoveryBody,"discovery response body").await }
