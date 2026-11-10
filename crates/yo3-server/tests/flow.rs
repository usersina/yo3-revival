//! Drives the real TCP servers the way the client does: gate -> front -> login -> main screen.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_proto::{Message, Packet, Value, decrypt, frame, frame_len};
use yo3_server::{Config, start};

async fn send(s: &mut (impl AsyncWrite + Unpin), plain: &[u8]) {
    s.write_all(&frame(plain)).await.unwrap();
}

async fn recv(s: &mut (impl AsyncRead + Unpin)) -> Message {
    let mut head = [0; 4];
    s.read_exact(&mut head).await.unwrap();
    let mut body = vec![0; frame_len(head)];
    s.read_exact(&mut body).await.unwrap();
    match Packet::decode(&decrypt(&body).unwrap()).unwrap() {
        Packet::Message(m) => m,
        other => panic!("unexpected {other:?}"),
    }
}

async fn rpc(s: &mut (impl AsyncRead + AsyncWrite + Unpin), msg: Message) -> Message {
    send(s, &msg.encode()).await;
    recv(s).await
}

fn version() -> Vec<u8> {
    [b"\x00\x122010.03.23 skelton".as_slice(), &[0; 4]].concat()
}

#[tokio::test]
async fn client_reaches_the_main_screen() {
    let log_dir = std::env::temp_dir().join(format!("yo3-flow-{}", std::process::id()));
    let cfg = Config::ephemeral(log_dir.clone());
    let certificate = cfg.tls.certificate.clone();
    let key = std::fs::read_to_string(log_dir.join("test-auth/client.key")).unwrap();
    let server = start(cfg).await.unwrap();
    let e = yo3_tunnel::Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    let session = e.login(&key).await.unwrap();
    let ticket = e.ticket(&session, yo3_tunnel::Role::Front).await.unwrap();
    let mut front = e.connect(&ticket, yo3_tunnel::Role::Front, 43210).await.unwrap();
    send(&mut front, &version()).await;
    assert_eq!(recv(&mut front).await.request(), Some(0x219F));
    for (req, reply) in [(0x1F47, 0x219A), (0x1F48, 0x2201), (0x1F49, 0x2260)] {
        assert_eq!(rpc(&mut front, Message::reply(req)).await.request(), Some(reply));
        if req == 0x1F48 {
            assert_eq!(recv(&mut front).await.request(), Some(0x23F0), "the premium push after the login");
        }
    }
    let select = Message::reply(0x1F4C).with("player_id", Value::U32(1001));
    assert_eq!(rpc(&mut front, select).await.request(), Some(0x2266));
    let trunk = Message::reply(0x1FA4).with("player_id", Value::U32(1001));
    assert_eq!(rpc(&mut front, trunk).await.request(), Some(9000));
    assert_eq!(recv(&mut front).await.request(), Some(0x251C), "friend list push");
    assert_eq!(recv(&mut front).await.request(), Some(0x252A), "black list push");
    assert_eq!(recv(&mut front).await.request(), Some(0x2D6C), "rental records push");
    // Real game data supplies an automatic deck list after collection/rentals. Without that optional data
    // there is no legal candidate and no extra packet. In both cases the next main-screen RPC must succeed.
    send(&mut front, &Message::reply(0x2072).encode()).await;
    let mut next = recv(&mut front).await;
    if next.request() == Some(0x22C8) {
        assert!(!next.items.is_empty() && next.items.len() <= 10);
        assert!(next.items.iter().all(|deck| deck.tag == 6));
        next = recv(&mut front).await;
    }
    assert_eq!(next.request(), Some(0x2398));

    // every packet is logged decoded
    let logged: usize = std::fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("packets-"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap().lines().count())
        .sum();
    assert!(logged >= 16, "only {logged} packets logged");
}

#[tokio::test]
async fn patch_index_is_served_over_http() {
    let server = start(Config::ephemeral(std::env::temp_dir().join("yo3-http"))).await.unwrap();
    let mut s = TcpStream::connect(server.http).await.unwrap();
    s.write_all(b"GET /yo3/yo3.idx HTTP/1.1\r\nHost: x\r\n\r\n").await.unwrap();
    let mut resp = String::new();
    s.read_to_string(&mut resp).await.unwrap();
    let game_has_idx = yo3_data::default_game_dir().join("yo3.idx").is_file();
    assert!(resp.starts_with(if game_has_idx { "HTTP/1.1 200" } else { "HTTP/1.1 404" }), "{resp}");
}
