//! Drives the real TCP servers the way the client does: gate -> front -> login -> main screen.
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_proto::{Message, Packet, Value, decrypt, frame, frame_len};
use yo3_server::{Config, start};

async fn send(s: &mut TcpStream, plain: &[u8]) {
    s.write_all(&frame(plain)).await.unwrap();
}

async fn recv(s: &mut TcpStream) -> Message {
    let mut head = [0; 4];
    s.read_exact(&mut head).await.unwrap();
    let mut body = vec![0; frame_len(head)];
    s.read_exact(&mut body).await.unwrap();
    match Packet::decode(&decrypt(&body).unwrap()).unwrap() {
        Packet::Message(m) => m,
        other => panic!("unexpected {other:?}"),
    }
}

async fn rpc(s: &mut TcpStream, msg: Message) -> Message {
    send(s, &msg.encode()).await;
    recv(s).await
}

fn version() -> Vec<u8> {
    [b"\x00\x122010.03.23 skelton".as_slice(), &[0; 4]].concat()
}

#[tokio::test]
async fn client_reaches_the_main_screen() {
    let log_dir = std::env::temp_dir().join(format!("yo3-flow-{}", std::process::id()));
    let server = start(Config::ephemeral(log_dir.clone())).await.unwrap();

    let mut gate = TcpStream::connect(server.gate).await.unwrap();
    send(&mut gate, &version()).await;
    let redirect = recv(&mut gate).await;
    assert_eq!(redirect.request(), Some(0x21FE));
    let port = redirect.get("port").and_then(Value::as_u32).unwrap() as u16;
    assert_eq!(port, server.front.port());

    let mut front = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).await.unwrap();
    send(&mut front, &version()).await;
    assert_eq!(recv(&mut front).await.request(), Some(0x219F));
    for (req, reply) in [(0x1F47, 0x219A), (0x1F48, 0x2201), (0x1F49, 0x2260)] {
        assert_eq!(rpc(&mut front, Message::reply(req)).await.request(), Some(reply));
    }
    let select = Message::reply(0x1F4C).with("player_id", Value::U32(1001));
    assert_eq!(rpc(&mut front, select).await.request(), Some(0x2266));
    let trunk = Message::reply(0x1FA4).with("player_id", Value::U32(1001));
    assert_eq!(rpc(&mut front, trunk).await.request(), Some(9000));
    assert_eq!(rpc(&mut front, Message::reply(0x2072)).await.request(), Some(0x2398));

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
