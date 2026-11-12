//! Framed replies stay validated across orderly TLS closure and aborted/incomplete connections.
use std::{sync::Arc, time::Duration};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use yo3_tunnel::{Endpoint, MAGIC, MOD_ABI, PROTOCOL_VERSION, Role, ServerInfo, Token, tokio_rustls::{TlsAcceptor, rustls}};

fn info_reply(info: &ServerInfo) -> Vec<u8> {
    let body=serde_json::to_vec(info).unwrap();
    let mut reply=(body.len() as u16).to_be_bytes().to_vec();reply.extend(body);reply
}
fn token_reply(token: Token) -> Vec<u8> { let mut reply=vec![0];reply.extend(token);reply }

#[derive(Clone,Copy)]
enum Close { Graceful, Reset }
async fn response_peer(replies: Vec<(u8,Vec<u8>,Close)>) -> (Endpoint,tokio::task::JoinHandle<()>) {
    let rcgen::CertifiedKey{cert,signing_key}=rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let certificate=cert.pem();
    let config=rustls::ServerConfig::builder().with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()],rustls::pki_types::PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into()).unwrap();
    let acceptor=TlsAcceptor::from(Arc::new(config));
    let listener=TcpListener::bind(("127.0.0.1",0)).await.unwrap();
    let endpoint=Endpoint::from_pem(listener.local_addr().unwrap().to_string(),"localhost".into(),certificate.as_bytes()).unwrap();
    let task=tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5),async move {
        for (operation,reply,close) in replies {
            let (tcp,_)=listener.accept().await.unwrap();
            tcp.set_nodelay(true).unwrap();
            let mut tls=acceptor.accept(tcp).await.unwrap();
            let mut magic=[0;8];tls.read_exact(&mut magic).await.unwrap();assert_eq!(&magic,MAGIC);
            assert_eq!(tls.read_u8().await.unwrap(),operation);
            let bytes=match operation {1=>24,2=>33,4=>32,5=>0,_=>panic!("unsupported test operation")};
            let mut request=vec![0;bytes];tls.read_exact(&mut request).await.unwrap();
            tls.write_all(&reply).await.unwrap();tls.flush().await.unwrap();
            match close {
                Close::Reset => {
                    // A reset may discard unread bytes on Windows. Only rejection cases use this abort.
                    #[allow(deprecated, reason = "A zero-linger TCP reset exercises aborted and incomplete replies.")]
                    tls.get_ref().0.set_linger(Some(Duration::ZERO)).unwrap();
                    drop(tls);
                }
                Close::Graceful => {
                    // A caller rejecting a malformed reply may already have closed its socket.
                    let _=tls.shutdown().await;
                    let mut trailing=[0;1];let _=tls.read(&mut trailing).await;
                }
            }
        }
        }).await.expect("test peer did not receive the expected transport requests");
    });
    (endpoint,task)
}

#[tokio::test]
async fn complete_discovery_survives_orderly_connection_cleanup() {
    let (endpoint,peer)=response_peer(vec![(5,info_reply(&ServerInfo::current(true)),Close::Graceful)]).await;
    let info=endpoint.info().await.expect("a complete valid discovery reply must survive connection cleanup");
    assert_eq!(info.protocol,PROTOCOL_VERSION);assert_eq!(info.mod_abi,MOD_ABI);peer.await.unwrap();
}

#[tokio::test]
async fn complete_login_ticket_and_logout_survive_orderly_connection_cleanup() {
    let session=[7;32];let ticket=[8;32];
    let (endpoint,peer)=response_peer(vec![(5,info_reply(&ServerInfo::current(true)),Close::Graceful),(1,token_reply(session),Close::Graceful),(2,token_reply(ticket),Close::Graceful),(4,vec![0],Close::Graceful)]).await;
    assert_eq!(endpoint.login("abcdefghijklmnopqrst").await.unwrap(),session);
    assert_eq!(endpoint.ticket(&session,Role::Front).await.unwrap(),ticket);
    endpoint.logout(&session).await.unwrap();peer.await.unwrap();
}

#[tokio::test]
async fn truncated_or_invalid_discovery_is_still_rejected() {
    for close in [Close::Graceful,Close::Reset] { for reply in [vec![0,10,b'{'],vec![0,1,b'{'],vec![0,0]] {
        let (endpoint,peer)=response_peer(vec![(5,reply,close)]).await;
        assert!(endpoint.info().await.is_err());peer.await.unwrap();
    } }
}

#[tokio::test]
async fn refused_and_truncated_tokens_are_still_rejected() {
    for close in [Close::Graceful,Close::Reset] { for reply in [vec![1],vec![0,1,2]] {
        let (endpoint,peer)=response_peer(vec![(5,info_reply(&ServerInfo::current(true)),Close::Graceful),(1,reply,close)]).await;
        assert!(endpoint.login("abcdefghijklmnopqrst").await.is_err());peer.await.unwrap();
    }
    let (endpoint,peer)=response_peer(vec![(2,vec![1],close),(4,vec![1],close)]).await;
    assert!(endpoint.ticket(&[7;32],Role::Front).await.is_err());assert!(endpoint.logout(&[7;32]).await.is_err());peer.await.unwrap();
    }
}

#[tokio::test]
async fn incompatible_discovery_still_prevents_login() {
    let mut info=ServerInfo::current(true);info.mod_abi=MOD_ABI+1;
    let (endpoint,peer)=response_peer(vec![(5,info_reply(&info),Close::Graceful)]).await;
    let error=endpoint.login("abcdefghijklmnopqrst").await.unwrap_err().to_string();
    assert!(error.contains("connection DLL interface"));peer.await.unwrap();
}
