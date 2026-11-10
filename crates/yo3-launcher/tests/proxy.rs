//! The entire supported path: local mod cookie -> launcher -> validated TLS -> account-bound front and duel.
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_launcher::{Ports, Proxy};
use yo3_proto::{Message, Packet, Value, decrypt, frame, frame_len};
use yo3_tunnel::Endpoint;

async fn read(s: &mut TcpStream) -> Message {
    let mut h = [0; 4]; s.read_exact(&mut h).await.unwrap();
    let mut body = vec![0; frame_len(h)]; s.read_exact(&mut body).await.unwrap();
    let Packet::Message(m) = Packet::decode(&decrypt(&body).unwrap()).unwrap() else { panic!("message") }; m
}
async fn rpc(s: &mut TcpStream, m: Message) -> Message { s.write_all(&frame(&m.encode())).await.unwrap(); read(s).await }
async fn local(p: &Proxy, port: u16) -> TcpStream {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    s.write_all(&p.cookie).await.unwrap(); s
}

#[tokio::test]
async fn a_modded_client_logs_in_and_its_duel_is_bound_to_that_launcher() {
    let dir = std::env::temp_dir().join(format!("yo3-proxy-{}", std::process::id()));
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let has_index = cfg.game_dir.join("yo3.idx").is_file();
    let cert = cfg.tls.certificate.clone();
    let key = std::fs::read_to_string(dir.join("test-auth/client.key")).unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let e = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
    let session = e.login(&key).await.unwrap();
    let p = Proxy::start(e.clone(), session, Ports::default()).await.unwrap();
    let mut raw = TcpStream::connect(("127.0.0.1", p.ports.front)).await.unwrap();
    raw.write_all(&[0; 32]).await.unwrap();
    assert_eq!(raw.read(&mut [0; 1]).await.unwrap(), 0, "an unrelated local process cannot use the proxy");
    let version = frame(b"\x00\x122010.03.23 skelton");
    let mut gate = local(&p, p.ports.gate).await;
    gate.write_all(&version).await.unwrap();
    let redirect = read(&mut gate).await;
    assert_eq!(redirect.get("port"), Some(&Value::U32(p.ports.front as u32)));
    let mut front = local(&p, p.ports.front).await;
    front.write_all(&version).await.unwrap();
    assert_eq!(read(&mut front).await.request(), Some(0x219F));
    let players = rpc(&mut front, Message::reply(0x1F49)).await;
    let pid = players.items[0].get("player_id").cloned().unwrap();
    rpc(&mut front, Message::reply(0x1F4C).with("player_id", pid.clone())).await;
    let deck = rpc(&mut front, Message::reply(0x1F4E)).await.items.into_iter().next()
        .unwrap_or_else(|| Message::deck().with("main", Value::u16_be_array(&[1; 40])));
    let duel = rpc(&mut front, Message::reply(0x17AC).item(Message::record().with("cpu_id", Value::U32(124)))).await;
    let id = duel.get("duel_id").cloned().unwrap();
    assert_eq!(rpc(&mut front, Message::reply(0x1F4F).with("duel_id", id.clone()).with("update_type", Value::U32(1)).item(deck)).await.request(), Some(0x22CA));
    rpc(&mut front, Message::reply(0x17D4).with("duel_id", id)).await;
    let handoff = read(&mut front).await;
    assert_eq!(handoff.items[0].get("host"), Some(&Value::text("127.0.0.1")));
    assert_eq!(handoff.items[0].get("port"), Some(&Value::U32(p.ports.duel as u32)));
    let mut duel = local(&p, p.ports.duel).await;
    assert_eq!(rpc(&mut duel, Message::reply(0x200A).with("player_id", pid.clone())).await.request(), Some(0x200D));
    let other_session = e.login(&key).await.unwrap();
    let other = Proxy::start(e, other_session, Ports::default()).await.unwrap();
    let mut thief = local(&other, other.ports.duel).await;
    thief.write_all(&frame(&Message::reply(0x200A).with("player_id", pid).encode())).await.unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(250), thief.read(&mut [0; 4])).await.is_err());
    let mut http = TcpStream::connect(("127.0.0.1", p.ports.http)).await.unwrap();
    let path = format!("/session/{}/yo3/yo3.idx", yo3_launcher::hex(&p.cookie));
    http.write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).await.unwrap();
    let mut reply = vec![]; http.read_to_end(&mut reply).await.unwrap();
    assert!(reply.starts_with(if has_index { b"HTTP/1.1 200" } else { b"HTTP/1.1 404" }));
}
