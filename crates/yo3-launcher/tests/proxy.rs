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
async fn gate_redirect_stays_open_until_the_client_or_proxy_closes_it() {
    // 0x535210 dereferences a missing stock logger after premature gate EOF. Automatic login exposes the race.
    let dir = std::env::temp_dir().join(format!("yo3-gate-lifetime-{}", std::process::id()));
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let endpoint = Endpoint::new("127.0.0.1:1".into(), "localhost".into(), &cfg.tls.certificate).unwrap();
    let proxy = Proxy::prepare(endpoint, Ports::default()).await.unwrap();
    let mut gate = local(&proxy, proxy.ports.gate).await;
    gate.write_all(&frame(b"\x00\x122010.03.23 skelton")).await.unwrap();
    assert_eq!(read(&mut gate).await.request(), Some(0x21FE));
    assert!(tokio::time::timeout(Duration::from_millis(75), gate.read(&mut [0; 1])).await.is_err(),
        "the gate must not send EOF while the native login window still owns it");
    gate.shutdown().await.unwrap();
    assert_eq!(tokio::time::timeout(Duration::from_secs(1), gate.read(&mut [0; 1])).await.unwrap().unwrap(), 0);
    let mut next = local(&proxy, proxy.ports.gate).await;
    next.write_all(&frame(b"\x00\x122010.03.23 skelton")).await.unwrap();
    assert_eq!(read(&mut next).await.request(), Some(0x21FE));
    drop(proxy);
    assert_eq!(tokio::time::timeout(Duration::from_secs(1), next.read(&mut [0; 1])).await.unwrap().unwrap(), 0,
        "closing the proxy must release accepted gate sockets too");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_modded_client_logs_in_and_its_duel_is_bound_to_that_launcher() {
    let dir = std::env::temp_dir().join(format!("yo3-proxy-{}", std::process::id()));
    let cfg = yo3_server::Config::ephemeral(dir.clone());
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
    // The server's chat notes (0x2455, the duel's banlist) may follow Select Deck's answer: past them, the report's.
    let mut answered = rpc(&mut front, Message::reply(0x17D4).with("duel_id", id)).await;
    while answered.request() == Some(0x2455) { answered = read(&mut front).await; }
    assert_eq!(answered.request(), Some(0x17D5));
    let handoff = read(&mut front).await;
    assert_eq!(handoff.items[0].get("host"), Some(&Value::text("127.0.0.1")));
    assert_eq!(handoff.items[0].get("port"), Some(&Value::U32(p.ports.duel as u32)));
    let mut duel = local(&p, p.ports.duel).await;
    assert_eq!(rpc(&mut duel, Message::reply(0x200A).with("player_id", pid.clone())).await.request(), Some(0x200D));
    // Another account's launcher (a second login of the same account would end this one's session: the newest wins).
    yo3_server::logins::use_fast_hashing();
    let made = e.account(&serde_json::json!({ "op": "register", "user": "thief", "password": "LongPassphrase1", "rules": true })).await.unwrap();
    assert_eq!(made["ok"], true, "{made}");
    let signed = e.account(&serde_json::json!({ "op": "login", "user": "thief", "password": "LongPassphrase1" })).await.unwrap();
    let hex = signed["session"].as_str().unwrap();
    let other_session: [u8; 32] = (0..32).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap()).collect::<Vec<_>>().try_into().unwrap();
    let other = Proxy::start(e, other_session, Ports::default()).await.unwrap();
    let mut thief = local(&other, other.ports.duel).await;
    thief.write_all(&frame(&Message::reply(0x200A).with("player_id", pid).encode())).await.unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(250), thief.read(&mut [0; 4])).await.is_err());
    let mut http = TcpStream::connect(("127.0.0.1", p.ports.http)).await.unwrap();
    let path = format!("/session/{}/yo3/yo3.idx", yo3_launcher::hex(&p.cookie));
    http.write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).await.unwrap();
    let mut reply = vec![]; http.read_to_end(&mut reply).await.unwrap();
    assert!(reply.starts_with(b"HTTP/1.1 404"), "no local index given: the server's isn't relayed either (#39)");
}

async fn http_get(p: &Proxy, path: &str) -> Vec<u8> {
    let mut http = TcpStream::connect(("127.0.0.1", p.ports.http)).await.unwrap();
    let path = format!("/session/{}/{path}", yo3_launcher::hex(&p.cookie));
    http.write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).await.unwrap();
    let mut reply = vec![]; http.read_to_end(&mut reply).await.unwrap(); reply
}

#[tokio::test]
async fn the_patch_check_gets_the_games_own_index_and_no_server_reaches_the_patcher() {
    // Security review (#39): the stock patcher (PatchComponent 0x4EA470, 0x4EAA50) writes the files of a patch archive
    // the index leads it to into the game folder under their own names, and swaps yo3.exe itself. A server joined on a
    // normal Play must never answer the index or the patch files: the game's own index means "nothing to update".
    let dir = std::env::temp_dir().join(format!("yo3-proxy-index-{}", std::process::id()));
    let mut cfg = yo3_server::Config::ephemeral(dir.clone());
    let game = dir.join("server-game"); std::fs::create_dir_all(&game).unwrap();
    std::fs::write(game.join("yo3.idx"), b"SERVER INDEX").unwrap();
    cfg.game_dir = game; cfg.allow_missing_game_data = true;
    let cert = cfg.tls.certificate.clone();
    let key = std::fs::read_to_string(dir.join("test-auth/client.key")).unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let e = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
    let session = e.login(&key).await.unwrap();
    let p = Proxy::start(e, session, Ports::default()).await.unwrap();
    // No local index known: the server's still isn't relayed.
    let reply = http_get(&p, "yo3/yo3.idx").await;
    assert!(reply.starts_with(b"HTTP/1.1 404") && !reply.ends_with(b"SERVER INDEX"), "{}", String::from_utf8_lossy(&reply));
    p.local_index(b"LOCAL INDEX".to_vec());
    let reply = http_get(&p, "yo3/yo3.idx").await;
    assert!(reply.starts_with(b"HTTP/1.1 200") && reply.ends_with(b"\r\n\r\nLOCAL INDEX"), "{}", String::from_utf8_lossy(&reply));
    for path in ["ygoda/patch/yo3.exe", "ygoda/patch/00000001.ypt", "yo3/other", "YO3/yo3.idx"] {
        let reply = http_get(&p, path).await;
        assert!(reply.starts_with(b"HTTP/1.1 404"), "{path}: {}", String::from_utf8_lossy(&reply));
    }
    // The other files the game fetches still come from the server.
    assert!(http_get(&p, "web/v3/bpass/index/").await.starts_with(b"HTTP/1.1 200"));
    drop(p); drop(server);
    let _ = std::fs::remove_dir_all(dir);
}

/// A TCP forwarder in front of the server that can freeze the way a stopped or hung server process does: its
/// connections stay open and new ones are accepted, but no byte gets through either way.
async fn freezer(to: std::net::SocketAddr) -> (String, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    async fn pump(mut from: tokio::net::tcp::OwnedReadHalf, mut to: tokio::net::tcp::OwnedWriteHalf, frozen: Arc<AtomicBool>) {
        let mut buf = [0; 8192];
        loop {
            let n = match from.read(&mut buf).await { Ok(0) | Err(_) => { let _ = to.shutdown().await; return } Ok(n) => n };
            while frozen.load(Ordering::SeqCst) { tokio::time::sleep(Duration::from_millis(10)).await }
            if to.write_all(&buf[..n]).await.is_err() { return }
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let frozen = Arc::new(AtomicBool::new(false));
    let shared = frozen.clone();
    tokio::spawn(async move {
        while let Ok((client, _)) = listener.accept().await {
            let frozen = shared.clone();
            tokio::spawn(async move {
                let Ok(server) = TcpStream::connect(to).await else { return };
                let ((cr, cw), (sr, sw)) = (client.into_split(), server.into_split());
                tokio::join!(pump(cr, sw, frozen.clone()), pump(sr, cw, frozen));
            });
        }
    });
    (address, frozen)
}

#[tokio::test]
async fn a_server_that_stops_answering_drops_the_games_connections() {
    // A hung server process keeps its connections open, so the game waited in its duel for good with no message (live
    // with SIGSTOP). The launcher checks the server while the game is connected; when it stops answering, the game's
    // connections close and the game shows its own "disconnected" dialogs and returns to the title.
    let dir = std::env::temp_dir().join(format!("yo3-proxy-frozen-{}", std::process::id()));
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let cert = cfg.tls.certificate.clone();
    let key = std::fs::read_to_string(dir.join("test-auth/client.key")).unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let (address, frozen) = freezer(server.auth).await;
    let e = Endpoint::new(address, "localhost".into(), &cert).unwrap();
    let session = e.login(&key).await.unwrap();
    let watch = yo3_launcher::Liveness { every: Duration::from_millis(100), timeout: Duration::from_millis(300), misses: 3 };
    let p = Proxy::prepare_watched(e, Ports::default(), watch).await.unwrap();
    p.activate(session);
    let mut front = local(&p, p.ports.front).await;
    front.write_all(&frame(b"\x00\x122010.03.23 skelton")).await.unwrap();
    assert_eq!(read(&mut front).await.request(), Some(0x219F));
    assert!(tokio::time::timeout(Duration::from_millis(1200), front.read(&mut [0; 1])).await.is_err(),
        "a server that answers keeps the game's connection, however quiet it is");
    frozen.store(true, std::sync::atomic::Ordering::SeqCst);
    let closed = tokio::time::timeout(Duration::from_secs(5), front.read(&mut [0; 1])).await
        .expect("the game's connection must close once the server stops answering");
    assert!(matches!(closed, Ok(0) | Err(_)), "{closed:?}");
    std::fs::remove_dir_all(dir).ok();
}

/// One of the server's pages through a launcher's proxy, as the browser asks for it: status line and body.
async fn page(p: &Proxy, path: &str) -> (String, String) {
    let reply = String::from_utf8(http_get(p, path).await).unwrap();
    let (head, body) = reply.split_once("\r\n\r\n").unwrap_or((&reply, ""));
    (head.lines().next().unwrap_or_default().to_string(), body.to_string())
}

/// #74: the game's website links open the joined server's own pages, under its name, and when the server can't send
/// one the browser gets a page that says so (the game's own requests still just end).
#[tokio::test]
async fn each_launcher_shows_its_own_servers_pages_and_says_when_the_server_cant_answer() {
    let dir = std::env::temp_dir().join(format!("yo3-proxy-pages-{}", std::process::id()));
    let mut proxies = vec![];
    let mut servers = vec![];
    for name in ["Kaiba Land", "Duel <Academy>"] {
        let data = dir.join(name.replace(['<', '>', ' '], ""));
        let mut cfg = yo3_server::Config::ephemeral(data.clone());
        cfg.settings.push(("server-name".into(), name.into()));
        let cert = cfg.tls.certificate.clone();
        let key = std::fs::read_to_string(data.join("test-auth/client.key")).unwrap();
        let server = yo3_server::start(cfg).await.unwrap();
        let e = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
        let session = e.login(&key).await.unwrap();
        proxies.push(Proxy::start(e, session, Ports::default()).await.unwrap());
        servers.push(server);
    }
    let (status, kaiba) = page(&proxies[0], "web/").await;
    assert!(status.starts_with("HTTP/1.1 200") && kaiba.contains("<b>Kaiba Land</b>") && !kaiba.contains("Academy"), "{kaiba}");
    let (_, academy) = page(&proxies[1], "web/v3/events/rank_info?country=GB").await;
    assert!(academy.contains("<b>Duel &lt;Academy&gt;</b>") && academy.contains("<title>Leaderboard - "), "{academy}");
    assert!(academy.contains("href=\"../../packs\""), "the bar's links are relative: {academy}");
    // The first server goes away: its page says the server didn't answer, a duel log just ends.
    let first = servers.remove(0);
    first.stopper().send_replace(true);
    tokio::time::timeout(Duration::from_secs(10), first.wait()).await.expect("the server stops");
    let (status, gone) = page(&proxies[0], "web/v3/bpass/index/").await;
    assert!(status.starts_with("HTTP/1.1 502") && gone.contains("The server didn't answer"), "{status} {gone}");
    assert!(http_get(&proxies[0], "duellogs/x.dlog").await.is_empty(), "the game's own request gets no page");
    // The other launcher's server is still there.
    assert!(page(&proxies[1], "web/packs").await.0.starts_with("HTTP/1.1 200"));
    drop(proxies); drop(servers);
    let _ = std::fs::remove_dir_all(dir);
}
