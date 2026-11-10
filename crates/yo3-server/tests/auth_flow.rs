//! Real TLS and stock game frames: identity must follow the credential rather than the shared source IP.
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_proto::{Message, Packet, Value, decrypt, frame, frame_len};
use yo3_server::{Config, Running, start};
use yo3_server::auth::TlsConfig;
use yo3_tunnel::{Endpoint, Role, Stream, Token};

async fn setup(name: &str) -> (Running, Endpoint, [String; 2], PathBuf) {
    let dir = std::env::temp_dir().join(format!("yo3-auth-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let certificate = dir.join("server.crt"); let private_key = dir.join("server.key");
    std::fs::write(&certificate, cert.cert.pem()).unwrap();
    std::fs::write(&private_key, cert.signing_key.serialize_pem()).unwrap();
    let mut d = yo3_server::accounts::Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    let keys = [d.add_account(&["Alice".into()]).unwrap(), d.add_account(&["Bob".into()]).unwrap()];
    drop(d);
    let cfg = Config { tls: TlsConfig { port: 0, certificate: certificate.clone(), private_key },
        data_dir: Some(dir.clone()), ..Config::ephemeral(dir.join("logs")) };
    let server = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    (server, endpoint, keys, dir)
}

async fn send(s: &mut (impl AsyncWrite + Unpin), m: Message) { s.write_all(&frame(&m.encode())).await.unwrap(); }
async fn recv(s: &mut (impl AsyncRead + Unpin)) -> Message {
    let mut head = [0; 4]; s.read_exact(&mut head).await.unwrap();
    let mut body = vec![0; frame_len(head)]; s.read_exact(&mut body).await.unwrap();
    let Packet::Message(m) = Packet::decode(&decrypt(&body).unwrap()).unwrap() else { panic!("message") }; m
}
async fn front(e: &Endpoint, session: &Token) -> Stream {
    let ticket = e.ticket(session, Role::Front).await.unwrap();
    let mut s = e.connect(&ticket, Role::Front, 43210).await.unwrap();
    s.write_all(&frame(b"\x00\x122010.03.23 skelton")).await.unwrap();
    assert_eq!(recv(&mut s).await.request(), Some(0x219F)); s
}
async fn players(s: &mut Stream) -> Message { send(s, Message::reply(0x1F49)).await; recv(s).await }

#[tokio::test]
async fn two_accounts_on_one_ip_keep_their_identity_and_reject_wrong_keys_and_replay() {
    let (server, e, keys, dir) = setup("shared").await;
    assert!(e.login("xxxxxxxxxxxxxxxxxxxx").await.is_err());
    let a = e.login(&keys[0]).await.unwrap(); let b = e.login(&keys[1]).await.unwrap();
    // Even an old patch-key presentation cannot change an authenticated login.
    let mut http = TcpStream::connect(server.http).await.unwrap();
    http.write_all(format!("GET /yo3/{}/yo3.idx HTTP/1.1\r\nHost: localhost\r\n\r\n", keys[1]).as_bytes()).await.unwrap();
    let mut reply = vec![]; http.read_to_end(&mut reply).await.unwrap();
    let (mut fa, mut fb) = (front(&e, &a).await, front(&e, &b).await);
    for (s, name) in [(&mut fa, "Alice"), (&mut fb, "Bob")] {
        let p = players(s).await;
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].get("pname").and_then(Value::as_str).as_deref(), Some(name));
    }
    let t = e.ticket(&a, Role::Front).await.unwrap();
    let _live = e.connect(&t, Role::Front, 43210).await.unwrap();
    assert!(e.connect(&t, Role::Front, 43210).await.is_err());
    let wrong = e.ticket(&a, Role::Front).await.unwrap();
    assert!(e.connect(&wrong, Role::Duel, 0).await.is_err());
    let logged: String = std::fs::read_dir(dir.join("logs")).unwrap().filter_map(Result::ok)
        .filter_map(|p| std::fs::read_to_string(p.path()).ok()).collect();
    for key in keys { assert!(!logged.contains(&key), "credential in packet logs"); }
}

#[tokio::test]
async fn secure_mode_does_not_accept_a_raw_front_connection() {
    let (server, _, _, _) = setup("raw").await;
    let mut s = TcpStream::connect(server.auth).await.unwrap();
    let _ = s.write_all(&frame(b"\x00\x122010.03.23 skelton")).await;
    let mut response = vec![];
    let _ = tokio::time::timeout(Duration::from_secs(1), s.read_to_end(&mut response)).await.unwrap();
    // TLS may send a fatal alert (record type 21). It must never send the stock 0x219F version acknowledgement.
    assert!(response.is_empty() || response[0] == 21, "unauthenticated game opener was accepted");
}

#[tokio::test]
async fn untrusted_certificates_and_wrong_server_names_are_refused() {
    let (server, e, keys, own) = setup("cert-a").await;
    let (_, _, _, other) = setup("cert-b").await;
    let untrusted = Endpoint::new(server.auth.to_string(), "localhost".into(), &other.join("server.crt")).unwrap();
    assert!(untrusted.login(&keys[0]).await.is_err());
    let wrong_name = Endpoint::new(server.auth.to_string(), "wrong.example".into(), &own.join("server.crt")).unwrap();
    assert!(wrong_name.login(&keys[0]).await.is_err());
    assert!(e.login(&keys[0]).await.is_ok());
}

#[tokio::test]
async fn a_refused_game_frame_closes_tls_without_a_truncation_error() {
    let (_, e, keys, _) = setup("close").await;
    let session = e.login(&keys[0]).await.unwrap();
    let mut s = front(&e, &session).await;
    s.write_all(&[255; 4]).await.unwrap();
    let mut body = vec![];
    assert!(s.read_to_end(&mut body).await.is_ok(), "server must send TLS close_notify");
    assert!(body.is_empty());
}

#[tokio::test]
async fn logout_closes_live_connections_and_refuses_new_tickets() {
    let (_, e, keys, _) = setup("logout").await;
    let root = e.login(&keys[0]).await.unwrap();
    let mut live = front(&e, &root).await;
    e.logout(&root).await.unwrap();
    assert!(e.ticket(&root, Role::Front).await.is_err());
    let mut byte = [0];
    assert_eq!(tokio::time::timeout(Duration::from_secs(2), live.read(&mut byte)).await.unwrap().unwrap(), 0);
}
